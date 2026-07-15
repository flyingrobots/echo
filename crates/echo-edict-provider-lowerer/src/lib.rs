// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure Echo lowering for Edict's frozen target-provider component boundary.
//!
//! This crate translates only explicit, digest-bound canonical artifacts. It
//! performs no discovery or I/O and grants no Echo runtime authority.

#![deny(unsafe_code)]

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1, CanonicalValueV1,
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
const OUTPUT_DOMAIN: &str = "edict.target-ir.artifact/v1";
const INNER_TARGET_IR_DOMAIN: &str = "echo.span-ir/v1";
const TARGET_IR_ROLE: &str = "target-ir.echo-dpo";
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
/// The function emits no authoritative digest or runtime authority. A future
/// Edict host remains responsible for validating the returned bytes against the
/// owning Target IR schema and computing their authoritative digest.
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

    let outputs = if request.requested_outputs.is_empty() {
        Vec::new()
    } else {
        let bytes = encode_canonical_cbor_v1(&target_ir).map_err(|_| {
            refusal(
                ProviderRefusalKind::InvalidSemanticArtifact,
                TARGET_IR_ROLE,
                "echo.provider.target-ir-encoding",
                "the lowered Target IR could not be canonically encoded",
            )
        })?;
        vec![LoweringOutputArtifact {
            role: TARGET_IR_ROLE.to_owned(),
            kind: LoweringOutputKind::TargetIr,
            artifact: Artifact {
                domain: OUTPUT_DOMAIN.to_owned(),
                bytes,
            },
            logical_path: None,
        }]
    };

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
    match requests {
        [] => Ok(()),
        [request]
            if request.role == TARGET_IR_ROLE
                && request.kind == LoweringOutputKind::TargetIr
                && request.domain == OUTPUT_DOMAIN =>
        {
            Ok(())
        }
        [request, remaining @ ..] => {
            let unsupported = if request.role == TARGET_IR_ROLE
                && request.kind == LoweringOutputKind::TargetIr
                && request.domain == OUTPUT_DOMAIN
            {
                remaining.first().unwrap_or(request)
            } else {
                request
            };
            Err(refusal(
                ProviderRefusalKind::UnsupportedOutputRole,
                &unsupported.role,
                "echo.provider.unsupported-output-role",
                "the first lowerer serves only target-ir.echo-dpo",
            ))
        }
    }
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
        ("domain", canonical_text(INNER_TARGET_IR_DOMAIN)),
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
