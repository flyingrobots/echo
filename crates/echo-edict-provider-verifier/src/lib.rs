// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure Echo semantic verification for Edict's frozen provider boundary.
//!
//! This crate compares only explicit, digest-bound canonical artifacts. It
//! performs no discovery or I/O and grants no Echo runtime authority.
//! Reports bind only the exact Target IR reference they name; the provider
//! package binds that proposition to its Core, profile, closure, and component.

#![deny(unsafe_code)]

use std::fmt::Write as _;

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1, CanonicalValueV1,
};

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
const TARGET_IR_DOMAIN: &str = "edict.target-ir.artifact/v1";
const INNER_TARGET_IR_DOMAIN: &str = "echo.span-ir/v1";
const REPORT_DOMAIN: &str = "echo.verifier-report/v1";
const REPORT_ROLE: &str = "verifier-report.echo-dpo";
const OPERATION_COORDINATE: &str = "a.b@1.t";
const OPERATION_INPUT_TYPE: &str = "a.b@1.Input";
const OPERATION_OUTPUT_TYPE: &str = "a.b@1.Output";
const OPERATION_RECEIPT_TYPE: &str = "a.b@1.Receipt";
const INPUT_ID_TYPE: &str = "a.b@1.Input.id";
const OUTPUT_ID_TYPE: &str = "a.b@1.Output.id";
const RECEIPT_ID_TYPE: &str = "a.b@1.Receipt.id";
const OPERATION_PROFILE: &str = "continuum.profile.write/v1";
const SEMANTIC_EFFECT: &str = "target.replace";
const TARGET_INTRINSIC: &str = "echo.dpo@1.replace";
const FAILURE_COORDINATE: &str = "rejected";
const FAILURE_PAYLOAD_TYPE: &str = "target.replace.rejected";
const DOMAIN_OBSTRUCTION: &str = "domain.WriteRejected";
const MAX_EXPRESSION_DEPTH: usize = 64;

const DIAGNOSTIC_ABI_DIGEST: [u8; 32] = [
    0x28, 0xfd, 0x72, 0xa9, 0x82, 0x23, 0x15, 0x39, 0x82, 0xca, 0x08, 0x4c, 0x29, 0xdb, 0xb1, 0xb2,
    0xd4, 0x30, 0x62, 0x39, 0x67, 0xab, 0x3b, 0x6d, 0xb9, 0xd7, 0xfe, 0xe6, 0x68, 0xe6, 0x14, 0xb9,
];

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

/// Output authority available to the verifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationOutputKind {
    /// Target-owned verifier report.
    VerifierReport,
}

/// One requested verifier output role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationOutputRequest {
    /// Unique output role.
    pub role: String,
    /// Structurally permitted output kind.
    pub kind: VerificationOutputKind,
    /// Required owning domain.
    pub domain: String,
}

/// One provider-authored verifier output without a provider-authored digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationOutputArtifact {
    /// Requested output role.
    pub role: String,
    /// Requested output kind.
    pub kind: VerificationOutputKind,
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

/// Target-owned refusal categories frozen by the provider ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderRefusalKind {
    /// The request or Core ABI is unsupported.
    UnsupportedCoreAbi,
    /// The selected target profile is unsupported.
    UnsupportedTargetProfile,
    /// The supplied semantics cannot be evaluated faithfully.
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

/// Explicit pure verification request mirroring the frozen WIT record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRequestV1 {
    /// Requested provider protocol version.
    pub protocol_version: ProtocolVersionV1,
    /// Exact canonical Edict Core artifact.
    pub core: BoundArtifact,
    /// Exact canonical target-profile artifact.
    pub target_profile: BoundArtifact,
    /// Exact canonical Target IR artifact under judgment.
    pub target_ir: BoundArtifact,
    /// Complete explicit semantic closure.
    pub semantic_inputs: Vec<SemanticInput>,
    /// Exact requested verifier-report roles.
    pub requested_outputs: Vec<VerificationOutputRequest>,
    /// Host-owned response limits, which cannot alter canonical provider output.
    pub limits: ResponseLimitsV1,
}

/// Successful semantic adjudication response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationSuccessV1 {
    /// Exactly the requested and supported outputs.
    pub outputs: Vec<VerificationOutputArtifact>,
    /// Deterministically ordered semantic diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Pure verifier result using the frozen provider refusal vocabulary.
pub type VerificationResultV1 = Result<VerificationSuccessV1, ProviderRefusalV1>;

/// Verify the first supported Echo Core-to-Target-IR semantic relation.
///
/// A supported but false Target IR returns a successful response containing a
/// rejected report. A refusal means the component could not adjudicate the
/// relation under its declared closure.
pub fn verify(request: VerificationRequestV1) -> VerificationResultV1 {
    if request.protocol_version != PROVIDER_ABI {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedCoreAbi,
            "protocol-version",
            "echo.verifier.unsupported-protocol",
            "the verifier accepts only target-provider protocol 1.0.0",
        ));
    }
    validate_target_profile(&request.target_profile)?;
    validate_semantic_closure(&request.semantic_inputs)?;
    validate_requested_outputs(&request.requested_outputs)?;

    let core = validate_core(&request.core)?;
    let target_ir = validate_target_ir_artifact(&request.target_ir)?;
    let expected = expected_target_ir(&core, &request.target_profile.reference.digest)
        .map_err(|()| unsupported_semantics(OPERATION_COORDINATE))?;
    let failure = relation_failure(&target_ir, &expected);
    let (outcome, diagnostics) = match failure {
        None => ("accepted", Vec::new()),
        Some(failure) => (
            "rejected",
            vec![Diagnostic {
                code: failure.code.to_owned(),
                severity: DiagnosticSeverity::Error,
                message: failure.message.to_owned(),
                repair: None,
            }],
        ),
    };
    let report = build_report(&request.target_ir.reference, outcome).map_err(|()| {
        invalid_artifact(
            REPORT_ROLE,
            "the canonical verifier report could not be constructed",
        )
    })?;
    Ok(VerificationSuccessV1 {
        outputs: vec![VerificationOutputArtifact {
            role: REPORT_ROLE.to_owned(),
            kind: VerificationOutputKind::VerifierReport,
            artifact: Artifact {
                domain: REPORT_DOMAIN.to_owned(),
                bytes: report,
            },
            logical_path: None,
        }],
        diagnostics,
    })
}

fn validate_target_profile(profile: &BoundArtifact) -> Result<(), ProviderRefusalV1> {
    validate_binding(profile).map_err(|()| {
        invalid_artifact(
            "target-profile.echo-dpo",
            "target-profile binding is invalid",
        )
    })?;
    if profile.reference.coordinate != TARGET_PROFILE_COORDINATE
        || profile.artifact.domain != TARGET_PROFILE_DOMAIN
        || profile.artifact.bytes != TARGET_PROFILE_BYTES
    {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedTargetProfile,
            &profile.reference.coordinate,
            "echo.verifier.unsupported-target-profile",
            "the verifier accepts only the exact checked Echo DPO target profile",
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
    for ((role, kind, coordinate, domain, bytes), input) in EXPECTED.into_iter().zip(inputs) {
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
        || lowerability.artifact.reference.coordinate != LOWERABILITY_COORDINATE
        || lowerability.artifact.artifact.domain != LOWERABILITY_DOMAIN
    {
        return Err(unsupported_semantics(&lowerability.role));
    }
    let value = validate_binding(&lowerability.artifact)
        .map_err(|()| invalid_artifact(&lowerability.role, "artifact binding is invalid"))?;
    if value != expected_lowerability().map_err(|()| unsupported_semantics(&lowerability.role))? {
        return Err(unsupported_semantics(&lowerability.role));
    }
    Ok(())
}

fn validate_requested_outputs(
    requests: &[VerificationOutputRequest],
) -> Result<(), ProviderRefusalV1> {
    let Some((request, remaining)) = requests.split_first() else {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedOutputRole,
            "requested-outputs",
            "echo.verifier.unsupported-output-role",
            "the first verifier requires its verifier-report output",
        ));
    };
    if request.role != REPORT_ROLE
        || request.kind != VerificationOutputKind::VerifierReport
        || request.domain != REPORT_DOMAIN
    {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedOutputRole,
            &request.role,
            "echo.verifier.unsupported-output-role",
            "the first verifier serves only verifier-report.echo-dpo",
        ));
    }
    if let Some(unsupported) = remaining.first() {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedOutputRole,
            &unsupported.role,
            "echo.verifier.unsupported-output-role",
            "the first verifier serves exactly one verifier-report.echo-dpo output",
        ));
    }
    Ok(())
}

fn validate_core(core: &BoundArtifact) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    if core.artifact.domain != CORE_DOMAIN {
        return Err(invalid_artifact(
            "core.echo-provider",
            "Core domain is invalid",
        ));
    }
    let value = validate_binding(core)
        .map_err(|()| invalid_artifact("core.echo-provider", "Core binding is invalid"))?;
    let api = text_field(&value, "apiVersion")
        .ok_or_else(|| invalid_artifact("core.echo-provider", "Core apiVersion is absent"))?;
    if api != CORE_ABI {
        return Err(refusal(
            ProviderRefusalKind::UnsupportedCoreAbi,
            api,
            "echo.verifier.unsupported-core-abi",
            "the verifier accepts only edict.core/v1",
        ));
    }
    if !has_exact_fields(
        &value,
        &[
            "apiVersion",
            "coordinate",
            "imports",
            "types",
            "intents",
            "requiredCoreCapabilities",
        ],
    ) {
        return Err(unsupported_semantics("core.echo-provider"));
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
    if !matches!(array_field(&value, "imports"), Some(values) if values.is_empty())
        || map_field(&value, "types") != expected_core_types().ok().as_ref()
        || !matches!(array_field(&value, "requiredCoreCapabilities"), Some(values) if values.is_empty())
    {
        return Err(unsupported_semantics(coordinate));
    }
    validate_core_intent(&value)?;
    Ok(value)
}

fn validate_core_intent(core: &CanonicalValueV1) -> Result<(), ProviderRefusalV1> {
    let intents = map_field(core, "intents")
        .and_then(as_map)
        .ok_or_else(|| invalid_artifact("core.echo-provider", "Core intents are invalid"))?;
    let [(key, intent)] = intents.as_slice() else {
        return Err(unsupported_semantics(CORE_COORDINATE));
    };
    if as_text(key) != Some("t")
        || !has_exact_fields(
            intent,
            &[
                "input",
                "output",
                "requiredOperationProfile",
                "inputConstraints",
                "coreEvaluationBudget",
                "body",
            ],
        )
        || text_field(intent, "input") != Some(OPERATION_INPUT_TYPE)
        || text_field(intent, "output") != Some(OPERATION_OUTPUT_TYPE)
        || text_field(intent, "requiredOperationProfile") != Some(OPERATION_PROFILE)
        || !matches!(array_field(intent, "inputConstraints"), Some(values) if values.is_empty())
        || map_field(intent, "coreEvaluationBudget") != expected_core_budget().ok().as_ref()
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let body = map_field(intent, "body")
        .filter(|body| has_exact_fields(body, &["locals", "nodes", "result"]))
        .ok_or_else(|| unsupported_semantics(OPERATION_COORDINATE))?;
    let locals = array_field(body, "locals")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core locals are invalid"))?;
    let expected_input = reviewed_input_local();
    let expected_receipt = reviewed_receipt_local();
    let expected_reason = reviewed_reason_local();
    if locals
        != &[
            expected_input.clone(),
            expected_receipt.clone(),
            expected_reason.clone(),
        ]
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let nodes = array_field(body, "nodes")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core nodes are invalid"))?;
    let [node] = nodes.as_slice() else {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    };
    validate_core_effect(node, &expected_input, &expected_receipt, &expected_reason)?;
    let result = map_field(body, "result")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "Core result is absent"))?;
    validate_supported_expr(
        result,
        &[&expected_input, &expected_receipt],
        &[],
        MAX_EXPRESSION_DEPTH,
    )?;
    if reviewed_expr_shape(
        result,
        &[&expected_input, &expected_receipt],
        MAX_EXPRESSION_DEPTH,
    ) != Some(ReviewedValueShape::RecordLiteralWithId)
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    Ok(())
}

fn validate_core_effect(
    node: &CanonicalValueV1,
    input_local: &CanonicalValueV1,
    receipt_local: &CanonicalValueV1,
    reason_local: &CanonicalValueV1,
) -> Result<(), ProviderRefusalV1> {
    if !has_exact_fields(
        node,
        &["kind", "binding", "effect", "input", "obstructionMap"],
    ) || text_field(node, "kind") != Some("effect")
        || map_field(node, "binding") != Some(receipt_local)
        || text_field(node, "effect") != Some(SEMANTIC_EFFECT)
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let input = map_field(node, "input")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "effect input is absent"))?;
    validate_supported_expr(input, &[input_local], &[], MAX_EXPRESSION_DEPTH)?;
    if reviewed_expr_shape(input, &[input_local], MAX_EXPRESSION_DEPTH)
        != Some(ReviewedValueShape::StringScalar)
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let obstruction_map = map_field(node, "obstructionMap")
        .and_then(as_map)
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "obstruction map is invalid"))?;
    let [(failure, arm)] = obstruction_map.as_slice() else {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    };
    if as_text(failure) != Some(FAILURE_COORDINATE)
        || !has_exact_fields(arm, &["binder", "value"])
        || map_field(arm, "binder") != Some(reason_local)
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    let value = map_field(arm, "value")
        .ok_or_else(|| invalid_artifact(OPERATION_COORDINATE, "obstruction value is absent"))?;
    validate_supported_expr(
        value,
        &[input_local, reason_local],
        &[DOMAIN_OBSTRUCTION],
        MAX_EXPRESSION_DEPTH,
    )?;
    if text_field(value, "callee") != Some(DOMAIN_OBSTRUCTION)
        || !matches!(array_field(value, "typeArgs"), Some(values) if values.is_empty())
        || !matches!(array_field(value, "args"), Some(values) if values.is_empty())
    {
        return Err(unsupported_semantics(OPERATION_COORDINATE));
    }
    Ok(())
}

fn validate_target_ir_artifact(
    target_ir: &BoundArtifact,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    if target_ir.reference.coordinate.is_empty() || target_ir.artifact.domain != TARGET_IR_DOMAIN {
        return Err(invalid_artifact(
            "target-ir.echo-dpo",
            "Target IR identity or outer domain is invalid",
        ));
    }
    let value = validate_binding(target_ir)
        .map_err(|()| invalid_artifact("target-ir.echo-dpo", "Target IR binding is invalid"))?;
    if !validate_target_ir_shape(&value) {
        return Err(invalid_artifact(
            "target-ir.echo-dpo",
            "Target IR structure is invalid",
        ));
    }
    Ok(value)
}

fn validate_target_ir_shape(value: &CanonicalValueV1) -> bool {
    if !has_exact_fields(
        value,
        &[
            "kind",
            "domain",
            "targetProfile",
            "sourceCoreCoordinate",
            "intents",
        ],
    ) || text_field(value, "kind") != Some("targetIrArtifact")
        || text_field(value, "domain").is_none_or(str::is_empty)
        || text_field(value, "sourceCoreCoordinate").is_none_or(str::is_empty)
        || !map_field(value, "targetProfile").is_some_and(validate_resource_ref_value)
    {
        return false;
    }
    map_field(value, "intents")
        .and_then(as_map)
        .is_some_and(|intents| {
            intents.iter().all(|(name, intent)| {
                as_text(name).is_some_and(|name| !name.is_empty())
                    && validate_target_intent_shape(intent)
            })
        })
}

fn validate_target_intent_shape(value: &CanonicalValueV1) -> bool {
    if !has_exact_fields(
        value,
        &[
            "operationProfile",
            "inputConstraints",
            "coreEvaluationBudget",
            "requirements",
            "steps",
            "result",
        ],
    ) || text_field(value, "operationProfile").is_none_or(str::is_empty)
        || array_field(value, "inputConstraints").is_none()
        || !map_field(value, "coreEvaluationBudget").is_some_and(validate_budget_shape)
        || !array_field(value, "requirements")
            .is_some_and(|requirements| requirements.iter().all(validate_requirement_shape))
        || !array_field(value, "steps").is_some_and(|steps| steps.iter().all(validate_step_shape))
    {
        return false;
    }
    map_field(value, "result").is_some_and(validate_target_expr_shape)
}

fn validate_budget_shape(value: &CanonicalValueV1) -> bool {
    has_exact_fields(
        value,
        &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"],
    ) && ["maxSteps", "maxAllocatedBytes", "maxOutputBytes"]
        .into_iter()
        .all(|field| {
            matches!(map_field(value, field), Some(CanonicalValueV1::Integer(value)) if *value >= 0)
        })
}

fn validate_requirement_shape(value: &CanonicalValueV1) -> bool {
    has_exact_fields(value, &["id", "predicate", "onFailure"])
        && text_field(value, "id").is_some_and(|id| !id.is_empty())
        && map_field(value, "predicate").and_then(as_map).is_some()
        && map_field(value, "onFailure").and_then(as_map).is_some()
}

fn validate_step_shape(value: &CanonicalValueV1) -> bool {
    if !has_exact_fields(
        value,
        &[
            "id",
            "binding",
            "effect",
            "targetIntrinsic",
            "input",
            "obstructionFailures",
            "obstructionArms",
        ],
    ) || text_field(value, "id").is_none_or(str::is_empty)
        || !map_field(value, "binding").is_some_and(validate_local)
        || text_field(value, "effect").is_none_or(str::is_empty)
        || text_field(value, "targetIntrinsic").is_none_or(str::is_empty)
        || !map_field(value, "input").is_some_and(validate_target_expr_shape)
        || !array_field(value, "obstructionFailures").is_some_and(|failures| {
            failures
                .iter()
                .all(|failure| as_text(failure).is_some_and(|failure| !failure.is_empty()))
        })
    {
        return false;
    }
    map_field(value, "obstructionArms")
        .and_then(as_map)
        .is_some_and(|arms| {
            arms.iter().all(|(failure, arm)| {
                as_text(failure).is_some_and(|failure| !failure.is_empty())
                    && has_exact_fields(arm, &["binder", "value"])
                    && map_field(arm, "binder").is_some_and(validate_local)
                    && map_field(arm, "value").is_some_and(validate_target_expr_shape)
            })
        })
}

fn validate_target_expr_shape(value: &CanonicalValueV1) -> bool {
    matches!(
        preflight_expr(value, MAX_EXPRESSION_DEPTH),
        Ok(()) | Err(ExpressionPreflightError::Unsupported)
    )
}

fn expected_target_ir(
    core: &CanonicalValueV1,
    target_profile_digest: &Digest,
) -> Result<CanonicalValueV1, ()> {
    let intent = map_field(map_field(core, "intents").ok_or(())?, "t").ok_or(())?;
    let body = map_field(intent, "body").ok_or(())?;
    let nodes = array_field(body, "nodes").ok_or(())?;
    let [node] = nodes.as_slice() else {
        return Err(());
    };
    let obstruction_map = map_field(node, "obstructionMap").ok_or(())?;
    let expected_step = canonical_sorted_map([
        ("id", canonical_text("t.step.0")),
        ("binding", map_field(node, "binding").ok_or(())?.clone()),
        ("effect", canonical_text(SEMANTIC_EFFECT)),
        ("targetIntrinsic", canonical_text(TARGET_INTRINSIC)),
        ("input", map_field(node, "input").ok_or(())?.clone()),
        (
            "obstructionFailures",
            CanonicalValueV1::Array(vec![canonical_text(FAILURE_COORDINATE)]),
        ),
        ("obstructionArms", obstruction_map.clone()),
    ])?;
    let expected_intent = canonical_sorted_map([
        ("operationProfile", canonical_text(OPERATION_PROFILE)),
        (
            "inputConstraints",
            map_field(intent, "inputConstraints").ok_or(())?.clone(),
        ),
        (
            "coreEvaluationBudget",
            map_field(intent, "coreEvaluationBudget").ok_or(())?.clone(),
        ),
        ("requirements", CanonicalValueV1::Array(Vec::new())),
        ("steps", CanonicalValueV1::Array(vec![expected_step])),
        ("result", map_field(body, "result").ok_or(())?.clone()),
    ])?;
    canonical_sorted_map([
        ("kind", canonical_text("targetIrArtifact")),
        ("domain", canonical_text(INNER_TARGET_IR_DOMAIN)),
        (
            "targetProfile",
            canonical_sorted_map([
                ("id", canonical_text(TARGET_PROFILE_COORDINATE)),
                ("digest", digest_value(target_profile_digest).ok_or(())?),
            ])?,
        ),
        ("sourceCoreCoordinate", canonical_text(CORE_COORDINATE)),
        ("intents", canonical_sorted_map([("t", expected_intent)])?),
    ])
}

struct RelationFailure {
    code: &'static str,
    message: &'static str,
}

fn relation_failure(
    target_ir: &CanonicalValueV1,
    expected: &CanonicalValueV1,
) -> Option<RelationFailure> {
    if text_field(target_ir, "domain") != text_field(expected, "domain") {
        return Some(RelationFailure {
            code: "echo.verifier.target-domain-mismatch",
            message: "Target IR names a domain outside the reviewed Echo target",
        });
    }
    if map_field(target_ir, "targetProfile") != map_field(expected, "targetProfile") {
        return Some(RelationFailure {
            code: "echo.verifier.target-profile-mismatch",
            message: "Target IR does not bind the exact supplied Echo target profile",
        });
    }

    let target_intents = map_field(target_ir, "intents").and_then(as_map);
    let expected_intents = map_field(expected, "intents").and_then(as_map);
    match (target_intents, expected_intents) {
        (Some(target), Some(expected)) if target.len() > expected.len() => {
            return Some(RelationFailure {
                code: "echo.verifier.introduced-claim",
                message: "Target IR introduces an operation absent from Core",
            });
        }
        (Some(target), Some(expected)) if target.len() < expected.len() => {
            return Some(RelationFailure {
                code: "echo.verifier.silent-loss",
                message: "Target IR drops an operation required by Core",
            });
        }
        _ => {}
    }

    let target_intent = map_field(target_ir, "intents").and_then(|value| map_field(value, "t"));
    let expected_intent = map_field(expected, "intents").and_then(|value| map_field(value, "t"));
    let (Some(target_intent), Some(expected_intent)) = (target_intent, expected_intent) else {
        return Some(RelationFailure {
            code: "echo.verifier.silent-loss",
            message: "Target IR omits the reviewed Core operation",
        });
    };

    let target_requirements = array_field(target_intent, "requirements");
    let expected_requirements = array_field(expected_intent, "requirements");
    match (target_requirements, expected_requirements) {
        (Some(target), Some(expected)) if target.len() > expected.len() => {
            return Some(RelationFailure {
                code: "echo.verifier.introduced-claim",
                message: "Target IR introduces a guard absent from Core",
            });
        }
        (Some(target), Some(expected)) if target.len() < expected.len() => {
            return Some(RelationFailure {
                code: "echo.verifier.silent-loss",
                message: "Target IR drops a guard required by Core",
            });
        }
        _ => {}
    }

    if map_field(target_intent, "coreEvaluationBudget")
        != map_field(expected_intent, "coreEvaluationBudget")
    {
        return Some(RelationFailure {
            code: "echo.verifier.budget-mismatch",
            message: "Target IR does not preserve the exact Core evaluation budget",
        });
    }

    let target_steps = array_field(target_intent, "steps");
    let expected_steps = array_field(expected_intent, "steps");
    match (target_steps, expected_steps) {
        (Some(target), Some(expected)) if target.len() > expected.len() => {
            return Some(RelationFailure {
                code: "echo.verifier.introduced-claim",
                message: "Target IR introduces an effect absent from Core",
            });
        }
        (Some(target), Some(expected)) if target.len() < expected.len() => {
            return Some(RelationFailure {
                code: "echo.verifier.silent-loss",
                message: "Target IR drops an effect required by Core",
            });
        }
        _ => {}
    }
    let target_step = target_steps.and_then(|steps| steps.first());
    let expected_step = expected_steps.and_then(|steps| steps.first());
    let (Some(target_step), Some(expected_step)) = (target_step, expected_step) else {
        return Some(RelationFailure {
            code: "echo.verifier.silent-loss",
            message: "Target IR omits the reviewed Core effect",
        });
    };

    if text_field(target_step, "targetIntrinsic") != Some(TARGET_INTRINSIC) {
        return Some(RelationFailure {
            code: "echo.verifier.target-intrinsic-mismatch",
            message: "Target IR uses an intrinsic outside the reviewed Echo capability",
        });
    }
    if map_field(target_step, "input") != map_field(expected_step, "input") {
        return Some(RelationFailure {
            code: "echo.verifier.effect-input-mismatch",
            message: "Target IR does not preserve the exact Core effect input",
        });
    }
    if map_field(target_step, "obstructionFailures")
        != map_field(expected_step, "obstructionFailures")
        || map_field(target_step, "obstructionArms") != map_field(expected_step, "obstructionArms")
    {
        return Some(RelationFailure {
            code: "echo.verifier.obstruction-mismatch",
            message: "Target IR does not preserve the exact Core obstruction relation",
        });
    }
    if map_field(target_intent, "result") != map_field(expected_intent, "result") {
        return Some(RelationFailure {
            code: "echo.verifier.result-mismatch",
            message: "Target IR does not preserve the exact Core result expression",
        });
    }
    (target_ir != expected).then_some(RelationFailure {
        code: "echo.verifier.semantic-relation-mismatch",
        message: "Target IR does not exactly preserve the reviewed Core obligations",
    })
}

fn build_report(target_ir: &ResourceRef, outcome: &str) -> Result<Vec<u8>, ()> {
    let report = canonical_sorted_map([
        ("apiVersion", canonical_text(REPORT_DOMAIN)),
        ("targetIr", resource_ref_value(target_ir)?),
        ("outcome", canonical_text(outcome)),
        (
            "diagnosticAbi",
            resource_ref_value(&ResourceRef {
                coordinate: "edict.diagnostics/v1".to_owned(),
                digest: Digest {
                    algorithm: DigestAlgorithm::Sha256,
                    bytes: DIAGNOSTIC_ABI_DIGEST.to_vec(),
                },
            })?,
        ),
        ("diagnosticBytes", CanonicalValueV1::Bytes(Vec::new())),
    ])?;
    encode_canonical_cbor_v1(&report).map_err(|_| ())
}

fn resource_ref_value(reference: &ResourceRef) -> Result<CanonicalValueV1, ()> {
    canonical_sorted_map([
        ("id", canonical_text(&reference.coordinate)),
        ("digest", digest_value(&reference.digest).ok_or(())?),
    ])
}

fn digest_value(digest: &Digest) -> Option<CanonicalValueV1> {
    (digest.algorithm == DigestAlgorithm::Sha256 && digest.bytes.len() == 32).then(|| {
        CanonicalValueV1::Array(vec![
            canonical_text("sha256"),
            CanonicalValueV1::Bytes(digest.bytes.clone()),
        ])
    })
}

fn validate_resource_ref_value(value: &CanonicalValueV1) -> bool {
    has_exact_fields(value, &["id", "digest"])
        && text_field(value, "id").is_some_and(|id| !id.is_empty())
        && map_field(value, "digest").is_some_and(|digest| {
            matches!(
                digest,
                CanonicalValueV1::Array(values)
                    if matches!(values.as_slice(),
                        [CanonicalValueV1::Text(algorithm), CanonicalValueV1::Bytes(bytes)]
                            if algorithm == "sha256" && bytes.len() == 32)
            )
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
    let actual = digest_canonical_value_v1(&bound.artifact.domain, &value).map_err(|_| ())?;
    if actual != digest_review(&bound.reference.digest).ok_or(())? {
        return Err(());
    }
    Ok(value)
}

fn digest_review(digest: &Digest) -> Option<String> {
    if digest.algorithm != DigestAlgorithm::Sha256 || digest.bytes.len() != 32 {
        return None;
    }
    let mut value = String::with_capacity(71);
    value.push_str("sha256:");
    for byte in &digest.bytes {
        write!(value, "{byte:02x}").ok()?;
    }
    Some(value)
}

fn expected_lowerability() -> Result<CanonicalValueV1, ()> {
    let guard_kinds = || CanonicalValueV1::Array(vec![canonical_text("precommit-atomic")]);
    let obstructions = || CanonicalValueV1::Array(vec![canonical_text(FAILURE_COORDINATE)]);
    let footprints = || CanonicalValueV1::Array(vec![canonical_text("target.replace.footprint")]);
    let costs = || CanonicalValueV1::Array(vec![canonical_text("target.replace.cost")]);
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
                ("footprintObligations", footprints()),
                ("costObligations", costs()),
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
        ("footprintObligations", footprints()),
        ("costObligations", costs()),
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

fn expected_core_budget() -> Result<CanonicalValueV1, ()> {
    canonical_sorted_map([
        ("maxSteps", CanonicalValueV1::Integer(8)),
        ("maxAllocatedBytes", CanonicalValueV1::Integer(1024)),
        ("maxOutputBytes", CanonicalValueV1::Integer(256)),
    ])
}

fn reviewed_input_local() -> CanonicalValueV1 {
    local("arg.0", "$arg0", OPERATION_INPUT_TYPE)
}

fn reviewed_receipt_local() -> CanonicalValueV1 {
    local("local.0", "$local0", OPERATION_RECEIPT_TYPE)
}

fn reviewed_reason_local() -> CanonicalValueV1 {
    local("obstruction.0", "$obstruction0", FAILURE_PAYLOAD_TYPE)
}

fn local(id: &str, alpha_name: &str, ty: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Map(vec![
        (canonical_text("id"), canonical_text(id)),
        (canonical_text("type"), canonical_text(ty)),
        (canonical_text("alphaName"), canonical_text(alpha_name)),
    ])
}

#[derive(Clone, Copy)]
enum ExpressionPreflightError {
    InvalidDiscriminator,
    InvalidShape,
    Unsupported,
    DepthExceeded,
}

fn validate_supported_expr(
    value: &CanonicalValueV1,
    scope: &[&CanonicalValueV1],
    allowed_callees: &[&str],
    remaining_depth: usize,
) -> Result<(), ProviderRefusalV1> {
    preflight_expr(value, remaining_depth).map_err(expression_preflight_refusal)?;
    if validate_expr(value, scope, allowed_callees, remaining_depth) {
        Ok(())
    } else {
        Err(unsupported_semantics(OPERATION_COORDINATE))
    }
}

fn preflight_expr(
    value: &CanonicalValueV1,
    remaining_depth: usize,
) -> Result<(), ExpressionPreflightError> {
    if remaining_depth == 0 {
        return Err(ExpressionPreflightError::DepthExceeded);
    }
    let kind = text_field(value, "kind").ok_or(ExpressionPreflightError::InvalidDiscriminator)?;
    match kind {
        "local" => {
            if has_exact_fields(value, &["kind", "ref"])
                && map_field(value, "ref").is_some_and(validate_local)
            {
                Ok(())
            } else {
                Err(ExpressionPreflightError::InvalidShape)
            }
        }
        "const" => {
            if !has_exact_fields(value, &["kind", "value"]) {
                return Err(ExpressionPreflightError::InvalidShape);
            }
            preflight_core_value(
                map_field(value, "value").ok_or(ExpressionPreflightError::InvalidShape)?,
            )
        }
        "record" => {
            if !has_exact_fields(value, &["kind", "fields"]) {
                return Err(ExpressionPreflightError::InvalidShape);
            }
            let fields = map_field(value, "fields")
                .and_then(as_map)
                .ok_or(ExpressionPreflightError::InvalidShape)?;
            for (key, field) in fields {
                if as_text(key).is_none_or(str::is_empty) {
                    return Err(ExpressionPreflightError::InvalidShape);
                }
                preflight_expr(field, remaining_depth - 1)?;
            }
            Ok(())
        }
        "field" => {
            if !has_exact_fields(value, &["kind", "base", "field"])
                || text_field(value, "field").is_none_or(str::is_empty)
            {
                return Err(ExpressionPreflightError::InvalidShape);
            }
            preflight_expr(
                map_field(value, "base").ok_or(ExpressionPreflightError::InvalidShape)?,
                remaining_depth - 1,
            )
        }
        "call" => {
            if !has_exact_fields(value, &["kind", "callee", "typeArgs", "args"])
                || text_field(value, "callee").is_none_or(str::is_empty)
            {
                return Err(ExpressionPreflightError::InvalidShape);
            }
            let type_args =
                array_field(value, "typeArgs").ok_or(ExpressionPreflightError::InvalidShape)?;
            if !type_args
                .iter()
                .all(|argument| as_text(argument).is_some_and(|argument| !argument.is_empty()))
            {
                return Err(ExpressionPreflightError::InvalidShape);
            }
            let args = array_field(value, "args").ok_or(ExpressionPreflightError::InvalidShape)?;
            for argument in args {
                preflight_expr(argument, remaining_depth - 1)?;
            }
            Ok(())
        }
        "variant" | "match" | "list" | "map" | "if" => Err(ExpressionPreflightError::Unsupported),
        _ => Err(ExpressionPreflightError::InvalidDiscriminator),
    }
}

fn preflight_core_value(value: &CanonicalValueV1) -> Result<(), ExpressionPreflightError> {
    let kind = text_field(value, "kind").ok_or(ExpressionPreflightError::InvalidDiscriminator)?;
    match kind {
        "string"
            if has_exact_fields(value, &["kind", "value"])
                && matches!(map_field(value, "value"), Some(CanonicalValueV1::Text(_))) =>
        {
            Ok(())
        }
        "string" => Err(ExpressionPreflightError::InvalidShape),
        "null" | "bool" | "int" | "bytes" | "record" | "variant" | "list" | "map"
        | "capability" => Err(ExpressionPreflightError::Unsupported),
        _ => Err(ExpressionPreflightError::InvalidDiscriminator),
    }
}

fn expression_preflight_refusal(error: ExpressionPreflightError) -> ProviderRefusalV1 {
    match error {
        ExpressionPreflightError::InvalidDiscriminator => refusal(
            ProviderRefusalKind::InvalidSemanticArtifact,
            OPERATION_COORDINATE,
            "echo.verifier.invalid-expression-discriminator",
            "Core expression has a missing or unknown kind discriminator",
        ),
        ExpressionPreflightError::InvalidShape => refusal(
            ProviderRefusalKind::InvalidSemanticArtifact,
            OPERATION_COORDINATE,
            "echo.verifier.invalid-expression-shape",
            "Core expression does not match the selected kind shape",
        ),
        ExpressionPreflightError::DepthExceeded => refusal(
            ProviderRefusalKind::InvalidSemanticArtifact,
            OPERATION_COORDINATE,
            "echo.verifier.expression-depth-exceeded",
            "Core expression exceeds the verifier recursion bound",
        ),
        ExpressionPreflightError::Unsupported => unsupported_semantics(OPERATION_COORDINATE),
    }
}

fn validate_expr(
    value: &CanonicalValueV1,
    scope: &[&CanonicalValueV1],
    allowed_callees: &[&str],
    remaining_depth: usize,
) -> bool {
    if remaining_depth == 0 {
        return false;
    }
    match text_field(value, "kind") {
        Some("local") => {
            has_exact_fields(value, &["kind", "ref"])
                && map_field(value, "ref").is_some_and(|reference| {
                    validate_local(reference) && scope.contains(&reference)
                })
        }
        Some("const") => {
            has_exact_fields(value, &["kind", "value"])
                && map_field(value, "value")
                    .is_some_and(|inner| validate_core_value(inner, remaining_depth - 1))
        }
        Some("record") => {
            if !has_exact_fields(value, &["kind", "fields"]) {
                return false;
            }
            map_field(value, "fields")
                .and_then(as_map)
                .is_some_and(|fields| {
                    fields.iter().all(|(key, field)| {
                        as_text(key).is_some_and(|key| !key.is_empty())
                            && validate_expr(field, scope, allowed_callees, remaining_depth - 1)
                    })
                })
        }
        Some("field") => {
            has_exact_fields(value, &["kind", "base", "field"])
                && text_field(value, "field").is_some_and(|field| !field.is_empty())
                && map_field(value, "base").is_some_and(|base| {
                    validate_expr(base, scope, allowed_callees, remaining_depth - 1)
                })
        }
        Some("call") => {
            has_exact_fields(value, &["kind", "callee", "typeArgs", "args"])
                && text_field(value, "callee")
                    .is_some_and(|callee| allowed_callees.contains(&callee))
                && matches!(array_field(value, "typeArgs"), Some(values) if values.is_empty())
                && matches!(array_field(value, "args"), Some(values) if values.is_empty())
        }
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReviewedValueShape {
    StringScalar,
    InputRecord,
    OutputRecord,
    ReceiptRecord,
    RecordLiteralWithId,
}

fn reviewed_expr_shape(
    value: &CanonicalValueV1,
    scope: &[&CanonicalValueV1],
    remaining_depth: usize,
) -> Option<ReviewedValueShape> {
    if remaining_depth == 0 {
        return None;
    }
    match text_field(value, "kind")? {
        "local" => {
            let reference = map_field(value, "ref")?;
            if !scope.contains(&reference) {
                return None;
            }
            match text_field(reference, "type")? {
                OPERATION_INPUT_TYPE => Some(ReviewedValueShape::InputRecord),
                OPERATION_OUTPUT_TYPE => Some(ReviewedValueShape::OutputRecord),
                OPERATION_RECEIPT_TYPE => Some(ReviewedValueShape::ReceiptRecord),
                INPUT_ID_TYPE | OUTPUT_ID_TYPE | RECEIPT_ID_TYPE => {
                    Some(ReviewedValueShape::StringScalar)
                }
                _ => None,
            }
        }
        "const"
            if matches!(
                map_field(value, "value").and_then(|value| text_field(value, "kind")),
                Some("string")
            ) =>
        {
            Some(ReviewedValueShape::StringScalar)
        }
        "field" => {
            let base = map_field(value, "base")?;
            let base_shape = reviewed_expr_shape(base, scope, remaining_depth - 1);
            (text_field(value, "field") == Some("id")
                && matches!(
                    base_shape,
                    Some(
                        ReviewedValueShape::InputRecord
                            | ReviewedValueShape::OutputRecord
                            | ReviewedValueShape::ReceiptRecord
                            | ReviewedValueShape::RecordLiteralWithId
                    )
                ))
            .then_some(ReviewedValueShape::StringScalar)
        }
        "record" => {
            let fields = map_field(value, "fields").and_then(as_map)?;
            let [(field, expression)] = fields.as_slice() else {
                return None;
            };
            (as_text(field) == Some("id")
                && reviewed_expr_shape(expression, scope, remaining_depth - 1)
                    == Some(ReviewedValueShape::StringScalar))
            .then_some(ReviewedValueShape::RecordLiteralWithId)
        }
        _ => None,
    }
}

fn validate_core_value(value: &CanonicalValueV1, remaining_depth: usize) -> bool {
    remaining_depth > 0
        && match text_field(value, "kind") {
            Some("string") => {
                has_exact_fields(value, &["kind", "value"])
                    && map_field(value, "value").is_some_and(
                        |value| matches!(value, CanonicalValueV1::Text(value) if value.chars().count() <= 16),
                    )
            }
            _ => false,
        }
}

fn validate_local(value: &CanonicalValueV1) -> bool {
    has_exact_fields(value, &["id", "alphaName", "type"])
        && text_field(value, "id").is_some_and(|value| !value.is_empty())
        && text_field(value, "alphaName").is_some_and(|value| !value.is_empty())
        && text_field(value, "type").is_some_and(|value| !value.is_empty())
}

fn has_exact_fields(value: &CanonicalValueV1, fields: &[&str]) -> bool {
    as_map(value).is_some_and(|entries| {
        entries.len() == fields.len()
            && fields.iter().all(|field| map_field(value, field).is_some())
    })
}

fn canonical_sorted_map<'a>(
    entries: impl IntoIterator<Item = (&'a str, CanonicalValueV1)>,
) -> Result<CanonicalValueV1, ()> {
    let mut entries = entries
        .into_iter()
        .map(|(key, value)| {
            let key = canonical_text(key);
            let encoded = encode_canonical_cbor_v1(&key).map_err(|_| ())?;
            Ok((encoded, key, value))
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
        "echo.verifier.invalid-semantic-artifact",
        message,
    )
}

fn unsupported_semantics(subject: &str) -> ProviderRefusalV1 {
    refusal(
        ProviderRefusalKind::UnsupportedSemantics,
        subject,
        "echo.verifier.unsupported-semantics",
        "the semantic closure is outside the first reviewed Echo verifier boundary",
    )
}
