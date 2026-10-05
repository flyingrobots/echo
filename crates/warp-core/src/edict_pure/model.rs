// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::collections::BTreeMap;

use echo_edict_canonical::CanonicalValueV1 as Value;

use super::EvaluationLimits;

pub(crate) const MAX_DEPTH: usize = 64;
pub(crate) const MAX_PROGRAM_NODES: usize = 65_536;
pub(crate) const MAX_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Expr {
    Constant(Value),
    Local(String),
    Field(Box<Expr>, String),
    Record(Vec<(String, Expr)>),
    If(Box<Predicate>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    ByteLength {
        min: u64,
        max: u64,
        value: Box<Expr>,
    },
    ByteSlice {
        min: u64,
        max: u64,
        value: Box<Expr>,
        start: Box<Expr>,
        end: Box<Expr>,
    },
    ByteConcat {
        left_min: u64,
        left_max: u64,
        right_min: u64,
        right_max: u64,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnsignedSubtract {
        max: u64,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Predicate {
    pub op: Comparison,
    pub left: Expr,
    pub right: Expr,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Comparison {
    Equal,
    LessOrEqual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeType {
    Unsigned(u64),
    Bytes { min: u64, max: u64 },
    Record(Vec<(String, RuntimeType)>),
}

pub(crate) struct Binding {
    pub id: String,
    pub ty: RuntimeType,
    pub value: Expr,
}

pub(crate) struct Parameter {
    pub id: String,
    pub ty: RuntimeType,
}

pub(crate) struct Helper {
    pub source_owned: bool,
    pub params: Vec<Parameter>,
    pub bindings: Vec<Binding>,
    pub result: Expr,
    pub ty: RuntimeType,
}

pub(crate) struct Program {
    pub input_id: String,
    pub input_type: RuntimeType,
    pub output_type: RuntimeType,
    pub constraints: Vec<(String, Predicate)>,
    pub bindings: Vec<Binding>,
    pub helpers: BTreeMap<String, Helper>,
    pub result: Expr,
    pub limits: EvaluationLimits,
}
