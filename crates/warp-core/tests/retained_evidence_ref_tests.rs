// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Retained evidence reference regression tests.

use echo_cas::{blob_hash, RetainedBlobRole, SemanticBlobCoordinate};
use warp_core::causal_wal::{
    EvidenceMaterialPosture, ReadingRefRecord, RecoveredRetentionIndex,
    RecoveredRetentionIndexError, RetainedMaterialKind, RetainedMaterialRecord,
};
use warp_core::{
    make_head_id, ContractEvidenceIdentity, ContractObstruction, ContractObstructionKind,
    ContractObstructionSubject, ContractOperationKind, GlobalTick, InstalledContractPackageId,
    IntentOutcome, IntentOutcomeDecision, IntentOutcomeObservation, ReceiptCorrelationRecord,
    RetainedEvidenceAccess, RetainedEvidenceBoundaryPosture, RetainedEvidenceCompleteness,
    RetainedEvidenceCoordinate, RetainedEvidenceLayer, RetainedEvidenceOrigin,
    RetainedEvidencePosture, RetainedEvidenceProofStrength, RetainedEvidenceRef,
    RetainedEvidenceRole, TickReceiptRejection, WorldlineId, WorldlineTick, WriterHeadKey,
};

fn hash(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn content_hash(bytes: &[u8]) -> [u8; 32] {
    blake3::hash(bytes).into()
}

fn contract(seed: u8, op_id: u32, op_kind: ContractOperationKind) -> ContractEvidenceIdentity {
    ContractEvidenceIdentity {
        package_id: InstalledContractPackageId::from_bytes(hash(seed)),
        echo_abi_version: 1,
        package_name: format!("pkg-{seed}"),
        package_version: "0.1.0".to_owned(),
        artifact_hash_hex: format!("{seed:02x}").repeat(32),
        codec_id: format!("codec-{seed}"),
        registry_version: 1,
        wesley_generator_version: "echo-wesley-gen/0.1.0".to_owned(),
        helper_api_version: 1,
        schema_sha256_hex: format!("{:02x}", seed.wrapping_add(1)).repeat(32),
        op_id,
        op_kind,
    }
}

fn coordinate(role: RetainedEvidenceRole, semantic_seed: u8) -> RetainedEvidenceCoordinate {
    RetainedEvidenceCoordinate::new(
        contract(7, 42, ContractOperationKind::Query),
        role,
        hash(semantic_seed),
    )
}

fn semantic_blob_coordinate(role: RetainedBlobRole, semantic_seed: u8) -> SemanticBlobCoordinate {
    SemanticBlobCoordinate {
        namespace: "echo:test-retained-evidence-crosswalk".to_owned(),
        schema_hash_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_owned(),
        artifact_hash_hex: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_owned(),
        role,
        semantic_digest: hash(semantic_seed),
    }
}

fn writer_head(seed: u8) -> WriterHeadKey {
    WriterHeadKey {
        worldline_id: WorldlineId::from_bytes(hash(seed)),
        head_id: make_head_id(&format!("retained-evidence-head-{seed}")),
    }
}

fn receipt_correlation(contract: ContractEvidenceIdentity) -> ReceiptCorrelationRecord {
    ReceiptCorrelationRecord {
        ticketed_ingress_id: hash(21),
        submission_id: hash(22),
        ticket_digest: hash(23),
        ingress_id: hash(24),
        head_key: writer_head(25),
        contract: Some(contract),
        commit_global_tick: GlobalTick::from_raw(26),
        worldline_tick_after: WorldlineTick::from_raw(27),
        tick_receipt_digest: hash(28),
        commit_hash: hash(29),
    }
}

fn assert_missing_retention(posture: &RetainedEvidencePosture) {
    assert_eq!(
        posture.obstruction().map(|obstruction| obstruction.kind),
        Some(ContractObstructionKind::MissingRetention)
    );
    assert!(matches!(
        posture
            .obstruction()
            .map(|obstruction| &obstruction.subject),
        Some(ContractObstructionSubject::Retention { .. })
    ));
}

#[test]
fn retained_evidence_ref_id_binds_semantic_coordinate() {
    let content_hash = content_hash(b"same bytes");
    let first = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ReadingPayload, 1),
        content_hash,
        10,
    );
    let second = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ReadingPayload, 2),
        content_hash,
        10,
    );

    assert_ne!(
        first.coordinate.coordinate_id(),
        second.coordinate.coordinate_id()
    );
    assert_ne!(first.evidence_ref_id(), second.evidence_ref_id());
}

#[test]
fn retained_evidence_ref_id_binds_content_hash_and_byte_len() {
    let coord = coordinate(RetainedEvidenceRole::ContractReceipt, 3);
    let first = RetainedEvidenceRef::new(coord.clone(), content_hash(b"receipt"), 7);
    let different_bytes = RetainedEvidenceRef::new(coord.clone(), content_hash(b"receipt!"), 8);
    let different_len = RetainedEvidenceRef::new(coord, content_hash(b"receipt"), 99);

    assert_ne!(first.evidence_ref_id(), different_bytes.evidence_ref_id());
    assert_ne!(first.evidence_ref_id(), different_len.evidence_ref_id());
}

#[test]
fn retained_evidence_role_separates_payload_from_envelope() {
    let semantic_digest = hash(4);
    let reading_payload = RetainedEvidenceCoordinate::new(
        contract(8, 7, ContractOperationKind::Query),
        RetainedEvidenceRole::ReadingPayload,
        semantic_digest,
    );
    let reading_envelope = RetainedEvidenceCoordinate::new(
        reading_payload.contract.clone(),
        RetainedEvidenceRole::ReadingEnvelope,
        semantic_digest,
    );

    assert_ne!(
        reading_payload.coordinate_id(),
        reading_envelope.coordinate_id()
    );
}

#[test]
fn missing_coordinate_returns_missing_retention_obstruction_with_contract() {
    let coord = coordinate(RetainedEvidenceRole::Witness, 5);
    let posture = RetainedEvidencePosture::missing_coordinate(&coord);

    assert_eq!(
        posture.obstruction().map(|obstruction| obstruction.kind),
        Some(ContractObstructionKind::MissingRetention)
    );
    assert_eq!(
        posture
            .obstruction()
            .and_then(|obstruction| obstruction.contract.as_ref()),
        Some(&coord.contract)
    );
    assert_eq!(
        posture
            .obstruction()
            .map(|obstruction| &obstruction.subject),
        Some(&ContractObstructionSubject::Retention {
            retention_id: coord.coordinate_id(),
        })
    );
}

#[test]
fn missing_content_returns_missing_retention_obstruction_with_ref_id() {
    let reference = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ReadingPayload, 6),
        content_hash(b"retained reading"),
        16,
    );
    let posture = RetainedEvidencePosture::missing_content(&reference);

    assert_eq!(
        posture.obstruction().map(|obstruction| obstruction.kind),
        Some(ContractObstructionKind::MissingRetention)
    );
    assert_eq!(
        posture
            .obstruction()
            .map(|obstruction| &obstruction.subject),
        Some(&ContractObstructionSubject::Retention {
            retention_id: reference.evidence_ref_id(),
        })
    );
}

#[test]
fn retained_reading_missing_payload_is_not_empty_success() {
    let query_contract = contract(14, 15, ContractOperationKind::Query);
    let reading_id = hash(16);
    let payload_bytes = b"retained query payload bytes";
    let envelope_coordinate = RetainedEvidenceCoordinate::new(
        query_contract.clone(),
        RetainedEvidenceRole::ReadingEnvelope,
        reading_id,
    );
    let payload_ref = RetainedEvidenceRef::new(
        RetainedEvidenceCoordinate::new(
            query_contract,
            RetainedEvidenceRole::ReadingPayload,
            reading_id,
        ),
        content_hash(payload_bytes),
        payload_bytes.len() as u64,
    );
    let reading_postures = [
        RetainedEvidencePosture::missing_coordinate(&envelope_coordinate),
        RetainedEvidencePosture::missing_content(&payload_ref),
    ];

    assert_eq!(reading_postures.len(), 2);
    assert!(matches!(
        &reading_postures[0],
        RetainedEvidencePosture::MissingCoordinate { .. }
    ));
    if let RetainedEvidencePosture::MissingCoordinate {
        coordinate,
        obstruction,
    } = &reading_postures[0]
    {
        assert_eq!(coordinate.role, RetainedEvidenceRole::ReadingEnvelope);
        assert_eq!(coordinate.semantic_digest, reading_id);
        assert_eq!(obstruction.kind, ContractObstructionKind::MissingRetention);
        assert_eq!(
            obstruction.subject,
            ContractObstructionSubject::Retention {
                retention_id: coordinate.coordinate_id()
            }
        );
    }
    assert!(matches!(
        &reading_postures[1],
        RetainedEvidencePosture::MissingContent { .. }
    ));
    if let RetainedEvidencePosture::MissingContent {
        reference,
        obstruction,
    } = &reading_postures[1]
    {
        assert_eq!(
            reference.coordinate.role,
            RetainedEvidenceRole::ReadingPayload
        );
        assert_eq!(reference.coordinate.semantic_digest, reading_id);
        assert_eq!(reference.content_hash, content_hash(payload_bytes));
        assert_eq!(reference.byte_len, payload_bytes.len() as u64);
        assert_eq!(obstruction.kind, ContractObstructionKind::MissingRetention);
        assert_eq!(
            obstruction.subject,
            ContractObstructionSubject::Retention {
                retention_id: reference.evidence_ref_id()
            }
        );
    }
    for posture in &reading_postures {
        assert_missing_retention(posture);
    }

    let receipt_contract = contract(17, 18, ContractOperationKind::Mutation);
    let correlation = receipt_correlation(receipt_contract.clone());
    let applied = IntentOutcome::from_observation(IntentOutcomeObservation::Decided {
        correlation: Box::new(correlation.clone()),
        decision: IntentOutcomeDecision::Applied {
            receipt_entry_index: 3,
            rule_id: hash(30),
        },
    });
    let rejected = IntentOutcome::from_observation(IntentOutcomeObservation::Decided {
        correlation: Box::new(correlation.clone()),
        decision: IntentOutcomeDecision::Rejected {
            receipt_entry_index: 4,
            rule_id: hash(31),
            reason: TickReceiptRejection::FootprintConflict,
            blocked_by: vec![3],
        },
    });

    for outcome in [&applied, &rejected] {
        assert!(matches!(
            outcome,
            IntentOutcome::Applied { .. } | IntentOutcome::Rejected { .. }
        ));
        if let IntentOutcome::Applied { receipt, .. } | IntentOutcome::Rejected { receipt, .. } =
            outcome
        {
            assert_eq!(receipt.contract, Some(receipt_contract.clone()));
            assert!(matches!(
                receipt.retained_evidence.as_slice(),
                [RetainedEvidencePosture::MissingCoordinate { .. }]
            ));
            if let [RetainedEvidencePosture::MissingCoordinate {
                coordinate,
                obstruction,
            }] = receipt.retained_evidence.as_slice()
            {
                assert_eq!(coordinate.contract, receipt_contract);
                assert_eq!(coordinate.role, RetainedEvidenceRole::ContractReceipt);
                assert_eq!(coordinate.semantic_digest, correlation.tick_receipt_digest);
                assert_eq!(obstruction.kind, ContractObstructionKind::MissingRetention);
            }
        }
    }
    assert!(matches!(
        rejected,
        IntentOutcome::Rejected {
            reason: TickReceiptRejection::FootprintConflict,
            ref blocked_by,
            ..
        } if blocked_by == &[3]
    ));

    let no_matching_receipt = IntentOutcome::from_observation(IntentOutcomeObservation::Decided {
        correlation: Box::new(correlation),
        decision: IntentOutcomeDecision::NoMatchingReceiptEntry {
            tick_receipt_digest: hash(28),
        },
    });
    assert!(matches!(
        no_matching_receipt,
        IntentOutcome::Obstructed {
            obstruction,
            ..
        } if obstruction.kind == ContractObstructionKind::MissingRetention
    ));
}

#[test]
fn available_retained_evidence_is_not_an_obstruction() {
    let reference = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ObserverArtifact, 9),
        content_hash(b"observer artifact"),
        17,
    );
    let posture = RetainedEvidencePosture::available(reference.clone());

    assert_eq!(posture.obstruction(), None);
    assert_eq!(posture, RetainedEvidencePosture::Available(reference));
}

#[test]
fn native_and_fixture_retained_evidence_do_not_alias() {
    let reference = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::Witness, 40),
        content_hash(b"same witness bytes"),
        18,
    );
    let native = RetainedEvidenceBoundaryPosture::available_citation(
        reference.clone(),
        RetainedEvidenceLayer::WitnessCore,
        RetainedEvidenceOrigin::Native,
        RetainedEvidenceProofStrength::Signature,
    );
    let fixture = RetainedEvidenceBoundaryPosture::available_citation(
        reference,
        RetainedEvidenceLayer::WitnessCore,
        RetainedEvidenceOrigin::Fixture,
        RetainedEvidenceProofStrength::Signature,
    );

    assert_ne!(native, fixture);
    assert_ne!(native.boundary_posture_id(), fixture.boundary_posture_id());
    assert_eq!(native.origin, RetainedEvidenceOrigin::Native);
    assert_eq!(fixture.origin, RetainedEvidenceOrigin::Fixture);
}

#[test]
fn redacted_retained_evidence_is_not_missing_content() {
    let reference = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ContractReceipt, 41),
        content_hash(b"receipt shell bytes"),
        19,
    );
    let redacted = RetainedEvidenceBoundaryPosture::redacted(
        reference,
        RetainedEvidenceLayer::ReceiptShell,
        RetainedEvidenceOrigin::Native,
        RetainedEvidenceProofStrength::ReplayCertificate,
    );

    assert_eq!(redacted.access, RetainedEvidenceAccess::Redacted);
    assert_eq!(
        redacted.completeness,
        RetainedEvidenceCompleteness::DeclaredLost
    );
    assert_ne!(
        redacted
            .obstruction
            .as_ref()
            .map(|obstruction| obstruction.kind),
        Some(ContractObstructionKind::MissingRetention)
    );
}

#[test]
fn unsupported_evidence_kind_obstructs_not_missing_retention() {
    let coord = coordinate(RetainedEvidenceRole::Witness, 42);
    let unsupported = RetainedEvidenceBoundaryPosture::unsupported_evidence_kind(
        coord.clone(),
        RetainedEvidenceLayer::WitnessCore,
        RetainedEvidenceOrigin::Translated,
        RetainedEvidenceProofStrength::ZkProof,
    );

    assert_eq!(unsupported.access, RetainedEvidenceAccess::Unsupported);
    assert_eq!(
        unsupported.completeness,
        RetainedEvidenceCompleteness::Obstructed
    );
    assert_eq!(
        unsupported
            .obstruction
            .as_ref()
            .map(|obstruction| &obstruction.subject),
        Some(&ContractObstructionSubject::Retention {
            retention_id: coord.coordinate_id()
        })
    );
    assert_ne!(
        unsupported
            .obstruction
            .as_ref()
            .map(|obstruction| obstruction.kind),
        Some(ContractObstructionKind::MissingRetention)
    );
}

#[test]
fn boundary_posture_id_binds_obstruction_contract_evidence() {
    let coord = coordinate(RetainedEvidenceRole::Witness, 46);
    let base_obstruction =
        ContractObstruction::admission_obstruction(ContractObstructionSubject::Retention {
            retention_id: coord.coordinate_id(),
        });
    let first = RetainedEvidenceBoundaryPosture {
        coordinate: coord.clone(),
        reference: None,
        layer: RetainedEvidenceLayer::WitnessCore,
        origin: RetainedEvidenceOrigin::Translated,
        proof_strength: RetainedEvidenceProofStrength::Composite,
        access: RetainedEvidenceAccess::AuthorityBlocked,
        completeness: RetainedEvidenceCompleteness::Obstructed,
        obstruction: Some(base_obstruction.clone().with_contract(contract(
            46,
            47,
            ContractOperationKind::Mutation,
        ))),
    };
    let second = RetainedEvidenceBoundaryPosture {
        obstruction: Some(base_obstruction.with_contract(contract(
            48,
            49,
            ContractOperationKind::Mutation,
        ))),
        ..first.clone()
    };

    assert_ne!(first.obstruction, second.obstruction);
    assert_ne!(first.boundary_posture_id(), second.boundary_posture_id());
}

#[test]
fn retained_content_hash_does_not_identify_semantic_evidence() {
    let content_hash = content_hash(b"shared cold proof bytes");
    let first = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ReadingPayload, 43),
        content_hash,
        23,
    );
    let second = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ReadingPayload, 44),
        content_hash,
        23,
    );

    let first_posture = RetainedEvidenceBoundaryPosture::available_citation(
        first,
        RetainedEvidenceLayer::ReintegrationCore,
        RetainedEvidenceOrigin::Native,
        RetainedEvidenceProofStrength::MerkleOpening,
    );
    let second_posture = RetainedEvidenceBoundaryPosture::available_citation(
        second,
        RetainedEvidenceLayer::ReintegrationCore,
        RetainedEvidenceOrigin::Native,
        RetainedEvidenceProofStrength::MerkleOpening,
    );

    assert_eq!(
        first_posture
            .reference
            .as_ref()
            .map(|reference| reference.content_hash),
        second_posture
            .reference
            .as_ref()
            .map(|reference| reference.content_hash)
    );
    assert_ne!(
        first_posture.coordinate.coordinate_id(),
        second_posture.coordinate.coordinate_id()
    );
    assert_ne!(
        first_posture.boundary_posture_id(),
        second_posture.boundary_posture_id()
    );
}

#[test]
fn available_retained_evidence_ref_does_not_grant_reveal() {
    let reference = RetainedEvidenceRef::new(
        coordinate(RetainedEvidenceRole::ReadingEnvelope, 45),
        content_hash(b"reading envelope"),
        16,
    );
    let citation = RetainedEvidenceBoundaryPosture::available_citation(
        reference.clone(),
        RetainedEvidenceLayer::ReintegrationCore,
        RetainedEvidenceOrigin::Native,
        RetainedEvidenceProofStrength::DigestOnly,
    );
    let revealable = RetainedEvidenceBoundaryPosture::available_revealable(
        reference,
        RetainedEvidenceLayer::ReintegrationCore,
        RetainedEvidenceOrigin::Native,
        RetainedEvidenceProofStrength::DigestOnly,
    );

    assert_eq!(citation.access, RetainedEvidenceAccess::CitationOnly);
    assert!(!citation.grants_reveal());
    assert!(revealable.grants_reveal());
}

#[test]
fn query_identity_does_not_imply_payload_retention() {
    let query_reading_id = hash(10);
    let reading_payload = RetainedEvidenceCoordinate::new(
        contract(10, 11, ContractOperationKind::Query),
        RetainedEvidenceRole::ReadingPayload,
        query_reading_id,
    );
    let missing = RetainedEvidencePosture::missing_coordinate(&reading_payload);

    assert_eq!(
        missing.obstruction().map(|obstruction| obstruction.kind),
        Some(ContractObstructionKind::MissingRetention)
    );
}

#[test]
fn wal_retention_crosswalk_keeps_coordinate_reading_and_payload_axes_distinct(
) -> Result<(), RecoveredRetentionIndexError> {
    let bytes = b"same reading bytes";
    let content_hash = blob_hash(bytes);
    let content_digest = *content_hash.as_bytes();
    let first_coordinate = semantic_blob_coordinate(RetainedBlobRole::ReadingPayload, 11);
    let second_coordinate = semantic_blob_coordinate(RetainedBlobRole::ReadingPayload, 12);
    let first_ref = RetainedEvidenceRef::new(
        RetainedEvidenceCoordinate::new(
            contract(11, 12, ContractOperationKind::Query),
            RetainedEvidenceRole::ReadingPayload,
            first_coordinate.semantic_digest,
        ),
        content_digest,
        bytes.len() as u64,
    );
    let second_ref = RetainedEvidenceRef::new(
        RetainedEvidenceCoordinate::new(
            first_ref.coordinate.contract.clone(),
            RetainedEvidenceRole::ReadingPayload,
            second_coordinate.semantic_digest,
        ),
        content_digest,
        bytes.len() as u64,
    );

    assert_eq!(first_ref.content_hash, content_digest);
    assert_eq!(second_ref.content_hash, content_digest);
    assert_ne!(
        first_ref.coordinate.coordinate_id(),
        second_ref.coordinate.coordinate_id()
    );
    assert_ne!(first_ref.evidence_ref_id(), second_ref.evidence_ref_id());

    let reading_id = hash(13);
    let material = RetainedMaterialRecord {
        material_digest: content_digest,
        semantic_coordinate_digest: first_coordinate.semantic_digest,
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Present,
    };
    let reading = ReadingRefRecord {
        reading_id,
        semantic_coordinate_digest: first_coordinate.semantic_digest,
        payload_digest: content_digest,
        envelope_digest: hash(14),
        posture: EvidenceMaterialPosture::Present,
    };
    let retention = RecoveredRetentionIndex::from_retention_records([material], [reading])?;

    assert_ne!(reading_id, content_digest);
    assert_eq!(
        retention.material_by_digest.get(&content_digest),
        Some(&material)
    );
    assert_eq!(retention.reading_by_id.get(&reading_id), Some(&reading));
    assert!(!retention.material_by_digest.contains_key(&reading_id));
    assert!(!retention.reading_by_id.contains_key(&content_digest));
    assert!(retention
        .material_by_semantic_coordinate
        .get(&first_coordinate.semantic_digest)
        .is_some_and(|digests| digests.contains(&content_digest)));
    assert!(retention
        .readings_by_semantic_coordinate
        .get(&first_coordinate.semantic_digest)
        .is_some_and(|readings| readings.contains(&reading_id)));
    Ok(())
}
