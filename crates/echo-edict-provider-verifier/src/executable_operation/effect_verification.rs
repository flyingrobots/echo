// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public verification of a compiler-produced authored effect package.
//!
//! This module owns the component route that judges an
//! `edict-effect-program/v1` package against the compiler request it claims
//! to lower. It binds the request's lawpack, adapter and configuration to the
//! compiled program, rebuilds the exact package that request determines, and
//! accepts only that package, and only when the verifier's own effect-program
//! judgment in [`crate::effect_package`] also accepts it. It shares no code
//! with the lowerer.
use super::{
    build_report, canonical_map, canonical_text, encode_canonical_cbor_v1, map_field,
    profile_digest, require_resource_ref, required_array, required_map, single_text_map_entry,
    text_field, validate_bound, validate_compiler_lawpack, Artifact, BoundArtifact,
    CanonicalValueV1 as Value, ClosureInputs, Diagnostic, DiagnosticSeverity, ProviderRefusalV1,
    VerificationOutputArtifact, VerificationOutputKind, VerificationRequestV1,
    VerificationSuccessV1, ADAPTER_DOMAIN, CONFIGURATION_DOMAIN, CORE_ABI, CORE_DOMAIN,
    EXPORTS_DOMAIN, LAWPACK_DOMAIN, PACKAGE_SCHEMA, REPORT_DOMAIN, REPORT_ROLE, TARGET_IR_DOMAIN,
};
use echo_edict_canonical::{decode_canonical_cbor_v1, digest_canonical_value_bytes_v1};

const SUBJECT: &str = "edict-effect-program";
const BUNDLE_SCHEMA: &str = "echo.compiler-produced-effect-program/v1";
const ORDERED_TARGET_IR: &str = "echo.span-ir/v2";
const PROGRAM_KIND: &str = "edict-effect-program/v1";
const MAX_POLICY_BYTES: usize = 16 * 1024 * 1024;
const MISMATCH: &str = "echo.verifier.effect-package-mismatch";
const REFUSED: &str = "echo.verifier.effect-program-refused";

/// Every fixed profile and schema identity an effect package carries.
const PACKAGE_PROFILES: [(&str, &str); 10] = [
    (
        "target_profile_identity",
        "echo.operation-target.edict-effect/v1",
    ),
    (
        "interpreter_profile_identity",
        "echo.operation-interpreter.edict-effect/v1",
    ),
    (
        "intrinsic_profile_identity",
        "echo.operation-intrinsics.edict-effect/v1",
    ),
    (
        "application_basis_schema_identity",
        "echo.operation.basis.edict-effect/v1",
    ),
    (
        "input_schema_identity",
        "echo.operation.input.edict-effect/v1",
    ),
    (
        "result_schema_identity",
        "echo.operation.result.edict-effect/v1",
    ),
    (
        "obstruction_schema_identity",
        "echo.operation.obstruction.edict-effect/v1",
    ),
    (
        "result_interpretation_identity",
        "echo.operation.result-interpretation.edict-effect/v1",
    ),
    (
        "obstruction_interpretation_identity",
        "echo.operation.obstruction-interpretation.edict-effect/v1",
    ),
    (
        "evaluation_basis_schema_identity",
        "echo.operation.evaluation-basis/v1",
    ),
];

pub(super) fn is_configuration(value: &Value) -> bool {
    text_field(value, "apiVersion") == Some(super::CONFIGURATION_ABI)
        && text_field(value, "programKind") == Some(PROGRAM_KIND)
}

fn unsupported(check: &str) -> ProviderRefusalV1 {
    crate::unsupported_semantics(SUBJECT, check)
}

/// The configuration fields that shape the package.
struct Configuration<'a> {
    catalog: &'a [u8],
    graph_schema: &'a [u8],
    footprint_law: &'a [u8],
    max_input: u64,
    max_result: u64,
    max_allocated: u64,
    steps: u64,
    read_bytes: u64,
    write_bytes: u64,
    authority: &'a str,
}

/// Verifies one effect package against the compiler request it lowers.
///
/// # Errors
/// Refuses a Core, Target IR, lawpack, adapter or configuration that is not
/// bound to the request. A bound request receives a report: `accepted` only
/// for the exact package the request determines, when the effect judgment
/// also accepts it, and `rejected` otherwise.
pub(super) fn verify(
    request: &VerificationRequestV1,
    closure: &ClosureInputs<'_>,
    source_domain: &str,
) -> Result<VerificationSuccessV1, ProviderRefusalV1> {
    let core = validate_bound(&request.core, CORE_DOMAIN)?;
    let target = validate_bound(&request.target_ir, TARGET_IR_DOMAIN)?;
    let exports = validate_bound(&closure.exports.artifact, EXPORTS_DOMAIN)?;
    let lawpack = validate_bound(&closure.lawpack.artifact, LAWPACK_DOMAIN)?;
    let adapter = validate_bound(&closure.adapter.artifact, ADAPTER_DOMAIN)?;
    let configuration_value =
        validate_bound(&closure.configuration.artifact, CONFIGURATION_DOMAIN)?;
    let configuration = configuration(&configuration_value, &request.target_profile)?;
    let module = text_field(&core, "coordinate");
    if text_field(&core, "apiVersion") != Some(CORE_ABI)
        || module != Some(request.core.reference.coordinate.as_str())
    {
        return Err(unsupported("Core is not the bound edict.core/v1 module"));
    }
    let (intent_name, intent) = single_text_map_entry(required_map(&core, "intents", SUBJECT)?)
        .ok_or_else(|| unsupported("an effect program has exactly one intent"))?;
    validate_compiler_lawpack(
        &lawpack,
        &exports,
        &adapter,
        closure,
        &request.target_profile,
        intent,
        request,
    )?;
    bind_closure(&core, &target, request, closure)?;
    let (target_name, lowered) = single_text_map_entry(required_map(&target, "intents", SUBJECT)?)
        .ok_or_else(|| unsupported("Target IR has exactly one intent"))?;
    if target_name != intent_name {
        return Err(unsupported("Target IR lowers a different intent"));
    }
    let has_steps = !required_array(lowered, "steps", SUBJECT)?.is_empty();
    let expected = expected_package(
        request,
        closure,
        source_domain,
        intent_name,
        text_field(intent, "output").ok_or_else(|| unsupported("intent has no output type"))?,
        has_steps,
        &configuration,
    )?;
    let actual = &closure.package.artifact.artifact.bytes;
    let refusal = if crate::effect_package::verify_effect_package(actual).is_err() {
        Some((REFUSED, "the effect-program judgment refuses the package"))
    } else if actual != &expected {
        Some((
            MISMATCH,
            "the package is not the one this compiler request determines",
        ))
    } else {
        None
    };
    report(request, closure, refusal)
}

fn report(
    request: &VerificationRequestV1,
    closure: &ClosureInputs<'_>,
    refusal: Option<(&str, &str)>,
) -> Result<VerificationSuccessV1, ProviderRefusalV1> {
    let (outcome, diagnostics, bytes) = match refusal {
        None => ("accepted", Vec::new(), Vec::new()),
        Some((code, message)) => (
            "rejected",
            vec![Diagnostic {
                code: code.to_owned(),
                severity: DiagnosticSeverity::Error,
                message: message.to_owned(),
                repair: None,
            }],
            code.as_bytes().to_vec(),
        ),
    };
    let report = build_report(
        &closure.package.artifact.reference,
        &request.target_ir.reference,
        &closure.result_projection.artifact.reference,
        outcome,
        bytes,
    )?;
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

fn policy<'a>(value: &'a Value, key: &str) -> Result<&'a [u8], ProviderRefusalV1> {
    match map_field(value, key) {
        Some(Value::Bytes(bytes))
            if !bytes.is_empty()
                && bytes.len() <= MAX_POLICY_BYTES
                && decode_canonical_cbor_v1(bytes).is_ok() =>
        {
            Ok(bytes)
        }
        _ => Err(unsupported(
            "configuration policy bytes are not bounded canonical CBOR",
        )),
    }
}

fn number(value: &Value, key: &str, max: u64) -> Result<u64, ProviderRefusalV1> {
    match map_field(value, key) {
        Some(Value::Integer(number)) => u64::try_from(*number)
            .ok()
            .filter(|number| *number <= max)
            .ok_or_else(|| unsupported("configuration limit is out of range")),
        _ => Err(unsupported("configuration limit is not an integer")),
    }
}

fn exact_keys(value: &Value, keys: &[&str]) -> bool {
    matches!(value, Value::Map(entries)
        if entries.len() == keys.len()
            && entries.iter().all(|(key, _)| matches!(key, Value::Text(key) if keys.contains(&key.as_str()))))
}

/// The effect configuration, whose catalog must be the one the selected
/// target profile pins.
fn configuration<'a>(
    value: &'a Value,
    profile: &BoundArtifact,
) -> Result<Configuration<'a>, ProviderRefusalV1> {
    let keys = [
        "apiVersion",
        "programKind",
        "identityLaw",
        "intrinsicCatalog",
        "graphSchema",
        "footprintLaw",
        "maxInputBytes",
        "maxResultBytes",
        "maxAllocatedBytes",
        "budgetCeiling",
        "authorityProfile",
    ];
    let budget = map_field(value, "budgetCeiling");
    if !exact_keys(value, &keys)
        || !is_configuration(value)
        || map_field(value, "identityLaw") != Some(&Value::Integer(1))
        || !budget.is_some_and(|budget| exact_keys(budget, &["steps", "readBytes", "writeBytes"]))
    {
        return Err(unsupported(
            "configuration is not the current effect configuration",
        ));
    }
    let budget = budget.ok_or_else(|| unsupported("configuration has no budget ceiling"))?;
    let authority = text_field(value, "authorityProfile")
        .filter(|label| !label.is_empty())
        .ok_or_else(|| unsupported("configuration has no authority profile label"))?;
    let configuration = Configuration {
        catalog: policy(value, "intrinsicCatalog")?,
        graph_schema: policy(value, "graphSchema")?,
        footprint_law: policy(value, "footprintLaw")?,
        max_input: number(value, "maxInputBytes", 65_536)?,
        max_result: number(value, "maxResultBytes", 65_536)?,
        max_allocated: number(value, "maxAllocatedBytes", 16 * 1024 * 1024)?,
        steps: number(budget, "steps", u64::MAX)?,
        read_bytes: number(budget, "readBytes", u64::MAX)?,
        write_bytes: number(budget, "writeBytes", u64::MAX)?,
        authority,
    };
    if configuration.steps == 0 {
        return Err(unsupported("configuration step ceiling is zero"));
    }
    let profile = validate_bound(profile, "edict.target-profile/v1")?;
    // The catalog coordinate, which is also its digest domain, is the one the
    // selected profile's namespace pins.
    let coordinate = format!(
        "{}@{}",
        text_field(&profile, "id").unwrap_or_default(),
        text_field(&profile, "version").unwrap_or_default()
    );
    let catalog_domain = super::intrinsic_names::namespace(&coordinate)
        .ok_or_else(|| unsupported("the target profile names no effect intrinsic namespace"))?
        .catalog;
    let catalog = decode_canonical_cbor_v1(configuration.catalog)
        .map_err(|_| unsupported("configuration catalog is not canonical"))?;
    let identity = digest_canonical_value_bytes_v1(catalog_domain, &catalog)
        .map_err(|_| unsupported("configuration catalog identity cannot be computed"))?;
    let pinned = canonical_map([
        ("id", canonical_text(catalog_domain)),
        (
            "digest",
            Value::Array(vec![
                canonical_text("sha256"),
                Value::Bytes(identity.to_vec()),
            ]),
        ),
    ]);
    if map_field(&profile, "intrinsics") != Some(&pinned) {
        return Err(unsupported(
            "configuration catalog is not the one the target profile pins",
        ));
    }
    Ok(configuration)
}

/// The Target IR names this Core, profile and lawpack, and the Core imports
/// exactly that lawpack.
fn bind_closure(
    core: &Value,
    target: &Value,
    request: &VerificationRequestV1,
    closure: &ClosureInputs<'_>,
) -> Result<(), ProviderRefusalV1> {
    if request.target_ir.reference.coordinate != ORDERED_TARGET_IR {
        return Err(unsupported(
            "Target IR is not bound as the ordered echo.span-ir/v2",
        ));
    }
    if text_field(target, "sourceCoreCoordinate")
        != Some(request.core.reference.coordinate.as_str())
    {
        return Err(unsupported("Target IR lowers a different Core coordinate"));
    }
    let field = |value: &Value, key: &str| {
        map_field(value, key)
            .cloned()
            .ok_or_else(|| unsupported("a closure reference is missing"))
    };
    require_resource_ref(
        &field(target, "targetProfile")?,
        &request.target_profile,
        SUBJECT,
        "effect Target IR target profile reference",
    )?;
    let semantic = field(target, "semanticClosure")?;
    require_resource_ref(
        &field(&semantic, "sourceCore")?,
        &request.core,
        SUBJECT,
        "effect Target IR source Core reference",
    )?;
    let lawpacks = required_array(&semantic, "lawpacks", SUBJECT)?;
    let [lawpack] = lawpacks.as_slice() else {
        return Err(unsupported("Target IR names exactly one lawpack"));
    };
    require_resource_ref(
        lawpack,
        &closure.lawpack.artifact,
        SUBJECT,
        "effect Target IR lawpack reference",
    )?;
    let imports: Vec<&Value> = required_array(core, "imports", SUBJECT)?
        .iter()
        .filter(|import| text_field(import, "kind") == Some("lawpack"))
        .collect();
    let [import] = imports.as_slice() else {
        return Err(unsupported("Core imports exactly one lawpack"));
    };
    require_resource_ref(
        &field(import, "ref")?,
        &closure.lawpack.artifact,
        SUBJECT,
        "effect Core lawpack import reference",
    )
}

fn raw(artifact: &BoundArtifact) -> Value {
    Value::Bytes(artifact.artifact.bytes.clone())
}

fn identity(artifact: &BoundArtifact, domain: &str) -> Result<Value, ProviderRefusalV1> {
    let value = decode_canonical_cbor_v1(&artifact.artifact.bytes)
        .map_err(|_| unsupported("a bound artifact is not canonical"))?;
    digest_canonical_value_bytes_v1(domain, &value)
        .map(|digest| Value::Bytes(digest.to_vec()))
        .map_err(|_| unsupported("an artifact identity cannot be computed"))
}

fn label(value: &str) -> Value {
    Value::Bytes(profile_digest(value).to_vec())
}

fn integer(value: u64) -> Value {
    Value::Integer(i128::from(value))
}

/// The exact package the request determines.
#[allow(
    clippy::too_many_lines,
    reason = "The expected wire projection is kept in one place, field for field."
)]
fn expected_package(
    request: &VerificationRequestV1,
    closure: &ClosureInputs<'_>,
    source_domain: &str,
    intent: &str,
    output: &str,
    has_steps: bool,
    configuration: &Configuration<'_>,
) -> Result<Vec<u8>, ProviderRefusalV1> {
    let encode = |value: &Value| {
        encode_canonical_cbor_v1(value)
            .map_err(|_| unsupported("the expected package cannot be encoded"))
    };
    let lowering = if has_steps {
        canonical_map([
            ("source_artifact", raw(&closure.source.artifact)),
            ("lawpack_artifact", raw(&closure.lawpack.artifact)),
            ("adapter_artifact", raw(&closure.adapter.artifact)),
            ("target_profile_artifact", raw(&request.target_profile)),
            (
                "intrinsics_artifact",
                Value::Bytes(configuration.catalog.to_vec()),
            ),
        ])
    } else {
        Value::Null
    };
    let bundle = encode(&canonical_map([
        ("schema", canonical_text(BUNDLE_SCHEMA)),
        ("intent", canonical_text(intent)),
        ("core_artifact", raw(&request.core)),
        ("target_ir_artifact", raw(&request.target_ir)),
        ("lawpack_exports_artifact", raw(&closure.exports.artifact)),
        ("lowering_artifacts", lowering),
    ]))?;
    let core_identity = identity(&request.core, CORE_DOMAIN)?;
    let target_identity = identity(&request.target_ir, TARGET_IR_DOMAIN)?;
    let mut footprint = blake3::Hasher::new();
    footprint.update(b"echo:operation-footprint-law:v1\0");
    footprint.update(configuration.footprint_law);
    let program = encode(&canonical_map([
        ("schema", canonical_text("echo.operation-program/v1")),
        ("kind", canonical_text(PROGRAM_KIND)),
        (
            "interpreter_profile_identity",
            label("echo.operation-interpreter.edict-effect/v1"),
        ),
        (
            "intrinsic_profile_identity",
            label("echo.operation-intrinsics.edict-effect/v1"),
        ),
        ("compiled_program", Value::Bytes(bundle)),
        ("target_ir_identity", target_identity.clone()),
        ("identity_law", Value::Integer(1)),
        (
            "graph_schema",
            Value::Bytes(configuration.graph_schema.to_vec()),
        ),
        (
            "footprint_law",
            Value::Bytes(configuration.footprint_law.to_vec()),
        ),
        ("max_input_bytes", integer(configuration.max_input)),
        ("max_result_bytes", integer(configuration.max_result)),
        ("max_allocated_bytes", integer(configuration.max_allocated)),
        ("output_type", canonical_text(output)),
    ]))?;
    let mut package = vec![
        (canonical_text("schema"), canonical_text(PACKAGE_SCHEMA)),
        (
            canonical_text("operation_coordinate"),
            canonical_text(&format!("{}.{intent}", request.core.reference.coordinate)),
        ),
        (
            canonical_text("authority_profile_identity"),
            label(configuration.authority),
        ),
        (
            canonical_text("semantic_closure"),
            canonical_map([
                (
                    "edict_source_identity",
                    identity(&closure.source.artifact, source_domain)?,
                ),
                ("canonical_meaning_identity", core_identity.clone()),
                ("core_identity", core_identity),
                ("target_ir_identity", target_identity),
                (
                    "application_schema_coordinate",
                    canonical_text(&closure.exports.artifact.reference.coordinate),
                ),
                (
                    "application_schema_identity",
                    identity(&closure.exports.artifact, EXPORTS_DOMAIN)?,
                ),
            ]),
        ),
        (canonical_text("program"), Value::Bytes(program)),
        (
            canonical_text("budget_ceiling"),
            canonical_map([
                ("steps", integer(configuration.steps)),
                ("read_bytes", integer(configuration.read_bytes)),
                ("write_bytes", integer(configuration.write_bytes)),
            ]),
        ),
        (
            canonical_text("footprint_contract_identity"),
            Value::Bytes(footprint.finalize().as_bytes().to_vec()),
        ),
    ];
    package.extend(
        PACKAGE_PROFILES
            .into_iter()
            .map(|(key, profile)| (canonical_text(key), label(profile))),
    );
    encode(&Value::Map(package))
}
