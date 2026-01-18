// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Direct tests for BOAW Phase 6A parallel execution.
//!
//! These tests validate the core parallel execution and merge logic
//! without going through the full Engine pipeline.
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
    execute_parallel, execute_serial, make_node_id, make_type_id, merge_deltas, AtomPayload,
    AttachmentKey, AttachmentValue, ExecItem, GraphStore, GraphView, NodeId, NodeKey, NodeRecord,
    OpOrigin, TickDelta, WarpOp,
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
                op_ix: 0, // Will be assigned by ScopedDelta
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
    eprintln!("\n=== WORKER COUNT INVARIANCE TEST ===");
    eprintln!("Testing worker counts: {:?}", WORKER_COUNTS);

    let (store, nodes) = make_test_store(20);
    let view = GraphView::new(&store);
    let items = make_exec_items(&nodes);

    // Baseline with 1 worker
    let baseline_deltas = execute_parallel(view, &items, 1);
    let baseline_ops = merge_deltas(baseline_deltas).expect("merge failed");
    eprintln!("Baseline: {} ops from 1 worker", baseline_ops.len());

    // Test all worker counts
    for &workers in WORKER_COUNTS {
        let deltas = execute_parallel(view, &items, workers);
        let ops = merge_deltas(deltas).expect("merge failed");

        eprintln!("  workers={:2} → {} ops ✓", workers, ops.len());

        assert_eq!(
            baseline_ops.len(),
            ops.len(),
            "op count differs for {workers} workers"
        );

        for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
            assert_eq!(b, o, "op {i} differs for {workers} workers");
        }
    }
    eprintln!("=== ALL WORKER COUNTS PASS ===\n");
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
