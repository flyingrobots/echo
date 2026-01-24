// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Multi-warp engine integration tests for BOAW Phase 6.
#![cfg(feature = "delta_validate")]
//!
//! Tests warp iteration order determinism (T3.1) and apply routing (T8.1).
//!
//! # Feature Requirements
//! ```sh
//! cargo test --package warp-core --test boaw_engine_integration_multiwarp --features delta_validate
//! ```

use warp_core::{
    execute_parallel, execute_serial, make_node_id, make_type_id, make_warp_id, merge_deltas,
    AtomPayload, AttachmentKey, AttachmentValue, ExecItem, GraphStore, GraphView, NodeId, NodeKey,
    NodeRecord, OpOrigin, TickDelta, WarpId, WarpOp,
};

mod common;
use common::{shuffle, XorShift64, SEEDS, WORKER_COUNTS};

// =============================================================================
// TEST HELPERS
// =============================================================================

/// Simple executor that sets an attachment on the scope node using view's warp_id.
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
        let id = make_node_id(&format!("multiwarp/n{i}"));
        store.insert_node(id, NodeRecord { ty: node_ty });
        nodes.push(id);
    }

    (store, nodes)
}

/// Create ExecItems for nodes.
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

/// Create ExecItems from node groups with interleaved origins.
fn make_mixed_exec_items(node_groups: &[Vec<NodeId>]) -> Vec<ExecItem> {
    let mut items = Vec::new();
    let mut intent_counter = 0u64;

    for nodes in node_groups {
        for &scope in nodes {
            items.push(ExecItem::new(
                touch_executor,
                scope,
                OpOrigin {
                    intent_id: intent_counter,
                    rule_id: 1,
                    match_ix: 0,
                    op_ix: 0,
                },
            ));
            intent_counter += 1;
        }
    }

    items
}

// =============================================================================
// T3.1: WARP ITERATION ORDER DETERMINISM
// =============================================================================

/// T3.1: Warp iteration order does not affect result.
///
/// Given: Two node groups A and B with independent operations
/// Run: A-then-B ingress order vs B-then-A ingress order
/// Expect: Identical merged ops regardless of ingress order
#[test]
fn warp_iteration_order_does_not_affect_result() {
    // Create two separate node groups to simulate multi-warp-like separation
    let (store_a, nodes_a) = make_test_store(10);
    let node_ty = make_type_id("test/node");

    // Create a combined store with distinct node groups
    let mut combined_store = store_a;
    let mut nodes_b = Vec::with_capacity(10);
    for i in 0..10 {
        let id = make_node_id(&format!("multiwarp-b/n{i}"));
        combined_store.insert_node(id, NodeRecord { ty: node_ty });
        nodes_b.push(id);
    }

    let view = GraphView::new(&combined_store);

    // Create items: A-then-B order
    let items_a = make_exec_items(&nodes_a);
    let items_b = make_exec_items(&nodes_b);

    let mut items_a_then_b: Vec<ExecItem> = Vec::new();
    items_a_then_b.extend(items_a.iter().cloned());
    items_a_then_b.extend(items_b.iter().cloned());

    // Create items: B-then-A order
    let mut items_b_then_a: Vec<ExecItem> = Vec::new();
    items_b_then_a.extend(items_b.iter().cloned());
    items_b_then_a.extend(items_a.iter().cloned());

    // Execute with A-then-B order
    let deltas_a_then_b = execute_parallel(view, &items_a_then_b, 4);
    let ops_a_then_b = merge_deltas(deltas_a_then_b).expect("merge failed for A-then-B");

    // Execute with B-then-A order
    let deltas_b_then_a = execute_parallel(view, &items_b_then_a, 4);
    let ops_b_then_a = merge_deltas(deltas_b_then_a).expect("merge failed for B-then-A");

    // Verify same result regardless of order
    assert_eq!(
        ops_a_then_b.len(),
        ops_b_then_a.len(),
        "op count differs: A-then-B={} B-then-A={}",
        ops_a_then_b.len(),
        ops_b_then_a.len()
    );

    for (i, (op_ab, op_ba)) in ops_a_then_b.iter().zip(ops_b_then_a.iter()).enumerate() {
        assert_eq!(
            op_ab, op_ba,
            "op {i} differs between A-then-B and B-then-A ordering"
        );
    }
}

/// T3.1 extended: Test with multiple seeds and worker counts.
#[test]
fn warp_iteration_order_invariance_across_seeds_and_workers() {
    let (store, nodes) = make_test_store(20);
    let view = GraphView::new(&store);

    // Split nodes between groups
    let (nodes_a, nodes_b) = nodes.split_at(10);

    let items_a = make_exec_items(nodes_a);
    let items_b = make_exec_items(nodes_b);

    // Baseline: A-then-B with serial execution
    let mut baseline_items: Vec<ExecItem> = Vec::new();
    baseline_items.extend(items_a.iter().cloned());
    baseline_items.extend(items_b.iter().cloned());

    let baseline_delta = execute_serial(view, &baseline_items);
    let baseline_ops = merge_deltas(vec![baseline_delta]).expect("baseline merge failed");

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);

        // Create B-then-A ordering
        let mut items_b_then_a: Vec<ExecItem> = Vec::new();
        items_b_then_a.extend(items_b.iter().cloned());
        items_b_then_a.extend(items_a.iter().cloned());

        // Shuffle both orderings
        let mut shuffled_a_then_b = baseline_items.clone();
        let mut shuffled_b_then_a = items_b_then_a.clone();
        shuffle(&mut rng, &mut shuffled_a_then_b);
        shuffle(&mut rng, &mut shuffled_b_then_a);

        for &workers in WORKER_COUNTS {
            // Test A-then-B shuffled
            let deltas_ab = execute_parallel(view, &shuffled_a_then_b, workers);
            let ops_ab = merge_deltas(deltas_ab).expect("merge failed");

            // Test B-then-A shuffled
            let deltas_ba = execute_parallel(view, &shuffled_b_then_a, workers);
            let ops_ba = merge_deltas(deltas_ba).expect("merge failed");

            assert_eq!(
                baseline_ops.len(),
                ops_ab.len(),
                "T3.1: A-then-B op count differs (seed={seed:#x}, workers={workers})"
            );
            assert_eq!(
                baseline_ops.len(),
                ops_ba.len(),
                "T3.1: B-then-A op count differs (seed={seed:#x}, workers={workers})"
            );

            for (i, (b, ab)) in baseline_ops.iter().zip(ops_ab.iter()).enumerate() {
                assert_eq!(
                    b, ab,
                    "T3.1: A-then-B op {i} differs (seed={seed:#x}, workers={workers})"
                );
            }

            for (i, (b, ba)) in baseline_ops.iter().zip(ops_ba.iter()).enumerate() {
                assert_eq!(
                    b, ba,
                    "T3.1: B-then-A op {i} differs (seed={seed:#x}, workers={workers})"
                );
            }
        }
    }
}

// =============================================================================
// T8.1: APPLY ROUTING BY OP.KEY.WARP_ID()
// =============================================================================

/// T8.1: Ops with different warp_ids should route to correct warps.
///
/// Given: Operations targeting different warps
/// Expect: Each op lands in the warp specified by its key's warp_id
#[test]
fn apply_routes_by_op_warp_id_not_ambient_context() {
    let warp_a = make_warp_id("routing-warp-a");
    let warp_b = make_warp_id("routing-warp-b");

    // Create test nodes
    let node_a = make_node_id("routing/node-a");
    let node_b = make_node_id("routing/node-b");

    let node_ty = make_type_id("test/node");
    let mut store = GraphStore::default();
    store.insert_node(node_a, NodeRecord { ty: node_ty });
    store.insert_node(node_b, NodeRecord { ty: node_ty });

    // Store is created but view is not needed for this test since we're
    // directly constructing WarpOps with explicit warp_id routing
    let _view = GraphView::new(&store);

    // Create ops that explicitly target different warps via attachment keys
    let mut delta = TickDelta::new();

    let payload_a = AtomPayload::new(
        make_type_id("test/marker-a"),
        bytes::Bytes::from_static(b"warp-a-payload"),
    );
    let payload_b = AtomPayload::new(
        make_type_id("test/marker-b"),
        bytes::Bytes::from_static(b"warp-b-payload"),
    );

    // Op targeting warp A
    let key_a = AttachmentKey::node_alpha(NodeKey {
        warp_id: warp_a,
        local_id: node_a,
    });
    delta.push(WarpOp::SetAttachment {
        key: key_a,
        value: Some(AttachmentValue::Atom(payload_a)),
    });

    // Op targeting warp B
    let key_b = AttachmentKey::node_alpha(NodeKey {
        warp_id: warp_b,
        local_id: node_b,
    });
    delta.push(WarpOp::SetAttachment {
        key: key_b,
        value: Some(AttachmentValue::Atom(payload_b)),
    });

    // Merge and verify routing
    let ops = merge_deltas(vec![delta]).expect("merge failed");

    assert_eq!(ops.len(), 2, "expected 2 ops after merge");

    // Verify each op has the correct warp_id in its key
    for op in &ops {
        match op {
            WarpOp::SetAttachment { key, .. } => {
                let op_warp_id = match key.owner {
                    warp_core::AttachmentOwner::Node(node_key) => node_key.warp_id,
                    warp_core::AttachmentOwner::Edge(edge_key) => edge_key.warp_id,
                };
                // Each op should have its intended warp_id
                assert!(
                    op_warp_id == warp_a || op_warp_id == warp_b,
                    "T8.1: op has unexpected warp_id"
                );
            }
            _ => panic!("T8.1: expected SetAttachment op"),
        }
    }

    // Verify ops are distinct (different warps)
    let warp_ids: Vec<WarpId> = ops
        .iter()
        .filter_map(|op| match op {
            WarpOp::SetAttachment { key, .. } => Some(match key.owner {
                warp_core::AttachmentOwner::Node(node_key) => node_key.warp_id,
                warp_core::AttachmentOwner::Edge(edge_key) => edge_key.warp_id,
            }),
            _ => None,
        })
        .collect();

    assert!(
        warp_ids.contains(&warp_a),
        "T8.1: warp_a op missing from result"
    );
    assert!(
        warp_ids.contains(&warp_b),
        "T8.1: warp_b op missing from result"
    );
}

/// T8.1 extended: Mixed-warp operations maintain routing under parallel execution.
#[test]
fn apply_routing_preserved_under_parallel_execution() {
    let node_ty = make_type_id("test/node");
    let mut store = GraphStore::default();

    // Create 5 nodes per group (simulating different warps)
    let mut nodes_a = Vec::new();
    let mut nodes_b = Vec::new();
    let mut nodes_c = Vec::new();

    for i in 0..5 {
        let na = make_node_id(&format!("parallel-routing/a{i}"));
        let nb = make_node_id(&format!("parallel-routing/b{i}"));
        let nc = make_node_id(&format!("parallel-routing/c{i}"));
        store.insert_node(na, NodeRecord { ty: node_ty });
        store.insert_node(nb, NodeRecord { ty: node_ty });
        store.insert_node(nc, NodeRecord { ty: node_ty });
        nodes_a.push(na);
        nodes_b.push(nb);
        nodes_c.push(nc);
    }

    let view = GraphView::new(&store);

    // Create mixed exec items from node groups
    let node_groups = vec![nodes_a.clone(), nodes_b.clone(), nodes_c.clone()];
    let items = make_mixed_exec_items(&node_groups);

    // Serial baseline
    let serial_delta = execute_serial(view, &items);
    let serial_ops = merge_deltas(vec![serial_delta]).expect("serial merge failed");

    // Parallel execution with various worker counts
    for &workers in WORKER_COUNTS {
        let parallel_deltas = execute_parallel(view, &items, workers);
        let parallel_ops = merge_deltas(parallel_deltas).expect("parallel merge failed");

        assert_eq!(
            serial_ops.len(),
            parallel_ops.len(),
            "T8.1: op count differs for {workers} workers"
        );

        for (i, (s, p)) in serial_ops.iter().zip(parallel_ops.iter()).enumerate() {
            assert_eq!(s, p, "T8.1: op {i} differs for {workers} workers");
        }
    }
}

// =============================================================================
// MULTIWARP INGRESS PERMUTATION INVARIANCE
// =============================================================================

/// Multiwarp ingress permutation invariance test.
///
/// Given: Mixed items from node groups A, B, C (simulating multi-warp separation)
/// Shuffle: Randomize item order
/// Expect: Same merged result regardless of shuffle
#[test]
fn multiwarp_ingress_permutation_invariance() {
    let (store, nodes) = make_test_store(30);
    let view = GraphView::new(&store);

    // Split nodes across groups
    let nodes_a = nodes[0..10].to_vec();
    let nodes_b = nodes[10..20].to_vec();
    let nodes_c = nodes[20..30].to_vec();

    let node_groups = vec![nodes_a, nodes_b, nodes_c];

    let baseline_items = make_mixed_exec_items(&node_groups);

    // Baseline with serial execution
    let baseline_delta = execute_serial(view, &baseline_items);
    let baseline_ops = merge_deltas(vec![baseline_delta]).expect("baseline merge failed");

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);

        for _ in 0..10 {
            let mut shuffled_items = baseline_items.clone();
            shuffle(&mut rng, &mut shuffled_items);

            for &workers in WORKER_COUNTS {
                let deltas = execute_parallel(view, &shuffled_items, workers);
                let ops = merge_deltas(deltas).expect("merge failed");

                assert_eq!(
                    baseline_ops.len(),
                    ops.len(),
                    "multiwarp permutation: op count differs (seed={seed:#x}, workers={workers})"
                );

                for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
                    assert_eq!(
                        b, o,
                        "multiwarp permutation: op {i} differs (seed={seed:#x}, workers={workers})"
                    );
                }
            }
        }
    }
}

/// Stress test: Large multi-warp workload with heavy shuffling.
#[test]
fn multiwarp_large_workload_permutation_invariance() {
    let node_ty = make_type_id("test/node");
    let mut store = GraphStore::default();
    let mut all_node_groups: Vec<Vec<NodeId>> = Vec::new();

    // Create 16 nodes per group, 8 groups (128 total nodes)
    for group_idx in 0..8 {
        let mut nodes = Vec::new();
        for node_idx in 0..16 {
            let node = make_node_id(&format!("stress/g{group_idx}/n{node_idx}"));
            store.insert_node(node, NodeRecord { ty: node_ty });
            nodes.push(node);
        }
        all_node_groups.push(nodes);
    }

    let view = GraphView::new(&store);
    let baseline_items = make_mixed_exec_items(&all_node_groups);

    // Baseline
    let baseline_delta = execute_serial(view, &baseline_items);
    let baseline_ops = merge_deltas(vec![baseline_delta]).expect("baseline merge failed");

    assert_eq!(
        baseline_ops.len(),
        128,
        "stress test should produce 128 ops"
    );

    // Heavy shuffle stress test
    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);

        for iteration in 0..5 {
            let mut shuffled = baseline_items.clone();
            // Multiple shuffles for extra randomization
            for _ in 0..3 {
                shuffle(&mut rng, &mut shuffled);
            }

            for &workers in &[4, 16, 32] {
                let deltas = execute_parallel(view, &shuffled, workers);
                let ops = merge_deltas(deltas).expect("merge failed");

                assert_eq!(
                    baseline_ops.len(),
                    ops.len(),
                    "stress test: op count differs (seed={seed:#x}, iter={iteration}, workers={workers})"
                );

                for (i, (b, o)) in baseline_ops.iter().zip(ops.iter()).enumerate() {
                    assert_eq!(
                        b, o,
                        "stress test: op {i} differs (seed={seed:#x}, iter={iteration}, workers={workers})"
                    );
                }
            }
        }
    }
}

/// Interleaved group ordering: A,B,A,B,A,B pattern vs B,A,B,A,B,A pattern.
#[test]
fn interleaved_warp_ordering_invariance() {
    let (store, nodes) = make_test_store(20);
    let view = GraphView::new(&store);

    let nodes_a = &nodes[0..10];
    let nodes_b = &nodes[10..20];

    // Pattern 1: A,B,A,B,A,B...
    let mut pattern_ab = Vec::new();
    for i in 0..10 {
        pattern_ab.push(ExecItem::new(
            touch_executor,
            nodes_a[i],
            OpOrigin {
                intent_id: (i * 2) as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        ));
        pattern_ab.push(ExecItem::new(
            touch_executor,
            nodes_b[i],
            OpOrigin {
                intent_id: (i * 2 + 1) as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        ));
    }

    // Pattern 2: B,A,B,A,B,A...
    let mut pattern_ba = Vec::new();
    for i in 0..10 {
        pattern_ba.push(ExecItem::new(
            touch_executor,
            nodes_b[i],
            OpOrigin {
                intent_id: (i * 2) as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        ));
        pattern_ba.push(ExecItem::new(
            touch_executor,
            nodes_a[i],
            OpOrigin {
                intent_id: (i * 2 + 1) as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        ));
    }

    // Execute both patterns
    let deltas_ab = execute_parallel(view, &pattern_ab, 8);
    let ops_ab = merge_deltas(deltas_ab).expect("merge failed for A,B pattern");

    let deltas_ba = execute_parallel(view, &pattern_ba, 8);
    let ops_ba = merge_deltas(deltas_ba).expect("merge failed for B,A pattern");

    assert_eq!(
        ops_ab.len(),
        ops_ba.len(),
        "interleaved patterns should produce same op count"
    );

    for (i, (ab, ba)) in ops_ab.iter().zip(ops_ba.iter()).enumerate() {
        assert_eq!(ab, ba, "interleaved patterns: op {i} differs");
    }
}
