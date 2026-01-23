// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(missing_docs)]
//! BOAW Phase 6B performance baseline benchmarks.
//!
//! Measures parallel vs serial execution across different workload sizes
//! and worker counts. Use these baselines to detect regressions in future phases.
//!
//! # Running
//!
//! ```sh
//! cargo bench --package warp-benches --bench boaw_baseline
//! ```
//!
//! # What This Measures
//!
//! - `serial_vs_parallel_N`: Compare parallel sharded execution vs serial baseline
//! - `worker_scaling_100`: How throughput scales with worker count (1, 2, 4, 8, 16)
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use warp_core::{
    execute_parallel, execute_serial, make_node_id, make_type_id, AtomPayload, AttachmentKey,
    AttachmentValue, ExecItem, GraphStore, GraphView, NodeId, NodeKey, NodeRecord, OpOrigin,
    TickDelta, WarpOp,
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
    }
    group.finish();
}

// TODO: Add benchmark for full work-queue + merge pipeline (Phase 6B path)

// =============================================================================
// Worker scaling at fixed workload (100 items)
// =============================================================================

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

criterion_group!(benches, bench_serial_vs_parallel, bench_worker_scaling);
criterion_main!(benches);
