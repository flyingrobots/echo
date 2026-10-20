// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A commit abandoned by a panicking rule executor must not leave its touch
//! log recording. Writes made afterwards, outside any tick, must cost exactly
//! what they cost after a clean commit: if the log were still running, each
//! write would also keep a prior value, and memory would grow with every
//! write until the next commit began.
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

const PANIC: &str = "abandoned-commit/panic";
const NOOP: &str = "abandoned-commit/noop";
const WRITES: usize = 4_096;

fn rule(
    name: &'static str,
    executor: fn(&mut ExecutionGraphView<'_, '_>, &NodeId, &mut TickDelta),
) -> RewriteRule {
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn footprint(_view: GraphView<'_>, _n: &NodeId) -> Footprint {
        Footprint::default()
    }
    let id: Hash = blake3::hash(name.as_bytes()).into();
    RewriteRule {
        id,
        name,
        left: PatternGraph { nodes: vec![] },
        matcher,
        executor: RuleExecutor::observed(executor),
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn panicking(_view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, _delta: &mut TickDelta) {
    panic!("abandoned-commit: executor aborts this commit");
}

fn noop(_view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, _delta: &mut TickDelta) {}

/// An engine after one commit through `rule_name`, whether it completed or
/// was abandoned by a panic the caller caught.
fn engine_after(rule_name: &'static str) -> Engine {
    let mut engine = build_motion_demo_engine();
    engine.register_rule(rule(PANIC, panicking)).expect("rule");
    engine.register_rule(rule(NOOP, noop)).expect("rule");
    let scope = make_node_id("abandoned-commit/scope");
    let ty = make_type_id("abandoned-commit/entity");
    engine.insert_node(scope, NodeRecord { ty }).expect("scope");
    let tx = engine.begin();
    engine.apply(tx, rule_name, &scope).expect("rule applies");
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = engine.commit_with_receipt(tx);
    }));
    engine
}

/// Bytes allocated by `WRITES` node inserts made outside any tick.
fn out_of_tick_write_bytes(mut engine: Engine) -> usize {
    let ty = make_type_id("abandoned-commit/entity");
    let ids: Vec<NodeId> = (0..WRITES)
        .map(|i| make_node_id(&format!("abandoned-commit/write/{i}")))
        .collect();
    BYTES.store(0, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
    for id in &ids {
        engine.insert_node(*id, NodeRecord { ty }).expect("write");
    }
    ACTIVE.store(false, Ordering::SeqCst);
    std::hint::black_box(engine);
    BYTES.load(Ordering::SeqCst)
}

#[test]
fn writes_after_an_abandoned_commit_keep_no_priors() {
    // Silence the expected panic's report; the hook is not under measurement.
    std::panic::set_hook(Box::new(|_| {}));
    let clean = out_of_tick_write_bytes(engine_after(NOOP));
    let abandoned = out_of_tick_write_bytes(engine_after(PANIC));
    let _ = std::panic::take_hook();
    assert!(
        clean > 0,
        "the allocator measured nothing; the witness did not run"
    );
    assert_eq!(
        abandoned, clean,
        "{WRITES} out-of-tick writes allocated {abandoned} bytes after an abandoned commit \
         but {clean} after a clean one"
    );
}
