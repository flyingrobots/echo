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
    Variant {
        identity: String,
        case: String,
        payload: Option<Box<Expr>>,
    },
    Match(Box<Expr>, Vec<MatchArm>),
    Local(String),
    Field(Box<Expr>, String),
    List(Vec<Expr>),
    ListLength {
        ty: RuntimeType,
        value: Box<Expr>,
    },
    ListAt {
        ty: RuntimeType,
        value: Box<Expr>,
        index: Box<Expr>,
    },
    ListAppend {
        ty: RuntimeType,
        value: Box<Expr>,
        item: Box<Expr>,
    },
    ListConcat {
        left_type: Box<RuntimeType>,
        right_type: Box<RuntimeType>,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Record(Vec<(String, Expr)>),
    If(Box<Predicate>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Fold {
        initial: Box<Expr>,
        step: String,
        bound: u64,
        state: RuntimeType,
    },
    ByteUtf8 {
        min: u64,
        max: u64,
        value: Box<Expr>,
    },
    ByteAt {
        min: u64,
        max: u64,
        value: Box<Expr>,
        index: Box<Expr>,
    },
    ByteLength {
        min: u64,
        max: u64,
        value: Box<Expr>,
    },
    ByteCount {
        min: u64,
        max: u64,
        value: Box<Expr>,
        needle: u8,
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
    UnsignedAdd {
        max: u64,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MatchArm {
    pub case: String,
    pub binder: Option<String>,
    pub body: Expr,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Predicate {
    Compare {
        op: Comparison,
        left: Expr,
        right: Expr,
    },
    All(Vec<Predicate>),
    Any(Vec<Predicate>),
    Not(Box<Predicate>),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Comparison {
    Equal,
    LessOrEqual,
    Less,
    Greater,
    GreaterOrEqual,
    NotEqual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeType {
    Bool,
    Unsigned(u64),
    Bytes {
        min: u64,
        max: u64,
    },
    List {
        item: Option<Box<RuntimeType>>,
        max: u64,
    },
    Record(Vec<(String, RuntimeType)>),
    Nominal(String, Box<RuntimeType>),
    Variant {
        identity: String,
        cases: BTreeMap<String, Option<RuntimeType>>,
    },
}

impl RuntimeType {
    /// Nominal identity is static; the wire value uses its representation.
    pub(crate) fn representation(&self) -> &Self {
        let mut ty = self;
        while let Self::Nominal(_, inner) = ty {
            ty = inner;
        }
        ty
    }
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
    pub fold_capability: bool,
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
