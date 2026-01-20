// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Direct tests for BOAW Phase 6B parallel execution.
//!
//! Tests validate sharded partitioning, canonical merge ordering, and
//! determinism under various worker counts and input permutations.
//!
//! # Feature Requirements
//!
//! These tests require the `delta_validate` feature to be enabled:
//! ```sh
//! cargo test --package warp-core --test boaw_parallel_exec --features delta_validate
//! ```

// This test requires `--features delta_validate` to compile.
// The merge_deltas function is feature-gated.

use warp_core::{
    execute_parallel, execute_parallel_sharded, execute_serial, make_node_id, make_type_id,
    merge_deltas, AtomPayload, AttachmentKey, AttachmentValue, ExecItem, GraphStore, GraphView,
    NodeId, NodeKey, NodeRecord, OpOrigin, TickDelta, WarpOp, NUM_SHARDS,
};

mod common;
use common::{shuffle, XorShift64, SEEDS, WORKER_COUNTS};

/// Simple executor that sets an attachment on the scope node.
fn touch_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let payload = AtomPayload::new(
        make_type_id("test/marker"),
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

/// Create a test graph with N independent nodes.
fn make_test_store(n: usize) -> (GraphStore, Vec<NodeId>) {
    let node_ty = make_type_id("test/node");
    let mut store = GraphStore::default();
    let mut nodes = Vec::with_capacity(n);

    for i in 0..n {
        let id = make_node_id(&format!("test/n{i}"));
        store.insert_node(id, NodeRecord { ty: node_ty });
        nodes.push(id);
    }

    (store, nodes)
}

/// Create ExecItems for all nodes with proper origins.
fn make_exec_items(nodes: &[NodeId]) -> Vec<ExecItem> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, &scope)| ExecItem {
            exec: touch_executor,
            scope,
            origin: OpOrigin {
                intent_id: i as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        })
        .collect()
}

#[test]
fn parallel_equals_serial_basic() {
    let (store, nodes) = make_test_store(10);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Serial execution
    let serial_delta = execute_serial(view, &items);
    let serial_ops = merge_deltas(vec![serial_delta]).expect("merge failed");

    // Parallel execution with 4 workers
    let parallel_deltas = execute_parallel(view, &items, 4);
    let parallel_ops = merge_deltas(parallel_deltas).expect("merge failed");

    // Must produce same number of ops
    assert_eq!(
        serial_ops.len(),
        parallel_ops.len(),
        "op count mismatch: serial={} parallel={}",
        serial_ops.len(),
        parallel_ops.len()
    );

    // Ops must be identical (same canonical order)
    for (i, (s, p)) in serial_ops.iter().zip(parallel_ops.iter()).enumerate() {
        assert_eq!(s, p, "op {i} differs");
    }
}

#[test]
fn worker_count_invariance() {
    let (store, nodes) = make_test_store(20);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Baseline with 1 worker
    let baseline_deltas = execute_parallel(view, &items, 1);
    let baseline_ops = merge_deltas(baseline_deltas).expect("merge failed");

    // Test all worker counts
    for &workers in WORKER_COUNTS {
        let deltas = execute_parallel(view, &items, workers);
        let ops = merge_deltas(deltas).expect("merge failed");

        assert_eq!(
            baseline_ops.len(),
            ops.len(),
            "op count differs for {workers} workers"
        );

        for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(b, o, "op {i} differs for {workers} workers");
        }
    }
}

#[test]
fn permutation_invariance_under_parallelism() {
    let (store, nodes) = make_test_store(16);
    let view = GraphView::new(&store);
    let mut items = make_exec_items(&nodes);

    // Baseline
    let baseline_deltas = execute_parallel(view, &items, 1);
    let baseline_ops = merge_deltas(baseline_deltas).expect("merge failed");

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);

        for _ in 0..10 {
            shuffle(&mut rng, &mut items);

            for &workers in WORKER_COUNTS {
                let deltas = execute_parallel(view, &items, workers);
                let ops = merge_deltas(deltas).expect("merge failed");

                assert_eq!(
                    baseline_ops.len(),
                    ops.len(),
                    "op count differs (seed={seed:#x}, workers={workers})"
                );

                for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
                    assert_eq!(b, o, "op {i} differs (seed={seed:#x}, workers={workers})");
                }
            }
        }
    }
}

#[test]
fn merge_dedupes_identical_ops() {
    // Test that merge_deltas correctly dedupes identical ops from different workers
    let (store, nodes) = make_test_store(4);
    let view = GraphView::new(&store);

    // Create two deltas that emit the same ops (simulating redundant work)
    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();

    for (i, &node) in nodes.iter().enumerate() {
        let payload = AtomPayload::new(
            make_type_id("test/marker"),
            bytes::Bytes::from_static(b"touched"),
        );
        let key = AttachmentKey::node_alpha(NodeKey {
            warp_id: view.warp_id(),
            local_id: node,
        });
        let op = WarpOp::SetAttachment {
            key,
            value: Some(AttachmentValue::Atom(payload)),
        };

        // Same origin for identical ops
        let origin = OpOrigin {
            intent_id: i as u64,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        };

        delta1.push_with_origin(op.clone(), origin);
        delta2.push_with_origin(op, origin);
    }

    // Merge should dedupe identical ops
    let merged = merge_deltas(vec![delta1, delta2]).expect("merge failed");

    // Should have exactly 4 ops (one per node), not 8
    assert_eq!(merged.len(), 4, "merge should dedupe identical ops");
}

#[test]
fn empty_execution_produces_empty_result() {
    let (store, _nodes) = make_test_store(5);
    let view = GraphView::new(&store);
    let items: Vec<ExecItem> = vec![];

    // Serial
    let serial_delta = execute_serial(view, &items);
    assert!(serial_delta.is_empty(), "serial delta should be empty");

    // Parallel
    let parallel_deltas = execute_parallel(view, &items, 4);
    let merged = merge_deltas(parallel_deltas).expect("merge failed");
    assert!(merged.is_empty(), "parallel merged should be empty");
}

#[test]
fn single_item_execution() {
    let (store, nodes) = make_test_store(1);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Serial
    let serial_delta = execute_serial(view, &items);
    let serial_ops = merge_deltas(vec![serial_delta]).expect("merge failed");

    // Parallel with various worker counts
    for &workers in WORKER_COUNTS {
        let parallel_deltas = execute_parallel(view, &items, workers);
        let parallel_ops = merge_deltas(parallel_deltas).expect("merge failed");

        assert_eq!(
            serial_ops.len(),
            parallel_ops.len(),
            "single item: op count differs for {workers} workers"
        );

        for (i, (s, p)) in serial_ops.iter().zip(parallel_ops.iter()).enumerate() {
            assert_eq!(s, p, "single item: op {i} differs for {workers} workers");
        }
    }
}

#[test]
fn large_workload_worker_count_invariance() {
    // Test with more items than any reasonable worker count
    let (store, nodes) = make_test_store(100);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Baseline
    let baseline_deltas = execute_parallel(view, &items, 1);
    let baseline_ops = merge_deltas(baseline_deltas).expect("merge failed");

    assert_eq!(baseline_ops.len(), 100, "should have 100 ops");

    // Test all worker counts
    for &workers in WORKER_COUNTS {
        let deltas = execute_parallel(view, &items, workers);
        let ops = merge_deltas(deltas).expect("merge failed");

        assert_eq!(
            baseline_ops.len(),
            ops.len(),
            "large workload: op count differs for {workers} workers"
        );

        for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(b, o, "large workload: op {i} differs for {workers} workers");
        }
    }
}

// =============================================================================
// PHASE 6B TESTS: Sharded Execution
// =============================================================================

/// Phase 6B: Worker count is capped at NUM_SHARDS.
///
/// Requesting 512 workers should not spawn 512 threads when NUM_SHARDS is 256.
/// This test verifies the capping behavior produces correct results.
#[test]
fn worker_count_capped_at_num_shards() {
    let (store, nodes) = make_test_store(20);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Baseline with NUM_SHARDS workers (the cap)
    let baseline_deltas = execute_parallel(view, &items, NUM_SHARDS);
    let baseline_ops = merge_deltas(baseline_deltas).expect("merge failed");

    // Request more workers than shards - should be capped
    let capped_deltas = execute_parallel(view, &items, NUM_SHARDS * 2);

    // The number of deltas returned should be capped
    assert_eq!(
        capped_deltas.len(),
        NUM_SHARDS,
        "expected {} deltas (capped), got {}",
        NUM_SHARDS,
        capped_deltas.len()
    );

    let capped_ops = merge_deltas(capped_deltas).expect("merge failed");

    // Results should still be correct
    assert_eq!(
        baseline_ops.len(),
        capped_ops.len(),
        "capped execution produced different op count"
    );

    for (i, (b, c)) in baseline_ops.iter().zip(capped_ops.iter()).enumerate() {
        assert_eq!(b, c, "capped execution: op {i} differs");
    }
}

/// Phase 6B: Sharded execution distributes work by shard_of(scope).
///
/// This test verifies that items are partitioned correctly - items with
/// the same shard ID should be processed together (though we can't directly
/// observe which worker got which shard without instrumentation).
#[test]
fn sharded_distribution_is_deterministic() {
    use warp_core::shard_of;

    let (store, nodes) = make_test_store(64);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Verify shard distribution is consistent
    let mut shard_counts = [0usize; 256];
    for item in &items {
        let shard = shard_of(&item.scope);
        shard_counts[shard] += 1;
    }

    // With 64 items spread across 256 shards, we expect sparse distribution
    let non_empty_shards: usize = shard_counts.iter().filter(|&&c| c > 0).count();
    // Sanity check: items should be distributed across multiple shards
    assert!(
        non_empty_shards > 1,
        "items should be distributed across shards"
    );

    // Run sharded execution multiple times - should be deterministic
    let first_deltas = execute_parallel_sharded(view, &items, 8);
    let first_ops = merge_deltas(first_deltas).expect("merge failed");

    for run in 1..=5 {
        let deltas = execute_parallel_sharded(view, &items, 8);
        let ops = merge_deltas(deltas).expect("merge failed");

        assert_eq!(
            first_ops.len(),
            ops.len(),
            "run {run}: op count differs from first run"
        );

        for (i, (f, o)) in first_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(f, o, "run {run}: op {i} differs from first run");
        }
    }
}

/// Phase 6B: Default execute_parallel uses sharded (not stride).
///
/// This test verifies that the default path is sharded execution.
/// The result should match execute_parallel_sharded exactly.
#[test]
fn default_parallel_uses_sharded() {
    let (store, nodes) = make_test_store(30);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Default execute_parallel
    let default_deltas = execute_parallel(view, &items, 4);
    let default_ops = merge_deltas(default_deltas).expect("merge failed");

    // Explicit sharded
    let sharded_deltas = execute_parallel_sharded(view, &items, 4);
    let sharded_ops = merge_deltas(sharded_deltas).expect("merge failed");

    assert_eq!(
        default_ops.len(),
        sharded_ops.len(),
        "default should use sharded implementation"
    );

    for (i, (d, s)) in default_ops.iter().zip(sharded_ops.iter()).enumerate() {
        assert_eq!(d, s, "default vs sharded: op {i} differs");
    }
}
