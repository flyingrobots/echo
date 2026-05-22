// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Retained evidence reference regression tests.

use warp_core::{
    ContractEvidenceIdentity, ContractObstructionKind, ContractObstructionSubject,
    ContractOperationKind, InstalledContractPackageId, RetainedEvidenceCoordinate,
    RetainedEvidencePosture, RetainedEvidenceRef, RetainedEvidenceRole,
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
    let obstruction = posture
        .obstruction()
        .expect("missing coordinate should obstruct");

    assert_eq!(obstruction.kind, ContractObstructionKind::MissingRetention);
    assert_eq!(obstruction.contract.as_ref(), Some(&coord.contract));
    assert_eq!(
        obstruction.subject,
        ContractObstructionSubject::Retention {
            retention_id: coord.coordinate_id()
        }
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
    let obstruction = posture
        .obstruction()
        .expect("missing content should obstruct");

    assert_eq!(obstruction.kind, ContractObstructionKind::MissingRetention);
    assert_eq!(
        obstruction.subject,
        ContractObstructionSubject::Retention {
            retention_id: reference.evidence_ref_id()
        }
    );
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
