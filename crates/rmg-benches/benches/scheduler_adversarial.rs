#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use rand::Rng;
use rustc_hash::FxHashMap;

/// Key type that forces all entries into the same hash bucket (constant hash).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Colliding(u64);

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
                    let mut rng = rand::thread_rng();
                    let mut map: FxHashMap<u64, u64> = FxHashMap::default();
                    for _ in 0..n {
                        let k = rng.gen::<u64>();
                        map.insert(k, k);
                    }
                    map
                },
                |mut map| {
                    let mut rng = rand::thread_rng();
                    let k = rng.gen::<u64>();
                    let _ = map.get(&k);
                    map.insert(k, k);
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
