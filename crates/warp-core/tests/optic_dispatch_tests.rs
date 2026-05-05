// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for Echo optic intent dispatch semantics.

#![allow(clippy::unwrap_used)]

use warp_core::{
    AdmissionLawId, CoordinateAt, DispatchOpticIntentRequest, EchoCoordinate, IntentFamilyId,
    OpticActorId, OpticCapability, OpticCapabilityId, OpticCause, OpticFocus, OpticId,
    OpticIntentPayload, OpticObstructionKind, OpticReadBudget, ProjectionVersion, WorldlineId,
    WorldlineTick,
};

fn worldline(seed: u8) -> WorldlineId {
    WorldlineId::from_bytes([seed; 32])
}

fn intent_family(seed: u8) -> IntentFamilyId {
    IntentFamilyId::from_bytes([seed; 32])
}

fn actor(seed: u8) -> OpticActorId {
    OpticActorId::from_bytes([seed; 32])
}

fn dispatch_request(base_tick: u64) -> DispatchOpticIntentRequest {
    let worldline_id = worldline(3);
    let focus = OpticFocus::Worldline { worldline_id };
    let actor = actor(4);
    let intent_family = intent_family(5);

    DispatchOpticIntentRequest {
        optic_id: OpticId::from_bytes([1; 32]),
        base_coordinate: EchoCoordinate::Worldline {
            worldline_id,
            at: CoordinateAt::Tick(WorldlineTick::from_raw(base_tick)),
        },
        intent_family,
        focus: focus.clone(),
        cause: OpticCause {
            actor,
            cause_hash: [6; 32],
            label: Some("stale basis test".into()),
        },
        capability: OpticCapability {
            capability_id: OpticCapabilityId::from_bytes([7; 32]),
            actor,
            issuer_ref: None,
            policy_hash: [8; 32],
            allowed_focus: focus,
            projection_version: ProjectionVersion::from_raw(1),
            reducer_version: None,
            allowed_intent_family: intent_family,
            max_budget: OpticReadBudget {
                max_bytes: Some(4096),
                max_nodes: Some(64),
                max_ticks: Some(8),
                max_attachments: Some(0),
            },
        },
        admission_law: AdmissionLawId::from_bytes([9; 32]),
        payload: OpticIntentPayload::EintV1 {
            bytes: echo_wasm_abi::pack_intent_v1(77, b"optic-vars").unwrap(),
        },
    }
}

#[test]
fn stale_worldline_base_coordinate_obstructs_before_dispatch() {
    let worldline_id = worldline(3);
    let current_coordinate = EchoCoordinate::Worldline {
        worldline_id,
        at: CoordinateAt::Tick(WorldlineTick::from_raw(2)),
    };

    let obstruction = dispatch_request(1)
        .validate_proposal_against_current(&current_coordinate)
        .unwrap_err();

    assert_eq!(obstruction.kind, OpticObstructionKind::StaleBasis);
    assert_eq!(
        obstruction.coordinate,
        Some(dispatch_request(1).base_coordinate)
    );
}

#[test]
fn matching_worldline_base_coordinate_remains_dispatchable() {
    let worldline_id = worldline(3);
    let current_coordinate = EchoCoordinate::Worldline {
        worldline_id,
        at: CoordinateAt::Tick(WorldlineTick::from_raw(2)),
    };

    dispatch_request(2)
        .validate_proposal_against_current(&current_coordinate)
        .unwrap();
}
