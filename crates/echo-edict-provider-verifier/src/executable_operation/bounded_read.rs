// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent read verification: reconstruct Core from the ordered target.

mod audit;

use super::{
    array_field, as_map, as_text, build_report, canonical_text, encode_canonical_cbor_v1 as encode,
    hash_from_bound, invalid_artifact, map_field, profile_digest, text_field, validate_bound,
    Artifact, BoundArtifact, CanonicalValueV1 as Value, ClosureInputs, Diagnostic,
    DiagnosticSeverity, ProviderRefusalV1, VerificationOutputArtifact, VerificationOutputKind,
    VerificationRequestV1, VerificationSuccessV1, ADAPTER_DOMAIN, CONFIGURATION_DOMAIN,
    CORE_DOMAIN, EXPORTS_DOMAIN, LAWPACK_DOMAIN, PACKAGE_DOMAIN, REPORT_DOMAIN, REPORT_ROLE,
    RESULT_PROJECTION_DOMAIN, SOURCE_DOMAIN, TARGET_IR_DOMAIN,
};

type Check<T> = Result<T, ()>;
const KIND: &str = "compiler-produced-bounded-read/v1";
const PROFILE: &str = "continuum.profile.read-only/v1";

pub(super) fn selected(value: &Value) -> bool {
    text_field(value, "apiVersion") == Some(CONFIGURATION_DOMAIN)
        && text_field(value, "programKind") == Some(KIND)
}

fn get<'a>(value: &'a Value, name: &str) -> Check<&'a Value> {
    map_field(value, name).ok_or(())
}

fn string(value: &Value) -> Check<&str> {
    as_text(value)
        .filter(|text| !text.is_empty() && text.len() <= 1024)
        .ok_or(())
}

fn text<'a>(value: &'a Value, name: &str) -> Check<&'a str> {
    string(get(value, name)?)
}

fn sequence<'a>(value: &'a Value, name: &str) -> Check<&'a [Value]> {
    array_field(value, name).map(Vec::as_slice).ok_or(())
}

fn number(value: &Value) -> Check<u64> {
    match value {
        Value::Integer(value) => u64::try_from(*value).map_err(|_| ()),
        _ => Err(()),
    }
}

fn members(value: &Value) -> Check<&[(Value, Value)]> {
    as_map(value).map(Vec::as_slice).ok_or(())
}

fn exact(value: &Value, fields: &[&str]) -> Check<()> {
    let entries = members(value)?;
    if entries.len() != fields.len() || fields.iter().any(|key| map_field(value, key).is_none()) {
        return Err(());
    }
    Ok(())
}

fn same(left: &Value, right: &Value) -> Check<()> {
    if encode(left).map_err(|_| ())? != encode(right).map_err(|_| ())? {
        return Err(());
    }
    Ok(())
}

fn reference(value: &Value, artifact: &BoundArtifact) -> Check<()> {
    exact(value, &["id", "digest"])?;
    if text(value, "id")? != artifact.reference.coordinate
        || get(value, "digest")?
            != &Value::Array(vec![
                canonical_text("sha256"),
                Value::Bytes(artifact.reference.digest.bytes.clone()),
            ])
    {
        return Err(());
    }
    Ok(())
}

struct Evidence<'a> {
    request: &'a VerificationRequestV1,
    closure: &'a ClosureInputs<'a>,
    core: Value,
    target: Value,
    exports: Value,
    lawpack: Value,
    adapter: Value,
    configuration: Value,
    source: Value,
    projection: Value,
    package: Value,
}

pub(super) fn verify(
    request: &VerificationRequestV1,
    closure: &ClosureInputs<'_>,
) -> Result<VerificationSuccessV1, ProviderRefusalV1> {
    let evidence = Evidence {
        request,
        closure,
        core: validate_bound(&request.core, CORE_DOMAIN)?,
        target: validate_bound(&request.target_ir, TARGET_IR_DOMAIN)?,
        exports: validate_bound(&closure.exports.artifact, EXPORTS_DOMAIN)?,
        lawpack: validate_bound(&closure.lawpack.artifact, LAWPACK_DOMAIN)?,
        adapter: validate_bound(&closure.adapter.artifact, ADAPTER_DOMAIN)?,
        configuration: validate_bound(&closure.configuration.artifact, CONFIGURATION_DOMAIN)?,
        source: validate_bound(&closure.source.artifact, SOURCE_DOMAIN)?,
        projection: validate_bound(
            &closure.result_projection.artifact,
            RESULT_PROJECTION_DOMAIN,
        )?,
        package: validate_bound(&closure.package.artifact, PACKAGE_DOMAIN)?,
    };
    let accepted = audit::check(&evidence).is_ok();
    let code = "echo.verifier.bounded-read-relation-mismatch";
    let (outcome, diagnostics, diagnostic_bytes) = if accepted {
        ("accepted", Vec::new(), Vec::new())
    } else {
        ("rejected", vec![Diagnostic { code: code.to_owned(), severity: DiagnosticSeverity::Error,
            message: "Read package does not preserve the ordered Core, signature, scope, and artifact relation".to_owned(), repair: None }], code.as_bytes().to_vec())
    };
    let report = build_report(
        &closure.package.artifact.reference,
        &request.target_ir.reference,
        &closure.result_projection.artifact.reference,
        outcome,
        diagnostic_bytes,
    )?;
    let response_bytes = [REPORT_ROLE.len(), REPORT_DOMAIN.len(), report.len()]
        .into_iter()
        .chain(diagnostics.iter().flat_map(|diagnostic| {
            [
                diagnostic.code.len(),
                diagnostic.message.len(),
                diagnostic.repair.as_ref().map_or(0, String::len),
            ]
        }))
        .try_fold(0_u64, |sum, size| {
            sum.checked_add(u64::try_from(size).ok()?)
        })
        .ok_or_else(|| invalid_artifact(REPORT_ROLE, "read response size overflow"))?;
    if request.limits.max_output_count == 0
        || diagnostics.len() as u64 > u64::from(request.limits.max_diagnostic_count)
        || response_bytes > request.limits.max_total_response_bytes
    {
        return Err(invalid_artifact(
            REPORT_ROLE,
            "read report exceeds host response budget",
        ));
    }
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

fn package(evidence: &Evidence<'_>, name: &str, intent: &Value, coordinate: &str) -> Check<()> {
    let package = &evidence.package;
    exact(
        package,
        &[
            "authority_profile_identity",
            "budget_ceiling",
            "footprint_contract_identity",
            "interpreter_profile_identity",
            "operation_coordinate",
            "package_kind",
            "program",
            "schema",
            "semantic_closure",
            "target_profile_identity",
        ],
    )?;
    if text(package, "schema")? != PACKAGE_DOMAIN
        || text(package, "package_kind")? != KIND
        || text(package, "operation_coordinate")? != coordinate
    {
        return Err(());
    }
    for (field, profile) in [
        (
            "authority_profile_identity",
            "echo.operation.authority.explicit-read-view/v1",
        ),
        (
            "footprint_contract_identity",
            "echo.operation.footprint.explicit-node-read-aperture/v1",
        ),
        (
            "interpreter_profile_identity",
            "echo.operation-interpreter.compiler-produced-bounded-read/v1",
        ),
    ] {
        if get(package, field)? != &Value::Bytes(profile_digest(profile).to_vec()) {
            return Err(());
        }
    }
    identity(
        package,
        "target_profile_identity",
        &evidence.request.target_profile,
    )?;
    let budget = get(package, "budget_ceiling")?;
    exact(
        budget,
        &["max_allocated_bytes", "max_output_bytes", "max_steps"],
    )?;
    for (outer, inner) in [
        ("max_allocated_bytes", "maxAllocatedBytes"),
        ("max_output_bytes", "maxOutputBytes"),
        ("max_steps", "maxSteps"),
    ] {
        same(
            get(budget, outer)?,
            get(get(intent, "coreEvaluationBudget")?, inner)?,
        )?;
    }
    let closure = get(package, "semantic_closure")?;
    exact(
        closure,
        &[
            "application_schema_coordinate",
            "application_schema_identity",
            "canonical_meaning_identity",
            "core_identity",
            "edict_source_identity",
            "lawpack_coordinate",
            "lawpack_identity",
            "target_ir_identity",
        ],
    )?;
    for (field, artifact) in [
        (
            "application_schema_identity",
            &evidence.closure.exports.artifact,
        ),
        ("core_identity", &evidence.request.core),
        ("canonical_meaning_identity", &evidence.request.core),
        ("edict_source_identity", &evidence.closure.source.artifact),
        ("lawpack_identity", &evidence.closure.lawpack.artifact),
        ("target_ir_identity", &evidence.request.target_ir),
    ] {
        identity(closure, field, artifact)?;
    }
    if text(closure, "application_schema_coordinate")?
        != evidence.closure.exports.artifact.reference.coordinate
        || text(closure, "lawpack_coordinate")?
            != evidence.closure.lawpack.artifact.reference.coordinate
    {
        return Err(());
    }
    let Value::Bytes(bytes) = get(package, "program")? else {
        return Err(());
    };
    let program = echo_edict_canonical::decode_canonical_cbor_v1(bytes).map_err(|_| ())?;
    exact(
        &program,
        &[
            "kind",
            "schema",
            "intent",
            "core_artifact",
            "target_ir_artifact",
            "lawpack_exports_artifact",
            "lawpack_artifact",
            "lawpack_adapter_artifact",
            "target_configuration_artifact",
            "result_projection_artifact",
            "source_artifact",
        ],
    )?;
    if text(&program, "kind")? != KIND
        || text(&program, "schema")? != "echo.compiler-produced-read-program/v1"
        || text(&program, "intent")? != name
    {
        return Err(());
    }
    for (field, artifact) in [
        ("core_artifact", &evidence.request.core),
        ("target_ir_artifact", &evidence.request.target_ir),
        (
            "lawpack_exports_artifact",
            &evidence.closure.exports.artifact,
        ),
        ("lawpack_artifact", &evidence.closure.lawpack.artifact),
        (
            "lawpack_adapter_artifact",
            &evidence.closure.adapter.artifact,
        ),
        (
            "target_configuration_artifact",
            &evidence.closure.configuration.artifact,
        ),
        (
            "result_projection_artifact",
            &evidence.closure.result_projection.artifact,
        ),
        ("source_artifact", &evidence.closure.source.artifact),
    ] {
        if get(&program, field)? != &Value::Bytes(artifact.artifact.bytes.clone()) {
            return Err(());
        }
    }
    Ok(())
}

fn identity(value: &Value, key: &str, artifact: &BoundArtifact) -> Check<()> {
    if get(value, key)? != &Value::Bytes(hash_from_bound(artifact).map_err(|_| ())?.to_vec()) {
        return Err(());
    }
    Ok(())
}
