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

// =============================================================================
// T16: Single-Warp Worker Count Invariance (SPEC-0004)
// =============================================================================

/// T16: Worker count invariance for writer advance.
///
/// This test verifies that when a writer cursor advances (via Engine commit),
/// the resulting `commit_hash` is identical regardless of worker count.
/// This is the "free money" proof for BOAW Phase 6B: parallelism doesn't
/// affect correctness.
///
/// # Test Strategy
///
/// 1. Build a base snapshot with a deterministic graph structure.
/// 2. Create a fixed ingress queue (same intents for all runs).
/// 3. Execute a single tick with different worker pool sizes.
/// 4. Compare `commit_hash` across all runs - must be identical.
///
/// # Why This Matters
///
/// The Engine uses BOAW (Batch of Active Warps) for parallel rule execution.
/// `ECHO_WORKERS` (or `EngineBuilder::workers()`) controls the thread pool size.
/// This test proves that scaling workers doesn't change the deterministic outcome.
#[test]
fn worker_count_invariance_for_writer_advance() {
    use warp_core::{ApplyResult, EngineBuilder, NodeRecord};

    const TOUCH_RULE_NAME: &str = "t16/touch";
    let make_touch_rule = || make_touch_rule!("t16/touch", "t16/marker", b"touched-t16");

    // Build a deterministic base snapshot with 20 independent nodes
    // (mirrors ManyIndependent scenario from BOAW tests)
    let node_ty = warp_core::make_type_id("t16/node");
    let mut base_store = warp_core::GraphStore::default();

    let root = warp_core::make_node_id("t16/root");
    base_store.insert_node(root, NodeRecord { ty: node_ty });

    // Create 19 more independent nodes (total 20)
    let mut all_nodes = vec![root];
    for i in 1..20 {
        let node = warp_core::make_node_id(&format!("t16/node{}", i));
        base_store.insert_node(node, NodeRecord { ty: node_ty });
        all_nodes.push(node);
    }

    // Build fixed ingress: touch all 20 nodes
    let ingress: Vec<(&str, warp_core::NodeId)> = all_nodes
        .iter()
        .map(|&node| (TOUCH_RULE_NAME, node))
        .collect();

    // Run with baseline (1 worker) to establish expected commit_hash
    let baseline_commit_hash = {
        let mut engine = EngineBuilder::new(base_store.clone(), root)
            .workers(1)
            .build();

        engine
            .register_rule(make_touch_rule())
            .expect("failed to register rule");

        let tx = engine.begin();
        for (rule_name, scope) in &ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {}
                Err(e) => panic!("apply error: {:?}", e),
            }
        }

        let (snapshot, _receipt, _patch) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        snapshot.hash
    };

    // Run with each worker count and verify identical commit_hash
    for &workers in WORKER_COUNTS {
        let mut engine = EngineBuilder::new(base_store.clone(), root)
            .workers(workers)
            .build();

        engine
            .register_rule(make_touch_rule())
            .expect("failed to register rule");

        let tx = engine.begin();
        for (rule_name, scope) in &ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {}
                Err(e) => panic!("apply error with {} workers: {:?}", workers, e),
            }
        }

        let (snapshot, _receipt, _patch) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        assert_eq!(
            baseline_commit_hash, snapshot.hash,
            "commit_hash differs for {} workers\n  baseline: {:02x?}\n  got:      {:02x?}",
            workers, baseline_commit_hash, snapshot.hash
        );
    }
}

/// T16 variant: Worker count invariance with shuffled ingress order.
///
/// This test combines worker count invariance with permutation invariance.
/// The ingress order is shuffled before each run, proving that both
/// the order of intents and the number of workers don't affect the result.
#[test]
fn worker_count_invariance_for_writer_advance_shuffled() {
    use warp_core::{ApplyResult, EngineBuilder, NodeRecord};

    // Seeds for deterministic shuffling
    const SHUFFLE_SEEDS: &[u64] = &[0x1234, 0xDEAD, 0xBEEF];

    const TOUCH_RULE_NAME: &str = "t16s/touch";
    let make_touch_rule = || make_touch_rule!("t16s/touch", "t16s/marker", b"touched-t16s");

    // Build deterministic base snapshot
    let node_ty = warp_core::make_type_id("t16s/node");
    let mut base_store = warp_core::GraphStore::default();

    let root = warp_core::make_node_id("t16s/root");
    base_store.insert_node(root, NodeRecord { ty: node_ty });

    let mut all_nodes = vec![root];
    for i in 1..20 {
        let node = warp_core::make_node_id(&format!("t16s/node{}", i));
        base_store.insert_node(node, NodeRecord { ty: node_ty });
        all_nodes.push(node);
    }

    // Baseline ingress (canonical order)
    let canonical_ingress: Vec<(&str, warp_core::NodeId)> = all_nodes
        .iter()
        .map(|&node| (TOUCH_RULE_NAME, node))
        .collect();

    // Get baseline commit_hash with 1 worker, canonical order
    let baseline_commit_hash = {
        let mut engine = EngineBuilder::new(base_store.clone(), root)
            .workers(1)
            .build();

        engine
            .register_rule(make_touch_rule())
            .expect("failed to register rule");

        let tx = engine.begin();
        for (rule_name, scope) in &canonical_ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {}
                Err(e) => panic!("apply error: {:?}", e),
            }
        }

        let (snapshot, _, _) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        snapshot.hash
    };

    // Test with each seed and worker count
    for &seed in SHUFFLE_SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = canonical_ingress.clone();

        // Shuffle ingress order
        shuffle(&mut rng, &mut ingress);

        for &workers in WORKER_COUNTS {
            let mut engine = EngineBuilder::new(base_store.clone(), root)
                .workers(workers)
                .build();

            engine
                .register_rule(make_touch_rule())
                .expect("failed to register rule");

            let tx = engine.begin();
            for (rule_name, scope) in &ingress {
                match engine.apply(tx, rule_name, scope) {
                    Ok(ApplyResult::Applied) => {}
                    Ok(ApplyResult::NoMatch) => {}
                    Err(e) => panic!(
                        "apply error (seed={:#x}, workers={}): {:?}",
                        seed, workers, e
                    ),
                }
            }

            let (snapshot, _, _) = engine
                .commit_with_receipt(tx)
                .expect("commit_with_receipt failed");

            assert_eq!(
                baseline_commit_hash, snapshot.hash,
                "commit_hash differs (seed={:#x}, workers={})\n  baseline: {:02x?}\n  got:      {:02x?}",
                seed, workers, baseline_commit_hash, snapshot.hash
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
