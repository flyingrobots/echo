// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for optic attachment boundary semantics.

#![allow(clippy::panic, clippy::unwrap_used)]

use warp_core::{
    make_node_id, make_type_id, AttachmentDescentPolicy, AttachmentKey, CoordinateAt,
    EchoCoordinate, Engine, EngineBuilder, GraphStore, NodeKey, NodeRecord, ObservationService,
    ObserveOpticRequest, ObserveOpticResult, OpticAperture, OpticApertureShape, OpticCapabilityId,
    OpticFocus, OpticId, OpticObstructionKind, OpticReadBudget, ProjectionVersion,
    ProvenanceService, SchedulerKind, WorldlineId, WorldlineRuntime, WorldlineState,
};

struct OpticHarness {
    runtime: WorldlineRuntime,
    provenance: ProvenanceService,
    engine: Engine,
    worldline_id: WorldlineId,
    attachment_key: AttachmentKey,
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
    let attachment_key = AttachmentKey::node_alpha(NodeKey {
        warp_id: engine.root_key().warp_id,
        local_id: root,
    });

    OpticHarness {
        runtime,
        provenance,
        engine,
        worldline_id,
        attachment_key,
    }
}

fn attachment_request(
    harness: &OpticHarness,
    attachment_descent: AttachmentDescentPolicy,
    max_attachments: Option<u64>,
) -> ObserveOpticRequest {
    ObserveOpticRequest {
        optic_id: OpticId::from_bytes([90; 32]),
        focus: OpticFocus::AttachmentBoundary {
            key: harness.attachment_key,
        },
        coordinate: EchoCoordinate::Worldline {
            worldline_id: harness.worldline_id,
            at: CoordinateAt::Frontier,
        },
        aperture: OpticAperture {
            shape: OpticApertureShape::AttachmentBoundary,
            budget: OpticReadBudget {
                max_bytes: Some(256),
                max_nodes: Some(1),
                max_ticks: Some(1),
                max_attachments,
            },
            attachment_descent,
        },
        projection_version: ProjectionVersion::from_raw(1),
        reducer_version: None,
        capability: OpticCapabilityId::from_bytes([91; 32]),
    }
}

#[test]
fn attachment_boundary_read_without_descent_returns_boundary_posture() {
    let harness = harness();
    let request = attachment_request(&harness, AttachmentDescentPolicy::BoundaryOnly, Some(0));
    let result = ObservationService::observe_optic(
        &harness.runtime,
        &harness.provenance,
        &harness.engine,
        request,
    );

    let obstruction = match result {
        ObserveOpticResult::Obstructed(obstruction) => obstruction,
        ObserveOpticResult::Reading(reading) => {
            panic!("attachment boundary read should stop at boundary, got {reading:?}");
        }
    };

    assert_eq!(
        obstruction.kind,
        OpticObstructionKind::AttachmentDescentRequired
    );
    assert_eq!(
        obstruction.focus,
        Some(OpticFocus::AttachmentBoundary {
            key: harness.attachment_key
        })
    );
}

#[test]
fn attachment_boundary_explicit_descent_without_authority_is_denied() {
    let harness = harness();
    let request = attachment_request(&harness, AttachmentDescentPolicy::Explicit, Some(1));
    let result = ObservationService::observe_optic(
        &harness.runtime,
        &harness.provenance,
        &harness.engine,
        request,
    );

    let obstruction = match result {
        ObserveOpticResult::Obstructed(obstruction) => obstruction,
        ObserveOpticResult::Reading(reading) => {
            panic!("unauthorized attachment descent should obstruct, got {reading:?}");
        }
    };

    assert_eq!(
        obstruction.kind,
        OpticObstructionKind::AttachmentDescentDenied
    );
}

#[test]
fn attachment_boundary_explicit_descent_requires_attachment_budget() {
    let harness = harness();
    let request = attachment_request(&harness, AttachmentDescentPolicy::Explicit, Some(0));
    let result = ObservationService::observe_optic(
        &harness.runtime,
        &harness.provenance,
        &harness.engine,
        request,
    );

    let obstruction = match result {
        ObserveOpticResult::Obstructed(obstruction) => obstruction,
        ObserveOpticResult::Reading(reading) => {
            panic!("attachment descent without budget should obstruct, got {reading:?}");
        }
    };

    assert_eq!(obstruction.kind, OpticObstructionKind::BudgetExceeded);
}
