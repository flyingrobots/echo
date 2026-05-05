// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for the narrow worldline-head optic example.

#![allow(clippy::panic, clippy::unwrap_used)]

use warp_core::{
    make_node_id, make_type_id, AdmissionLawId, CoordinateAt, EchoCoordinate, Engine,
    EngineBuilder, GraphStore, IntentFamilyId, NodeRecord, ObservationPayload, ObservationService,
    ObserveOpticResult, OpticActorId, OpticObstructionKind, OpticReadBudget, ProvenanceService,
    ReadingBudgetPosture, SchedulerKind, WorldlineHeadOptic, WorldlineId, WorldlineRuntime,
    WorldlineState, WorldlineTick,
};

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

    OpticHarness {
        runtime,
        provenance,
        engine,
        worldline_id,
    }
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
fn worldline_head_optic_example_reads_bounded_head() {
    let harness = harness();
    let optic = optic(harness.worldline_id);
    let request = optic.observe_head_request(metadata_budget());

    let result = ObservationService::observe_optic(
        &harness.runtime,
        &harness.provenance,
        &harness.engine,
        request.clone(),
    );

    let reading = match result {
        ObserveOpticResult::Reading(reading) => reading,
        ObserveOpticResult::Obstructed(obstruction) => {
            panic!("worldline head optic should read, got {obstruction:?}");
        }
    };

    assert_eq!(reading.read_identity.optic_id, optic.optic.optic_id);
    assert_eq!(reading.read_identity.coordinate, request.coordinate);
    assert_eq!(
        reading.read_identity.projection_version,
        request.projection_version
    );
    assert!(matches!(reading.payload, ObservationPayload::Head(_)));
    assert!(matches!(
        reading.envelope.budget_posture,
        ReadingBudgetPosture::Bounded {
            max_payload_bytes: 1024,
            ..
        }
    ));
}

#[test]
fn worldline_head_optic_example_query_shape_obstructs_typed() {
    let harness = harness();
    let optic = optic(harness.worldline_id);
    let request = optic.observe_query_bytes_request(17, [9; 32], metadata_budget());

    let result = ObservationService::observe_optic(
        &harness.runtime,
        &harness.provenance,
        &harness.engine,
        request,
    );

    let obstruction = match result {
        ObserveOpticResult::Reading(reading) => {
            panic!("query-shaped example optic should obstruct, got {reading:?}");
        }
        ObserveOpticResult::Obstructed(obstruction) => obstruction,
    };

    assert_eq!(
        obstruction.kind,
        OpticObstructionKind::UnsupportedProjectionLaw
    );
    assert_eq!(obstruction.optic_id, Some(optic.optic.optic_id));
}

#[test]
fn worldline_head_optic_example_dispatches_eint_with_explicit_base() {
    let harness = harness();
    let optic = optic(harness.worldline_id);
    let base_coordinate = EchoCoordinate::Worldline {
        worldline_id: harness.worldline_id,
        at: CoordinateAt::Tick(WorldlineTick::from_raw(0)),
    };
    let request = optic.dispatch_eint_v1_request(
        base_coordinate.clone(),
        warp_core::OpticCause {
            actor: optic.capability.actor,
            cause_hash: [10; 32],
            label: Some("example eint proposal".to_owned()),
        },
        AdmissionLawId::from_bytes([11; 32]),
        echo_wasm_abi::pack_intent_v1(77, b"example-vars").unwrap(),
    );

    request
        .validate_proposal_against_current(&base_coordinate)
        .unwrap();
    assert_eq!(request.base_coordinate, base_coordinate);
}

#[test]
fn worldline_head_optic_example_stale_base_obstructs() {
    let harness = harness();
    let optic = optic(harness.worldline_id);
    let request = optic.dispatch_eint_v1_request(
        EchoCoordinate::Worldline {
            worldline_id: harness.worldline_id,
            at: CoordinateAt::Tick(WorldlineTick::from_raw(0)),
        },
        warp_core::OpticCause {
            actor: optic.capability.actor,
            cause_hash: [10; 32],
            label: Some("stale example proposal".to_owned()),
        },
        AdmissionLawId::from_bytes([11; 32]),
        echo_wasm_abi::pack_intent_v1(77, b"example-vars").unwrap(),
    );
    let current = EchoCoordinate::Worldline {
        worldline_id: harness.worldline_id,
        at: CoordinateAt::Tick(WorldlineTick::from_raw(1)),
    };

    let obstruction = request
        .validate_proposal_against_current(&current)
        .unwrap_err();

    assert_eq!(obstruction.kind, OpticObstructionKind::StaleBasis);
}
