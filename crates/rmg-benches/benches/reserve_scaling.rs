#![allow(missing_docs)]
//! Benchmark: reserve() scaling with footprint size and number of reserved rewrites
//!
//! Measures how reserve() performance scales with:
//! 1. Number of previously reserved rewrites (k)
//! 2. Size of footprint being reserved (m)
//!
//! The current GenSet-based implementation should scale as O(m), independent of k.
//! A naive Vec<Footprint> implementation would scale as O(k × m).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rmg_core::Hash;
use std::time::Duration;

// Import the scheduler - it's crate-private so we can't access it directly
// Instead we'll use it through the Engine API
// Actually, we need direct access for this micro-benchmark, so we'll create
// a test module inside rmg-core and expose it via a feature flag or just
// write an integration test instead.

// For now, let's write a simpler benchmark that measures reserve through the Engine API

fn make_hash(val: u8) -> Hash {
    let mut h = [0u8; 32];
    h[0] = val;
    h
}

// Note: This benchmark requires access to DeterministicScheduler which is crate-private.
// Moving this to rmg-core/src/scheduler.rs tests module or using a pub(crate) test harness.

fn bench_reserve_scaling(_c: &mut Criterion) {
    // This is a placeholder - the actual benchmark needs to be in rmg-core
    // where we can access the scheduler directly.

    // TODO: Implement this properly by either:
    // 1. Adding a test-only public API to DeterministicScheduler
    // 2. Moving this benchmark into rmg-core as a test module
    // 3. Using Engine API indirectly (less precise)

    let _ = (
        BenchmarkId::new("placeholder", "reserve_scaling"),
        Throughput::Elements(1),
        make_hash(0),
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .sample_size(50);
    targets = bench_reserve_scaling
}
criterion_main!(benches);
