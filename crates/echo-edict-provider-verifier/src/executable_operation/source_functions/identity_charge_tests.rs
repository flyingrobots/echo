// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Identity constructors called from a helper and from a fold step are
//! charged in the helper's summary, once per iteration (echo#1005).
#![allow(clippy::expect_used, reason = "test code fails loudly")]
use echo_edict_canonical::CanonicalValueV1 as Value;

use super::types::{Charge, Types};
use super::Trail;

const ID: &str = "Bytes<exact=32>";

fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn function(parameter: &Value, result: Value) -> Value {
    map([
        ("params", Value::Array(vec![parameter.clone()])),
        ("returnType", text(ID)),
        (
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", result),
            ]),
        ),
    ])
}

/// `make(id)` folds `step` `bound` times from `id`; `step(id)` returns
/// `echo.dpo@2.slotEdgeId(id, id, id)`.
fn fixture(bound: u64) -> (Value, Value) {
    let parameter = map([
        ("id", text("param.0")),
        ("type", text(ID)),
        ("alphaName", text("id")),
    ]);
    let local = map([("kind", text("local")), ("ref", parameter.clone())]);
    let slot = map([
        ("kind", text("call")),
        ("callee", text("echo.dpo@2.slotEdgeId")),
        (
            "args",
            Value::Array(vec![local.clone(), local.clone(), local.clone()]),
        ),
        ("typeArgs", Value::Array(vec![])),
    ]);
    let fold = map([
        ("kind", text("fold")),
        ("initial", local),
        ("step", text("identity.example@1.step")),
        ("bound", Value::Integer(bound.into())),
        ("stateType", text(ID)),
    ]);
    (
        map([
            ("coordinate", text("identity.example@1")),
            ("types", map([])),
            ("intents", map([])),
            (
                "requiredCoreCapabilities",
                Value::Array(vec![text("core.fold/v1")]),
            ),
            (
                "functions",
                map([
                    ("step", function(&parameter, slot)),
                    ("make", function(&parameter, fold)),
                ]),
            ),
        ]),
        map([
            ("effects", Value::Array(vec![])),
            ("pureFunctions", Value::Array(vec![])),
        ]),
    )
}

/// The summary charge of `make`, or `None` when the helpers are refused.
fn charge(bound: u64, identity_admitted: bool) -> Option<Charge> {
    let (core, exports) = fixture(bound);
    let mut types = Types::new(&core);
    let (_, summaries) = super::prepare(
        &core,
        &exports,
        &mut types,
        0,
        identity_admitted,
        &mut Trail::new(),
    )
    .ok()?;
    summaries
        .iter()
        .find(|(name, _)| name.ends_with("make"))
        .map(|(_, summary)| summary.charge)
}

#[test]
fn a_fold_step_identity_call_is_charged_once_per_iteration() {
    let two = charge(2, true).expect("bound 2 is admitted");
    let three = charge(3, true).expect("bound 3 is admitted");
    // slotEdgeId is documented at 69 ticks and 96 bytes of storage.
    assert!(
        three.ticks >= two.ticks + 69,
        "ticks {} -> {}",
        two.ticks,
        three.ticks
    );
    assert!(
        three.storage >= two.storage + 96,
        "storage {} -> {}",
        two.storage,
        three.storage
    );
    assert!(two.ticks >= 2 * 69, "two iterations {}", two.ticks);
}

#[test]
fn a_helper_identity_call_refuses_without_the_pinned_catalog() {
    assert!(charge(2, false).is_none());
}
