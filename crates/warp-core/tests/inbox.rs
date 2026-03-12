// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Runtime-owned ingress integration tests.

use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, Engine, EngineBuilder, GraphStore,
    InboxAddress, InboxPolicy, IngressDisposition, IngressEnvelope, IngressTarget, NodeId,
    NodeRecord, PlaybackMode, SchedulerCoordinator, WorldlineId, WorldlineRuntime, WorldlineState,
    WriterHead, WriterHeadKey,
};

fn wl(n: u8) -> WorldlineId {
    WorldlineId([n; 32])
}

fn empty_engine() -> Engine {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    EngineBuilder::new(store, root).build()
}

fn register_head(
    runtime: &mut WorldlineRuntime,
    worldline_id: WorldlineId,
    label: &str,
    public_inbox: Option<&str>,
    is_default_writer: bool,
) -> WriterHeadKey {
    let key = WriterHeadKey {
        worldline_id,
        head_id: make_head_id(label),
    };
    runtime
        .register_writer_head(WriterHead::with_routing(
            key,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            public_inbox.map(|name| InboxAddress(name.to_owned())),
            is_default_writer,
        ))
        .unwrap();
    key
}

fn runtime_store(runtime: &WorldlineRuntime, worldline_id: WorldlineId) -> &GraphStore {
    let frontier = runtime.worldlines.get(&worldline_id).unwrap();
    frontier
        .state()
        .warp_state()
        .store(&frontier.state().root().warp_id)
        .unwrap()
}

#[test]
fn runtime_ingest_commits_without_legacy_graph_inbox_nodes() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);

    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"runtime-intent".to_vec(),
    );
    assert_eq!(
        runtime.ingest(envelope.clone()).unwrap(),
        IngressDisposition::Accepted {
            ingress_id: envelope.ingress_id(),
            head_key,
        }
    );

    let records = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].head_key, head_key);

    let store = runtime_store(&runtime, worldline_id);
    assert!(store.node(&NodeId(envelope.ingress_id())).is_some());
    assert!(store.node(&make_node_id("sim")).is_none());
    assert!(store.node(&make_node_id("sim/inbox")).is_none());
}

#[test]
fn runtime_ingest_is_idempotent_per_resolved_head_after_commit() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let default_key = register_head(&mut runtime, worldline_id, "default", None, true);
    let named_key = register_head(&mut runtime, worldline_id, "orders", Some("orders"), false);

    let kind = make_intent_kind("test/runtime");
    let default_env = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        kind,
        b"same-intent".to_vec(),
    );
    let named_env = IngressEnvelope::local_intent(
        IngressTarget::InboxAddress {
            worldline_id,
            inbox: InboxAddress("orders".to_owned()),
        },
        kind,
        b"same-intent".to_vec(),
    );

    assert_eq!(
        runtime.ingest(default_env.clone()).unwrap(),
        IngressDisposition::Accepted {
            ingress_id: default_env.ingress_id(),
            head_key: default_key,
        }
    );
    SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();

    assert_eq!(
        runtime.ingest(default_env).unwrap(),
        IngressDisposition::Duplicate {
            ingress_id: named_env.ingress_id(),
            head_key: default_key,
        }
    );
    assert_eq!(
        runtime.ingest(named_env.clone()).unwrap(),
        IngressDisposition::Accepted {
            ingress_id: named_env.ingress_id(),
            head_key: named_key,
        }
    );
}

#[test]
fn runtime_ingest_keeps_distinct_intents_as_distinct_event_nodes() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);

    let intent_a = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"intent-alpha".to_vec(),
    );
    let intent_b = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"intent-beta".to_vec(),
    );

    runtime.ingest(intent_a.clone()).unwrap();
    runtime.ingest(intent_b.clone()).unwrap();

    let records = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].admitted_count, 2);

    let store = runtime_store(&runtime, worldline_id);
    assert!(store.node(&NodeId(intent_a.ingress_id())).is_some());
    assert!(store.node(&NodeId(intent_b.ingress_id())).is_some());
}
