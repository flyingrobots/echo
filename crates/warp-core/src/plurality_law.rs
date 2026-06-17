// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Named plurality law registry, cards, readings, and obstruction evidence.

use std::collections::BTreeMap;

use blake3::Hasher;
use thiserror::Error;

use crate::ident::Hash;
use crate::revelation::AuthorityDomainRef;
use crate::sealed_membership::DisclosureBudget;
use crate::witness::{WitnessAttestation, WitnessReceipt};

const LAW_NAME_DOMAIN: &[u8] = b"echo.plurality-law.name.v1\0";
const LAW_REF_DOMAIN: &[u8] = b"echo.plurality-law.ref.v1\0";
const LAW_CARD_DOMAIN: &[u8] = b"echo.plurality-law.card.v1\0";
const LAW_READING_DOMAIN: &[u8] = b"echo.plurality-law.reading.v1\0";
const LAW_OBSTRUCTION_DOMAIN: &[u8] = b"echo.plurality-law.obstruction.v1\0";

/// Hash-backed plurality law name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluralityLawName(Hash);

impl PluralityLawName {
    /// Constructs a law name from canonical bytes.
    #[must_use]
    pub const fn from_bytes(bytes: Hash) -> Self {
        Self(bytes)
    }

    /// Derives a law name from a stable label.
    #[must_use]
    pub fn from_label(label: &str) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(LAW_NAME_DOMAIN);
        hash_len(&mut hasher, label.len());
        hasher.update(label.as_bytes());
        Self(hasher.finalize().into())
    }

    /// Returns the canonical law-name bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Plurality law family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralityLawFamily {
    /// Settlement law family.
    Settlement,
    /// Collapse law family.
    Collapse,
    /// Conflict-preserving law family.
    ConflictPreserving,
    /// Quorum law family.
    Quorum,
    /// Authority law family.
    Authority,
    /// Adapter-provided law family scoped to an authority-domain digest.
    AdapterProvided {
        /// Digest of the authority domain that owns the adapter-provided law family.
        authority_digest: Hash,
    },
}

impl PluralityLawFamily {
    /// Constructs an adapter-provided law family from an authority domain.
    #[must_use]
    pub fn adapter_provided(authority: AuthorityDomainRef) -> Self {
        Self::AdapterProvided {
            authority_digest: authority_digest(authority),
        }
    }

    fn hash_into(self, hasher: &mut Hasher) {
        match self {
            Self::Settlement => {
                hasher.update(&[0x01]);
            }
            Self::Collapse => {
                hasher.update(&[0x02]);
            }
            Self::ConflictPreserving => {
                hasher.update(&[0x03]);
            }
            Self::Quorum => {
                hasher.update(&[0x04]);
            }
            Self::Authority => {
                hasher.update(&[0x05]);
            }
            Self::AdapterProvided { authority_digest } => {
                hasher.update(&[0x06]);
                hasher.update(&authority_digest);
            }
        }
    }
}

/// Error raised when constructing an invalid plurality law reference.
#[derive(Error, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluralityLawRefError {
    /// Law names must not be the all-zero digest.
    #[error("plurality law name must be non-empty")]
    EmptyName,
    /// Law versions start at 1.
    #[error("plurality law version must be non-zero")]
    ZeroVersion,
}

/// Name, family, and version for a plurality law.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluralityLawRef {
    family: PluralityLawFamily,
    name: PluralityLawName,
    version: u32,
}

impl PluralityLawRef {
    /// Constructs a named, versioned plurality law reference.
    ///
    /// # Errors
    ///
    /// Returns [`PluralityLawRefError::ZeroVersion`] when `version` is zero.
    pub fn new(
        family: PluralityLawFamily,
        name: PluralityLawName,
        version: u32,
    ) -> Result<Self, PluralityLawRefError> {
        if name.as_bytes().iter().all(|byte| *byte == 0) {
            return Err(PluralityLawRefError::EmptyName);
        }
        if version == 0 {
            return Err(PluralityLawRefError::ZeroVersion);
        }
        Ok(Self {
            family,
            name,
            version,
        })
    }

    /// Constructs the v1 settlement-law reference for an existing policy id.
    ///
    /// # Errors
    ///
    /// Returns [`PluralityLawRefError::EmptyName`] when `policy_id` is the
    /// all-zero digest.
    pub fn settlement_policy(policy_id: Hash) -> Result<Self, PluralityLawRefError> {
        Self::new(
            PluralityLawFamily::Settlement,
            PluralityLawName::from_bytes(policy_id),
            1,
        )
    }

    /// Constructs the v1 collapse-law reference for an existing policy id.
    ///
    /// # Errors
    ///
    /// Returns [`PluralityLawRefError::EmptyName`] when `policy_id` is the
    /// all-zero digest.
    pub fn collapse_policy(policy_id: Hash) -> Result<Self, PluralityLawRefError> {
        Self::new(
            PluralityLawFamily::Collapse,
            PluralityLawName::from_bytes(policy_id),
            1,
        )
    }

    /// Returns the law family.
    #[must_use]
    pub const fn family(self) -> PluralityLawFamily {
        self.family
    }

    /// Returns the law name.
    #[must_use]
    pub const fn name(self) -> PluralityLawName {
        self.name
    }

    /// Returns the law version.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }

    /// Returns the canonical law reference digest.
    #[must_use]
    pub fn digest(self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(LAW_REF_DOMAIN);
        self.hash_into(&mut hasher);
        hasher.finalize().into()
    }

    fn hash_into(self, hasher: &mut Hasher) {
        self.family.hash_into(hasher);
        hasher.update(self.name.as_bytes());
        hasher.update(&self.version.to_le_bytes());
    }
}

/// Machine-readable requirement named by a plurality Law Card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralityLawRequirement {
    /// The law requires member support pins.
    SupportPins,
    /// The law requires the retained settlement frontier digest.
    FrontierDigest,
    /// The law requires the posture floor.
    PostureFloor,
    /// The law requires proof-envelope binding.
    ProofBinding,
    /// The law requires witness receipt evidence.
    WitnessReceipt,
    /// The law requires a capability presentation.
    CapabilityPresentation,
    /// The law requires an explicit disclosure budget.
    DisclosureBudget,
}

impl PluralityLawRequirement {
    const fn tag(self) -> u8 {
        match self {
            Self::SupportPins => 0x01,
            Self::FrontierDigest => 0x02,
            Self::PostureFloor => 0x03,
            Self::ProofBinding => 0x04,
            Self::WitnessReceipt => 0x05,
            Self::CapabilityPresentation => 0x06,
            Self::DisclosureBudget => 0x07,
        }
    }
}

/// Machine-readable emission named by a plurality Law Card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralityLawEmission {
    /// The law emits a plural artifact.
    PluralArtifact,
    /// The law emits a derived shell.
    DerivedShell,
    /// The law emits conflict residue.
    ConflictResidue,
    /// The law emits obstruction evidence.
    ObstructionEvidence,
    /// The law emits an audit reading.
    AuditReading,
}

impl PluralityLawEmission {
    const fn tag(self) -> u8 {
        match self {
            Self::PluralArtifact => 0x01,
            Self::DerivedShell => 0x02,
            Self::ConflictResidue => 0x03,
            Self::ObstructionEvidence => 0x04,
            Self::AuditReading => 0x05,
        }
    }
}

/// Machine-readable concealed material named by a plurality Law Card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralityLawConcealment {
    /// The law conceals sealed member source chains.
    SealedMemberSourceChain,
    /// The law conceals member blinding material.
    MemberBlindingMaterial,
    /// The law conceals private member history.
    PrivateMemberHistory,
}

impl PluralityLawConcealment {
    const fn tag(self) -> u8 {
        match self {
            Self::SealedMemberSourceChain => 0x01,
            Self::MemberBlindingMaterial => 0x02,
            Self::PrivateMemberHistory => 0x03,
        }
    }
}

/// Evidence posture attached to plurality law execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralityLawEvidencePosture {
    /// Integrity-only evidence, including E1 self-witness scaffolding.
    SelfWitnessIntegrityOnly,
    /// Independent external witness evidence.
    ExternalWitness,
}

impl PluralityLawEvidencePosture {
    const fn tag(self) -> u8 {
        match self {
            Self::SelfWitnessIntegrityOnly => 0x01,
            Self::ExternalWitness => 0x02,
        }
    }
}

/// Error raised when constructing an invalid Law Card.
#[derive(Error, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluralityLawCardError {
    /// A Law Card must name at least one emitted artifact or reading.
    #[error("plurality Law Card must name at least one emission")]
    MissingEmission,
}

/// Machine-readable Law Card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluralityLawCard {
    law_ref: PluralityLawRef,
    requires: Vec<PluralityLawRequirement>,
    emits: Vec<PluralityLawEmission>,
    conceals: Vec<PluralityLawConcealment>,
    evidence_posture: PluralityLawEvidencePosture,
}

impl PluralityLawCard {
    /// Constructs a machine-readable Law Card.
    ///
    /// Requirements, emissions, and concealments are sorted and deduplicated so
    /// card identity does not depend on caller vector ordering.
    ///
    /// # Errors
    ///
    /// Returns [`PluralityLawCardError::MissingEmission`] when the card names
    /// no emitted artifact, obstruction, or reading.
    pub fn new(
        law_ref: PluralityLawRef,
        requires: Vec<PluralityLawRequirement>,
        emits: Vec<PluralityLawEmission>,
        conceals: Vec<PluralityLawConcealment>,
        evidence_posture: PluralityLawEvidencePosture,
    ) -> Result<Self, PluralityLawCardError> {
        let emits = normalized(emits);
        if emits.is_empty() {
            return Err(PluralityLawCardError::MissingEmission);
        }
        Ok(Self {
            law_ref,
            requires: normalized(requires),
            emits,
            conceals: normalized(conceals),
            evidence_posture,
        })
    }

    /// Returns the law reference named by this card.
    #[must_use]
    pub const fn law_ref(&self) -> PluralityLawRef {
        self.law_ref
    }

    /// Returns the law version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.law_ref.version()
    }

    /// Returns required evidence/support facts.
    #[must_use]
    pub fn requires(&self) -> &[PluralityLawRequirement] {
        &self.requires
    }

    /// Returns emitted artifact/reading classes.
    #[must_use]
    pub fn emits(&self) -> &[PluralityLawEmission] {
        &self.emits
    }

    /// Returns concealed material classes.
    #[must_use]
    pub fn conceals(&self) -> &[PluralityLawConcealment] {
        &self.conceals
    }

    /// Returns the evidence posture for this Law Card.
    #[must_use]
    pub const fn evidence_posture(&self) -> PluralityLawEvidencePosture {
        self.evidence_posture
    }

    /// Returns the canonical Law Card digest.
    #[must_use]
    pub fn digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(LAW_CARD_DOMAIN);
        self.law_ref.hash_into(&mut hasher);
        hash_tag_vec(
            &mut hasher,
            self.requires
                .iter()
                .copied()
                .map(PluralityLawRequirement::tag),
        );
        hash_tag_vec(
            &mut hasher,
            self.emits.iter().copied().map(PluralityLawEmission::tag),
        );
        hash_tag_vec(
            &mut hasher,
            self.conceals
                .iter()
                .copied()
                .map(PluralityLawConcealment::tag),
        );
        hasher.update(&[self.evidence_posture.tag()]);
        hasher.finalize().into()
    }
}

/// Registry errors raised while registering Law Cards.
#[derive(Error, Clone, Debug, PartialEq, Eq)]
pub enum PluralityLawRegistryError {
    /// A Law Card for this reference is already registered.
    #[error("plurality law is already registered: {law_ref:?}")]
    DuplicateLaw {
        /// Duplicate law reference.
        law_ref: PluralityLawRef,
    },
}

/// Deterministic plurality law registry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluralityLawRegistry {
    cards: BTreeMap<PluralityLawRef, PluralityLawCard>,
}

impl PluralityLawRegistry {
    /// Creates an empty plurality law registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cards: BTreeMap::new(),
        }
    }

    /// Registers a Law Card.
    ///
    /// # Errors
    ///
    /// Returns [`PluralityLawRegistryError::DuplicateLaw`] when a card with
    /// the same law reference is already registered.
    pub fn register(&mut self, card: PluralityLawCard) -> Result<(), PluralityLawRegistryError> {
        let law_ref = card.law_ref();
        if self.cards.contains_key(&law_ref) {
            return Err(PluralityLawRegistryError::DuplicateLaw { law_ref });
        }
        self.cards.insert(law_ref, card);
        Ok(())
    }

    /// Returns a registered Law Card.
    #[must_use]
    pub fn card(&self, law_ref: &PluralityLawRef) -> Option<&PluralityLawCard> {
        self.cards.get(law_ref)
    }

    /// Returns the number of registered Law Cards.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Returns true when the registry contains no Law Cards.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Authorizes execution of a registered law.
    ///
    /// # Errors
    ///
    /// Returns [`PluralityLawObstruction`] when the law is unsupported by this
    /// registry or when an adapter-provided law is not authorized by its
    /// owning authority domain.
    pub fn authorize(
        &self,
        law_ref: PluralityLawRef,
        authorized_by: Option<AuthorityDomainRef>,
    ) -> Result<PluralityLawAuthorization, PluralityLawObstruction> {
        let card = self.card(&law_ref).ok_or_else(|| {
            PluralityLawObstruction::new(
                PluralityLawObstructionKind::UnsupportedLaw,
                law_ref,
                authorized_by,
            )
        })?;
        if let PluralityLawFamily::AdapterProvided {
            authority_digest: required_authority,
        } = law_ref.family()
        {
            if authorized_by.map(authority_digest) != Some(required_authority) {
                return Err(PluralityLawObstruction::new(
                    PluralityLawObstructionKind::UnauthorizedLaw,
                    law_ref,
                    authorized_by,
                ));
            }
        }
        Ok(PluralityLawAuthorization {
            law_ref,
            card_digest: card.digest(),
            authorized_by,
        })
    }
}

/// Successful law authorization evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluralityLawAuthorization {
    /// Authorized law reference.
    pub law_ref: PluralityLawRef,
    /// Digest of the registered Law Card.
    pub card_digest: Hash,
    /// Authority that authorized execution, if any.
    pub authorized_by: Option<AuthorityDomainRef>,
}

/// Typed plurality law obstruction kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralityLawObstructionKind {
    /// The requested law is not registered.
    UnsupportedLaw,
    /// The requested law exists but lacks required authority.
    UnauthorizedLaw,
}

impl PluralityLawObstructionKind {
    const fn tag(self) -> u8 {
        match self {
            Self::UnsupportedLaw => 0x01,
            Self::UnauthorizedLaw => 0x02,
        }
    }
}

/// Typed obstruction evidence for plurality law execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluralityLawObstruction {
    /// Obstruction kind.
    pub kind: PluralityLawObstructionKind,
    /// Law reference that could not execute.
    pub law_ref: PluralityLawRef,
    /// Authority-domain digest that attempted to authorize execution, if any.
    pub authorized_by: Option<Hash>,
}

impl PluralityLawObstruction {
    /// Constructs typed law obstruction evidence.
    #[must_use]
    pub fn new(
        kind: PluralityLawObstructionKind,
        law_ref: PluralityLawRef,
        authorized_by: Option<AuthorityDomainRef>,
    ) -> Self {
        let authorized_by = authorized_by.map(authority_digest);
        Self {
            kind,
            law_ref,
            authorized_by,
        }
    }

    /// Returns the canonical obstruction digest.
    #[must_use]
    pub fn digest(self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(LAW_OBSTRUCTION_DOMAIN);
        hasher.update(&[self.kind.tag()]);
        self.law_ref.hash_into(&mut hasher);
        hash_optional_digest(&mut hasher, self.authorized_by);
        hasher.finalize().into()
    }
}

/// Witnessed plurality law reading identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluralityLawReading {
    law_ref: PluralityLawRef,
    support_digest: Hash,
    witness_receipt: WitnessReceipt,
    evidence_posture: PluralityLawEvidencePosture,
    disclosure_budget: DisclosureBudget,
}

impl PluralityLawReading {
    /// Constructs a witnessed plurality law reading.
    #[must_use]
    pub fn new(
        law_ref: PluralityLawRef,
        support_digest: Hash,
        witness_receipt: WitnessReceipt,
        disclosure_budget: DisclosureBudget,
    ) -> Self {
        let evidence_posture = evidence_posture_for(witness_receipt);
        Self {
            law_ref,
            support_digest,
            witness_receipt,
            evidence_posture,
            disclosure_budget,
        }
    }

    /// Returns the law reference used for this reading.
    #[must_use]
    pub const fn law_ref(self) -> PluralityLawRef {
        self.law_ref
    }

    /// Returns the retained support digest interpreted by the law.
    #[must_use]
    pub const fn support_digest(self) -> Hash {
        self.support_digest
    }

    /// Returns the witness receipt supporting this reading.
    #[must_use]
    pub const fn witness_receipt(self) -> WitnessReceipt {
        self.witness_receipt
    }

    /// Returns the evidence posture for this reading.
    #[must_use]
    pub const fn evidence_posture(self) -> PluralityLawEvidencePosture {
        self.evidence_posture
    }

    /// Returns the disclosure budget for this reading.
    #[must_use]
    pub const fn disclosure_budget(self) -> DisclosureBudget {
        self.disclosure_budget
    }

    /// Returns the canonical law reading digest.
    #[must_use]
    pub fn digest(self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(LAW_READING_DOMAIN);
        self.law_ref.hash_into(&mut hasher);
        hasher.update(&self.support_digest);
        hasher.update(&self.witness_receipt.digest());
        hasher.update(&[self.evidence_posture.tag()]);
        hasher.update(&[disclosure_budget_tag(self.disclosure_budget)]);
        hasher.finalize().into()
    }
}

fn evidence_posture_for(receipt: WitnessReceipt) -> PluralityLawEvidencePosture {
    match receipt.attestation() {
        WitnessAttestation::IntegrityOnly => PluralityLawEvidencePosture::SelfWitnessIntegrityOnly,
        WitnessAttestation::IndependentAttestation => PluralityLawEvidencePosture::ExternalWitness,
    }
}

fn normalized<T: Ord>(mut values: Vec<T>) -> Vec<T> {
    values.sort();
    values.dedup();
    values
}

fn hash_len(hasher: &mut Hasher, len: usize) {
    hasher.update(&(len as u64).to_le_bytes());
}

fn hash_tag_vec(hasher: &mut Hasher, tags: impl Iterator<Item = u8>) {
    let tags: Vec<u8> = tags.collect();
    hash_len(hasher, tags.len());
    hasher.update(&tags);
}

fn authority_digest(authority: AuthorityDomainRef) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(authority.origin_id.as_bytes());
    hasher.update(authority.domain_id.as_bytes());
    hasher.finalize().into()
}

fn hash_optional_digest(hasher: &mut Hasher, digest: Option<Hash>) {
    if let Some(digest) = digest {
        hasher.update(&[0x01]);
        hasher.update(&digest);
    } else {
        hasher.update(&[0x00]);
    }
}

const fn disclosure_budget_tag(budget: DisclosureBudget) -> u8 {
    match budget {
        DisclosureBudget::Public => 0x01,
        DisclosureBudget::AuthorityScoped => 0x02,
        DisclosureBudget::CapabilityScoped => 0x03,
        DisclosureBudget::HolderOnly => 0x04,
        DisclosureBudget::ZeroKnowledge => 0x05,
    }
}
