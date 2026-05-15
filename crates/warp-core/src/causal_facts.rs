// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native causal graph fact publication primitives.
//!
//! Facts are world statements. Receipts explain publication or refusal
//! boundaries and reference fact digests; they do not replace facts.

const FACT_DIGEST_DOMAIN: &[u8] = b"echo.causal-fact.v0";

/// Stable digest for one canonical graph fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FactDigest([u8; 32]);

impl FactDigest {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Obstruction kind recorded when artifact registration fails before a handle
/// is issued.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactRegistrationObstructionKind {
    /// Descriptor artifact id did not match artifact identity.
    ArtifactIdMismatch,
    /// Descriptor artifact hash did not match artifact identity.
    ArtifactHashMismatch,
    /// Descriptor schema id did not match artifact schema id.
    SchemaIdMismatch,
    /// Descriptor operation id did not match artifact operation id.
    OperationIdMismatch,
    /// Descriptor requirements digest did not match artifact requirements.
    RequirementsDigestMismatch,
}

impl ArtifactRegistrationObstructionKind {
    fn digest_label(self) -> &'static [u8] {
        match self {
            Self::ArtifactIdMismatch => b"artifact-id-mismatch",
            Self::ArtifactHashMismatch => b"artifact-hash-mismatch",
            Self::SchemaIdMismatch => b"schema-id-mismatch",
            Self::OperationIdMismatch => b"operation-id-mismatch",
            Self::RequirementsDigestMismatch => b"requirements-digest-mismatch",
        }
    }
}

/// Native Echo graph fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphFact {
    /// Echo registered a Wesley-compiled optic artifact and issued a
    /// runtime-local handle.
    ArtifactRegistered {
        /// Echo-owned runtime-local handle id.
        handle_id: String,
        /// Wesley artifact hash.
        artifact_hash: String,
        /// Wesley schema id.
        schema_id: String,
        /// Wesley operation id.
        operation_id: String,
        /// Wesley requirements digest.
        requirements_digest: String,
    },
    /// Echo refused artifact registration before issuing a handle.
    ArtifactRegistrationObstructed {
        /// Artifact hash named by the attempted registration, if available.
        artifact_hash: Option<String>,
        /// Structured obstruction kind.
        obstruction: ArtifactRegistrationObstructionKind,
    },
}

impl GraphFact {
    /// Computes the deterministic fact digest.
    #[must_use]
    pub fn digest(&self) -> FactDigest {
        let mut bytes = Vec::new();
        push_digest_field(&mut bytes, b"domain", FACT_DIGEST_DOMAIN);

        match self {
            Self::ArtifactRegistered {
                handle_id,
                artifact_hash,
                schema_id,
                operation_id,
                requirements_digest,
            } => {
                push_digest_field(&mut bytes, b"variant", b"artifact-registered");
                push_digest_field(&mut bytes, b"handle-id", handle_id.as_bytes());
                push_digest_field(&mut bytes, b"artifact-hash", artifact_hash.as_bytes());
                push_digest_field(&mut bytes, b"schema-id", schema_id.as_bytes());
                push_digest_field(&mut bytes, b"operation-id", operation_id.as_bytes());
                push_digest_field(
                    &mut bytes,
                    b"requirements-digest",
                    requirements_digest.as_bytes(),
                );
            }
            Self::ArtifactRegistrationObstructed {
                artifact_hash,
                obstruction,
            } => {
                push_digest_field(&mut bytes, b"variant", b"artifact-registration-obstructed");
                push_optional_digest_field(
                    &mut bytes,
                    b"artifact-hash",
                    artifact_hash.as_deref().map(str::as_bytes),
                );
                push_digest_field(&mut bytes, b"obstruction", obstruction.digest_label());
            }
        }

        FactDigest(*blake3::hash(&bytes).as_bytes())
    }
}

/// Graph fact plus its deterministic digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedGraphFact {
    /// Published fact.
    pub fact: GraphFact,
    /// Digest computed from the fact.
    pub digest: FactDigest,
}

impl PublishedGraphFact {
    /// Publishes a fact by computing its digest.
    #[must_use]
    pub fn new(fact: GraphFact) -> Self {
        let digest = fact.digest();
        Self { fact, digest }
    }
}

/// Receipt kind for artifact registration publication.
pub const ARTIFACT_REGISTRATION_RECEIPT_KIND: &str = "artifact-registration-receipt";

/// Receipt linking artifact registration to the graph fact it published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRegistrationReceipt {
    /// Stable discriminator for callers and wire adapters.
    pub kind: String,
    /// Echo-owned runtime-local handle id issued by registration.
    pub handle_id: String,
    /// Wesley artifact hash registered by Echo.
    pub artifact_hash: String,
    /// Wesley operation id registered by Echo.
    pub operation_id: String,
    /// Digest of the `ArtifactRegistered` graph fact.
    pub fact_digest: FactDigest,
}

fn push_digest_field(bytes: &mut Vec<u8>, tag: &[u8], value: &[u8]) {
    push_len_prefixed(bytes, tag);
    push_len_prefixed(bytes, value);
}

fn push_optional_digest_field(bytes: &mut Vec<u8>, tag: &[u8], value: Option<&[u8]>) {
    push_len_prefixed(bytes, tag);
    match value {
        Some(value) => {
            bytes.push(1);
            push_len_prefixed(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn push_len_prefixed(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}
