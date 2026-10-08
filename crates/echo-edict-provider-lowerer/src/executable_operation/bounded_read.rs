// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Data-only compilation of explicitly selected bounded atom reads.

mod relation;

use super::{
    as_map, canonical_map, canonical_text, encode_canonical_cbor_v1, encode_compiler_package,
    invalid_artifact, map_field, require_exact_fields, require_resource_ref, required_array,
    required_nonempty_text, required_u64, single_text_map_entry, text_field, validate_bound,
    validate_compiler_lawpack, validate_source, Artifact, BoundArtifact, CanonicalValueV1 as Value,
    ClosureInputs, CompilerPackageProfile, LoweringOutputArtifact, LoweringOutputKind,
    LoweringRequestV1, LoweringSuccessV1, ProviderRefusalV1, ADAPTER_DOMAIN, CONFIGURATION_ABI,
    CONFIGURATION_DOMAIN, CORE_ABI, CORE_DOMAIN, EXPORTS_DOMAIN, LAWPACK_DOMAIN, PACKAGE_DOMAIN,
    PACKAGE_ROLE, RESULT_PROJECTION_DOMAIN, SOURCE_DOMAIN, TARGET_IR_DOMAIN,
};

const SUBJECT: &str = "compiler-produced-bounded-read";
const KIND: &str = "compiler-produced-bounded-read/v1";
const ORDERED_IR: &str = "echo.span-ir/v2";
const READ_PROFILE: &str = "continuum.profile.read-only/v1";
const MAX_READS: u64 = 65_536;
const MAX_READ_BYTES: u64 = 64 * 1024 * 1024;
const MAX_BOUND_TEXT_BYTES: usize = 1024;

pub(super) fn is_configuration(value: &Value) -> bool {
    text_field(value, "apiVersion") == Some(CONFIGURATION_ABI)
        && text_field(value, "programKind") == Some(KIND)
}

fn invalid() -> ProviderRefusalV1 {
    invalid_artifact(
        SUBJECT,
        "bounded read Core, target, type, or closure relation is invalid",
    )
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, ProviderRefusalV1> {
    map_field(value, key).ok_or_else(invalid)
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, ProviderRefusalV1> {
    let text = required_nonempty_text(value, key, SUBJECT)?;
    if text.len() > MAX_BOUND_TEXT_BYTES {
        return Err(invalid());
    }
    Ok(text)
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], ProviderRefusalV1> {
    Ok(required_array(value, key, SUBJECT)?.as_slice())
}

pub(super) fn lower(
    request: &LoweringRequestV1,
    closure: &ClosureInputs<'_>,
) -> Result<LoweringSuccessV1, ProviderRefusalV1> {
    let core = validate_bound(&request.core, CORE_DOMAIN)?;
    let target = validate_bound(&closure.target_ir.artifact, TARGET_IR_DOMAIN)?;
    let exports = validate_bound(&closure.exports.artifact, EXPORTS_DOMAIN)?;
    let lawpack = validate_bound(&closure.lawpack.artifact, LAWPACK_DOMAIN)?;
    let adapter = validate_bound(&closure.adapter.artifact, ADAPTER_DOMAIN)?;
    let configuration = validate_bound(&closure.configuration.artifact, CONFIGURATION_DOMAIN)?;
    let projection = validate_bound(
        &closure.result_projection.artifact,
        RESULT_PROJECTION_DOMAIN,
    )?;
    let source = validate_bound(&closure.source.artifact, SOURCE_DOMAIN)?;
    let source = validate_source(&source, closure.source, &request.core)?;
    let mut core_fields = vec![
        "apiVersion",
        "coordinate",
        "imports",
        "types",
        "intents",
        "requiredCoreCapabilities",
    ];
    if map_field(&core, "functions").is_some() {
        core_fields.push("functions");
    }
    require_exact_fields(&core, &core_fields, SUBJECT)?;
    super::source_functions::validate(&core, &exports, true)?;
    require_exact_fields(
        &configuration,
        &["apiVersion", "programKind", "maxReads", "maxReadBytes"],
        SUBJECT,
    )?;
    let max_reads = required_u64(&configuration, "maxReads", SUBJECT)?;
    let max_bytes = required_u64(&configuration, "maxReadBytes", SUBJECT)?;
    if !(1..=MAX_READS).contains(&max_reads) || !(1..=MAX_READ_BYTES).contains(&max_bytes) {
        return Err(invalid());
    }
    if text(&core, "apiVersion")? != CORE_ABI
        || text(&core, "coordinate")? != request.core.reference.coordinate
        || !array(&core, "requiredCoreCapabilities")?.is_empty()
    {
        return Err(invalid());
    }
    let (name, intent) = single_text_map_entry(field(&core, "intents")?).ok_or_else(invalid)?;
    let coordinate = format!("{}.{}", request.core.reference.coordinate, name);
    require_exact_fields(
        intent,
        &[
            "input",
            "output",
            "body",
            "basis",
            "inputConstraints",
            "coreEvaluationBudget",
            "requiredOperationProfile",
        ],
        SUBJECT,
    )?;
    if text(intent, "requiredOperationProfile")? != READ_PROFILE {
        return Err(invalid());
    }
    validate_compiler_lawpack(
        &lawpack,
        &exports,
        &adapter,
        closure,
        &request.target_profile,
        intent,
    )?;
    validate_closure(&core, &target, request, closure)?;
    let (target_name, target_intent) =
        single_text_map_entry(field(&target, "intents")?).ok_or_else(invalid)?;
    if target_name != name {
        return Err(invalid());
    }
    let context = relation::ReadRelation {
        core: &core,
        exports: &exports,
        adapter: &adapter,
        source,
        closure,
        max_reads,
        max_bytes,
    };
    context.validate(intent, target_intent, &projection, &coordinate)?;
    let program = canonical_map([
        (
            "schema",
            canonical_text("echo.compiler-produced-read-program/v1"),
        ),
        ("kind", canonical_text(KIND)),
        ("intent", canonical_text(name)),
        (
            "core_artifact",
            Value::Bytes(request.core.artifact.bytes.clone()),
        ),
        ("target_ir_artifact", bytes(&closure.target_ir.artifact)),
        ("lawpack_exports_artifact", bytes(&closure.exports.artifact)),
        ("lawpack_artifact", bytes(&closure.lawpack.artifact)),
        ("lawpack_adapter_artifact", bytes(&closure.adapter.artifact)),
        (
            "target_configuration_artifact",
            bytes(&closure.configuration.artifact),
        ),
        (
            "result_projection_artifact",
            bytes(&closure.result_projection.artifact),
        ),
        ("source_artifact", bytes(&closure.source.artifact)),
    ]);
    let program = encode_canonical_cbor_v1(&program).map_err(|_| invalid())?;
    let bytes = encode_compiler_package(
        request,
        closure,
        intent,
        &coordinate,
        program,
        CompilerPackageProfile {
            kind: KIND,
            authority: "echo.operation.authority.explicit-read-view/v1",
            footprint: "echo.operation.footprint.explicit-node-read-aperture/v1",
            interpreter: "echo.operation-interpreter.compiler-produced-bounded-read/v1",
        },
    )?;
    let response_bytes = [PACKAGE_ROLE.len(), PACKAGE_DOMAIN.len(), bytes.len()]
        .into_iter()
        .try_fold(0_u64, |sum, size| {
            sum.checked_add(u64::try_from(size).ok()?)
        })
        .ok_or_else(|| invalid_artifact(SUBJECT, "read response size overflow"))?;
    if request.limits.max_output_count == 0
        || response_bytes > request.limits.max_total_response_bytes
    {
        return Err(invalid_artifact(
            SUBJECT,
            "read package exceeds host response budget",
        ));
    }
    Ok(LoweringSuccessV1 {
        outputs: vec![LoweringOutputArtifact {
            role: PACKAGE_ROLE.to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            artifact: Artifact {
                domain: PACKAGE_DOMAIN.to_owned(),
                bytes,
            },
            logical_path: None,
        }],
        diagnostics: Vec::new(),
    })
}

fn bytes(artifact: &BoundArtifact) -> Value {
    Value::Bytes(artifact.artifact.bytes.clone())
}

fn validate_closure(
    core: &Value,
    target: &Value,
    request: &LoweringRequestV1,
    closure: &ClosureInputs<'_>,
) -> Result<(), ProviderRefusalV1> {
    require_exact_fields(
        target,
        &[
            "kind",
            "domain",
            "intents",
            "targetProfile",
            "semanticClosure",
            "sourceCoreCoordinate",
        ],
        SUBJECT,
    )?;
    if closure.target_ir.artifact.reference.coordinate != ORDERED_IR
        || text(target, "domain")? != ORDERED_IR
        || text(target, "kind")? != "orderedTargetIrArtifact"
        || text(target, "sourceCoreCoordinate")? != request.core.reference.coordinate
    {
        return Err(invalid());
    }
    require_resource_ref(
        field(target, "targetProfile")?,
        &request.target_profile,
        SUBJECT,
    )?;
    let semantic = field(target, "semanticClosure")?;
    require_resource_ref(field(semantic, "sourceCore")?, &request.core, SUBJECT)?;
    let [lawpack] = array(semantic, "lawpacks")? else {
        return Err(invalid());
    };
    require_resource_ref(lawpack, &closure.lawpack.artifact, SUBJECT)?;
    let [import] = array(core, "imports")? else {
        return Err(invalid());
    };
    if text(import, "kind")? != "lawpack" {
        return Err(invalid());
    }
    require_resource_ref(field(import, "ref")?, &closure.lawpack.artifact, SUBJECT)
}
