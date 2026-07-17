// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
//! Edict-native pre-execution readiness witness for the checked Echo package.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use edict_provider_host_wasmtime::{
    ProviderComponentHost, ProviderHostFailureKind, ProviderHostLimits, ProviderHostPhase,
    ResolvedProviderComponent,
};
use edict_provider_schema::{
    ProviderArtifactSchemaRegistry, ProviderSchemaRegistryFailureKind,
    ResolvedProviderSchemaArtifact,
};
use edict_syntax::{
    bind_target_provider_manifest, compile_to_core, decode_canonical_cbor, encode_canonical_cbor,
    encode_core_module, encode_target_ir_artifact, lower_with_builtin_lowerer, parse_module,
    select_provider_component, validate_provider_lowering_request,
    validate_provider_verification_request, BuiltinLowererRequest, BuiltinTargetLowerer,
    CanonicalValue, CompilerContext, CoreBudget, CoreModule, ProviderArtifact,
    ProviderArtifactBinding, ProviderArtifactSchemaValidator, ProviderArtifactSource,
    ProviderBoundArtifact, ProviderDigest, ProviderDigestAlgorithm, ProviderInvocationKind,
    ProviderInvocationValidationFailureKind, ProviderLoweringInvocationContract,
    ProviderLoweringOutputKind, ProviderLoweringOutputRequest, ProviderLoweringRequest,
    ProviderResourceRef, ProviderResponseLimits, ProviderSchemaFormat, ProviderSemanticInput,
    ProviderSemanticInputBinding, ProviderSemanticInputKind,
    ProviderVerificationInvocationContract, ProviderVerificationOutputKind,
    ProviderVerificationOutputRequest, ProviderVerificationRequest, ResourceRef,
    TargetEffectLowering, TargetIrLoweringFacts, TargetProviderManifest, WriteClass,
    AUTHORITY_FACTS_API_VERSION, CORE_DIGEST_FRAME, CORE_MODULE_DIGEST_DOMAIN,
    ECHO_DPO_TARGET_PROFILE, ECHO_SPAN_IR_DOMAIN, PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
    TARGET_IR_ARTIFACT_DIGEST_DOMAIN, TARGET_PROFILE_API_VERSION, TARGET_PROVIDER_PROTOCOL_VERSION,
};
use sha2::{Digest as _, Sha256};

const ECHO_SOURCE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let receipt: Receipt = target.replace(input.id)\n\
        else { rejected(reason) => domain.WriteRejected };\n\
      return { id: input.id };\n\
    }";

const SCHEMA_ROLE: &str = "schema.echo-provider-artifacts";
const GENERATED_ARTIFACT_PROFILE_DOMAIN: &str = "echo.generated-artifact-profile/v1";
const GENERATED_ARTIFACT_PROFILE_ROLE: &str = "generated-artifact-profile.echo-dpo-registration";
const LOWERABILITY_DOMAIN: &str = "edict.lowering-requirements/v1";
const LOWERER_ROLE: &str = "lowerer.echo-dpo";
const TARGET_IR_ROLE: &str = "target-ir.echo-dpo";
const VERIFIER_ROLE: &str = "verifier.echo-dpo";
const VERIFIER_REPORT_DOMAIN: &str = "echo.verifier-report/v1";
const VERIFIER_REPORT_ROLE: &str = "verifier-report.echo-dpo";
const EXPECTED_SCHEMA_BINDING_COUNT: usize = 24;

const MANIFEST_BYTES: &[u8] =
    include_bytes!("../../../schemas/edict-provider/package/v1/provider-manifest.echo.json");
const SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/generated/primary/schema.echo-provider-artifacts.cddl"
);
const GENERATED_ARTIFACT_PROFILE_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/generated/primary/generated-artifact-profile.echo-dpo-registration.cbor"
);
const LAWPACK_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/generated/primary/lawpack.echo-dpo.cbor"
);
const TARGET_PROFILE_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/generated/primary/target-profile.echo-dpo.cbor"
);
const TARGET_AUTHORITY_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/generated/primary/authority-facts.echo-dpo.cbor"
);
const LAWPACK_AUTHORITY_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/generated/primary/authority-facts.echo-lawpack.cbor"
);
const LOWERER_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm"
);
const VERIFIER_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/package/v1/components/verifier.echo-dpo.component.wasm"
);

struct RoutedCanonicalArtifact {
    role: &'static str,
    domain: &'static str,
    root: &'static str,
    bytes: &'static [u8],
}

const ROUTED_CANONICAL_ARTIFACTS: &[RoutedCanonicalArtifact] = &[
    RoutedCanonicalArtifact {
        role: "authority-facts.echo-dpo",
        domain: AUTHORITY_FACTS_API_VERSION,
        root: "authority-facts",
        bytes: TARGET_AUTHORITY_BYTES,
    },
    RoutedCanonicalArtifact {
        role: "authority-facts.echo-lawpack",
        domain: AUTHORITY_FACTS_API_VERSION,
        root: "authority-facts",
        bytes: LAWPACK_AUTHORITY_BYTES,
    },
    RoutedCanonicalArtifact {
        role: GENERATED_ARTIFACT_PROFILE_ROLE,
        domain: GENERATED_ARTIFACT_PROFILE_DOMAIN,
        root: "generated-artifact-profile",
        bytes: GENERATED_ARTIFACT_PROFILE_BYTES,
    },
    RoutedCanonicalArtifact {
        role: "lawpack.echo-dpo",
        domain: PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
        root: "lawpack-manifest",
        bytes: LAWPACK_BYTES,
    },
    RoutedCanonicalArtifact {
        role: "target-profile.echo-dpo",
        domain: TARGET_PROFILE_API_VERSION,
        root: "target-profile-manifest",
        bytes: TARGET_PROFILE_BYTES,
    },
];

struct GeneratedResourceFixture {
    path: &'static str,
    domain: &'static str,
    root: &'static str,
    bytes: &'static [u8],
}

const GENERATED_RESOURCE_FIXTURES: &[GeneratedResourceFixture] = &[
    GeneratedResourceFixture {
        path: "resource.conformance-corpus.cbor",
        domain: "echo.dpo.fixtures/v1",
        root: "echo-provider-conformance-corpus",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.conformance-corpus.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.lawpack-compatibility.cbor",
        domain: "echo.dpo-lawpack.compatibility@1",
        root: "echo-provider-lawpack-compatibility",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.lawpack-compatibility.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.lawpack-exports.cbor",
        domain: "echo.dpo-lawpack.exports@1",
        root: "lawpack-exports",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.lawpack-exports.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.lawpack-target-adapter.cbor",
        domain: "echo.dpo-lawpack.adapter.echo-dpo@1",
        root: "echo-provider-lawpack-target-adapter",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.lawpack-target-adapter.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.lawpack-verifier.cbor",
        domain: "echo.dpo-lawpack.verifier@1",
        root: "echo-provider-lawpack-verifier",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.lawpack-verifier.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-bundle-profile.cbor",
        domain: "echo.dpo.bundle/v1",
        root: "echo-dpo-bundle",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-bundle-profile.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-cost-algebra.cbor",
        domain: "echo.dpo.cost/v1",
        root: "echo-dpo-cost",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-cost-algebra.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-footprint-algebra.cbor",
        domain: "echo.dpo.footprint/v1",
        root: "echo-dpo-footprint",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-footprint-algebra.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-intrinsics.cbor",
        domain: "echo.dpo.intrinsics/v1",
        root: "intrinsics-document",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-intrinsics.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-ir.cbor",
        domain: "echo.span-ir/v1",
        root: "echo-span-ir",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-ir.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-lowerer-contract.cbor",
        domain: "echo.dpo.lowerer/v1",
        root: "echo-dpo-lowerer",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-lowerer-contract.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-obstruction-taxonomy.cbor",
        domain: "echo.dpo.obstructions/v1",
        root: "echo-dpo-obstructions",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-obstruction-taxonomy.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-operation-profiles.cbor",
        domain: "echo.dpo.operation-profiles/v1",
        root: "operation-profiles-document",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-operation-profiles.cbor"
        ),
    },
    GeneratedResourceFixture {
        path: "resource.target-verifier-contract.cbor",
        domain: "echo.dpo.verifier/v1",
        root: "echo-dpo-verifier",
        bytes: include_bytes!(
            "../../../schemas/edict-provider/package/v1/generated/resources/resource.target-verifier-contract.cbor"
        ),
    },
];

#[derive(Clone, Copy)]
struct GeneratedResourceMaterial<'a> {
    coordinate: &'a str,
    domain: &'a str,
    bytes: &'a [u8],
}

struct OwnerResourceReference<'a> {
    expected_coordinate: &'static str,
    reference: &'a CanonicalValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackageIdentityFailure {
    ReferenceMalformed,
    ConflictingOwnerReference,
    FieldReferenceMismatch,
    DuplicateMaterial,
    CoordinateClosureMismatch,
    DigestMismatch,
    PrimaryReferenceMismatch,
    AuthoritySourceMismatch,
}

fn checked_manifest() -> TargetProviderManifest {
    serde_json::from_slice(MANIFEST_BYTES)
        .expect("the checked package manifest is exact typed Edict JSON")
}

fn required_domains(manifest: &TargetProviderManifest) -> Vec<&str> {
    manifest
        .schema_bindings
        .iter()
        .map(|binding| binding.domain.as_str())
        .collect()
}

fn declared_package_registry(manifest: &TargetProviderManifest) -> ProviderArtifactSchemaRegistry {
    let proof = bind_target_provider_manifest(manifest)
        .expect("the checked package manifest satisfies the Edict envelope");
    ProviderArtifactSchemaRegistry::from_manifest(
        &proof,
        [ResolvedProviderSchemaArtifact {
            role: SCHEMA_ROLE.to_owned(),
            bytes: Arc::<[u8]>::from(SCHEMA_BYTES),
        }],
        required_domains(manifest),
    )
    .expect("every currently declared package schema binding compiles")
}

fn package_registry(manifest: &TargetProviderManifest) -> ProviderArtifactSchemaRegistry {
    let proof = bind_target_provider_manifest(manifest)
        .expect("the checked package manifest satisfies the Edict envelope");
    let mut required = required_domains(manifest);
    required.push(GENERATED_ARTIFACT_PROFILE_DOMAIN);
    required.sort_unstable();
    required.dedup();
    ProviderArtifactSchemaRegistry::from_manifest(
        &proof,
        [ResolvedProviderSchemaArtifact {
            role: SCHEMA_ROLE.to_owned(),
            bytes: Arc::<[u8]>::from(SCHEMA_BYTES),
        }],
        required,
    )
    .expect("every routed canonical primary has an owning package schema binding")
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing hexadecimal to String cannot fail");
    }
    output
}

fn raw_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", hex(&Sha256::digest(bytes)))
}

fn provider_digest(domain: &str, canonical_bytes: &[u8]) -> ProviderDigest {
    decode_canonical_cbor(canonical_bytes).expect("artifact is canonical CBOR");
    let mut framed = vec![0x83];
    framed
        .extend(encode_canonical_cbor(&text(CORE_DIGEST_FRAME)).expect("digest frame tag encodes"));
    framed.extend(encode_canonical_cbor(&text(domain)).expect("digest domain encodes"));
    framed.extend_from_slice(canonical_bytes);
    ProviderDigest {
        algorithm: ProviderDigestAlgorithm::Sha256,
        bytes: Sha256::digest(framed).to_vec(),
    }
}

fn rendered_provider_digest(digest: &ProviderDigest) -> String {
    assert_eq!(digest.algorithm, ProviderDigestAlgorithm::Sha256);
    format!("sha256:{}", hex(&digest.bytes))
}

fn bound_artifact(coordinate: &str, domain: &str, bytes: &[u8]) -> ProviderBoundArtifact {
    ProviderBoundArtifact {
        reference: ProviderResourceRef {
            coordinate: coordinate.to_owned(),
            digest: provider_digest(domain, bytes),
        },
        artifact: ProviderArtifact {
            domain: domain.to_owned(),
            bytes: bytes.to_vec(),
        },
    }
}

fn artifact_binding(bound: &ProviderBoundArtifact) -> ProviderArtifactBinding {
    ProviderArtifactBinding {
        reference: bound.reference.clone(),
        domain: bound.artifact.domain.clone(),
    }
}

fn text(value: &str) -> CanonicalValue {
    CanonicalValue::Text(value.to_owned())
}

fn map(entries: impl IntoIterator<Item = (&'static str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn lowerability_bytes() -> Vec<u8> {
    encode_canonical_cbor(&map([
        ("apiVersion", text(LOWERABILITY_DOMAIN)),
        ("operationProfile", text("continuum.profile.write/v1")),
        (
            "semanticEffects",
            CanonicalValue::Array(vec![map([
                ("coordinate", text("target.replace")),
                ("writeClass", text("replace")),
                (
                    "guardKinds",
                    CanonicalValue::Array(vec![text("precommit-atomic")]),
                ),
                (
                    "obstructionCoordinates",
                    CanonicalValue::Array(vec![text("rejected")]),
                ),
                (
                    "footprintObligations",
                    CanonicalValue::Array(vec![text("target.replace.footprint")]),
                ),
                (
                    "costObligations",
                    CanonicalValue::Array(vec![text("target.replace.cost")]),
                ),
            ])]),
        ),
        (
            "requiredWriteClasses",
            CanonicalValue::Array(vec![text("replace")]),
        ),
        (
            "guardKinds",
            CanonicalValue::Array(vec![text("precommit-atomic")]),
        ),
        ("atomicity", text("atomic")),
        ("postconditionSupport", CanonicalValue::Bool(true)),
        (
            "obstructionCoordinates",
            CanonicalValue::Array(vec![text("rejected")]),
        ),
        (
            "footprintObligations",
            CanonicalValue::Array(vec![text("target.replace.footprint")]),
        ),
        (
            "costObligations",
            CanonicalValue::Array(vec![text("target.replace.cost")]),
        ),
        ("opticContract", text("replace-point")),
    ]))
    .expect("lowerability facts encode canonically")
}

fn echo_core() -> CoreModule {
    let context = CompilerContext::new()
        .with_operation_profile("p.effectful", "continuum.profile.write/v1")
        .with_operation_profile_write_classes("p.effectful", [WriteClass::Replace])
        .with_effect_write_class("target.replace", WriteClass::Replace)
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 8,
                max_allocated_bytes: 1024,
                max_output_bytes: 256,
            },
        );
    let module = parse_module(ECHO_SOURCE).expect("Echo source parses");
    compile_to_core(&module, &context).expect("Echo source compiles to Core")
}

fn semantic_input(
    role: &str,
    kind: ProviderSemanticInputKind,
    coordinate: &str,
    domain: &str,
    bytes: &[u8],
) -> ProviderSemanticInput {
    ProviderSemanticInput {
        role: role.to_owned(),
        kind,
        artifact: bound_artifact(coordinate, domain, bytes),
    }
}

fn semantic_inputs() -> Vec<ProviderSemanticInput> {
    let lowerability = lowerability_bytes();
    vec![
        semantic_input(
            "authority-facts.echo-dpo",
            ProviderSemanticInputKind::AuthorityFacts,
            "echo.dpo-authority-facts@1",
            AUTHORITY_FACTS_API_VERSION,
            TARGET_AUTHORITY_BYTES,
        ),
        semantic_input(
            "authority-facts.echo-lawpack",
            ProviderSemanticInputKind::AuthorityFacts,
            "echo.dpo-lawpack-authority-facts@1",
            AUTHORITY_FACTS_API_VERSION,
            LAWPACK_AUTHORITY_BYTES,
        ),
        semantic_input(
            "lawpack.echo-dpo",
            ProviderSemanticInputKind::Lawpack,
            "echo.dpo-lawpack@1",
            PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
            LAWPACK_BYTES,
        ),
        semantic_input(
            "lowerability.echo-dpo",
            ProviderSemanticInputKind::LowerabilityFacts,
            "echo.dpo-lowerability@1",
            LOWERABILITY_DOMAIN,
            &lowerability,
        ),
    ]
}

fn lowering_request(
    core: &CoreModule,
) -> (ProviderLoweringInvocationContract, ProviderLoweringRequest) {
    let core_bytes = encode_core_module(core).expect("Core module encodes canonically");
    let core_artifact = bound_artifact("a.b@1", CORE_MODULE_DIGEST_DOMAIN, &core_bytes);
    let target_profile = bound_artifact(
        ECHO_DPO_TARGET_PROFILE,
        TARGET_PROFILE_API_VERSION,
        TARGET_PROFILE_BYTES,
    );
    let semantic_inputs = semantic_inputs();
    let contract = ProviderLoweringInvocationContract {
        core: artifact_binding(&core_artifact),
        target_profile: artifact_binding(&target_profile),
        semantic_inputs: semantic_inputs
            .iter()
            .map(|input| ProviderSemanticInputBinding {
                role: input.role.clone(),
                kind: input.kind.clone(),
                artifact: artifact_binding(&input.artifact),
            })
            .collect(),
    };
    let request = ProviderLoweringRequest {
        protocol_version: TARGET_PROVIDER_PROTOCOL_VERSION,
        core: core_artifact,
        target_profile,
        semantic_inputs,
        requested_outputs: vec![ProviderLoweringOutputRequest {
            role: TARGET_IR_ROLE.to_owned(),
            kind: ProviderLoweringOutputKind::TargetIr,
            domain: TARGET_IR_ARTIFACT_DIGEST_DOMAIN.to_owned(),
        }],
        limits: response_limits(),
    };
    (contract, request)
}

fn oracle_target_ir(core: &CoreModule, target_profile: ResourceRef) -> Vec<u8> {
    let facts = TargetIrLoweringFacts {
        target_profile,
        target_ir_domain: ECHO_SPAN_IR_DOMAIN.to_owned(),
        operation_profiles: vec!["continuum.profile.write/v1".to_owned()],
        obstruction_coordinates: vec!["rejected".to_owned()],
        effect_lowerings: vec![TargetEffectLowering {
            effect: "target.replace".to_owned(),
            target_intrinsic: "echo.dpo@1.replace".to_owned(),
        }],
    };
    let report = lower_with_builtin_lowerer(
        BuiltinTargetLowerer::EchoDpo,
        BuiltinLowererRequest {
            core,
            facts: &facts,
        },
    )
    .expect("Edict's built-in Echo lowerer accepts the exact fixture");
    let artifact = report
        .artifact
        .expect("the exact fixture lowers to Target IR");
    encode_target_ir_artifact(&artifact).expect("oracle Target IR encodes canonically")
}

fn verification_request(
    core: &CoreModule,
    target_ir_bytes: &[u8],
) -> (
    ProviderVerificationInvocationContract,
    ProviderVerificationRequest,
) {
    let core_bytes = encode_core_module(core).expect("Core module encodes canonically");
    let core_artifact = bound_artifact("a.b@1", CORE_MODULE_DIGEST_DOMAIN, &core_bytes);
    let target_profile = bound_artifact(
        ECHO_DPO_TARGET_PROFILE,
        TARGET_PROFILE_API_VERSION,
        TARGET_PROFILE_BYTES,
    );
    let target_ir = bound_artifact(
        "echo.target-ir@1",
        TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
        target_ir_bytes,
    );
    let semantic_inputs = semantic_inputs();
    let contract = ProviderVerificationInvocationContract {
        core: artifact_binding(&core_artifact),
        target_profile: artifact_binding(&target_profile),
        target_ir: artifact_binding(&target_ir),
        semantic_inputs: semantic_inputs
            .iter()
            .map(|input| ProviderSemanticInputBinding {
                role: input.role.clone(),
                kind: input.kind.clone(),
                artifact: artifact_binding(&input.artifact),
            })
            .collect(),
    };
    let request = ProviderVerificationRequest {
        protocol_version: TARGET_PROVIDER_PROTOCOL_VERSION,
        core: core_artifact,
        target_profile,
        target_ir,
        semantic_inputs,
        requested_outputs: vec![ProviderVerificationOutputRequest {
            role: VERIFIER_REPORT_ROLE.to_owned(),
            kind: ProviderVerificationOutputKind::VerifierReport,
            domain: VERIFIER_REPORT_DOMAIN.to_owned(),
        }],
        limits: response_limits(),
    };
    (contract, request)
}

const fn response_limits() -> ProviderResponseLimits {
    ProviderResponseLimits {
        max_output_count: 8,
        max_diagnostic_count: 8,
        max_total_response_bytes: 64 * 1024,
    }
}

const fn host_limits() -> ProviderHostLimits {
    ProviderHostLimits {
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 3 * 1024 * 1024,
        max_diagnostic_bytes: 3 * 1024 * 1024,
        max_wasm_memory_bytes: 16 * 1024 * 1024,
        max_table_elements: 10_000,
        max_instances: 100,
        max_memories: 8,
        max_tables: 8,
        max_wasm_fuel: 50_000_000,
        max_hostcall_bytes: 4 * 1024 * 1024,
        max_host_diagnostic_bytes: 512,
    }
}

struct PackageConformanceObservation {
    target_ir_digest: String,
    verifier_outcome: &'static str,
}

fn complete_package_conformance_observation() -> PackageConformanceObservation {
    let manifest = checked_manifest();
    let proof = bind_target_provider_manifest(&manifest)
        .expect("the checked package manifest satisfies the Edict envelope");
    let registry = package_registry(&manifest);
    let host = ProviderComponentHost::new().expect("the deterministic host configures");

    let lowerer = select_provider_component(&proof, LOWERER_ROLE, ProviderInvocationKind::Lowering)
        .expect("the checked package selects its lowerer");
    let lowerer = ResolvedProviderComponent::new(lowerer, Arc::<[u8]>::from(LOWERER_BYTES));
    let prepared_lowerer = host
        .prepare(&lowerer)
        .expect("the exact packaged lowerer passes Edict preflight");

    let core = echo_core();
    let (lowering_contract, lowering_request) = lowering_request(&core);
    assert_request_semantics_are_package_routed(
        &manifest,
        &lowering_request.target_profile,
        &lowering_request.semantic_inputs,
    );
    let lowering_request =
        validate_provider_lowering_request(&registry, &lowering_contract, &lowering_request)
            .expect("the package-routed lowering request has an Edict validation proof");
    let lowering_outcome = host
        .invoke_lowerer(
            &prepared_lowerer,
            &lowering_request,
            &registry,
            host_limits(),
        )
        .expect("the packaged lowerer completes through Edict's bounded host");
    assert!(lowering_outcome.refusal().is_none());
    let lowering_response = lowering_outcome
        .response()
        .expect("the packaged lowerer emits Target IR");
    assert!(lowering_response.diagnostics.is_empty());
    let [target_ir_output] = lowering_response.outputs.as_slice() else {
        panic!("the packaged lowerer must emit exactly one Target IR output");
    };
    assert_eq!(target_ir_output.role, TARGET_IR_ROLE);
    assert_eq!(target_ir_output.kind, ProviderLoweringOutputKind::TargetIr);
    assert_eq!(
        target_ir_output.artifact.domain,
        TARGET_IR_ARTIFACT_DIGEST_DOMAIN
    );
    assert_eq!(target_ir_output.logical_path, None);

    let builtin_target_ir = oracle_target_ir(
        &core,
        routed_resource(&manifest, "target-profile.echo-dpo").clone(),
    );
    assert_eq!(target_ir_output.artifact.bytes, builtin_target_ir);
    let builtin_target_ir_digest =
        provider_digest(TARGET_IR_ARTIFACT_DIGEST_DOMAIN, &builtin_target_ir);
    let lowering_manifest = lowering_outcome
        .manifest()
        .expect("the Edict host authors the external lowerer output manifest");
    let [target_ir_entry] = lowering_manifest.outputs() else {
        panic!("the lowerer manifest must bind exactly one Target IR output");
    };
    assert_eq!(target_ir_entry.role, TARGET_IR_ROLE);
    assert_eq!(target_ir_entry.kind, ProviderLoweringOutputKind::TargetIr);
    assert_eq!(target_ir_entry.domain, TARGET_IR_ARTIFACT_DIGEST_DOMAIN);
    assert_eq!(target_ir_entry.digest, builtin_target_ir_digest);

    let verifier =
        select_provider_component(&proof, VERIFIER_ROLE, ProviderInvocationKind::Verification)
            .expect("the checked package selects its verifier");
    let verifier = ResolvedProviderComponent::new(verifier, Arc::<[u8]>::from(VERIFIER_BYTES));
    let prepared_verifier = host
        .prepare(&verifier)
        .expect("the exact packaged verifier passes Edict preflight");
    let (verification_contract, verification_request) =
        verification_request(&core, &target_ir_output.artifact.bytes);
    assert_request_semantics_are_package_routed(
        &manifest,
        &verification_request.target_profile,
        &verification_request.semantic_inputs,
    );
    let verification_request = validate_provider_verification_request(
        &registry,
        &verification_contract,
        &verification_request,
    )
    .expect("the package-routed verification request has an Edict validation proof");
    let verification_outcome = host
        .invoke_verifier(
            &prepared_verifier,
            &verification_request,
            &registry,
            host_limits(),
        )
        .expect("the packaged verifier completes through Edict's bounded host");
    assert!(verification_outcome.refusal().is_none());
    let verification_response = verification_outcome
        .response()
        .expect("the packaged verifier emits a verifier report");
    assert!(verification_response.diagnostics.is_empty());
    let [verifier_report] = verification_response.outputs.as_slice() else {
        panic!("the packaged verifier must emit exactly one verifier report");
    };
    assert_eq!(verifier_report.role, VERIFIER_REPORT_ROLE);
    assert_eq!(
        verifier_report.kind,
        ProviderVerificationOutputKind::VerifierReport
    );
    assert_eq!(verifier_report.artifact.domain, VERIFIER_REPORT_DOMAIN);
    assert_eq!(verifier_report.logical_path, None);
    let verifier_report_value = decode_canonical_cbor(&verifier_report.artifact.bytes)
        .expect("the admitted verifier report is canonical CBOR");
    assert_eq!(
        map_field(&verifier_report_value, "outcome"),
        Some(&text("accepted"))
    );
    let target_ir_reference = map_field(&verifier_report_value, "targetIr")
        .expect("the verifier report binds its exact Target IR subject");
    let (target_ir_coordinate, target_ir_digest) = embedded_resource_ref(target_ir_reference)
        .expect("the verifier report carries a strict Target IR reference");
    assert_eq!(target_ir_coordinate, "echo.target-ir@1");
    assert_eq!(target_ir_digest, builtin_target_ir_digest.bytes.as_slice());
    let verification_manifest = verification_outcome
        .manifest()
        .expect("the Edict host authors the verifier output manifest");
    let [verifier_entry] = verification_manifest.outputs() else {
        panic!("the verifier manifest must bind exactly one report output");
    };
    assert_eq!(verifier_entry.role, VERIFIER_REPORT_ROLE);
    assert_eq!(
        verifier_entry.kind,
        ProviderVerificationOutputKind::VerifierReport
    );
    assert_eq!(verifier_entry.domain, VERIFIER_REPORT_DOMAIN);
    assert_eq!(
        verifier_entry.digest,
        provider_digest(VERIFIER_REPORT_DOMAIN, &verifier_report.artifact.bytes)
    );

    PackageConformanceObservation {
        target_ir_digest: rendered_provider_digest(&builtin_target_ir_digest),
        verifier_outcome: "accepted",
    }
}

fn routed_resource<'a>(manifest: &'a TargetProviderManifest, role: &str) -> &'a ResourceRef {
    &manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.role == role)
        .expect("the package contains every required routed role")
        .resource
}

fn assert_bound_matches_routed(
    manifest: &TargetProviderManifest,
    role: &str,
    bound: &ProviderBoundArtifact,
) {
    let routed = routed_resource(manifest, role);
    assert_eq!(bound.reference.coordinate, routed.coordinate);
    assert_eq!(
        Some(rendered_provider_digest(&bound.reference.digest).as_str()),
        routed.digest.as_deref()
    );
}

fn assert_request_semantics_are_package_routed(
    manifest: &TargetProviderManifest,
    target_profile: &ProviderBoundArtifact,
    inputs: &[ProviderSemanticInput],
) {
    assert_bound_matches_routed(manifest, "target-profile.echo-dpo", target_profile);
    for role in [
        "authority-facts.echo-dpo",
        "authority-facts.echo-lawpack",
        "lawpack.echo-dpo",
    ] {
        let input = inputs
            .iter()
            .find(|input| input.role == role)
            .expect("the exact request carries every routed semantic input");
        assert_bound_matches_routed(manifest, role, &input.artifact);
    }
}

fn assert_schema_binding(manifest: &TargetProviderManifest, domain: &str, root: &str) {
    let binding = manifest
        .schema_bindings
        .iter()
        .find(|binding| binding.domain == domain)
        .expect("every canonical package member has an exact owning schema binding");
    assert_eq!(binding.schema_role, SCHEMA_ROLE);
    assert_eq!(binding.format, ProviderSchemaFormat::SelfContainedCddlV1);
    assert_eq!(binding.root_rule, root);
}

fn map_field<'a>(value: &'a CanonicalValue, field: &str) -> Option<&'a CanonicalValue> {
    let CanonicalValue::Map(entries) = value else {
        return None;
    };
    entries.iter().find_map(|(key, value)| match key {
        CanonicalValue::Text(key) if key == field => Some(value),
        _ => None,
    })
}

fn map_field_mut<'a>(value: &'a mut CanonicalValue, field: &str) -> Option<&'a mut CanonicalValue> {
    let CanonicalValue::Map(entries) = value else {
        return None;
    };
    entries.iter_mut().find_map(|(key, value)| match key {
        CanonicalValue::Text(key) if key == field => Some(value),
        _ => None,
    })
}

fn required_field<'a>(
    value: &'a CanonicalValue,
    field: &str,
) -> Result<&'a CanonicalValue, PackageIdentityFailure> {
    map_field(value, field).ok_or(PackageIdentityFailure::ReferenceMalformed)
}

fn required_singleton(value: &CanonicalValue) -> Result<&CanonicalValue, PackageIdentityFailure> {
    let CanonicalValue::Array(values) = value else {
        return Err(PackageIdentityFailure::ReferenceMalformed);
    };
    let [value] = values.as_slice() else {
        return Err(PackageIdentityFailure::CoordinateClosureMismatch);
    };
    Ok(value)
}

fn typed_digest_bytes(value: &CanonicalValue) -> Result<&[u8], PackageIdentityFailure> {
    let CanonicalValue::Array(parts) = value else {
        return Err(PackageIdentityFailure::ReferenceMalformed);
    };
    let [CanonicalValue::Text(algorithm), CanonicalValue::Bytes(bytes)] = parts.as_slice() else {
        return Err(PackageIdentityFailure::ReferenceMalformed);
    };
    if algorithm != "sha256" || bytes.len() != 32 {
        return Err(PackageIdentityFailure::ReferenceMalformed);
    }
    Ok(bytes)
}

fn embedded_resource_ref(value: &CanonicalValue) -> Result<(&str, &[u8]), PackageIdentityFailure> {
    let CanonicalValue::Text(coordinate) = required_field(value, "id")? else {
        return Err(PackageIdentityFailure::ReferenceMalformed);
    };
    let digest = typed_digest_bytes(required_field(value, "digest")?)?;
    Ok((coordinate, digest))
}

fn insert_owner_reference(
    closure: &mut BTreeMap<String, Vec<u8>>,
    reference: &CanonicalValue,
) -> Result<(), PackageIdentityFailure> {
    let (coordinate, digest) = embedded_resource_ref(reference)?;
    if let Some(existing) = closure.get(coordinate) {
        if existing != digest {
            return Err(PackageIdentityFailure::ConflictingOwnerReference);
        }
    } else {
        closure.insert(coordinate.to_owned(), digest.to_vec());
    }
    Ok(())
}

fn owner_generated_resource_closure(
    lawpack: &CanonicalValue,
    target_profile: &CanonicalValue,
) -> Result<BTreeMap<String, Vec<u8>>, PackageIdentityFailure> {
    let target_adapters = required_field(lawpack, "targetAdapters")?;
    let target_adapter = required_singleton(target_adapters)?;
    let lawpack_verifier = required_field(lawpack, "verifier")?;
    let mut closure = BTreeMap::new();
    for owner in [
        OwnerResourceReference {
            expected_coordinate: "echo.dpo-lawpack.exports@1",
            reference: required_field(lawpack, "exports")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.span-ir/v1",
            reference: required_field(target_adapter, "acceptedTargetIr")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo-lawpack.adapter.echo-dpo@1",
            reference: required_field(target_adapter, "adapter")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo-lawpack.verifier@1",
            reference: required_field(lawpack_verifier, "ruleset")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo-lawpack.compatibility@1",
            reference: required_field(lawpack, "compatibility")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.fixtures/v1",
            reference: required_field(lawpack, "conformanceFixtureCorpus")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.intrinsics/v1",
            reference: required_field(target_profile, "intrinsics")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.operation-profiles/v1",
            reference: required_field(target_profile, "operationProfiles")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.footprint/v1",
            reference: required_field(target_profile, "footprintAlgebra")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.cost/v1",
            reference: required_field(target_profile, "costAlgebra")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.span-ir/v1",
            reference: required_field(target_profile, "targetIr")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.obstructions/v1",
            reference: required_field(target_profile, "obstructionTaxonomy")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.verifier/v1",
            reference: required_field(target_profile, "verifier")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.lowerer/v1",
            reference: required_field(target_profile, "lowerer")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.bundle/v1",
            reference: required_field(target_profile, "bundleProfile")?,
        },
        OwnerResourceReference {
            expected_coordinate: "echo.dpo.fixtures/v1",
            reference: required_field(target_profile, "conformanceFixtureCorpus")?,
        },
    ] {
        let (coordinate, _) = embedded_resource_ref(owner.reference)?;
        if coordinate != owner.expected_coordinate {
            return Err(PackageIdentityFailure::FieldReferenceMismatch);
        }
        insert_owner_reference(&mut closure, owner.reference)?;
    }
    Ok(closure)
}

fn validate_generated_resource_identity_closure<'a>(
    lawpack: &CanonicalValue,
    target_profile: &CanonicalValue,
    materials: impl IntoIterator<Item = GeneratedResourceMaterial<'a>>,
) -> Result<(), PackageIdentityFailure> {
    let owner_closure = owner_generated_resource_closure(lawpack, target_profile)?;
    let mut material_closure = BTreeMap::new();
    for material in materials {
        let digest = provider_digest(material.domain, material.bytes).bytes;
        if material_closure
            .insert(material.coordinate.to_owned(), digest)
            .is_some()
        {
            return Err(PackageIdentityFailure::DuplicateMaterial);
        }
    }
    if owner_closure.len() != material_closure.len()
        || owner_closure
            .keys()
            .zip(material_closure.keys())
            .any(|(owner, material)| owner != material)
    {
        return Err(PackageIdentityFailure::CoordinateClosureMismatch);
    }
    if material_closure
        .iter()
        .any(|(coordinate, digest)| owner_closure.get(coordinate) != Some(digest))
    {
        return Err(PackageIdentityFailure::DigestMismatch);
    }
    Ok(())
}

fn embedded_ref_matches_routed(
    manifest: &TargetProviderManifest,
    role: &str,
    reference: &CanonicalValue,
) -> Result<(), PackageIdentityFailure> {
    let (coordinate, digest) = embedded_resource_ref(reference)?;
    let routed = routed_resource(manifest, role);
    if coordinate != routed.coordinate
        || Some(format!("sha256:{}", hex(digest)).as_str()) != routed.digest.as_deref()
    {
        return Err(PackageIdentityFailure::PrimaryReferenceMismatch);
    }
    Ok(())
}

fn authority_source_matches_routed(
    manifest: &TargetProviderManifest,
    facts: &CanonicalValue,
    expected_kind: &str,
    expected_role: &str,
) -> Result<(), PackageIdentityFailure> {
    let source = required_field(facts, "source")?;
    let CanonicalValue::Text(kind) = required_field(source, "kind")? else {
        return Err(PackageIdentityFailure::ReferenceMalformed);
    };
    let CanonicalValue::Text(coordinate) = required_field(source, "coordinate")? else {
        return Err(PackageIdentityFailure::ReferenceMalformed);
    };
    let digest = typed_digest_bytes(required_field(source, "digest")?)?;
    let routed = routed_resource(manifest, expected_role);
    if kind != expected_kind
        || coordinate != &routed.coordinate
        || Some(format!("sha256:{}", hex(digest)).as_str()) != routed.digest.as_deref()
    {
        return Err(PackageIdentityFailure::AuthoritySourceMismatch);
    }
    Ok(())
}

fn validate_adjacent_primary_identity_edges(
    manifest: &TargetProviderManifest,
    lawpack: &CanonicalValue,
    target_profile: &CanonicalValue,
    target_authority: &CanonicalValue,
    lawpack_authority: &CanonicalValue,
) -> Result<(), PackageIdentityFailure> {
    let generated_profiles = required_field(target_profile, "generatedArtifactProfiles")?;
    embedded_ref_matches_routed(
        manifest,
        GENERATED_ARTIFACT_PROFILE_ROLE,
        required_singleton(generated_profiles)?,
    )?;
    let target_adapters = required_field(lawpack, "targetAdapters")?;
    let target_adapter = required_singleton(target_adapters)?;
    embedded_ref_matches_routed(
        manifest,
        "target-profile.echo-dpo",
        required_field(target_adapter, "acceptedTargetProfile")?,
    )?;
    authority_source_matches_routed(
        manifest,
        target_authority,
        "targetProfile",
        "target-profile.echo-dpo",
    )?;
    authority_source_matches_routed(manifest, lawpack_authority, "lawpack", "lawpack.echo-dpo")
}

fn exact_generated_resource_materials() -> impl Iterator<Item = GeneratedResourceMaterial<'static>>
{
    GENERATED_RESOURCE_FIXTURES
        .iter()
        .map(|fixture| GeneratedResourceMaterial {
            coordinate: fixture.domain,
            domain: fixture.domain,
            bytes: fixture.bytes,
        })
}

fn assert_routed_canonical_artifacts(
    manifest: &TargetProviderManifest,
    registry: &ProviderArtifactSchemaRegistry,
) {
    for fixture in ROUTED_CANONICAL_ARTIFACTS {
        assert_schema_binding(manifest, fixture.domain, fixture.root);
        let value = decode_canonical_cbor(fixture.bytes)
            .expect("the routed package artifact is exact canonical CBOR");
        registry
            .validate_canonical_value(fixture.domain, &value)
            .expect("the routed package artifact satisfies its owning root");
        let expected = rendered_provider_digest(&provider_digest(fixture.domain, fixture.bytes));
        assert_eq!(
            routed_resource(manifest, fixture.role).digest.as_deref(),
            Some(expected.as_str())
        );
    }
}

fn assert_generated_resources(
    manifest: &TargetProviderManifest,
    registry: &ProviderArtifactSchemaRegistry,
) {
    for fixture in GENERATED_RESOURCE_FIXTURES {
        assert_schema_binding(manifest, fixture.domain, fixture.root);
        let value = decode_canonical_cbor(fixture.bytes)
            .unwrap_or_else(|error| panic!("{} is not canonical: {error}", fixture.path));
        registry
            .validate_canonical_value(fixture.domain, &value)
            .unwrap_or_else(|error| {
                panic!(
                    "{} does not satisfy {}: {error:?}",
                    fixture.path, fixture.root
                );
            });
    }
}

#[test]
fn checked_package_is_ready_for_edict_before_guest_execution() {
    let manifest = checked_manifest();
    let proof = bind_target_provider_manifest(&manifest)
        .expect("the checked package manifest satisfies the Edict envelope");
    let registry = package_registry(&manifest);
    assert_eq!(
        manifest.schema_bindings.len(),
        EXPECTED_SCHEMA_BINDING_COUNT
    );
    assert_eq!(registry.bindings().len(), EXPECTED_SCHEMA_BINDING_COUNT);
    assert_routed_canonical_artifacts(&manifest, &registry);
    assert_generated_resources(&manifest, &registry);
    let lawpack = decode_canonical_cbor(LAWPACK_BYTES).expect("lawpack is canonical");
    let target_profile =
        decode_canonical_cbor(TARGET_PROFILE_BYTES).expect("target profile is canonical");
    let target_authority =
        decode_canonical_cbor(TARGET_AUTHORITY_BYTES).expect("target facts are canonical");
    let lawpack_authority =
        decode_canonical_cbor(LAWPACK_AUTHORITY_BYTES).expect("lawpack facts are canonical");
    validate_generated_resource_identity_closure(
        &lawpack,
        &target_profile,
        exact_generated_resource_materials(),
    )
    .expect("the exact 14-resource owner union binds every domain-framed digest");
    validate_adjacent_primary_identity_edges(
        &manifest,
        &lawpack,
        &target_profile,
        &target_authority,
        &lawpack_authority,
    )
    .expect("the exact package primary identity graph is closed");

    let host = ProviderComponentHost::new().expect("the deterministic host configures");
    let lowerer = select_provider_component(&proof, LOWERER_ROLE, ProviderInvocationKind::Lowering)
        .expect("the checked package selects its lowerer");
    let lowerer = ResolvedProviderComponent::new(lowerer, Arc::<[u8]>::from(LOWERER_BYTES));
    let _prepared_lowerer = host
        .prepare(&lowerer)
        .expect("the exact packaged lowerer passes Edict preflight");
    let verifier =
        select_provider_component(&proof, VERIFIER_ROLE, ProviderInvocationKind::Verification)
            .expect("the checked package selects its verifier");
    let verifier = ResolvedProviderComponent::new(verifier, Arc::<[u8]>::from(VERIFIER_BYTES));
    let _prepared_verifier = host
        .prepare(&verifier)
        .expect("the exact packaged verifier passes Edict preflight");

    let core = echo_core();
    let (lowering_contract, lowering_request) = lowering_request(&core);
    assert_request_semantics_are_package_routed(
        &manifest,
        &lowering_request.target_profile,
        &lowering_request.semantic_inputs,
    );
    let _lowering_proof =
        validate_provider_lowering_request(&registry, &lowering_contract, &lowering_request)
            .expect("the exact lowerer request has an Edict validation proof");

    let target_ir = oracle_target_ir(
        &core,
        routed_resource(&manifest, "target-profile.echo-dpo").clone(),
    );
    let (verification_contract, verification_request) = verification_request(&core, &target_ir);
    assert_request_semantics_are_package_routed(
        &manifest,
        &verification_request.target_profile,
        &verification_request.semantic_inputs,
    );
    let _verification_proof = validate_provider_verification_request(
        &registry,
        &verification_contract,
        &verification_request,
    )
    .expect("the exact verifier request has an Edict validation proof");
}

#[test]
fn schema_valid_resource_replacement_cannot_cross_the_exact_owner_digest() {
    let manifest = checked_manifest();
    let registry = package_registry(&manifest);
    let mut replacement = decode_canonical_cbor(
        GENERATED_RESOURCE_FIXTURES
            .iter()
            .find(|fixture| fixture.domain == "echo.dpo.cost/v1")
            .expect("the cost resource fixture exists")
            .bytes,
    )
    .expect("the exact cost resource is canonical");
    let capabilities = map_field_mut(&mut replacement, "capabilities")
        .expect("the cost resource carries capabilities");
    let CanonicalValue::Map(capabilities) = capabilities else {
        panic!("cost capabilities must be a map");
    };
    let capability = &mut capabilities
        .first_mut()
        .expect("the exact cost resource carries one capability")
        .1;
    *map_field_mut(capability, "costTemplate").expect("the cost capability has a template") =
        text("schema-valid-but-different");
    registry
        .validate_canonical_value("echo.dpo.cost/v1", &replacement)
        .expect("the replacement deliberately preserves the owning CDDL shape");
    let replacement_bytes =
        encode_canonical_cbor(&replacement).expect("the replacement encodes canonically");
    let exact_cost_bytes = GENERATED_RESOURCE_FIXTURES
        .iter()
        .find(|fixture| fixture.domain == "echo.dpo.cost/v1")
        .expect("the cost resource fixture exists")
        .bytes;
    assert_ne!(replacement_bytes, exact_cost_bytes);

    let lawpack = decode_canonical_cbor(LAWPACK_BYTES).expect("lawpack is canonical");
    let target_profile =
        decode_canonical_cbor(TARGET_PROFILE_BYTES).expect("target profile is canonical");
    let materials = GENERATED_RESOURCE_FIXTURES
        .iter()
        .map(|fixture| GeneratedResourceMaterial {
            coordinate: fixture.domain,
            domain: fixture.domain,
            bytes: if fixture.domain == "echo.dpo.cost/v1" {
                &replacement_bytes
            } else {
                fixture.bytes
            },
        });
    let error = validate_generated_resource_identity_closure(&lawpack, &target_profile, materials)
        .expect_err("schema validity cannot replace the exact owner-bound resource identity");
    assert_eq!(error, PackageIdentityFailure::DigestMismatch);
}

#[test]
fn schema_valid_lawpack_reference_mismatch_fails_identity_closure() {
    let manifest = checked_manifest();
    let registry = declared_package_registry(&manifest);
    let mut lawpack = decode_canonical_cbor(LAWPACK_BYTES).expect("lawpack is canonical");
    let exports = map_field_mut(&mut lawpack, "exports").expect("lawpack carries exports");
    let digest = map_field_mut(exports, "digest").expect("exports reference carries a digest");
    let CanonicalValue::Array(parts) = digest else {
        panic!("exports digest must be typed");
    };
    let Some(CanonicalValue::Bytes(bytes)) = parts.get_mut(1) else {
        panic!("exports digest must carry raw bytes");
    };
    bytes[0] ^= 1;
    let changed_bytes =
        encode_canonical_cbor(&lawpack).expect("changed lawpack encodes canonically");
    let changed_lawpack =
        decode_canonical_cbor(&changed_bytes).expect("changed lawpack remains canonical");
    registry
        .validate_canonical_value(PROVIDER_LAWPACK_ARTIFACT_DOMAIN, &changed_lawpack)
        .expect("the mismatching reference deliberately preserves the lawpack CDDL shape");
    let target_profile =
        decode_canonical_cbor(TARGET_PROFILE_BYTES).expect("target profile is canonical");
    let error = validate_generated_resource_identity_closure(
        &changed_lawpack,
        &target_profile,
        exact_generated_resource_materials(),
    )
    .expect_err("schema-valid reference bytes cannot replace the exact resource digest");
    assert_eq!(error, PackageIdentityFailure::DigestMismatch);
}

#[test]
fn schema_valid_reference_swap_cannot_hide_behind_union_equality() {
    let manifest = checked_manifest();
    let registry = declared_package_registry(&manifest);
    let mut lawpack = decode_canonical_cbor(LAWPACK_BYTES).expect("lawpack is canonical");
    let exports = map_field(&lawpack, "exports")
        .expect("lawpack carries exports")
        .clone();
    let compatibility = map_field(&lawpack, "compatibility")
        .expect("lawpack carries compatibility")
        .clone();
    *map_field_mut(&mut lawpack, "exports").expect("lawpack carries exports") = compatibility;
    *map_field_mut(&mut lawpack, "compatibility").expect("lawpack carries compatibility") = exports;
    let changed_bytes =
        encode_canonical_cbor(&lawpack).expect("swapped lawpack encodes canonically");
    let changed_lawpack =
        decode_canonical_cbor(&changed_bytes).expect("swapped lawpack remains canonical");
    registry
        .validate_canonical_value(PROVIDER_LAWPACK_ARTIFACT_DOMAIN, &changed_lawpack)
        .expect("the field swap deliberately preserves the lawpack CDDL shape");
    let target_profile =
        decode_canonical_cbor(TARGET_PROFILE_BYTES).expect("target profile is canonical");
    let error = validate_generated_resource_identity_closure(
        &changed_lawpack,
        &target_profile,
        exact_generated_resource_materials(),
    )
    .expect_err("coordinate-union equality cannot erase semantic field ownership");
    assert_eq!(error, PackageIdentityFailure::FieldReferenceMismatch);
}

#[test]
fn schema_valid_authority_source_mismatch_fails_primary_identity_closure() {
    let manifest = checked_manifest();
    let registry = declared_package_registry(&manifest);
    let lawpack = decode_canonical_cbor(LAWPACK_BYTES).expect("lawpack is canonical");
    let target_profile =
        decode_canonical_cbor(TARGET_PROFILE_BYTES).expect("target profile is canonical");
    let mut target_authority =
        decode_canonical_cbor(TARGET_AUTHORITY_BYTES).expect("target facts are canonical");
    let source =
        map_field_mut(&mut target_authority, "source").expect("target facts carry a source");
    *map_field_mut(source, "coordinate").expect("authority source carries a coordinate") =
        text("echo.dpo@1.schema-valid-mismatch");
    let changed_bytes = encode_canonical_cbor(&target_authority)
        .expect("changed authority facts encode canonically");
    let changed_target_authority =
        decode_canonical_cbor(&changed_bytes).expect("changed authority facts remain canonical");
    registry
        .validate_canonical_value(AUTHORITY_FACTS_API_VERSION, &changed_target_authority)
        .expect("the mismatching source deliberately preserves the authority-facts CDDL shape");
    let lawpack_authority =
        decode_canonical_cbor(LAWPACK_AUTHORITY_BYTES).expect("lawpack facts are canonical");
    let error = validate_adjacent_primary_identity_edges(
        &manifest,
        &lawpack,
        &target_profile,
        &changed_target_authority,
        &lawpack_authority,
    )
    .expect_err("schema-valid authority facts cannot change their exact subject identity");
    assert_eq!(error, PackageIdentityFailure::AuthoritySourceMismatch);
}

#[test]
fn malformed_manifest_fails_before_component_selection() {
    let mut manifest = checked_manifest();
    manifest.provider_abi = "edict:target-provider@invalid".to_owned();
    let report = bind_target_provider_manifest(&manifest)
        .expect_err("an invalid provider ABI cannot produce an Edict manifest proof");
    assert!(report.failures.iter().any(|failure| {
        failure.kind == edict_syntax::ProviderManifestValidationFailureKind::InvalidProviderAbi
    }));
}

#[test]
fn malformed_schema_fails_before_component_selection() {
    let mut manifest = checked_manifest();
    let malformed_schema: &[u8] = b"this is not CDDL";
    let schema = manifest
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.role == SCHEMA_ROLE)
        .expect("the package routes its schema artifact");
    schema.resource.digest = Some(raw_digest(malformed_schema));
    let proof = bind_target_provider_manifest(&manifest)
        .expect("the coherently rebound malformed schema remains a typed manifest");
    let required = required_domains(&manifest);
    let error = ProviderArtifactSchemaRegistry::from_manifest(
        &proof,
        [ResolvedProviderSchemaArtifact {
            role: SCHEMA_ROLE.to_owned(),
            bytes: Arc::<[u8]>::from(malformed_schema),
        }],
        required,
    )
    .expect_err("malformed schema bytes cannot produce an Edict registry");
    assert_eq!(
        error.kind(),
        ProviderSchemaRegistryFailureKind::SchemaCompileFailed
    );
}

#[test]
fn malformed_component_fails_during_preflight() {
    let mut manifest = checked_manifest();
    let mut malformed_component = LOWERER_BYTES.to_vec();
    malformed_component[0] ^= 1;
    let digest = raw_digest(&malformed_component);
    let lowerer = manifest
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.role == LOWERER_ROLE)
        .expect("the package routes its lowerer component");
    lowerer.resource.digest = Some(digest.clone());
    match &mut lowerer.source {
        ProviderArtifactSource::Component { component } => {
            component.digest = Some(digest);
        }
        ProviderArtifactSource::Generated { .. } => panic!("lowerer source must be a component"),
    }
    let proof = bind_target_provider_manifest(&manifest)
        .expect("the coherently rebound component remains a typed manifest");
    let selected =
        select_provider_component(&proof, LOWERER_ROLE, ProviderInvocationKind::Lowering)
            .expect("the coherently rebound lowerer still selects");
    let resolved = ResolvedProviderComponent::new(selected, Arc::<[u8]>::from(malformed_component));
    let host = ProviderComponentHost::new().expect("the deterministic host configures");
    let error = host
        .prepare(&resolved)
        .expect_err("malformed component bytes cannot pass Edict preflight");
    assert_eq!(error.kind(), ProviderHostFailureKind::ComponentDecodeFailed);
    assert_eq!(error.phase(), ProviderHostPhase::Preflight);
}

#[test]
fn malformed_artifact_fails_before_component_execution() {
    let manifest = checked_manifest();
    let proof = bind_target_provider_manifest(&manifest)
        .expect("the checked package manifest satisfies the Edict envelope");
    let required = required_domains(&manifest);
    let registry = ProviderArtifactSchemaRegistry::from_manifest(
        &proof,
        [ResolvedProviderSchemaArtifact {
            role: SCHEMA_ROLE.to_owned(),
            bytes: Arc::<[u8]>::from(SCHEMA_BYTES),
        }],
        required,
    )
    .expect("the exact manifest bindings produce an Edict registry");
    let core = echo_core();
    let (mut contract, mut request) = lowering_request(&core);
    let malformed =
        encode_canonical_cbor(&CanonicalValue::Null).expect("null has canonical CBOR bytes");
    let target_profile = bound_artifact(
        ECHO_DPO_TARGET_PROFILE,
        TARGET_PROFILE_API_VERSION,
        &malformed,
    );
    contract.target_profile = artifact_binding(&target_profile);
    request.target_profile = target_profile;

    let report = validate_provider_lowering_request(&registry, &contract, &request)
        .expect_err("schema-invalid target profile cannot produce an invocation proof");
    assert!(report.failures.iter().any(|failure| {
        failure.kind == ProviderInvocationValidationFailureKind::ArtifactSchemaMismatch
    }));
}

#[test]
fn checked_package_completes_external_builtin_parity_observation() {
    let observation = complete_package_conformance_observation();

    assert_eq!(
        observation.target_ir_digest,
        "sha256:2244345f046448c7b519ade05a167137659361ed144b46315ea32dabfbad85fc"
    );
    assert_eq!(observation.verifier_outcome, "accepted");
}
