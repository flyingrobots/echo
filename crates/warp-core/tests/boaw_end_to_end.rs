// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW End-to-End Compliance Tests (ADR-0007)
//!
//! The "god test" that proves determinism across:
//! - Ingress permutations
//! - Worker counts
//! - WSC roundtrip
//!
//! BOAW infrastructure is now wired - these tests verify determinism.

mod common;

use common::{
    assert_hash_eq, boaw_harness, shuffle, BoawScenario, BoawTestHarness, XorShift64, SEEDS,
    WORKER_COUNTS,
};

/// Tick value used for end-to-end determinism tests.
const TEST_TICK: u64 = 42;

/// Number of shuffle iterations per seed to verify permutation invariance.
const SHUFFLE_ITERATIONS: usize = 20;

#[test]
fn boaw_end_to_end_is_deterministic_across_permutations_and_workers() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let tick = TEST_TICK;

        let mut ingress = h.make_ingress(scenario, tick);
        // Permute ingress to prove canonicalization doesn't care about arrival order.
        for _ in 0..SHUFFLE_ITERATIONS {
            shuffle(&mut rng, &mut ingress);

            // Reference run: serial
            let r0 = h.execute_serial(&base, &ingress, tick);

            // Parallel runs: varying worker counts
            for &workers in WORKER_COUNTS {
                let rp = h.execute_parallel(&base, &ingress, tick, workers);

                assert_hash_eq(
                    &r0.state_root,
                    &rp.state_root,
                    &format!("state_root differs (seed={seed:#x}, workers={workers})"),
                );
                assert_hash_eq(
                    &r0.patch_digest,
                    &rp.patch_digest,
                    &format!("patch_digest differs (seed={seed:#x}, workers={workers})"),
                );
                assert_hash_eq(
                    &r0.commit_hash,
                    &rp.commit_hash,
                    &format!("commit_hash differs (seed={seed:#x}, workers={workers})"),
                );

                // If WSC bytes are produced, verify zero-copy roundtrip yields same state_root.
                if let Some(wsc) = &rp.wsc_bytes {
                    let root2 = h.wsc_roundtrip_state_root(wsc);
                    assert_hash_eq(
                        &rp.state_root,
                        &root2,
                        &format!(
                            "WSC roundtrip state_root mismatch (seed={seed:#x}, workers={workers})"
                        ),
                    );
                }
            }
        }
    }
}

#[test]
fn boaw_small_scenario_serial_parallel_equivalence() {
    let h = boaw_harness();
    let scenario = BoawScenario::Small;
    let base = h.build_base_snapshot(scenario);
    let tick = 1;

    let ingress = h.make_ingress(scenario, tick);

    let r_serial = h.execute_serial(&base, &ingress, tick);

    for &workers in WORKER_COUNTS {
        let r_parallel = h.execute_parallel(&base, &ingress, tick, workers);

        assert_hash_eq(
            &r_serial.commit_hash,
            &r_parallel.commit_hash,
            &format!("Small scenario: commit_hash differs for {workers} workers"),
        );
    }
}

#[test]
fn boaw_conflicts_scenario_deterministic_across_permutations() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyConflicts;
    let base = h.build_base_snapshot(scenario);
    let tick = 7;

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = h.make_ingress(scenario, tick);

        // Baseline with serial (1 worker)
        let r_base = h.execute_serial(&base, &ingress, tick);

        for _ in 0..50 {
            shuffle(&mut rng, &mut ingress);

            // Test across multiple worker counts for determinism
            for &workers in WORKER_COUNTS {
                let r = h.execute_parallel(&base, &ingress, tick, workers);

                assert_hash_eq(
                    &r_base.state_root,
                    &r.state_root,
                    &format!("Conflicts scenario: state_root differs (seed={seed:#x}, workers={workers})"),
                );
                assert_hash_eq(
                    &r_base.patch_digest,
                    &r.patch_digest,
                    &format!("Conflicts scenario: patch_digest differs (seed={seed:#x}, workers={workers})"),
                );
                assert_hash_eq(
                    &r_base.commit_hash,
                    &r.commit_hash,
                    &format!("Conflicts scenario: commit_hash differs (seed={seed:#x}, workers={workers})"),
                );
            }
        }
    }
}
