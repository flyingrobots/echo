// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Committing must not allocate in proportion to the reachable graph. The
//! commit hashes the reachable graph into its state root; that walk keeps its
//! working memory across commits, so after a warm-up commit, one that touches
//! nothing allocates the same whether 1,000 or 16,000 entities hang off the
//! root.
//!
//! `commit_allocation.rs` covers entities that are not reachable; this file
//! covers entities that are. Each file holds one test and its own allocator,
//! so that nothing else in the process allocates while the measurement runs.
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
    make_node_id, make_type_id, ConflictPolicy, Engine, ExecutionGraphView, Footprint, GraphView,
    Hash, NodeId, NodeRecord, PatternGraph, RewriteRule, RuleExecutor, TickDelta,
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

const RULE_NAME: &str = "commit-allocation/noop";

fn noop_rule() -> RewriteRule {
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn executor(_view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, _delta: &mut TickDelta) {}
    fn footprint(_view: GraphView<'_>, _n: &NodeId) -> Footprint {
        Footprint::default()
    }
    let id: Hash = blake3::hash(b"rule:commit-allocation/noop").into();
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

/// How the entities hang off the root.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// Every entity is a child of the root: one wide BFS level.
    Star,
    /// The entities form one chain from the root: `n` levels of one node.
    Chain,
    /// All but one entity are children of the root, and the last hangs off
    /// one of them: a wide level, then a narrow one.
    WideThenNarrow,
}

fn engine_with_entities(n: usize, shape: Shape) -> (Engine, NodeId) {
    let mut engine = build_motion_demo_engine();
    engine.register_rule(noop_rule()).expect("rule registers");
    let ty = make_type_id("entity");
    let root_key = engine.root_key();
    let root = root_key.local_id;
    let link = make_type_id("commit-allocation/link");
    let store = engine
        .state_mut()
        .store_mut(&root_key.warp_id)
        .expect("root store");
    let id = |i: usize| make_node_id(&format!("commit-allocation-{i}"));
    for i in 0..n {
        store.insert_node(id(i), NodeRecord { ty });
        let from = match shape {
            Shape::Star => root,
            Shape::Chain if i == 0 => root,
            Shape::Chain => id(i - 1),
            Shape::WideThenNarrow if i + 1 < n => root,
            Shape::WideThenNarrow => id(0),
        };
        store.insert_edge(
            from,
            warp_core::EdgeRecord {
                id: warp_core::make_edge_id(&format!("commit-allocation-edge-{i}")),
                from,
                to: id(i),
                ty: link,
            },
        );
    }
    (engine, id(0))
}

/// Bytes allocated by one commit of one rewrite, after a warm-up commit.
fn commit_bytes(entities: usize, shape: Shape) -> usize {
    let (mut engine, scope) = engine_with_entities(entities, shape);
    for measured in [false, true] {
        let tx = engine.begin();
        engine.apply(tx, RULE_NAME, &scope).expect("rule applies");
        BYTES.store(0, Ordering::SeqCst);
        ACTIVE.store(measured, Ordering::SeqCst);
        let snapshot = engine.commit(tx).expect("tick commits");
        ACTIVE.store(false, Ordering::SeqCst);
        std::hint::black_box(snapshot);
    }
    BYTES.load(Ordering::SeqCst)
}

#[test]
fn commit_cost_does_not_grow_with_untouched_reachable_graph_size() {
    // A `delta_validate` build copies the whole state to cross-check every
    // delta, by design; the measurement is about ordinary builds.
    if warp_core::DELTA_VALIDATE {
        // The performance lane sets ECHO_ALLOC_WITNESS_REQUIRED: there a
        // skipped witness would prove nothing, so it fails instead.
        assert!(
            std::env::var_os("ECHO_ALLOC_WITNESS_REQUIRED").is_none(),
            "the allocation witness must run on an ordinary build here"
        );
        eprintln!("not measured: warp-core is built with delta_validate");
        return;
    }
    // One warm-up commit must size the state root's working memory for any
    // shape: a deep chain, or a wide level followed by a narrow one.
    for shape in [Shape::Star, Shape::Chain, Shape::WideThenNarrow] {
        let small = commit_bytes(1_000, shape);
        let large = commit_bytes(16_000, shape);
        assert!(
            small > 0,
            "the allocator measured nothing; the witness did not run"
        );
        assert!(
            large < small * 2,
            "{shape:?}: commit allocated {large} bytes with 16000 reachable entities but {small} \
             with 1000"
        );
    }
}
