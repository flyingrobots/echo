// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A warm commit has an allocation budget. After a warm-up commit, committing
//! one rewrite allocates at most `BUDGET` bytes: the commit's outputs (its
//! receipt, snapshot and patch) and the scratch that is not yet reused. The
//! budget falls as scratch moves into reused engine memory (#903); raising it
//! is a deliberate decision, made in review, never a way to make this pass.
//!
//! Measured in test builds, which also build footprint guards
//! (`debug_assertions`). This file holds one test and its own allocator, so
//! that nothing else in the process allocates while the measurement runs.
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

fn engine_with_entities(n: usize) -> (Engine, NodeId) {
    let mut engine = build_motion_demo_engine();
    engine.register_rule(noop_rule()).expect("rule registers");
    let ty = make_type_id("entity");
    let mut first = None;
    for i in 0..n {
        let id = make_node_id(&format!("commit-allocation-{i}"));
        engine
            .insert_node(id, NodeRecord { ty })
            .expect("node inserts");
        first.get_or_insert(id);
    }
    (engine, first.expect("at least one entity"))
}

/// Bytes allocated by one commit of one rewrite, after a warm-up commit.
fn commit_bytes(entities: usize) -> usize {
    let (mut engine, scope) = engine_with_entities(entities);
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

/// The allocation budget of a warm single-rewrite commit, in bytes.
const BUDGET: usize = 16 * 1024;

#[test]
fn a_warm_single_rewrite_commit_stays_within_its_budget() {
    if warp_core::DELTA_VALIDATE {
        assert!(
            std::env::var_os("ECHO_ALLOC_WITNESS_REQUIRED").is_none(),
            "the allocation witness must run on an ordinary build here"
        );
        eprintln!("not measured: warp-core is built with delta_validate");
        return;
    }
    let bytes = commit_bytes(1_000);
    assert!(
        bytes > 0,
        "the allocator measured nothing; the witness did not run"
    );
    eprintln!("a warm single-rewrite commit allocated {bytes} bytes (budget {BUDGET})");
    assert!(
        bytes <= BUDGET,
        "a warm single-rewrite commit allocated {bytes} bytes, over its {BUDGET}-byte budget"
    );
}
