// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unwrap_used
)]
//! Standalone Rust 1.94 witness for the frozen Edict provider-host contract.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use edict_provider_host_wasmtime::{
    PreparedProviderComponent, ProviderComponentHost, ProviderHostFailureKind, ProviderHostLimits,
    ProviderHostPhase, ProviderReplayObservation, ResolvedProviderComponent,
};
use edict_provider_schema::{ProviderArtifactSchemaRegistry, ResolvedProviderSchemaArtifact};
use edict_syntax::{
    bind_target_provider_manifest, compile_to_core, decode_canonical_cbor,
    digest_target_ir_artifact, encode_canonical_cbor, encode_core_module,
    encode_target_ir_artifact, lower_with_builtin_lowerer, parse_module, select_provider_component,
    validate_provider_lowering_request, BuiltinLowererRequest, BuiltinTargetLowerer,
    CanonicalValue, CompilerContext, CoreBudget, CoreModule, ProviderArtifact,
    ProviderArtifactBinding, ProviderArtifactKind, ProviderArtifactRef, ProviderArtifactSource,
    ProviderBoundArtifact, ProviderDiagnosticSeverity, ProviderDigest, ProviderDigestAlgorithm,
    ProviderInvocationKind, ProviderInvocationValidationFailureKind,
    ProviderLoweringInvocationContract, ProviderLoweringOutputKind, ProviderLoweringOutputRequest,
    ProviderLoweringRequest, ProviderOutputManifest, ProviderRefusalKind, ProviderResourceRef,
    ProviderResponseLimits, ProviderSchemaBinding, ProviderSchemaFormat, ProviderSemanticInput,
    ProviderSemanticInputBinding, ProviderSemanticInputKind,
    ProviderVerificationInvocationContract, ProviderVerificationOutputKind,
    ProviderVerificationOutputRequest, ProviderVerificationRequest, ProviderVerificationSuccess,
    ResourceRef, TargetEffectLowering, TargetIrLoweringFacts, TargetProviderManifest,
    ValidatedProviderLoweringRequest, ValidatedProviderVerificationRequest, WriteClass,
    AUTHORITY_FACTS_API_VERSION, CORE_DIGEST_FRAME, CORE_MODULE_DIGEST_DOMAIN,
    ECHO_DPO_TARGET_PROFILE, ECHO_SPAN_IR_DOMAIN, MAX_CANONICAL_NESTING_DEPTH,
    PROVIDER_LAWPACK_ARTIFACT_DOMAIN, TARGET_IR_ARTIFACT_DIGEST_DOMAIN, TARGET_PROFILE_API_VERSION,
    TARGET_PROVIDER_ABI, TARGET_PROVIDER_MANIFEST_API_VERSION, TARGET_PROVIDER_PROTOCOL_VERSION,
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

const LOWERABILITY_DOMAIN: &str = "edict.lowering-requirements/v1";
const TARGET_IR_ROLE: &str = "target-ir.echo-dpo";
const LOWERER_ROLE: &str = "lowerer.echo-dpo";
const VERIFIER_ROLE: &str = "verifier.echo-dpo";
const VERIFIER_REPORT_ROLE: &str = "verifier-report.echo-dpo";
const VERIFIER_REPORT_DOMAIN: &str = "echo.verifier-report/v1";
const DIAGNOSTIC_ABI_DIGEST: &str =
    "28fd72a98223153982ca084c29dbb1b2d430623967ab3b6db9d7fee668e614b9";
const SCHEMA_ROLE: &str = "schema.echo-provider-artifacts";
const RAW_TARGET_IR_SHA256: &str =
    "41ae7a1d95e5068cb09ec581f16a90cc6e26a80f83ec073e86d5108c3a61ea41";
const DOMAIN_TARGET_IR_SHA256: &str =
    "b0d9e218f00a102d1e951c73e5063a9bbe6077e6c7468d171ec08b420e7b47da";
const TARGET_PROFILE_SHA256: &str =
    "f41df38156625a05c1ee8bce652ffddf04e71b54fe027eeab9d255d0d8322db0";
const OBSERVATION_MARKER: &str = "ECHO_EDICT_HOST_OBSERVATION=";

const SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/generated/v1/primary/schema.echo-provider-artifacts.cddl"
);
const TARGET_PROFILE_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/generated/v1/primary/target-profile.echo-dpo.cbor"
);
const LAWPACK_BYTES: &[u8] =
    include_bytes!("../../../schemas/edict-provider/generated/v1/primary/lawpack.echo-dpo.cbor");
const TARGET_AUTHORITY_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/generated/v1/primary/authority-facts.echo-dpo.cbor"
);
const LAWPACK_AUTHORITY_BYTES: &[u8] = include_bytes!(
    "../../../schemas/edict-provider/generated/v1/primary/authority-facts.echo-lawpack.cbor"
);
const FIXTURE_LOWERER_BYTES: &[u8] =
    include_bytes!("../fixtures/edict-7cd8858c/lowerer.component.wasm");
const FIXTURE_MALFORMED_BYTES: &[u8] =
    include_bytes!("../fixtures/edict-7cd8858c/malformed-lowerer.component.wasm");

const FIXTURE_SCHEMA: &[u8] = br#"
artifact = null / {
  kind: "targetIrArtifact",
  domain: tstr,
  intents: { * tstr => any },
  targetProfile: { * tstr => any },
  sourceCoreCoordinate: tstr,
}
"#;
const NULL_BYTES: &[u8] = &[0xf6];
const FIXTURE_OUTPUT_DOMAIN: &str = "runtime.output/v1";

struct LowerHarness {
    host: ProviderComponentHost,
    prepared: PreparedProviderComponent<'static>,
    request: ValidatedProviderLoweringRequest<'static>,
    schema: &'static ProviderArtifactSchemaRegistry,
}

struct VerifyHarness {
    host: ProviderComponentHost,
    prepared: PreparedProviderComponent<'static>,
    request: ValidatedProviderVerificationRequest<'static>,
    schema: &'static ProviderArtifactSchemaRegistry,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn echo_component_path() -> PathBuf {
    let configured = std::env::var_os("ECHO_PROVIDER_LOWERER_COMPONENT").map(PathBuf::from);
    let path = configured.unwrap_or_else(|| {
        PathBuf::from("schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm")
    });
    if path.is_absolute() {
        path
    } else {
        repo_root().join(path)
    }
}

fn echo_component_bytes() -> &'static [u8] {
    let path = echo_component_path();
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    Box::leak(bytes.into_boxed_slice())
}

fn echo_verifier_component_path() -> PathBuf {
    let configured = std::env::var_os("ECHO_PROVIDER_VERIFIER_COMPONENT").map(PathBuf::from);
    let path = configured.unwrap_or_else(|| {
        PathBuf::from("schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm")
    });
    if path.is_absolute() {
        path
    } else {
        repo_root().join(path)
    }
}

fn echo_verifier_component_bytes() -> &'static [u8] {
    let path = echo_verifier_component_path();
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    Box::leak(bytes.into_boxed_slice())
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing hexadecimal to String cannot fail");
    }
    output
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "hexadecimal input has whole bytes");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => panic!("invalid hexadecimal digit"),
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

fn raw_sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn raw_resource(coordinate: &str, bytes: &[u8]) -> ResourceRef {
    ResourceRef {
        coordinate: coordinate.to_owned(),
        digest: Some(format!("sha256:{}", raw_sha256(bytes))),
    }
}

fn locked_test_resource(coordinate: &str, digit: char) -> ResourceRef {
    ResourceRef {
        coordinate: coordinate.to_owned(),
        digest: Some(format!("sha256:{}", digit.to_string().repeat(64))),
    }
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
    let value = map([
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
    ]);
    encode_canonical_cbor(&value).expect("lowerability facts encode canonically")
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

fn echo_manifest(component_bytes: &'static [u8]) -> &'static TargetProviderManifest {
    let component = raw_resource("echo.dpo.lowerer/component@1", component_bytes);
    let schema = raw_resource("echo.provider-artifacts.cddl@1", SCHEMA_BYTES);
    Box::leak(Box::new(TargetProviderManifest {
        api_version: TARGET_PROVIDER_MANIFEST_API_VERSION.to_owned(),
        provider_abi: TARGET_PROVIDER_ABI.to_owned(),
        provider: locked_test_resource("echo.edict-provider-host-witness@1", '1'),
        artifacts: vec![
            ProviderArtifactRef {
                role: LOWERER_ROLE.to_owned(),
                artifact_kind: ProviderArtifactKind::Lowerer,
                resource: component.clone(),
                source: ProviderArtifactSource::Component { component },
            },
            ProviderArtifactRef {
                role: SCHEMA_ROLE.to_owned(),
                artifact_kind: ProviderArtifactKind::ArtifactSchema,
                resource: schema,
                source: ProviderArtifactSource::Generated {
                    semantic_source: locked_test_resource(
                        "echo.edict-provider-host-witness.schema-source@1",
                        '2',
                    ),
                    generator: locked_test_resource(
                        "echo.edict-provider-host-witness.schema-generator@1",
                        '3',
                    ),
                },
            },
        ],
        schema_bindings: [
            (AUTHORITY_FACTS_API_VERSION, "authority-facts"),
            (CORE_MODULE_DIGEST_DOMAIN, "core-module"),
            (PROVIDER_LAWPACK_ARTIFACT_DOMAIN, "lawpack-manifest"),
            (LOWERABILITY_DOMAIN, "lowering-requirements"),
            (TARGET_IR_ARTIFACT_DIGEST_DOMAIN, "target-ir-artifact"),
            (TARGET_PROFILE_API_VERSION, "target-profile-manifest"),
        ]
        .into_iter()
        .map(|(domain, root_rule)| ProviderSchemaBinding {
            domain: domain.to_owned(),
            schema_role: SCHEMA_ROLE.to_owned(),
            format: ProviderSchemaFormat::SelfContainedCddlV1,
            root_rule: root_rule.to_owned(),
        })
        .collect(),
    }))
}

fn echo_registry(
    manifest: &'static TargetProviderManifest,
) -> &'static ProviderArtifactSchemaRegistry {
    let proof = bind_target_provider_manifest(manifest).expect("Echo provider manifest validates");
    Box::leak(Box::new(
        ProviderArtifactSchemaRegistry::from_manifest(
            &proof,
            [ResolvedProviderSchemaArtifact {
                role: SCHEMA_ROLE.to_owned(),
                bytes: Arc::from(SCHEMA_BYTES),
            }],
            [
                AUTHORITY_FACTS_API_VERSION,
                CORE_MODULE_DIGEST_DOMAIN,
                PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
                LOWERABILITY_DOMAIN,
                TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
                TARGET_PROFILE_API_VERSION,
            ],
        )
        .expect("Echo provider schema registry constructs"),
    ))
}

fn echo_verifier_manifest(component_bytes: &'static [u8]) -> &'static TargetProviderManifest {
    let component = raw_resource("echo.dpo.verifier/component@1", component_bytes);
    let schema = raw_resource("echo.provider-artifacts.cddl@1", SCHEMA_BYTES);
    Box::leak(Box::new(TargetProviderManifest {
        api_version: TARGET_PROVIDER_MANIFEST_API_VERSION.to_owned(),
        provider_abi: TARGET_PROVIDER_ABI.to_owned(),
        provider: locked_test_resource("echo.edict-provider-host-witness@1", '1'),
        artifacts: vec![
            ProviderArtifactRef {
                role: VERIFIER_ROLE.to_owned(),
                artifact_kind: ProviderArtifactKind::Verifier,
                resource: component.clone(),
                source: ProviderArtifactSource::Component { component },
            },
            ProviderArtifactRef {
                role: SCHEMA_ROLE.to_owned(),
                artifact_kind: ProviderArtifactKind::ArtifactSchema,
                resource: schema,
                source: ProviderArtifactSource::Generated {
                    semantic_source: locked_test_resource(
                        "echo.edict-provider-host-witness.schema-source@1",
                        '2',
                    ),
                    generator: locked_test_resource(
                        "echo.edict-provider-host-witness.schema-generator@1",
                        '3',
                    ),
                },
            },
        ],
        schema_bindings: [
            (VERIFIER_REPORT_DOMAIN, "verifier-report"),
            (AUTHORITY_FACTS_API_VERSION, "authority-facts"),
            (CORE_MODULE_DIGEST_DOMAIN, "core-module"),
            (PROVIDER_LAWPACK_ARTIFACT_DOMAIN, "lawpack-manifest"),
            (LOWERABILITY_DOMAIN, "lowering-requirements"),
            (TARGET_IR_ARTIFACT_DIGEST_DOMAIN, "target-ir-artifact"),
            (TARGET_PROFILE_API_VERSION, "target-profile-manifest"),
        ]
        .into_iter()
        .map(|(domain, root_rule)| ProviderSchemaBinding {
            domain: domain.to_owned(),
            schema_role: SCHEMA_ROLE.to_owned(),
            format: ProviderSchemaFormat::SelfContainedCddlV1,
            root_rule: root_rule.to_owned(),
        })
        .collect(),
    }))
}

fn echo_verifier_registry(
    manifest: &'static TargetProviderManifest,
) -> &'static ProviderArtifactSchemaRegistry {
    let proof = bind_target_provider_manifest(manifest).expect("Echo verifier manifest validates");
    Box::leak(Box::new(
        ProviderArtifactSchemaRegistry::from_manifest(
            &proof,
            [ResolvedProviderSchemaArtifact {
                role: SCHEMA_ROLE.to_owned(),
                bytes: Arc::from(SCHEMA_BYTES),
            }],
            [
                VERIFIER_REPORT_DOMAIN,
                AUTHORITY_FACTS_API_VERSION,
                CORE_MODULE_DIGEST_DOMAIN,
                PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
                LOWERABILITY_DOMAIN,
                TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
                TARGET_PROFILE_API_VERSION,
            ],
        )
        .expect("Echo verifier schema registry constructs"),
    ))
}

fn echo_request_from_core_bytes(
    core_bytes: &[u8],
    output_role: &str,
) -> (ProviderLoweringInvocationContract, ProviderLoweringRequest) {
    let core_artifact = bound_artifact("a.b@1", CORE_MODULE_DIGEST_DOMAIN, core_bytes);
    let target_profile_artifact = bound_artifact(
        ECHO_DPO_TARGET_PROFILE,
        TARGET_PROFILE_API_VERSION,
        TARGET_PROFILE_BYTES,
    );
    let lowerability = lowerability_bytes();
    let semantic_inputs = vec![
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
    ];
    let contract = ProviderLoweringInvocationContract {
        core: artifact_binding(&core_artifact),
        target_profile: artifact_binding(&target_profile_artifact),
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
        target_profile: target_profile_artifact,
        semantic_inputs,
        requested_outputs: vec![ProviderLoweringOutputRequest {
            role: output_role.to_owned(),
            kind: ProviderLoweringOutputKind::TargetIr,
            domain: TARGET_IR_ARTIFACT_DIGEST_DOMAIN.to_owned(),
        }],
        limits: ProviderResponseLimits {
            max_output_count: 8,
            max_diagnostic_count: 8,
            max_total_response_bytes: 64 * 1024,
        },
    };
    (contract, request)
}

fn echo_request(
    core: &CoreModule,
    output_role: &str,
) -> (ProviderLoweringInvocationContract, ProviderLoweringRequest) {
    let core_bytes = encode_core_module(core).expect("Core module encodes canonically");
    echo_request_from_core_bytes(&core_bytes, output_role)
}

fn echo_verification_request(
    core: &CoreModule,
    target_ir_bytes: &[u8],
) -> (
    ProviderVerificationInvocationContract,
    ProviderVerificationRequest,
) {
    let core_bytes = encode_core_module(core).expect("Core module encodes canonically");
    let core_artifact = bound_artifact("a.b@1", CORE_MODULE_DIGEST_DOMAIN, &core_bytes);
    let target_profile_artifact = bound_artifact(
        ECHO_DPO_TARGET_PROFILE,
        TARGET_PROFILE_API_VERSION,
        TARGET_PROFILE_BYTES,
    );
    let target_ir_artifact = bound_artifact(
        "echo.target-ir@1",
        TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
        target_ir_bytes,
    );
    let lowerability = lowerability_bytes();
    let semantic_inputs = vec![
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
    ];
    let contract = ProviderVerificationInvocationContract {
        core: artifact_binding(&core_artifact),
        target_profile: artifact_binding(&target_profile_artifact),
        target_ir: artifact_binding(&target_ir_artifact),
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
        target_profile: target_profile_artifact,
        target_ir: target_ir_artifact,
        semantic_inputs,
        requested_outputs: vec![ProviderVerificationOutputRequest {
            role: VERIFIER_REPORT_ROLE.to_owned(),
            kind: ProviderVerificationOutputKind::VerifierReport,
            domain: VERIFIER_REPORT_DOMAIN.to_owned(),
        }],
        limits: ProviderResponseLimits {
            max_output_count: 8,
            max_diagnostic_count: 8,
            max_total_response_bytes: 64 * 1024,
        },
    };
    (contract, request)
}

fn echo_verifier_harness_with_request(
    contract: ProviderVerificationInvocationContract,
    request: ProviderVerificationRequest,
) -> VerifyHarness {
    let component = echo_verifier_component_bytes();
    let manifest = echo_verifier_manifest(component);
    let manifest_proof = Box::leak(Box::new(
        bind_target_provider_manifest(manifest).expect("Echo verifier manifest validates"),
    ));
    let selected = select_provider_component(
        manifest_proof,
        VERIFIER_ROLE,
        ProviderInvocationKind::Verification,
    )
    .expect("Echo verifier selects");
    let resolved = ResolvedProviderComponent::new(selected, Arc::from(component));
    let host = ProviderComponentHost::new().expect("host configures");
    let prepared = host.prepare(&resolved).expect("Echo verifier prepares");
    let schema = echo_verifier_registry(manifest);
    let contract = Box::leak(Box::new(contract));
    let request = Box::leak(Box::new(request));
    let request = edict_syntax::validate_provider_verification_request(schema, contract, request)
        .expect("Echo verification request passes complete schema admission");
    VerifyHarness {
        host,
        prepared,
        request,
        schema,
    }
}

fn echo_verifier_harness(core: &CoreModule, target_ir_bytes: &[u8]) -> VerifyHarness {
    let (contract, request) = echo_verification_request(core, target_ir_bytes);
    echo_verifier_harness_with_request(contract, request)
}

fn canonical_map_field_mut<'a>(
    value: &'a mut CanonicalValue,
    field: &str,
) -> &'a mut CanonicalValue {
    let CanonicalValue::Map(entries) = value else {
        panic!("canonical value is not a map");
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .unwrap_or_else(|| panic!("canonical map field `{field}` is absent"))
}

fn canonical_core_intent_mut(core: &mut CanonicalValue) -> &mut CanonicalValue {
    canonical_map_field_mut(canonical_map_field_mut(core, "intents"), "t")
}

fn target_ir_with_mismatched_intrinsic(core: &CoreModule) -> Vec<u8> {
    let (target_ir_bytes, _) = oracle_target_ir(core);
    let mut target_ir =
        decode_canonical_cbor(&target_ir_bytes).expect("oracle Target IR decodes canonically");
    let intent = canonical_map_field_mut(canonical_map_field_mut(&mut target_ir, "intents"), "t");
    let CanonicalValue::Array(steps) = canonical_map_field_mut(intent, "steps") else {
        panic!("Target IR steps are not an array");
    };
    let step = steps
        .first_mut()
        .expect("reviewed Target IR has one effect step");
    *canonical_map_field_mut(step, "targetIntrinsic") = text("echo.dpo@1.unreviewed");
    encode_canonical_cbor(&target_ir).expect("mismatched Target IR encodes canonically")
}

fn resource_ref_value(reference: &ProviderResourceRef) -> CanonicalValue {
    assert_eq!(
        reference.digest.algorithm,
        ProviderDigestAlgorithm::Sha256,
        "reviewed resource reference uses sha256"
    );
    map([
        ("id", text(&reference.coordinate)),
        (
            "digest",
            CanonicalValue::Array(vec![
                text("sha256"),
                CanonicalValue::Bytes(reference.digest.bytes.clone()),
            ]),
        ),
    ])
}

fn expected_verifier_report_bytes(
    target_ir_reference: &ProviderResourceRef,
    outcome: &str,
) -> Vec<u8> {
    encode_canonical_cbor(&map([
        ("apiVersion", text(VERIFIER_REPORT_DOMAIN)),
        ("targetIr", resource_ref_value(target_ir_reference)),
        ("outcome", text(outcome)),
        (
            "diagnosticAbi",
            resource_ref_value(&ProviderResourceRef {
                coordinate: "edict.diagnostics/v1".to_owned(),
                digest: ProviderDigest {
                    algorithm: ProviderDigestAlgorithm::Sha256,
                    bytes: decode_hex(DIAGNOSTIC_ABI_DIGEST),
                },
            }),
        ),
        ("diagnosticBytes", CanonicalValue::Bytes(Vec::new())),
    ]))
    .expect("independent verifier report oracle encodes canonically")
}

fn assert_admitted_verifier_success(
    harness: &VerifyHarness,
    response: &ProviderVerificationSuccess,
    manifest: &ProviderOutputManifest<ProviderVerificationOutputKind>,
    expected_outcome: &str,
) {
    assert_eq!(response.outputs.len(), 1);
    let output = &response.outputs[0];
    assert_eq!(output.role, VERIFIER_REPORT_ROLE);
    assert_eq!(output.kind, ProviderVerificationOutputKind::VerifierReport);
    assert_eq!(output.artifact.domain, VERIFIER_REPORT_DOMAIN);
    assert_eq!(output.logical_path, None);

    let request = harness.request.request();
    let contract = harness.request.contract();
    let expected_bytes =
        expected_verifier_report_bytes(&request.target_ir.reference, expected_outcome);
    assert_eq!(output.artifact.bytes, expected_bytes);

    assert_eq!(manifest.invocation(), ProviderInvocationKind::Verification);
    assert_eq!(
        manifest.protocol_version(),
        TARGET_PROVIDER_PROTOCOL_VERSION
    );
    assert_eq!(manifest.inputs().core(), &contract.core);
    assert_eq!(manifest.inputs().target_profile(), &contract.target_profile);
    assert_eq!(manifest.inputs().target_ir(), Some(&contract.target_ir));
    assert_eq!(
        manifest.inputs().semantic_inputs(),
        contract.semantic_inputs.as_slice()
    );
    assert_eq!(manifest.requested_outputs().len(), 1);
    let requested = &manifest.requested_outputs()[0];
    assert_eq!(requested.role, VERIFIER_REPORT_ROLE);
    assert_eq!(
        requested.kind,
        ProviderVerificationOutputKind::VerifierReport
    );
    assert_eq!(requested.domain, VERIFIER_REPORT_DOMAIN);
    assert_eq!(manifest.outputs().len(), 1);
    let entry = &manifest.outputs()[0];
    assert_eq!(entry.role, VERIFIER_REPORT_ROLE);
    assert_eq!(entry.kind, ProviderVerificationOutputKind::VerifierReport);
    assert_eq!(entry.domain, VERIFIER_REPORT_DOMAIN);
    assert_eq!(entry.logical_path, None);
    assert_eq!(
        entry.digest,
        provider_digest(VERIFIER_REPORT_DOMAIN, &expected_bytes)
    );
}

fn echo_harness_with_request(
    contract: ProviderLoweringInvocationContract,
    request: ProviderLoweringRequest,
) -> LowerHarness {
    let component = echo_component_bytes();
    let manifest = echo_manifest(component);
    let manifest_proof = Box::leak(Box::new(
        bind_target_provider_manifest(manifest).expect("Echo provider manifest validates"),
    ));
    let selected = select_provider_component(
        manifest_proof,
        LOWERER_ROLE,
        ProviderInvocationKind::Lowering,
    )
    .expect("Echo lowerer selects");
    let resolved = ResolvedProviderComponent::new(selected, Arc::from(component));
    let host = ProviderComponentHost::new().expect("host configures");
    let prepared = host.prepare(&resolved).expect("Echo lowerer prepares");
    let schema = echo_registry(manifest);
    let contract = Box::leak(Box::new(contract));
    let request = Box::leak(Box::new(request));
    let request = validate_provider_lowering_request(schema, contract, request)
        .expect("Echo lowering request validates");
    LowerHarness {
        host,
        prepared,
        request,
        schema,
    }
}

fn echo_harness(core: &CoreModule, output_role: &str) -> LowerHarness {
    let (contract, request) = echo_request(core, output_role);
    echo_harness_with_request(contract, request)
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

fn assert_echo_refusal(
    harness: &LowerHarness,
    expected_kind: ProviderRefusalKind,
    expected_subject: Option<&str>,
) {
    let outcome = harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("typed Echo refusal crosses the WIT transport");
    assert!(outcome.response().is_none());
    assert!(outcome.manifest().is_none());
    let refusal = outcome
        .refusal()
        .expect("Echo component returned a typed refusal");
    assert_eq!(refusal.kind, expected_kind);
    assert_eq!(refusal.subject.as_deref(), expected_subject);

    let replay = harness
        .host
        .replay_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("typed Echo refusal replays identically in fresh stores");
    let ProviderReplayObservation::Completed(replayed) = replay.observation() else {
        panic!("typed Echo refusal must be a completed replay observation");
    };
    assert_eq!(replayed.refusal(), Some(refusal));
    assert!(replayed.response().is_none());
    assert!(replayed.manifest().is_none());
}

fn fixture_manifest(component_bytes: &'static [u8]) -> &'static TargetProviderManifest {
    let component = raw_resource("runtime.lowerer/component@1", component_bytes);
    let schema = raw_resource("runtime.artifacts.cddl@1", FIXTURE_SCHEMA);
    Box::leak(Box::new(TargetProviderManifest {
        api_version: TARGET_PROVIDER_MANIFEST_API_VERSION.to_owned(),
        provider_abi: TARGET_PROVIDER_ABI.to_owned(),
        provider: locked_test_resource("runtime.provider@1", '4'),
        artifacts: vec![
            ProviderArtifactRef {
                role: "lowerer.runtime".to_owned(),
                artifact_kind: ProviderArtifactKind::Lowerer,
                resource: component.clone(),
                source: ProviderArtifactSource::Component { component },
            },
            ProviderArtifactRef {
                role: "schema.runtime".to_owned(),
                artifact_kind: ProviderArtifactKind::ArtifactSchema,
                resource: schema,
                source: ProviderArtifactSource::Generated {
                    semantic_source: locked_test_resource("runtime.semantic-source@1", '5'),
                    generator: locked_test_resource("runtime.provider-generator@1", '6'),
                },
            },
        ],
        schema_bindings: [
            CORE_MODULE_DIGEST_DOMAIN,
            TARGET_PROFILE_API_VERSION,
            FIXTURE_OUTPUT_DOMAIN,
        ]
        .into_iter()
        .map(|domain| ProviderSchemaBinding {
            domain: domain.to_owned(),
            schema_role: "schema.runtime".to_owned(),
            format: ProviderSchemaFormat::SelfContainedCddlV1,
            root_rule: "artifact".to_owned(),
        })
        .collect(),
    }))
}

fn fixture_registry(
    manifest: &'static TargetProviderManifest,
) -> &'static ProviderArtifactSchemaRegistry {
    let proof = bind_target_provider_manifest(manifest).expect("fixture manifest validates");
    Box::leak(Box::new(
        ProviderArtifactSchemaRegistry::from_manifest(
            &proof,
            [ResolvedProviderSchemaArtifact {
                role: "schema.runtime".to_owned(),
                bytes: Arc::from(FIXTURE_SCHEMA),
            }],
            [
                CORE_MODULE_DIGEST_DOMAIN,
                TARGET_PROFILE_API_VERSION,
                FIXTURE_OUTPUT_DOMAIN,
            ],
        )
        .expect("fixture schema registry constructs"),
    ))
}

fn fixture_request(role: &str) -> (ProviderLoweringInvocationContract, ProviderLoweringRequest) {
    let core = bound_artifact("core@1", CORE_MODULE_DIGEST_DOMAIN, NULL_BYTES);
    let target_profile = bound_artifact("profile@1", TARGET_PROFILE_API_VERSION, NULL_BYTES);
    let contract = ProviderLoweringInvocationContract {
        core: artifact_binding(&core),
        target_profile: artifact_binding(&target_profile),
        semantic_inputs: Vec::new(),
    };
    let request = ProviderLoweringRequest {
        protocol_version: TARGET_PROVIDER_PROTOCOL_VERSION,
        core,
        target_profile,
        semantic_inputs: Vec::new(),
        requested_outputs: vec![ProviderLoweringOutputRequest {
            role: role.to_owned(),
            kind: ProviderLoweringOutputKind::GeneratedArtifact,
            domain: FIXTURE_OUTPUT_DOMAIN.to_owned(),
        }],
        limits: ProviderResponseLimits {
            max_output_count: 4,
            max_diagnostic_count: 4,
            max_total_response_bytes: 1024 * 1024,
        },
    };
    (contract, request)
}

fn fixture_harness_with_component(role: &str, component: &'static [u8]) -> LowerHarness {
    let manifest = fixture_manifest(component);
    let manifest_proof = Box::leak(Box::new(
        bind_target_provider_manifest(manifest).expect("fixture manifest validates"),
    ));
    let selected = select_provider_component(
        manifest_proof,
        "lowerer.runtime",
        ProviderInvocationKind::Lowering,
    )
    .expect("fixture lowerer selects");
    let resolved = ResolvedProviderComponent::new(selected, Arc::from(component));
    let host = ProviderComponentHost::new().expect("host configures");
    let prepared = host.prepare(&resolved).expect("fixture lowerer prepares");
    let schema = fixture_registry(manifest);
    let (contract, request) = fixture_request(role);
    let contract = Box::leak(Box::new(contract));
    let request = Box::leak(Box::new(request));
    let request = validate_provider_lowering_request(schema, contract, request)
        .expect("fixture request validates");
    LowerHarness {
        host,
        prepared,
        request,
        schema,
    }
}

fn fixture_harness(role: &str) -> LowerHarness {
    fixture_harness_with_component(role, FIXTURE_LOWERER_BYTES)
}

fn echo_observation() -> String {
    let core = echo_core();
    let harness = echo_harness(&core, TARGET_IR_ROLE);
    let replay = harness
        .host
        .replay_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("fresh-store Echo component observations agree");
    let ProviderReplayObservation::Completed(outcome) = replay.observation() else {
        panic!("valid Echo component must replay as a completed observation");
    };
    let response = outcome.response().expect("Echo component returns success");
    let manifest = outcome
        .manifest()
        .expect("host authors the Echo component output manifest");
    format!(
        "{}:{}:{}",
        raw_sha256(echo_component_bytes()),
        raw_sha256(&response.outputs[0].artifact.bytes),
        hex(&manifest.outputs()[0].digest.bytes)
    )
}

fn echo_verifier_observation() -> String {
    let core = echo_core();
    let (accepted_target_ir, _) = oracle_target_ir(&core);
    let accepted = echo_verifier_harness(&core, &accepted_target_ir);
    let accepted_outcome = accepted
        .host
        .invoke_verifier(
            &accepted.prepared,
            &accepted.request,
            accepted.schema,
            host_limits(),
        )
        .expect("accepted verifier observation crosses complete Edict host admission");
    assert!(accepted_outcome.refusal().is_none());
    let accepted_response = accepted_outcome
        .response()
        .expect("accepted verifier observation returns a response");
    let accepted_manifest = accepted_outcome
        .manifest()
        .expect("accepted verifier observation has a host-authored manifest");
    assert_admitted_verifier_success(&accepted, accepted_response, accepted_manifest, "accepted");

    let rejected_target_ir = target_ir_with_mismatched_intrinsic(&core);
    let rejected = echo_verifier_harness(&core, &rejected_target_ir);
    let rejected_outcome = rejected
        .host
        .invoke_verifier(
            &rejected.prepared,
            &rejected.request,
            rejected.schema,
            host_limits(),
        )
        .expect("rejected verifier observation crosses complete Edict host admission");
    assert!(rejected_outcome.refusal().is_none());
    let rejected_response = rejected_outcome
        .response()
        .expect("rejected verifier observation returns a response");
    let rejected_manifest = rejected_outcome
        .manifest()
        .expect("rejected verifier observation has a host-authored manifest");
    assert_admitted_verifier_success(&rejected, rejected_response, rejected_manifest, "rejected");
    let [rejected_diagnostic] = rejected_response.diagnostics.as_slice() else {
        panic!("rejected verifier observation has one diagnostic");
    };
    assert_eq!(
        rejected_diagnostic.code,
        "echo.verifier.target-intrinsic-mismatch"
    );
    assert_eq!(
        rejected_diagnostic.severity,
        ProviderDiagnosticSeverity::Error
    );
    assert_eq!(
        rejected_diagnostic.message,
        "Target IR uses an intrinsic outside the reviewed Echo capability"
    );
    assert_eq!(rejected_diagnostic.repair, None);

    let (refused_contract, mut refused_request) =
        echo_verification_request(&core, &accepted_target_ir);
    refused_request
        .requested_outputs
        .push(ProviderVerificationOutputRequest {
            role: "verifier-report.unreviewed".to_owned(),
            kind: ProviderVerificationOutputKind::VerifierReport,
            domain: VERIFIER_REPORT_DOMAIN.to_owned(),
        });
    let refused = echo_verifier_harness_with_request(refused_contract, refused_request);
    let refused_outcome = refused
        .host
        .invoke_verifier(
            &refused.prepared,
            &refused.request,
            refused.schema,
            host_limits(),
        )
        .expect("refused verifier observation crosses the WIT transport");
    assert!(refused_outcome.response().is_none());
    assert!(refused_outcome.manifest().is_none());
    let refusal = refused_outcome
        .refusal()
        .expect("refused verifier observation preserves a typed refusal");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedOutputRole);
    assert_eq!(
        refusal.subject.as_deref(),
        Some("verifier-report.unreviewed")
    );
    let [refusal_diagnostic] = refusal.diagnostics.as_slice() else {
        panic!("refused verifier observation has one diagnostic");
    };
    assert_eq!(
        refusal_diagnostic.code,
        "echo.verifier.unsupported-output-role"
    );
    assert_eq!(
        refusal_diagnostic.severity,
        ProviderDiagnosticSeverity::Error
    );
    assert_eq!(
        refusal_diagnostic.message,
        "the first verifier serves exactly one verifier-report.echo-dpo output"
    );
    assert_eq!(refusal_diagnostic.repair, None);

    format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        raw_sha256(echo_verifier_component_bytes()),
        raw_sha256(&accepted_response.outputs[0].artifact.bytes),
        hex(&accepted_manifest.outputs()[0].digest.bytes),
        raw_sha256(&rejected_response.outputs[0].artifact.bytes),
        hex(&rejected_manifest.outputs()[0].digest.bytes),
        rejected_diagnostic.code,
        "unsupported-output-role",
        refusal_diagnostic.code,
    )
}

fn oracle_target_ir(core: &CoreModule) -> (Vec<u8>, ProviderDigest) {
    let facts = TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: ECHO_DPO_TARGET_PROFILE.to_owned(),
            digest: Some(format!("sha256:{TARGET_PROFILE_SHA256}")),
        },
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
    .expect("Edict built-in Echo lowerer accepts the exact fixture");
    let artifact = report.artifact.expect("Echo Core lowers to Target IR");
    let bytes = encode_target_ir_artifact(&artifact).expect("oracle Target IR encodes");
    let digest = digest_target_ir_artifact(&artifact).expect("oracle Target IR digests");
    assert_eq!(digest.algorithm(), "sha256");
    (
        bytes,
        ProviderDigest {
            algorithm: ProviderDigestAlgorithm::Sha256,
            bytes: digest.bytes().to_vec(),
        },
    )
}

#[test]
fn echo_component_matches_independent_edict_target_ir_bytes_and_digest() {
    let core = echo_core();
    let core_bytes = encode_core_module(&core).expect("Core module encodes");
    assert_eq!(core_bytes.len(), 1209);
    assert_eq!(
        hex(&provider_digest(CORE_MODULE_DIGEST_DOMAIN, &core_bytes).bytes),
        "c3dbe413c78a82f6120e64c9a04bc94e2d79505f9e4b8a65c2bc26b408d775de"
    );
    assert_eq!(
        hex(&provider_digest(TARGET_PROFILE_API_VERSION, TARGET_PROFILE_BYTES).bytes),
        TARGET_PROFILE_SHA256
    );

    let (oracle_bytes, oracle_digest) = oracle_target_ir(&core);
    let harness = echo_harness(&core, TARGET_IR_ROLE);
    let outcome = harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("component result crosses complete Edict host admission");
    assert!(outcome.refusal().is_none());
    let response = outcome.response().expect("component returns success");
    assert!(response.diagnostics.is_empty());
    assert_eq!(response.outputs.len(), 1);
    let output = &response.outputs[0];
    assert_eq!(output.role, TARGET_IR_ROLE);
    assert_eq!(output.kind, ProviderLoweringOutputKind::TargetIr);
    assert_eq!(output.artifact.domain, TARGET_IR_ARTIFACT_DIGEST_DOMAIN);
    assert_eq!(output.logical_path, None);
    assert_eq!(output.artifact.bytes, oracle_bytes);
    assert_eq!(output.artifact.bytes.len(), 848);
    assert_eq!(raw_sha256(&output.artifact.bytes), RAW_TARGET_IR_SHA256);

    let manifest = outcome.manifest().expect("host authors an output manifest");
    assert_eq!(manifest.outputs().len(), 1);
    let entry = &manifest.outputs()[0];
    assert_eq!(entry.role, TARGET_IR_ROLE);
    assert_eq!(entry.kind, ProviderLoweringOutputKind::TargetIr);
    assert_eq!(entry.domain, TARGET_IR_ARTIFACT_DIGEST_DOMAIN);
    assert_eq!(entry.digest, oracle_digest);
    assert_eq!(hex(&entry.digest.bytes), DOMAIN_TARGET_IR_SHA256);

    let replay = harness
        .host
        .replay_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("Echo component success replays identically in fresh stores");
    let ProviderReplayObservation::Completed(replayed) = replay.observation() else {
        panic!("Echo component success must be a completed replay observation");
    };
    assert_eq!(replayed.response(), outcome.response());
    assert_eq!(replayed.manifest(), outcome.manifest());
    assert!(replayed.refusal().is_none());
}

#[test]
fn exact_merged_edict_host_fixtures_are_unchanged() {
    assert_eq!(FIXTURE_LOWERER_BYTES.len(), 28_007);
    assert_eq!(
        raw_sha256(FIXTURE_LOWERER_BYTES),
        "bec69c4fa02aa2dfb4f492d4a1c6849ae4ebe81c1725477efb6b0f2e885676aa"
    );
    assert_eq!(FIXTURE_MALFORMED_BYTES.len(), 2_165);
    assert_eq!(
        raw_sha256(FIXTURE_MALFORMED_BYTES),
        "dfcd171918373d18b9dff16778e98b7618eeb4ac85976dd7134b9e201562f41b"
    );
}

#[test]
fn provider_digest_accepts_the_maximum_canonical_value_depth() {
    let mut value = text("leaf");
    for _ in 0..MAX_CANONICAL_NESTING_DEPTH {
        value = CanonicalValue::Array(vec![value]);
    }
    let bytes = encode_canonical_cbor(&value).expect("maximum-depth value encodes");

    let digest = provider_digest("test.maximum-depth/v1", &bytes);

    assert_eq!(digest.algorithm, ProviderDigestAlgorithm::Sha256);
    assert_eq!(digest.bytes.len(), 32);
}

#[test]
fn echo_component_refuses_an_unsupported_profile_through_the_actual_host() {
    let core = echo_core();
    let (mut contract, mut request) = echo_request(&core, TARGET_IR_ROLE);
    request.target_profile.reference.coordinate = "echo.other@1".to_owned();
    contract.target_profile = artifact_binding(&request.target_profile);
    let harness = echo_harness_with_request(contract, request);

    assert_echo_refusal(
        &harness,
        ProviderRefusalKind::UnsupportedTargetProfile,
        Some("echo.other@1"),
    );
}

#[test]
fn echo_component_refuses_unsupported_core_semantics_through_the_actual_host() {
    let mut core = echo_core();
    core.coordinate = "x.y@1".to_owned();
    let (mut contract, mut request) = echo_request(&core, TARGET_IR_ROLE);
    request.core.reference.coordinate = core.coordinate;
    contract.core = artifact_binding(&request.core);
    let harness = echo_harness_with_request(contract, request);

    assert_echo_refusal(
        &harness,
        ProviderRefusalKind::UnsupportedSemantics,
        Some("x.y@1"),
    );
}

#[test]
fn echo_component_refuses_nonempty_input_constraints_through_the_actual_host() {
    let core_bytes = encode_core_module(&echo_core()).expect("Core module encodes canonically");
    let mut core = decode_canonical_cbor(&core_bytes).expect("Core module decodes canonically");
    *canonical_map_field_mut(canonical_core_intent_mut(&mut core), "inputConstraints") =
        CanonicalValue::Array(vec![map([
            ("coordinate", text("a.b@1.t.where.0")),
            ("source", text("where")),
            (
                "predicate",
                map([
                    ("kind", text("call")),
                    ("predicate", text("domain.Unreviewed")),
                    ("args", CanonicalValue::Array(Vec::new())),
                ]),
            ),
        ])]);
    let core_bytes = encode_canonical_cbor(&core).expect("mutated Core encodes canonically");
    let (contract, request) = echo_request_from_core_bytes(&core_bytes, TARGET_IR_ROLE);
    let harness = echo_harness_with_request(contract, request);

    assert_echo_refusal(
        &harness,
        ProviderRefusalKind::UnsupportedSemantics,
        Some("a.b@1.t"),
    );
}

#[test]
fn echo_component_refuses_incomplete_local_inventory_through_the_actual_host() {
    let core_bytes = encode_core_module(&echo_core()).expect("Core module encodes canonically");
    let mut core = decode_canonical_cbor(&core_bytes).expect("Core module decodes canonically");
    let body = canonical_map_field_mut(canonical_core_intent_mut(&mut core), "body");
    let CanonicalValue::Array(locals) = canonical_map_field_mut(body, "locals") else {
        panic!("Core locals is not an array");
    };
    locals.retain(|local| {
        let CanonicalValue::Map(entries) = local else {
            return true;
        };
        !entries
            .iter()
            .any(|(key, value)| key == &text("type") && value == &text("target.replace.rejected"))
    });
    let core_bytes = encode_canonical_cbor(&core).expect("mutated Core encodes canonically");
    let (contract, request) = echo_request_from_core_bytes(&core_bytes, TARGET_IR_ROLE);
    let harness = echo_harness_with_request(contract, request);

    assert_echo_refusal(
        &harness,
        ProviderRefusalKind::InvalidSemanticArtifact,
        Some("a.b@1.t"),
    );
}

#[test]
fn echo_component_refuses_output_overclaim_through_the_actual_host() {
    let core = echo_core();
    let harness = echo_harness(&core, "generated.echo-dpo");

    assert_echo_refusal(
        &harness,
        ProviderRefusalKind::UnsupportedOutputRole,
        Some("generated.echo-dpo"),
    );
}

#[test]
fn wrong_target_profile_domain_rejects_before_component_invocation() {
    let core = echo_core();
    let (mut contract, mut request) = echo_request(&core, TARGET_IR_ROLE);
    request.target_profile = bound_artifact(
        ECHO_DPO_TARGET_PROFILE,
        "wrong.target-profile/v1",
        TARGET_PROFILE_BYTES,
    );
    contract.target_profile = artifact_binding(&request.target_profile);
    let manifest = echo_manifest(echo_component_bytes());
    let schema = echo_registry(manifest);

    let report = validate_provider_lowering_request(schema, &contract, &request)
        .expect_err("wrong fixed target-profile domain rejects before invocation");
    assert!(report.failures.iter().any(|failure| {
        failure.kind == ProviderInvocationValidationFailureKind::ArtifactDomainMismatch
    }));
}

#[test]
fn typed_provider_refusal_is_completed_without_an_output_manifest() {
    let harness = fixture_harness("fixture.refusal");
    let outcome = harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("typed refusal crosses the WIT transport");
    assert!(outcome.response().is_none());
    assert!(outcome.manifest().is_none());
    let refusal = outcome
        .refusal()
        .expect("provider returned a typed refusal");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(refusal.subject, None);

    let replay = harness
        .host
        .replay_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("typed refusal replays identically in fresh stores");
    let ProviderReplayObservation::Completed(replayed) = replay.observation() else {
        panic!("typed refusal must be a completed replay observation");
    };
    assert_eq!(replayed.refusal(), Some(refusal));
    assert!(replayed.manifest().is_none());
}

#[test]
fn host_rejections_preserve_trap_lifting_and_envelope_identity() {
    let trapped = fixture_harness("fixture.trap");
    let failure = trapped
        .host
        .invoke_lowerer(
            &trapped.prepared,
            &trapped.request,
            trapped.schema,
            host_limits(),
        )
        .expect_err("explicit guest trap rejects");
    assert_eq!(failure.kind(), ProviderHostFailureKind::GuestTrap);
    assert_eq!(failure.phase(), ProviderHostPhase::Lower);
    assert!(failure.validation_report().is_none());

    let malformed = fixture_harness_with_component("output.runtime", FIXTURE_MALFORMED_BYTES);
    let failure = malformed
        .host
        .invoke_lowerer(
            &malformed.prepared,
            &malformed.request,
            malformed.schema,
            host_limits(),
        )
        .expect_err("invalid canonical ABI discriminant rejects during lifting");
    assert_eq!(failure.kind(), ProviderHostFailureKind::MalformedResponse);
    assert!(failure.validation_report().is_none());

    let undeclared = fixture_harness("fixture.undeclared-output");
    let failure = undeclared
        .host
        .invoke_lowerer(
            &undeclared.prepared,
            &undeclared.request,
            undeclared.schema,
            host_limits(),
        )
        .expect_err("undeclared output cannot cross host admission");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResponseEnvelopeInvalid
    );
    assert_eq!(failure.phase(), ProviderHostPhase::ValidateResponse);
    let report = failure
        .validation_report()
        .expect("envelope rejection retains structured validation evidence");
    assert!(report
        .failures
        .iter()
        .any(|item| { item.kind == ProviderInvocationValidationFailureKind::UndeclaredOutput }));
}

#[test]
fn rejected_host_invocation_replays_with_the_same_typed_failure() {
    let harness = fixture_harness("fixture.trap");
    let replay = harness
        .host
        .replay_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("guest trap replays as equal host rejections");
    let ProviderReplayObservation::Rejected(failure) = replay.observation() else {
        panic!("guest trap must remain a rejected replay observation");
    };
    assert_eq!(failure.kind(), ProviderHostFailureKind::GuestTrap);
    assert_eq!(failure.phase(), ProviderHostPhase::Lower);
}

#[test]
#[ignore = "child entrypoint exercised by the independent-process witness"]
fn emit_echo_component_host_observation() {
    println!("{OBSERVATION_MARKER}{}", echo_observation());
}

#[test]
fn independent_processes_reproduce_the_same_echo_component_observation() {
    let executable = std::env::current_exe().expect("current test executable is discoverable");
    let run_child = || {
        let output = Command::new(&executable)
            .arg("emit_echo_component_host_observation")
            .args(["--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .output()
            .expect("child host process launches");
        assert!(
            output.status.success(),
            "child host witness failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("child output is UTF-8");
        stdout
            .lines()
            .find_map(|line| {
                line.split_once(OBSERVATION_MARKER)
                    .map(|(_, observation)| observation)
            })
            .unwrap_or_else(|| panic!("child omitted stable observation:\n{stdout}"))
            .to_owned()
    };

    let first = run_child();
    let second = run_child();
    assert_eq!(first, second);
}

#[test]
fn echo_verifier_component_admits_the_exact_relation_through_the_actual_host() {
    let core = echo_core();
    let (target_ir_bytes, _) = oracle_target_ir(&core);
    let harness = echo_verifier_harness(&core, &target_ir_bytes);
    let outcome = harness
        .host
        .invoke_verifier(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("accepted Echo relation crosses complete Edict host admission");
    assert!(outcome.refusal().is_none());
    let response = outcome
        .response()
        .expect("accepted Echo relation returns a verifier report");
    assert!(response.diagnostics.is_empty());
    let manifest = outcome
        .manifest()
        .expect("host authors the accepted verifier output manifest");
    assert_admitted_verifier_success(&harness, response, manifest, "accepted");
}

#[test]
fn echo_verifier_component_admits_a_well_formed_semantic_rejection() {
    let core = echo_core();
    let target_ir_bytes = target_ir_with_mismatched_intrinsic(&core);
    let harness = echo_verifier_harness(&core, &target_ir_bytes);
    let outcome = harness
        .host
        .invoke_verifier(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("well-formed semantic disagreement crosses complete Edict host admission");
    assert!(outcome.refusal().is_none());
    let response = outcome
        .response()
        .expect("semantic disagreement returns a rejected verifier report");
    assert_eq!(response.diagnostics.len(), 1);
    let diagnostic = &response.diagnostics[0];
    assert_eq!(diagnostic.code, "echo.verifier.target-intrinsic-mismatch");
    assert_eq!(diagnostic.severity, ProviderDiagnosticSeverity::Error);
    assert_eq!(
        diagnostic.message,
        "Target IR uses an intrinsic outside the reviewed Echo capability"
    );
    assert_eq!(diagnostic.repair, None);
    let manifest = outcome
        .manifest()
        .expect("host authors the rejected verifier output manifest");
    assert_admitted_verifier_success(&harness, response, manifest, "rejected");
}

#[test]
fn echo_verifier_component_preserves_a_typed_output_overclaim_refusal() {
    let core = echo_core();
    let (target_ir_bytes, _) = oracle_target_ir(&core);
    let (contract, mut request) = echo_verification_request(&core, &target_ir_bytes);
    request
        .requested_outputs
        .push(ProviderVerificationOutputRequest {
            role: "verifier-report.unreviewed".to_owned(),
            kind: ProviderVerificationOutputKind::VerifierReport,
            domain: VERIFIER_REPORT_DOMAIN.to_owned(),
        });
    let harness = echo_verifier_harness_with_request(contract, request);
    let outcome = harness
        .host
        .invoke_verifier(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("typed verifier refusal crosses the WIT transport");
    assert!(outcome.response().is_none());
    assert!(outcome.manifest().is_none());
    let refusal = outcome
        .refusal()
        .expect("output overclaim returns a typed provider refusal");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedOutputRole);
    assert_eq!(
        refusal.subject.as_deref(),
        Some("verifier-report.unreviewed")
    );
    assert_eq!(refusal.diagnostics.len(), 1);
    let diagnostic = &refusal.diagnostics[0];
    assert_eq!(diagnostic.code, "echo.verifier.unsupported-output-role");
    assert_eq!(diagnostic.severity, ProviderDiagnosticSeverity::Error);
    assert_eq!(
        diagnostic.message,
        "the first verifier serves exactly one verifier-report.echo-dpo output"
    );
    assert_eq!(diagnostic.repair, None);
}

#[test]
fn echo_verifier_component_replays_all_completed_outcome_classes_identically() {
    let core = echo_core();
    let (accepted_target_ir, _) = oracle_target_ir(&core);
    let accepted = echo_verifier_harness(&core, &accepted_target_ir);
    let rejected_target_ir = target_ir_with_mismatched_intrinsic(&core);
    let rejected = echo_verifier_harness(&core, &rejected_target_ir);
    let (refusal_contract, mut refusal_request) =
        echo_verification_request(&core, &accepted_target_ir);
    refusal_request
        .requested_outputs
        .push(ProviderVerificationOutputRequest {
            role: "verifier-report.unreviewed".to_owned(),
            kind: ProviderVerificationOutputKind::VerifierReport,
            domain: VERIFIER_REPORT_DOMAIN.to_owned(),
        });
    let refused = echo_verifier_harness_with_request(refusal_contract, refusal_request);

    for (harness, expects_refusal) in [(&accepted, false), (&rejected, false), (&refused, true)] {
        let outcome = harness
            .host
            .invoke_verifier(
                &harness.prepared,
                &harness.request,
                harness.schema,
                host_limits(),
            )
            .expect("completed verifier outcome crosses the Edict host");
        let replay = harness
            .host
            .replay_verifier(
                &harness.prepared,
                &harness.request,
                harness.schema,
                host_limits(),
            )
            .expect("verifier outcome replays identically in fresh stores");
        let ProviderReplayObservation::Completed(replayed) = replay.observation() else {
            panic!("accepted, rejected, and refused outcomes must all be completed observations");
        };
        assert_eq!(replayed, &outcome);
        if expects_refusal {
            assert!(outcome.response().is_none());
            assert!(outcome.manifest().is_none());
            assert!(outcome.refusal().is_some());
        } else {
            assert!(outcome.response().is_some());
            assert!(outcome.manifest().is_some());
            assert!(outcome.refusal().is_none());
        }
    }
}

#[test]
#[ignore = "child entrypoint exercised by the verifier independent-process witness"]
fn emit_echo_verifier_host_observation() {
    println!("{OBSERVATION_MARKER}{}", echo_verifier_observation());
}

#[test]
fn independent_processes_reproduce_the_same_echo_verifier_observation() {
    let executable = std::env::current_exe().expect("current test executable is discoverable");
    let run_child = || {
        let output = Command::new(&executable)
            .arg("emit_echo_verifier_host_observation")
            .args(["--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .output()
            .expect("child verifier host process launches");
        assert!(
            output.status.success(),
            "child verifier host witness failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("child output is UTF-8");
        stdout
            .lines()
            .find_map(|line| {
                line.split_once(OBSERVATION_MARKER)
                    .map(|(_, observation)| observation)
            })
            .unwrap_or_else(|| panic!("child omitted stable verifier observation:\n{stdout}"))
            .to_owned()
    };

    let first = run_child();
    let second = run_child();
    assert_eq!(first, second);
}
