// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure Echo lowering for Edict's frozen target-provider component boundary.
//!
//! This crate translates only explicit, digest-bound canonical artifacts. It
//! performs no discovery or I/O and grants no Echo runtime authority.

#![deny(unsafe_code)]

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, digest_canonical_value_v1,
    encode_canonical_cbor_v1, CanonicalValueV1,
};

#[cfg(target_arch = "wasm32")]
mod component;

const PROVIDER_ABI: ProtocolVersionV1 = ProtocolVersionV1 {
    major: 1,
    minor: 0,
    patch: 0,
};
const CORE_DOMAIN: &str = "edict.core.module/v1";
const CORE_ABI: &str = "edict.core/v1";
const CORE_COORDINATE: &str = "a.b@1";
const TARGET_PROFILE_DOMAIN: &str = "edict.target-profile/v1";
const TARGET_PROFILE_COORDINATE: &str = "echo.dpo@1";
const LAWPACK_DOMAIN: &str = "edict.lawpack/v1";
const AUTHORITY_DOMAIN: &str = "edict.authority-facts/v1";
const LOWERABILITY_DOMAIN: &str = "edict.lowering-requirements/v1";
const LOWERABILITY_COORDINATE: &str = "echo.dpo-lowerability@1";
const TARGET_IR_ARTIFACT_DOMAIN: &str = "edict.target-ir.artifact/v1";
const TARGET_IR_COORDINATE: &str = "echo.span-ir/v1";
const GENERATED_ARTIFACT_DOMAIN: &str = "echo.generated-artifact/v1";
const REVIEW_PAYLOAD_DOMAIN: &str = "echo.review-payload/v1";
const GENERATED_ARTIFACT_ROLE: &str = "generated.echo-dpo";
const REVIEW_PAYLOAD_ROLE: &str = "review.echo-dpo";
const TARGET_IR_ROLE: &str = "target-ir.echo-dpo";
const GENERATED_ARTIFACT_PROFILE: &str = "echo.dpo.registration/v1";
const GENERATED_ARTIFACT_PROFILE_DIGEST: &str =
    "sha256:ff88be93c26cc533948d8a93601954dc391912d593ca1e96115c846cbf2c5b5d";
const TARGET_BUNDLE_PROFILE: &str = "echo.dpo.bundle/v1";
const TARGET_BUNDLE_PROFILE_DIGEST: &str =
    "sha256:aa0438bcc6ef14ee6cb6d4976622f6080381d731459dcb7b9102595c9bed92c0";
const SEMANTIC_BUNDLE_DIGEST_DOMAIN: &str = "edict.bundle.semantic/v1";
const RELEASE_BUNDLE_DIGEST_DOMAIN: &str = "edict.bundle.release/v1";
const GENERATED_SOURCE_MEDIA_TYPE: &str = "text/rust; charset=utf-8";
const REVIEW_MEDIA_TYPE: &str = "application/json";
const GENERATED_SOURCE_PATH: &str = "generated/echo_dpo.rs";
const REVIEW_PATH: &str = "review/echo_dpo.json";
const OPERATION_COORDINATE: &str = "a.b@1.t";
const OPERATION_INPUT_TYPE: &str = "a.b@1.Input";
const OPERATION_OUTPUT_TYPE: &str = "a.b@1.Output";
const OPERATION_RECEIPT_TYPE: &str = "a.b@1.Receipt";
const OPERATION_PROFILE: &str = "continuum.profile.write/v1";
const SEMANTIC_EFFECT: &str = "target.replace";
const TARGET_INTRINSIC: &str = "echo.dpo@1.replace";
const FAILURE_COORDINATE: &str = "rejected";
const FAILURE_PAYLOAD_TYPE: &str = "target.replace.rejected";
const DOMAIN_OBSTRUCTION: &str = "domain.WriteRejected";

const TARGET_PROFILE_BYTES: &[u8] = include_bytes!("../resources/target-profile.echo-dpo.cbor");
const LAWPACK_BYTES: &[u8] = include_bytes!("../resources/lawpack.echo-dpo.cbor");
const TARGET_AUTHORITY_BYTES: &[u8] = include_bytes!("../resources/authority-facts.echo-dpo.cbor");
const LAWPACK_AUTHORITY_BYTES: &[u8] =
    include_bytes!("../resources/authority-facts.echo-lawpack.cbor");

/// Semantic version carried by every invocation of the frozen provider ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolVersionV1 {
    /// Major protocol version.
    pub major: u32,
    /// Minor protocol version.
    pub minor: u32,
    /// Patch protocol version.
    pub patch: u32,
}

/// Digest algorithms admitted by the provider transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestAlgorithm {
    /// SHA-256.
    Sha256,
}

/// Typed digest bytes carried by a resource reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Digest {
    /// Digest algorithm.
    pub algorithm: DigestAlgorithm,
    /// Raw digest bytes.
    pub bytes: Vec<u8>,
}

/// Digest-bound semantic resource coordinate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceRef {
    /// Stable semantic coordinate.
    pub coordinate: String,
    /// Host-verified digest.
    pub digest: Digest,
}

/// Opaque canonical artifact transported with an explicit owning domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact {
    /// Owning artifact domain.
    pub domain: String,
    /// Exact canonical bytes.
    pub bytes: Vec<u8>,
}

/// Artifact bound to its semantic coordinate and digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundArtifact {
    /// Digest-bound resource reference.
    pub reference: ResourceRef,
    /// Exact artifact domain and bytes.
    pub artifact: Artifact,
}

/// Structural role of one semantic-closure input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticInputKind {
    /// Lawpack semantics.
    Lawpack,
    /// Source-partitioned authority facts.
    AuthorityFacts,
    /// Explicit lowerability requirements and facts.
    LowerabilityFacts,
    /// A separately constrained auxiliary semantic input.
    Auxiliary(String),
}

/// One role-constrained semantic input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticInput {
    /// Unique invocation role.
    pub role: String,
    /// Structural semantic kind.
    pub kind: SemanticInputKind,
    /// Digest-bound artifact.
    pub artifact: BoundArtifact,
}

/// Output kinds that a lowerer can structurally claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoweringOutputKind {
    /// Target-owned intermediate representation.
    TargetIr,
    /// Generated application artifact.
    GeneratedArtifact,
    /// Non-authoritative review payload.
    ReviewPayload,
}

/// One requested output role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringOutputRequest {
    /// Unique output role.
    pub role: String,
    /// Structurally permitted output kind.
    pub kind: LoweringOutputKind,
    /// Required owning domain.
    pub domain: String,
}

/// One provider-authored output without a provider-authored digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringOutputArtifact {
    /// Requested output role.
    pub role: String,
    /// Requested output kind.
    pub kind: LoweringOutputKind,
    /// Canonical output artifact.
    pub artifact: Artifact,
    /// Optional logical package-relative path.
    pub logical_path: Option<String>,
}

/// Host-enforced response bounds carried through the WIT request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResponseLimitsV1 {
    /// Maximum number of successful outputs.
    pub max_output_count: u32,
    /// Maximum number of diagnostics.
    pub max_diagnostic_count: u32,
    /// Maximum provider-authored response bytes.
    pub max_total_response_bytes: u64,
}

/// Diagnostic severity declared by the provider ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    /// Error diagnostic.
    Error,
    /// Warning diagnostic.
    Warning,
    /// Informational diagnostic.
    Info,
}

/// Stable bounded diagnostic attached to success or refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Stable machine-readable code.
    pub code: String,
    /// Severity.
    pub severity: DiagnosticSeverity,
    /// Deterministic human-readable explanation.
    pub message: String,
    /// Optional deterministic repair guidance.
    pub repair: Option<String>,
}

/// Target-owned refusal categories frozen by `edict:target-provider@1.0.0`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderRefusalKind {
    /// The request or Core ABI is unsupported.
    UnsupportedCoreAbi,
    /// The selected target profile is unsupported.
    UnsupportedTargetProfile,
    /// The supplied semantics cannot be represented faithfully.
    UnsupportedSemantics,
    /// The requested output role, kind, or domain is unsupported.
    UnsupportedOutputRole,
    /// A semantic artifact is malformed, noncanonical, or incorrectly bound.
    InvalidSemanticArtifact,
}

/// Typed target-owned refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRefusalV1 {
    /// Stable refusal kind.
    pub kind: ProviderRefusalKind,
    /// Optional stable subject of the refusal.
    pub subject: Option<String>,
    /// Deterministically ordered diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Explicit pure lowering request mirroring the frozen WIT record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringRequestV1 {
    /// Requested provider protocol version.
    pub protocol_version: ProtocolVersionV1,
    /// Exact canonical Edict Core artifact.
    pub core: BoundArtifact,
    /// Exact canonical target-profile artifact.
    pub target_profile: BoundArtifact,
    /// Complete explicit semantic closure.
    pub semantic_inputs: Vec<SemanticInput>,
    /// Exact requested output roles.
    pub requested_outputs: Vec<LoweringOutputRequest>,
    /// Host-owned response limits, which cannot alter canonical provider output.
    pub limits: ResponseLimitsV1,
}

/// Successful pure lowering response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringSuccessV1 {
    /// Exactly the requested and supported outputs.
    pub outputs: Vec<LoweringOutputArtifact>,
    /// Deterministically ordered diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Result returned by the first provider lowerer.
pub type LoweringResultV1 = Result<LoweringSuccessV1, ProviderRefusalV1>;

/// Lowers the explicit canonical Echo provider closure without discovery or I/O.
///
/// The function may embed deterministic, non-authoritative byte identities in
/// generated and review projections. The Edict host remains responsible for
/// validating every returned artifact against its owning schema, recomputing
/// host-authored output identities, and admitting any later bundle occurrence.
/// No output grants Echo runtime authority.
///
/// # Errors
///
/// Returns a typed provider refusal when the request crosses an unsupported ABI,
/// profile, semantic, output, or artifact boundary.
pub fn lower(request: LoweringRequestV1) -> LoweringResultV1 {
    if request.protocol_version != PROVIDER_ABI {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedCoreAbi,
            format!(
                "edict:target-provider@{}.{}.{}",
                request.protocol_version.major,
                request.protocol_version.minor,
                request.protocol_version.patch
            ),
            "echo.provider.unsupported-protocol",
            "the provider accepts only edict:target-provider@1.0.0",
        ));
    }

    validate_target_profile(&request.target_profile)?;
    validate_semantic_closure(&request.semantic_inputs)?;
    validate_requested_outputs(&request.requested_outputs)?;
    let target_ir = lower_core(&request.core, &request.target_profile.reference.digest)?;

    let outputs = build_requested_outputs(
        &request.requested_outputs,
        &target_ir,
        &request.target_profile.reference.digest,
    )?;

    // Response limits are deliberately host-owned. Canonical provider output is
    // invariant under limit changes and the host decides whether it fits.
    let _ = request.limits;
    Ok(LoweringSuccessV1 {
        outputs,
        diagnostics: Vec::new(),
    })
}

fn validate_target_profile(profile: &BoundArtifact) -> Result<(), ProviderRefusalV1> {
    validate_binding(profile).map_err(|()| {
        refusal(
            ProviderRefusalKind::InvalidSemanticArtifact,
            "target-profile.echo-dpo",
            "echo.provider.invalid-target-profile-artifact",
            "the target-profile artifact is not canonically digest-bound",
        )
    })?;
    if profile.reference.coordinate != TARGET_PROFILE_COORDINATE
        || profile.artifact.domain != TARGET_PROFILE_DOMAIN
        || profile.artifact.bytes != TARGET_PROFILE_BYTES
    {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedTargetProfile,
            &profile.reference.coordinate,
            "echo.provider.unsupported-target-profile",
            "the lowerer accepts only the exact checked Echo DPO target profile",
        ));
    }
    Ok(())
}

fn validate_semantic_closure(inputs: &[SemanticInput]) -> Result<(), ProviderRefusalV1> {
    const EXPECTED: [(&str, SemanticInputKind, &str, &str, &[u8]); 3] = [
        (
            "authority-facts.echo-dpo",
            SemanticInputKind::AuthorityFacts,
            "echo.dpo-authority-facts@1",
            AUTHORITY_DOMAIN,
            TARGET_AUTHORITY_BYTES,
        ),
        (
            "authority-facts.echo-lawpack",
            SemanticInputKind::AuthorityFacts,
            "echo.dpo-lawpack-authority-facts@1",
            AUTHORITY_DOMAIN,
            LAWPACK_AUTHORITY_BYTES,
        ),
        (
            "lawpack.echo-dpo",
            SemanticInputKind::Lawpack,
            "echo.dpo-lawpack@1",
            LAWPACK_DOMAIN,
            LAWPACK_BYTES,
        ),
    ];
    if inputs.len() != 4 {
        return Err(unsupported_semantics("semantic-inputs"));
    }

    for ((role, kind, coordinate, domain, bytes), input) in EXPECTED.into_iter().zip(inputs.iter())
    {
        if input.role != role || input.kind != kind {
            return Err(unsupported_semantics(&input.role));
        }
        validate_binding(&input.artifact)
            .map_err(|()| invalid_artifact(&input.role, "artifact binding is invalid"))?;
        if input.artifact.reference.coordinate != coordinate
            || input.artifact.artifact.domain != domain
            || input.artifact.artifact.bytes != bytes
        {
            return Err(invalid_artifact(
                &input.role,
                "artifact does not equal the checked provider closure",
            ));
        }
    }

    let lowerability = &inputs[3];
    if lowerability.role != "lowerability.echo-dpo"
        || lowerability.kind != SemanticInputKind::LowerabilityFacts
        || lowerability.artifact.artifact.domain != LOWERABILITY_DOMAIN
    {
        return Err(unsupported_semantics(&lowerability.role));
    }
    let value = validate_binding(&lowerability.artifact)
        .map_err(|()| invalid_artifact(&lowerability.role, "artifact binding is invalid"))?;
    if lowerability.artifact.reference.coordinate != LOWERABILITY_COORDINATE {
        return Err(invalid_artifact(
            &lowerability.role,
            "artifact does not equal the checked provider closure",
        ));
    }
    validate_lowerability(&value).map_err(|()| unsupported_semantics(&lowerability.role))
}

fn validate_lowerability(value: &CanonicalValueV1) -> Result<(), ()> {
    if value != &expected_lowerability()? {
        return Err(());
    }
    Ok(())
}

fn expected_lowerability() -> Result<CanonicalValueV1, ()> {
    let guard_kinds = || CanonicalValueV1::Array(vec![canonical_text("precommit-atomic")]);
    let obstructions = || CanonicalValueV1::Array(vec![canonical_text(FAILURE_COORDINATE)]);
    let footprint_obligations =
        || CanonicalValueV1::Array(vec![canonical_text("target.replace.footprint")]);
    let cost_obligations = || CanonicalValueV1::Array(vec![canonical_text("target.replace.cost")]);
    canonical_sorted_map([
        ("apiVersion", canonical_text(LOWERABILITY_DOMAIN)),
        ("operationProfile", canonical_text(OPERATION_PROFILE)),
        (
            "semanticEffects",
            CanonicalValueV1::Array(vec![canonical_sorted_map([
                ("coordinate", canonical_text(SEMANTIC_EFFECT)),
                ("writeClass", canonical_text("replace")),
                ("guardKinds", guard_kinds()),
                ("obstructionCoordinates", obstructions()),
                ("footprintObligations", footprint_obligations()),
                ("costObligations", cost_obligations()),
            ])?]),
        ),
        (
            "requiredWriteClasses",
            CanonicalValueV1::Array(vec![canonical_text("replace")]),
        ),
        ("guardKinds", guard_kinds()),
        ("atomicity", canonical_text("atomic")),
        ("postconditionSupport", CanonicalValueV1::Bool(true)),
        ("obstructionCoordinates", obstructions()),
        ("footprintObligations", footprint_obligations()),
        ("costObligations", cost_obligations()),
        ("opticContract", canonical_text("replace-point")),
    ])
}

fn expected_core_types() -> Result<CanonicalValueV1, ()> {
    canonical_sorted_map([
        ("Input", expected_record_type("a.b@1.Input.id")?),
        ("Output", expected_record_type("a.b@1.Output.id")?),
        ("Receipt", expected_record_type("a.b@1.Receipt.id")?),
        ("Input.id", expected_string_type()?),
        ("Output.id", expected_string_type()?),
        ("Receipt.id", expected_string_type()?),
    ])
}

fn expected_record_type(field_type: &str) -> Result<CanonicalValueV1, ()> {
    canonical_sorted_map([
        ("kind", canonical_text("Record")),
        (
            "fields",
            canonical_sorted_map([("id", canonical_text(field_type))])?,
        ),
    ])
}

fn expected_string_type() -> Result<CanonicalValueV1, ()> {
    canonical_sorted_map([
        ("kind", canonical_text("String")),
        ("max", CanonicalValueV1::Integer(16)),
        ("canonical", canonical_text("raw-utf8")),
    ])
}

fn expected_core_evaluation_budget() -> Result<CanonicalValueV1, ()> {
    canonical_sorted_map([
        ("maxSteps", CanonicalValueV1::Integer(8)),
        ("maxAllocatedBytes", CanonicalValueV1::Integer(1024)),
        ("maxOutputBytes", CanonicalValueV1::Integer(256)),
    ])
}

fn canonical_sorted_map<'a>(
    entries: impl IntoIterator<Item = (&'a str, CanonicalValueV1)>,
) -> Result<CanonicalValueV1, ()> {
    let mut entries = entries
        .into_iter()
        .map(|(key, value)| {
            let key = canonical_text(key);
            let encoded_key = encode_canonical_cbor_v1(&key).map_err(|_| ())?;
            Ok((encoded_key, key, value))
        })
        .collect::<Result<Vec<_>, ()>>()?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(CanonicalValueV1::Map(
        entries
            .into_iter()
            .map(|(_, key, value)| (key, value))
            .collect(),
    ))
}

fn validate_requested_outputs(requests: &[LoweringOutputRequest]) -> Result<(), ProviderRefusalV1> {
    let mut previous_role: Option<&str> = None;
    for request in requests {
        if previous_role.is_some_and(|previous| previous.as_bytes() >= request.role.as_bytes())
            || !is_declared_output(request)
        {
            return Err(refusal(
                ProviderRefusalKind::UnsupportedOutputRole,
                &request.role,
                "echo.provider.unsupported-output-role",
                "the lowerer serves only the exact sorted declared output roles",
            ));
        }
        previous_role = Some(&request.role);
    }
    Ok(())
}

fn is_declared_output(request: &LoweringOutputRequest) -> bool {
    matches!(
        (request.role.as_str(), request.kind, request.domain.as_str()),
        (
            GENERATED_ARTIFACT_ROLE,
            LoweringOutputKind::GeneratedArtifact,
            GENERATED_ARTIFACT_DOMAIN
        ) | (
            REVIEW_PAYLOAD_ROLE,
            LoweringOutputKind::ReviewPayload,
            REVIEW_PAYLOAD_DOMAIN
        ) | (
            TARGET_IR_ROLE,
            LoweringOutputKind::TargetIr,
            TARGET_IR_ARTIFACT_DOMAIN
        )
    )
}

fn build_requested_outputs(
    requests: &[LoweringOutputRequest],
    target_ir: &CanonicalValueV1,
    target_profile_digest: &Digest,
) -> Result<Vec<LoweringOutputArtifact>, ProviderRefusalV1> {
    if requests.is_empty() {
        return Ok(Vec::new());
    }

    let target_ir_bytes = encode_canonical_cbor_v1(target_ir).map_err(|_| {
        invalid_output_artifact(
            TARGET_IR_ROLE,
            "echo.provider.target-ir-encoding",
            "the lowered Target IR could not be canonically encoded",
        )
    })?;
    let target_ir_digest_bytes =
        digest_canonical_value_bytes_v1(TARGET_IR_ARTIFACT_DOMAIN, target_ir).map_err(|_| {
            invalid_output_artifact(
                TARGET_IR_ROLE,
                "echo.provider.target-ir-digest",
                "the lowered Target IR identity could not be computed",
            )
        })?;
    let target_ir_digest = Digest {
        algorithm: DigestAlgorithm::Sha256,
        bytes: target_ir_digest_bytes.to_vec(),
    };
    let generated = requests
        .iter()
        .any(|request| request.kind != LoweringOutputKind::TargetIr)
        .then(|| render_generated_artifact(&target_ir_digest, target_profile_digest))
        .transpose()?;

    requests
        .iter()
        .map(|request| match request.kind {
            LoweringOutputKind::TargetIr => Ok(LoweringOutputArtifact {
                role: request.role.clone(),
                kind: request.kind,
                artifact: Artifact {
                    domain: request.domain.clone(),
                    bytes: target_ir_bytes.clone(),
                },
                logical_path: None,
            }),
            LoweringOutputKind::GeneratedArtifact => {
                let generated = generated.as_ref().ok_or_else(|| {
                    invalid_output_artifact(
                        GENERATED_ARTIFACT_ROLE,
                        "echo.provider.generated-artifact-absent",
                        "the generated artifact was not rendered",
                    )
                })?;
                Ok(LoweringOutputArtifact {
                    role: request.role.clone(),
                    kind: request.kind,
                    artifact: Artifact {
                        domain: request.domain.clone(),
                        bytes: generated.bytes.clone(),
                    },
                    logical_path: Some(GENERATED_SOURCE_PATH.to_owned()),
                })
            }
            LoweringOutputKind::ReviewPayload => {
                let generated = generated.as_ref().ok_or_else(|| {
                    invalid_output_artifact(
                        REVIEW_PAYLOAD_ROLE,
                        "echo.provider.review-subject-absent",
                        "the generated artifact review subject was not rendered",
                    )
                })?;
                let review = render_review_payload(
                    &target_ir_digest,
                    target_profile_digest,
                    &generated.digest,
                );
                let envelope = review_payload_envelope(&generated.digest, review.into_bytes())?;
                Ok(LoweringOutputArtifact {
                    role: request.role.clone(),
                    kind: request.kind,
                    artifact: Artifact {
                        domain: request.domain.clone(),
                        bytes: encode_output_envelope(request.role.as_str(), &envelope)?,
                    },
                    logical_path: Some(REVIEW_PATH.to_owned()),
                })
            }
        })
        .collect()
}

struct RenderedGeneratedArtifact {
    bytes: Vec<u8>,
    digest: Digest,
}

fn render_generated_artifact(
    target_ir_digest: &Digest,
    target_profile_digest: &Digest,
) -> Result<RenderedGeneratedArtifact, ProviderRefusalV1> {
    let source = render_generated_source(target_ir_digest, target_profile_digest);
    let envelope = generated_artifact_envelope(source.into_bytes())?;
    let bytes = encode_output_envelope(GENERATED_ARTIFACT_ROLE, &envelope)?;
    let digest =
        digest_canonical_value_bytes_v1(GENERATED_ARTIFACT_DOMAIN, &envelope).map_err(|_| {
            invalid_output_artifact(
                GENERATED_ARTIFACT_ROLE,
                "echo.provider.generated-artifact-digest",
                "the generated-artifact identity could not be computed",
            )
        })?;
    Ok(RenderedGeneratedArtifact {
        bytes,
        digest: Digest {
            algorithm: DigestAlgorithm::Sha256,
            bytes: digest.to_vec(),
        },
    })
}

fn generated_artifact_envelope(
    source_bytes: Vec<u8>,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    let profile_digest =
        decode_sha256_review(GENERATED_ARTIFACT_PROFILE_DIGEST).ok_or_else(|| {
            invalid_output_artifact(
                GENERATED_ARTIFACT_ROLE,
                "echo.provider.generated-profile-digest",
                "the checked generated-artifact profile digest is invalid",
            )
        })?;
    canonical_sorted_map([
        ("apiVersion", canonical_text(GENERATED_ARTIFACT_DOMAIN)),
        (
            "profile",
            resource_ref_value(GENERATED_ARTIFACT_PROFILE, profile_digest)?,
        ),
        ("operation", canonical_text(OPERATION_COORDINATE)),
        ("mediaType", canonical_text(GENERATED_SOURCE_MEDIA_TYPE)),
        ("bytes", CanonicalValueV1::Bytes(source_bytes)),
    ])
    .map_err(|()| {
        invalid_output_artifact(
            GENERATED_ARTIFACT_ROLE,
            "echo.provider.generated-envelope",
            "the generated-artifact envelope could not be constructed",
        )
    })
}

fn review_payload_envelope(
    generated_artifact_digest: &Digest,
    review_bytes: Vec<u8>,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    canonical_sorted_map([
        ("apiVersion", canonical_text(REVIEW_PAYLOAD_DOMAIN)),
        ("authoritative", CanonicalValueV1::Bool(false)),
        (
            "subject",
            resource_ref_value(
                GENERATED_ARTIFACT_ROLE,
                generated_artifact_digest.bytes.clone(),
            )?,
        ),
        ("mediaType", canonical_text(REVIEW_MEDIA_TYPE)),
        ("bytes", CanonicalValueV1::Bytes(review_bytes)),
    ])
    .map_err(|()| {
        invalid_output_artifact(
            REVIEW_PAYLOAD_ROLE,
            "echo.provider.review-envelope",
            "the review-payload envelope could not be constructed",
        )
    })
}

fn resource_ref_value(
    coordinate: &str,
    digest_bytes: Vec<u8>,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    canonical_sorted_map([
        ("id", canonical_text(coordinate)),
        (
            "digest",
            CanonicalValueV1::Array(vec![
                canonical_text("sha256"),
                CanonicalValueV1::Bytes(digest_bytes),
            ]),
        ),
    ])
    .map_err(|()| {
        invalid_output_artifact(
            coordinate,
            "echo.provider.resource-reference",
            "the output resource reference could not be constructed",
        )
    })
}

fn encode_output_envelope(
    role: &str,
    envelope: &CanonicalValueV1,
) -> Result<Vec<u8>, ProviderRefusalV1> {
    encode_canonical_cbor_v1(envelope).map_err(|_| {
        invalid_output_artifact(
            role,
            "echo.provider.output-envelope-encoding",
            "the output envelope could not be canonically encoded",
        )
    })
}

fn render_generated_source(target_ir_digest: &Digest, target_profile_digest: &Digest) -> String {
    const TEMPLATE: &str = r#"// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generated Echo helper projection for one admitted Edict operation.
//! Final Edict contract-bundle identity is bound explicitly after assembly.

/// Namespaced generated contract and invocation surface for `a.b@1.t`.
pub mod echo_dpo {
    use echo_registry_api::{
        OpKind, ProviderBundleIdentityV1, ProviderDigestIdentityV1,
        ProviderFootprintIdentityV1, ProviderOperationV1, ProviderRegistryV1,
        ProviderSchemaIdentityV1, ProviderSemanticIdentityV1, ProviderValueContractV1,
    };
    use echo_wasm_abi::codec::{
        decode_from_bytes, encode_to_vec, CodecError, Decode, Encode, Reader, Writer,
    };
    use echo_wasm_abi::{pack_intent_v1, EnvelopeError};
    use warp_core::{
        matches_eint_op, propose_provider_contract_package_v1, ContractPackageIdentity,
        GeneratedProviderMutationDispatchV1, GraphView, NodeId,
        ProviderContractPackageProposalV1, ProviderMutationHooksV1,
        ProviderMutationImplementationIdentityV1, ProviderPackageProposalError,
    };

    /// Exact Edict-authored semantic operation coordinate.
    pub const OPERATION_COORDINATE: &str = "a.b@1.t";
    /// Semantic domain owning the operation coordinate.
    pub const OPERATION_DOMAIN: &str = "echo.edict-provider/operation/v1";
    /// Echo-owned law that derives the persisted operation id.
    pub const OPERATION_ID_LAW: &str = "echo.semantic-operation-id.fnv1-32/v1";
    /// Exact persisted operation id carried by the generated-artifact profile.
    pub const OPERATION_ID: u32 = 3_389_142_194;
    /// Exact Echo value codec carried by the generated-artifact profile.
    pub const VALUE_CODEC_ID: &str = "le-binary-v1";
    /// Exact input schema coordinate owned by the generated-artifact profile.
    pub const INPUT_SCHEMA: &str = "a.b@1.Input";
    /// Exact output schema coordinate owned by the generated-artifact profile.
    pub const OUTPUT_SCHEMA: &str = "a.b@1.Output";
    /// Shared semantic domain owning the exact operation type schemas.
    pub const TYPE_SCHEMA_DOMAIN: &str = "echo.edict-provider/value/v1";
    /// Exact typed obstruction coordinate for the reviewed failure mapping.
    pub const OBSTRUCTION_COORDINATE: &str = "domain.WriteRejected";
    /// Semantic domain owning the typed obstruction coordinate.
    pub const OBSTRUCTION_DOMAIN: &str = "echo.edict-provider/obstruction/v1";
    /// Exact target failure payload schema before obstruction mapping.
    pub const EFFECT_FAILURE_SCHEMA: &str = "target.replace.rejected";
    /// Exact domain obstruction payload schema after obstruction mapping.
    pub const OBSTRUCTION_PAYLOAD_SCHEMA: &str = "domain.WriteRejected.Payload";
    /// Semantic coordinate carried by the emitted Target IR artifact.
    pub const TARGET_IR_COORDINATE: &str = "echo.span-ir/v1";
    /// Digest-framing domain for the complete Target IR artifact envelope.
    pub const TARGET_IR_DIGEST_DOMAIN: &str = "edict.target-ir.artifact/v1";
    /// Exact domain-framed identity of the emitted Target IR artifact.
    pub const TARGET_IR_DIGEST: &str = "__TARGET_IR_DIGEST__";
    /// Exact target-profile coordinate.
    pub const TARGET_PROFILE_COORDINATE: &str = "echo.dpo@1";
    /// Digest-framing domain for the target-profile artifact.
    pub const TARGET_PROFILE_DIGEST_DOMAIN: &str = "edict.target-profile/v1";
    /// Exact domain-framed identity of the target profile.
    pub const TARGET_PROFILE_DIGEST: &str = "__TARGET_PROFILE_DIGEST__";
    /// Semantic profile for Echo contract bundles; not a bundle occurrence.
    pub const TARGET_BUNDLE_PROFILE_COORDINATE: &str = "echo.dpo.bundle/v1";
    /// Digest-framing domain for the target-bundle profile artifact.
    pub const TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN: &str = "echo.dpo.bundle/v1";
    /// Exact domain-framed identity of the target-bundle profile.
    pub const TARGET_BUNDLE_PROFILE_DIGEST: &str = "__TARGET_BUNDLE_PROFILE_DIGEST__";
    /// Echo contract ABI targeted by this generated helper.
    pub const ECHO_CONTRACT_ABI_VERSION: u32 = 1;
    /// Contract-host helper API targeted by this generated helper.
    pub const CONTRACT_HOST_HELPER_API_VERSION: u32 = 1;
    /// Exact self-contained provider CDDL coordinate.
    pub const PROVIDER_SCHEMA_COORDINATE: &str = "echo.provider-artifacts.cddl@1";
    /// Raw SHA-256 of the exact self-contained provider CDDL bytes.
    pub const PROVIDER_SCHEMA_SHA256_HEX: &str =
        "faece52eaf8ec040c374e5fe2a5ea040b522b58f415973f481e9c836ecfc4cde";
    /// Exact generated-artifact profile coordinate owning operation schemas.
    pub const GENERATED_ARTIFACT_PROFILE: &str = "echo.dpo.registration/v1";
    /// Digest-framing domain for the generated-artifact profile.
    pub const GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN: &str =
        "echo.generated-artifact-profile/v1";
    /// Domain-framed identity of the generated-artifact profile.
    pub const GENERATED_ARTIFACT_PROFILE_DIGEST: &str =
        "sha256:ff88be93c26cc533948d8a93601954dc391912d593ca1e96115c846cbf2c5b5d";
    /// Exact semantic operation profile selected by the authored operation.
    pub const OPERATION_PROFILE: &str = "continuum.profile.write/v1";
    /// Semantic domain owning the selected operation profile.
    pub const OPERATION_PROFILE_DOMAIN: &str =
        "echo.edict-provider/operation-profile/v1";
    /// Exact operation-profiles document coordinate.
    pub const OPERATION_PROFILES_COORDINATE: &str = "echo.dpo.operation-profiles/v1";
    /// Digest-framing domain for the operation-profiles document.
    pub const OPERATION_PROFILES_DIGEST_DOMAIN: &str = "echo.dpo.operation-profiles/v1";
    /// Domain-framed identity of the operation-profiles document.
    pub const OPERATION_PROFILES_DIGEST: &str =
        "sha256:53256c51f6c817a77cc8694458bf9d3891abd15b9c94f79ca97d920d3c5f0416";
    /// Abstract footprint obligation carried across lowering.
    pub const FOOTPRINT_OBLIGATION: &str = "target.replace.footprint";
    /// Exact target footprint-algebra coordinate.
    pub const FOOTPRINT_ALGEBRA: &str = "echo.dpo.footprint/v1";
    /// Digest-framing domain for the target footprint algebra.
    pub const FOOTPRINT_ALGEBRA_DIGEST_DOMAIN: &str = "echo.dpo.footprint/v1";
    /// Domain-framed identity of the target footprint algebra.
    pub const FOOTPRINT_ALGEBRA_DIGEST: &str =
        "sha256:f47bb65867e78099ddcfd6ae7af83870df8823f974a496a111ed94e5d785c769";
    /// Edict domain for the semantic contract-bundle digest proposition.
    pub const SEMANTIC_BUNDLE_DIGEST_DOMAIN: &str = "edict.bundle.semantic/v1";
    /// Edict domain for the release contract-bundle digest proposition.
    pub const RELEASE_BUNDLE_DIGEST_DOMAIN: &str = "edict.bundle.release/v1";

    const MUTATION_RULE_NAME: &str = concat!(
        "cmd/contract/",
        "faece52eaf8ec040c374e5fe2a5ea040b522b58f415973f481e9c836ecfc4cde",
        "/3389142194/a.b@1.t"
    );
    const PROVIDER_OPERATIONS: [ProviderOperationV1<'static>; 1] = [ProviderOperationV1 {
        coordinate: OPERATION_COORDINATE,
        semantic_domain: OPERATION_DOMAIN,
        kind: OpKind::Mutation,
        operation_id_law: OPERATION_ID_LAW,
        operation_id: OPERATION_ID,
        input: ProviderValueContractV1 {
            schema_coordinate: INPUT_SCHEMA,
            schema_domain: TYPE_SCHEMA_DOMAIN,
            codec_id: VALUE_CODEC_ID,
        },
        output: ProviderValueContractV1 {
            schema_coordinate: OUTPUT_SCHEMA,
            schema_domain: TYPE_SCHEMA_DOMAIN,
            codec_id: VALUE_CODEC_ID,
        },
        target_failure_schema: EFFECT_FAILURE_SCHEMA,
        obstruction: ProviderSemanticIdentityV1 {
            coordinate: OBSTRUCTION_COORDINATE,
            semantic_domain: OBSTRUCTION_DOMAIN,
        },
        obstruction_payload_schema: OBSTRUCTION_PAYLOAD_SCHEMA,
        target_ir: ProviderDigestIdentityV1 {
            coordinate: TARGET_IR_COORDINATE,
            digest_domain: TARGET_IR_DIGEST_DOMAIN,
            digest: TARGET_IR_DIGEST,
        },
        target_profile: ProviderDigestIdentityV1 {
            coordinate: TARGET_PROFILE_COORDINATE,
            digest_domain: TARGET_PROFILE_DIGEST_DOMAIN,
            digest: TARGET_PROFILE_DIGEST,
        },
        generated_artifact_profile: ProviderDigestIdentityV1 {
            coordinate: GENERATED_ARTIFACT_PROFILE,
            digest_domain: GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN,
            digest: GENERATED_ARTIFACT_PROFILE_DIGEST,
        },
        operation_profile: ProviderSemanticIdentityV1 {
            coordinate: OPERATION_PROFILE,
            semantic_domain: OPERATION_PROFILE_DOMAIN,
        },
        operation_profiles: ProviderDigestIdentityV1 {
            coordinate: OPERATION_PROFILES_COORDINATE,
            digest_domain: OPERATION_PROFILES_DIGEST_DOMAIN,
            digest: OPERATION_PROFILES_DIGEST,
        },
        footprint: ProviderFootprintIdentityV1 {
            obligation: FOOTPRINT_OBLIGATION,
            algebra_coordinate: FOOTPRINT_ALGEBRA,
            algebra_digest_domain: FOOTPRINT_ALGEBRA_DIGEST_DOMAIN,
            algebra_digest: FOOTPRINT_ALGEBRA_DIGEST,
        },
    }];

    const ID_MAX_SCALAR_VALUES: usize = 16;
    const ID_MAX_UTF8_BYTES: usize = ID_MAX_SCALAR_VALUES * 4;

    /// Exact bounded value of semantic type `a.b@1.Id`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Id(String);

    impl Id {
        /// Construct an id after enforcing the authored Unicode-scalar bound.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError::StringTooLong`] when `value` contains more
        /// than sixteen Unicode scalar values.
        pub fn new(value: impl Into<String>) -> Result<Self, CodecError> {
            let value = value.into();
            if value.chars().count() > ID_MAX_SCALAR_VALUES {
                return Err(CodecError::StringTooLong);
            }
            Ok(Self(value))
        }

        /// Borrow the exact raw UTF-8 value without normalization.
        pub fn as_str(&self) -> &str {
            &self.0
        }

        /// Consume this value and return its exact raw UTF-8 string.
        pub fn into_string(self) -> String {
            self.0
        }
    }

    impl Encode for Id {
        fn encode(&self, writer: &mut Writer) -> Result<(), CodecError> {
            if self.0.chars().count() > ID_MAX_SCALAR_VALUES {
                return Err(CodecError::StringTooLong);
            }
            writer.write_len_prefixed_bytes(self.0.as_bytes())
        }
    }

    impl Decode for Id {
        fn decode(reader: &mut Reader<'_>) -> Result<Self, CodecError> {
            let bytes = reader.read_len_prefixed_bytes(ID_MAX_UTF8_BYTES)?;
            let value = core::str::from_utf8(bytes).map_err(|_| CodecError::InvalidUtf8)?;
            Self::new(String::from(value))
        }
    }

    /// Exact typed input for semantic operation `a.b@1.t`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Input {
        id: Id,
    }

    impl Input {
        /// Construct a validated operation input.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError::StringTooLong`] when `id` exceeds its
        /// authored Unicode-scalar bound.
        pub fn new(id: impl Into<String>) -> Result<Self, CodecError> {
            Ok(Self { id: Id::new(id)? })
        }

        /// Borrow the exact raw UTF-8 id.
        pub fn id(&self) -> &str {
            self.id.as_str()
        }
    }

    impl Encode for Input {
        fn encode(&self, writer: &mut Writer) -> Result<(), CodecError> {
            self.id.encode(writer)
        }
    }

    impl Decode for Input {
        fn decode(reader: &mut Reader<'_>) -> Result<Self, CodecError> {
            Ok(Self {
                id: Id::decode(reader)?,
            })
        }
    }

    /// Exact typed output for semantic operation `a.b@1.t`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Output {
        id: Id,
    }

    impl Output {
        /// Construct a validated operation output.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError::StringTooLong`] when `id` exceeds its
        /// authored Unicode-scalar bound.
        pub fn new(id: impl Into<String>) -> Result<Self, CodecError> {
            Ok(Self { id: Id::new(id)? })
        }

        /// Borrow the exact raw UTF-8 id.
        pub fn id(&self) -> &str {
            self.id.as_str()
        }
    }

    impl Encode for Output {
        fn encode(&self, writer: &mut Writer) -> Result<(), CodecError> {
            self.id.encode(writer)
        }
    }

    impl Decode for Output {
        fn decode(reader: &mut Reader<'_>) -> Result<Self, CodecError> {
            Ok(Self {
                id: Id::decode(reader)?,
            })
        }
    }

    /// Stable failure produced while constructing one canonical invocation.
    #[derive(Debug, Eq, PartialEq)]
    pub enum GeneratedInvocationError {
        /// The typed input violates its generated codec contract.
        Codec(CodecError),
        /// The canonical Echo intent envelope could not be constructed.
        Envelope(EnvelopeError),
    }

    /// Independent host pin for the final assembled bundle identity.
    ///
    /// This value is explicit expected evidence, not an admission token.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ExpectedContractBundleIdentityV1<'a> {
        /// Digest proposition domain for `semantic_digest`.
        pub semantic_digest_domain: &'a str,
        /// Exact semantic-layer bundle digest expected by the host.
        pub semantic_digest: &'a str,
        /// Digest proposition domain for `release_digest`.
        pub release_digest_domain: &'a str,
        /// Exact release-layer bundle digest expected by the host.
        pub release_digest: &'a str,
    }

    /// Untrusted identity and semantic claims read from one assembled bundle.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ContractBundleIdentityV1<'a> {
        /// Digest proposition domain for `semantic_digest`.
        pub semantic_digest_domain: &'a str,
        /// Claimed semantic-layer bundle digest computed by Edict.
        pub semantic_digest: &'a str,
        /// Digest proposition domain for `release_digest`.
        pub release_digest_domain: &'a str,
        /// Claimed release-layer bundle digest computed by Edict.
        pub release_digest: &'a str,
        /// Semantic operation coordinate carried by the bundle.
        pub operation_coordinate: &'a str,
        /// Semantic domain owning the operation coordinate.
        pub operation_domain: &'a str,
        /// Operation-id law claimed by the generated-artifact profile.
        pub operation_id_law: &'a str,
        /// Persisted operation id claimed by the generated-artifact profile.
        pub operation_id: u32,
        /// Echo value codec claimed by the generated-artifact profile.
        pub value_codec: &'a str,
        /// Semantic coordinate carried by the Target IR artifact.
        pub target_ir_coordinate: &'a str,
        /// Digest-framing domain for the Target IR artifact.
        pub target_ir_digest_domain: &'a str,
        /// Target IR digest carried by the bundle.
        pub target_ir_digest: &'a str,
        /// Target-profile coordinate carried by the bundle.
        pub target_profile_coordinate: &'a str,
        /// Digest-framing domain for the target-profile artifact.
        pub target_profile_digest_domain: &'a str,
        /// Target-profile digest carried by the bundle.
        pub target_profile_digest: &'a str,
        /// Target-bundle-profile coordinate carried by the bundle.
        pub target_bundle_profile_coordinate: &'a str,
        /// Digest-framing domain for the target-bundle-profile artifact.
        pub target_bundle_profile_digest_domain: &'a str,
        /// Target-bundle-profile digest carried by the bundle.
        pub target_bundle_profile_digest: &'a str,
        /// Echo contract ABI claimed by the generated registry.
        pub echo_contract_abi_version: u32,
        /// Contract-host helper API claimed by the generated registry.
        pub helper_api_version: u32,
        /// Provider CDDL coordinate claimed by the assembled bundle.
        pub provider_schema_coordinate: &'a str,
        /// Raw provider CDDL SHA-256 claimed by the assembled bundle.
        pub provider_schema_sha256_hex: &'a str,
        /// Input schema coordinate claimed for this operation.
        pub input_schema: &'a str,
        /// Output schema coordinate claimed for this operation.
        pub output_schema: &'a str,
        /// Semantic domain owning all claimed operation type schemas.
        pub type_schema_domain: &'a str,
        /// Typed obstruction coordinate claimed for the reviewed failure.
        pub obstruction_coordinate: &'a str,
        /// Semantic domain owning the typed obstruction coordinate.
        pub obstruction_domain: &'a str,
        /// Target failure payload schema claimed before obstruction mapping.
        pub effect_failure_schema: &'a str,
        /// Domain obstruction payload schema claimed after obstruction mapping.
        pub obstruction_payload_schema: &'a str,
        /// Generated-artifact profile coordinate claimed by the bundle.
        pub generated_artifact_profile: &'a str,
        /// Digest-framing domain for the generated-artifact profile.
        pub generated_artifact_profile_digest_domain: &'a str,
        /// Generated-artifact profile digest claimed by the bundle.
        pub generated_artifact_profile_digest: &'a str,
        /// Semantic operation profile claimed by the bundle.
        pub operation_profile: &'a str,
        /// Semantic domain owning the operation profile.
        pub operation_profile_domain: &'a str,
        /// Operation-profiles document coordinate claimed by the bundle.
        pub operation_profiles_coordinate: &'a str,
        /// Digest-framing domain for the operation-profiles document.
        pub operation_profiles_digest_domain: &'a str,
        /// Operation-profiles document digest claimed by the bundle.
        pub operation_profiles_digest: &'a str,
        /// Abstract footprint obligation claimed for this operation.
        pub footprint_obligation: &'a str,
        /// Footprint-algebra coordinate claimed by the bundle.
        pub footprint_algebra: &'a str,
        /// Digest-framing domain for the footprint algebra.
        pub footprint_algebra_digest_domain: &'a str,
        /// Footprint-algebra digest claimed by the bundle.
        pub footprint_algebra_digest: &'a str,
    }

    /// Stable reason an assembled bundle cannot bind this generated helper.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum BindingMismatchKind {
        /// A bundle digest is framed under the wrong proposition domain.
        BundleDigestDomain,
        /// A semantic or release bundle digest is not a typed SHA-256 review value.
        BundleDigest,
        /// The assembled semantic-bundle digest differs from the host pin.
        SemanticBundleDigest,
        /// The assembled release-bundle digest differs from the host pin.
        ReleaseBundleDigest,
        /// The bundle names a different semantic operation.
        Operation,
        /// The bundle names a different persisted operation-id proposition.
        OperationId,
        /// The bundle names a different Echo value codec.
        Codec,
        /// The bundle names a different Target IR artifact.
        TargetIr,
        /// The bundle names a different target profile.
        TargetProfile,
        /// The bundle names a different target-bundle profile.
        TargetBundleProfile,
        /// The generated registry targets a different Echo contract ABI.
        EchoAbi,
        /// The generated registry targets a different contract-host helper API.
        HelperApi,
        /// The bundle names different provider or operation schema identities.
        Schema,
        /// The bundle names a different generated-artifact profile.
        GeneratedArtifactProfile,
        /// The bundle names a different semantic operation profile.
        OperationProfile,
        /// The bundle names a different footprint obligation or algebra.
        Footprint,
    }

    /// Exact generated registration binding, still without runtime authority.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct RegistrationDescriptorV1<'a> {
        contract_bundle: ContractBundleIdentityV1<'a>,
    }

    impl<'a> RegistrationDescriptorV1<'a> {
        /// Return the exact bundle claims that matched the explicit host pin.
        pub const fn contract_bundle(&self) -> &ContractBundleIdentityV1<'a> {
            &self.contract_bundle
        }

        /// Return the exact persisted operation id matched by this descriptor.
        pub const fn operation_id(&self) -> u32 {
            self.contract_bundle.operation_id
        }

        /// Return the exact provider-generic registry claims retained by this
        /// already matched descriptor.
        ///
        /// This constructs descriptive evidence only. It does not admit or
        /// install a registry.
        pub const fn provider_registry(&self) -> ProviderRegistryV1<'a> {
            ProviderRegistryV1 {
                echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION,
                helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
                provider_schema: ProviderSchemaIdentityV1 {
                    coordinate: PROVIDER_SCHEMA_COORDINATE,
                    raw_sha256_hex: PROVIDER_SCHEMA_SHA256_HEX,
                },
                target_bundle_profile: ProviderDigestIdentityV1 {
                    coordinate: TARGET_BUNDLE_PROFILE_COORDINATE,
                    digest_domain: TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN,
                    digest: TARGET_BUNDLE_PROFILE_DIGEST,
                },
                bundle: ProviderBundleIdentityV1 {
                    semantic_digest_domain: self.contract_bundle.semantic_digest_domain,
                    semantic_digest: self.contract_bundle.semantic_digest,
                    release_digest_domain: self.contract_bundle.release_digest_domain,
                    release_digest: self.contract_bundle.release_digest,
                },
                operations: &PROVIDER_OPERATIONS,
            }
        }

        /// Return the exact identity a host implementation must independently
        /// claim before its callbacks can be proposed for this operation.
        pub const fn mutation_implementation_identity(
            &self,
        ) -> ProviderMutationImplementationIdentityV1<'a> {
            let registry = self.provider_registry();
            ProviderMutationImplementationIdentityV1 {
                echo_contract_abi_version: registry.echo_contract_abi_version,
                helper_api_version: registry.helper_api_version,
                provider_schema: registry.provider_schema,
                target_bundle_profile: registry.target_bundle_profile,
                bundle: registry.bundle,
                operation: PROVIDER_OPERATIONS[0],
            }
        }

        /// Construct one opaque package proposal from exact matched claims and
        /// an explicit host executor/footprint binding.
        ///
        /// The result grants no runtime admission, registration, installation,
        /// scheduling, execution, durability, or receipt authority.
        ///
        /// # Errors
        ///
        /// Returns [`ProviderPackageProposalError`] when occurrence metadata,
        /// generated dispatch identity, or any host implementation claim does
        /// not exactly match this descriptor.
        pub fn propose_contract_package<'proposal>(
            &'proposal self,
            occurrence: ContractPackageIdentity<'proposal>,
            hooks: ProviderMutationHooksV1<'proposal>,
        ) -> Result<ProviderContractPackageProposalV1<'proposal>, ProviderPackageProposalError>
        where
            'a: 'proposal,
        {
            let registry: ProviderRegistryV1<'proposal> = self.provider_registry();
            propose_provider_contract_package_v1(
                occurrence,
                registry,
                GeneratedProviderMutationDispatchV1::new(
                    OPERATION_ID,
                    MUTATION_RULE_NAME,
                    matches_operation,
                ),
                hooks,
            )
        }

        /// Encode one exact typed input under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] when the typed value violates its generated
        /// bound or cannot be represented by the selected codec.
        #[allow(clippy::unused_self)]
        pub fn encode_input(&self, input: &Input) -> Result<Vec<u8>, CodecError> {
            encode_to_vec(input)
        }

        /// Decode one exact typed input under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] for malformed, over-bound, truncated, or
        /// trailing bytes.
        #[allow(clippy::unused_self)]
        pub fn decode_input(&self, bytes: &[u8]) -> Result<Input, CodecError> {
            decode_from_bytes(bytes)
        }

        /// Encode one exact typed output under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] when the typed value violates its generated
        /// bound or cannot be represented by the selected codec.
        #[allow(clippy::unused_self)]
        pub fn encode_output(&self, output: &Output) -> Result<Vec<u8>, CodecError> {
            encode_to_vec(output)
        }

        /// Decode one exact typed output under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] for malformed, over-bound, truncated, or
        /// trailing bytes.
        #[allow(clippy::unused_self)]
        pub fn decode_output(&self, bytes: &[u8]) -> Result<Output, CodecError> {
            decode_from_bytes(bytes)
        }

        /// Encode a typed input and wrap it in the canonical Echo EINT v1
        /// envelope for this matched operation id.
        ///
        /// # Errors
        ///
        /// Returns [`GeneratedInvocationError::Codec`] when the input violates
        /// its generated value contract, or
        /// [`GeneratedInvocationError::Envelope`] when Echo refuses envelope
        /// construction.
        pub fn pack_intent(&self, input: &Input) -> Result<Vec<u8>, GeneratedInvocationError> {
            let vars = self
                .encode_input(input)
                .map_err(GeneratedInvocationError::Codec)?;
            pack_intent_v1(self.operation_id(), &vars)
                .map_err(GeneratedInvocationError::Envelope)
        }
    }

    fn matches_operation(view: GraphView<'_>, scope: &NodeId) -> bool {
        matches_eint_op(view, scope, OPERATION_ID)
    }

    /// Compare assembled bundle claims to an independent exact host pin and to
    /// this generated helper's semantic identities.
    ///
    /// This pure equality/consistency preflight neither authenticates the pin,
    /// admits the bundle, nor installs a package. Those remain separate trusted
    /// host and Echo runtime crossings.
    pub fn bind_contract_bundle<'a>(
        expected: ExpectedContractBundleIdentityV1<'a>,
        identity: &ContractBundleIdentityV1<'a>,
    ) -> Result<RegistrationDescriptorV1<'a>, BindingMismatchKind> {
        if expected.semantic_digest_domain != SEMANTIC_BUNDLE_DIGEST_DOMAIN
            || identity.semantic_digest_domain != SEMANTIC_BUNDLE_DIGEST_DOMAIN
            || expected.release_digest_domain != RELEASE_BUNDLE_DIGEST_DOMAIN
            || identity.release_digest_domain != RELEASE_BUNDLE_DIGEST_DOMAIN
        {
            return Err(BindingMismatchKind::BundleDigestDomain);
        }
        if !is_sha256_review(expected.semantic_digest)
            || !is_sha256_review(expected.release_digest)
            || !is_sha256_review(identity.semantic_digest)
            || !is_sha256_review(identity.release_digest)
        {
            return Err(BindingMismatchKind::BundleDigest);
        }
        if identity.semantic_digest != expected.semantic_digest {
            return Err(BindingMismatchKind::SemanticBundleDigest);
        }
        if identity.release_digest != expected.release_digest {
            return Err(BindingMismatchKind::ReleaseBundleDigest);
        }
        if identity.operation_coordinate != OPERATION_COORDINATE
            || identity.operation_domain != OPERATION_DOMAIN
        {
            return Err(BindingMismatchKind::Operation);
        }
        if identity.operation_id_law != OPERATION_ID_LAW || identity.operation_id != OPERATION_ID {
            return Err(BindingMismatchKind::OperationId);
        }
        if identity.value_codec != VALUE_CODEC_ID {
            return Err(BindingMismatchKind::Codec);
        }
        if identity.target_ir_coordinate != TARGET_IR_COORDINATE
            || identity.target_ir_digest_domain != TARGET_IR_DIGEST_DOMAIN
            || identity.target_ir_digest != TARGET_IR_DIGEST
        {
            return Err(BindingMismatchKind::TargetIr);
        }
        if identity.target_profile_coordinate != TARGET_PROFILE_COORDINATE
            || identity.target_profile_digest_domain != TARGET_PROFILE_DIGEST_DOMAIN
            || identity.target_profile_digest != TARGET_PROFILE_DIGEST
        {
            return Err(BindingMismatchKind::TargetProfile);
        }
        if identity.target_bundle_profile_coordinate != TARGET_BUNDLE_PROFILE_COORDINATE
            || identity.target_bundle_profile_digest_domain
                != TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN
            || identity.target_bundle_profile_digest != TARGET_BUNDLE_PROFILE_DIGEST
        {
            return Err(BindingMismatchKind::TargetBundleProfile);
        }
        if identity.echo_contract_abi_version != ECHO_CONTRACT_ABI_VERSION {
            return Err(BindingMismatchKind::EchoAbi);
        }
        if identity.helper_api_version != CONTRACT_HOST_HELPER_API_VERSION {
            return Err(BindingMismatchKind::HelperApi);
        }
        if identity.provider_schema_coordinate != PROVIDER_SCHEMA_COORDINATE
            || identity.provider_schema_sha256_hex != PROVIDER_SCHEMA_SHA256_HEX
            || identity.input_schema != INPUT_SCHEMA
            || identity.output_schema != OUTPUT_SCHEMA
            || identity.type_schema_domain != TYPE_SCHEMA_DOMAIN
            || identity.obstruction_coordinate != OBSTRUCTION_COORDINATE
            || identity.obstruction_domain != OBSTRUCTION_DOMAIN
            || identity.effect_failure_schema != EFFECT_FAILURE_SCHEMA
            || identity.obstruction_payload_schema != OBSTRUCTION_PAYLOAD_SCHEMA
        {
            return Err(BindingMismatchKind::Schema);
        }
        if identity.generated_artifact_profile != GENERATED_ARTIFACT_PROFILE
            || identity.generated_artifact_profile_digest_domain
                != GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN
            || identity.generated_artifact_profile_digest
                != GENERATED_ARTIFACT_PROFILE_DIGEST
        {
            return Err(BindingMismatchKind::GeneratedArtifactProfile);
        }
        if identity.operation_profile != OPERATION_PROFILE
            || identity.operation_profile_domain != OPERATION_PROFILE_DOMAIN
            || identity.operation_profiles_coordinate != OPERATION_PROFILES_COORDINATE
            || identity.operation_profiles_digest_domain != OPERATION_PROFILES_DIGEST_DOMAIN
            || identity.operation_profiles_digest != OPERATION_PROFILES_DIGEST
        {
            return Err(BindingMismatchKind::OperationProfile);
        }
        if identity.footprint_obligation != FOOTPRINT_OBLIGATION
            || identity.footprint_algebra != FOOTPRINT_ALGEBRA
            || identity.footprint_algebra_digest_domain != FOOTPRINT_ALGEBRA_DIGEST_DOMAIN
            || identity.footprint_algebra_digest != FOOTPRINT_ALGEBRA_DIGEST
        {
            return Err(BindingMismatchKind::Footprint);
        }
        Ok(RegistrationDescriptorV1 {
            contract_bundle: *identity,
        })
    }

    fn is_sha256_review(value: &str) -> bool {
        let Some(hex) = value.strip_prefix("sha256:") else {
            return false;
        };
        hex.len() == 64
            && hex
                .as_bytes()
                .iter()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    }
}
"#;

    TEMPLATE
        .replace("__TARGET_IR_DIGEST__", &digest_review(target_ir_digest))
        .replace(
            "__TARGET_PROFILE_DIGEST__",
            &digest_review(target_profile_digest),
        )
        .replace(
            "__TARGET_BUNDLE_PROFILE_DIGEST__",
            TARGET_BUNDLE_PROFILE_DIGEST,
        )
}

fn render_review_payload(
    target_ir_digest: &Digest,
    target_profile_digest: &Digest,
    generated_artifact_digest: &Digest,
) -> String {
    format!(
        concat!(
            "{{\"apiVersion\":\"echo.generated-helper-review/v1\",",
            "\"authoritative\":false,",
            "\"operation\":\"a.b@1.t\",",
            "\"targetIr\":{{\"coordinate\":\"{}\",\"digestDomain\":\"{}\",",
            "\"digest\":\"{}\"}},",
            "\"targetProfile\":{{\"id\":\"echo.dpo@1\",\"digest\":\"{}\"}},",
            "\"targetBundleProfile\":{{\"id\":\"{}\",\"digest\":\"{}\"}},",
            "\"generatedArtifact\":{{\"coordinate\":\"{}\",\"digestDomain\":\"{}\",",
            "\"digest\":\"{}\"}},",
            "\"contractBundle\":{{\"binding\":\"explicit-after-assembly\",",
            "\"semanticDigestDomain\":\"{}\",\"semanticDigest\":null,",
            "\"releaseDigestDomain\":\"{}\",\"releaseDigest\":null}},",
            "\"runtimeAuthority\":false}}\n"
        ),
        TARGET_IR_COORDINATE,
        TARGET_IR_ARTIFACT_DOMAIN,
        digest_review(target_ir_digest),
        digest_review(target_profile_digest),
        TARGET_BUNDLE_PROFILE,
        TARGET_BUNDLE_PROFILE_DIGEST,
        GENERATED_ARTIFACT_ROLE,
        GENERATED_ARTIFACT_DOMAIN,
        digest_review(generated_artifact_digest),
        SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        RELEASE_BUNDLE_DIGEST_DOMAIN,
    )
}

fn decode_sha256_review(value: &str) -> Option<Vec<u8>> {
    let hex = value.strip_prefix("sha256:")?;
    if hex.len() != 64 {
        return None;
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Some((high << 4) | low)
        })
        .collect()
}

const fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn invalid_output_artifact(subject: &str, code: &str, message: &str) -> ProviderRefusalV1 {
    refusal(
        ProviderRefusalKind::InvalidSemanticArtifact,
        subject,
        code,
        message,
    )
}

fn lower_core(
    core: &BoundArtifact,
    target_profile_digest: &Digest,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    if core.artifact.domain != CORE_DOMAIN {
        return Err(invalid_artifact(
            "core.echo-provider",
            "Core domain is invalid",
        ));
    }
    let value = validate_binding(core)
        .map_err(|()| invalid_artifact("core.echo-provider", "Core binding is invalid"))?;
    let api_version = text_field(&value, "apiVersion")
        .ok_or_else(|| invalid_artifact("core.echo-provider", "Core apiVersion is absent"))?;
    if api_version != CORE_ABI {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedCoreAbi,
            api_version,
            "echo.provider.unsupported-core-abi",
            "the lowerer accepts only edict.core/v1",
        ));
    }
    let coordinate = text_field(&value, "coordinate")
        .filter(|coordinate| !coordinate.is_empty())
        .ok_or_else(|| invalid_artifact("core.echo-provider", "Core coordinate is invalid"))?;
    if coordinate != core.reference.coordinate {
        return Err(invalid_artifact(
            "core.echo-provider",
            "Core coordinate does not equal its bound reference",
        ));
    }
    if coordinate != CORE_COORDINATE {
        return Err(unsupported_semantics(coordinate));
    }
    let expected_types = expected_core_types().map_err(|()| {
        invalid_artifact(
            "core.echo-provider",
            "reviewed Core type definitions are invalid",
        )
    })?;
    if !matches!(array_field(&value, "imports"), Some(imports) if imports.is_empty())
        || map_field(&value, "types") != Some(&expected_types)
        || !matches!(array_field(&value, "requiredCoreCapabilities"), Some(capabilities) if capabilities.is_empty())
    {
        return Err(unsupported_semantics(coordinate));
    }

    let intents = map_field(&value, "intents")
        .and_then(as_map)
        .ok_or_else(|| invalid_artifact("core.echo-provider", "Core intents map is invalid"))?;
    let [(intent_key, intent)] = intents.as_slice() else {
        return Err(unsupported_semantics(coordinate));
    };
    let intent_name = as_text(intent_key).ok_or_else(|| unsupported_semantics(coordinate))?;
    if intent_name != "t" {
        return Err(unsupported_semantics(intent_name));
    }
    let lowered_intent = lower_intent(intent_name, intent)?;
    let digest_value = target_profile_digest_value(target_profile_digest)
        .ok_or_else(|| invalid_artifact("target-profile.echo-dpo", "digest is invalid"))?;

    Ok(canonical_map([
        ("kind", canonical_text("targetIrArtifact")),
        ("domain", canonical_text(TARGET_IR_COORDINATE)),
        (
            "targetProfile",
            canonical_map([
                ("id", canonical_text(TARGET_PROFILE_COORDINATE)),
                ("digest", digest_value),
            ]),
        ),
        ("sourceCoreCoordinate", canonical_text(coordinate)),
        ("intents", canonical_map([(intent_name, lowered_intent)])),
    ]))
}

fn lower_intent(
    intent_name: &str,
    intent: &CanonicalValueV1,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    if text_field(intent, "input") != Some(OPERATION_INPUT_TYPE)
        || text_field(intent, "output") != Some(OPERATION_OUTPUT_TYPE)
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    if map_field(intent, "optic").is_some() {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    if text_field(intent, "requiredOperationProfile") != Some(OPERATION_PROFILE) {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let input_constraints = array_field(intent, "inputConstraints")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "input constraints are invalid"))?;
    if !input_constraints.is_empty() {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let budget = map_field(intent, "coreEvaluationBudget")
        .filter(|budget| validate_budget(budget))
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core budget is invalid"))?;
    let expected_budget = expected_core_evaluation_budget().map_err(|()| {
        invalid_artifact(
            OPERATION_COORDINATE,
            "reviewed Core evaluation budget is invalid",
        )
    })?;
    if budget != &expected_budget {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let body = map_field(intent, "body")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core body is absent"))?;
    let locals = array_field(body, "locals")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core locals are invalid"))?;
    if !validate_local_inventory(locals) {
        return Err(invalid_artifact(
            OPERATION_COORDINATE,
            "Core local declarations are invalid",
        ));
    }
    let nodes = array_field(body, "nodes")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core nodes are invalid"))?;
    let [node] = nodes.as_slice() else {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    };
    let lowered_effect = lower_effect_node(intent_name, node, locals)?;
    let result_scope = [lowered_effect.input_local, lowered_effect.binding];
    let result = map_field(body, "result")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core result is invalid"))?;
    validate_expr(result, &result_scope, &[])
        .map_err(|error| expression_refusal(error, "Core result is invalid"))?;

    Ok(canonical_map([
        ("operationProfile", canonical_text(OPERATION_PROFILE)),
        (
            "inputConstraints",
            CanonicalValueV1::Array(input_constraints.clone()),
        ),
        ("coreEvaluationBudget", budget.clone()),
        ("requirements", CanonicalValueV1::Array(Vec::new())),
        ("steps", CanonicalValueV1::Array(vec![lowered_effect.value])),
        ("result", result.clone()),
    ]))
}

struct LoweredEffect<'a> {
    value: CanonicalValueV1,
    input_local: &'a CanonicalValueV1,
    binding: &'a CanonicalValueV1,
}

fn lower_effect_node<'a>(
    intent_name: &str,
    node: &'a CanonicalValueV1,
    locals: &'a [CanonicalValueV1],
) -> Result<LoweredEffect<'a>, ProviderRefusalV1> {
    if text_field(node, "kind") != Some("effect")
        || text_field(node, "effect") != Some(SEMANTIC_EFFECT)
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let binding = map_field(node, "binding")
        .filter(|binding| validate_local(binding))
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "effect binding is invalid"))?;
    let obstruction_map = map_field(node, "obstructionMap")
        .and_then(as_map)
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "obstruction map is invalid"))?;
    let [(failure, arm)] = obstruction_map.as_slice() else {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    };
    if as_text(failure) != Some(FAILURE_COORDINATE) {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let obstruction_binder = map_field(arm, "binder")
        .filter(|binder| validate_local(binder))
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "obstruction binder is invalid"))?;
    let input_local =
        reviewed_input_local(locals, binding, obstruction_binder).ok_or_else(|| {
            invalid_artifact(OPERATION_COORDINATE, "Core local declarations are invalid")
        })?;

    let pre_effect_scope = [input_local];
    let input = map_field(node, "input")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "effect input is invalid"))?;
    validate_expr(input, &pre_effect_scope, &[])
        .map_err(|error| expression_refusal(error, "effect input is invalid"))?;

    let obstruction_scope = [input_local, obstruction_binder];
    let obstruction_value = map_field(arm, "value")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "obstruction value is invalid"))?;
    if text_field(obstruction_value, "kind") != Some("call")
        || text_field(obstruction_value, "callee") != Some(DOMAIN_OBSTRUCTION)
        || !matches!(array_field(obstruction_value, "typeArgs"), Some(arguments) if arguments.is_empty())
        || !matches!(array_field(obstruction_value, "args"), Some(arguments) if arguments.is_empty())
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    validate_expr(obstruction_value, &obstruction_scope, &[DOMAIN_OBSTRUCTION])
        .map_err(|error| expression_refusal(error, "obstruction value is invalid"))?;

    Ok(LoweredEffect {
        value: canonical_map([
            ("id", canonical_text(&format!("{intent_name}.step.0"))),
            ("binding", binding.clone()),
            ("effect", canonical_text(SEMANTIC_EFFECT)),
            ("targetIntrinsic", canonical_text(TARGET_INTRINSIC)),
            ("input", input.clone()),
            (
                "obstructionFailures",
                CanonicalValueV1::Array(vec![canonical_text(FAILURE_COORDINATE)]),
            ),
            (
                "obstructionArms",
                canonical_map([(FAILURE_COORDINATE, arm.clone())]),
            ),
        ]),
        input_local,
        binding,
    })
}

fn validate_binding(bound: &BoundArtifact) -> Result<CanonicalValueV1, ()> {
    if bound.reference.coordinate.is_empty()
        || bound.reference.digest.algorithm != DigestAlgorithm::Sha256
        || bound.reference.digest.bytes.len() != 32
        || bound.artifact.domain.is_empty()
    {
        return Err(());
    }
    let value = decode_canonical_cbor_v1(&bound.artifact.bytes).map_err(|_| ())?;
    let computed = digest_canonical_value_v1(&bound.artifact.domain, &value).map_err(|_| ())?;
    if computed != digest_review(&bound.reference.digest) {
        return Err(());
    }
    Ok(value)
}

fn digest_review(digest: &Digest) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut review = String::with_capacity(7 + digest.bytes.len() * 2);
    review.push_str("sha256:");
    for byte in &digest.bytes {
        review.push(char::from(HEX[usize::from(byte >> 4)]));
        review.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    review
}

fn target_profile_digest_value(digest: &Digest) -> Option<CanonicalValueV1> {
    (digest.algorithm == DigestAlgorithm::Sha256 && digest.bytes.len() == 32).then(|| {
        CanonicalValueV1::Array(vec![
            canonical_text("sha256"),
            CanonicalValueV1::Bytes(digest.bytes.clone()),
        ])
    })
}

fn validate_budget(value: &CanonicalValueV1) -> bool {
    ["maxSteps", "maxAllocatedBytes", "maxOutputBytes"]
        .into_iter()
        .all(|field| matches!(map_field(value, field), Some(CanonicalValueV1::Integer(value)) if *value >= 0))
}

fn validate_local(value: &CanonicalValueV1) -> bool {
    ["id", "alphaName", "type"]
        .into_iter()
        .all(|field| text_field(value, field).is_some_and(|value| !value.is_empty()))
}

fn validate_local_inventory(locals: &[CanonicalValueV1]) -> bool {
    locals.iter().enumerate().all(|(index, local)| {
        validate_local(local)
            && locals[index + 1..]
                .iter()
                .all(|other| !same_local_id(local, other))
    })
}

fn reviewed_input_local<'a>(
    locals: &'a [CanonicalValueV1],
    binding: &CanonicalValueV1,
    obstruction_binder: &CanonicalValueV1,
) -> Option<&'a CanonicalValueV1> {
    if locals.len() != 3
        || text_field(binding, "type") != Some(OPERATION_RECEIPT_TYPE)
        || text_field(obstruction_binder, "type") != Some(FAILURE_PAYLOAD_TYPE)
        || !locals.contains(binding)
        || !locals.contains(obstruction_binder)
        || same_local_id(binding, obstruction_binder)
    {
        return None;
    }

    let mut inputs = locals.iter().filter(|local| {
        !same_local_id(local, binding) && !same_local_id(local, obstruction_binder)
    });
    let input = inputs.next()?;
    if inputs.next().is_some() || text_field(input, "type") != Some(OPERATION_INPUT_TYPE) {
        return None;
    }
    Some(input)
}

fn same_local_id(left: &CanonicalValueV1, right: &CanonicalValueV1) -> bool {
    text_field(left, "id").is_some_and(|id| text_field(right, "id") == Some(id))
}

#[derive(Clone, Copy)]
enum ExpressionValidationError {
    Invalid,
    LocalOutOfScope,
    UnsupportedCall,
}

fn validate_expr(
    value: &CanonicalValueV1,
    scope: &[&CanonicalValueV1],
    allowed_callees: &[&str],
) -> Result<(), ExpressionValidationError> {
    match text_field(value, "kind") {
        Some("local") => {
            let reference = map_field(value, "ref")
                .filter(|reference| validate_local(reference))
                .ok_or(ExpressionValidationError::Invalid)?;
            if scope.contains(&reference) {
                Ok(())
            } else {
                Err(ExpressionValidationError::LocalOutOfScope)
            }
        }
        Some("const") => map_field(value, "value")
            .filter(|value| validate_core_value(value))
            .map(|_| ())
            .ok_or(ExpressionValidationError::Invalid),
        Some("record") => {
            let fields = map_field(value, "fields")
                .and_then(as_map)
                .ok_or(ExpressionValidationError::Invalid)?;
            for (key, value) in fields {
                if as_text(key).is_none_or(str::is_empty) {
                    return Err(ExpressionValidationError::Invalid);
                }
                validate_expr(value, scope, allowed_callees)?;
            }
            Ok(())
        }
        Some("field") => {
            if text_field(value, "field").is_none_or(str::is_empty) {
                return Err(ExpressionValidationError::Invalid);
            }
            validate_expr(
                map_field(value, "base").ok_or(ExpressionValidationError::Invalid)?,
                scope,
                allowed_callees,
            )
        }
        Some("call") => {
            let callee = text_field(value, "callee")
                .filter(|callee| !callee.is_empty())
                .ok_or(ExpressionValidationError::Invalid)?;
            if !allowed_callees.contains(&callee) {
                return Err(ExpressionValidationError::UnsupportedCall);
            }
            if !array_field(value, "typeArgs")
                .is_some_and(|values| values.iter().all(|value| as_text(value).is_some()))
            {
                return Err(ExpressionValidationError::Invalid);
            }
            for argument in array_field(value, "args").ok_or(ExpressionValidationError::Invalid)? {
                validate_expr(argument, scope, allowed_callees)?;
            }
            Ok(())
        }
        _ => Err(ExpressionValidationError::Invalid),
    }
}

fn expression_refusal(
    error: ExpressionValidationError,
    invalid_message: &str,
) -> ProviderRefusalV1 {
    match error {
        ExpressionValidationError::Invalid => {
            invalid_artifact(OPERATION_COORDINATE, invalid_message)
        }
        ExpressionValidationError::LocalOutOfScope => local_scope_refusal(),
        ExpressionValidationError::UnsupportedCall => unsupported_semantics(OPERATION_COORDINATE),
    }
}

fn validate_core_value(value: &CanonicalValueV1) -> bool {
    match text_field(value, "kind") {
        Some("null") => true,
        Some("bool") => matches!(map_field(value, "value"), Some(CanonicalValueV1::Bool(_))),
        Some("int") => {
            text_field(value, "width").is_some()
                && matches!(
                    map_field(value, "value"),
                    Some(CanonicalValueV1::Integer(_))
                )
        }
        Some("string") => text_field(value, "value").is_some(),
        Some("bytes") => matches!(map_field(value, "value"), Some(CanonicalValueV1::Bytes(_))),
        _ => false,
    }
}

fn canonical_map<'a>(
    entries: impl IntoIterator<Item = (&'a str, CanonicalValueV1)>,
) -> CanonicalValueV1 {
    CanonicalValueV1::Map(
        entries
            .into_iter()
            .map(|(key, value)| (canonical_text(key), value))
            .collect(),
    )
}

fn canonical_text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn map_field<'a>(value: &'a CanonicalValueV1, field: &str) -> Option<&'a CanonicalValueV1> {
    as_map(value)?.iter().find_map(|(key, value)| {
        (matches!(key, CanonicalValueV1::Text(key) if key == field)).then_some(value)
    })
}

fn text_field<'a>(value: &'a CanonicalValueV1, field: &str) -> Option<&'a str> {
    map_field(value, field).and_then(as_text)
}

fn array_field<'a>(value: &'a CanonicalValueV1, field: &str) -> Option<&'a Vec<CanonicalValueV1>> {
    match map_field(value, field)? {
        CanonicalValueV1::Array(values) => Some(values),
        _ => None,
    }
}

fn as_map(value: &CanonicalValueV1) -> Option<&Vec<(CanonicalValueV1, CanonicalValueV1)>> {
    match value {
        CanonicalValueV1::Map(entries) => Some(entries),
        _ => None,
    }
}

fn as_text(value: &CanonicalValueV1) -> Option<&str> {
    match value {
        CanonicalValueV1::Text(value) => Some(value),
        _ => None,
    }
}

fn refusal(
    kind: ProviderRefusalKind,
    subject: impl Into<String>,
    code: &str,
    message: &str,
) -> ProviderRefusalV1 {
    ProviderRefusalV1 {
        kind,
        subject: Some(subject.into()),
        diagnostics: vec![Diagnostic {
            code: code.to_owned(),
            severity: DiagnosticSeverity::Error,
            message: message.to_owned(),
            repair: None,
        }],
    }
}

fn invalid_artifact(subject: &str, message: &str) -> ProviderRefusalV1 {
    refusal(
        ProviderRefusalKind::InvalidSemanticArtifact,
        subject,
        "echo.provider.invalid-semantic-artifact",
        message,
    )
}

fn local_scope_refusal() -> ProviderRefusalV1 {
    refusal(
        ProviderRefusalKind::InvalidSemanticArtifact,
        OPERATION_COORDINATE,
        "echo.provider.local-reference-out-of-scope",
        "Core local reference is not in scope",
    )
}

fn unsupported_semantics(subject: &str) -> ProviderRefusalV1 {
    refusal(
        ProviderRefusalKind::UnsupportedSemantics,
        subject,
        "echo.provider.unsupported-semantics",
        "the supplied semantics are outside the exact first Echo lowering closure",
    )
}
