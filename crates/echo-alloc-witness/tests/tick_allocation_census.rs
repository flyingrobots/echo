// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A census, not a witness: allocations made by committing one engine tick of `n`
//! independent rewrites, first on a fresh engine and then in steady state on
//! the same engine. Each rewrite writes one node attachment, as the worker
//! sweep bench's rewrites do with no hashing work.
//!
//! It asserts only that the counter ran. Run it explicitly:
//! `cargo test -p echo-alloc-witness --test tick_allocation_census -- --ignored --nocapture`.
//!
//! This file holds one test and its own allocator, so that nothing else in
//! the process allocates while the measurement runs.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::print_stdout
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use warp_core::{
    make_node_id, make_type_id, AtomPayload, AttachmentKey, AttachmentValue, ConflictPolicy,
    EngineBuilder, ExecutionGraphView, Footprint, GraphStore, GraphView, Hash, NodeId, NodeKey,
    NodeRecord, PatternGraph, RewriteRule, RuleExecutor, TickDelta, WarpOp,
};

/// Counts allocation calls and bytes requested while `ACTIVE` is set.
struct CountingAllocator;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counters do not affect what is allocated or freed.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::SeqCst) {
            CALLS.fetch_add(1, Ordering::SeqCst);
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
            CALLS.fetch_add(1, Ordering::SeqCst);
            BYTES.fetch_add(new_size, Ordering::SeqCst);
        }
        // SAFETY: the caller's arguments, passed through.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const RULE_NAME: &str = "tick-allocation-census/write";

fn write_rule() -> RewriteRule {
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn executor(view: &mut ExecutionGraphView<'_, '_>, n: &NodeId, delta: &mut TickDelta) {
        delta.push(WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(NodeKey {
                warp_id: view.warp_id(),
                local_id: *n,
            }),
            value: Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id("census/result"),
                bytes::Bytes::copy_from_slice(&n.0),
            ))),
        });
    }
    fn footprint(view: GraphView<'_>, n: &NodeId) -> Footprint {
        let warp_id = view.warp_id();
        let mut footprint = Footprint::default();
        footprint.n_read.insert_with_warp(warp_id, *n);
        footprint.a_write.insert(AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: *n,
        }));
        footprint
    }
    let id: Hash = blake3::hash(b"rule:tick-allocation-census/write").into();
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

/// (calls, bytes) for each of `ticks` commits of `n` rewrites on one engine.
fn census(n: usize, ticks: usize) -> Vec<(usize, usize)> {
    let root = make_node_id("census-root");
    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("census-root"),
        },
    );
    let ids: Vec<NodeId> = (0..n)
        .map(|i| make_node_id(&format!("census-entity-{i}")))
        .collect();
    for id in &ids {
        store.insert_node(
            *id,
            NodeRecord {
                ty: make_type_id("census-entity"),
            },
        );
    }
    let mut engine = EngineBuilder::new(store, root).workers(1).build();
    engine.register_rule(write_rule()).expect("rule registers");
    let mut out = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let tx = engine.begin();
        for id in &ids {
            engine.apply(tx, RULE_NAME, id).expect("rule applies");
        }
        CALLS.store(0, Ordering::SeqCst);
        BYTES.store(0, Ordering::SeqCst);
        ACTIVE.store(true, Ordering::SeqCst);
        let result = engine.commit_with_receipt(tx).expect("tick commits");
        ACTIVE.store(false, Ordering::SeqCst);
        std::hint::black_box(result);
        out.push((CALLS.load(Ordering::SeqCst), BYTES.load(Ordering::SeqCst)));
    }
    out
}

#[test]
#[ignore = "a measurement; run with --ignored --nocapture"]
fn tick_allocation_census() {
    if warp_core::DELTA_VALIDATE {
        println!("ALLOC not measured: warp-core is built with delta_validate");
        return;
    }
    for n in [100, 10_000] {
        let ticks = census(n, 4);
        for (k, (calls, bytes)) in ticks.iter().enumerate() {
            println!(
                "ALLOC rewrites={n} tick={} allocations={calls} bytes={bytes} \
                 allocations_per_rewrite={:.2} bytes_per_rewrite={:.1}",
                k + 1,
                *calls as f64 / n as f64,
                *bytes as f64 / n as f64
            );
        }
        assert!(ticks.iter().all(|(calls, _)| *calls > 0), "the counter ran");
    }
}
