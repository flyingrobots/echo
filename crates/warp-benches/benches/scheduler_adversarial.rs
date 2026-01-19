// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use rustc_hash::FxHashMap;
use warp_core::math::Prng;

/// Key type that forces all entries into the same hash bucket (constant hash).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Colliding(u64);

#[inline]
fn prng_u64(prng: &mut Prng) -> u64 {
    // `Prng` currently exposes `next_int`/`next_f32`. Compose two i32 samples into
    // a stable 64-bit key without reaching for OS-backed RNGs.
    let hi = prng.next_int(i32::MIN, i32::MAX) as u32 as u64;
    let lo = prng.next_int(i32::MIN, i32::MAX) as u32 as u64;
    (hi << 32) | lo
}

fn bench_fxhash_collision(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_adversarial/colliding");
    for &n in &[1_000u64, 5_000, 10_000] {
        group.bench_function(format!("insert_and_probe/{n}"), |b| {
            b.iter_batched(
                || {
                    let mut map: FxHashMap<Colliding, u64> = FxHashMap::default();
                    // pre-seed with colliding keys
                    for i in 0..n {
                        map.insert(Colliding(i), i);
                    }
                    map
                },
                |mut map| {
                    // probe and insert another colliding key
                    let key = Colliding(n + 1);
                    let _ = map.get(&key);
                    map.insert(key, n + 1);
                    black_box(map);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_fxhash_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_adversarial/random");
    for &n in &[1_000u64, 5_000, 10_000] {
        group.bench_function(format!("insert_and_probe/{n}"), |b| {
            b.iter_batched(
                || {
                    // Deterministic input generation: benchmarks must not depend on
                    // process-local / OS-backed randomness.
                    let mut prng = Prng::from_seed(0x45_43_48_4f_2d_62_65_6e, n);
                    let mut map: FxHashMap<u64, u64> = FxHashMap::default();
                    for _ in 0..n {
                        let k = prng_u64(&mut prng);
                        map.insert(k, k);
                    }
                    let probe = prng_u64(&mut prng);
                    (map, probe)
                },
                |(mut map, probe)| {
                    let _ = map.get(&probe);
                    map.insert(probe, probe);
                    black_box(map);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_fxhash_collision, bench_fxhash_random);
criterion_main!(benches);
