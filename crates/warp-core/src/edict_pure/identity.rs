// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo's pure identity intrinsics: `ContentAddressV1` over evaluated operands.
//!
//! An authored program calls these coordinates to compute the ids that effect
//! admission recomputes. The identity law lives only in
//! [`crate::content_address_v1`]; nothing here restates or alters it.
//!
//! | Coordinate                  | Operands                                    |
//! | --------------------------- | ------------------------------------------- |
//! | `echo.dpo@1.contentAddress` | warp, node type, atom type, atom, edges     |
//! | `echo.dpo@1.contentEdgeId`  | warp, source node, edge type, target node   |
//! | `echo.dpo@1.slotEdgeId`     | warp, source node, edge type                |
//!
//! Every id operand is exactly 32 bytes. The atom is at most
//! [`MAX_ATOM_BYTES`] raw bytes, and `edges` is a list of at most
//! [`MAX_OUTGOING_EDGES`] records `{edgeType, to}` of two ids, in any order.
//! Every call returns one 32-byte id and takes no type arguments.
//!
//! A call evaluates its operands once, left to right, requires each value to
//! inhabit its type, charges all of its work, and only then hashes. Beyond
//! evaluating its operands a call costs:
//!
//! - `contentAddress`: 71 steps, one per atom byte and 67 per edge; 96 bytes
//!   of storage and 64 per edge. Edge order is not a cost input.
//! - `contentEdgeId`: 70 steps and 96 bytes.
//! - `slotEdgeId`: 69 steps and 96 bytes.
//!
//! The same (edge type, target) twice is refused with
//! [`Error::DuplicateOutgoingEdge`] after the work is charged and before any
//! id is produced. The edge set is never deduplicated.
use std::collections::BTreeMap;

use echo_edict_canonical::CanonicalValueV1 as Value;

use super::evaluate::{expression, validate, Meter};
use super::functions::accepts;
use super::model::{Expr, Helper, IdentityIntrinsic, RuntimeType};
use super::syntax::Parser;
use super::values::{bytes, field};
use super::EvaluationError as Error;
use crate::content_address_v1::{content_edge_id, content_node_id, slot_edge_id, OutgoingEdge};
use crate::{NodeId, TypeId, WarpId};

/// Largest atom a `contentAddress` call accepts.
pub(crate) const MAX_ATOM_BYTES: u64 = 1_048_576;
/// Most outgoing edges a `contentAddress` call accepts.
pub(crate) const MAX_OUTGOING_EDGES: u64 = 16;

const ID_BYTES: u64 = 32;
/// Fixed framing and digest work of any identity call.
const CALL_STEPS: u64 = 32;
/// The 64 hashed bytes of one outgoing edge.
const EDGE_STEPS: u64 = 64;
/// One edge's place in the canonical-order copy.
const EDGE_SCRATCH_BYTES: u64 = 64;

impl IdentityIntrinsic {
    /// The intrinsic a callee coordinate names, if any.
    pub(super) fn named(callee: &str) -> Option<Self> {
        match callee {
            "echo.dpo@1.contentAddress" => Some(Self::ContentAddress),
            "echo.dpo@1.contentEdgeId" => Some(Self::ContentEdgeId),
            "echo.dpo@1.slotEdgeId" => Some(Self::SlotEdgeId),
            _ => None,
        }
    }

    /// The ceiling type of each operand, in call order.
    pub(super) fn parameter_types(self) -> Vec<RuntimeType> {
        match self {
            Self::ContentAddress => vec![
                id_type(),
                id_type(),
                id_type(),
                RuntimeType::Bytes {
                    min: 0,
                    max: MAX_ATOM_BYTES,
                },
                RuntimeType::List {
                    item: Some(Box::new(RuntimeType::Record(vec![
                        ("to".to_owned(), id_type()),
                        ("edgeType".to_owned(), id_type()),
                    ]))),
                    max: MAX_OUTGOING_EDGES,
                },
            ],
            Self::ContentEdgeId => vec![id_type(); 4],
            Self::SlotEdgeId => vec![id_type(); 3],
        }
    }
}

/// The type of every identity call's result and of every id operand.
pub(super) fn id_type() -> RuntimeType {
    RuntimeType::Bytes {
        min: ID_BYTES,
        max: ID_BYTES,
    }
}

/// Whether an operand of static type `actual` fits its ceiling. A named type
/// fits through its representation, at any depth.
pub(super) fn fits(ceiling: &RuntimeType, actual: &RuntimeType) -> bool {
    accepts(ceiling, &structural(actual))
}

fn structural(ty: &RuntimeType) -> RuntimeType {
    match ty.representation() {
        RuntimeType::List { item, max } => RuntimeType::List {
            item: item.as_deref().map(|item| Box::new(structural(item))),
            max: *max,
        },
        RuntimeType::Record(fields) => RuntimeType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), structural(ty)))
                .collect(),
        ),
        other => other.clone(),
    }
}

pub(super) fn parse(
    parser: &mut Parser<'_>,
    intrinsic: IdentityIntrinsic,
    args: &[Value],
    types: &[Value],
    depth: usize,
) -> Result<Expr, Error> {
    if !types.is_empty() || args.len() != intrinsic.parameter_types().len() {
        return Err(Error::UnsupportedProgram);
    }
    let args = args
        .iter()
        .map(|value| parser.expr(value, depth + 1))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expr::Identity(intrinsic, args))
}

pub(super) fn evaluate(
    intrinsic: IdentityIntrinsic,
    args: &[Expr],
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    let types = intrinsic.parameter_types();
    if args.len() != types.len() {
        return Err(Error::InvalidArtifact);
    }
    let mut operands = Vec::new();
    operands
        .try_reserve_exact(args.len())
        .map_err(|_| Error::HostAllocationFailed)?;
    for (argument, ty) in args.iter().zip(&types) {
        let value = expression(argument, locals, helpers, meter, depth + 1)?;
        validate(&value, ty, meter, depth + 1)?;
        operands.push(value);
    }
    meter.charge_steps(CALL_STEPS)?;
    let identity = match (intrinsic, operands.as_slice()) {
        (IdentityIntrinsic::ContentAddress, [warp, node_type, atom_type, atom, edges]) => {
            let outgoing = outgoing_edges(atom, edges, meter)?;
            content_node_id(
                WarpId(id(warp)?),
                TypeId(id(node_type)?),
                TypeId(id(atom_type)?),
                bytes(atom)?,
                &outgoing,
            )
            .map_err(|_| Error::DuplicateOutgoingEdge)?
            .0
        }
        (IdentityIntrinsic::ContentEdgeId, [warp, from, edge_type, to]) => {
            content_edge_id(
                WarpId(id(warp)?),
                NodeId(id(from)?),
                TypeId(id(edge_type)?),
                NodeId(id(to)?),
            )
            .0
        }
        (IdentityIntrinsic::SlotEdgeId, [warp, from, edge_type]) => {
            slot_edge_id(WarpId(id(warp)?), NodeId(id(from)?), TypeId(id(edge_type)?)).0
        }
        _ => return Err(Error::InvalidArtifact),
    };
    meter.copy_bytes(&identity)
}

/// Charges the atom and edge work, then collects the validated edges.
fn outgoing_edges(
    atom: &Value,
    edges: &Value,
    meter: &mut Meter,
) -> Result<Vec<OutgoingEdge>, Error> {
    let Value::Array(items) = edges else {
        return Err(Error::InvalidArtifact);
    };
    let atom_bytes = u64::try_from(bytes(atom)?.len()).map_err(|_| Error::StepBudgetExceeded)?;
    let count = u64::try_from(items.len()).map_err(|_| Error::StepBudgetExceeded)?;
    meter.charge_steps(atom_bytes)?;
    meter.charge_steps(
        EDGE_STEPS
            .checked_mul(count)
            .ok_or(Error::StepBudgetExceeded)?,
    )?;
    meter.allocate(
        EDGE_SCRATCH_BYTES
            .checked_mul(count)
            .ok_or(Error::AllocationBudgetExceeded)?,
    )?;
    let mut outgoing = Vec::new();
    outgoing
        .try_reserve_exact(items.len())
        .map_err(|_| Error::HostAllocationFailed)?;
    for item in items {
        outgoing.push(OutgoingEdge {
            edge_type: TypeId(id(field(item, "edgeType")?)?),
            to: NodeId(id(field(item, "to")?)?),
        });
    }
    Ok(outgoing)
}

fn id(value: &Value) -> Result<[u8; 32], Error> {
    bytes(value)?.try_into().map_err(|_| Error::InvalidArtifact)
}
