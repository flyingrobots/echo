// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Peak temporary memory of a tick patch's digest when one op carries a
//! large atom payload.
//!
//! The digest covers every payload byte, and nothing in `warp-core` bounds
//! an atom payload. Gathering the whole digest input before hashing it
//! would hold a second copy of the payload for the length of the hash, in
//! `WarpTickPatchV1::new` and again in `validate_digest`. This witness
//! bounds the peak at a constant that does not grow with the payload.
//!
//! The allocator tracks live bytes (allocated minus freed) and records
//! their peak above the level when the measurement began. This file holds
//! one test and its own allocator, so that nothing else in the process
//! allocates while the measurement runs.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use bytes::Bytes;
use warp_core::{
    make_node_id, make_type_id, make_warp_id, AtomPayload, AttachmentKey, AttachmentValue, NodeKey,
    TickCommitStatus, WarpOp, WarpTickPatchV1,
};

/// Tracks live bytes, and their peak, while `ACTIVE` is set.
struct PeakAllocator;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);

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

/// Size of the one atom payload in the measured patch.
const PAYLOAD_BYTES: usize = 32 << 20;
/// How far live bytes may rise while the digest is computed: room for the
/// patch's own vectors and a bounded hashing buffer, far below the payload.
const PEAK_BUDGET: isize = 1 << 20;

/// Peak live bytes above the starting level while `f` runs.
fn peak_during<T>(f: impl FnOnce() -> T) -> (T, isize) {
    LIVE.store(0, Ordering::SeqCst);
    PEAK.store(0, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
    let value = f();
    ACTIVE.store(false, Ordering::SeqCst);
    (value, PEAK.load(Ordering::SeqCst))
}

#[test]
fn patch_digest_peak_does_not_grow_with_the_payload() {
    let payload = Bytes::from(vec![0xA5u8; PAYLOAD_BYTES]);
    let node = NodeKey {
        warp_id: make_warp_id("patch-digest-payload-peak/warp"),
        local_id: make_node_id("patch-digest-payload-peak/node"),
    };
    let ops = vec![WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(node),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("patch-digest-payload-peak/value"),
            payload,
        ))),
    }];

    let (patch, build_peak) = peak_during(|| {
        WarpTickPatchV1::new(
            0,
            [0u8; 32],
            TickCommitStatus::Committed,
            Vec::new(),
            Vec::new(),
            ops,
        )
    });
    let (validated, validate_peak) = peak_during(|| patch.validate_digest());
    validated.expect("the patch validates its own digest");

    assert!(
        build_peak < PEAK_BUDGET,
        "building the patch raised live bytes by {build_peak} with a {PAYLOAD_BYTES}-byte payload; budget {PEAK_BUDGET}"
    );
    assert!(
        validate_peak < PEAK_BUDGET,
        "validating the digest raised live bytes by {validate_peak} with a {PAYLOAD_BYTES}-byte payload; budget {PEAK_BUDGET}"
    );
}
