// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// criterion_group!/criterion_main! expand to undocumented functions that cannot
// carry #[allow] (attributes on macro invocations are ignored). Crate-level
// suppress is required for benchmark binaries using Criterion.
#![allow(missing_docs)]
//! BOAW Phase 6B performance baseline benchmarks.
//!
//! Measures parallel vs serial execution across different workload sizes
//! and worker counts. Use these baselines to detect regressions in future phases.
//!
//! # Running
//!
//! ```sh
//! cargo +nightly bench --package warp-benches --bench boaw_baseline
//! ```
//!
//! # What This Measures
//!
//! - `serial_vs_parallel_N`: Compare parallel sharded execution vs serial baseline
//! - `work_queue_pipeline_N`: Full Phase 6B pipeline (build_work_units → execute_work_queue)
//! - `worker_scaling_100`: How throughput scales with worker count (1, 2, 4, 8, 16)
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use std::collections::BTreeMap;
use std::time::Duration;
use warp_core::boaw::{build_work_units, execute_work_queue};
use warp_core::{
    execute_parallel, execute_serial, make_node_id, make_type_id, make_warp_id, AtomPayload,
    AttachmentKey, AttachmentValue, ExecItem, GraphStore, GraphView, NodeId, NodeKey, NodeRecord,
    OpOrigin, TickDelta, WarpId, WarpOp,
};

/// Simple executor that sets an attachment on the scope node.
fn touch_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let payload = AtomPayload::new(
        make_type_id("bench/marker"),
        bytes::Bytes::from_static(b"touched"),
    );
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: *scope,
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(payload)),
    });
}

/// Create a test graph with N independent nodes.
fn make_test_store(n: usize) -> (GraphStore, Vec<NodeId>) {
    let node_ty = make_type_id("bench/node");
    let mut store = GraphStore::default();
    let mut nodes = Vec::with_capacity(n);

    for i in 0..n {
        let id = make_node_id(&format!("bench/n{i}"));
        store.insert_node(id, NodeRecord { ty: node_ty });
        nodes.push(id);
    }

    (store, nodes)
}

/// Create ExecItems for all nodes with proper origins.
fn make_exec_items(nodes: &[NodeId]) -> Vec<ExecItem> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, &scope)| ExecItem {
            exec: touch_executor,
            scope,
            origin: OpOrigin {
                intent_id: i as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        })
        .collect()
}

// =============================================================================
// Serial vs Parallel comparison at different workload sizes
// =============================================================================

/// Compares serial vs parallel sharded execution across workload sizes (10, 100, 1000 items).
fn bench_serial_vs_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("serial_vs_parallel");
    group
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(50);

    for &n in &[10usize, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));

        // Serial execution
        group.bench_with_input(BenchmarkId::new("serial", n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let (store, nodes) = make_test_store(n);
                    let items = make_exec_items(&nodes);
                    (store, items)
                },
                |(store, items)| {
                    let view = GraphView::new(&store);
                    let delta = execute_serial(view, &items);
                    criterion::black_box(delta)
                },
                BatchSize::SmallInput,
            )
        });

        // Parallel execution with 4 workers
        group.bench_with_input(BenchmarkId::new("parallel_4w", n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let (store, nodes) = make_test_store(n);
                    let items = make_exec_items(&nodes);
                    (store, items)
                },
                |(store, items)| {
                    let view = GraphView::new(&store);
                    let deltas = execute_parallel(view, &items, 4);
                    criterion::black_box(deltas)
                },
                BatchSize::SmallInput,
            )
        });

        // Phase 6B work-queue pipeline
        group.bench_with_input(BenchmarkId::new("workqueue_4w", n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let (store, nodes) = make_test_store(n);
                    let items = make_exec_items(&nodes);
                    let warp_id = make_warp_id("bench/warp-0");
                    (store, items, warp_id)
                },
                |(store, items, warp_id)| {
                    let by_warp = vec![(warp_id, items)];
                    let units = build_work_units(by_warp);
                    let stores: BTreeMap<WarpId, GraphStore> =
                        [(warp_id, store)].into_iter().collect();
                    let deltas = execute_work_queue(&units, 4, |wid| stores.get(wid))
                        .expect("work queue should succeed");
                    criterion::black_box(deltas)
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

// =============================================================================
// Phase 6B work-queue + merge pipeline
// =============================================================================

/// Create a multi-warp test setup with `num_warps` warps, each having `items_per_warp` nodes.
fn make_multi_warp_setup(
    num_warps: usize,
    items_per_warp: usize,
) -> (
    BTreeMap<WarpId, GraphStore>,
    BTreeMap<WarpId, Vec<ExecItem>>,
) {
    let mut stores = BTreeMap::new();
    let mut items_by_warp = BTreeMap::new();

    for w in 0..num_warps {
        let warp_id = make_warp_id(&format!("bench/warp-{w}"));
        let node_ty = make_type_id("bench/node");
        let mut store = GraphStore::default();
        let mut items = Vec::with_capacity(items_per_warp);

        for i in 0..items_per_warp {
            let id = make_node_id(&format!("bench/w{w}/n{i}"));
            store.insert_node(id, NodeRecord { ty: node_ty });
            items.push(ExecItem {
                exec: touch_executor,
                scope: id,
                origin: OpOrigin {
                    intent_id: (w * items_per_warp + i) as u64,
                    rule_id: 1,
                    match_ix: 0,
                    op_ix: 0,
                },
            });
        }

        stores.insert(warp_id, store);
        items_by_warp.insert(warp_id, items);
    }

    (stores, items_by_warp)
}

/// Benchmarks the Phase 6B work-queue pipeline (build_work_units → execute_work_queue) across multi-warp setups.
fn bench_work_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("work_queue_pipeline");
    group
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(50);

    // Vary total items across multiple warps (4 warps × N items each)
    let num_warps = 4;
    for &items_per_warp in &[10usize, 100, 250] {
        let total = num_warps * items_per_warp;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("build_and_execute_4w", total),
            &items_per_warp,
            |b, &ipw| {
                b.iter_batched(
                    || make_multi_warp_setup(num_warps, ipw),
                    |(stores, items_by_warp)| {
                        let units = build_work_units(items_by_warp.into_iter());
                        // Cap workers at 4 but never more than the number of
                        // work units; max(1) prevents zero-division on empty input.
                        let workers = 4.min(units.len().max(1));
                        let deltas =
                            execute_work_queue(&units, workers, |warp_id| stores.get(warp_id))
                                .expect("bench: all stores exist");
                        criterion::black_box(deltas)
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

// =============================================================================
// Worker scaling at fixed workload (100 items)
// =============================================================================

/// Measures throughput scaling as worker count increases (1, 2, 4, 8, 16 workers) with a fixed 100-item workload.
fn bench_worker_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("worker_scaling_100");
    group
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(50);

    const WORKLOAD_SIZE: usize = 100;
    group.throughput(Throughput::Elements(WORKLOAD_SIZE as u64));

    for &workers in &[1usize, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{workers}w")),
            &workers,
            |b, &workers| {
                b.iter_batched(
                    || {
                        let (store, nodes) = make_test_store(WORKLOAD_SIZE);
                        let items = make_exec_items(&nodes);
                        (store, items)
                    },
                    |(store, items)| {
                        let view = GraphView::new(&store);
                        let deltas = execute_parallel(view, &items, workers);
                        criterion::black_box(deltas)
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_serial_vs_parallel,
    bench_work_queue,
    bench_worker_scaling
);
criterion_main!(benches);
