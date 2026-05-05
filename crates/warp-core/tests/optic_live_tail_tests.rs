// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for live-tail honesty in optic read identities.

#![allow(clippy::panic, clippy::unwrap_used)]

use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, CoordinateAt, Engine,
    EngineBuilder, GraphStore, IngressEnvelope, IngressTarget, IntentFamilyId, NodeRecord,
    ObservationPayload, ObservationService, ObserveOpticResult, OpticActorId, OpticReadBudget,
    ProvenanceService, SchedulerCoordinator, SchedulerKind, WitnessBasis, WorldlineHeadOptic,
    WorldlineId, WorldlineRuntime, WorldlineState,
};
use warp_core::{InboxPolicy, PlaybackMode, WriterHead, WriterHeadKey};

struct OpticHarness {
    runtime: WorldlineRuntime,
    provenance: ProvenanceService,
    engine: Engine,
    worldline_id: WorldlineId,
}

fn harness() -> OpticHarness {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );

    let engine = EngineBuilder::new(store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let worldline_id = WorldlineId::from_bytes(*engine.root_key().warp_id.as_bytes());
    let state = WorldlineState::try_from(engine.state().clone()).unwrap();
    let mut runtime = WorldlineRuntime::new();
    let mut provenance = ProvenanceService::new();
    provenance.register_worldline(worldline_id, &state).unwrap();
    runtime.register_worldline(worldline_id, state).unwrap();
    runtime
        .register_writer_head(WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("default"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .unwrap();

    OpticHarness {
        runtime,
        provenance,
        engine,
        worldline_id,
    }
}

fn commit(harness: &mut OpticHarness, label: &str) {
    harness
        .runtime
        .ingest(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: harness.worldline_id,
            },
            make_intent_kind("echo.intent/live-tail-test"),
            label.as_bytes().to_vec(),
        ))
        .unwrap();
    SchedulerCoordinator::super_tick(
        &mut harness.runtime,
        &mut harness.provenance,
        &mut harness.engine,
    )
    .unwrap();
}

fn optic(worldline_id: WorldlineId) -> WorldlineHeadOptic {
    WorldlineHeadOptic::open(
        worldline_id,
        CoordinateAt::Frontier,
        OpticActorId::from_bytes([3; 32]),
        warp_core::OpticCapabilityId::from_bytes([4; 32]),
        IntentFamilyId::from_bytes([5; 32]),
        [6; 32],
    )
    .unwrap()
}

fn metadata_budget() -> OpticReadBudget {
    OpticReadBudget {
        max_bytes: Some(1024),
        max_nodes: Some(8),
        max_ticks: Some(4),
        max_attachments: Some(0),
    }
}

#[test]
fn frontier_read_after_checkpoint_names_live_tail_witnesses() {
    let mut harness = harness();
    let optic = optic(harness.worldline_id);

    commit(&mut harness, "checkpoint-basis");
    let checkpoint_state = harness
        .runtime
        .worldlines()
        .get(&harness.worldline_id)
        .unwrap()
        .state();
    let checkpoint = harness
        .provenance
        .checkpoint(harness.worldline_id, checkpoint_state)
        .unwrap();
    let checkpoint_reading = match ObservationService::observe_optic(
        &harness.runtime,
        &harness.provenance,
        &harness.engine,
        optic.observe_head_request(metadata_budget()),
    ) {
        ObserveOpticResult::Reading(reading) => reading,
        ObserveOpticResult::Obstructed(obstruction) => {
            panic!("checkpoint optic read should succeed, got {obstruction:?}");
        }
    };

    commit(&mut harness, "live-tail");
    let live_reading = match ObservationService::observe_optic(
        &harness.runtime,
        &harness.provenance,
        &harness.engine,
        optic.observe_head_request(metadata_budget()),
    ) {
        ObserveOpticResult::Reading(reading) => reading,
        ObserveOpticResult::Obstructed(obstruction) => {
            panic!("live-tail optic read should succeed, got {obstruction:?}");
        }
    };

    assert_ne!(
        checkpoint_reading.read_identity.read_identity_hash,
        live_reading.read_identity.read_identity_hash
    );
    assert!(matches!(live_reading.payload, ObservationPayload::Head(_)));
    match live_reading.read_identity.witness_basis {
        WitnessBasis::CheckpointPlusTail {
            checkpoint_ref,
            checkpoint_hash,
            tail_witness_refs,
            tail_digest,
        } => {
            assert_eq!(checkpoint_ref.worldline_id, harness.worldline_id);
            assert_eq!(checkpoint_ref.worldline_tick.as_u64(), 0);
            assert_eq!(checkpoint_hash, checkpoint.state_hash);
            assert_eq!(tail_witness_refs.len(), 1);
            assert_eq!(tail_witness_refs[0].worldline_id, harness.worldline_id);
            assert_eq!(tail_witness_refs[0].worldline_tick.as_u64(), 1);
            assert_ne!(tail_digest, [0; 32]);
        }
        witness_basis => {
            panic!("expected checkpoint-plus-tail witness basis, got {witness_basis:?}");
        }
    }
}
