// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! External-consumer plurality law public API checks.

use warp_core::{
    AuthorityDomainId, AuthorityDomainRef, DisclosureBudget, OriginId, PluralityLawCard,
    PluralityLawCardError, PluralityLawConcealment, PluralityLawEmission,
    PluralityLawEvidencePosture, PluralityLawFamily, PluralityLawName, PluralityLawObstruction,
    PluralityLawObstructionKind, PluralityLawReading, PluralityLawRef, PluralityLawRefError,
    PluralityLawRegistry, PluralityLawRegistryError, PluralityLawRequirement, WitnessAttestation,
    WitnessCompatibilityRule, WitnessKind, WitnessReceipt,
};

fn law_name(byte: u8) -> PluralityLawName {
    PluralityLawName::from_bytes([byte; 32])
}

fn authority_ref(byte: u8) -> AuthorityDomainRef {
    AuthorityDomainRef::new(
        OriginId::from_bytes([byte; 32]),
        AuthorityDomainId::from_bytes([byte.wrapping_add(1); 32]),
    )
}

fn law_ref(byte: u8, version: u32) -> Result<PluralityLawRef, PluralityLawRefError> {
    PluralityLawRef::new(PluralityLawFamily::Settlement, law_name(byte), version)
}

fn law_card(law_ref: PluralityLawRef) -> Result<PluralityLawCard, PluralityLawCardError> {
    PluralityLawCard::new(
        law_ref,
        vec![
            PluralityLawRequirement::SupportPins,
            PluralityLawRequirement::FrontierDigest,
            PluralityLawRequirement::PostureFloor,
        ],
        vec![PluralityLawEmission::PluralArtifact],
        vec![PluralityLawConcealment::SealedMemberSourceChain],
        PluralityLawEvidencePosture::SelfWitnessIntegrityOnly,
    )
}

#[test]
fn public_plurality_law_registry_registers_machine_readable_cards(
) -> Result<(), Box<dyn std::error::Error>> {
    let law_ref = law_ref(0x51, 1)?;
    let card = law_card(law_ref)?;
    let mut registry = PluralityLawRegistry::new();

    registry.register(card.clone())?;

    assert_eq!(registry.card(&law_ref), Some(&card));
    assert_eq!(card.law_ref(), law_ref);
    assert_eq!(card.version(), 1);
    assert!(card
        .requires()
        .contains(&PluralityLawRequirement::FrontierDigest));
    assert_eq!(
        registry.register(card),
        Err(PluralityLawRegistryError::DuplicateLaw { law_ref })
    );
    Ok(())
}

#[test]
fn public_plurality_law_ref_requires_name_and_version() {
    assert_eq!(
        PluralityLawRef::new(PluralityLawFamily::Settlement, law_name(0x00), 1),
        Err(PluralityLawRefError::EmptyName)
    );
    assert_eq!(
        PluralityLawRef::new(PluralityLawFamily::Settlement, law_name(0x55), 0),
        Err(PluralityLawRefError::ZeroVersion)
    );
}

#[test]
fn public_plurality_law_reading_identity_binds_law_name_and_version(
) -> Result<(), Box<dyn std::error::Error>> {
    let witness = WitnessReceipt::self_witness([0xAA; 32], [0xBB; 32]);
    let v1 = PluralityLawReading::new(
        law_ref(0x52, 1)?,
        [0xCC; 32],
        witness,
        DisclosureBudget::AuthorityScoped,
    );
    let v2 = PluralityLawReading::new(
        law_ref(0x52, 2)?,
        [0xCC; 32],
        witness,
        DisclosureBudget::AuthorityScoped,
    );

    assert_eq!(v1.law_ref().version(), 1);
    assert_eq!(v2.law_ref().version(), 2);
    assert_ne!(v1.digest(), v2.digest());
    Ok(())
}

#[test]
fn public_plurality_law_reading_does_not_promote_integrity_only_receipts(
) -> Result<(), Box<dyn std::error::Error>> {
    let witness = WitnessReceipt::new(
        WitnessKind::SignedWitness,
        [0xAA; 32],
        [0xBB; 32],
        WitnessCompatibilityRule::StableV1,
        WitnessAttestation::IntegrityOnly,
    )?;
    let reading = PluralityLawReading::new(
        law_ref(0x56, 1)?,
        [0xCC; 32],
        witness,
        DisclosureBudget::AuthorityScoped,
    );

    assert_eq!(
        reading.evidence_posture(),
        PluralityLawEvidencePosture::SelfWitnessIntegrityOnly
    );
    Ok(())
}

#[test]
fn public_adapter_provided_laws_route_through_authority_without_app_nouns(
) -> Result<(), Box<dyn std::error::Error>> {
    let authority = authority_ref(0xA0);
    let law_ref = PluralityLawRef::new(
        PluralityLawFamily::adapter_provided(authority),
        law_name(0x53),
        1,
    )?;
    let mut registry = PluralityLawRegistry::new();
    registry.register(law_card(law_ref)?)?;

    assert_eq!(
        registry.authorize(law_ref, None),
        Err(PluralityLawObstruction::new(
            PluralityLawObstructionKind::UnauthorizedLaw,
            law_ref,
            None,
        ))
    );
    assert!(registry.authorize(law_ref, Some(authority)).is_ok());
    Ok(())
}

#[test]
fn public_unsupported_law_execution_yields_typed_obstruction(
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = PluralityLawRegistry::new();
    let law_ref = law_ref(0x54, 1)?;

    assert_eq!(
        registry.authorize(law_ref, Some(authority_ref(0xB0))),
        Err(PluralityLawObstruction::new(
            PluralityLawObstructionKind::UnsupportedLaw,
            law_ref,
            Some(authority_ref(0xB0)),
        ))
    );
    Ok(())
}
