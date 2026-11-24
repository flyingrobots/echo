// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Core ordered records (`kind: "orderedRecord"`) in authored effect programs.
//!
//! The controls are the Core Amblicit emits for
//!
//! ```text
//! failure First;
//! failure Second;
//! fn pair(at: U64) -> { a: U64, z: U64 } fails First, Second {
//!   return { z: fail First, a: fail Second };          // `Literal`
//!   return { z: if at <= 9 then 1 else fail First,     // `Guarded`
//!            a: if at <= 4 then 2 else fail Second };
//! }
//! intent update(input: Input) returns Bool ... {
//!   let pair = pair(input.at) else {
//!     First => FirstRaised { at: input.at },
//!     Second => SecondRaised { at: input.at },
//!   };
//!   return if pair.z <= 1 then true else false;
//! }
//! ```
//!
//! and the same records with their fields authored in the other order. The
//! first field to fail decides the obstruction. Each forgery changes one
//! thing the three judges must each refuse.
use super::proven_ranges::{at_most, declared, input, local, package_editing, Node};
use super::typed_failures::{boolean, branch, call, fail, int, parameter};
use super::*;

pub(super) const FIRST: &str = "app.First";
pub(super) const SECOND: &str = "app.Second";
pub(super) const FIRST_RAISED: &str = "app.FirstRaised";
pub(super) const SECOND_RAISED: &str = "app.SecondRaised";
const PAIR: &str = "Record<a:U64,z:U64>";

/// Which field the record evaluates first.
#[derive(Clone, Copy)]
pub(super) enum Order {
    /// `{ z: ..., a: ... }`: source order differs from canonical key order.
    ZFirst,
    /// `{ a: ..., z: ... }`.
    AFirst,
}

/// The helper's record, or one forgery of it.
#[derive(Clone, Copy)]
pub(super) enum Shape {
    /// Both fields always fail.
    Literal(Order),
    /// Each field returns below its bound and fails above it.
    Guarded(Order),
    /// The ordered record names `z` twice.
    DuplicateName,
    /// The ordered record has no fields.
    Empty,
    /// No ordered field can fail; Core spells such a record as `record`.
    Total,
    /// The fallible fields are spelled as the unordered `record` map.
    Unordered,
    /// One field entry carries a key beside `name` and `value`.
    ExtraEntryKey,
}

fn entry(name: &str, value: V) -> V {
    map([("name", text(name)), ("value", value)])
}
fn ordered(entries: Vec<V>) -> V {
    map([
        ("kind", text("orderedRecord")),
        ("fields", V::Array(entries)),
    ])
}
fn at() -> V {
    local(&parameter(0, "$at"))
}
fn guarded(bound: u64, value: u64, failure: &str) -> V {
    branch(at_most(at(), int(bound)), int(value), fail(failure))
}
fn in_order(order: Order, z: V, a: V) -> Vec<V> {
    match order {
        Order::ZFirst => vec![entry("z", z), entry("a", a)],
        Order::AFirst => vec![entry("a", a), entry("z", z)],
    }
}

/// The helper result for `shape`, and the failures it declares.
fn result(shape: Shape) -> (V, Vec<&'static str>) {
    let both = vec![FIRST, SECOND];
    match shape {
        Shape::Literal(order) => (ordered(in_order(order, fail(FIRST), fail(SECOND))), both),
        Shape::Guarded(order) => (
            ordered(in_order(order, guarded(9, 1, FIRST), guarded(4, 2, SECOND))),
            both,
        ),
        Shape::DuplicateName => (
            ordered(vec![entry("z", fail(FIRST)), entry("z", fail(SECOND))]),
            both,
        ),
        Shape::Empty => (ordered(vec![]), both),
        Shape::Total => (
            branch(
                at_most(at(), int(9)),
                ordered(vec![entry("z", int(1)), entry("a", int(2))]),
                fail(FIRST),
            ),
            vec![FIRST],
        ),
        Shape::Unordered => (
            map([
                ("kind", text("record")),
                ("fields", map([("a", fail(SECOND)), ("z", fail(FIRST))])),
            ]),
            both,
        ),
        Shape::ExtraEntryKey => {
            let mut first = entry("z", fail(FIRST));
            add(&mut first, "order", V::Integer(0));
            (ordered(vec![first, entry("a", fail(SECOND))]), both)
        }
    }
}

fn helper(shape: Shape) -> V {
    let (result, failures) = result(shape);
    map([
        ("params", V::Array(vec![parameter(0, "$at")])),
        ("returnType", text(PAIR)),
        (
            "body",
            map([
                ("locals", V::Array(vec![])),
                ("bindings", V::Array(vec![])),
                ("result", result),
            ]),
        ),
        (
            "failures",
            V::Array(failures.iter().map(|failure| text(failure)).collect()),
        ),
    ])
}

fn arm(reason: &str) -> V {
    map([
        ("reasonKind", text(reason)),
        ("payload", map([("at", input("at"))])),
    ])
}

/// The control's failure map, covering exactly the helper's set.
fn arms(shape: Shape) -> V {
    let mut arms = vec![(text(FIRST), arm(FIRST_RAISED))];
    if !matches!(shape, Shape::Total) {
        arms.push((text(SECOND), arm(SECOND_RAISED)));
    }
    V::Map(arms)
}

/// The program around `shape`, with an ample budget.
pub(super) fn ordered_record_program(shape: Shape) -> V {
    let pair = declared("local.pair", PAIR);
    let field = map([
        ("kind", text("field")),
        ("base", local(&pair)),
        ("field", text("z")),
    ]);
    package_editing(
        vec![],
        vec![Node::Attempt(
            pair,
            call("app.pair", &[], vec![input("at")]),
            arms(shape),
        )],
        branch(at_most(field, int(1)), boolean(true), boolean(false)),
        |core| {
            add(core, "functions", map([("pair", helper(shape))]));
            set(
                core,
                "requiredCoreCapabilities",
                V::Array(vec![text("core.failure/v1")]),
            );
            set(
                value_field_mut(
                    value_field_mut(value_field_mut(core, "intents"), "update@1"),
                    "coreEvaluationBudget",
                ),
                "maxSteps",
                V::Integer(1_000_000),
            );
        },
        |_| {},
    )
}

/// An input whose `at` is the given value.
pub(super) fn ordered_record_input(at: u64) -> Vec<u8> {
    encoded(&map([
        ("at", V::Integer(i128::from(at))),
        ("other", V::Bytes(Vec::new())),
        ("text", V::Bytes(Vec::new())),
    ]))
}
