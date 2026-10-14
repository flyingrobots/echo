// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Memory the canonical decoder asks for must follow the size of its input,
//! not the size times the nesting depth.
//!
//! This file holds one test and its own allocator, so that nothing else in
//! the process allocates while the measurement runs.
#![allow(unsafe_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use echo_wasm_abi::{CanonError, MAX_CANONICAL_NESTING_DEPTH, decode_value};

/// Counts bytes requested and not yet released, and remembers the peak.
struct MeasuringAllocator {
    live: AtomicUsize,
    peak: AtomicUsize,
}

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counters do not affect what is allocated or freed.
unsafe impl GlobalAlloc for MeasuringAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = self.live.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
        self.peak.fetch_max(live, Ordering::SeqCst);
        // SAFETY: `layout` is the caller's layout, passed through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.live.fetch_sub(layout.size(), Ordering::SeqCst);
        // SAFETY: `ptr` and `layout` are the caller's, passed through.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: MeasuringAllocator = MeasuringAllocator {
    live: AtomicUsize::new(0),
    peak: AtomicUsize::new(0),
};

/// Arrays nested to the limit, each claiming as many elements as there are
/// bytes after its header, around a run of one-byte integers.
///
/// Every claim passes the "fits in the remaining input" check, because each
/// level points at the same remaining bytes.
fn nested_arrays_each_claiming_the_remainder(total_len: usize) -> Vec<u8> {
    const HEADER_LEN: usize = 5; // 0x9a and a four-byte count
    let mut bytes = Vec::with_capacity(total_len);
    for level in 0..MAX_CANONICAL_NESTING_DEPTH {
        let remaining = total_len - (level + 1) * HEADER_LEN;
        bytes.push(0x9a);
        bytes.extend_from_slice(&u32::try_from(remaining).unwrap().to_be_bytes());
    }
    bytes.resize(total_len, 0x00);
    bytes
}

#[test]
fn nested_containers_cannot_multiply_the_memory_an_input_asks_for() {
    const INPUT_LEN: usize = 256 * 1024;
    let input = nested_arrays_each_claiming_the_remainder(INPUT_LEN);

    let before = ALLOCATOR.live.load(Ordering::SeqCst);
    ALLOCATOR.peak.store(before, Ordering::SeqCst);
    let result = decode_value(&input);
    let peak = ALLOCATOR.peak.load(Ordering::SeqCst) - before;

    // The innermost array is short of the elements it claims.
    assert_eq!(result, Err(CanonError::Incomplete));

    // One decoded element costs a few dozen bytes, and a growing vector can
    // briefly hold twice its contents. Reserving every level's claim up front
    // would ask for about a thousand megabytes here.
    let budget = INPUT_LEN * 192;
    assert!(
        peak <= budget,
        "decoding {INPUT_LEN} bytes asked for {peak} bytes at its peak; the budget is {budget}"
    );
}
