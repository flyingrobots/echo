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

/// Obstruction kind recorded when optic invocation admission refuses before a
/// success ticket, witness, scheduler selection, or execution boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationObstructionKind {
    /// Echo did not issue the supplied artifact handle.
    UnknownHandle,
    /// Invocation operation id did not match the registered artifact operation.
    OperationMismatch,
    /// Invocation supplied no basis request bytes.
    MissingBasisRequest,
    /// Invocation supplied no aperture request bytes.
    MissingApertureRequest,
    /// Invocation reached the basis boundary before Echo had a resolver that
    /// can bind the request to a causal basis.
    UnsupportedBasisResolution,
    /// Invocation reached the aperture boundary after basis resolution, but
    /// Echo had no aperture resolver that could bind the request to a graph
    /// region.
    UnsupportedApertureResolution,
    /// Invocation supplied no capability presentation.
    MissingCapability,
    /// Invocation supplied a malformed capability presentation.
    MalformedCapabilityPresentation,
    /// Invocation supplied a presentation not bound to a grant id.
    UnboundCapabilityPresentation,
    /// Invocation supplied a placeholder presentation before grant validation
    /// is wired into invocation admission.
    CapabilityValidationUnavailable,
}

/// Obstruction kind recorded when capability grant validation fails before any
/// successful admission ticket, law witness, scheduler selection, or execution
/// boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityGrantValidationObstructionKind {
    /// Presentation supplied unusable shape for grant validation.
    MalformedCapabilityPresentation,
    /// Presentation did not bind to a grant id.
    UnboundCapabilityPresentation,
    /// Presentation named grant material Echo has not recorded.
    UnknownGrant,
    /// Grant artifact hash did not cover the registered artifact.
    ArtifactHashMismatch,
    /// Grant operation id did not cover the registered artifact operation.
    OperationIdMismatch,
    /// Grant requirements digest did not cover the registered artifact
    /// requirements.
    RequirementsDigestMismatch,
    /// Grant expiry posture obstructed validation.
    ExpiredGrant,
}

impl InvocationObstructionKind {
    fn digest_label(self) -> &'static [u8] {
        match self {
            Self::UnknownHandle => b"unknown-handle",
            Self::OperationMismatch => b"operation-mismatch",
            Self::MissingBasisRequest => b"missing-basis-request",
            Self::MissingApertureRequest => b"missing-aperture-request",
            Self::UnsupportedBasisResolution => b"unsupported-basis-resolution",
            Self::UnsupportedApertureResolution => b"unsupported-aperture-resolution",
            Self::MissingCapability => b"missing-capability",
            Self::MalformedCapabilityPresentation => b"malformed-capability-presentation",
            Self::UnboundCapabilityPresentation => b"unbound-capability-presentation",
            Self::CapabilityValidationUnavailable => b"capability-validation-unavailable",
        }
    }
}

impl CapabilityGrantValidationObstructionKind {
    fn digest_label(self) -> &'static [u8] {
        match self {
            Self::MalformedCapabilityPresentation => b"malformed-capability-presentation",
            Self::UnboundCapabilityPresentation => b"unbound-capability-presentation",
            Self::UnknownGrant => b"unknown-grant",
            Self::ArtifactHashMismatch => b"artifact-hash-mismatch",
            Self::OperationIdMismatch => b"operation-id-mismatch",
            Self::RequirementsDigestMismatch => b"requirements-digest-mismatch",
            Self::ExpiredGrant => b"expired-grant",
        }
    }
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
    /// Echo refused optic invocation before admission success.
    OpticInvocationObstructed {
        /// Echo-owned runtime-local artifact handle id named by the invocation.
        artifact_handle_id: String,
        /// Operation id named by the invocation.
        operation_id: String,
        /// Digest of the invocation's canonical variable bytes.
        canonical_variables_digest: Vec<u8>,
        /// Digest of the opaque basis request bytes.
        basis_request_digest: [u8; 32],
        /// Digest of the opaque aperture request bytes.
        aperture_request_digest: [u8; 32],
        /// Structured invocation obstruction kind.
        obstruction: InvocationObstructionKind,
    },
    /// Echo refused capability grant validation before treating grant material
    /// as authority.
    CapabilityGrantValidationObstructed {
        /// Presentation identity supplied by the caller.
        presentation_id: String,
        /// Grant id named by the presentation, when structurally available.
        grant_id: Option<String>,
        /// Echo-owned runtime-local artifact handle id being covered.
        artifact_handle_id: String,
        /// Registered artifact hash Echo expected the grant to cover.
        expected_artifact_hash: String,
        /// Artifact hash named by the grant material, when available.
        grant_artifact_hash: Option<String>,
        /// Registered operation id Echo expected the grant to cover.
        expected_operation_id: String,
        /// Operation id named by the grant material, when available.
        grant_operation_id: Option<String>,
        /// Registered requirements digest Echo expected the grant to cover.
        expected_requirements_digest: String,
        /// Requirements digest named by the grant material, when available.
        grant_requirements_digest: Option<String>,
        /// Structured capability grant validation obstruction kind.
        obstruction: CapabilityGrantValidationObstructionKind,
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
            Self::OpticInvocationObstructed {
                artifact_handle_id,
                operation_id,
                canonical_variables_digest,
                basis_request_digest,
                aperture_request_digest,
                obstruction,
            } => {
                push_digest_field(&mut bytes, b"variant", b"optic-invocation-obstructed");
                push_digest_field(
                    &mut bytes,
                    b"artifact-handle-id",
                    artifact_handle_id.as_bytes(),
                );
                push_digest_field(&mut bytes, b"operation-id", operation_id.as_bytes());
                push_digest_field(
                    &mut bytes,
                    b"canonical-variables-digest",
                    canonical_variables_digest,
                );
                push_digest_field(&mut bytes, b"basis-request-digest", basis_request_digest);
                push_digest_field(
                    &mut bytes,
                    b"aperture-request-digest",
                    aperture_request_digest,
                );
                push_digest_field(&mut bytes, b"obstruction", obstruction.digest_label());
            }
            Self::CapabilityGrantValidationObstructed {
                presentation_id,
                grant_id,
                artifact_handle_id,
                expected_artifact_hash,
                grant_artifact_hash,
                expected_operation_id,
                grant_operation_id,
                expected_requirements_digest,
                grant_requirements_digest,
                obstruction,
            } => {
                push_digest_field(
                    &mut bytes,
                    b"variant",
                    b"capability-grant-validation-obstructed",
                );
                push_digest_field(&mut bytes, b"presentation-id", presentation_id.as_bytes());
                push_optional_digest_field(
                    &mut bytes,
                    b"grant-id",
                    grant_id.as_deref().map(str::as_bytes),
                );
                push_digest_field(
                    &mut bytes,
                    b"artifact-handle-id",
                    artifact_handle_id.as_bytes(),
                );
                push_digest_field(
                    &mut bytes,
                    b"expected-artifact-hash",
                    expected_artifact_hash.as_bytes(),
                );
                push_optional_digest_field(
                    &mut bytes,
                    b"grant-artifact-hash",
                    grant_artifact_hash.as_deref().map(str::as_bytes),
                );
                push_digest_field(
                    &mut bytes,
                    b"expected-operation-id",
                    expected_operation_id.as_bytes(),
                );
                push_optional_digest_field(
                    &mut bytes,
                    b"grant-operation-id",
                    grant_operation_id.as_deref().map(str::as_bytes),
                );
                push_digest_field(
                    &mut bytes,
                    b"expected-requirements-digest",
                    expected_requirements_digest.as_bytes(),
                );
                push_optional_digest_field(
                    &mut bytes,
                    b"grant-requirements-digest",
                    grant_requirements_digest.as_deref().map(str::as_bytes),
                );
                push_digest_field(&mut bytes, b"obstruction", obstruction.digest_label());
            }
        }

        FactDigest(*blake3::hash(&bytes).as_bytes())
    }
}

/// Computes a deterministic digest for opaque invocation request bytes named
/// inside graph facts.
#[must_use]
pub fn digest_invocation_request_bytes(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut input = Vec::new();
    push_digest_field(&mut input, b"domain", domain);
    push_digest_field(&mut input, b"bytes", bytes);
    *blake3::hash(&input).as_bytes()
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
