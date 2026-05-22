// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Retained contract evidence references.
//!
//! CAS names bytes. These references name retained evidence under contract
//! semantics so missing material can obstruct explicitly instead of becoming an
//! empty read, cache hit, or generic runtime failure.

use blake3::Hasher;

use crate::contract_obstruction::ContractObstruction;
use crate::contract_registry::{ContractEvidenceIdentity, ContractOperationKind};
use crate::ident::Hash;

const RETAINED_EVIDENCE_COORDINATE_ID_DOMAIN: &[u8] = b"echo:retained-evidence-coordinate-id:v1\0";
const RETAINED_EVIDENCE_REF_ID_DOMAIN: &[u8] = b"echo:retained-evidence-ref-id:v1\0";

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
        let mut hasher = Hasher::new();
        hasher.update(RETAINED_EVIDENCE_REF_ID_DOMAIN);
        hasher.update(&self.coordinate.coordinate_id());
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
    /// Required retained evidence is missing.
    MissingRetention(ContractObstruction),
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
        Self::MissingRetention(coordinate.missing_retention_obstruction())
    }

    /// Builds a missing posture for a descriptor whose content bytes are absent.
    #[must_use]
    pub fn missing_content(reference: &RetainedEvidenceRef) -> Self {
        Self::MissingRetention(reference.missing_retention_obstruction())
    }

    /// Returns the missing-retention obstruction when this posture obstructs.
    #[must_use]
    pub fn obstruction(&self) -> Option<&ContractObstruction> {
        match self {
            Self::Available(_) => None,
            Self::MissingRetention(obstruction) => Some(obstruction),
        }
    }
}

fn update_contract_identity(hasher: &mut Hasher, contract: &ContractEvidenceIdentity) {
    hasher.update(contract.package_id.as_bytes());
    update_string(hasher, &contract.package_name);
    update_string(hasher, &contract.package_version);
    update_string(hasher, &contract.artifact_hash_hex);
    update_string(hasher, &contract.codec_id);
    hasher.update(&contract.registry_version.to_le_bytes());
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
