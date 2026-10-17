// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A runtime tick must cost what it touches, not what the worldline holds,
//! including the copies it keeps so a failed tick can be undone.
//!
//! This file holds one test and its own allocator, so that nothing else in
//! the process allocates while the measurement runs.
// `delta_validate` deliberately keeps a whole-state copy to cross-check every
// delta, so the measurement applies only without it.
#![cfg(not(feature = "delta_validate"))]
#![allow(
    unsafe_code,
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, EngineBuilder, GraphStore,
    InboxPolicy, IngressEnvelope, IngressTarget, NodeRecord, PlaybackMode, ProvenanceService,
    SchedulerCoordinator, SchedulerKind, WorldlineId, WorldlineRuntime, WorldlineState, WriterHead,
    WriterHeadKey,
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

/// Bytes allocated by one runtime tick of one intent, after a warm-up tick,
/// on a worldline holding `entities` untouched nodes.
fn tick_bytes(entities: usize) -> usize {
    let root = make_node_id("runtime-tick-allocation/root");
    let world = make_type_id("world");
    let entity = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(root, NodeRecord { ty: world });
    for i in 0..entities {
        store.insert_node(
            make_node_id(&format!("runtime-tick-allocation/{i}")),
            NodeRecord { ty: entity },
        );
    }
    let mut engine_store = GraphStore::default();
    engine_store.insert_node(root, NodeRecord { ty: world });
    let mut engine = EngineBuilder::new(engine_store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();

    let worldline_id = WorldlineId::from_bytes([7; 32]);
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(
            worldline_id,
            WorldlineState::from_root_store(store, root).expect("state"),
        )
        .expect("worldline registers");
    runtime
        .register_writer_head(WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("runtime-tick-allocation"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("head registers");
    let mut provenance = ProvenanceService::new();
    for (id, frontier) in runtime.worldlines().iter() {
        provenance
            .register_worldline(*id, frontier.state())
            .expect("provenance registers");
    }

    for (round, measured) in [(0u8, false), (1u8, true)] {
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("runtime-tick-allocation/intent"),
                vec![round],
            ))
            .expect("intent ingests");
        BYTES.store(0, Ordering::SeqCst);
        ACTIVE.store(measured, Ordering::SeqCst);
        let records = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .expect("tick commits");
        ACTIVE.store(false, Ordering::SeqCst);
        assert_eq!(records.len(), 1);
    }
    BYTES.load(Ordering::SeqCst)
}

#[test]
fn runtime_tick_cost_does_not_grow_with_untouched_worldline_size() {
    let small = tick_bytes(1_000);
    let large = tick_bytes(16_000);
    // Sixteen times the untouched worldline must not cost sixteen times the
    // bytes. Copying the state, to diff it or to keep an undo point, fails
    // this.
    assert!(
        large < small * 2,
        "a runtime tick allocated {large} bytes with 16000 entities but {small} with 1000"
    );
}
