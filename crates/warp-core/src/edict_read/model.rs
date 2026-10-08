// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::ReadLimits;
pub(super) use crate::edict_pure::model::MAX_ARTIFACT_BYTES;
use crate::edict_pure::model::{Expr, Helper, Predicate, RuntimeType};
use std::collections::BTreeMap;

pub(super) struct ReadInstruction {
    pub binding: String,
    pub ty: RuntimeType,
    pub input: Expr,
    pub max_bytes: u64,
    pub missing: String,
    pub not_atom: String,
    pub type_mismatch: String,
    pub too_large: String,
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
}
