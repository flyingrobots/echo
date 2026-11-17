// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::ReadLimits;
pub(super) use crate::edict_pure::model::MAX_ARTIFACT_BYTES;
use crate::edict_pure::model::{Expr, Helper, Predicate, RuntimeType};
use std::collections::BTreeMap;

/// The runtime read primitive one lowered step is bound to.
pub(super) enum ReadKind {
    /// `echo.dpo@1.node-atom-read`, bounded by the binding's byte maximum.
    NodeAtom { max_bytes: u64 },
    /// `echo.dpo@1.outgoing-edge-read`, bounded by the binding's list maximum.
    OutgoingEdges { max_edges: u64 },
    /// `echo.dpo@1.closure-read`, bounded by the binding's three list maxima.
    Closure(super::graph::ClosureShape),
}

pub(super) struct ReadInstruction {
    pub binding: String,
    pub ty: RuntimeType,
    pub input: Expr,
    pub kind: ReadKind,
    /// Application obstruction constructor for each runtime failure tag.
    pub arms: BTreeMap<&'static str, String>,
}

pub(super) enum Instruction {
    Read(ReadInstruction),
    Let {
        binding: String,
        ty: RuntimeType,
        value: Expr,
    },
    Require {
        predicate: Predicate,
        obstruction: String,
    },
}

/// Package-declared graph-read ceilings and the bound closure selector.
#[derive(Clone)]
pub(super) struct GraphReads {
    /// Bucket edges inspected per evaluation, matching or not.
    pub max_edge_scans: u64,
    /// Distinct nodes one closure read may materialize.
    pub max_closure_nodes: u64,
    /// Deepest occurrence one closure read may expand to.
    pub max_closure_depth: u64,
    /// Fully qualified source helper deciding each candidate child.
    pub selector: Option<String>,
}

pub(super) struct Program {
    pub input_id: String,
    pub input_type: RuntimeType,
    pub output_type: RuntimeType,
    pub basis: Expr,
    pub constraints: Vec<(String, Predicate)>,
    pub instructions: Vec<Instruction>,
    pub result: Expr,
    pub helpers: BTreeMap<String, Helper>,
    pub limits: ReadLimits,
    pub graph: Option<GraphReads>,
}
