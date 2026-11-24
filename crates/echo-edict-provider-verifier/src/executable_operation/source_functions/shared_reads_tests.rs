// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The verifier's charge check against the shared `edict.core-cost/v2` corpus.
//!
//! Oracles: each module's Core budget is the compiler minimum (Amblicit
//! `da1574f1`, `edict.core-cost/v2`), so the verifier must admit it and refuse
//! one step or one byte less. The copying costs are change-detection goldens
//! measured with this audit as it is on `origin/main` at `ed37a03d`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test fixtures fail loudly"
)]
use super::*;
use echo_edict_canonical::decode_canonical_cbor_v1;

const READS: &str =
    include_str!("../../../../warp-core/tests/fixtures/edict-core-cost-v2/reads.core.cbor.hex");
const FILL: &str =
    include_str!("../../../../warp-core/tests/fixtures/edict-core-cost-v2/fill.core.cbor.hex");
const NESTED: &str =
    include_str!("../../../../warp-core/tests/fixtures/edict-core-cost-v2/nested.core.cbor.hex");
const MATCHES: &str =
    include_str!("../../../../warp-core/tests/fixtures/edict-core-cost-v2/matches.core.cbor.hex");
const SHARED: &str = "core.cost.shared-reads/v1";

fn core(hex: &str) -> Value {
    decode_canonical_cbor_v1(&::hex::decode(hex.trim()).unwrap()).unwrap()
}

fn entry<'v>(value: &'v mut Value, key: &str) -> &'v mut Value {
    let Value::Map(fields) = value else {
        panic!("expected a map around {key}")
    };
    &mut fields
        .iter_mut()
        .find(|(name, _)| *name == Value::Text(key.into()))
        .unwrap_or_else(|| panic!("missing {key}"))
        .1
}

fn exports() -> Value {
    Value::Map(vec![
        (Value::Text("effects".into()), Value::Array(vec![])),
        (Value::Text("pureFunctions".into()), Value::Array(vec![])),
    ])
}

/// The corpus module with one taken from one budget axis.
fn below(hex: &str, axis: &str) -> Value {
    let mut core = core(hex);
    let budget = entry(
        entry(entry(&mut core, "intents"), "run"),
        "coreEvaluationBudget",
    );
    let Value::Integer(value) = entry(budget, axis) else {
        panic!("{axis} is an integer")
    };
    *value -= 1;
    core
}

fn copying(mut core: Value) -> Value {
    let Value::Array(capabilities) = entry(&mut core, "requiredCoreCapabilities") else {
        panic!("capabilities are a list")
    };
    capabilities.retain(|capability| *capability != Value::Text(SHARED.into()));
    core
}

fn admitted(core: &Value) -> bool {
    validate(core, &exports(), false, 0).is_ok()
}

/// The judgment admits the compiler minimum and refuses one less on each
/// axis. Output size is schedule-neutral; Echo's bound equals the compiler's.
fn assert_minimum_is_exact(hex: &str) {
    assert!(admitted(&core(hex)), "refused at the compiler minimum");
    assert!(!admitted(&below(hex, "maxSteps")), "admitted one step less");
    assert!(
        !admitted(&below(hex, "maxAllocatedBytes")),
        "admitted one byte less"
    );
    assert!(
        !admitted(&below(hex, "maxOutputBytes")),
        "admitted one output byte less"
    );
}

/// The copying (v1) whole-intent charge of the module without the capability.
fn copying_cost(hex: &str) -> (u128, u128, u128) {
    let core = copying(core(hex));
    let exports = exports();
    let mut measured = None;
    each_intent(&core, &exports, 0, |checker, intent| {
        let (charge, output) = checker.intent_charge(intent, false)?;
        measured = Some((charge.ticks, charge.storage, output));
        Ok(())
    })
    .unwrap();
    measured.unwrap()
}

#[test]
fn shared_reads_of_a_full_list_are_judged_at_the_compiler_minimum() {
    assert_minimum_is_exact(READS);
}

#[test]
fn appends_to_capacity_are_judged_at_the_compiler_minimum() {
    assert_minimum_is_exact(FILL);
}

#[test]
fn nested_list_rebuilding_folds_are_judged_at_the_compiler_minimum() {
    assert_minimum_is_exact(NESTED);
}

#[test]
fn variant_state_with_payload_binders_is_judged_at_the_compiler_minimum() {
    assert_minimum_is_exact(MATCHES);
}

#[test]
fn copying_schedule_judges_the_corpus_exactly_as_before() {
    assert_eq!(copying_cost(READS), (160_635, 7_514_044, 32), "reads");
    assert_eq!(copying_cost(FILL), (131_793, 6_847_227, 1_758), "fill");
    assert_eq!(copying_cost(NESTED), (100_385, 4_567_145, 28), "nested");
    assert_eq!(copying_cost(MATCHES), (50_800, 2_253_602, 28), "matches");
}

#[test]
fn shared_schedule_is_refused_on_the_read_route() {
    // The bounded-read and graph-read charges have no shared-read rules.
    assert!(validate(&core(READS), &exports(), true, 0).is_err());
}

#[test]
fn shared_capability_admits_only_the_pure_inventory() {
    assert!(super::super::capabilities::validate(&core(READS)).is_ok());
    let mut duplicated = core(READS);
    let Value::Array(capabilities) = entry(&mut duplicated, "requiredCoreCapabilities") else {
        panic!("capabilities are a list")
    };
    capabilities.push(Value::Text(SHARED.into()));
    assert!(super::super::capabilities::validate(&duplicated).is_err());
}
