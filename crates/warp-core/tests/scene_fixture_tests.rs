// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Conformance tests for the benchmark scene fixtures.
//!
//! These run in the ordinary test suite: generator vectors, the separation of
//! workload and expectation identities, C1 parity with `action_scene`, one
//! validated F5-S execution, and validator canaries that must be invalid.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(missing_docs, dead_code, clippy::expect_used, clippy::panic)]

#[path = "../benches/support/action_scene.rs"]
mod action_scene;
#[path = "../benches/support/fixtures/mod.rs"]
mod fixtures;

use fixtures::manifest::hex32;
use fixtures::outcome::ResultClass;
use fixtures::prng::SplitMix64;
use fixtures::{star, think_append};

/// SplitMix64 from seed 1234567: the reference sequence, cross-checked with
/// an independent Python implementation of Vigna's code.
#[test]
fn splitmix64_matches_reference_vectors() {
    let mut rng = SplitMix64::new(1_234_567);
    let expected = [
        6_457_827_717_110_365_317_u64,
        3_203_168_211_198_807_973,
        9_817_491_932_198_370_423,
        4_593_380_528_125_082_431,
        16_408_922_859_458_223_821,
    ];
    for want in expected {
        assert_eq!(rng.next_u64(), want);
    }
}

/// The bounded-integer mapping, pinned separately from the output sequence,
/// including a bound above 2^63 where rejection matters.
#[test]
fn bounded_mapping_matches_vectors() {
    let cases: [(u64, [u64; 6]); 3] = [
        (10, [3, 1, 8, 4, 0, 2]),
        (7, [5, 5, 0, 2, 6, 4]),
        (
            (1 << 63) + 1,
            [
                4_456_085_495_900_499_604,
                6_792_609_088_808_213_253,
                5_545_679_290_133_000_099,
                2_185_608_355_395_893_165,
                247_114_729_376_335_589,
                369_180_215_851_445_686,
            ],
        ),
    ];
    for (bound, want) in cases {
        let mut rng = SplitMix64::new(42);
        let got: Vec<u64> = (0..6)
            .map(|_| rng.below(bound).expect("non-zero bound"))
            .collect();
        assert_eq!(got, want, "bound {bound}");
    }
    assert_eq!(SplitMix64::new(42).below(0), None);
}

#[test]
fn bytes_are_little_endian_outputs_truncated() {
    let first = SplitMix64::new(9).next_u64().to_le_bytes();
    let got = SplitMix64::new(9).bytes(10);
    assert_eq!(got.len(), 10);
    assert_eq!(&got[..8], &first);
}

#[test]
fn goldens_do_not_change_the_workload_identity() {
    let spec = think_append::workload(think_append::S, 7);
    let id = spec.workload_id();
    let a = think_append::expectation(think_append::S, None);
    let b = think_append::expectation(think_append::S, Some("00"));
    assert_eq!(spec.workload_id(), id);
    assert_ne!(a.expectation_id(&id), b.expectation_id(&id));
    assert_ne!(think_append::workload(think_append::S, 8).workload_id(), id);
    assert_ne!(think_append::workload(think_append::M, 7).workload_id(), id);
}

#[test]
fn c1_reproduces_the_action_scene_state_root() {
    for cells in [1_usize, 100, 1_000] {
        let scene = action_scene::Scene::new(
            cells,
            action_scene::Durability::None,
            std::path::Path::new("target"),
        );
        let scene_root = scene
            .host
            .runtime()
            .worldlines()
            .get(&warp_core::WorldlineId::from_bytes([67; 32]))
            .expect("scene worldline")
            .state()
            .state_root();
        assert_eq!(star::state_root(cells as u64), scene_root, "{cells} cells");
    }
}

/// The C1 root at 1,000 cells, under state-root format 1.
const C1_1000_ROOT: &str = "2a2f58e3ba1313c1c21918af58c4ab872dcb4bf871360b1f4f7592f4c2760f5f";

#[test]
fn c1_root_matches_its_golden() {
    let expect = star::expectation(1_000, Some(C1_1000_ROOT));
    assert_eq!(
        Some(hex32(&star::state_root(1_000)).as_str()),
        expect.get("state_root_v1")
    );
}

/// The F5-S script for seed 7, version 1 of the generator.
const F5_S_SEED7_SCRIPT: &str = "efc633360e2e237b9ed238e1cb0f57b8c12e64c779a52bba6e223c9bdfd33d4d";

#[test]
fn f5_script_matches_its_golden() {
    let script = think_append::script(think_append::S, 7);
    assert_eq!(script.len(), 200);
    assert_eq!(
        hex32(&think_append::script_digest(&script)),
        F5_S_SEED7_SCRIPT
    );
}

fn f5_s() -> (
    Vec<think_append::Append>,
    fixtures::manifest::Expectation,
    think_append::Observation,
) {
    let script = think_append::script(think_append::S, 7);
    let expect = think_append::expectation(think_append::S, Some(F5_S_SEED7_SCRIPT));
    let obs = think_append::run(think_append::S, 7);
    (script, expect, obs)
}

#[test]
fn f5_s_executes_validly_and_canaries_are_rejected() {
    let tier = think_append::S;
    let (script, expect, obs) = f5_s();
    assert_eq!(
        think_append::validate(tier, &script, &expect, &obs),
        ResultClass::Valid
    );

    let invalid = |obs: &think_append::Observation| {
        let class = think_append::validate(tier, &script, &expect, obs);
        assert!(!class.comparable(), "canary was accepted: {class:?}");
        class
    };

    // A missing accepted operation.
    let mut canary = obs.clone();
    canary.steps.pop();
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // An incorrect historical reading.
    let mut canary = obs.clone();
    canary.checkpoints[0].2 = Some([0xAB; 32]);
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // Replay that failed.
    let mut canary = obs.clone();
    canary.replay_root = None;
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // A duplicated authored effect: two commits for one occurrence.
    let mut canary = obs.clone();
    let last = *canary.steps.last().expect("steps");
    canary.steps.push(last);
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // An entry missing from the final state.
    let mut canary = obs.clone();
    canary.entries_verified -= 1;
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // Work left unfinished.
    let mut canary = obs.clone();
    canary.accounting.committed -= 1;
    canary.accounting.unfinished = 1;
    assert!(matches!(invalid(&canary), ResultClass::Incomplete(_)));

    // A workload that is not the pinned one.
    let wrong = think_append::expectation(tier, Some("00"));
    assert!(matches!(
        think_append::validate(tier, &script, &wrong, &obs),
        ResultClass::Invalid(_)
    ));
}

#[test]
fn f5_rejects_an_observation_from_a_different_executed_script() {
    let tier = think_append::S;
    let expected_script = think_append::script(tier, 7);
    let other_script = think_append::script(tier, 8);
    assert_ne!(
        think_append::script_digest(&expected_script),
        think_append::script_digest(&other_script)
    );
    let obs = think_append::run(tier, 8);
    let pinned = think_append::expectation(tier, Some(F5_S_SEED7_SCRIPT));
    let unpinned = think_append::expectation(tier, None);
    for expect in [&pinned, &unpinned] {
        let class = think_append::validate(tier, &expected_script, expect, &obs);
        assert!(
            matches!(class, ResultClass::Invalid(_)),
            "accepted an observation from a different script: {class:?}"
        );
    }
    assert_eq!(
        think_append::validate(tier, &other_script, &unpinned, &obs),
        ResultClass::Valid
    );
}
