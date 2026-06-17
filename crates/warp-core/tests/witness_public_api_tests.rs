// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! External-consumer witness receipt public API checks.

use warp_core::{
    AuthorityDomainId, AuthorityDomainRef, BraidCoordinate, DisclosureBudget, OriginId,
    PresentationPurpose, SealedMembershipPresentation, WitnessAttestation, WitnessBackend,
    WitnessBackendSimulator, WitnessCompatibilityRule, WitnessError, WitnessKind, WitnessReceipt,
    WitnessRejectionCode, WitnessRequest, WitnessSimulatorFixture,
};

fn subject_digest() -> [u8; 32] {
    [0xA1; 32]
}

fn evidence_digest() -> [u8; 32] {
    [0xE1; 32]
}

fn authority_ref() -> AuthorityDomainRef {
    AuthorityDomainRef::new(
        OriginId::from_bytes([0x10; 32]),
        AuthorityDomainId::from_bytes([0x20; 32]),
    )
}

#[test]
fn public_witness_backend_reports_unsupported_kind_as_typed_error() {
    let backend = WitnessBackendSimulator::new(WitnessSimulatorFixture::UnsupportedWitnessFixture);
    let request = WitnessRequest::new(
        WitnessKind::ZkVerifierReceipt,
        subject_digest(),
        evidence_digest(),
        WitnessCompatibilityRule::StableV1,
    );

    assert_eq!(
        backend.verify(&request),
        Err(WitnessError::UnsupportedBackend {
            kind: WitnessKind::ZkVerifierReceipt,
        })
    );
}

#[test]
fn public_witness_simulator_returns_deterministic_fixture_receipts() -> Result<(), WitnessError> {
    let backend = WitnessBackendSimulator::new(WitnessSimulatorFixture::SignedWitnessFixture);
    let request = WitnessRequest::new(
        WitnessKind::SignedWitness,
        subject_digest(),
        evidence_digest(),
        WitnessCompatibilityRule::StableV1,
    );

    let first = backend.verify(&request)?;
    let second = backend.verify(&request)?;

    assert_eq!(first, second);
    assert_eq!(first.kind, WitnessKind::SignedWitness);
    assert_eq!(
        first.attestation,
        WitnessAttestation::IndependentAttestation
    );
    assert_eq!(first.compatibility, WitnessCompatibilityRule::StableV1);
    assert_eq!(first.digest(), second.digest());
    Ok(())
}

#[test]
fn public_rejected_witness_fixture_is_typed() {
    let backend = WitnessBackendSimulator::new(WitnessSimulatorFixture::RejectedWitnessFixture);
    let request = WitnessRequest::new(
        WitnessKind::ThresholdWitness,
        subject_digest(),
        evidence_digest(),
        WitnessCompatibilityRule::StableV1,
    );

    assert_eq!(
        backend.verify(&request),
        Err(WitnessError::BackendRejected {
            kind: WitnessKind::ThresholdWitness,
            reason: WitnessRejectionCode::Rejected,
        })
    );
}

#[test]
fn public_self_witness_fixture_rejects_stable_identity_requests() {
    let backend = WitnessBackendSimulator::new(WitnessSimulatorFixture::SelfWitness);
    let request = WitnessRequest::new(
        WitnessKind::SelfWitness,
        subject_digest(),
        evidence_digest(),
        WitnessCompatibilityRule::StableV1,
    );

    assert_eq!(
        backend.verify(&request),
        Err(WitnessError::UnsupportedCompatibility {
            kind: WitnessKind::SelfWitness,
            compatibility: WitnessCompatibilityRule::StableV1,
        })
    );
}

#[test]
fn public_witness_receipt_identity_binds_compatibility_rule() {
    let scaffold = WitnessReceipt::new(
        WitnessKind::SelfWitness,
        subject_digest(),
        evidence_digest(),
        WitnessCompatibilityRule::E1Scaffold,
        WitnessAttestation::IntegrityOnly,
    );
    let stable = WitnessReceipt::new(
        WitnessKind::SelfWitness,
        subject_digest(),
        evidence_digest(),
        WitnessCompatibilityRule::StableV1,
        WitnessAttestation::IntegrityOnly,
    );

    assert_ne!(scaffold.digest(), stable.digest());
}

#[test]
fn public_sealed_membership_presentation_uses_generic_purpose_and_budget() {
    let purpose = PresentationPurpose::new([0x44; 32]);
    let receipt = WitnessReceipt::self_witness(subject_digest(), evidence_digest());
    let presentation = SealedMembershipPresentation::new(
        BraidCoordinate([0xBC; 32]),
        purpose,
        authority_ref(),
        [0xA5; 32],
        receipt,
        DisclosureBudget::CapabilityScoped,
    );

    assert_eq!(presentation.purpose, purpose);
    assert_eq!(presentation.purpose.purpose_id(), [0x44; 32]);
    assert_eq!(
        presentation.disclosure_budget,
        DisclosureBudget::CapabilityScoped
    );
    assert_eq!(presentation.witness_receipt, receipt);
}
