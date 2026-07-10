// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Retained contract evidence references.
//!
//! CAS names bytes. These references name retained evidence under contract
//! semantics so missing material can obstruct explicitly instead of becoming an
//! empty read, cache hit, or generic runtime failure.

use blake3::Hasher;

use crate::contract_obstruction::{
    ContractObstruction, ContractObstructionKind, ContractObstructionSubject,
};
use crate::contract_registry::{ContractEvidenceIdentity, ContractOperationKind};
use crate::ident::Hash;

const RETAINED_EVIDENCE_COORDINATE_ID_DOMAIN: &[u8] = b"echo:retained-evidence-coordinate-id:v1\0";
const RETAINED_EVIDENCE_REF_ID_DOMAIN: &[u8] = b"echo:retained-evidence-ref-id:v1\0";
const RETAINED_EVIDENCE_BOUNDARY_POSTURE_ID_DOMAIN: &[u8] =
    b"echo:retained-evidence-boundary-posture-id:v1\0";

/// Semantic role of retained contract evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedEvidenceRole {
    /// Generated contract artifact bytes.
    ContractArtifact,
    /// Scheduler receipt or receipt-adjacent material.
    ContractReceipt,
    /// Law witness or witness-adjacent material.
    Witness,
    /// Reading payload bytes.
    ReadingPayload,
    /// Encoded reading envelope bytes.
    ReadingEnvelope,
    /// Generated observer artifact bytes.
    ObserverArtifact,
}

impl RetainedEvidenceRole {
    fn tag(self) -> u8 {
        match self {
            Self::ContractArtifact => 0,
            Self::ContractReceipt => 1,
            Self::Witness => 2,
            Self::ReadingPayload => 3,
            Self::ReadingEnvelope => 4,
            Self::ObserverArtifact => 5,
        }
    }
}

/// Boundary layer occupied by retained evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedEvidenceLayer {
    /// Coordinates and anchors needed to reintegrate the evidence with causal
    /// history.
    ReintegrationCore,
    /// Proof-bearing material that certifies admission or observation
    /// lawfulness.
    WitnessCore,
    /// Operational receipt or explanation shell.
    ReceiptShell,
}

impl RetainedEvidenceLayer {
    fn tag(self) -> u8 {
        match self {
            Self::ReintegrationCore => 0,
            Self::WitnessCore => 1,
            Self::ReceiptShell => 2,
        }
    }
}

/// Origin posture for retained evidence at a boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedEvidenceOrigin {
    /// Native Echo retained evidence.
    Native,
    /// Evidence translated through a substrate/refinement boundary.
    Translated,
    /// Fixture or test-only evidence.
    Fixture,
    /// Evidence derived from other retained support.
    Derived,
    /// Opaque evidence whose origin cannot be refined locally.
    Opaque,
}

impl RetainedEvidenceOrigin {
    fn tag(self) -> u8 {
        match self {
            Self::Native => 0,
            Self::Translated => 1,
            Self::Fixture => 2,
            Self::Derived => 3,
            Self::Opaque => 4,
        }
    }
}

/// Proof strength carried by retained evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedEvidenceProofStrength {
    /// No accepted proof is carried.
    None,
    /// Digest-only evidence.
    DigestOnly,
    /// Signature-backed evidence.
    Signature,
    /// Replay-certificate-backed evidence.
    ReplayCertificate,
    /// Merkle opening or equivalent inclusion proof.
    MerkleOpening,
    /// Zero-knowledge proof-backed evidence.
    ZkProof,
    /// Composite proof bundle.
    Composite,
}

impl RetainedEvidenceProofStrength {
    fn tag(self) -> u8 {
        match self {
            Self::None => 0,
            Self::DigestOnly => 1,
            Self::Signature => 2,
            Self::ReplayCertificate => 3,
            Self::MerkleOpening => 4,
            Self::ZkProof => 5,
            Self::Composite => 6,
        }
    }
}

/// Access posture for retained evidence at the current observer boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedEvidenceAccess {
    /// The current observer may reveal the retained bytes.
    Revealable,
    /// The current observer may cite the evidence but not reveal bytes.
    CitationOnly,
    /// The evidence is explicitly redacted at this boundary.
    Redacted,
    /// Authority denies the requested access.
    AuthorityBlocked,
    /// Required decryption or unsealing key is unavailable.
    KeyUnavailable,
    /// The participant/runtime does not support this evidence kind.
    Unsupported,
}

impl RetainedEvidenceAccess {
    fn tag(self) -> u8 {
        match self {
            Self::Revealable => 0,
            Self::CitationOnly => 1,
            Self::Redacted => 2,
            Self::AuthorityBlocked => 3,
            Self::KeyUnavailable => 4,
            Self::Unsupported => 5,
        }
    }
}

/// Completeness posture for retained evidence support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedEvidenceCompleteness {
    /// Obligated support is complete for this boundary.
    Complete,
    /// Some support is present, but the obligation is only partially met.
    Partial,
    /// Loss, redaction, or unavailability was declared rather than hidden.
    DeclaredLost,
    /// Evidence is stale for the requested basis or context.
    Stale,
    /// No semantic coordinate descriptor exists.
    MissingCoordinate,
    /// A descriptor exists, but the retained content is absent.
    MissingContent,
    /// Retained content is present but corrupt or hash-invalid.
    Corrupt,
    /// The boundary cannot admit the evidence for the requested purpose.
    Obstructed,
}

impl RetainedEvidenceCompleteness {
    fn tag(self) -> u8 {
        match self {
            Self::Complete => 0,
            Self::Partial => 1,
            Self::DeclaredLost => 2,
            Self::Stale => 3,
            Self::MissingCoordinate => 4,
            Self::MissingContent => 5,
            Self::Corrupt => 6,
            Self::Obstructed => 7,
        }
    }
}

/// Semantic coordinate for retained contract evidence.
///
/// The coordinate names what the bytes answer. It is intentionally separate
/// from content hash because equal bytes can answer different semantic
/// questions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedEvidenceCoordinate {
    /// Installed contract evidence identity that owns the coordinate.
    pub contract: ContractEvidenceIdentity,
    /// Retained evidence role.
    pub role: RetainedEvidenceRole,
    /// Domain-separated semantic digest for the exact receipt, witness,
    /// reading, or artifact coordinate.
    pub semantic_digest: Hash,
}

impl RetainedEvidenceCoordinate {
    /// Builds a retained evidence coordinate.
    #[must_use]
    pub fn new(
        contract: ContractEvidenceIdentity,
        role: RetainedEvidenceRole,
        semantic_digest: Hash,
    ) -> Self {
        Self {
            contract,
            role,
            semantic_digest,
        }
    }

    /// Stable coordinate id used for missing-retention obstruction subjects.
    #[must_use]
    pub fn coordinate_id(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(RETAINED_EVIDENCE_COORDINATE_ID_DOMAIN);
        update_contract_identity(&mut hasher, &self.contract);
        hasher.update(&[self.role.tag()]);
        hasher.update(&self.semantic_digest);
        hasher.finalize().into()
    }

    /// Builds a typed missing-retention obstruction for this coordinate.
    #[must_use]
    pub fn missing_retention_obstruction(&self) -> ContractObstruction {
        ContractObstruction::missing_retention(self.coordinate_id())
            .with_contract(self.contract.clone())
    }
}

/// First-class reference to retained contract evidence bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedEvidenceRef {
    /// Semantic coordinate that explains what the retained bytes answer.
    pub coordinate: RetainedEvidenceCoordinate,
    /// Content-only hash for the retained bytes.
    pub content_hash: Hash,
    /// Retained byte length.
    pub byte_len: u64,
}

impl RetainedEvidenceRef {
    /// Builds a retained evidence reference.
    #[must_use]
    pub fn new(coordinate: RetainedEvidenceCoordinate, content_hash: Hash, byte_len: u64) -> Self {
        Self {
            coordinate,
            content_hash,
            byte_len,
        }
    }

    /// Stable id that binds semantic coordinate, content hash, and byte length.
    #[must_use]
    pub fn evidence_ref_id(&self) -> Hash {
        let coordinate_id = self.coordinate.coordinate_id();
        self.evidence_ref_id_with_coordinate_id(&coordinate_id)
    }

    /// Stable ref id when the caller already has the coordinate id.
    #[must_use]
    pub(crate) fn evidence_ref_id_with_coordinate_id(&self, coordinate_id: &Hash) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(RETAINED_EVIDENCE_REF_ID_DOMAIN);
        hasher.update(coordinate_id);
        hasher.update(&self.content_hash);
        hasher.update(&self.byte_len.to_le_bytes());
        hasher.finalize().into()
    }

    /// Builds a typed missing-retention obstruction for this exact retained ref.
    #[must_use]
    pub fn missing_retention_obstruction(&self) -> ContractObstruction {
        ContractObstruction::missing_retention(self.evidence_ref_id())
            .with_contract(self.coordinate.contract.clone())
    }
}

/// Retained evidence availability posture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetainedEvidencePosture {
    /// The retained evidence is locally available.
    Available(RetainedEvidenceRef),
    /// No retained descriptor exists for this semantic coordinate.
    MissingCoordinate {
        /// Coordinate that should eventually name retained evidence.
        coordinate: RetainedEvidenceCoordinate,
        /// Typed missing-retention obstruction for the coordinate.
        obstruction: ContractObstruction,
    },
    /// A descriptor is known, but its retained bytes are unavailable.
    MissingContent {
        /// Retained evidence reference whose bytes are unavailable.
        reference: RetainedEvidenceRef,
        /// Typed missing-retention obstruction for the exact reference.
        obstruction: ContractObstruction,
    },
}

impl RetainedEvidencePosture {
    /// Builds an available retained evidence posture.
    #[must_use]
    pub fn available(reference: RetainedEvidenceRef) -> Self {
        Self::Available(reference)
    }

    /// Builds a missing posture for a semantic coordinate with no local
    /// descriptor.
    #[must_use]
    pub fn missing_coordinate(coordinate: &RetainedEvidenceCoordinate) -> Self {
        Self::MissingCoordinate {
            coordinate: coordinate.clone(),
            obstruction: coordinate.missing_retention_obstruction(),
        }
    }

    /// Builds a missing posture for a descriptor whose content bytes are absent.
    #[must_use]
    pub fn missing_content(reference: &RetainedEvidenceRef) -> Self {
        Self::MissingContent {
            reference: reference.clone(),
            obstruction: reference.missing_retention_obstruction(),
        }
    }

    /// Returns the missing-retention obstruction when this posture obstructs.
    #[must_use]
    pub fn obstruction(&self) -> Option<&ContractObstruction> {
        match self {
            Self::Available(_) => None,
            Self::MissingCoordinate { obstruction, .. }
            | Self::MissingContent { obstruction, .. } => Some(obstruction),
        }
    }
}

/// Observer-facing retained evidence boundary posture.
///
/// This is the projection surface above local byte retention. It keeps
/// semantic identity, witness-ladder layer, evidence origin, proof strength,
/// access, completeness, and obstruction posture distinct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedEvidenceBoundaryPosture {
    /// Semantic coordinate the posture answers for.
    pub coordinate: RetainedEvidenceCoordinate,
    /// Exact retained reference when a descriptor is known.
    pub reference: Option<RetainedEvidenceRef>,
    /// Witness-ladder layer occupied by the evidence.
    pub layer: RetainedEvidenceLayer,
    /// Evidence origin posture.
    pub origin: RetainedEvidenceOrigin,
    /// Proof strength posture.
    pub proof_strength: RetainedEvidenceProofStrength,
    /// Access posture for the current observer boundary.
    pub access: RetainedEvidenceAccess,
    /// Completeness posture for the current support obligation.
    pub completeness: RetainedEvidenceCompleteness,
    /// Typed obstruction when this posture obstructs the boundary claim.
    pub obstruction: Option<ContractObstruction>,
}

impl RetainedEvidenceBoundaryPosture {
    /// Builds available retained evidence that may be cited but not revealed.
    #[must_use]
    pub fn available_citation(
        reference: RetainedEvidenceRef,
        layer: RetainedEvidenceLayer,
        origin: RetainedEvidenceOrigin,
        proof_strength: RetainedEvidenceProofStrength,
    ) -> Self {
        Self::available_with_access(
            reference,
            layer,
            origin,
            proof_strength,
            RetainedEvidenceAccess::CitationOnly,
        )
    }

    /// Builds available retained evidence that may be revealed.
    #[must_use]
    pub fn available_revealable(
        reference: RetainedEvidenceRef,
        layer: RetainedEvidenceLayer,
        origin: RetainedEvidenceOrigin,
        proof_strength: RetainedEvidenceProofStrength,
    ) -> Self {
        Self::available_with_access(
            reference,
            layer,
            origin,
            proof_strength,
            RetainedEvidenceAccess::Revealable,
        )
    }

    /// Builds available retained evidence with an explicit access posture.
    #[must_use]
    pub fn available_with_access(
        reference: RetainedEvidenceRef,
        layer: RetainedEvidenceLayer,
        origin: RetainedEvidenceOrigin,
        proof_strength: RetainedEvidenceProofStrength,
        access: RetainedEvidenceAccess,
    ) -> Self {
        match access {
            RetainedEvidenceAccess::Revealable | RetainedEvidenceAccess::CitationOnly => Self {
                coordinate: reference.coordinate.clone(),
                reference: Some(reference),
                layer,
                origin,
                proof_strength,
                access,
                completeness: RetainedEvidenceCompleteness::Complete,
                obstruction: None,
            },
            RetainedEvidenceAccess::Redacted => {
                Self::redacted(reference, layer, origin, proof_strength)
            }
            RetainedEvidenceAccess::AuthorityBlocked
            | RetainedEvidenceAccess::KeyUnavailable
            | RetainedEvidenceAccess::Unsupported => {
                let obstruction = ContractObstruction::admission_obstruction(
                    ContractObstructionSubject::Retention {
                        retention_id: reference.evidence_ref_id(),
                    },
                )
                .with_contract(reference.coordinate.contract.clone());
                Self {
                    coordinate: reference.coordinate.clone(),
                    reference: Some(reference),
                    layer,
                    origin,
                    proof_strength,
                    access,
                    completeness: RetainedEvidenceCompleteness::Obstructed,
                    obstruction: Some(obstruction),
                }
            }
        }
    }

    /// Builds redacted retained evidence posture. Redaction is declared loss,
    /// not missing content.
    #[must_use]
    pub fn redacted(
        reference: RetainedEvidenceRef,
        layer: RetainedEvidenceLayer,
        origin: RetainedEvidenceOrigin,
        proof_strength: RetainedEvidenceProofStrength,
    ) -> Self {
        Self {
            coordinate: reference.coordinate.clone(),
            reference: Some(reference),
            layer,
            origin,
            proof_strength,
            access: RetainedEvidenceAccess::Redacted,
            completeness: RetainedEvidenceCompleteness::DeclaredLost,
            obstruction: None,
        }
    }

    /// Builds missing-coordinate boundary posture using the existing
    /// `MissingRetention` obstruction for true local retained-coordinate absence.
    #[must_use]
    pub fn missing_coordinate(
        coordinate: RetainedEvidenceCoordinate,
        layer: RetainedEvidenceLayer,
        origin: RetainedEvidenceOrigin,
        proof_strength: RetainedEvidenceProofStrength,
        access: RetainedEvidenceAccess,
    ) -> Self {
        Self {
            obstruction: Some(coordinate.missing_retention_obstruction()),
            coordinate,
            reference: None,
            layer,
            origin,
            proof_strength,
            access,
            completeness: RetainedEvidenceCompleteness::MissingCoordinate,
        }
    }

    /// Builds missing-content boundary posture using the existing
    /// `MissingRetention` obstruction for true local retained-content absence.
    #[must_use]
    pub fn missing_content(
        reference: RetainedEvidenceRef,
        layer: RetainedEvidenceLayer,
        origin: RetainedEvidenceOrigin,
        proof_strength: RetainedEvidenceProofStrength,
        access: RetainedEvidenceAccess,
    ) -> Self {
        Self {
            obstruction: Some(reference.missing_retention_obstruction()),
            coordinate: reference.coordinate.clone(),
            reference: Some(reference),
            layer,
            origin,
            proof_strength,
            access,
            completeness: RetainedEvidenceCompleteness::MissingContent,
        }
    }

    /// Builds posture for an unsupported proof/evidence kind. This is an
    /// admission obstruction, not missing retention.
    #[must_use]
    pub fn unsupported_evidence_kind(
        coordinate: RetainedEvidenceCoordinate,
        layer: RetainedEvidenceLayer,
        origin: RetainedEvidenceOrigin,
        proof_strength: RetainedEvidenceProofStrength,
    ) -> Self {
        let obstruction =
            ContractObstruction::admission_obstruction(ContractObstructionSubject::Retention {
                retention_id: coordinate.coordinate_id(),
            })
            .with_contract(coordinate.contract.clone());
        Self {
            coordinate,
            reference: None,
            layer,
            origin,
            proof_strength,
            access: RetainedEvidenceAccess::Unsupported,
            completeness: RetainedEvidenceCompleteness::Obstructed,
            obstruction: Some(obstruction),
        }
    }

    /// Stable id for the full observer-facing boundary posture.
    #[must_use]
    pub fn boundary_posture_id(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(RETAINED_EVIDENCE_BOUNDARY_POSTURE_ID_DOMAIN);
        hasher.update(&self.coordinate.coordinate_id());
        match &self.reference {
            Some(reference) => {
                hasher.update(&[1]);
                hasher.update(&reference.evidence_ref_id());
            }
            None => {
                hasher.update(&[0]);
            }
        }
        hasher.update(&[self.layer.tag()]);
        hasher.update(&[self.origin.tag()]);
        hasher.update(&[self.proof_strength.tag()]);
        hasher.update(&[self.access.tag()]);
        hasher.update(&[self.completeness.tag()]);
        match &self.obstruction {
            Some(obstruction) => {
                hasher.update(&[1]);
                update_obstruction(&mut hasher, obstruction);
            }
            None => {
                hasher.update(&[0]);
            }
        }
        hasher.finalize().into()
    }

    /// Returns true only when this boundary posture grants byte revelation.
    #[must_use]
    pub fn grants_reveal(&self) -> bool {
        self.access == RetainedEvidenceAccess::Revealable
            && self.completeness == RetainedEvidenceCompleteness::Complete
            && self.obstruction.is_none()
    }
}

fn update_contract_identity(hasher: &mut Hasher, contract: &ContractEvidenceIdentity) {
    hasher.update(contract.package_id.as_bytes());
    hasher.update(&contract.echo_abi_version.to_le_bytes());
    update_string(hasher, &contract.package_name);
    update_string(hasher, &contract.package_version);
    update_string(hasher, &contract.artifact_hash_hex);
    update_string(hasher, &contract.codec_id);
    hasher.update(&contract.registry_version.to_le_bytes());
    update_string(hasher, &contract.wesley_generator_version);
    hasher.update(&contract.helper_api_version.to_le_bytes());
    update_string(hasher, &contract.schema_sha256_hex);
    hasher.update(&contract.op_id.to_le_bytes());
    hasher.update(&[contract_operation_kind_tag(contract.op_kind)]);
}

fn update_string(hasher: &mut Hasher, value: &str) {
    let bytes = value.as_bytes();
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn contract_operation_kind_tag(kind: ContractOperationKind) -> u8 {
    match kind {
        ContractOperationKind::Mutation => 0,
        ContractOperationKind::Query => 1,
    }
}

fn update_obstruction(hasher: &mut Hasher, obstruction: &ContractObstruction) {
    hasher.update(&[contract_obstruction_kind_tag(obstruction.kind)]);
    update_obstruction_subject(hasher, &obstruction.subject);
    match &obstruction.contract {
        Some(contract) => {
            hasher.update(&[1]);
            update_contract_identity(hasher, contract);
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn contract_obstruction_kind_tag(kind: ContractObstructionKind) -> u8 {
    match kind {
        ContractObstructionKind::UnsupportedOperation => 0,
        ContractObstructionKind::UnsupportedQuery => 1,
        ContractObstructionKind::AdmissionObstruction => 2,
        ContractObstructionKind::RuntimeFault => 3,
        ContractObstructionKind::MissingRetention => 4,
        ContractObstructionKind::StaleBasis => 5,
        ContractObstructionKind::ResidualReading => 6,
        ContractObstructionKind::BudgetExceeded => 7,
    }
}

fn update_obstruction_subject(hasher: &mut Hasher, subject: &ContractObstructionSubject) {
    match subject {
        ContractObstructionSubject::Unspecified => {
            hasher.update(&[0]);
        }
        ContractObstructionSubject::Operation { op_id } => {
            hasher.update(&[1]);
            hasher.update(&op_id.to_le_bytes());
        }
        ContractObstructionSubject::Query { query_id } => {
            hasher.update(&[2]);
            hasher.update(&query_id.to_le_bytes());
        }
        ContractObstructionSubject::Submission { submission_id } => {
            hasher.update(&[3]);
            hasher.update(submission_id);
        }
        ContractObstructionSubject::Ticket { ticket_digest } => {
            hasher.update(&[4]);
            hasher.update(ticket_digest);
        }
        ContractObstructionSubject::Reading { reading_id } => {
            hasher.update(&[5]);
            hasher.update(reading_id);
        }
        ContractObstructionSubject::Retention { retention_id } => {
            hasher.update(&[6]);
            hasher.update(retention_id);
        }
        ContractObstructionSubject::SchedulerFault { fault_id } => {
            hasher.update(&[7]);
            hasher.update(fault_id.as_bytes());
        }
    }
}
