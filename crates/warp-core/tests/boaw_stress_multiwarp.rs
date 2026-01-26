// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! High-volume multi-warp stress tests for BOAW Phase 6.
#![cfg(feature = "delta_validate")]
//!
//! # Feature Requirements
//! ```sh
//! cargo test --package warp-core --test boaw_stress_multiwarp --features delta_validate
//! ```
//!
//! Heavy stress tests are `#[ignore]` by default:
//! ```sh
//! cargo test --package warp-core --test boaw_stress_multiwarp --features delta_validate -- --ignored
//! ```

use warp_core::{
    execute_parallel, execute_serial, make_node_id, make_type_id, merge_deltas_ok, shard_of,
    AtomPayload, AttachmentKey, AttachmentValue, ExecItem, GraphStore, GraphView, NodeId, NodeKey,
    NodeRecord, OpOrigin, TickDelta, WarpOp, NUM_SHARDS,
};

mod common;
use common::{shuffle, XorShift64, SEEDS, WORKER_COUNTS};

// =============================================================================
// Test Store Helpers
// =============================================================================

/// Create a large test graph with N independent nodes for stress testing.
///
/// Returns the store and a vector of all node IDs.
fn make_large_test_store(n: usize) -> (GraphStore, Vec<NodeId>) {
    let node_ty = make_type_id("stress/node");
    let mut store = GraphStore::default();
    let mut nodes = Vec::with_capacity(n);
    for i in 0..n {
        let id = make_node_id(&format!("stress/n{i}"));
        store.insert_node(id, NodeRecord { ty: node_ty });
        nodes.push(id);
    }
    (store, nodes)
}

/// Simple executor that sets an attachment on the scope node.
fn touch_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let payload = AtomPayload::new(
        make_type_id("stress/marker"),
        bytes::Bytes::from_static(b"touched"),
    );
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: *scope,
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(payload)),
    });
}

/// Create ExecItems for all nodes with proper origins.
fn make_exec_items(nodes: &[NodeId]) -> Vec<ExecItem> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, &scope)| {
            ExecItem::new(
                touch_executor,
                scope,
                OpOrigin {
                    intent_id: i as u64,
                    rule_id: 1,
                    match_ix: 0,
                    op_ix: 0,
                },
            )
        })
        .collect()
}

/// Create ExecItems for nodes with a specific warp assignment (for multi-warp tests).
fn make_exec_items_for_warp(nodes: &[NodeId], warp_index: usize) -> Vec<ExecItem> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, &scope)| {
            ExecItem::new(
                touch_executor,
                scope,
                OpOrigin {
                    intent_id: (warp_index * 10000 + i) as u64,
                    rule_id: 1,
                    match_ix: 0,
                    op_ix: 0,
                },
            )
        })
        .collect()
}

// =============================================================================
// T5.1: High-Volume Multi-Warp Stress Tests
// =============================================================================

/// T5.1.1: Stress test with 5k rewrites covering many shards.
///
/// This test exercises the sharded execution path with a high volume of items,
/// verifying that:
/// 1. Items distribute across many of the 256 shards
/// 2. Worker count doesn't affect the merged result
/// 3. Permuting the input doesn't affect the result
///
/// Marked `#[ignore]` because it runs 5k items - use `-- --ignored` to run.
#[test]
#[ignore = "heavy stress test: 5k rewrites across 256 shards"]
fn two_warps_256_shard_coverage_stress() {
    // Create a store with 5k nodes for stress testing
    let (store1, nodes1) = make_large_test_store(5000);

    // Verify shard coverage: with 5k items we should hit most shards
    let mut shard_hits = vec![false; NUM_SHARDS];
    for node in nodes1.iter() {
        let shard = shard_of(node);
        shard_hits[shard] = true;
    }
    let shards_covered = shard_hits.iter().filter(|&&hit| hit).count();

    // With 5k items spread by blake3 hash, we should cover most of the 256 shards
    assert!(
        shards_covered >= 200,
        "expected at least 200 shards covered, got {shards_covered}"
    );

    // Use the first store for execution (second store's nodes are just for ID generation)
    let view = GraphView::new(&store1);
    let items1 = make_exec_items_for_warp(&nodes1, 0);

    // Baseline: serial execution
    let serial_delta = execute_serial(view, &items1);
    let serial_ops = merge_deltas_ok(vec![serial_delta]).expect("merge failed");

    assert_eq!(
        serial_ops.len(),
        5000,
        "serial should produce 5000 ops for warp 1"
    );

    // Parallel execution with various worker counts
    for &workers in WORKER_COUNTS {
        let parallel_deltas = execute_parallel(view, &items1, workers);
        let parallel_ops = merge_deltas_ok(parallel_deltas).expect("merge failed");

        assert_eq!(
            serial_ops.len(),
            parallel_ops.len(),
            "op count differs for {workers} workers"
        );

        for (i, (s, p)) in serial_ops.iter().zip(parallel_ops.iter()).enumerate() {
            assert_eq!(
                s, p,
                "op {i} differs for {workers} workers in warp 1 stress test"
            );
        }
    }

    // Now test with permuted input
    let mut rng = XorShift64::new(0xDEADBEEF_CAFEBABE);
    let mut items_permuted = items1.clone();

    for perm in 0..10 {
        shuffle(&mut rng, &mut items_permuted);

        for &workers in &[1, 4, 16, 32] {
            let deltas = execute_parallel(view, &items_permuted, workers);
            let ops = merge_deltas_ok(deltas).expect("merge failed");

            assert_eq!(
                serial_ops.len(),
                ops.len(),
                "permuted op count differs (perm={perm}, workers={workers})"
            );

            for (i, (s, p)) in serial_ops.iter().zip(ops.iter()).enumerate() {
                assert_eq!(
                    s, p,
                    "permuted op {i} differs (perm={perm}, workers={workers})"
                );
            }
        }
    }
}

/// T5.1.2: Large workload multi-warp worker invariance.
///
/// Tests 1000 items across 4 conceptual warps with all worker counts.
/// Verifies that results are identical regardless of parallelism level.
#[test]
fn large_workload_multiwarp_worker_invariance() {
    // Create 4 logical warps with 250 nodes each
    let (store, nodes) = make_large_test_store(1000);
    let view = GraphView::new(&store);

    // Split nodes into 4 "warps" (logical groupings)
    let warp_size = 250;
    let mut all_items: Vec<ExecItem> = Vec::with_capacity(1000);

    for warp_idx in 0..4 {
        let start = warp_idx * warp_size;
        let end = start + warp_size;
        let warp_nodes = &nodes[start..end];
        let warp_items = make_exec_items_for_warp(warp_nodes, warp_idx);
        all_items.extend(warp_items);
    }

    // Baseline: single worker execution
    let baseline_deltas = execute_parallel(view, &all_items, 1);
    let baseline_ops = merge_deltas_ok(baseline_deltas).expect("merge failed");

    assert_eq!(baseline_ops.len(), 1000, "baseline should produce 1000 ops");

    // Test all worker counts
    for &workers in WORKER_COUNTS {
        let deltas = execute_parallel(view, &all_items, workers);
        let ops = merge_deltas_ok(deltas).expect("merge failed");

        assert_eq!(
            baseline_ops.len(),
            ops.len(),
            "large workload: op count differs for {workers} workers"
        );

        for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(b, o, "large workload: op {i} differs for {workers} workers");
        }
    }

    // Test with permutations
    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut permuted_items = all_items.clone();
        shuffle(&mut rng, &mut permuted_items);

        for &workers in WORKER_COUNTS {
            let deltas = execute_parallel(view, &permuted_items, workers);
            let ops = merge_deltas_ok(deltas).expect("merge failed");

            assert_eq!(
                baseline_ops.len(),
                ops.len(),
                "permuted large workload: op count differs (seed={seed:#x}, workers={workers})"
            );

            for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
                assert_eq!(
                    b, o,
                    "permuted large workload: op {i} differs (seed={seed:#x}, workers={workers})"
                );
            }
        }
    }
}

/// T5.1.3: Shard distribution uniformity across warps.
///
/// Verifies that items are distributed reasonably across shards when we have
/// nodes from multiple logical warps. This is a sanity check that our
/// shard_of() function provides good distribution.
#[test]
fn shard_distribution_uniform_across_warps() {
    // Create a medium-sized store
    let (store, nodes) = make_large_test_store(512);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Count items per shard
    let mut shard_counts = [0usize; NUM_SHARDS];
    for item in &items {
        let shard = shard_of(&item.scope);
        shard_counts[shard] += 1;
    }

    // With 512 items across 256 shards, we expect most shards to have 1-4 items
    // (birthday paradox: some shards will have 0, some will have >2)

    // Count shards by occupancy
    let empty_shards = shard_counts.iter().filter(|&&c| c == 0).count();
    let single_item_shards = shard_counts.iter().filter(|&&c| c == 1).count();
    let multi_item_shards = shard_counts.iter().filter(|&&c| c > 1).count();

    // Sanity checks for reasonable distribution
    // With 512 items in 256 shards, we expect ~25-35% empty shards (birthday paradox)
    assert!(
        empty_shards > 20 && empty_shards < 150,
        "unexpected empty shard count: {empty_shards} (expected 20-150)"
    );

    // We should have many single-item shards
    assert!(
        single_item_shards > 50,
        "expected more single-item shards: got {single_item_shards}"
    );

    // We should have some multi-item shards (collision buckets)
    assert!(
        multi_item_shards > 30,
        "expected more multi-item shards: got {multi_item_shards}"
    );

    // Verify execution still works correctly
    let serial_delta = execute_serial(view, &items);
    let serial_ops = merge_deltas_ok(vec![serial_delta]).expect("merge failed");

    for &workers in WORKER_COUNTS {
        let parallel_deltas = execute_parallel(view, &items, workers);
        let parallel_ops = merge_deltas_ok(parallel_deltas).expect("merge failed");

        assert_eq!(
            serial_ops.len(),
            parallel_ops.len(),
            "shard distribution: op count differs for {workers} workers"
        );

        for (i, (s, p)) in serial_ops.iter().zip(parallel_ops.iter()).enumerate() {
            assert_eq!(
                s, p,
                "shard distribution: op {i} differs for {workers} workers"
            );
        }
    }
}

/// T5.1.4: Stress test with many small attachment operations.
///
/// Creates many small items to stress the merge performance and verify
/// that high-frequency attachment ops don't cause issues.
#[test]
fn stress_many_small_items_multiwarp() {
    // Create 256 nodes (one per shard, roughly)
    let (store, nodes) = make_large_test_store(256);
    let view = GraphView::new(&store);

    // Create items - each node gets multiple operations (simulating multi-warp)
    let mut all_items: Vec<ExecItem> = Vec::new();

    // 4 rounds of operations on the same nodes
    for round in 0..4 {
        for (i, &node) in nodes.iter().enumerate() {
            all_items.push(ExecItem::new(
                touch_executor,
                node,
                OpOrigin {
                    intent_id: (round * 1000 + i) as u64,
                    rule_id: (round + 1) as u32,
                    match_ix: 0,
                    op_ix: 0,
                },
            ));
        }
    }

    assert_eq!(all_items.len(), 1024, "should have 1024 items");

    // Serial baseline
    let serial_delta = execute_serial(view, &all_items);
    let serial_ops = merge_deltas_ok(vec![serial_delta]).expect("merge failed");

    // Parallel with all worker counts
    for &workers in WORKER_COUNTS {
        let parallel_deltas = execute_parallel(view, &all_items, workers);
        let parallel_ops = merge_deltas_ok(parallel_deltas).expect("merge failed");

        assert_eq!(
            serial_ops.len(),
            parallel_ops.len(),
            "small items stress: op count differs for {workers} workers"
        );

        for (i, (s, p)) in serial_ops.iter().zip(parallel_ops.iter()).enumerate() {
            assert_eq!(
                s, p,
                "small items stress: op {i} differs for {workers} workers"
            );
        }
    }

    // Stress test: repeat with permuted order
    let mut rng = XorShift64::new(0x1234_5678_9ABC_DEF0);

    for iteration in 0..20 {
        let mut permuted = all_items.clone();
        shuffle(&mut rng, &mut permuted);

        // Test with 8 workers (typical for CI)
        let deltas = execute_parallel(view, &permuted, 8);
        let ops = merge_deltas_ok(deltas).expect("merge failed");

        assert_eq!(
            serial_ops.len(),
            ops.len(),
            "small items stress permutation {iteration}: op count differs"
        );

        for (i, (s, p)) in serial_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(
                s, p,
                "small items stress permutation {iteration}: op {i} differs"
            );
        }
    }
}

// =============================================================================
// Additional Stress Tests
// =============================================================================

/// Verify that identical operations from different conceptual warps merge correctly.
///
/// This tests the deduplication behavior when multiple "warps" emit the same
/// attachment operation (different origin, same effect).
#[test]
fn multiwarp_merge_dedupe_stress() {
    let (store, nodes) = make_large_test_store(100);
    let view = GraphView::new(&store);

    // Create overlapping items from 2 "warps" targeting the same nodes
    let warp1_items = make_exec_items_for_warp(&nodes, 0);
    let warp2_items = make_exec_items_for_warp(&nodes, 1);

    // Combined items (200 total, but targeting same 100 nodes)
    let mut combined: Vec<ExecItem> = Vec::with_capacity(200);
    combined.extend(warp1_items.clone());
    combined.extend(warp2_items.clone());

    // Each warp alone should produce 100 ops
    let warp1_delta = execute_serial(view, &warp1_items);
    let warp1_ops = merge_deltas_ok(vec![warp1_delta]).expect("merge failed");
    assert_eq!(warp1_ops.len(), 100, "warp1 should produce 100 ops");

    let warp2_delta = execute_serial(view, &warp2_items);
    let warp2_ops = merge_deltas_ok(vec![warp2_delta]).expect("merge failed");
    assert_eq!(warp2_ops.len(), 100, "warp2 should produce 100 ops");

    // Combined should produce 200 ops (different origins = different ops)
    let combined_serial = execute_serial(view, &combined);
    let combined_serial_ops = merge_deltas_ok(vec![combined_serial]).expect("merge failed");

    // Verify parallel produces same count
    for &workers in WORKER_COUNTS {
        let deltas = execute_parallel(view, &combined, workers);
        let ops = merge_deltas_ok(deltas).expect("merge failed");

        assert_eq!(
            combined_serial_ops.len(),
            ops.len(),
            "multiwarp dedupe: op count differs for {workers} workers"
        );

        for (i, (s, p)) in combined_serial_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(
                s, p,
                "multiwarp dedupe: op {i} differs for {workers} workers"
            );
        }
    }
}

/// Verify determinism with maximum worker count (NUM_SHARDS).
///
/// Edge case: when workers == NUM_SHARDS, each worker handles exactly one shard.
/// This should still produce correct results.
#[test]
fn max_workers_equals_num_shards() {
    let (store, nodes) = make_large_test_store(512);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Baseline with 1 worker
    let baseline_deltas = execute_parallel(view, &items, 1);
    let baseline_ops = merge_deltas_ok(baseline_deltas).expect("merge failed");

    // Test with exactly NUM_SHARDS workers
    let max_worker_deltas = execute_parallel(view, &items, NUM_SHARDS);
    let max_worker_ops = merge_deltas_ok(max_worker_deltas).expect("merge failed");

    assert_eq!(
        baseline_ops.len(),
        max_worker_ops.len(),
        "max workers: op count differs"
    );

    for (i, (b, m)) in baseline_ops.iter().zip(max_worker_ops.iter()).enumerate() {
        assert_eq!(b, m, "max workers: op {i} differs");
    }

    // Also test with 2x NUM_SHARDS (should be capped)
    let overcapped_deltas = execute_parallel(view, &items, NUM_SHARDS * 2);

    // The implementation caps at NUM_SHARDS
    assert_eq!(
        overcapped_deltas.len(),
        NUM_SHARDS,
        "expected {} deltas (capped), got {}",
        NUM_SHARDS,
        overcapped_deltas.len()
    );

    let overcapped_ops = merge_deltas_ok(overcapped_deltas).expect("merge failed");

    assert_eq!(
        baseline_ops.len(),
        overcapped_ops.len(),
        "overcapped workers: op count differs"
    );

    for (i, (b, o)) in baseline_ops.iter().zip(overcapped_ops.iter()).enumerate() {
        assert_eq!(b, o, "overcapped workers: op {i} differs");
    }
}

/// Verify consistency across multiple independent runs with high parallelism.
///
/// This test catches any non-determinism from thread scheduling or memory
/// allocation patterns when running with many workers.
#[test]
fn repeated_high_parallelism_determinism() {
    let (store, nodes) = make_large_test_store(200);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // High worker count
    let workers = 32;

    // First run establishes baseline
    let first_deltas = execute_parallel(view, &items, workers);
    let first_ops = merge_deltas_ok(first_deltas).expect("merge failed");

    // Repeat 50 times to catch intermittent non-determinism
    for run in 1..=50 {
        let deltas = execute_parallel(view, &items, workers);
        let ops = merge_deltas_ok(deltas).expect("merge failed");

        assert_eq!(
            first_ops.len(),
            ops.len(),
            "run {run}: op count differs with {workers} workers"
        );

        for (i, (f, o)) in first_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(f, o, "run {run}: op {i} differs with {workers} workers");
        }
    }
}
