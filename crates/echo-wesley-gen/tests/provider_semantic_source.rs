// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Integration tests for the strict Echo Edict provider semantic source.

use echo_wesley_gen::provider_semantics::{
    parse_provider_semantic_source_v1, ApertureRequirementDeclaration, ArtifactResourceProvision,
    AuthorityClass, AuthorityFactSourceKind, AuthoritySourceKind, BoundaryKind,
    CoreStringCanonicalization, EffectKindHint, ExecutionClass, GeneratedArtifactKind,
    InvocationInputKind, InvocationOutputKind, OpticKind, ProviderComponentKind,
    ProviderSemanticSourceErrorKind, SemanticTypeShape, ECHO_PROVIDER_SEMANTIC_SOURCE_API_V1,
};
use serde_json::{json, Value};

const SOURCE: &str = include_str!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");

fn source_value() -> Value {
    serde_json::from_str(SOURCE).expect("checked source fixture is JSON")
}

fn assert_failure(mutate: impl FnOnce(&mut Value), expected: ProviderSemanticSourceErrorKind) {
    let mut source = source_value();
    mutate(&mut source);
    let text = serde_json::to_string(&source).expect("mutated source serializes");
    let error =
        parse_provider_semantic_source_v1(&text).expect_err("invalid semantic source must fail");
    assert_eq!(error.kind(), expected);
}

fn assert_failure_tuple(
    mutate: impl FnOnce(&mut Value),
    expected: (ProviderSemanticSourceErrorKind, &str, &str),
) {
    let mut source = source_value();
    mutate(&mut source);
    assert_source_failure_tuple(&source, expected);
}

fn assert_source_failure_tuple(
    source: &Value,
    expected: (ProviderSemanticSourceErrorKind, &str, &str),
) {
    let text = serde_json::to_string(source).expect("mutated source serializes");
    let error =
        parse_provider_semantic_source_v1(&text).expect_err("invalid semantic source must fail");
    assert_eq!((error.kind(), error.subject(), error.reference()), expected);
}

#[test]
fn checked_echo_provider_semantic_source_validates() {
    let validated = parse_provider_semantic_source_v1(SOURCE)
        .expect("checked Echo provider semantic source validates");
    let source = validated.source();

    assert_eq!(source.api_version, ECHO_PROVIDER_SEMANTIC_SOURCE_API_V1);
    assert_eq!(source.coordinate, "echo.semantic-schema@1");
    assert_eq!(
        source
            .package_manifest
            .components
            .iter()
            .map(|component| {
                (
                    component.role.as_str(),
                    component.kind,
                    component.coordinate.as_str(),
                    component.contract.as_str(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "lowerer.echo-dpo",
                ProviderComponentKind::Lowerer,
                "echo.dpo.lowerer/component@1",
                "edict:target-provider/lowerer@1.0.0",
            ),
            (
                "verifier.echo-dpo",
                ProviderComponentKind::Verifier,
                "echo.dpo.verifier/component@1",
                "edict:target-provider/verifier@1.0.0",
            ),
        ]
    );
    assert_eq!(source.operations.len(), 1);
    assert_eq!(source.operations[0].identity.coordinate, "a.b@1.t");
    assert_eq!(source.operations[0].effect, "target.replace");
    assert_eq!(source.types.len(), 6);
    assert_eq!(
        source
            .types
            .iter()
            .map(|declaration| declaration.identity.coordinate.as_str())
            .collect::<Vec<_>>(),
        vec![
            "a.b@1.Id",
            "a.b@1.Input",
            "a.b@1.Output",
            "a.b@1.Receipt",
            "domain.WriteRejected.Payload",
            "target.replace.rejected",
        ]
    );
    assert!(matches!(
        &source.types[0].shape,
        SemanticTypeShape::CoreStringAlias {
            max_scalar_values: 16,
            canonical: CoreStringCanonicalization::RawUtf8,
        }
    ));
    assert_eq!(
        source.types[0].shape.core_type_coordinate().as_deref(),
        Some("String<max=16,canonical=raw-utf8>")
    );
    assert_eq!(
        source.types[0].shape.accepts_raw_string(&"é".repeat(16)),
        Some(true)
    );
    assert_eq!(
        source.types[0].shape.accepts_raw_string(&"é".repeat(17)),
        Some(false)
    );
    for coordinate in ["domain.WriteRejected.Payload", "target.replace.rejected"] {
        let declaration = source
            .types
            .iter()
            .find(|declaration| declaration.identity.coordinate == coordinate)
            .expect("empty payload type is declared");
        assert!(matches!(
            &declaration.shape,
            SemanticTypeShape::Record { fields } if fields.is_empty()
        ));
    }
    assert_eq!(source.write_classes[0].identity.coordinate, "replace");
    assert!(source
        .generated_artifacts
        .iter()
        .filter(|artifact| artifact.kind == GeneratedArtifactKind::AuthorityFacts)
        .all(|artifact| artifact.contract_owner.as_deref() == Some("flyingrobots/edict#157")));
    assert_eq!(
        source.obstructions[0].identity.coordinate,
        "domain.WriteRejected"
    );
    assert_eq!(
        source.obstructions[0].authority_class,
        AuthorityClass::DomainMappable
    );
    assert_eq!(
        source.obstructions[0].payload_schema,
        "domain.WriteRejected.Payload"
    );

    let effect = &source.effects[0];
    assert_eq!(effect.identity.coordinate, "target.replace");
    assert_eq!(effect.parameter_types, ["a.b@1.Id"]);
    assert_eq!(effect.result_type, "a.b@1.Receipt");
    assert_eq!(effect.execution_class, ExecutionClass::Runtime);
    assert_eq!(effect.effect_kind_hint, EffectKindHint::Replace);
    assert_eq!(effect.guard_kinds, ["precommit-atomic"]);
    assert!(effect.guard_support);
    assert_eq!(effect.footprint_obligation, "target.replace.footprint");
    assert_eq!(effect.cost_obligation, "target.replace.cost");
    assert_eq!(effect.failures[0].key, "rejected");
    assert_eq!(
        effect.failures[0].domain,
        "echo.edict-provider/effect-failure/v1"
    );
    assert_eq!(
        effect.failures[0].authority,
        "echo.provider-target-metadata@1"
    );
    assert_eq!(
        effect.failures[0].authority_class,
        AuthorityClass::DomainMappable
    );
    assert_eq!(effect.failures[0].payload_type, "target.replace.rejected");

    let profile = &source.profiles[0];
    assert_eq!(profile.identity.coordinate, "continuum.profile.write/v1");
    assert_eq!(profile.source_names, ["p.effectful"]);
    assert_eq!(profile.allowed_write_classes, ["replace"]);
    assert_eq!(profile.guard_kinds, ["precommit-atomic"]);
    assert_eq!(profile.atomicity, "atomic");
    assert!(profile.postcondition_support);
    assert_eq!(
        profile.optic_template.optic_kind,
        OpticKind::AffectReintegration
    );
    assert_eq!(profile.optic_template.boundary_kind, BoundaryKind::Affect);
    assert_eq!(
        profile.optic_template.support_policy,
        "continuum.support.carry-or-obstruct/v1"
    );
    assert_eq!(
        profile.optic_template.loss_disposition,
        "continuum.support.reject-on-loss/v1"
    );
    assert_eq!(
        profile.optic_template.aperture_requirement,
        Some(
            ApertureRequirementDeclaration::AbstractFootprintObligation {
                reference: "target.replace.footprint".to_owned(),
            }
        )
    );
    assert_eq!(
        profile.effect_predicate,
        "echo.dpo.operation-mode.replace-only/v1"
    );
    assert_eq!(profile.optic_contract, "replace-point");
    assert!(source.direct_adapters.is_empty());
    assert_eq!(source.budgets[0].max_steps, 8);
    assert_eq!(source.budgets[0].max_allocated_bytes, 1024);
    assert_eq!(source.budgets[0].max_output_bytes, 256);
    assert_eq!(source.capabilities[0].effect, "target.replace");
    assert_eq!(source.capabilities[0].effect_kind, EffectKindHint::Replace);
    assert_eq!(source.capabilities[0].write_class, "replace");
    assert!(source.capabilities[0].guard_support);
    assert_eq!(
        source.capabilities[0].footprint_template,
        "target.replace.footprint"
    );
    assert_eq!(source.capabilities[0].cost_template, "target.replace.cost");
    assert_eq!(
        source.capabilities[0].semantic_discharge.effect_kind_hint,
        EffectKindHint::Replace
    );
    assert_eq!(
        source.capabilities[0]
            .semantic_discharge
            .footprint_obligation,
        "target.replace.footprint"
    );
    assert_eq!(
        source.capabilities[0].semantic_discharge.cost_obligation,
        "target.replace.cost"
    );
    assert!(source.capabilities[0].can_participate_in_atomic_guard);
    assert_eq!(source.capabilities[0].target_profile, "echo.dpo@1");
    assert_eq!(source.capabilities[0].target_ir_domain, "echo.span-ir/v1");
    assert_eq!(source.operations[0].obstruction_mappings.len(), 1);
    assert_eq!(
        source.operations[0].obstruction_mappings[0].failure,
        "rejected"
    );
    assert_eq!(
        source.operations[0].obstruction_mappings[0].obstruction,
        "domain.WriteRejected"
    );

    let authority_kind = |coordinate: &str| {
        source
            .authority_sources
            .iter()
            .find(|authority| authority.coordinate == coordinate)
            .map(|authority| authority.kind)
            .expect("fact authority is declared")
    };
    let fact_contracts = [
        (
            &source.types[0].identity,
            "echo.edict-provider/value/v1",
            AuthoritySourceKind::EchoSemanticDeclaration,
        ),
        (
            &source.write_classes[0].identity,
            "echo.edict-provider/write-class/v1",
            AuthoritySourceKind::TargetMetadata,
        ),
        (
            &source.obstructions[0].identity,
            "echo.edict-provider/obstruction/v1",
            AuthoritySourceKind::EchoSemanticDeclaration,
        ),
        (
            &source.effects[0].identity,
            "echo.edict-provider/semantic-effect/v1",
            AuthoritySourceKind::EchoSemanticDeclaration,
        ),
        (
            &source.profiles[0].identity,
            "echo.edict-provider/operation-profile/v1",
            AuthoritySourceKind::TargetMetadata,
        ),
        (
            &source.budgets[0].identity,
            "echo.edict-provider/core-budget/v1",
            AuthoritySourceKind::EchoSemanticDeclaration,
        ),
        (
            &source.capabilities[0].identity,
            "echo.edict-provider/target-intrinsic/v1",
            AuthoritySourceKind::TargetMetadata,
        ),
        (
            &source.operations[0].identity,
            "echo.edict-provider/operation/v1",
            AuthoritySourceKind::EchoSemanticDeclaration,
        ),
    ];
    for (identity, domain, kind) in fact_contracts {
        assert_eq!(identity.domain, domain);
        assert_eq!(authority_kind(&identity.authority), kind);
    }
    assert_eq!(
        authority_kind(&effect.failures[0].authority),
        AuthoritySourceKind::TargetMetadata
    );
    assert_eq!(
        authority_kind(&source.lawpack_projection.authority),
        AuthoritySourceKind::EchoSemanticDeclaration
    );
    assert_eq!(
        authority_kind(&source.target_profile_projection.authority),
        AuthoritySourceKind::TargetMetadata
    );

    let generated = source
        .generated_artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.role.as_str(),
                artifact.kind,
                artifact.coordinate.as_str(),
                artifact.schema_contract.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        generated,
        vec![
            (
                "authority-facts.echo-dpo",
                GeneratedArtifactKind::AuthorityFacts,
                "echo.dpo-authority-facts@1",
                "edict.authority-facts/v1",
            ),
            (
                "authority-facts.echo-lawpack",
                GeneratedArtifactKind::AuthorityFacts,
                "echo.dpo-lawpack-authority-facts@1",
                "edict.authority-facts/v1",
            ),
            (
                "generated-artifact-profile.echo-dpo-registration",
                GeneratedArtifactKind::GeneratedArtifactProfile,
                "echo.dpo.registration/v1",
                "echo.generated-artifact-profile/v1",
            ),
            (
                "lawpack.echo-dpo",
                GeneratedArtifactKind::Lawpack,
                "echo.dpo-lawpack@1",
                "edict.lawpack/v1",
            ),
            (
                "provenance.provider-generation",
                GeneratedArtifactKind::GenerationProvenance,
                "echo.edict-provider-generation-provenance@1",
                "wesley:GenerationProvenanceManifestV1",
            ),
            (
                "review.provider-generation",
                GeneratedArtifactKind::ReviewArtifact,
                "echo.edict-provider-generation-review@1",
                "wesley:GenerationReviewV1",
            ),
            (
                "schema.echo-provider-artifacts",
                GeneratedArtifactKind::ArtifactSchema,
                "echo.provider-artifacts.cddl@1",
                "selfContainedCddlV1",
            ),
            (
                "target-profile.echo-dpo",
                GeneratedArtifactKind::TargetProfile,
                "echo.dpo@1",
                "edict.target-profile/v1",
            ),
        ]
    );
    let authority_fact_sources = source
        .generated_artifacts
        .iter()
        .filter_map(|artifact| {
            artifact.authority_fact_source.as_ref().map(|fact_source| {
                (
                    artifact.role.as_str(),
                    fact_source.kind,
                    fact_source.coordinate.as_str(),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        authority_fact_sources,
        vec![
            (
                "authority-facts.echo-dpo",
                AuthorityFactSourceKind::TargetProfile,
                "echo.dpo@1",
            ),
            (
                "authority-facts.echo-lawpack",
                AuthorityFactSourceKind::Lawpack,
                "echo.dpo-lawpack@1",
            ),
        ]
    );
    assert_eq!(
        (
            source.package_manifest.role.as_str(),
            source.package_manifest.coordinate.as_str(),
            source.package_manifest.schema_contract.as_str(),
            source.package_manifest.provider_abi.as_str(),
            source.package_manifest.provider_coordinate.as_str(),
        ),
        (
            "provider-manifest.echo",
            "echo.edict-provider-manifest@1",
            "edict.provider-manifest/v1",
            "edict:target-provider@1.0.0",
            "echo.edict-provider@1",
        )
    );
    let generation_provenance = source
        .generated_artifacts
        .iter()
        .find(|artifact| artifact.kind == GeneratedArtifactKind::GenerationProvenance)
        .expect("generation provenance is a generated package member");
    assert_eq!(
        generation_provenance.contract_owner.as_deref(),
        Some("flyingrobots/wesley#728")
    );
    let generation_review = source
        .generated_artifacts
        .iter()
        .find(|artifact| artifact.kind == GeneratedArtifactKind::ReviewArtifact)
        .expect("generation review is a generated package member");
    assert_eq!(
        generation_review.contract_owner.as_deref(),
        Some("flyingrobots/wesley#728")
    );
    assert_eq!(source.artifact_resources.len(), 19);
    assert_eq!(
        source
            .artifact_resources
            .iter()
            .filter(|resource| resource.provision == ArtifactResourceProvision::External)
            .map(|resource| resource.coordinate.as_str())
            .collect::<Vec<_>>(),
        vec![
            "edict.canonical-cbor/v1",
            "edict.determinism/v1",
            "edict.diagnostics/v1",
            "edict.fuel/v1",
            "edict.wasm-component/v1",
        ]
    );
    assert_eq!(source.lawpack_projection.artifact_role, "lawpack.echo-dpo");
    assert_eq!(source.lawpack_projection.target_adapters.len(), 1);
    assert_eq!(
        source.lawpack_projection.target_adapters[0].effects,
        ["target.replace"]
    );
    assert_eq!(
        source.target_profile_projection.artifact_role,
        "target-profile.echo-dpo"
    );
    assert_eq!(
        source
            .target_profile_projection
            .generated_artifact_profile_roles,
        ["generated-artifact-profile.echo-dpo-registration"]
    );

    let outputs = source
        .invocation_outputs
        .iter()
        .map(|output| (output.role.as_str(), output.kind, output.domain.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        outputs,
        vec![
            (
                "generated.echo-dpo",
                InvocationOutputKind::GeneratedArtifact,
                "echo.generated-artifact/v1",
            ),
            (
                "review.echo-dpo",
                InvocationOutputKind::ReviewPayload,
                "echo.review-payload/v1",
            ),
            (
                "target-ir.echo-dpo",
                InvocationOutputKind::TargetIr,
                "edict.target-ir.artifact/v1",
            ),
            (
                "verifier-report.echo-dpo",
                InvocationOutputKind::VerifierReport,
                "echo.verifier-report/v1",
            ),
        ]
    );
    assert_eq!(
        source
            .invocation_inputs
            .iter()
            .map(|input| (input.role.as_str(), input.kind, input.domain.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (
                "authority-facts.echo-dpo",
                InvocationInputKind::AuthorityFacts,
                "edict.authority-facts/v1",
            ),
            (
                "authority-facts.echo-lawpack",
                InvocationInputKind::AuthorityFacts,
                "edict.authority-facts/v1",
            ),
            (
                "core.echo-provider",
                InvocationInputKind::Core,
                "edict.core.module/v1",
            ),
            (
                "lawpack.echo-dpo",
                InvocationInputKind::Lawpack,
                "edict.lawpack/v1",
            ),
            (
                "lowerability.echo-dpo",
                InvocationInputKind::LowerabilityFacts,
                "edict.lowering-requirements/v1",
            ),
            (
                "target-ir.echo-dpo",
                InvocationInputKind::TargetIr,
                "edict.target-ir.artifact/v1",
            ),
            (
                "target-profile.echo-dpo",
                InvocationInputKind::TargetProfile,
                "edict.target-profile/v1",
            ),
        ]
    );
    assert_eq!(
        source
            .schema_bindings
            .iter()
            .map(|binding| (binding.domain.as_str(), binding.root_rule.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (
                "echo.dpo-lawpack.adapter.echo-dpo@1",
                "echo-provider-lawpack-target-adapter",
            ),
            (
                "echo.dpo-lawpack.compatibility@1",
                "echo-provider-lawpack-compatibility",
            ),
            ("echo.dpo-lawpack.exports@1", "lawpack-exports"),
            (
                "echo.dpo-lawpack.verifier@1",
                "echo-provider-lawpack-verifier",
            ),
            ("echo.dpo.bundle/v1", "echo-dpo-bundle"),
            ("echo.dpo.cost/v1", "echo-dpo-cost"),
            ("echo.dpo.fixtures/v1", "echo-provider-conformance-corpus",),
            ("echo.dpo.footprint/v1", "echo-dpo-footprint"),
            ("echo.dpo.intrinsics/v1", "intrinsics-document"),
            ("echo.dpo.lowerer/v1", "echo-dpo-lowerer"),
            ("echo.dpo.obstructions/v1", "echo-dpo-obstructions"),
            (
                "echo.dpo.operation-profiles/v1",
                "operation-profiles-document",
            ),
            ("echo.dpo.verifier/v1", "echo-dpo-verifier"),
            (
                "echo.generated-artifact-profile/v1",
                "generated-artifact-profile",
            ),
            ("echo.generated-artifact/v1", "generated-artifact"),
            ("echo.review-payload/v1", "review-payload"),
            ("echo.span-ir/v1", "echo-span-ir"),
            ("echo.verifier-report/v1", "verifier-report"),
            ("edict.authority-facts/v1", "authority-facts"),
            ("edict.core.module/v1", "core-module"),
            ("edict.lawpack/v1", "lawpack-manifest"),
            ("edict.lowering-requirements/v1", "lowering-requirements"),
            ("edict.target-ir.artifact/v1", "target-ir-artifact"),
            ("edict.target-profile/v1", "target-profile-manifest"),
        ]
    );
}

#[test]
fn generated_artifacts_require_their_owner_schema_bindings() {
    assert_failure_tuple(
        |source| {
            let bindings = source["schemaBindings"]
                .as_array_mut()
                .expect("schema bindings");
            let index = bindings
                .iter()
                .position(|binding| {
                    binding["domain"] == Value::String("echo.generated-artifact-profile/v1".into())
                })
                .expect("generated-artifact-profile binding");
            bindings.remove(index);
        },
        (
            ProviderSemanticSourceErrorKind::MissingSchemaBinding,
            "generated-artifact-profile.echo-dpo-registration",
            "echo.generated-artifact-profile/v1",
        ),
    );
    assert_failure_tuple(
        |source| {
            let bindings = source["schemaBindings"]
                .as_array_mut()
                .expect("schema bindings");
            let index = bindings
                .iter()
                .position(|binding| {
                    binding["domain"] == Value::String("echo.dpo.fixtures/v1".into())
                })
                .expect("generated resource binding");
            bindings.remove(index);
        },
        (
            ProviderSemanticSourceErrorKind::MissingSchemaBinding,
            "resource.conformance-corpus",
            "echo.dpo.fixtures/v1",
        ),
    );
    assert_failure_tuple(
        |source| {
            let binding = source["schemaBindings"]
                .as_array_mut()
                .expect("schema bindings")
                .iter_mut()
                .find(|binding| {
                    binding["domain"] == Value::String("echo.generated-artifact-profile/v1".into())
                })
                .expect("generated-artifact-profile binding");
            binding["rootRule"] = Value::String("wrong-profile-root".into());
        },
        (
            ProviderSemanticSourceErrorKind::SchemaRootMismatch,
            "echo.generated-artifact-profile/v1",
            "wrong-profile-root",
        ),
    );
    assert_failure_tuple(
        |source| {
            let binding = source["schemaBindings"]
                .as_array_mut()
                .expect("schema bindings")
                .iter_mut()
                .find(|binding| binding["domain"] == Value::String("echo.dpo.fixtures/v1".into()))
                .expect("generated resource binding");
            binding["rootRule"] = Value::String("wrong-resource-root".into());
        },
        (
            ProviderSemanticSourceErrorKind::SchemaRootMismatch,
            "resource.conformance-corpus",
            "wrong-resource-root",
        ),
    );
}

#[test]
fn set_like_source_reordering_preserves_validated_semantics() {
    let mut ordered = source_value();
    ordered["types"][0]["shape"]["fields"]
        .as_array_mut()
        .expect("record fields are a set")
        .push(json!({"name": "alternate", "type": "a.b@1.Id"}));
    ordered["writeClasses"]
        .as_array_mut()
        .expect("write classes are a set")
        .push(json!({
            "identity": {
                "coordinate": "read",
                "domain": "echo.edict-provider/write-class/v1",
                "authority": "echo.provider-target-metadata@1"
            }
        }));
    ordered["obstructions"]
        .as_array_mut()
        .expect("obstructions are a set")
        .push(json!({
            "identity": {
                "coordinate": "domain.OtherRejection",
                "domain": "echo.edict-provider/obstruction/v1",
                "authority": "echo.provider-semantic-declaration@1"
            },
            "authorityClass": "domainMappable",
            "payloadSchema": "domain.WriteRejected.Payload"
        }));
    ordered["effects"][0]["guardKinds"]
        .as_array_mut()
        .expect("effect guards are a set")
        .push(Value::String("preflight".to_owned()));
    ordered["effects"][0]["failures"]
        .as_array_mut()
        .expect("effect failures are a set")
        .push(json!({
            "key": "other",
            "domain": "echo.edict-provider/effect-failure/v1",
            "authority": "echo.provider-target-metadata@1",
            "authorityClass": "domainMappable",
            "payloadType": "target.replace.rejected"
        }));
    ordered["profiles"][0]["sourceNames"]
        .as_array_mut()
        .expect("profile source names are a set")
        .push(Value::String("p.write".to_owned()));
    ordered["profiles"][0]["allowedWriteClasses"]
        .as_array_mut()
        .expect("allowed write classes are a set")
        .push(Value::String("read".to_owned()));
    ordered["profiles"][0]["guardKinds"]
        .as_array_mut()
        .expect("profile guards are a set")
        .push(Value::String("preflight".to_owned()));
    ordered["operations"][0]["obstructionMappings"]
        .as_array_mut()
        .expect("obstruction mappings are a key set")
        .push(json!({
            "failure": "other",
            "obstruction": "domain.OtherRejection"
        }));
    let mut second_effect = ordered["effects"][0].clone();
    second_effect["identity"]["coordinate"] = Value::String("target.ensure".to_owned());
    second_effect["effectKindHint"] = Value::String("ensure".to_owned());
    second_effect["footprintObligation"] = Value::String("target.ensure.footprint".to_owned());
    second_effect["costObligation"] = Value::String("target.ensure.cost".to_owned());
    ordered["effects"]
        .as_array_mut()
        .expect("effects")
        .push(second_effect);
    let mut second_capability = ordered["capabilities"][0].clone();
    second_capability["identity"]["coordinate"] = Value::String("echo.dpo@1.ensure".to_owned());
    second_capability["effect"] = Value::String("target.ensure".to_owned());
    second_capability["effectKind"] = Value::String("ensure".to_owned());
    second_capability["footprintTemplate"] =
        Value::String("echo.dpo.ensure-footprint/v1".to_owned());
    second_capability["costTemplate"] = Value::String("echo.dpo.ensure-cost/v1".to_owned());
    second_capability["semanticDischarge"]["effectKindHint"] = Value::String("ensure".to_owned());
    second_capability["semanticDischarge"]["footprintObligation"] =
        Value::String("target.ensure.footprint".to_owned());
    second_capability["semanticDischarge"]["costObligation"] =
        Value::String("target.ensure.cost".to_owned());
    ordered["capabilities"]
        .as_array_mut()
        .expect("capabilities")
        .push(second_capability);
    ordered["lawpackProjection"]["targetAdapters"][0]["effects"]
        .as_array_mut()
        .expect("adapter effects")
        .push(Value::String("target.ensure".to_owned()));

    let ordered_text = serde_json::to_string(&ordered).expect("ordered source serializes");
    let baseline = parse_provider_semantic_source_v1(&ordered_text)
        .expect("expanded baseline semantic source validates");
    let mut reordered = ordered;

    for pointer in [
        "/types/0/shape/fields",
        "/effects/0/guardKinds",
        "/effects/0/failures",
        "/profiles/0/sourceNames",
        "/profiles/0/allowedWriteClasses",
        "/profiles/0/guardKinds",
        "/operations/0/obstructionMappings",
        "/lawpackProjection/acceptedCoreAbis",
        "/lawpackProjection/dependencies",
        "/lawpackProjection/targetAdapters",
        "/lawpackProjection/targetAdapters/0/effects",
        "/targetProfileProjection/acceptedCoreAbis",
        "/targetProfileProjection/generatedArtifactProfileRoles",
        "/targetProfileProjection/acceptedLawpackAdapterAbis",
        "/packageManifest/components",
    ] {
        reordered
            .pointer_mut(pointer)
            .and_then(Value::as_array_mut)
            .expect("nested set-like field is an array")
            .reverse();
    }

    for key in [
        "authoritySources",
        "types",
        "writeClasses",
        "obstructions",
        "effects",
        "profiles",
        "budgets",
        "capabilities",
        "directAdapters",
        "operations",
        "artifactResources",
        "generatedArtifacts",
        "invocationInputs",
        "invocationOutputs",
        "schemaBindings",
    ] {
        reordered[key]
            .as_array_mut()
            .expect("set-like declaration family is an array")
            .reverse();
    }

    let text = serde_json::to_string(&reordered).expect("reordered source serializes");
    let normalized =
        parse_provider_semantic_source_v1(&text).expect("reordered semantic source validates");
    assert_eq!(baseline, normalized);
}

#[test]
fn conflicting_duplicate_coordinate_fails_deterministically() {
    let mut source = source_value();
    let mut duplicate = source["effects"][0].clone();
    duplicate["resultType"] = Value::String("echo.unknown.Result@1".to_owned());
    source["effects"]
        .as_array_mut()
        .expect("effects array")
        .push(duplicate);
    let expected = (
        ProviderSemanticSourceErrorKind::DuplicateCoordinate,
        "effects",
        "target.replace",
    );
    assert_source_failure_tuple(&source, expected);
    source["effects"]
        .as_array_mut()
        .expect("effects array")
        .reverse();
    assert_source_failure_tuple(&source, expected);
}

#[test]
fn recursive_type_graphs_fail_with_deterministic_edges() {
    assert_failure_tuple(
        |source| {
            source["types"][0]["shape"]["fields"][0]["type"] =
                Value::String("a.b@1.Input".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::UnboundedTypeGraph,
            "a.b@1.Input",
            "a.b@1.Input",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["types"][0]["shape"]["fields"][0]["type"] =
                Value::String("a.b@1.Output".to_owned());
            source["types"][2]["shape"]["fields"][0]["type"] =
                Value::String("a.b@1.Input".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::UnboundedTypeGraph,
            "a.b@1.Input",
            "a.b@1.Output",
        ),
    );

    let mut source = source_value();
    for index in [0, 2] {
        let coordinate = source["types"][index]["identity"]["coordinate"]
            .as_str()
            .expect("type coordinate")
            .to_owned();
        source["types"][index]["shape"]["fields"][0]["type"] = Value::String(coordinate);
    }
    let expected = (
        ProviderSemanticSourceErrorKind::UnboundedTypeGraph,
        "a.b@1.Input",
        "a.b@1.Input",
    );
    assert_source_failure_tuple(&source, expected);
    source["types"]
        .as_array_mut()
        .expect("types array")
        .reverse();
    assert_source_failure_tuple(&source, expected);
}

#[test]
fn core_string_alias_keeps_edict_core_authority() {
    assert_failure(
        |source| {
            source["types"][3]["shape"]["maxBytes"] = json!(16);
        },
        ProviderSemanticSourceErrorKind::MalformedDocument,
    );
    assert_failure(
        |source| {
            source["types"][3]["shape"]
                .as_object_mut()
                .expect("core alias shape")
                .remove("canonical");
        },
        ProviderSemanticSourceErrorKind::MalformedDocument,
    );
    assert_failure(
        |source| {
            source["types"][3]["shape"]["canonical"] = Value::String("unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::MalformedDocument,
    );
    for coordinate in ["String<max=16>", "Bool", "List<a.b@1.Id,max=1>"] {
        assert_failure_tuple(
            |source| {
                source["types"][3]["identity"]["coordinate"] = Value::String(coordinate.to_owned());
            },
            (
                ProviderSemanticSourceErrorKind::CoreTypeAuthorityMismatch,
                coordinate,
                "edict.core/v1",
            ),
        );
    }
}

#[test]
fn stale_relocated_sdl_cannot_override_declared_coordinate() {
    assert_failure(
        |source| {
            source["authoritySources"]
                .as_array_mut()
                .expect("authority source array")
                .push(json!({
                        "coordinate": "echo.stale-relocated-sdl@1",
                        "kind": "graphql",
                        "artifact": "schemas/wesley-relocated/echo.graphql"
                }));
            source["types"][0]["identity"]["authority"] =
                Value::String("echo.stale-relocated-sdl@1".to_owned());
        },
        ProviderSemanticSourceErrorKind::NonAuthoritativeSource,
    );
}

#[test]
fn authority_locators_must_be_authored_repository_paths() {
    for locator in [
        "/tmp/semantic.json",
        "../semantic.json",
        "https://example.com/semantic.json",
        "schemas\\semantic.json",
        "target/generated/semantic.json",
    ] {
        assert_failure_tuple(
            |source| {
                source["authoritySources"][0]["artifact"] = Value::String(locator.to_owned());
            },
            (
                ProviderSemanticSourceErrorKind::NonAuthoritativeSource,
                "echo.provider-semantic-declaration@1",
                locator,
            ),
        );
    }
}

#[test]
fn fact_domains_and_authority_families_fail_closed() {
    let wrong_domain = "echo.edict-provider/wrong/v1";
    for (pointer, coordinate) in [
        ("/types/0/identity", "a.b@1.Input"),
        ("/writeClasses/0/identity", "replace"),
        ("/obstructions/0/identity", "domain.WriteRejected"),
        ("/effects/0/identity", "target.replace"),
        ("/profiles/0/identity", "continuum.profile.write/v1"),
        ("/budgets/0/identity", "p.tiny"),
        ("/capabilities/0/identity", "echo.dpo@1.replace"),
        ("/operations/0/identity", "a.b@1.t"),
    ] {
        assert_failure_tuple(
            |source| {
                source.pointer_mut(pointer).expect("fact identity")["domain"] =
                    Value::String(wrong_domain.to_owned());
            },
            (
                ProviderSemanticSourceErrorKind::FactDomainMismatch,
                coordinate,
                wrong_domain,
            ),
        );
    }
    assert_failure_tuple(
        |source| {
            source["effects"][0]["failures"][0]["domain"] = Value::String(wrong_domain.to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::FactDomainMismatch,
            "target.replace",
            wrong_domain,
        ),
    );

    for (pointer, coordinate, authority) in [
        (
            "/types/0/identity",
            "a.b@1.Input",
            "echo.provider-target-metadata@1",
        ),
        (
            "/writeClasses/0/identity",
            "replace",
            "echo.provider-semantic-declaration@1",
        ),
        (
            "/obstructions/0/identity",
            "domain.WriteRejected",
            "echo.provider-target-metadata@1",
        ),
        (
            "/effects/0/identity",
            "target.replace",
            "echo.provider-target-metadata@1",
        ),
        (
            "/profiles/0/identity",
            "continuum.profile.write/v1",
            "echo.provider-semantic-declaration@1",
        ),
        (
            "/budgets/0/identity",
            "p.tiny",
            "echo.provider-target-metadata@1",
        ),
        (
            "/capabilities/0/identity",
            "echo.dpo@1.replace",
            "echo.provider-semantic-declaration@1",
        ),
        (
            "/operations/0/identity",
            "a.b@1.t",
            "echo.provider-target-metadata@1",
        ),
    ] {
        assert_failure_tuple(
            |source| {
                source.pointer_mut(pointer).expect("fact identity")["authority"] =
                    Value::String(authority.to_owned());
            },
            (
                ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
                coordinate,
                authority,
            ),
        );
    }
    assert_failure_tuple(
        |source| {
            source["effects"][0]["failures"][0]["authority"] =
                Value::String("echo.provider-semantic-declaration@1".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
            "target.replace",
            "echo.provider-semantic-declaration@1",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["lawpackProjection"]["authority"] =
                Value::String("echo.provider-target-metadata@1".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
            "lawpackProjection",
            "echo.provider-target-metadata@1",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["targetProfileProjection"]["authority"] =
                Value::String("echo.provider-semantic-declaration@1".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
            "targetProfileProjection",
            "echo.provider-semantic-declaration@1",
        ),
    );

    let adapter = json!({
        "identity": {
            "coordinate": "echo.dpo@1.replace-adapter",
            "domain": "echo.edict-provider/direct-adapter/v1",
            "authority": "echo.provider-target-metadata@1"
        },
        "consumesEffect": "target.replace",
        "capability": "echo.dpo@1.replace",
        "emitsEffects": []
    });
    assert_failure_tuple(
        |source| {
            source["directAdapters"] = json!([adapter.clone()]);
            source["directAdapters"][0]["identity"]["domain"] =
                Value::String(wrong_domain.to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::FactDomainMismatch,
            "echo.dpo@1.replace-adapter",
            wrong_domain,
        ),
    );
    assert_failure_tuple(
        |source| {
            source["directAdapters"] = json!([adapter.clone()]);
            source["directAdapters"][0]["identity"]["authority"] =
                Value::String("echo.provider-semantic-declaration@1".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
            "echo.dpo@1.replace-adapter",
            "echo.provider-semantic-declaration@1",
        ),
    );
}

#[test]
fn unknown_semantic_references_fail_with_stable_kinds() {
    assert_failure(
        |source| {
            source["operations"][0]["inputType"] = Value::String("type.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownType,
    );
    assert_failure(
        |source| {
            source["operations"][0]["obstructionMappings"][0]["obstruction"] =
                Value::String("obstruction.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownObstruction,
    );
    assert_failure(
        |source| {
            source["operations"][0]["obstructionMappings"][0]["failure"] =
                Value::String("failure_unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownFailure,
    );
    assert_failure(
        |source| {
            source["operations"][0]["profile"] = Value::String("profile.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownProfile,
    );
    assert_failure(
        |source| {
            source["capabilities"][0]["effect"] = Value::String("effect.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownEffect,
    );
    assert_failure(
        |source| {
            source["capabilities"][0]["targetProfile"] =
                Value::String("target-profile.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownProfile,
    );
    assert_failure(
        |source| {
            source["operations"][0]["budget"] = Value::String("budget.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownBudget,
    );
    assert_failure(
        |source| {
            source["operations"][0]["implementation"]["capability"] =
                Value::String("capability.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownCapability,
    );
    assert_failure(
        |source| {
            source["operations"][0]["implementation"] = json!({
                "kind": "directAdapter",
                "adapter": "adapter.unknown"
            });
        },
        ProviderSemanticSourceErrorKind::UnknownAdapter,
    );
}

#[test]
fn semantic_source_display_uses_a_stable_kind_label() {
    let mut source = source_value();
    source["operations"][0]["implementation"]["capability"] =
        Value::String("capability.unknown".to_owned());
    let text = serde_json::to_string(&source).expect("mutated source serializes");

    let error = parse_provider_semantic_source_v1(&text).expect_err("unknown capability must fail");
    assert_eq!(
        error.to_string(),
        "provider semantic source unknown-capability: a.b@1.t -> capability.unknown"
    );
}

#[test]
fn strict_shape_and_set_duplicates_fail_deterministically() {
    assert_failure(
        |source| {
            source["unexpected"] = Value::Bool(true);
        },
        ProviderSemanticSourceErrorKind::MalformedDocument,
    );
    for pointer in [
        "/effects/0/guardKinds",
        "/profiles/0/sourceNames",
        "/profiles/0/allowedWriteClasses",
        "/profiles/0/guardKinds",
    ] {
        assert_failure(
            |source| {
                let values = source
                    .pointer_mut(pointer)
                    .and_then(Value::as_array_mut)
                    .expect("set field is an array");
                values.push(values[0].clone());
            },
            ProviderSemanticSourceErrorKind::DuplicateKey,
        );
    }
    for pointer in ["/effects/0/failures", "/operations/0/obstructionMappings"] {
        assert_failure(
            |source| {
                let values = source
                    .pointer_mut(pointer)
                    .and_then(Value::as_array_mut)
                    .expect("keyed declaration field is an array");
                values.push(values[0].clone());
            },
            ProviderSemanticSourceErrorKind::DuplicateKey,
        );
    }
}

#[test]
fn profile_source_names_are_globally_unique() {
    assert_failure_tuple(
        |source| {
            let mut profile = source["profiles"][0].clone();
            profile["identity"]["coordinate"] =
                Value::String("continuum.profile.write-secondary/v1".to_owned());
            source["profiles"]
                .as_array_mut()
                .expect("profiles")
                .push(profile);
        },
        (
            ProviderSemanticSourceErrorKind::DuplicateKey,
            "profiles.sourceNames",
            "p.effectful",
        ),
    );
}

#[test]
fn typed_failure_and_mapping_closure_fails_closed() {
    for key in ["bad-name", "1bad", "else"] {
        assert_failure_tuple(
            |source| {
                source["effects"][0]["failures"][0]["key"] = Value::String(key.to_owned());
                source["operations"][0]["obstructionMappings"][0]["failure"] =
                    Value::String(key.to_owned());
            },
            (
                ProviderSemanticSourceErrorKind::InvalidFailureKey,
                "target.replace",
                key,
            ),
        );
    }
    assert_failure(
        |source| {
            source["effects"][0]["failures"][0]["payloadType"] =
                Value::String("type.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownType,
    );
    assert_failure(
        |source| {
            source["obstructions"][0]["payloadSchema"] = Value::String("type.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownType,
    );
    assert_failure(
        |source| {
            source["effects"][0]["failures"][0]["authorityClass"] =
                Value::String("participantOwned".to_owned());
        },
        ProviderSemanticSourceErrorKind::ObstructionMappingMismatch,
    );
    assert_failure(
        |source| {
            source["operations"][0]["obstructionMappings"] = json!([]);
        },
        ProviderSemanticSourceErrorKind::ObstructionMappingMismatch,
    );
    assert_failure(
        |source| {
            source["effects"][0]["failures"][0]["payloadType"] =
                Value::String("a.b@1.Input".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnsupportedObstructionPayloadMapping,
    );
    assert_failure(
        |source| {
            source["obstructions"][0]["payloadSchema"] = Value::String("a.b@1.Input".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnsupportedObstructionPayloadMapping,
    );
}

#[test]
fn effect_profile_and_implementation_joins_fail_closed() {
    assert_failure(
        |source| {
            source["effects"][0]["parameterTypes"] = json!([]);
        },
        ProviderSemanticSourceErrorKind::UnsupportedEffectShape,
    );
    assert_failure(
        |source| {
            source["profiles"][0]["allowedWriteClasses"] = json!([]);
        },
        ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
    );
    assert_failure(
        |source| {
            source["profiles"][0]["guardKinds"] = json!([]);
        },
        ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
    );
    let mut independent_target_authority = source_value();
    independent_target_authority["capabilities"][0]["effectKind"] =
        Value::String("custom".to_owned());
    independent_target_authority["capabilities"][0]["footprintTemplate"] =
        Value::String("echo.dpo.replace-footprint/v1".to_owned());
    independent_target_authority["capabilities"][0]["costTemplate"] =
        Value::String("echo.dpo.replace-cost/v1".to_owned());
    let text = serde_json::to_string(&independent_target_authority)
        .expect("independent target authority serializes");
    parse_provider_semantic_source_v1(&text)
        .expect("target-authoritative facts need not equal advisory lawpack facts");
    assert_failure_tuple(
        |source| {
            source["capabilities"][0]["writeClass"] = Value::String("changed".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::UnknownWriteClass,
            "echo.dpo@1.replace",
            "changed",
        ),
    );
    for (field, value) in [
        ("effectKindHint", "create"),
        ("footprintObligation", "changed"),
        ("costObligation", "changed"),
    ] {
        assert_failure(
            |source| {
                source["capabilities"][0]["semanticDischarge"][field] =
                    Value::String(value.to_owned());
            },
            ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
        );
    }
    assert_failure_tuple(
        |source| {
            source["effects"][0]["executionClass"] = Value::String("proofOnly".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
            "echo.dpo@1.replace",
            "target.replace",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["capabilities"][0]["canParticipateInAtomicGuard"] = Value::Bool(false);
        },
        (
            ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
            "echo.dpo@1.replace",
            "target.replace",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["profiles"][0]["opticTemplate"]
                .as_object_mut()
                .expect("optic template")
                .remove("apertureRequirement");
        },
        (
            ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
            "a.b@1.t",
            "target.replace.footprint",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["profiles"][0]["opticTemplate"]["apertureRequirement"] = json!({
                "kind": "footprintCeiling",
                "ref": "target.replace.footprint"
            });
        },
        (
            ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
            "a.b@1.t",
            "target.replace.footprint",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["profiles"][0]["opticTemplate"]["apertureRequirement"]["ref"] =
                Value::String("changed".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
            "a.b@1.t",
            "changed",
        ),
    );
    for (field, value, reference) in [
        ("opticKind", "revelation", "revelation"),
        ("boundaryKind", "projection", "projection"),
    ] {
        assert_failure_tuple(
            |source| {
                source["profiles"][0]["opticTemplate"][field] = Value::String(value.to_owned());
            },
            (
                ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                "a.b@1.t",
                reference,
            ),
        );
    }
    assert_failure_tuple(
        |source| {
            source["writeClasses"][0]["identity"]["coordinate"] = json!("read");
            source["profiles"][0]["allowedWriteClasses"][0] = json!("read");
            source["effects"][0]["effectKindHint"] = json!("read");
            source["capabilities"][0]["effectKind"] = json!("read");
            source["capabilities"][0]["writeClass"] = json!("read");
            source["capabilities"][0]["semanticDischarge"]["effectKindHint"] = json!("read");
        },
        (
            ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
            "a.b@1.t",
            "affectReintegration",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["profiles"][0]["atomicity"] = Value::String("non-atomic".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
            "a.b@1.t",
            "non-atomic",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["writeClasses"]
                .as_array_mut()
                .expect("write classes")
                .push(json!({
                    "identity": {
                        "coordinate": "read",
                        "domain": "echo.edict-provider/write-class/v1",
                        "authority": "echo.provider-target-metadata@1"
                    }
                }));
            source["capabilities"][0]["writeClass"] = Value::String("read".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
            "a.b@1.t",
            "read",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["writeClasses"]
                .as_array_mut()
                .expect("write classes")
                .push(json!({
                    "identity": {
                        "coordinate": "read",
                        "domain": "echo.edict-provider/write-class/v1",
                        "authority": "echo.provider-target-metadata@1"
                    }
                }));
            let mut target_effect = source["effects"][0].clone();
            target_effect["identity"]["coordinate"] = Value::String("target.read".to_owned());
            target_effect["effectKindHint"] = Value::String("read".to_owned());
            target_effect["footprintObligation"] =
                Value::String("target.read.footprint".to_owned());
            target_effect["costObligation"] = Value::String("target.read.cost".to_owned());
            source["effects"]
                .as_array_mut()
                .expect("effects")
                .push(target_effect);
            source["capabilities"][0]["identity"]["coordinate"] =
                Value::String("echo.dpo@1.read".to_owned());
            source["capabilities"][0]["effect"] = Value::String("target.read".to_owned());
            source["capabilities"][0]["effectKind"] = Value::String("read".to_owned());
            source["capabilities"][0]["writeClass"] = Value::String("read".to_owned());
            source["capabilities"][0]["footprintTemplate"] =
                Value::String("target.read.footprint".to_owned());
            source["capabilities"][0]["costTemplate"] =
                Value::String("target.read.cost".to_owned());
            source["capabilities"][0]["semanticDischarge"]["effectKindHint"] =
                Value::String("read".to_owned());
            source["capabilities"][0]["semanticDischarge"]["footprintObligation"] =
                Value::String("target.read.footprint".to_owned());
            source["capabilities"][0]["semanticDischarge"]["costObligation"] =
                Value::String("target.read.cost".to_owned());
            source["directAdapters"] = json!([{
                "identity": {
                    "coordinate": "echo.dpo@1.replace-adapter",
                    "domain": "echo.edict-provider/direct-adapter/v1",
                    "authority": "echo.provider-target-metadata@1"
                },
                "consumesEffect": "target.replace",
                "capability": "echo.dpo@1.read",
                "emitsEffects": []
            }]);
            source["operations"][0]["implementation"] = json!({
                "kind": "directAdapter",
                "adapter": "echo.dpo@1.replace-adapter"
            });
        },
        (
            ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
            "a.b@1.t",
            "read",
        ),
    );
    assert_failure(
        |source| {
            let mut duplicate = source["capabilities"][0].clone();
            duplicate["identity"]["coordinate"] =
                Value::String("echo.dpo@1.replace-again".to_owned());
            source["capabilities"]
                .as_array_mut()
                .expect("capabilities array")
                .push(duplicate);
        },
        ProviderSemanticSourceErrorKind::AmbiguousEffectImplementation,
    );
    assert_failure_tuple(
        |source| {
            source["directAdapters"] = json!([{
                "identity": {
                    "coordinate": "echo.dpo@1.replace-adapter",
                    "domain": "echo.edict-provider/direct-adapter/v1",
                    "authority": "echo.provider-target-metadata@1"
                },
                "consumesEffect": "target.replace",
                "capability": "echo.dpo@1.replace",
                "emitsEffects": []
            }]);
        },
        (
            ProviderSemanticSourceErrorKind::AmbiguousEffectImplementation,
            "effectImplementations",
            "target.replace",
        ),
    );
    assert_failure(
        |source| {
            source["directAdapters"] = json!([{
                "identity": {
                    "coordinate": "echo.dpo@1.replace-adapter",
                    "domain": "echo.edict-provider/direct-adapter/v1",
                    "authority": "echo.provider-target-metadata@1"
                },
                "consumesEffect": "target.replace",
                "capability": "echo.dpo@1.replace",
                "emitsEffects": ["target.replace"]
            }]);
        },
        ProviderSemanticSourceErrorKind::UnsupportedAdapterChain,
    );
}

#[test]
fn every_runtime_effect_requires_exactly_one_implementation() {
    assert_failure_tuple(
        |source| {
            let mut effect = source["effects"][0].clone();
            effect["identity"]["coordinate"] = Value::String("target.unimplemented".to_owned());
            effect["effectKindHint"] = Value::String("ensure".to_owned());
            effect["footprintObligation"] =
                Value::String("target.unimplemented.footprint".to_owned());
            effect["costObligation"] = Value::String("target.unimplemented.cost".to_owned());
            source["effects"]
                .as_array_mut()
                .expect("effects")
                .push(effect);
            source["lawpackProjection"]["targetAdapters"][0]["effects"]
                .as_array_mut()
                .expect("adapter effects")
                .push(Value::String("target.unimplemented".to_owned()));
        },
        (
            ProviderSemanticSourceErrorKind::MissingEffectImplementation,
            "effectImplementations",
            "target.unimplemented",
        ),
    );
}

#[test]
fn lawpack_target_profile_selector_is_unique_and_order_independent() {
    let mut source = source_value();
    let mut duplicate = source["lawpackProjection"]["targetAdapters"][0].clone();
    duplicate["adapterResource"] = Value::String("resource.lawpack-target-adapter-copy".to_owned());
    source["lawpackProjection"]["targetAdapters"]
        .as_array_mut()
        .expect("target adapters")
        .push(duplicate);

    let expected = (
        ProviderSemanticSourceErrorKind::DuplicateKey,
        "lawpackProjection.targetAdapters.acceptedTargetProfileRole",
        "target-profile.echo-dpo",
    );
    assert_source_failure_tuple(&source, expected);
    source["lawpackProjection"]["targetAdapters"]
        .as_array_mut()
        .expect("target adapters")
        .reverse();
    assert_source_failure_tuple(&source, expected);
}

#[test]
fn one_target_profile_cannot_mix_inner_target_ir_domains() {
    assert_failure_tuple(
        |source| {
            let mut capability = source["capabilities"][0].clone();
            capability["identity"]["coordinate"] =
                Value::String("echo.dpo@1.replace-other".to_owned());
            capability["targetIrDomain"] = Value::String("echo.other-ir/v1".to_owned());
            source["capabilities"]
                .as_array_mut()
                .expect("capabilities")
                .push(capability);
        },
        (
            ProviderSemanticSourceErrorKind::TargetIrDomainMismatch,
            "echo.dpo@1",
            "echo.other-ir/v1",
        ),
    );
}

#[test]
fn generated_artifact_and_output_contracts_are_exact() {
    assert_failure(
        |source| {
            source["generatedArtifacts"][0]["schemaContract"] =
                Value::String("edict.wrong/v1".to_owned());
        },
        ProviderSemanticSourceErrorKind::ArtifactSchemaContractMismatch,
    );
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"]
                .as_array_mut()
                .expect("generated artifacts")
                .push(json!({
                    "role": "provider-manifest.echo-copy",
                    "kind": "providerManifest",
                    "coordinate": "echo.edict-provider-manifest-copy@1",
                    "schemaContract": "edict.provider-manifest/v1"
                }));
        },
        (
            ProviderSemanticSourceErrorKind::SelfReferentialManifestInventory,
            "provider-manifest.echo-copy",
            "echo.edict-provider-manifest-copy@1",
        ),
    );
    for field in ["providerAbi", "providerCoordinate"] {
        assert_failure_tuple(
            |source| {
                source["packageManifest"][field] = Value::String("wrong@1".to_owned());
            },
            (
                ProviderSemanticSourceErrorKind::ProviderManifestProjectionMismatch,
                "provider-manifest.echo",
                "wrong@1",
            ),
        );
    }
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"][0]
                .as_object_mut()
                .expect("authority facts")
                .remove("authorityFactSource");
        },
        (
            ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
            "authority-facts.echo-dpo",
            "authorityFactSource",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"][0]["contractOwner"] =
                Value::String("flyingrobots/echo#651".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::ArtifactContractOwnerMismatch,
            "authority-facts.echo-dpo",
            "flyingrobots/echo#651",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"][0]["authorityFactSource"]["coordinate"] =
                Value::String("echo.dpo-lawpack@1".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
            "authority-facts.echo-dpo",
            "echo.dpo-lawpack@1",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"][1]["authorityFactSource"] = json!({
                "kind": "targetProfile",
                "coordinate": "echo.dpo@1"
            });
        },
        (
            ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
            "generatedArtifacts",
            "lawpack",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"][3]["authorityFactSource"] = json!({
                "kind": "lawpack",
                "coordinate": "echo.dpo-lawpack@1"
            });
        },
        (
            ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
            "lawpack.echo-dpo",
            "echo.dpo-lawpack@1",
        ),
    );
    assert_failure(
        |source| {
            source["generatedArtifacts"][4]["schemaContract"] =
                Value::String("wesley:Unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::GenerationProvenanceContractMismatch,
    );
    assert_failure(
        |source| {
            source["generatedArtifacts"][4]["contractOwner"] =
                Value::String("flyingrobots/echo#651".to_owned());
        },
        ProviderSemanticSourceErrorKind::GenerationProvenanceContractMismatch,
    );
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"][5]["schemaContract"] =
                Value::String("wesley:Unknown".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::GenerationReviewContractMismatch,
            "review.provider-generation",
            "wesley:Unknown",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["generatedArtifacts"][5]["contractOwner"] =
                Value::String("flyingrobots/echo#651".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::GenerationReviewContractMismatch,
            "review.provider-generation",
            "flyingrobots/echo#651",
        ),
    );
    assert_failure(
        |source| {
            source["lawpackProjection"]["targetAdapters"] = json!([]);
        },
        ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
    );
    assert_failure(
        |source| {
            source["targetProfileProjection"]["intrinsicsResource"] =
                Value::String("resource.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
    );
    assert_failure(
        |source| {
            source["targetProfileProjection"]["generatedArtifactProfileRoles"] = json!([]);
        },
        ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
    );
    for index in 0..19 {
        assert_failure(
            |source| {
                source["artifactResources"][index]["schemaContract"] =
                    Value::String("echo.wrong/v1".to_owned());
            },
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
        );
    }
    assert_failure(
        |source| {
            source["invocationOutputs"][0]["domain"] =
                Value::String("echo.review-payload/v1".to_owned());
        },
        ProviderSemanticSourceErrorKind::OutputDomainMismatch,
    );
    assert_failure(
        |source| {
            source["schemaBindings"][2]["rootRule"] = Value::String("wrong-root".to_owned());
        },
        ProviderSemanticSourceErrorKind::SchemaRootMismatch,
    );
}

#[test]
fn package_component_declarations_fail_closed() {
    assert_failure_tuple(
        |source| {
            source["packageManifest"]["components"]
                .as_array_mut()
                .expect("package components")
                .remove(0);
        },
        (
            ProviderSemanticSourceErrorKind::ProviderComponentClosureMismatch,
            "packageManifest.components",
            "lowerer",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["packageManifest"]["components"][0]["contract"] =
                Value::String("edict:target-provider/verifier@1.0.0".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::ProviderComponentProjectionMismatch,
            "lowerer.echo-dpo",
            "edict:target-provider/verifier@1.0.0",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["packageManifest"]["components"][1]["kind"] =
                Value::String("lowerer".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::ProviderComponentClosureMismatch,
            "packageManifest.components",
            "lowerer:2",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["packageManifest"]["components"][1]["role"] =
                Value::String("lowerer.echo-dpo".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::DuplicateKey,
            "packageManifest.components.role",
            "lowerer.echo-dpo",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["packageManifest"]["components"][1]["coordinate"] =
                Value::String("echo.dpo.lowerer/component@1".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::DuplicateCoordinate,
            "packageManifest.components",
            "echo.dpo.lowerer/component@1",
        ),
    );
}

#[test]
fn invocation_inputs_and_outputs_require_declared_schema_roles_and_domains() {
    assert_failure_tuple(
        |source| {
            source["invocationInputs"]
                .as_array_mut()
                .expect("invocation inputs")
                .remove(2);
        },
        (
            ProviderSemanticSourceErrorKind::InvocationInputClosureMismatch,
            "invocationInputs",
            "core",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["invocationInputs"][2]["domain"] = Value::String("edict.wrong/v1".to_owned());
        },
        (
            ProviderSemanticSourceErrorKind::InputDomainMismatch,
            "core.echo-provider",
            "edict.wrong/v1",
        ),
    );
    assert_failure(
        |source| {
            source["invocationInputs"][0]["schemaRole"] =
                Value::String("schema.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownSchemaRole,
    );
    assert_failure(
        |source| {
            source["invocationOutputs"][0]["schemaRole"] =
                Value::String("schema.unknown".to_owned());
        },
        ProviderSemanticSourceErrorKind::UnknownSchemaRole,
    );
    assert_failure(
        |source| {
            source["schemaBindings"]
                .as_array_mut()
                .expect("schema bindings array")
                .remove(0);
        },
        ProviderSemanticSourceErrorKind::MissingSchemaBinding,
    );
    assert_failure_tuple(
        |source| {
            source["invocationOutputs"]
                .as_array_mut()
                .expect("invocation outputs")
                .remove(0);
        },
        (
            ProviderSemanticSourceErrorKind::InvocationOutputClosureMismatch,
            "invocationOutputs",
            "generatedArtifact",
        ),
    );
    assert_failure_tuple(
        |source| {
            source["schemaBindings"]
                .as_array_mut()
                .expect("schema bindings")
                .push(json!({
                    "domain": "echo.unused/v1",
                    "schemaRole": "schema.echo-provider-artifacts",
                    "format": "selfContainedCddlV1",
                    "rootRule": "unused"
                }));
        },
        (
            ProviderSemanticSourceErrorKind::UnexpectedSchemaBinding,
            "schemaBindings",
            "echo.unused/v1",
        ),
    );
}
