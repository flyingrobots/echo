// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Microbenchmarks for `MaterializationBus` performance.
#![allow(missing_docs)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use warp_core::materialization::{make_channel_id, ChannelPolicy, EmitKey, MaterializationBus};
use warp_core::Hash;

/// Helper to create a deterministic hash from a u64.
fn h(n: u64) -> Hash {
    let mut bytes = [0u8; 32];
    bytes[24..32].copy_from_slice(&n.to_be_bytes());
    bytes
}

/// Benchmark emitting 1000 items to a single `Log` channel.
fn bench_materialization_emit_log(c: &mut Criterion) {
    let bus = MaterializationBus::new();
    let ch = make_channel_id("bench:log");
    let payload = vec![0u8; 64];

    c.bench_function("materialization_emit_log_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let _ = bus.emit(
                    black_box(ch),
                    black_box(EmitKey::new(h(i), 1)),
                    black_box(payload.clone()),
                );
            }
            bus.clear();
        })
    });
}

/// Benchmark finalizing a single `Log` channel with 1000 items.
fn bench_materialization_finalize_log(c: &mut Criterion) {
    let bus = MaterializationBus::new();
    let ch = make_channel_id("bench:log");
    let payload = vec![0u8; 64];

    c.bench_function("materialization_finalize_log_1000", |b| {
        b.iter_with_setup(
            || {
                for i in 0..1000 {
                    let _ = bus.emit(ch, EmitKey::new(h(i), 1), payload.clone());
                }
            },
            |_| {
                let _ = black_box(bus.finalize());
            },
        )
    });
}

/// Benchmark emitting 1000 items across 1000 distinct `StrictSingle` channels.
fn bench_materialization_emit_strict_many(c: &mut Criterion) {
    let mut bus = MaterializationBus::new();
    let channels: Vec<_> = (0..1000)
        .map(|i| {
            let ch = make_channel_id(&format!("bench:strict:{}", i));
            bus.register_channel(ch, ChannelPolicy::StrictSingle);
            ch
        })
        .collect();
    let payload = vec![0u8; 64];

    c.bench_function("materialization_emit_strict_1000", |b| {
        b.iter(|| {
            for ch in &channels {
                let _ = bus.emit(
                    black_box(*ch),
                    black_box(EmitKey::new(h(0), 1)),
                    black_box(payload.clone()),
                );
            }
            bus.clear();
        })
    });
}

/// Benchmark finalizing 1000 `StrictSingle` channels.
fn bench_materialization_finalize_strict_many(c: &mut Criterion) {
    let mut bus = MaterializationBus::new();
    let channels: Vec<_> = (0..1000)
        .map(|i| {
            let ch = make_channel_id(&format!("bench:strict:{}", i));
            bus.register_channel(ch, ChannelPolicy::StrictSingle);
            ch
        })
        .collect();
    let payload = vec![0u8; 64];

    c.bench_function("materialization_finalize_strict_1000", |b| {
        b.iter_with_setup(
            || {
                for ch in &channels {
                    let _ = bus.emit(*ch, EmitKey::new(h(0), 1), payload.clone());
                }
            },
            |_| {
                let _ = black_box(bus.finalize());
            },
        )
    });
}

criterion_group!(
    benches,
    bench_materialization_emit_log,
    bench_materialization_finalize_log,
    bench_materialization_emit_strict_many,
    bench_materialization_finalize_strict_many
);
criterion_main!(benches);
