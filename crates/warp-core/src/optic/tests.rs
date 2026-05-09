// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use super::*;
use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane};
use crate::ident::{EdgeId, EdgeKey, NodeId, NodeKey, TypeId, WarpId};
use crate::observation::{
    BuiltinObserverPlan, ObservationBasisPosture, ReadingBudgetPosture, ReadingObserverBasis,
    ReadingObserverPlan, ReadingResidualPosture, ReadingRightsPosture, ReadingWitnessRef,
};
use crate::provenance_store::ProvenanceRef;
use crate::strand::StrandId;
use crate::worldline::WorldlineId;

fn worldline(seed: u8) -> WorldlineId {
    WorldlineId::from_bytes([seed; 32])
}

fn strand(seed: u8) -> StrandId {
    StrandId::from_bytes([seed; 32])
}

fn braid(seed: u8) -> BraidId {
    BraidId::from_bytes([seed; 32])
}

fn retained(seed: u8) -> RetainedReadingKey {
    RetainedReadingKey::from_bytes([seed; 32])
}

fn retained_codec(seed: u8) -> RetainedReadingCodecId {
    RetainedReadingCodecId::from_bytes([seed; 32])
}

fn intent_family(seed: u8) -> IntentFamilyId {
    IntentFamilyId::from_bytes([seed; 32])
}

fn capability(seed: u8) -> OpticCapabilityId {
    OpticCapabilityId::from_bytes([seed; 32])
}

fn admission_law(seed: u8) -> AdmissionLawId {
    AdmissionLawId::from_bytes([seed; 32])
}

fn actor(seed: u8) -> OpticActorId {
    OpticActorId::from_bytes([seed; 32])
}

fn cause(seed: u8) -> OpticCause {
    OpticCause {
        actor: actor(seed),
        cause_hash: [seed.wrapping_add(1); 32],
        label: Some("test cause".to_owned()),
    }
}

fn node_key(seed: u8) -> NodeKey {
    NodeKey {
        warp_id: WarpId([seed; 32]),
        local_id: NodeId([seed.wrapping_add(1); 32]),
    }
}

fn edge_key(seed: u8) -> EdgeKey {
    EdgeKey {
        warp_id: WarpId([seed; 32]),
        local_id: EdgeId([seed.wrapping_add(1); 32]),
    }
}

fn provenance(seed: u8, tick: u64) -> ProvenanceRef {
    ProvenanceRef {
        worldline_id: worldline(seed),
        worldline_tick: crate::clock::WorldlineTick::from_raw(tick),
        commit_hash: [seed.wrapping_add(1); 32],
    }
}

fn worldline_focus() -> OpticFocus {
    OpticFocus::Worldline {
        worldline_id: worldline(1),
    }
}

fn frontier_coordinate() -> EchoCoordinate {
    EchoCoordinate::Worldline {
        worldline_id: worldline(1),
        at: CoordinateAt::Frontier,
    }
}

fn head_aperture() -> OpticAperture {
    OpticAperture {
        shape: OpticApertureShape::Head,
        budget: OpticReadBudget {
            max_bytes: Some(512),
            max_nodes: Some(8),
            max_ticks: Some(1),
            max_attachments: Some(0),
        },
        attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
    }
}

fn witness_basis(seed: u8, tick: u64) -> WitnessBasis {
    let reference = provenance(seed, tick);
    WitnessBasis::ResolvedCommit {
        reference,
        state_root: [seed.wrapping_add(2); 32],
        commit_hash: reference.commit_hash,
    }
}

fn reading_envelope() -> ReadingEnvelope {
    ReadingEnvelope {
        observer_plan: ReadingObserverPlan::Builtin {
            plan: BuiltinObserverPlan::CommitBoundaryHead,
        },
        observer_instance: None,
        observer_basis: ReadingObserverBasis::CommitBoundary,
        witness_refs: vec![ReadingWitnessRef::ResolvedCommit {
            reference: provenance(1, 2),
        }],
        parent_basis_posture: ObservationBasisPosture::Worldline,
        budget_posture: ReadingBudgetPosture::UnboundedOneShot,
        rights_posture: ReadingRightsPosture::KernelPublic,
        residual_posture: ReadingResidualPosture::Complete,
    }
}

fn optic_capability(seed: u8, focus: OpticFocus) -> OpticCapability {
    OpticCapability {
        capability_id: capability(seed),
        actor: actor(seed),
        issuer_ref: Some(provenance(seed, 1)),
        policy_hash: [seed.wrapping_add(2); 32],
        allowed_focus: focus,
        projection_version: ProjectionVersion::from_raw(1),
        reducer_version: None,
        allowed_intent_family: intent_family(seed),
        max_budget: OpticReadBudget {
            max_bytes: Some(4096),
            max_nodes: Some(128),
            max_ticks: Some(8),
            max_attachments: Some(0),
        },
    }
}

#[test]
fn echo_optic_id_is_stable_and_descriptor_derived() {
    let focus = OpticFocus::Worldline {
        worldline_id: worldline(1),
    };
    let coordinate = EchoCoordinate::Worldline {
        worldline_id: worldline(1),
        at: CoordinateAt::Frontier,
    };

    let first = EchoOptic::new(
        focus.clone(),
        coordinate.clone(),
        ProjectionVersion::from_raw(1),
        Some(ReducerVersion::from_raw(7)),
        intent_family(4),
        capability(5),
    );
    let second = EchoOptic::new(
        focus,
        coordinate,
        ProjectionVersion::from_raw(1),
        Some(ReducerVersion::from_raw(7)),
        intent_family(4),
        capability(5),
    );

    assert_eq!(first.optic_id, second.optic_id);

    let changed_projection = EchoOptic::new(
        OpticFocus::Worldline {
            worldline_id: worldline(1),
        },
        EchoCoordinate::Worldline {
            worldline_id: worldline(1),
            at: CoordinateAt::Frontier,
        },
        ProjectionVersion::from_raw(2),
        Some(ReducerVersion::from_raw(7)),
        intent_family(4),
        capability(5),
    );

    assert_ne!(first.optic_id, changed_projection.optic_id);
}

#[test]
fn optic_focus_covers_generic_subjects_without_graph_handle() {
    let focuses = vec![
        OpticFocus::Worldline {
            worldline_id: worldline(1),
        },
        OpticFocus::Strand {
            strand_id: strand(2),
        },
        OpticFocus::Braid { braid_id: braid(3) },
        OpticFocus::RetainedReading { key: retained(4) },
        OpticFocus::AttachmentBoundary {
            key: AttachmentKey {
                owner: AttachmentOwner::Node(node_key(5)),
                plane: AttachmentPlane::Alpha,
            },
        },
        OpticFocus::AttachmentBoundary {
            key: AttachmentKey {
                owner: AttachmentOwner::Edge(edge_key(6)),
                plane: AttachmentPlane::Beta,
            },
        },
    ];

    for focus in focuses {
        let encoded = focus.to_abi();
        assert!(matches!(
            encoded,
            echo_wasm_abi::kernel_port::OpticFocus::Worldline { .. }
                | echo_wasm_abi::kernel_port::OpticFocus::Strand { .. }
                | echo_wasm_abi::kernel_port::OpticFocus::Braid { .. }
                | echo_wasm_abi::kernel_port::OpticFocus::RetainedReading { .. }
                | echo_wasm_abi::kernel_port::OpticFocus::AttachmentBoundary { .. }
        ));
    }
}

#[test]
fn strand_coordinate_names_explicit_parent_basis_in_abi() {
    let parent_basis = ProvenanceRef {
        worldline_id: worldline(9),
        worldline_tick: crate::clock::WorldlineTick::from_raw(11),
        commit_hash: [12; 32],
    };
    let coordinate = EchoCoordinate::Strand {
        strand_id: strand(2),
        at: CoordinateAt::Provenance(parent_basis),
        parent_basis: Some(parent_basis),
    };

    assert_eq!(
        coordinate.to_abi(),
        echo_wasm_abi::kernel_port::EchoCoordinate::Strand {
            strand_id: echo_wasm_abi::kernel_port::StrandId::from_bytes([2; 32]),
            at: echo_wasm_abi::kernel_port::CoordinateAt::Provenance {
                reference: echo_wasm_abi::kernel_port::ProvenanceRef {
                    worldline_id: echo_wasm_abi::kernel_port::WorldlineId::from_bytes([9; 32]),
                    worldline_tick: echo_wasm_abi::kernel_port::WorldlineTick(11),
                    commit_hash: vec![12; 32],
                },
            },
            parent_basis: Some(echo_wasm_abi::kernel_port::ProvenanceRef {
                worldline_id: echo_wasm_abi::kernel_port::WorldlineId::from_bytes([9; 32]),
                worldline_tick: echo_wasm_abi::kernel_port::WorldlineTick(11),
                commit_hash: vec![12; 32],
            }),
        }
    );
}

#[test]
fn optic_aperture_encodes_bounds_without_full_materialization_fallback() {
    let aperture = OpticAperture {
        shape: OpticApertureShape::QueryBytes {
            query_id: 42,
            vars_digest: [7; 32],
        },
        budget: OpticReadBudget {
            max_bytes: Some(4096),
            max_nodes: Some(128),
            max_ticks: Some(12),
            max_attachments: Some(0),
        },
        attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
    };

    assert_eq!(
        aperture.to_abi(),
        echo_wasm_abi::kernel_port::OpticAperture {
            shape: echo_wasm_abi::kernel_port::OpticApertureShape::QueryBytes {
                query_id: 42,
                vars_digest: vec![7; 32],
            },
            budget: echo_wasm_abi::kernel_port::OpticReadBudget {
                max_bytes: Some(4096),
                max_nodes: Some(128),
                max_ticks: Some(12),
                max_attachments: Some(0),
            },
            attachment_descent: echo_wasm_abi::kernel_port::AttachmentDescentPolicy::BoundaryOnly,
        }
    );
}

#[test]
fn truth_channel_aperture_converts_channel_ids_to_abi_bytes() {
    let channel = TypeId([3; 32]);
    let aperture = OpticAperture {
        shape: OpticApertureShape::TruthChannels {
            channels: Some(vec![channel]),
        },
        budget: OpticReadBudget::default(),
        attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
    };

    assert_eq!(
        aperture.to_abi().shape,
        echo_wasm_abi::kernel_port::OpticApertureShape::TruthChannels {
            channels: Some(vec![echo_wasm_abi::kernel_port::ChannelId::from_bytes(
                [3; 32]
            )]),
        }
    );
}

#[test]
fn read_identity_is_stable_for_same_read_question() {
    let focus = worldline_focus();
    let coordinate = frontier_coordinate();
    let aperture = head_aperture();
    let optic = EchoOptic::new(
        focus.clone(),
        coordinate.clone(),
        ProjectionVersion::from_raw(1),
        None,
        intent_family(1),
        capability(2),
    );

    let first = ReadIdentity::new(
        optic.optic_id,
        &focus,
        coordinate.clone(),
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 2),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let second = ReadIdentity::new(
        optic.optic_id,
        &focus,
        coordinate,
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 2),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );

    assert_eq!(first, second);
    assert_eq!(first.read_identity_hash, second.read_identity_hash);
    assert_eq!(first.focus_digest, focus.digest());
    assert_eq!(first.aperture_digest, aperture.digest());
}

#[test]
fn read_identity_changes_when_question_or_witness_changes() {
    let focus = worldline_focus();
    let coordinate = frontier_coordinate();
    let aperture = head_aperture();
    let optic_id = EchoOptic::new(
        focus.clone(),
        coordinate.clone(),
        ProjectionVersion::from_raw(1),
        None,
        intent_family(1),
        capability(2),
    )
    .optic_id;

    let base = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate.clone(),
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 2),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let changed_coordinate = ReadIdentity::new(
        optic_id,
        &focus,
        EchoCoordinate::Worldline {
            worldline_id: worldline(1),
            at: CoordinateAt::Tick(crate::clock::WorldlineTick::from_raw(3)),
        },
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 2),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let changed_aperture = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate.clone(),
        &OpticAperture {
            shape: OpticApertureShape::SnapshotMetadata,
            budget: aperture.budget,
            attachment_descent: aperture.attachment_descent,
        },
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 2),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let changed_projection = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate.clone(),
        &aperture,
        ProjectionVersion::from_raw(2),
        None,
        witness_basis(1, 2),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let changed_witness = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate,
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 3),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );

    assert_ne!(
        base.read_identity_hash,
        changed_coordinate.read_identity_hash
    );
    assert_ne!(base.read_identity_hash, changed_aperture.read_identity_hash);
    assert_ne!(
        base.read_identity_hash,
        changed_projection.read_identity_hash
    );
    assert_ne!(base.read_identity_hash, changed_witness.read_identity_hash);
}

#[test]
fn existing_reading_envelope_can_carry_compatible_optic_identity() {
    let focus = worldline_focus();
    let coordinate = frontier_coordinate();
    let aperture = head_aperture();
    let reading = reading_envelope();
    let optic_id = EchoOptic::new(
        focus.clone(),
        coordinate.clone(),
        ProjectionVersion::from_raw(1),
        None,
        intent_family(1),
        capability(2),
    )
    .optic_id;

    let identity = ReadIdentity::from_reading_envelope(
        optic_id,
        &focus,
        coordinate,
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 2),
        &reading,
    );
    let envelope = OpticReadingEnvelope::new(reading, identity);
    let abi = envelope.to_abi();

    assert_eq!(abi.read_identity.optic_id, optic_id_to_abi(optic_id));
    assert_eq!(
        abi.read_identity.rights_posture,
        echo_wasm_abi::kernel_port::ReadingRightsPosture::KernelPublic
    );
    assert_eq!(
        abi.read_identity.budget_posture,
        echo_wasm_abi::kernel_port::ReadingBudgetPosture::UnboundedOneShot
    );
    assert_eq!(
        abi.read_identity.residual_posture,
        echo_wasm_abi::kernel_port::ReadingResidualPosture::Complete
    );
}

#[test]
fn retained_reading_key_requires_content_hash_and_read_identity() {
    let focus = worldline_focus();
    let coordinate = frontier_coordinate();
    let aperture = head_aperture();
    let optic_id = EchoOptic::new(
        focus.clone(),
        coordinate.clone(),
        ProjectionVersion::from_raw(1),
        None,
        intent_family(1),
        capability(2),
    )
    .optic_id;
    let content_hash = [42; 32];
    let codec_id = retained_codec(7);
    let first_identity = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate.clone(),
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 2),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let second_identity = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate,
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(1, 3),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );

    let first = RetainedReadingDescriptor::new(first_identity, content_hash, codec_id, 128);
    let second = RetainedReadingDescriptor::new(second_identity, content_hash, codec_id, 128);
    let content_only_matches = [first.clone(), second.clone()]
        .iter()
        .filter(|descriptor| descriptor.content_hash == content_hash)
        .count();

    assert_ne!(first.key, second.key);
    assert_eq!(content_only_matches, 2);
    assert_eq!(first.to_abi().key, retained_reading_key_to_abi(first.key));
}

#[test]
fn checkpoint_plus_tail_identity_does_not_collapse_to_checkpoint_hash() {
    let focus = worldline_focus();
    let coordinate = frontier_coordinate();
    let aperture = head_aperture();
    let optic_id = EchoOptic::new(
        focus.clone(),
        coordinate.clone(),
        ProjectionVersion::from_raw(1),
        None,
        intent_family(1),
        capability(2),
    )
    .optic_id;
    let checkpoint_ref = provenance(4, 10);
    let checkpoint_hash = [44; 32];
    let checkpoint_only = WitnessBasis::ResolvedCommit {
        reference: checkpoint_ref,
        state_root: checkpoint_hash,
        commit_hash: checkpoint_hash,
    };
    let checkpoint_plus_tail = WitnessBasis::CheckpointPlusTail {
        checkpoint_ref,
        checkpoint_hash,
        tail_witness_refs: vec![provenance(4, 11)],
        tail_digest: [45; 32],
    };

    let checkpoint_only_identity = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate.clone(),
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        checkpoint_only,
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let live_tail_identity = ReadIdentity::new(
        optic_id,
        &focus,
        coordinate,
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        checkpoint_plus_tail,
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::UnboundedOneShot,
        ReadingResidualPosture::Complete,
    );
    let checkpoint_only_retained =
        RetainedReadingDescriptor::new(checkpoint_only_identity, [55; 32], retained_codec(7), 256);
    let live_tail_retained =
        RetainedReadingDescriptor::new(live_tail_identity, [55; 32], retained_codec(7), 256);

    assert_ne!(
        checkpoint_only_retained.read_identity.read_identity_hash,
        live_tail_retained.read_identity.read_identity_hash
    );
    assert_ne!(checkpoint_only_retained.key, live_tail_retained.key);
}

#[test]
fn optic_obstruction_kinds_keep_fail_closed_cases_distinct() {
    use std::collections::BTreeSet;

    let required = [
        (
            OpticObstructionKind::StaleBasis,
            echo_wasm_abi::kernel_port::OpticObstructionKind::StaleBasis,
        ),
        (
            OpticObstructionKind::MissingWitness,
            echo_wasm_abi::kernel_port::OpticObstructionKind::MissingWitness,
        ),
        (
            OpticObstructionKind::BudgetExceeded,
            echo_wasm_abi::kernel_port::OpticObstructionKind::BudgetExceeded,
        ),
        (
            OpticObstructionKind::CapabilityDenied,
            echo_wasm_abi::kernel_port::OpticObstructionKind::CapabilityDenied,
        ),
        (
            OpticObstructionKind::AttachmentDescentRequired,
            echo_wasm_abi::kernel_port::OpticObstructionKind::AttachmentDescentRequired,
        ),
    ];

    let mut names = BTreeSet::new();
    for (core, expected) in required {
        let abi = core.to_abi();
        assert_eq!(abi, expected);
        assert!(names.insert(format!("{abi:?}")));
    }

    assert_eq!(names.len(), required.len());
}

#[test]
fn intent_dispatch_result_matching_is_variant_exhaustive() {
    fn classify(result: &IntentDispatchResult) -> &'static str {
        match result {
            IntentDispatchResult::Admitted(_) => "admitted",
            IntentDispatchResult::Staged(_) => "staged",
            IntentDispatchResult::Plural(_) => "plural",
            IntentDispatchResult::Conflict(_) => "conflict",
            IntentDispatchResult::Obstructed(_) => "obstructed",
        }
    }

    let optic = EchoOptic::new(
        worldline_focus(),
        frontier_coordinate(),
        ProjectionVersion::from_raw(1),
        None,
        intent_family(1),
        capability(2),
    );
    let base_coordinate = frontier_coordinate();
    let family = intent_family(1);
    let admitted_ref = provenance(1, 3);
    let obstruction = OpticObstruction {
        kind: OpticObstructionKind::StaleBasis,
        optic_id: Some(optic.optic_id),
        focus: Some(worldline_focus()),
        coordinate: Some(base_coordinate.clone()),
        witness_basis: Some(WitnessBasis::Missing {
            reason: MissingWitnessBasisReason::EvidenceUnavailable,
        }),
        message: "base coordinate is stale".to_owned(),
    };
    let outcomes = vec![
        IntentDispatchResult::Admitted(AdmittedIntent {
            optic_id: optic.optic_id,
            base_coordinate: base_coordinate.clone(),
            intent_family: family,
            admitted_ref,
            receipt_hash: [4; 32],
        }),
        IntentDispatchResult::Staged(StagedIntent {
            optic_id: optic.optic_id,
            base_coordinate: base_coordinate.clone(),
            intent_family: family,
            stage_ref: [5; 32],
            reason: StagedIntentReason::RebaseRequired,
        }),
        IntentDispatchResult::Plural(PluralIntent {
            optic_id: optic.optic_id,
            base_coordinate: base_coordinate.clone(),
            intent_family: family,
            candidate_refs: vec![admitted_ref, provenance(1, 4)],
            residual_posture: ReadingResidualPosture::PluralityPreserved,
        }),
        IntentDispatchResult::Conflict(IntentConflict {
            optic_id: optic.optic_id,
            base_coordinate,
            intent_family: family,
            reason: IntentConflictReason::StaleBasis,
            conflict_ref: Some(admitted_ref),
            evidence_digest: [6; 32],
            message: "base conflicts with frontier".to_owned(),
        }),
        IntentDispatchResult::Obstructed(obstruction),
    ];

    assert_eq!(
        outcomes.iter().map(classify).collect::<Vec<_>>(),
        vec!["admitted", "staged", "plural", "conflict", "obstructed"]
    );
    assert!(matches!(
        outcomes[0].to_abi(),
        echo_wasm_abi::kernel_port::IntentDispatchResult::Admitted(_)
    ));
    assert!(matches!(
        outcomes[1].to_abi(),
        echo_wasm_abi::kernel_port::IntentDispatchResult::Staged(_)
    ));
    assert!(matches!(
        outcomes[2].to_abi(),
        echo_wasm_abi::kernel_port::IntentDispatchResult::Plural(_)
    ));
    assert!(matches!(
        outcomes[3].to_abi(),
        echo_wasm_abi::kernel_port::IntentDispatchResult::Conflict(_)
    ));
    assert!(matches!(
        outcomes[4].to_abi(),
        echo_wasm_abi::kernel_port::IntentDispatchResult::Obstructed(_)
    ));
}

#[test]
fn dispatch_optic_intent_request_carries_eint_v1_with_explicit_base() -> Result<(), String> {
    let focus = worldline_focus();
    let base_coordinate = frontier_coordinate();
    let payload_bytes = echo_wasm_abi::pack_intent_v1(77, b"optic-vars")
        .map_err(|error| format!("failed to pack EINT fixture: {error:?}"))?;
    let request = DispatchOpticIntentRequest {
        optic_id: OpticId::from_bytes([1; 32]),
        base_coordinate: base_coordinate.clone(),
        intent_family: intent_family(2),
        focus: focus.clone(),
        cause: cause(2),
        capability: optic_capability(2, focus),
        admission_law: admission_law(4),
        payload: OpticIntentPayload::EintV1 {
            bytes: payload_bytes.clone(),
        },
    };

    request
        .validate_proposal()
        .map_err(|obstruction| format!("expected valid optic dispatch, got {obstruction:?}"))?;

    let abi = request.to_abi();
    assert_eq!(
        abi.optic_id,
        echo_wasm_abi::kernel_port::OpticId::from_bytes([1; 32])
    );
    assert_eq!(abi.base_coordinate, base_coordinate.to_abi());
    assert_eq!(
        abi.admission_law,
        echo_wasm_abi::kernel_port::AdmissionLawId::from_bytes([4; 32])
    );
    assert!(matches!(
        abi.payload,
        echo_wasm_abi::kernel_port::OpticIntentPayload::EintV1 { ref bytes }
            if bytes == &payload_bytes
    ));
    Ok(())
}

#[test]
fn dispatch_optic_intent_request_rejects_capability_bypass() -> Result<(), String> {
    let request = DispatchOpticIntentRequest {
        optic_id: OpticId::from_bytes([1; 32]),
        base_coordinate: frontier_coordinate(),
        intent_family: intent_family(99),
        focus: worldline_focus(),
        cause: cause(2),
        capability: optic_capability(2, worldline_focus()),
        admission_law: admission_law(4),
        payload: OpticIntentPayload::EintV1 {
            bytes: echo_wasm_abi::pack_intent_v1(77, b"optic-vars")
                .map_err(|error| format!("failed to pack EINT fixture: {error:?}"))?,
        },
    };

    let obstruction = request
        .validate_proposal()
        .err()
        .ok_or_else(|| "capability mismatch should obstruct dispatch".to_owned())?;

    assert_eq!(
        obstruction.kind,
        OpticObstructionKind::UnsupportedIntentFamily
    );
    assert_eq!(obstruction.optic_id, Some(OpticId::from_bytes([1; 32])));
    assert_eq!(obstruction.coordinate, Some(frontier_coordinate()));
    Ok(())
}

#[test]
fn open_optic_request_validates_descriptor_without_mutable_handle() -> Result<(), String> {
    let focus = worldline_focus();
    let coordinate = frontier_coordinate();
    let grant = optic_capability(11, focus.clone());
    let request = OpenOpticRequest {
        focus: focus.clone(),
        coordinate: coordinate.clone(),
        projection_version: ProjectionVersion::from_raw(1),
        reducer_version: None,
        intent_family: intent_family(11),
        capability: grant,
        cause: cause(11),
    };

    let result = request
        .validate_descriptor()
        .map_err(|error| format!("expected valid optic descriptor, got {error:?}"))?;

    assert_eq!(
        result.optic,
        EchoOptic::new(
            focus,
            coordinate,
            ProjectionVersion::from_raw(1),
            None,
            intent_family(11),
            capability(11),
        )
    );
    assert_eq!(
        result.capability_posture,
        CapabilityPosture::Granted {
            capability_id: capability(11),
            actor: actor(11),
            issuer_ref: Some(provenance(11, 1)),
            policy_hash: [13; 32],
        }
    );
    assert_eq!(result.to_abi().optic, result.optic.to_abi());

    Ok(())
}

#[test]
fn open_optic_denied_capability_returns_typed_obstruction() -> Result<(), String> {
    let request = OpenOpticRequest {
        focus: worldline_focus(),
        coordinate: frontier_coordinate(),
        projection_version: ProjectionVersion::from_raw(1),
        reducer_version: None,
        intent_family: intent_family(12),
        capability: optic_capability(
            12,
            OpticFocus::Strand {
                strand_id: strand(2),
            },
        ),
        cause: cause(12),
    };

    let error = match request.validate_descriptor() {
        Ok(result) => return Err(format!("expected capability denial, got {result:?}")),
        Err(error) => error,
    };
    let OpticOpenError::Obstructed(obstruction) = error;

    assert_eq!(obstruction.kind, OpticObstructionKind::CapabilityDenied);
    assert_eq!(obstruction.optic_id, None);
    assert_eq!(obstruction.focus, Some(worldline_focus()));
    assert_eq!(obstruction.coordinate, Some(frontier_coordinate()));

    Ok(())
}

#[test]
fn close_optic_releases_only_session_descriptor_resource() {
    let optic_id = OpticId::from_bytes([9; 32]);
    let request = CloseOpticRequest {
        optic_id,
        cause: cause(9),
    };

    let result = request.close_session_descriptor();

    assert_eq!(result, CloseOpticResult { optic_id });
    assert_eq!(
        result.to_abi(),
        echo_wasm_abi::kernel_port::CloseOpticResult {
            optic_id: optic_id_to_abi(optic_id),
        }
    );
}
