// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::collections::BTreeMap;

use echo_edict_canonical::CanonicalValueV1 as Value;

use super::EvaluationLimits;

pub(super) const MAX_DEPTH: usize = 64;
pub(super) const MAX_PROGRAM_NODES: usize = 65_536;
pub(super) const MAX_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Expr {
    Constant(Value),
    Local(String),
    Field(Box<Expr>, String),
    Record(Vec<(String, Expr)>),
    If(Box<Predicate>, Box<Expr>, Box<Expr>),
    Call(String),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Predicate {
    pub op: Comparison,
    pub left: Expr,
    pub right: Expr,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Comparison {
    Equal,
    LessOrEqual,
}

pub(super) enum RuntimeType {
    Unsigned(u64),
    Bytes { min: u64, max: u64 },
    Record(Vec<(String, RuntimeType)>),
}

pub(super) struct Binding {
    pub id: String,
    pub ty: RuntimeType,
    pub value: Expr,
}

pub(super) struct Helper {
    pub result: Expr,
    pub ty: RuntimeType,
}

pub(super) struct Program {
    pub input_id: String,
    pub input_type: RuntimeType,
    pub output_type: RuntimeType,
    pub constraints: Vec<(String, Predicate)>,
    pub bindings: Vec<Binding>,
    pub helpers: BTreeMap<String, Helper>,
    pub result: Expr,
    pub limits: EvaluationLimits,
}
