// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public lowering of a compiler-produced authored effect program.
//!
//! This module owns the component route from a digest-bound compiler request
//! to an `edict-effect-program/v1` package. It binds the request's lawpack,
//! adapter and configuration to the compiled program, assembles the
//! compiler bundle from the request's own artifacts, and hands it to the
//! judged producer in [`crate::effect_publication`], which refuses any
//! program outside the lowerer's effect judgment.
use super::{
    canonical_map, canonical_text, encode_canonical_cbor_v1, map_field, require_resource_ref,
    required_array, required_map, single_text_map_entry, text_field, validate_bound,
    validate_compiler_lawpack, Artifact, BoundArtifact, CanonicalValueV1 as Value, ClosureInputs,
    LoweringOutputArtifact, LoweringOutputKind, LoweringRequestV1, LoweringSuccessV1,
    ProviderRefusalV1, ADAPTER_DOMAIN, CORE_ABI, CORE_DOMAIN, EXPORTS_DOMAIN, LAWPACK_DOMAIN,
    PACKAGE_DOMAIN, PACKAGE_ROLE, TARGET_IR_DOMAIN,
};
use crate::effect_publication::{encode_effect_publication, EffectPublicationInput};

const SUBJECT: &str = "edict-effect-program";
const BUNDLE_SCHEMA: &str = "echo.compiler-produced-effect-program/v1";
const ORDERED_TARGET_IR: &str = "echo.span-ir/v2";

fn unsupported(check: &str) -> ProviderRefusalV1 {
    crate::unsupported_semantics(SUBJECT, check)
}

/// Lowers one compiler-produced effect program into its operation package.
///
/// # Errors
/// Refuses a Core that is not the bound single-intent module, a Target IR,
/// lawpack, adapter or configuration not bound to that Core, and any program
/// the effect producer's judgment refuses.
pub(super) fn lower(
    request: &LoweringRequestV1,
    closure: &ClosureInputs<'_>,
    source_domain: &str,
) -> Result<LoweringSuccessV1, ProviderRefusalV1> {
    let core = validate_bound(&request.core, CORE_DOMAIN)?;
    let target = validate_bound(&closure.target_ir.artifact, TARGET_IR_DOMAIN)?;
    let exports = validate_bound(&closure.exports.artifact, EXPORTS_DOMAIN)?;
    let lawpack = validate_bound(&closure.lawpack.artifact, LAWPACK_DOMAIN)?;
    let adapter = validate_bound(&closure.adapter.artifact, ADAPTER_DOMAIN)?;
    if text_field(&core, "apiVersion") != Some(CORE_ABI)
        || text_field(&core, "coordinate") != Some(request.core.reference.coordinate.as_str())
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
    )?;
    bind_closure(&core, &target, request, closure)?;
    let (target_name, lowered) = single_text_map_entry(required_map(&target, "intents", SUBJECT)?)
        .ok_or_else(|| unsupported("Target IR has exactly one intent"))?;
    if target_name != intent_name {
        return Err(unsupported("Target IR lowers a different intent"));
    }
    let lowering = if required_array(lowered, "steps", SUBJECT)?.is_empty() {
        Value::Null
    } else {
        lowering_artifacts(request, closure)?
    };
    let bundle = canonical_map([
        ("schema", canonical_text(BUNDLE_SCHEMA)),
        ("intent", canonical_text(intent_name)),
        ("core_artifact", bytes(&request.core)),
        ("target_ir_artifact", bytes(&closure.target_ir.artifact)),
        ("lawpack_exports_artifact", bytes(&closure.exports.artifact)),
        ("lowering_artifacts", lowering),
    ]);
    let bundle = encode_canonical_cbor_v1(&bundle)
        .map_err(|_| unsupported("the compiler bundle cannot be encoded canonically"))?;
    let package = encode_effect_publication(EffectPublicationInput {
        compiler_program: &bundle,
        application_schema_coordinate: &closure.exports.artifact.reference.coordinate,
        source_artifact: &closure.source.artifact.artifact.bytes,
        source_domain,
        configuration: &closure.configuration.artifact.artifact.bytes,
    })
    .map_err(|error| unsupported(&error.to_string()))?;
    within_response_limits(request, &package)?;
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

fn bytes(artifact: &BoundArtifact) -> Value {
    Value::Bytes(artifact.artifact.bytes.clone())
}

/// The artifacts that authorize each effect step: the exact request bytes,
/// with the intrinsic catalog the configuration carries.
fn lowering_artifacts(
    request: &LoweringRequestV1,
    closure: &ClosureInputs<'_>,
) -> Result<Value, ProviderRefusalV1> {
    let configuration =
        validate_bound(&closure.configuration.artifact, super::CONFIGURATION_DOMAIN)?;
    let Some(catalog @ Value::Bytes(_)) = map_field(&configuration, "intrinsicCatalog") else {
        return Err(unsupported("configuration carries no intrinsic catalog"));
    };
    Ok(canonical_map([
        ("source_artifact", bytes(&closure.source.artifact)),
        ("lawpack_artifact", bytes(&closure.lawpack.artifact)),
        ("adapter_artifact", bytes(&closure.adapter.artifact)),
        ("target_profile_artifact", bytes(&request.target_profile)),
        ("intrinsics_artifact", catalog.clone()),
    ]))
}

/// The Target IR names this Core, profile and lawpack, and the Core imports
/// exactly that lawpack.
fn bind_closure(
    core: &Value,
    target: &Value,
    request: &LoweringRequestV1,
    closure: &ClosureInputs<'_>,
) -> Result<(), ProviderRefusalV1> {
    if closure.target_ir.artifact.reference.coordinate != ORDERED_TARGET_IR {
        return Err(unsupported(
            "Target IR is not bound as the ordered echo.span-ir/v2",
        ));
    }
    if text_field(target, "sourceCoreCoordinate")
        != Some(request.core.reference.coordinate.as_str())
    {
        return Err(unsupported("Target IR lowers a different Core coordinate"));
    }
    require_resource_ref(
        reference(target, "targetProfile")?,
        &request.target_profile,
        SUBJECT,
        "effect Target IR target profile reference",
    )?;
    let semantic = reference(target, "semanticClosure")?;
    require_resource_ref(
        reference(semantic, "sourceCore")?,
        &request.core,
        SUBJECT,
        "effect Target IR source Core reference",
    )?;
    let [lawpack] = required_array(semantic, "lawpacks", SUBJECT)?.as_slice() else {
        return Err(unsupported("Target IR names exactly one lawpack"));
    };
    require_resource_ref(
        lawpack,
        &closure.lawpack.artifact,
        SUBJECT,
        "effect Target IR lawpack reference",
    )?;
    let mut imports = required_array(core, "imports", SUBJECT)?
        .iter()
        .filter(|import| text_field(import, "kind") == Some("lawpack"));
    let (Some(import), None) = (imports.next(), imports.next()) else {
        return Err(unsupported("Core imports exactly one lawpack"));
    };
    require_resource_ref(
        reference(import, "ref")?,
        &closure.lawpack.artifact,
        SUBJECT,
        "effect Core lawpack import reference",
    )
}

fn reference<'a>(value: &'a Value, key: &str) -> Result<&'a Value, ProviderRefusalV1> {
    map_field(value, key).ok_or_else(|| unsupported("a closure reference is missing"))
}

fn within_response_limits(
    request: &LoweringRequestV1,
    package: &[u8],
) -> Result<(), ProviderRefusalV1> {
    let size = [PACKAGE_ROLE.len(), PACKAGE_DOMAIN.len(), package.len()]
        .into_iter()
        .try_fold(0_u64, |sum, size| {
            sum.checked_add(u64::try_from(size).ok()?)
        });
    match size {
        Some(size)
            if request.limits.max_output_count > 0
                && size <= request.limits.max_total_response_bytes =>
        {
            Ok(())
        }
        _ => Err(unsupported("the package exceeds the host response budget")),
    }
}
