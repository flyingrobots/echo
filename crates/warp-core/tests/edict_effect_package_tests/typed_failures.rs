// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Typed function failures (`core.failure/v1`) in authored effect programs.
//!
//! The control is the Core Amblicit emits for
//!
//! ```text
//! failure Underflow;
//! fn remaining(at: U64, size: U64) -> U64 fails Underflow {
//!   return if at <= size then size - at else fail Underflow;
//! }
//! intent update(input: Input) returns Bool ... {
//!   let rest = remaining(input.at, len(input.text)) else RangeInverted { at: input.at };
//!   return if rest <= 64 then true else false;
//! }
//! ```
//!
//! Each forgery changes one thing the three judges must each refuse.
use super::proven_ranges::{at_most, declared, input, len, local, package_editing, Node};
use super::*;

pub(super) const UNDERFLOW: &str = "app.Underflow";
pub(super) const REASON: &str = "app.RangeInverted";

/// One edit of the control that every judge must refuse, or none.
#[derive(Clone, Copy)]
pub(super) enum Forgery {
    /// The control itself.
    None,
    /// The helper declares a failure its body never produces, and not the
    /// one it does; the map follows the declared set.
    Undeclared,
    /// The intent binds the failing call with a plain `let`.
    Unmapped,
    /// The map names a failure the callee does not declare.
    UnknownArm,
    /// The helper's set names its failure twice.
    RepeatedSet,
    /// The helper's set is present but empty.
    EmptySet,
    /// The module omits `core.failure/v1`.
    NoCapability,
    /// The lowered failure arm maps to another reason than Core's.
    ArmsDiffer,
}

pub(super) fn int(value: u64) -> V {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", V::Integer(i128::from(value))),
            ]),
        ),
    ])
}
pub(super) fn boolean(value: bool) -> V {
    map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("bool")), ("value", V::Bool(value))]),
        ),
    ])
}
pub(super) fn call(callee: &str, types: &[&str], args: Vec<V>) -> V {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        (
            "typeArgs",
            V::Array(types.iter().map(|ty| text(ty)).collect()),
        ),
        ("args", V::Array(args)),
    ])
}
pub(super) fn branch(predicate: V, then: V, otherwise: V) -> V {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", then),
        ("else", otherwise),
    ])
}
pub(super) fn fail(failure: &str) -> V {
    map([("kind", text("fail")), ("failure", text(failure))])
}
fn parameter(index: usize, alpha: &str) -> V {
    map([
        ("id", text(&format!("arg.{index}"))),
        ("type", text("U64")),
        ("alphaName", text(alpha)),
    ])
}

/// `remaining`, declaring `failures`.
fn remaining(failures: Option<Vec<V>>) -> V {
    let at = parameter(0, "$at");
    let size = parameter(1, "$size");
    let mut function = map([
        ("params", V::Array(vec![at.clone(), size.clone()])),
        ("returnType", text("U64")),
        (
            "body",
            map([
                ("locals", V::Array(vec![])),
                ("bindings", V::Array(vec![])),
                (
                    "result",
                    branch(
                        at_most(local(&at), local(&size)),
                        call(
                            "core.integer.subtract",
                            &["U64"],
                            vec![local(&size), local(&at)],
                        ),
                        fail(UNDERFLOW),
                    ),
                ),
            ]),
        ),
    ]);
    if let Some(failures) = failures {
        add(&mut function, "failures", V::Array(failures));
    }
    function
}

fn rest() -> V {
    declared("local.rest", "U64")
}

/// The failure map the control's `attempt` carries, naming `failure`.
fn arms(failure: &str, reason: &str) -> V {
    V::Map(vec![(
        text(failure),
        map([
            ("reasonKind", text(reason)),
            ("payload", map([("at", input("at"))])),
        ]),
    )])
}

/// The control, or one forgery of it, with an ample budget.
pub(super) fn typed_failure_program(forgery: Forgery) -> V {
    let declared_failure = match forgery {
        Forgery::Undeclared => "app.Overflow",
        _ => UNDERFLOW,
    };
    let failures = match forgery {
        Forgery::RepeatedSet => vec![text(UNDERFLOW), text(UNDERFLOW)],
        Forgery::EmptySet => vec![],
        _ => vec![text(declared_failure)],
    };
    let value = call("app.remaining", &[], vec![input("at"), len(input("text"))]);
    let mapped = match forgery {
        Forgery::UnknownArm => "app.Overflow",
        _ => declared_failure,
    };
    let node = match forgery {
        Forgery::Unmapped => Node::Let(rest(), value),
        _ => Node::Attempt(rest(), value, arms(mapped, REASON)),
    };
    package_editing(
        vec![],
        vec![node],
        branch(
            at_most(local(&rest()), int(64)),
            boolean(true),
            boolean(false),
        ),
        |core| {
            add(
                core,
                "functions",
                map([("remaining", remaining(Some(failures)))]),
            );
            if !matches!(forgery, Forgery::NoCapability) {
                set(
                    core,
                    "requiredCoreCapabilities",
                    V::Array(vec![text("core.failure/v1")]),
                );
            }
            set(
                value_field_mut(
                    value_field_mut(value_field_mut(core, "intents"), "update@1"),
                    "coreEvaluationBudget",
                ),
                "maxSteps",
                V::Integer(1_000_000),
            );
        },
        |target| {
            if matches!(forgery, Forgery::ArmsDiffer) {
                let V::Array(bindings) = value_field_mut(target, "pureBindings") else {
                    panic!("bindings")
                };
                set(&mut bindings[0], "failureArms", arms(mapped, "app.Other"));
            }
        },
    )
}

/// An input whose `at` is within `text`, and one beyond it.
pub(super) fn typed_failure_input(at: u64) -> Vec<u8> {
    encoded(&map([
        ("at", V::Integer(i128::from(at))),
        ("other", V::Bytes(Vec::new())),
        ("text", V::Bytes(b"abc".to_vec())),
    ]))
}
