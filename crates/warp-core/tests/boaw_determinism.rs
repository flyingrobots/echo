// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW Determinism Tests (ADR-0007 §1, §5)
//!
//! Tests for snapshot hash invariance, serial vs parallel equivalence,
//! and insertion order independence.

mod common;

use common::{
    assert_hash_eq, boaw_harness, shuffle, BoawScenario, BoawTestHarness, XorShift64, SEEDS,
    WORKER_COUNTS,
};

// =============================================================================
// T1: Snapshot & Hash Determinism (WSC + state_root)
// =============================================================================

#[test]
fn t1_1_snapshot_hash_is_invariant_under_insertion_order() {
    // Given: same logical graph built from ops in different order
    // Expect: identical state_root, identical WSC bytes (or identical segment manifest)
    //
    // - Build base snapshot from a set of nodes/edges/attachments.
    // - Shuffle op order 50 times.
    // - Materialize.
    // - Assert state_root identical across all runs.
    let h = boaw_harness();
    let scenario = BoawScenario::Small;
    let base = h.build_base_snapshot(scenario);
    let tick = 1;

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = h.make_ingress(scenario, tick);

        // First run as baseline
        let r_baseline = h.execute_serial(&base, &ingress, tick);

        for _ in 0..50 {
            shuffle(&mut rng, &mut ingress);
            let r = h.execute_serial(&base, &ingress, tick);

            assert_hash_eq(
                &r_baseline.state_root,
                &r.state_root,
                &format!("T1.1: state_root differs under insertion order (seed={seed:#x})"),
            );
        }
    }
}

#[test]
#[ignore = "BOAW harness not yet wired"]
fn t1_2_zero_copy_read_roundtrip_is_exact() {
    // Given: WSC produced by builder
    // Expect: WarpView sees exactly the same tables (IDs, types, ranges, blobs)
    //
    // - Write WSC
    // - Read via WscFile::from_bytes
    // - Check node_ix, edge_ix, out_edges_for_node, attachment accessors
    // - Verify blob slices match original bytes
    let h = boaw_harness();
    let scenario = BoawScenario::Small;
    let base = h.build_base_snapshot(scenario);
    let tick = 1;

    let ingress = h.make_ingress(scenario, tick);
    let r = h.execute_serial(&base, &ingress, tick);

    if let Some(wsc_bytes) = &r.wsc_bytes {
        let roundtrip_root = h.wsc_roundtrip_state_root(wsc_bytes);
        assert_hash_eq(
            &r.state_root,
            &roundtrip_root,
            "T1.2: WSC roundtrip state_root mismatch",
        );
    } else {
        panic!("T1.2: WSC bytes not produced - SnapshotBuilder not wired");
    }
}

// =============================================================================
// T5: Parallel Execute: Lockless + Deterministic
// =============================================================================

#[test]
fn t5_1_parallel_equals_serial_functional_equivalence() {
    // Given: identical inputs
    // Expect: serial execute and parallel execute produce identical merged ops,
    //         commit hash, state_root
    //
    // - Run execute with 1 worker and N workers
    // - Compare results byte-for-byte
    let h = boaw_harness();
    let scenario = BoawScenario::Small;
    let base = h.build_base_snapshot(scenario);
    let tick = 1;

    let ingress = h.make_ingress(scenario, tick);
    let r0 = h.execute_serial(&base, &ingress, tick);
    let rp = h.execute_parallel(&base, &ingress, tick, 8);

    assert_hash_eq(&r0.state_root, &rp.state_root, "T5.1: state_root differs");
    assert_hash_eq(
        &r0.patch_digest,
        &rp.patch_digest,
        "T5.1: patch_digest differs",
    );
    assert_hash_eq(
        &r0.commit_hash,
        &rp.commit_hash,
        "T5.1: commit_hash differs",
    );
}

#[test]
fn t5_2_permutation_invariance_under_parallelism() {
    // Given: shuffled ingress order + varied worker counts
    // Expect: identical commit hash
    //
    // This is the "determinism drill sergeant" test for BOAW.
    let h = boaw_harness();
    let scenario = BoawScenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);
    let tick = 42;

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = h.make_ingress(scenario, tick);

        // Baseline
        let r_base = h.execute_serial(&base, &ingress, tick);

        for _ in 0..20 {
            shuffle(&mut rng, &mut ingress);

            for &workers in WORKER_COUNTS {
                let r = h.execute_parallel(&base, &ingress, tick, workers);

                assert_hash_eq(
                    &r_base.commit_hash,
                    &r.commit_hash,
                    &format!("T5.2: commit_hash differs (seed={seed:#x}, workers={workers})"),
                );
            }
        }
    }
}

// =============================================================================
// T4: Scheduling & Queues (virtual shards)
// =============================================================================

#[test]
fn t4_2_admission_does_not_depend_on_num_cpus() {
    // Given: same ingress set; run scheduler with worker counts {1,2,8,32}
    // Expect: same admitted set, same patch_digest, same state_root
    //
    // NOTE: We use state_root and patch_digest equality as a proxy for admission
    // invariance because BoawExecResult does not currently expose the admitted set.
    // Once admitted_items is added to ExecuteResult, update this test to directly
    // compare r_baseline.admitted vs r.admitted.
    let h = boaw_harness();
    let scenario = BoawScenario::ManyConflicts;
    let base = h.build_base_snapshot(scenario);
    let tick = 5;

    let ingress = h.make_ingress(scenario, tick);

    // Baseline with 1 worker
    let r_baseline = h.execute_parallel(&base, &ingress, tick, 1);

    for &workers in WORKER_COUNTS {
        let r = h.execute_parallel(&base, &ingress, tick, workers);

        assert_hash_eq(
            &r_baseline.state_root,
            &r.state_root,
            &format!("T4.2: state_root differs for {workers} workers"),
        );
        assert_hash_eq(
            &r_baseline.patch_digest,
            &r.patch_digest,
            &format!("T4.2: patch_digest differs for {workers} workers"),
        );
    }
}
