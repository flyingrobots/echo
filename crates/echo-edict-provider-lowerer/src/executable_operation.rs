// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic data-only lowering into Echo's bounded executable-operation profile.

use std::collections::BTreeSet;

use blake3::Hasher;
use echo_edict_canonical::{
    digest_canonical_value_bytes_v1, encode_canonical_cbor_v1, CanonicalValueV1,
};

use super::{
    array_field, as_map, as_text, canonical_map, canonical_text, invalid_artifact, map_field,
    text_field, validate_binding, Artifact, BoundArtifact, LoweringOutputArtifact,
    LoweringOutputKind, LoweringRequestV1, LoweringSuccessV1, ProviderRefusalKind,
    ProviderRefusalV1, SemanticInput, SemanticInputKind,
};

const CORE_DOMAIN: &str = "edict.core.module/v1";
const CORE_ABI: &str = "edict.core/v1";
const SOURCE_DOMAIN: &str = "edict.source/v1";
const LAWPACK_DOMAIN: &str = "edict.lawpack/v1";
const EXPORTS_DOMAIN: &str = "edict.lawpack-exports/v1";
const ADAPTER_DOMAIN: &str = "edict.lawpack-adapter/v1";
const ADAPTER_ABI: &str = "edict.lawpack-adapter/v1";
const CONFIGURATION_DOMAIN: &str = "echo.operation-lowering-configuration/v1";
const CONFIGURATION_ABI: &str = "echo.operation-lowering-configuration/v1";
const TARGET_IR_DOMAIN: &str = "edict.target-ir.artifact/v1";
const TARGET_IR_COORDINATE: &str = "echo.span-ir/v1";
const TARGET_IR_PAYLOAD_DOMAIN: &str = "echo.span-ir/v1";
const RESULT_PROJECTION_DOMAIN: &str = "edict.result-projection.artifact/v1";
const RESULT_PROJECTION_ABI: &str = "edict.result-projection/v1";
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const PACKAGE_ROLE: &str = "executable-operation-package.echo";

const TARGET_INTRINSIC: &str = "echo.dpo@1.anchored-node-attachment-create-if-absent";
const OPERATION_PROFILE: &str = "continuum.profile.create/v1";
const PROGRAM_KIND: &str = "anchored-node-attachment-create-if-absent/v1";
const PROGRAM_SCHEMA: &str = "echo.operation-program/v1";
const PACKAGE_SCHEMA: &str = "echo.operation-package/v1";
const INTERPRETER_PROFILE: &str = "echo.operation-interpreter/v1";
const INTRINSIC_PROFILE: &str = "echo.operation-attachment-algebra/v1";
const INPUT_SCHEMA: &str = "echo.operation.input.anchored-node-alpha-create-if-absent/v1";
const RESULT_SCHEMA: &str = "echo.operation.result.anchored-node-alpha-create-if-absent/v1";
const OBSTRUCTION_SCHEMA: &str =
    "echo.operation.obstruction.anchored-node-alpha-create-if-absent/v1";
const RESULT_INTERPRETATION: &str =
    "echo.operation.result-interpretation.anchored-node-alpha-create-if-absent/v1";
const OBSTRUCTION_INTERPRETATION: &str =
    "echo.operation.obstruction-interpretation.anchored-node-alpha-create-if-absent/v1";
const APPLICATION_BASIS_SCHEMA: &str =
    "echo.operation.basis.anchored-node-alpha-create-if-absent/v1";
const EVALUATION_BASIS_SCHEMA: &str = "echo.operation.evaluation-basis/v1";
const FOOTPRINT_CONTRACT: &str = "anchored-node-alpha-create-if-absent-exact/v1";
const TARGET_PROFILE: &str = "echo.operation-target.anchored-node-alpha-create-if-absent/v1";
const PRECONDITION_MISMATCH: &str = "echo.executable-operation/precondition-mismatch/v1";
const MAX_RESULT_PROJECTION_NODES: usize = 256;
const MAX_RESULT_PROJECTION_PATH_SEGMENTS: usize = 32;
const MAX_RESULT_PROJECTION_TEXT_BYTES: usize = 1_024;
const MAX_RESULT_PROJECTION_ARTIFACT_BYTES: usize = 64 * 1_024;
const MAX_APPLICATION_RESULT_BYTES: u64 = 64 * 1_024;

#[derive(Clone, Copy)]
struct ProgramConfiguration<'a> {
    authority_profile: &'a str,
    required_node_type_profile: &'a str,
    required_attachment_type_profile: &'a str,
    max_replacement_bytes: u64,
    steps: u64,
    read_bytes: u64,
    write_bytes: u64,
    node_key_field: &'a str,
    replacement_field: &'a str,
}

struct ApplicationIntent<'a> {
    name: &'a str,
    operation_coordinate: String,
    obstruction_coordinate: &'a str,
    effect_coordinate: &'a str,
    failure_name: &'a str,
}

struct ClosureInputs<'a> {
    adapter: &'a SemanticInput,
    exports: &'a SemanticInput,
    lawpack: &'a SemanticInput,
    source: &'a SemanticInput,
    configuration: &'a SemanticInput,
    result_projection: &'a SemanticInput,
    target_ir: &'a SemanticInput,
}

struct ResultProjectionPlan {
    runtime_expression: CanonicalValueV1,
    node_key_path: Vec<String>,
    replacement_path: Vec<String>,
}

pub(super) fn is_requested(request: &LoweringRequestV1) -> bool {
    request
        .requested_outputs
        .iter()
        .any(|output| output.role == PACKAGE_ROLE || output.domain == PACKAGE_DOMAIN)
}

pub(super) fn lower(request: &LoweringRequestV1) -> Result<LoweringSuccessV1, ProviderRefusalV1> {
    validate_requested_output(request)?;
    let core = validate_bound(&request.core, CORE_DOMAIN)?;
    let intent = validate_core(&core, &request.core)?;
    let closure = select_closure(&request.semantic_inputs)?;

    let adapter = validate_bound(&closure.adapter.artifact, ADAPTER_DOMAIN)?;
    let exports = validate_bound(&closure.exports.artifact, EXPORTS_DOMAIN)?;
    let lawpack = validate_bound(&closure.lawpack.artifact, LAWPACK_DOMAIN)?;
    let source = validate_bound(&closure.source.artifact, SOURCE_DOMAIN)?;
    let configuration = validate_bound(&closure.configuration.artifact, CONFIGURATION_DOMAIN)?;
    let result_projection = validate_bound(
        &closure.result_projection.artifact,
        RESULT_PROJECTION_DOMAIN,
    )?;
    let target_ir = validate_bound(&closure.target_ir.artifact, TARGET_IR_DOMAIN)?;

    let source = validate_source(&source, closure.source, &request.core)?;
    let semantic_effect =
        semantic_lawpack_member_coordinate(source, closure.lawpack, intent.effect_coordinate)?;
    let semantic_obstruction =
        semantic_lawpack_member_coordinate(source, closure.lawpack, intent.obstruction_coordinate)?;
    validate_exports(&exports, &intent, &semantic_effect)?;
    validate_lawpack(
        &lawpack,
        &exports,
        &adapter,
        closure.lawpack,
        closure.exports,
        closure.adapter,
        &request.target_profile,
    )?;
    validate_adapter(&adapter, closure.configuration, &intent, &semantic_effect)?;
    validate_target_ir(
        &target_ir,
        closure.target_ir,
        request,
        closure.lawpack,
        &intent,
    )?;
    let configuration = validate_configuration(&configuration)?;
    let result_projection = validate_result_projection(
        &result_projection,
        closure.result_projection,
        &target_ir,
        &intent,
        configuration,
    )?;

    let package = encode_package(
        request,
        &closure,
        &intent,
        &semantic_obstruction,
        configuration,
        &result_projection,
    )?;
    let _ = request.limits;
    Ok(LoweringSuccessV1 {
        outputs: vec![LoweringOutputArtifact {
            role: PACKAGE_ROLE.to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            artifact: Artifact {
                domain: PACKAGE_DOMAIN.to_owned(),
                bytes: package,
            },
            logical_path: None,
        }],
        diagnostics: Vec::new(),
    })
}

fn validate_requested_output(request: &LoweringRequestV1) -> Result<(), ProviderRefusalV1> {
    let [output] = request.requested_outputs.as_slice() else {
        return Err(unsupported_output(PACKAGE_ROLE));
    };
    if output.role != PACKAGE_ROLE
        || output.kind != LoweringOutputKind::GeneratedArtifact
        || output.domain != PACKAGE_DOMAIN
    {
        return Err(unsupported_output(&output.role));
    }
    Ok(())
}

fn unsupported_output(subject: &str) -> ProviderRefusalV1 {
    super::refusal(
        ProviderRefusalKind::UnsupportedOutputRole,
        subject,
        "echo.provider.unsupported-output-role",
        "the executable-operation lowerer emits only the generic Echo package role",
    )
}

fn select_closure(inputs: &[SemanticInput]) -> Result<ClosureInputs<'_>, ProviderRefusalV1> {
    if inputs.len() != 7 {
        return Err(super::unsupported_semantics(
            "semantic-inputs.echo-operation",
        ));
    }
    Ok(ClosureInputs {
        adapter: unique_input(
            inputs,
            &SemanticInputKind::Auxiliary("lawpack-adapter".to_owned()),
        )?,
        exports: unique_input(
            inputs,
            &SemanticInputKind::Auxiliary("lawpack-exports".to_owned()),
        )?,
        lawpack: unique_input(inputs, &SemanticInputKind::Lawpack)?,
        source: unique_input(
            inputs,
            &SemanticInputKind::Auxiliary("edict-source".to_owned()),
        )?,
        configuration: unique_input(
            inputs,
            &SemanticInputKind::Auxiliary("target-configuration".to_owned()),
        )?,
        result_projection: unique_input(
            inputs,
            &SemanticInputKind::Auxiliary("result-projection".to_owned()),
        )?,
        target_ir: unique_input(
            inputs,
            &SemanticInputKind::Auxiliary("target-ir".to_owned()),
        )?,
    })
}

fn unique_input<'a>(
    inputs: &'a [SemanticInput],
    kind: &SemanticInputKind,
) -> Result<&'a SemanticInput, ProviderRefusalV1> {
    let mut matching = inputs.iter().filter(|input| &input.kind == kind);
    let Some(input) = matching.next() else {
        return Err(super::unsupported_semantics(
            "semantic-inputs.echo-operation",
        ));
    };
    if matching.next().is_some() {
        return Err(super::unsupported_semantics(
            "semantic-inputs.echo-operation",
        ));
    }
    Ok(input)
}

fn validate_bound(
    bound: &BoundArtifact,
    domain: &str,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    if bound.artifact.domain != domain {
        return Err(super::unsupported_semantics(&bound.reference.coordinate));
    }
    validate_binding(bound).map_err(|()| {
        invalid_artifact(
            &bound.reference.coordinate,
            "canonical artifact binding is invalid",
        )
    })
}

fn validate_core<'a>(
    value: &'a CanonicalValueV1,
    core: &BoundArtifact,
) -> Result<ApplicationIntent<'a>, ProviderRefusalV1> {
    let coordinate = required_text(value, "coordinate", "core.echo-operation")?;
    if text_field(value, "apiVersion") != Some(CORE_ABI) || coordinate != core.reference.coordinate
    {
        return Err(super::unsupported_semantics("core.echo-operation"));
    }
    let (intent_name, intent) =
        single_text_map_entry(required_map(value, "intents", "core.echo-operation")?)
            .ok_or_else(|| super::unsupported_semantics("core.echo-operation"))?;
    if text_field(intent, "requiredOperationProfile") != Some(OPERATION_PROFILE) {
        return Err(super::unsupported_semantics(coordinate));
    }
    let body = required_map(intent, "body", coordinate)?;
    let [node] = required_array(body, "nodes", coordinate)?.as_slice() else {
        return Err(super::unsupported_semantics(coordinate));
    };
    if text_field(node, "kind") != Some("effect") {
        return Err(super::unsupported_semantics(coordinate));
    }
    let effect_coordinate = required_text(node, "effect", coordinate)?;
    let (failure_name, obstruction_arm) =
        single_text_map_entry(required_map(node, "obstructionMap", coordinate)?)
            .ok_or_else(|| super::unsupported_semantics(coordinate))?;
    let obstruction_value = required_map(obstruction_arm, "value", coordinate)?;
    let obstruction_coordinate = required_text(obstruction_value, "callee", coordinate)?;
    if obstruction_coordinate.is_empty() {
        return Err(super::unsupported_semantics(coordinate));
    }
    Ok(ApplicationIntent {
        name: intent_name,
        operation_coordinate: format!("{coordinate}.{intent_name}"),
        obstruction_coordinate,
        effect_coordinate,
        failure_name,
    })
}

fn validate_source<'a>(
    value: &'a CanonicalValueV1,
    source: &SemanticInput,
    core: &BoundArtifact,
) -> Result<&'a str, ProviderRefusalV1> {
    if source.artifact.reference.coordinate != core.reference.coordinate {
        return Err(super::unsupported_semantics(&source.role));
    }
    let CanonicalValueV1::Bytes(source_bytes) = value else {
        return Err(invalid_artifact(
            &source.role,
            "Edict source artifact must be one canonical byte string",
        ));
    };
    std::str::from_utf8(source_bytes)
        .map_err(|_| invalid_artifact(&source.role, "Edict source must be valid UTF-8"))
}

fn validate_exports(
    value: &CanonicalValueV1,
    intent: &ApplicationIntent<'_>,
    semantic_effect: &str,
) -> Result<(), ProviderRefusalV1> {
    let effects = required_array(value, "effects", "exports.echo-operation")?;
    let effect = effects
        .iter()
        .find(|effect| text_field(effect, "coordinate") == Some(semantic_effect))
        .ok_or_else(|| super::unsupported_semantics("exports.echo-operation"))?;
    if text_field(effect, "executionClass") != Some("runtime") {
        return Err(super::unsupported_semantics("exports.echo-operation"));
    }
    let failures = required_map(effect, "effectFailures", "exports.echo-operation")?;
    let (failure_name, _) = single_text_map_entry(failures)
        .ok_or_else(|| super::unsupported_semantics("exports.echo-operation"))?;
    if failure_name != intent.failure_name {
        return Err(super::unsupported_semantics("exports.echo-operation"));
    }
    Ok(())
}

fn validate_lawpack(
    value: &CanonicalValueV1,
    exports_value: &CanonicalValueV1,
    adapter_value: &CanonicalValueV1,
    lawpack: &SemanticInput,
    exports: &SemanticInput,
    adapter: &SemanticInput,
    target_profile: &BoundArtifact,
) -> Result<(), ProviderRefusalV1> {
    let id = required_text(value, "id", "lawpack.echo-operation")?;
    let version = required_text(value, "version", "lawpack.echo-operation")?;
    if text_field(value, "apiVersion") != Some(LAWPACK_DOMAIN)
        || format!("{id}@{version}") != lawpack.artifact.reference.coordinate
    {
        return Err(super::unsupported_semantics("lawpack.echo-operation"));
    }
    require_owner_resource_ref(
        required_map(value, "exports", "lawpack.echo-operation")?,
        &exports.artifact,
        exports_value,
        "lawpack.echo-operation",
    )?;
    let adapters = required_array(value, "targetAdapters", "lawpack.echo-operation")?;
    let selected = adapters
        .iter()
        .find(|candidate| {
            required_map(candidate, "acceptedTargetProfile", "lawpack.echo-operation")
                .is_ok_and(|reference| resource_ref_matches(reference, target_profile))
        })
        .ok_or_else(|| super::unsupported_semantics("lawpack.echo-operation"))?;
    require_owner_resource_ref(
        required_map(selected, "adapter", "lawpack.echo-operation")?,
        &adapter.artifact,
        adapter_value,
        "lawpack.echo-operation",
    )
}

fn validate_adapter(
    value: &CanonicalValueV1,
    configuration: &SemanticInput,
    intent: &ApplicationIntent<'_>,
    semantic_effect: &str,
) -> Result<(), ProviderRefusalV1> {
    if text_field(value, "apiVersion") != Some(ADAPTER_ABI)
        || text_field(value, "class") != Some("declarative")
    {
        return Err(super::unsupported_semantics("adapter.echo-operation"));
    }
    let effects = required_map(value, "effectImplementations", "adapter.echo-operation")?;
    let implementation = required_map_field(effects, semantic_effect, "adapter.echo-operation")?;
    if text_field(implementation, "targetIntrinsic") != Some(TARGET_INTRINSIC)
        || text_field(implementation, "writeClass") != Some("create")
    {
        return Err(super::unsupported_semantics("adapter.echo-operation"));
    }
    require_resource_ref(
        required_map(
            implementation,
            "targetConfiguration",
            "adapter.echo-operation",
        )?,
        &configuration.artifact,
        "adapter.echo-operation",
    )?;
    let mappings = required_map(implementation, "failureMappings", "adapter.echo-operation")?;
    if text_field(mappings, intent.failure_name) != Some(PRECONDITION_MISMATCH) {
        return Err(super::unsupported_semantics("adapter.echo-operation"));
    }
    Ok(())
}

fn semantic_lawpack_member_coordinate(
    source: &str,
    lawpack: &SemanticInput,
    core_effect: &str,
) -> Result<String, ProviderRefusalV1> {
    let lawpack_coordinate = lawpack.artifact.reference.coordinate.as_str();
    if core_effect
        .strip_prefix(lawpack_coordinate)
        .is_some_and(|suffix| suffix.starts_with('.') && suffix.len() > 1)
    {
        return Ok(core_effect.to_owned());
    }
    let expected_digest = super::digest_review(&lawpack.artifact.reference.digest);
    for line in source.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let [use_keyword, kind, coordinate, digest_keyword, digest, as_keyword, alias] =
            fields.as_slice()
        else {
            continue;
        };
        let alias = alias.strip_suffix(';').unwrap_or(alias);
        let digest = digest
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'));
        if *use_keyword == "use"
            && *kind == "lawpack"
            && *coordinate == lawpack_coordinate
            && *digest_keyword == "digest"
            && digest == Some(expected_digest.as_str())
            && *as_keyword == "as"
        {
            let prefix = format!("{alias}.");
            if let Some(member) = core_effect
                .strip_prefix(&prefix)
                .filter(|member| !member.is_empty())
            {
                return Ok(format!("{lawpack_coordinate}.{member}"));
            }
        }
    }
    Err(super::unsupported_semantics("source.lawpack-alias"))
}

fn validate_target_ir(
    value: &CanonicalValueV1,
    target_ir: &SemanticInput,
    request: &LoweringRequestV1,
    lawpack: &SemanticInput,
    intent: &ApplicationIntent<'_>,
) -> Result<(), ProviderRefusalV1> {
    if target_ir.artifact.reference.coordinate != TARGET_IR_COORDINATE
        || text_field(value, "domain") != Some(TARGET_IR_PAYLOAD_DOMAIN)
        || text_field(value, "sourceCoreCoordinate")
            != Some(request.core.reference.coordinate.as_str())
    {
        return Err(super::unsupported_semantics("target-ir.echo-operation"));
    }
    require_resource_ref(
        required_map(value, "targetProfile", "target-ir.echo-operation")?,
        &request.target_profile,
        "target-ir.echo-operation",
    )?;
    let closure = required_map(value, "semanticClosure", "target-ir.echo-operation")?;
    require_resource_ref(
        required_map(closure, "sourceCore", "target-ir.echo-operation")?,
        &request.core,
        "target-ir.echo-operation",
    )?;
    let [lawpack_reference] =
        required_array(closure, "lawpacks", "target-ir.echo-operation")?.as_slice()
    else {
        return Err(super::unsupported_semantics("target-ir.echo-operation"));
    };
    require_resource_ref(
        lawpack_reference,
        &lawpack.artifact,
        "target-ir.echo-operation",
    )?;

    let intents = required_map(value, "intents", "target-ir.echo-operation")?;
    let (intent_name, target_intent) = single_text_map_entry(intents)
        .ok_or_else(|| super::unsupported_semantics("target-ir.echo-operation"))?;
    if intent_name != intent.name
        || text_field(target_intent, "operationProfile") != Some(OPERATION_PROFILE)
    {
        return Err(super::unsupported_semantics("target-ir.echo-operation"));
    }
    let [step] = required_array(target_intent, "steps", "target-ir.echo-operation")?.as_slice()
    else {
        return Err(super::unsupported_semantics("target-ir.echo-operation"));
    };
    let [failure] =
        required_array(step, "obstructionFailures", "target-ir.echo-operation")?.as_slice()
    else {
        return Err(super::unsupported_semantics("target-ir.echo-operation"));
    };
    if text_field(step, "targetIntrinsic") != Some(TARGET_INTRINSIC)
        || as_text(failure) != Some(PRECONDITION_MISMATCH)
    {
        return Err(super::unsupported_semantics("target-ir.echo-operation"));
    }
    Ok(())
}

fn validate_configuration(
    value: &CanonicalValueV1,
) -> Result<ProgramConfiguration<'_>, ProviderRefusalV1> {
    if text_field(value, "apiVersion") != Some(CONFIGURATION_ABI)
        || text_field(value, "programKind") != Some(PROGRAM_KIND)
    {
        return Err(super::unsupported_semantics(
            "target-configuration.echo-operation",
        ));
    }
    let invocation = required_map(
        value,
        "invocationBinding",
        "target-configuration.echo-operation",
    )?;
    let node_key_field = required_nonempty_text(
        invocation,
        "nodeKeyField",
        "target-configuration.echo-operation",
    )?;
    let replacement_field = required_nonempty_text(
        invocation,
        "replacementField",
        "target-configuration.echo-operation",
    )?;
    validate_projection_text(node_key_field, "target-configuration.echo-operation")?;
    validate_projection_text(replacement_field, "target-configuration.echo-operation")?;
    if node_key_field == replacement_field
        || text_field(invocation, "nodeIdDerivation") != Some("sha256-utf8/v1")
        || text_field(invocation, "warpIdSource") != Some("action-lane/v1")
    {
        return Err(super::unsupported_semantics(
            "target-configuration.echo-operation",
        ));
    }
    let budget = required_map(
        value,
        "budgetCeiling",
        "target-configuration.echo-operation",
    )?;
    let configuration = ProgramConfiguration {
        authority_profile: required_nonempty_text(
            value,
            "authorityProfile",
            "target-configuration.echo-operation",
        )?,
        required_node_type_profile: required_nonempty_text(
            value,
            "requiredNodeTypeProfile",
            "target-configuration.echo-operation",
        )?,
        required_attachment_type_profile: required_nonempty_text(
            value,
            "requiredAttachmentTypeProfile",
            "target-configuration.echo-operation",
        )?,
        max_replacement_bytes: required_u64(
            value,
            "maxReplacementBytes",
            "target-configuration.echo-operation",
        )?,
        steps: required_u64(budget, "steps", "target-configuration.echo-operation")?,
        read_bytes: required_u64(budget, "readBytes", "target-configuration.echo-operation")?,
        write_bytes: required_u64(budget, "writeBytes", "target-configuration.echo-operation")?,
        node_key_field,
        replacement_field,
    };
    if configuration.max_replacement_bytes == 0
        || configuration.steps < 3
        || configuration.read_bytes < 64
        || configuration.write_bytes < 64
    {
        return Err(super::unsupported_semantics(
            "target-configuration.echo-operation",
        ));
    }
    Ok(configuration)
}

fn validate_result_projection(
    value: &CanonicalValueV1,
    input: &SemanticInput,
    target_ir: &CanonicalValueV1,
    intent: &ApplicationIntent<'_>,
    configuration: ProgramConfiguration<'_>,
) -> Result<ResultProjectionPlan, ProviderRefusalV1> {
    if input.artifact.artifact.bytes.len() > MAX_RESULT_PROJECTION_ARTIFACT_BYTES {
        return Err(invalid_artifact(
            &input.role,
            "result projection exceeds the canonical byte bound",
        ));
    }
    require_exact_fields(
        value,
        &[
            "expression",
            "maxOutputBytes",
            "operationCoordinate",
            "outputType",
            "schema",
        ],
        &input.role,
    )?;
    if text_field(value, "schema") != Some(RESULT_PROJECTION_ABI)
        || text_field(value, "operationCoordinate") != Some(intent.operation_coordinate.as_str())
        || input.artifact.reference.coordinate != intent.operation_coordinate
    {
        return Err(invalid_artifact(
            &input.role,
            "result projection is rebound from the application operation",
        ));
    }
    validate_projection_text(
        required_nonempty_text(value, "outputType", &input.role)?,
        &input.role,
    )?;
    let max_output_bytes = required_u64(value, "maxOutputBytes", &input.role)?;
    if max_output_bytes == 0 || max_output_bytes > MAX_APPLICATION_RESULT_BYTES {
        return Err(invalid_artifact(
            &input.role,
            "result projection output bound is outside the Echo runtime ceiling",
        ));
    }
    let target_intents = required_map(target_ir, "intents", &input.role)?;
    let target_intent = required_map_field(
        target_intents,
        intent.name,
        "result-projection.target-intent",
    )?;
    let [target_step] = required_array(target_intent, "steps", &input.role)?.as_slice() else {
        return Err(super::unsupported_semantics(
            "result-projection.target-steps",
        ));
    };
    let target_step_id = required_nonempty_text(target_step, "id", &input.role)?;
    let mut nodes = 0;
    let runtime_expression = lower_projection_expression(
        required_map(value, "expression", &input.role)?,
        target_step_id,
        configuration.node_key_field,
        &mut nodes,
        &input.role,
    )?;
    Ok(ResultProjectionPlan {
        runtime_expression,
        node_key_path: vec![configuration.node_key_field.to_owned()],
        replacement_path: vec![configuration.replacement_field.to_owned()],
    })
}

fn lower_projection_expression(
    value: &CanonicalValueV1,
    target_step_id: &str,
    node_key_field: &str,
    nodes: &mut usize,
    subject: &str,
) -> Result<CanonicalValueV1, ProviderRefusalV1> {
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(|| invalid_artifact(subject, "result projection node count overflowed"))?;
    if *nodes > MAX_RESULT_PROJECTION_NODES {
        return Err(invalid_artifact(
            subject,
            "result projection exceeds the expression-node bound",
        ));
    }
    let kind = required_text(value, "kind", subject)?;
    match kind {
        "record" => {
            require_exact_fields(value, &["fields", "kind"], subject)?;
            let fields = required_map(value, "fields", subject)?;
            let entries = as_map(fields)
                .ok_or_else(|| invalid_artifact(subject, "projection fields must be a map"))?;
            let mut lowered = Vec::with_capacity(entries.len());
            for (name, expression) in entries {
                let Some(name) = as_text(name) else {
                    return Err(invalid_artifact(
                        subject,
                        "projection field names must be text",
                    ));
                };
                validate_projection_text(name, subject)?;
                lowered.push((
                    name.to_owned(),
                    lower_projection_expression(
                        expression,
                        target_step_id,
                        node_key_field,
                        nodes,
                        subject,
                    )?,
                ));
            }
            Ok(canonical_map([
                ("kind", canonical_text("record")),
                (
                    "fields",
                    CanonicalValueV1::Map(
                        lowered
                            .into_iter()
                            .map(|(name, value)| (canonical_text(&name), value))
                            .collect(),
                    ),
                ),
            ]))
        }
        "source" => {
            require_exact_fields(value, &["kind", "path", "source"], subject)?;
            let source = required_map(value, "source", subject)?;
            let source_kind = required_text(source, "kind", subject)?;
            let path = required_array(value, "path", subject)?;
            if path.len() > MAX_RESULT_PROJECTION_PATH_SEGMENTS {
                return Err(invalid_artifact(
                    subject,
                    "result projection path exceeds the segment bound",
                ));
            }
            let mut lowered_path = Vec::with_capacity(path.len());
            for segment in path {
                let Some(segment) = as_text(segment) else {
                    return Err(invalid_artifact(
                        subject,
                        "result projection path segment must be text",
                    ));
                };
                validate_projection_text(segment, subject)?;
                lowered_path.push(segment.to_owned());
            }
            match source_kind {
                "applicationInput" => {
                    require_exact_fields(source, &["kind"], subject)?;
                }
                "capabilityResult" => {
                    require_exact_fields(source, &["kind", "stepId"], subject)?;
                    if required_text(source, "stepId", subject)? != target_step_id
                        || lowered_path != [node_key_field]
                    {
                        return Err(super::unsupported_semantics(
                            "result-projection.capability-result",
                        ));
                    }
                    lowered_path = vec![node_key_field.to_owned()];
                }
                _ => {
                    return Err(invalid_artifact(
                        subject,
                        "result projection names an unsupported source kind",
                    ));
                }
            }
            Ok(canonical_map([
                ("kind", canonical_text("source")),
                (
                    "path",
                    CanonicalValueV1::Array(
                        lowered_path
                            .into_iter()
                            .map(|segment| canonical_text(&segment))
                            .collect(),
                    ),
                ),
                (
                    "source",
                    canonical_map([("kind", canonical_text("applicationInput"))]),
                ),
            ]))
        }
        _ => Err(invalid_artifact(
            subject,
            "result projection names an unsupported expression kind",
        )),
    }
}

fn require_exact_fields(
    value: &CanonicalValueV1,
    expected: &[&str],
    subject: &str,
) -> Result<(), ProviderRefusalV1> {
    let entries = as_map(value)
        .ok_or_else(|| invalid_artifact(subject, "result projection node must be a map"))?;
    let mut actual = BTreeSet::new();
    for (key, _) in entries {
        let Some(key) = as_text(key) else {
            return Err(invalid_artifact(
                subject,
                "result projection map key must be text",
            ));
        };
        if !actual.insert(key) {
            return Err(invalid_artifact(
                subject,
                "result projection repeats a map field",
            ));
        }
    }
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(invalid_artifact(
            subject,
            "result projection has an unexpected field set",
        ));
    }
    Ok(())
}

fn validate_projection_text(value: &str, subject: &str) -> Result<(), ProviderRefusalV1> {
    if value.is_empty() || value.len() > MAX_RESULT_PROJECTION_TEXT_BYTES {
        return Err(invalid_artifact(
            subject,
            "result projection text is empty or exceeds its byte bound",
        ));
    }
    Ok(())
}

fn encode_package(
    request: &LoweringRequestV1,
    closure: &ClosureInputs<'_>,
    intent: &ApplicationIntent<'_>,
    obstruction_coordinate: &str,
    configuration: ProgramConfiguration<'_>,
    result_projection: &ResultProjectionPlan,
) -> Result<Vec<u8>, ProviderRefusalV1> {
    let program = canonical_map([
        (
            "interpreter_profile_identity",
            hash_value(profile_digest(INTERPRETER_PROFILE)),
        ),
        (
            "intrinsic_profile_identity",
            hash_value(profile_digest(INTRINSIC_PROFILE)),
        ),
        ("kind", canonical_text(PROGRAM_KIND)),
        (
            "max_replacement_bytes",
            CanonicalValueV1::Integer(i128::from(configuration.max_replacement_bytes)),
        ),
        (
            "required_attachment_type",
            hash_value(profile_digest(
                configuration.required_attachment_type_profile,
            )),
        ),
        (
            "required_node_type",
            hash_value(profile_digest(configuration.required_node_type_profile)),
        ),
        ("schema", canonical_text(PROGRAM_SCHEMA)),
    ]);
    let program_bytes = encode_canonical_cbor_v1(&program).map_err(|_| {
        invalid_artifact(
            PACKAGE_ROLE,
            "operation program could not be canonically encoded",
        )
    })?;
    let core_identity = hash_from_bound(&request.core)?;
    let semantic_closure = canonical_map([
        (
            "application_schema_coordinate",
            canonical_text(&closure.exports.artifact.reference.coordinate),
        ),
        (
            "application_schema_identity",
            hash_value(hash_from_bound(&closure.exports.artifact)?),
        ),
        ("canonical_meaning_identity", hash_value(core_identity)),
        ("core_identity", hash_value(core_identity)),
        (
            "edict_source_identity",
            hash_value(hash_from_bound(&closure.source.artifact)?),
        ),
        (
            "lawpack_coordinate",
            canonical_text(&closure.lawpack.artifact.reference.coordinate),
        ),
        (
            "lawpack_identity",
            hash_value(hash_from_bound(&closure.lawpack.artifact)?),
        ),
        (
            "target_ir_identity",
            hash_value(hash_from_bound(&closure.target_ir.artifact)?),
        ),
    ]);
    let package = canonical_map([
        (
            "application_result_projection",
            canonical_map([
                (
                    "application_input_node_key_path",
                    CanonicalValueV1::Array(
                        result_projection
                            .node_key_path
                            .iter()
                            .map(|segment| canonical_text(segment))
                            .collect(),
                    ),
                ),
                (
                    "application_input_replacement_path",
                    CanonicalValueV1::Array(
                        result_projection
                            .replacement_path
                            .iter()
                            .map(|segment| canonical_text(segment))
                            .collect(),
                    ),
                ),
                (
                    "artifact_bytes",
                    CanonicalValueV1::Bytes(
                        closure.result_projection.artifact.artifact.bytes.clone(),
                    ),
                ),
                (
                    "artifact_identity",
                    hash_value(hash_from_bound(&closure.result_projection.artifact)?),
                ),
                (
                    "runtime_expression",
                    result_projection.runtime_expression.clone(),
                ),
            ]),
        ),
        (
            "application_basis_schema_identity",
            hash_value(profile_digest(APPLICATION_BASIS_SCHEMA)),
        ),
        (
            "authority_profile_identity",
            hash_value(profile_digest(configuration.authority_profile)),
        ),
        (
            "budget_ceiling",
            canonical_map([
                (
                    "read_bytes",
                    CanonicalValueV1::Integer(i128::from(configuration.read_bytes)),
                ),
                (
                    "steps",
                    CanonicalValueV1::Integer(i128::from(configuration.steps)),
                ),
                (
                    "write_bytes",
                    CanonicalValueV1::Integer(i128::from(configuration.write_bytes)),
                ),
            ]),
        ),
        (
            "evaluation_basis_schema_identity",
            hash_value(profile_digest(EVALUATION_BASIS_SCHEMA)),
        ),
        (
            "footprint_contract_identity",
            hash_value(profile_digest(FOOTPRINT_CONTRACT)),
        ),
        (
            "interpreter_profile_identity",
            hash_value(profile_digest(INTERPRETER_PROFILE)),
        ),
        (
            "input_schema_identity",
            hash_value(profile_digest(INPUT_SCHEMA)),
        ),
        (
            "intrinsic_profile_identity",
            hash_value(profile_digest(INTRINSIC_PROFILE)),
        ),
        (
            "obstruction_schema_identity",
            hash_value(profile_digest(OBSTRUCTION_SCHEMA)),
        ),
        (
            "obstruction_interpretation_identity",
            hash_value(profile_digest(OBSTRUCTION_INTERPRETATION)),
        ),
        (
            "obstruction_coordinate",
            canonical_text(obstruction_coordinate),
        ),
        (
            "operation_coordinate",
            canonical_text(&intent.operation_coordinate),
        ),
        ("program", CanonicalValueV1::Bytes(program_bytes)),
        (
            "result_schema_identity",
            hash_value(profile_digest(RESULT_SCHEMA)),
        ),
        (
            "result_interpretation_identity",
            hash_value(profile_digest(RESULT_INTERPRETATION)),
        ),
        ("schema", canonical_text(PACKAGE_SCHEMA)),
        ("semantic_closure", semantic_closure),
        (
            "target_profile_identity",
            hash_value(profile_digest(TARGET_PROFILE)),
        ),
    ]);
    encode_canonical_cbor_v1(&package)
        .map_err(|_| invalid_artifact(PACKAGE_ROLE, "package could not be canonically encoded"))
}

fn require_resource_ref(
    value: &CanonicalValueV1,
    artifact: &BoundArtifact,
    subject: &str,
) -> Result<(), ProviderRefusalV1> {
    if resource_ref_matches(value, artifact) {
        Ok(())
    } else {
        Err(super::unsupported_semantics(subject))
    }
}

fn require_owner_resource_ref(
    value: &CanonicalValueV1,
    artifact: &BoundArtifact,
    artifact_value: &CanonicalValueV1,
    subject: &str,
) -> Result<(), ProviderRefusalV1> {
    let Some(coordinate) = text_field(value, "id") else {
        return Err(super::unsupported_semantics(subject));
    };
    let Some(digest) = array_field(value, "digest")
        .and_then(|values| <&[CanonicalValueV1; 2]>::try_from(values.as_slice()).ok())
    else {
        return Err(super::unsupported_semantics(subject));
    };
    let expected = digest_canonical_value_bytes_v1(coordinate, artifact_value)
        .map_err(|_| invalid_artifact(coordinate, "owner-framed digest could not be computed"))?;
    if coordinate == artifact.reference.coordinate
        && digest[0] == canonical_text("sha256")
        && digest[1] == CanonicalValueV1::Bytes(expected.to_vec())
    {
        Ok(())
    } else {
        Err(super::unsupported_semantics(subject))
    }
}

fn resource_ref_matches(value: &CanonicalValueV1, artifact: &BoundArtifact) -> bool {
    let Some(digest) = array_field(value, "digest")
        .and_then(|values| <&[CanonicalValueV1; 2]>::try_from(values.as_slice()).ok())
    else {
        return false;
    };
    text_field(value, "id") == Some(&artifact.reference.coordinate)
        && digest[0] == canonical_text("sha256")
        && digest[1] == CanonicalValueV1::Bytes(artifact.reference.digest.bytes.clone())
}

fn single_text_map_entry(value: &CanonicalValueV1) -> Option<(&str, &CanonicalValueV1)> {
    let [(key, value)] = as_map(value)?.as_slice() else {
        return None;
    };
    Some((as_text(key)?, value))
}

fn required_map<'a>(
    value: &'a CanonicalValueV1,
    field: &str,
    subject: &str,
) -> Result<&'a CanonicalValueV1, ProviderRefusalV1> {
    let value = map_field(value, field).ok_or_else(|| super::unsupported_semantics(subject))?;
    as_map(value)
        .is_some()
        .then_some(value)
        .ok_or_else(|| super::unsupported_semantics(subject))
}

fn required_map_field<'a>(
    value: &'a CanonicalValueV1,
    field: &str,
    subject: &str,
) -> Result<&'a CanonicalValueV1, ProviderRefusalV1> {
    let value = map_field(value, field).ok_or_else(|| super::unsupported_semantics(subject))?;
    as_map(value)
        .is_some()
        .then_some(value)
        .ok_or_else(|| super::unsupported_semantics(subject))
}

fn required_array<'a>(
    value: &'a CanonicalValueV1,
    field: &str,
    subject: &str,
) -> Result<&'a Vec<CanonicalValueV1>, ProviderRefusalV1> {
    array_field(value, field).ok_or_else(|| super::unsupported_semantics(subject))
}

fn required_text<'a>(
    value: &'a CanonicalValueV1,
    field: &str,
    subject: &str,
) -> Result<&'a str, ProviderRefusalV1> {
    text_field(value, field).ok_or_else(|| super::unsupported_semantics(subject))
}

fn required_nonempty_text<'a>(
    value: &'a CanonicalValueV1,
    field: &str,
    subject: &str,
) -> Result<&'a str, ProviderRefusalV1> {
    required_text(value, field, subject).and_then(|value| {
        (!value.is_empty())
            .then_some(value)
            .ok_or_else(|| super::unsupported_semantics(subject))
    })
}

fn required_u64(
    value: &CanonicalValueV1,
    field: &str,
    subject: &str,
) -> Result<u64, ProviderRefusalV1> {
    match map_field(value, field) {
        Some(CanonicalValueV1::Integer(value)) => {
            u64::try_from(*value).map_err(|_| super::unsupported_semantics(subject))
        }
        _ => Err(super::unsupported_semantics(subject)),
    }
}

fn hash_from_bound(bound: &BoundArtifact) -> Result<[u8; 32], ProviderRefusalV1> {
    bound
        .reference
        .digest
        .bytes
        .as_slice()
        .try_into()
        .map_err(|_| invalid_artifact(&bound.reference.coordinate, "digest must be 32 bytes"))
}

fn profile_digest(label: &str) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"echo:operation-profile:v1\0");
    hasher.update(&(label.len() as u64).to_le_bytes());
    hasher.update(label.as_bytes());
    hasher.finalize().into()
}

fn hash_value(hash: [u8; 32]) -> CanonicalValueV1 {
    CanonicalValueV1::Bytes(hash.to_vec())
}
