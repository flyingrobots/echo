// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{
    graph::{self, Session, Usage},
    model::{Instruction, Program, ReadInstruction, ReadKind},
    GraphReadObstruction, NodeAtomAccess, ReadError, ReadObstruction, ReadResult, ReadView,
};
use crate::edict_pure::{
    evaluate::{expression, predicate, validate, Meter, VALUE_CELL_BYTES},
    values::{bytes, exact_fields, field},
    EvaluationError as Error,
};
use crate::{NodeId, NodeKey, TypeId, WarpId};
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use std::collections::BTreeMap;

pub(super) fn run(
    program: &Program,
    input: Value,
    view: &ReadView<'_>,
) -> Result<ReadResult, ReadError> {
    let mut access = view;
    run_body(
        program,
        input,
        &mut access,
        &mut Meter::new(program.limits.evaluation),
    )
}

pub(super) fn run_with_meter<A: NodeAtomAccess>(
    program: &Program,
    input: Value,
    access: &mut A,
    meter: &mut Meter,
) -> Result<ReadResult, A::Error> {
    meter.with_component(program.limits.evaluation, |meter| {
        run_body(program, input, access, meter)
    })
}

fn run_body<A: NodeAtomAccess>(
    program: &Program,
    input: Value,
    view: &mut A,
    meter: &mut Meter,
) -> Result<ReadResult, A::Error> {
    validate(&input, &program.input_type, meter, 0).map_err(|error| {
        if error == Error::InvalidArtifact {
            Error::InvalidInput
        } else {
            error
        }
    })?;
    meter.charge(&input, 0)?;
    let mut locals = BTreeMap::from([(program.input_id.as_str(), input)]);
    let helpers = &program.helpers;
    let application_basis = expression(&program.basis, &locals, helpers, meter, 0)?;
    let application_basis = bytes(&application_basis)?
        .try_into()
        .map_err(|_| Error::InvalidInput)?;
    for (coordinate, constraint) in &program.constraints {
        if !predicate(constraint, &locals, helpers, meter, 0)? {
            return Err(Error::InputConstraintFailed(coordinate.clone()).into());
        }
    }
    let mut usage = Usage::default();
    for instruction in &program.instructions {
        meter.step(0)?;
        match instruction {
            Instruction::Read(read) => {
                let input = expression(&read.input, &locals, helpers, meter, 0)?;
                let value = match &read.kind {
                    ReadKind::NodeAtom { max_bytes } => {
                        node_atom(program, read, *max_bytes, &input, view, meter, &mut usage)?
                    }
                    kind => {
                        let mut session = Session {
                            graph: program.graph.as_ref().ok_or(Error::InvalidArtifact)?,
                            helpers,
                            view: &mut *view,
                            meter: &mut *meter,
                            usage: &mut usage,
                            limits: &program.limits,
                        };
                        let outcome = if let ReadKind::Closure(shape) = kind {
                            graph::closure(shape, &input, &mut session)?
                        } else if let ReadKind::OutgoingEdges { max_edges } = kind {
                            graph::edges(*max_edges, &input, &mut session)?
                        } else {
                            return Err(Error::InvalidArtifact.into());
                        };
                        outcome.map_err(|cause| graph_obstructed(read, cause))?
                    }
                };
                validate(&value, &read.ty, meter, 0)?;
                locals.insert(read.binding.as_str(), value);
            }
            Instruction::Let { binding, ty, value } => {
                let value = expression(value, &locals, helpers, meter, 0)?;
                validate(&value, ty, meter, 0)?;
                locals.insert(binding.as_str(), value);
            }
            Instruction::Require {
                predicate: condition,
                obstruction,
            } => {
                if !predicate(condition, &locals, helpers, meter, 0)? {
                    return Err(ReadError::Obstructed {
                        coordinate: obstruction.clone(),
                        cause: ReadObstruction::Guard,
                    }
                    .into());
                }
            }
        }
    }
    let value = expression(&program.result, &locals, helpers, meter, 0)?;
    validate(&value, &program.output_type, meter, 0)?;
    meter.charge(&value, 0)?;
    let output = encode(&value).map_err(|_| Error::InvalidArtifact)?;
    if output.len() as u64 > program.limits.evaluation.max_output_bytes {
        return Err(Error::OutputBudgetExceeded.into());
    }
    let (steps, allocated_bytes) = meter.usage();
    Ok(ReadResult {
        output,
        basis: view.basis(),
        application_basis,
        steps,
        allocated_bytes,
        reads: usage.reads,
        read_bytes: usage.read_bytes,
        edge_scans: usage.edge_scans,
    })
}

fn address_value(value: &Value) -> Result<(NodeKey, TypeId), Error> {
    exact_fields(value, &["nodeId", "typeId", "warpId"])?;
    let id = |key| -> Result<[u8; 32], Error> {
        bytes(field(value, key)?)?
            .try_into()
            .map_err(|_| Error::InvalidArtifact)
    };
    Ok((
        NodeKey {
            warp_id: WarpId(id("warpId")?),
            local_id: NodeId(id("nodeId")?),
        },
        TypeId(id("typeId")?),
    ))
}

fn obstructed(read: &ReadInstruction, cause: ReadObstruction) -> ReadError {
    let tag = match cause {
        ReadObstruction::Missing => "echo.atom-missing/v1",
        ReadObstruction::AtomRequired => "echo.atom-required/v1",
        ReadObstruction::TypeMismatch => "echo.atom-type-mismatch/v1",
        ReadObstruction::AtomTooLarge => "echo.atom-byte-bound-exceeded/v1",
        ReadObstruction::Guard => return Error::InvalidArtifact.into(),
    };
    read.arms.get(tag).map_or_else(
        || Error::InvalidArtifact.into(),
        |coordinate| ReadError::Obstructed {
            coordinate: coordinate.clone(),
            cause,
        },
    )
}

fn graph_obstructed(read: &ReadInstruction, cause: GraphReadObstruction) -> ReadError {
    read.arms.get(cause.runtime_tag()).map_or_else(
        || Error::InvalidArtifact.into(),
        |coordinate| ReadError::GraphObstructed {
            coordinate: coordinate.clone(),
            cause,
        },
    )
}

fn node_atom<A: NodeAtomAccess>(
    program: &Program,
    read: &ReadInstruction,
    max_bytes: u64,
    address: &Value,
    view: &mut A,
    meter: &mut Meter,
    usage: &mut Usage,
) -> Result<Value, A::Error> {
    let (node, expected_type) = address_value(address)?;
    if !view.contains(node) {
        return Err(ReadError::OutsideAperture(node).into());
    }
    usage.attempt(&program.limits)?;
    let bytes = view
        .atom(node, expected_type, max_bytes, meter)?
        .map_err(|cause| obstructed(read, cause))?;
    let count = bytes.len() as u64;
    usage.deliver(count, &program.limits)?;
    // Charge deterministic I/O work and storage BEFORE copying atom bytes.
    meter.charge_steps(count)?;
    meter.allocate(
        VALUE_CELL_BYTES
            .checked_add(count)
            .ok_or(Error::AllocationBudgetExceeded)?,
    )?;
    Ok(meter.copy_precharged_read_bytes(bytes)?)
}
