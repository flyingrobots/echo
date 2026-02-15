// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// criterion_group!/criterion_main! expand to undocumented functions that cannot
// carry #[allow] (attributes on macro invocations are ignored). Crate-level
// suppress is required for benchmark binaries using Criterion.
#![allow(missing_docs)]
//! Microbenchmarks for `MaterializationBus` performance.
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
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
    let payloads: Vec<Vec<u8>> = (0..1000).map(|_| vec![0u8; 64]).collect();

    c.bench_function("materialization_emit_log_1000", |b| {
        b.iter(|| {
            for (i, p) in payloads.iter().enumerate() {
                bus.emit(
                    black_box(ch),
                    black_box(EmitKey::new(h(i as u64), 1)),
                    black_box(p.clone()),
                )
                .unwrap();
            }
            bus.clear();
        })
    });
}

/// Benchmark finalizing a single `Log` channel with 1000 items.
fn bench_materialization_finalize_log(c: &mut Criterion) {
    let bus = MaterializationBus::new();
    let ch = make_channel_id("bench:log");
    let payloads: Vec<Vec<u8>> = (0..1000).map(|_| vec![0u8; 64]).collect();

    c.bench_function("materialization_finalize_log_1000", |b| {
        b.iter_batched(
            || {
                for (i, p) in payloads.iter().enumerate() {
                    bus.emit(ch, EmitKey::new(h(i as u64), 1), p.clone())
                        .unwrap();
                }
            },
            |_| {
                let _ = black_box(bus.finalize());
            },
            BatchSize::PerIteration,
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
    let payloads: Vec<Vec<u8>> = (0..1000).map(|_| vec![0u8; 64]).collect();

    c.bench_function("materialization_emit_strict_1000", |b| {
        b.iter(|| {
            for (i, ch) in channels.iter().enumerate() {
                bus.emit(
                    black_box(*ch),
                    black_box(EmitKey::new(h(0), 1)),
                    black_box(payloads[i].clone()),
                )
                .unwrap();
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
    let payloads: Vec<Vec<u8>> = (0..1000).map(|_| vec![0u8; 64]).collect();

    c.bench_function("materialization_finalize_strict_1000", |b| {
        b.iter_batched(
            || {
                for (i, ch) in channels.iter().enumerate() {
                    bus.emit(*ch, EmitKey::new(h(0), 1), payloads[i].clone())
                        .unwrap();
                }
            },
            |_| {
                let _ = black_box(bus.finalize());
            },
            BatchSize::PerIteration,
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
