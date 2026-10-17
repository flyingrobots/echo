// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Adding one edge to a high-degree node must cost one edge, not the node's
//! whole adjacency bucket. The undo record for a commit tracks the edges it
//! changes; it must never copy the buckets they sit in.
//!
//! This file holds one test and its own allocator, so that nothing else in
//! the process allocates while the measurement runs.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use echo_dry_tests::build_motion_demo_engine;
use warp_core::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, ConflictPolicy, EdgeRecord, Engine,
    ExecutionGraphView, Footprint, GraphView, Hash, NodeId, NodeRecord, PatternGraph, RewriteRule,
    RuleExecutor, TickDelta, WarpOp,
};

/// Counts bytes requested while `ACTIVE` is set.
struct CountingAllocator;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static BYTES: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counter does not affect what is allocated or freed.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::SeqCst) {
            BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        // SAFETY: `layout` is the caller's layout, passed through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` and `layout` are the caller's, passed through.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ACTIVE.load(Ordering::SeqCst) {
            BYTES.fetch_add(new_size, Ordering::SeqCst);
        }
        // SAFETY: the caller's arguments, passed through.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const RULE_NAME: &str = "commit-hub-bucket/add-edge";

fn hub() -> NodeId {
    make_node_id("commit-hub-bucket/hub")
}

fn target() -> NodeId {
    make_node_id("commit-hub-bucket/target")
}

fn new_edge() -> warp_core::EdgeId {
    make_edge_id("commit-hub-bucket/new-edge")
}

fn add_edge_rule() -> RewriteRule {
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn executor(view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, delta: &mut TickDelta) {
        delta.emit(WarpOp::UpsertEdge {
            warp_id: view.warp_id(),
            record: EdgeRecord {
                id: new_edge(),
                from: hub(),
                to: target(),
                ty: make_type_id("commit-hub-bucket/link"),
            },
        });
    }
    fn footprint(view: GraphView<'_>, _n: &NodeId) -> Footprint {
        let mut footprint = Footprint::default();
        footprint.n_write.insert_with_warp(view.warp_id(), hub());
        footprint
            .e_write
            .insert_with_warp(view.warp_id(), new_edge());
        footprint
    }
    let id: Hash = blake3::hash(b"rule:commit-hub-bucket/add-edge").into();
    RewriteRule {
        id,
        name: RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher,
        executor: RuleExecutor::observed(executor),
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// An engine whose hub node already has `degree` outbound edges.
fn engine_with_hub(degree: usize) -> Engine {
    let mut engine = build_motion_demo_engine();
    engine
        .register_rule(add_edge_rule())
        .expect("rule registers");
    let ty = make_type_id("commit-hub-bucket/entity");
    let link = make_type_id("commit-hub-bucket/link");
    engine.insert_node(hub(), NodeRecord { ty }).expect("hub");
    engine
        .insert_node(target(), NodeRecord { ty })
        .expect("target");
    let store = engine
        .state_mut()
        .store_mut(&make_warp_id("root"))
        .expect("root store");
    for i in 0..degree {
        let leaf = make_node_id(&format!("commit-hub-bucket/leaf/{i}"));
        store.insert_node(leaf, NodeRecord { ty });
        store.insert_edge(
            hub(),
            EdgeRecord {
                id: make_edge_id(&format!("commit-hub-bucket/edge/{i}")),
                from: hub(),
                to: leaf,
                ty: link,
            },
        );
    }
    engine
}

/// Bytes allocated by the commit that adds one edge to the hub.
fn commit_bytes(degree: usize) -> usize {
    let mut engine = engine_with_hub(degree);
    let tx = engine.begin();
    engine.apply(tx, RULE_NAME, &hub()).expect("rule applies");
    BYTES.store(0, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
    let snapshot = engine.commit(tx).expect("tick commits");
    ACTIVE.store(false, Ordering::SeqCst);
    std::hint::black_box(snapshot);
    BYTES.load(Ordering::SeqCst)
}

#[test]
fn adding_one_edge_does_not_copy_a_high_degree_bucket() {
    let small = commit_bytes(1_000);
    let large = commit_bytes(16_000);
    // The commit changes one edge in both cases. Copying the hub's bucket,
    // which holds 1,000 or 16,000 edges, fails this.
    assert!(
        large < small * 2,
        "adding one edge allocated {large} bytes on a 16000-edge hub but {small} on a 1000-edge hub"
    );
}
