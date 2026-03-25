// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Runtime-owned ingress integration tests.

use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, Engine, EngineBuilder, GraphStore,
    InboxAddress, InboxPolicy, IngressDisposition, IngressEnvelope, IngressTarget, NodeId,
    NodeRecord, PlaybackMode, ProvenanceEventKind, ProvenanceService, ProvenanceStore,
    SchedulerCoordinator, SchedulerKind, WorldlineId, WorldlineRuntime, WorldlineState,
    WorldlineTick, WorldlineTickPatchV1, WriterHead, WriterHeadKey,
};

fn wl(n: u8) -> WorldlineId {
    WorldlineId::from_bytes([n; 32])
}

fn wt(raw: u64) -> WorldlineTick {
    WorldlineTick::from_raw(raw)
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
    EngineBuilder::new(store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build()
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
    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    frontier
        .state()
        .warp_state()
        .store(&frontier.state().root().warp_id)
        .unwrap()
}

fn registered_worldlines_provenance(runtime: &WorldlineRuntime) -> ProvenanceService {
    let mut provenance = ProvenanceService::new();
    for (worldline_id, frontier) in runtime.worldlines().iter() {
        provenance
            .register_worldline(*worldline_id, frontier.state())
            .unwrap();
    }
    provenance
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

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
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
    let default_ingress_id = default_env.ingress_id();

    assert_eq!(
        runtime.ingest(default_env.clone()).unwrap(),
        IngressDisposition::Accepted {
            ingress_id: default_ingress_id,
            head_key: default_key,
        }
    );
    let mut provenance = registered_worldlines_provenance(&runtime);
    SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

    assert_eq!(
        runtime.ingest(default_env).unwrap(),
        IngressDisposition::Duplicate {
            ingress_id: default_ingress_id,
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
    SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    let named_ingress_id = named_env.ingress_id();
    assert_eq!(
        runtime.ingest(named_env).unwrap(),
        IngressDisposition::Duplicate {
            ingress_id: named_ingress_id,
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

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].admitted_count, 2);

    let store = runtime_store(&runtime, worldline_id);
    assert!(store.node(&NodeId(intent_a.ingress_id())).is_some());
    assert!(store.node(&NodeId(intent_b.ingress_id())).is_some());
}

#[test]
fn runtime_commit_patch_replays_to_post_state() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);

    runtime
        .ingest(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test/runtime"),
            b"patch-replay".to_vec(),
        ))
        .unwrap();

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert_eq!(records.len(), 1);

    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    let (snapshot, _receipt, patch) = frontier.state().tick_history().last().unwrap().clone();

    let mut replay_state = frontier.state().initial_state().clone();
    patch.apply_to_state(&mut replay_state).unwrap();
    let replay_root = engine
        .snapshot_for_state(&WorldlineState::new(replay_state, *frontier.state().root()).unwrap())
        .state_root;

    assert_eq!(
        replay_root, snapshot.state_root,
        "runtime tick patch must replay to the committed post-state"
    );
}

#[test]
fn runtime_commit_provenance_matches_worldline_state_mirror() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);

    runtime
        .ingest(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test/runtime"),
            b"mirror-consistency".to_vec(),
        ))
        .unwrap();

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert_eq!(records.len(), 1);

    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    let state = frontier.state();
    let (snapshot, _receipt, patch) = state.tick_history().last().unwrap().clone();
    let entry = provenance.entry(worldline_id, wt(0)).unwrap();
    let expected_outputs = state
        .last_materialization()
        .iter()
        .map(|channel| (channel.channel, channel.data.clone()))
        .collect::<Vec<_>>();

    assert_eq!(entry.worldline_id, worldline_id);
    assert_eq!(entry.worldline_tick, wt(0));
    assert_eq!(entry.commit_global_tick, runtime.global_tick());
    assert_eq!(entry.head_key, Some(head_key));
    assert!(matches!(entry.event_kind, ProvenanceEventKind::LocalCommit));
    assert!(
        entry.parents.is_empty(),
        "first local commit should be parentless"
    );
    assert_eq!(entry.expected.state_root, snapshot.state_root);
    assert_eq!(entry.expected.patch_digest, snapshot.patch_digest);
    assert_eq!(entry.expected.commit_hash, snapshot.hash);
    assert_eq!(entry.outputs, expected_outputs);

    let expected_patch = WorldlineTickPatchV1 {
        header: warp_core::WorldlineTickHeaderV1 {
            commit_global_tick: runtime.global_tick(),
            policy_id: patch.policy_id(),
            rule_pack_id: patch.rule_pack_id(),
            plan_digest: snapshot.plan_digest,
            decision_digest: snapshot.decision_digest,
            rewrites_digest: snapshot.rewrites_digest,
        },
        warp_id: snapshot.root.warp_id,
        ops: patch.ops().to_vec(),
        in_slots: patch.in_slots().to_vec(),
        out_slots: patch.out_slots().to_vec(),
        patch_digest: patch.digest(),
    };
    assert_eq!(entry.patch, Some(expected_patch));
}
