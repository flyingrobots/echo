// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Multi-warp worker-count invariance tests for BOAW Phase 6.
//!
//! These tests verify that execution results are identical regardless of
//! worker count - the "free money" proof that parallelism doesn't affect correctness.
//!
//! # Feature Requirements
//! ```sh
//! cargo test --package warp-core --test boaw_engine_worker_invariance --features delta_validate
//! ```

mod common;

use common::{
    assert_hash_eq, boaw_harness, shuffle, BoawScenario, BoawTestHarness, XorShift64, SEEDS,
    WORKER_COUNTS,
};

// =============================================================================
// T4.1: Multi-warp Worker-Count Invariance
// =============================================================================

/// T4.1.1: Worker count invariance for independent workloads.
///
/// Verifies that execution with any worker count in [1, 2, 4, 8, 16, 32]
/// produces identical hashes for the ManyIndependent scenario.
///
/// This is the "free money" proof: if parallelism doesn't change the result,
/// we can scale workers without affecting correctness.
#[test]
fn multiwarp_worker_count_invariance() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);
    let ingress = h.make_ingress(scenario, 42);

    // Baseline: single worker execution
    let r_baseline = h.execute_parallel(&base, &ingress, 42, 1);

    for &workers in WORKER_COUNTS {
        let r = h.execute_parallel(&base, &ingress, 42, workers);

        assert_hash_eq(
            &r_baseline.commit_hash,
            &r.commit_hash,
            &format!("commit_hash differs for {workers} workers"),
        );
        assert_hash_eq(
            &r_baseline.state_root,
            &r.state_root,
            &format!("state_root differs for {workers} workers"),
        );
        assert_hash_eq(
            &r_baseline.patch_digest,
            &r.patch_digest,
            &format!("patch_digest differs for {workers} workers"),
        );
    }
}

/// T4.1.2: Worker count invariance with conflict-heavy workloads.
///
/// Uses the ManyConflicts scenario where warps share attachments and edges.
/// This tests that the admission/rejection ordering is deterministic
/// regardless of how many workers are racing to process warps.
#[test]
fn multiwarp_worker_count_invariance_with_conflicts() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyConflicts;
    let base = h.build_base_snapshot(scenario);
    let ingress = h.make_ingress(scenario, 42);

    // Baseline: single worker execution
    let r_baseline = h.execute_parallel(&base, &ingress, 42, 1);

    for &workers in WORKER_COUNTS {
        let r = h.execute_parallel(&base, &ingress, 42, workers);

        assert_hash_eq(
            &r_baseline.commit_hash,
            &r.commit_hash,
            &format!("commit_hash differs for {workers} workers (ManyConflicts)"),
        );
        assert_hash_eq(
            &r_baseline.state_root,
            &r.state_root,
            &format!("state_root differs for {workers} workers (ManyConflicts)"),
        );
        assert_hash_eq(
            &r_baseline.patch_digest,
            &r.patch_digest,
            &format!("patch_digest differs for {workers} workers (ManyConflicts)"),
        );
    }
}

/// T4.1.3: Worker count invariance under permuted ingress order.
///
/// Shuffles the ingress order before each worker-count test run.
/// This combines two invariance properties:
/// 1. Permutation invariance (order doesn't matter)
/// 2. Worker count invariance (parallelism doesn't matter)
///
/// If both hold, we have strong evidence of deterministic execution.
#[test]
fn multiwarp_worker_count_invariance_permuted() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);

    // Establish baseline with canonical ingress order, single worker
    let canonical_ingress = h.make_ingress(scenario, 42);
    let r_baseline = h.execute_parallel(&base, &canonical_ingress, 42, 1);

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = h.make_ingress(scenario, 42);

        // Shuffle ingress for this seed
        shuffle(&mut rng, &mut ingress);

        for &workers in WORKER_COUNTS {
            let r = h.execute_parallel(&base, &ingress, 42, workers);

            assert_hash_eq(
                &r_baseline.commit_hash,
                &r.commit_hash,
                &format!("commit_hash differs (seed={seed:#x}, workers={workers})"),
            );
            assert_hash_eq(
                &r_baseline.state_root,
                &r.state_root,
                &format!("state_root differs (seed={seed:#x}, workers={workers})"),
            );
            assert_hash_eq(
                &r_baseline.patch_digest,
                &r.patch_digest,
                &format!("patch_digest differs (seed={seed:#x}, workers={workers})"),
            );
        }
    }
}

// =============================================================================
// Extended Invariance Tests
// =============================================================================

/// Worker count invariance across all scenarios.
///
/// Iterates through all BOAW scenarios to ensure worker count invariance
/// holds universally, not just for specific workload patterns.
#[test]
fn multiwarp_worker_count_invariance_all_scenarios() {
    let h = boaw_harness();

    let scenarios = [
        BoawScenario::Small,
        BoawScenario::ManyIndependent,
        BoawScenario::ManyConflicts,
        BoawScenario::DeletesAndAttachments,
        BoawScenario::PrivacyClaims,
    ];

    for scenario in scenarios {
        let base = h.build_base_snapshot(scenario);
        let ingress = h.make_ingress(scenario, 42);

        // Baseline: single worker
        let r_baseline = h.execute_parallel(&base, &ingress, 42, 1);

        for &workers in WORKER_COUNTS {
            let r = h.execute_parallel(&base, &ingress, 42, workers);

            assert_hash_eq(
                &r_baseline.commit_hash,
                &r.commit_hash,
                &format!("commit_hash differs for {scenario:?} with {workers} workers"),
            );
            assert_hash_eq(
                &r_baseline.state_root,
                &r.state_root,
                &format!("state_root differs for {scenario:?} with {workers} workers"),
            );
            assert_hash_eq(
                &r_baseline.patch_digest,
                &r.patch_digest,
                &format!("patch_digest differs for {scenario:?} with {workers} workers"),
            );
        }
    }
}

/// Worker count invariance across multiple ticks.
///
/// Verifies that invariance holds not just at tick 42, but across
/// a range of tick values. This catches any tick-dependent ordering issues.
#[test]
fn multiwarp_worker_count_invariance_multi_tick() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);

    let ticks: &[u64] = &[0, 1, 42, 100, 1000, u64::MAX];

    for &tick in ticks {
        let ingress = h.make_ingress(scenario, tick);

        // Baseline: single worker
        let r_baseline = h.execute_parallel(&base, &ingress, tick, 1);

        for &workers in WORKER_COUNTS {
            let r = h.execute_parallel(&base, &ingress, tick, workers);

            assert_hash_eq(
                &r_baseline.commit_hash,
                &r.commit_hash,
                &format!("commit_hash differs at tick={tick} with {workers} workers"),
            );
            assert_hash_eq(
                &r_baseline.state_root,
                &r.state_root,
                &format!("state_root differs at tick={tick} with {workers} workers"),
            );
            assert_hash_eq(
                &r_baseline.patch_digest,
                &r.patch_digest,
                &format!("patch_digest differs at tick={tick} with {workers} workers"),
            );
        }
    }
}

/// Repeated execution with same worker count produces identical results.
///
/// This test catches any non-determinism that might arise from thread
/// scheduling, memory allocation patterns, or other runtime variations.
#[test]
fn multiwarp_repeated_execution_determinism() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);
    let ingress = h.make_ingress(scenario, 42);

    for &workers in WORKER_COUNTS {
        // First execution establishes baseline for this worker count
        let r_first = h.execute_parallel(&base, &ingress, 42, workers);

        // Repeat 10 times and verify identical results
        for run in 1..=10 {
            let r = h.execute_parallel(&base, &ingress, 42, workers);

            assert_hash_eq(
                &r_first.commit_hash,
                &r.commit_hash,
                &format!("commit_hash differs on run {run} with {workers} workers"),
            );
            assert_hash_eq(
                &r_first.state_root,
                &r.state_root,
                &format!("state_root differs on run {run} with {workers} workers"),
            );
            assert_hash_eq(
                &r_first.patch_digest,
                &r.patch_digest,
                &format!("patch_digest differs on run {run} with {workers} workers"),
            );
        }
    }
}

/// Stress test: deep permutation drill with all worker counts.
///
/// Combines multiple seeds, multiple permutations per seed, and all worker
/// counts. This is the "drill sergeant" test that should catch subtle
/// ordering bugs that only manifest under specific conditions.
#[test]
fn multiwarp_worker_count_invariance_stress() {
    let h = boaw_harness();
    let scenario = BoawScenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);

    // Establish canonical baseline
    let canonical_ingress = h.make_ingress(scenario, 42);
    let r_canonical = h.execute_parallel(&base, &canonical_ingress, 42, 1);

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = h.make_ingress(scenario, 42);

        // 20 permutations per seed
        for perm in 0..20 {
            shuffle(&mut rng, &mut ingress);

            for &workers in WORKER_COUNTS {
                let r = h.execute_parallel(&base, &ingress, 42, workers);

                assert_hash_eq(
                    &r_canonical.commit_hash,
                    &r.commit_hash,
                    &format!(
                        "commit_hash differs (seed={seed:#x}, perm={perm}, workers={workers})"
                    ),
                );
                assert_hash_eq(
                    &r_canonical.state_root,
                    &r.state_root,
                    &format!("state_root differs (seed={seed:#x}, perm={perm}, workers={workers})"),
                );
                assert_hash_eq(
                    &r_canonical.patch_digest,
                    &r.patch_digest,
                    &format!(
                        "patch_digest differs (seed={seed:#x}, perm={perm}, workers={workers})"
                    ),
                );
            }
        }
    }
}
