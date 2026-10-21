// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Peak temporary memory of one commit whose rewrites claim heavily
//! overlapping footprints.
//!
//! Reservation collects the tick's `in_slots` and `out_slots` as plain
//! vectors that keep repeats until the patch sorts and dedups them, and the
//! patch digest gathers its whole input before hashing it. Both add
//! temporary memory. Here every rewrite reads the same `SHARED` nodes and
//! their attachments, so the repeats are as dense as footprints allow (reads
//! may overlap; writes may not).
//!
//! The allocator tracks live bytes (allocated minus freed) during the
//! commit and records their peak above the level when the commit began.
//! This file holds one test and its own allocator, so that nothing else in
//! the process allocates while the measurement runs.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::print_stderr
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};

use bytes::Bytes;
use echo_dry_tests::build_motion_demo_engine;
use warp_core::{
    make_node_id, make_type_id, AtomPayload, AttachmentKey, AttachmentValue, ConflictPolicy,
    Engine, ExecutionGraphView, Footprint, GraphView, NodeId, NodeKey, NodeRecord, PatternGraph,
    RewriteRule, RuleExecutor, TickDelta, WarpOp,
};

/// Tracks live bytes, and their peak, while `ACTIVE` is set.
struct PeakAllocator;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);

fn grew(bytes: usize) {
    let bytes = isize::try_from(bytes).unwrap_or(isize::MAX);
    let live = LIVE.fetch_add(bytes, Ordering::SeqCst) + bytes;
    PEAK.fetch_max(live, Ordering::SeqCst);
}

fn shrank(bytes: usize) {
    LIVE.fetch_sub(
        isize::try_from(bytes).unwrap_or(isize::MAX),
        Ordering::SeqCst,
    );
}

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counters do not affect what is allocated or freed.
unsafe impl GlobalAlloc for PeakAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::SeqCst) {
            TOTAL.fetch_add(layout.size(), Ordering::SeqCst);
            grew(layout.size());
        }
        // SAFETY: `layout` is the caller's layout, passed through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ACTIVE.load(Ordering::SeqCst) {
            shrank(layout.size());
        }
        // SAFETY: `ptr` and `layout` are the caller's, passed through.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ACTIVE.load(Ordering::SeqCst) {
            TOTAL.fetch_add(new_size, Ordering::SeqCst);
            // A moving realloc holds both blocks for a moment; count the
            // new block before the old one is freed.
            grew(new_size);
            shrank(layout.size());
        }
        // SAFETY: the caller's arguments, passed through.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: PeakAllocator = PeakAllocator;

/// Nodes every rewrite reads, with their attachments, in the overlapping
/// tick. The control tick reads none of them.
const SHARED: usize = 64;
/// Rewrites in the measured tick, one per own node.
const REWRITES: usize = 4_096;
/// How far the overlapping tick's peak may rise above the control's.
///
/// The overlapping tick claims 131 read slots per rewrite instead of three,
/// so the receipt and scheduler legitimately need more room: the
/// `BTreeSet`-based collection peaked at about 3.8 times the control. Slot
/// lists that keep every repeat and hand their full capacity to the patch
/// peaked at about 11 times. Compacting the lists keeps it near 4.
const BUDGET_FACTOR: isize = 6;

fn shared(i: usize) -> NodeId {
    make_node_id(&format!("commit-overlap-peak/shared/{i}"))
}

fn own(i: usize) -> NodeId {
    make_node_id(&format!("commit-overlap-peak/own/{i}"))
}

fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
    true
}

fn executor(view: &mut ExecutionGraphView<'_, '_>, n: &NodeId, delta: &mut TickDelta) {
    // The patch is a state diff, so each tick must change the value:
    // alternate between two payloads.
    let next: &'static [u8] = match view.node_attachment(n) {
        Some(AttachmentValue::Atom(atom)) if atom.bytes.as_ref() == b"even" => b"odd",
        _ => b"even",
    };
    delta.emit(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id: view.warp_id(),
            local_id: *n,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("commit-overlap-peak/value"),
            Bytes::from_static(next),
        ))),
    });
}

/// Reads the first `S` shared nodes and their attachments, and reads and
/// writes the scope's own attachment.
fn footprint<const S: usize>(view: GraphView<'_>, n: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut footprint = Footprint::default();
    for i in 0..S {
        let node = NodeKey {
            warp_id,
            local_id: shared(i),
        };
        footprint.n_read.insert(node);
        footprint.a_read.insert(AttachmentKey::node_alpha(node));
    }
    let own = NodeKey {
        warp_id,
        local_id: *n,
    };
    footprint.n_read.insert(own);
    footprint.a_read.insert(AttachmentKey::node_alpha(own));
    footprint.a_write.insert(AttachmentKey::node_alpha(own));
    footprint
}

fn rule<const S: usize>(name: &'static str) -> RewriteRule {
    RewriteRule {
        id: blake3::hash(name.as_bytes()).into(),
        name,
        left: PatternGraph { nodes: vec![] },
        matcher,
        executor: RuleExecutor::observed(executor),
        compute_footprint: footprint::<S>,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn engine(rule: RewriteRule) -> Engine {
    let mut engine = build_motion_demo_engine();
    engine.register_rule(rule).expect("rule registers");
    let ty = make_type_id("commit-overlap-peak/entity");
    for i in 0..SHARED {
        engine
            .insert_node(shared(i), NodeRecord { ty })
            .expect("shared node inserts");
    }
    for i in 0..REWRITES {
        engine
            .insert_node(own(i), NodeRecord { ty })
            .expect("own node inserts");
    }
    engine
}

struct Measured {
    peak: isize,
    net: isize,
    total: usize,
}

/// Peak live bytes above the commit's starting level, after a warm-up tick.
fn measure_commit(rule: RewriteRule) -> Measured {
    let name = rule.name;
    let mut engine = engine(rule);
    let mut measured = None;
    for measuring in [false, true] {
        let tx = engine.begin();
        for i in 0..REWRITES {
            engine.apply(tx, name, &own(i)).expect("rule applies");
        }
        LIVE.store(0, Ordering::SeqCst);
        PEAK.store(0, Ordering::SeqCst);
        TOTAL.store(0, Ordering::SeqCst);
        ACTIVE.store(measuring, Ordering::SeqCst);
        let (snapshot, receipt, patch) = engine.commit_with_receipt(tx).expect("tick commits");
        ACTIVE.store(false, Ordering::SeqCst);
        if measuring {
            let applied = receipt
                .entries()
                .iter()
                .filter(|entry| entry.disposition == warp_core::TickReceiptDisposition::Applied)
                .count();
            assert_eq!(applied, REWRITES, "every rewrite must be accepted");
            assert_eq!(patch.ops().len(), REWRITES, "one op per rewrite");
            measured = Some(Measured {
                peak: PEAK.load(Ordering::SeqCst),
                net: LIVE.load(Ordering::SeqCst),
                total: TOTAL.load(Ordering::SeqCst),
            });
        }
        std::hint::black_box((snapshot, receipt, patch));
    }
    measured.expect("the measured commit ran")
}

#[test]
fn overlapping_footprints_do_not_inflate_commit_peak_memory() {
    if warp_core::DELTA_VALIDATE {
        assert!(
            std::env::var_os("ECHO_ALLOC_WITNESS_REQUIRED").is_none(),
            "the allocation witness must run on an ordinary build here"
        );
        eprintln!("not measured: warp-core is built with delta_validate");
        return;
    }
    let control = measure_commit(rule::<0>("commit-overlap-peak/control"));
    let overlap = measure_commit(rule::<SHARED>("commit-overlap-peak/overlap"));
    for (label, m) in [("control", &control), ("overlap", &overlap)] {
        eprintln!(
            "PEAK commit_overlap {label} rewrites={REWRITES} peak_live_bytes={} \
             net_live_bytes={} total_alloc_bytes={}",
            m.peak, m.net, m.total
        );
    }
    assert!(control.peak > 0, "the allocator measured nothing");
    assert!(
        overlap.peak < control.peak * BUDGET_FACTOR,
        "overlapping footprints raised the commit's peak to {} bytes from {}",
        overlap.peak,
        control.peak
    );
}
