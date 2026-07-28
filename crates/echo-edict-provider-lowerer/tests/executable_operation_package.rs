// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Generic executable-operation lowering witnesses.

use blake3::Hasher;
use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, encode_canonical_cbor_v1,
    CanonicalValueV1,
};
use echo_edict_provider_lowerer::{
    lower, Artifact, BoundArtifact, Digest, DigestAlgorithm, LoweringOutputKind,
    LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1, ResourceRef, ResponseLimitsV1,
    SemanticInput, SemanticInputKind,
};
use warp_core::{
    echo_operation_create_if_absent_target_profile_identity_v1, EchoOperationBudgetV1,
    EchoOperationProgramV1, EchoOperationSemanticClosureV1, ExecutableOperationPackageV1, TypeId,
};

const TARGET_PROFILE: &[u8] = include_bytes!("../resources/target-profile.echo-dpo.cbor");
const PACKAGE_ROLE: &str = "executable-operation-package.echo";
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const TARGET_INTRINSIC: &str = "echo.dpo@1.anchored-node-attachment-create-if-absent";
const PRECONDITION_MISMATCH: &str = "echo.executable-operation/precondition-mismatch/v1";

#[derive(Clone, Copy)]
struct FixtureNames<'a> {
    application: &'a str,
    intent: &'a str,
    alias: &'a str,
    effect_member: &'a str,
    lawpack: &'a str,
    lawpack_id: &'a str,
    lawpack_version: &'a str,
    exports: &'a str,
    adapter: &'a str,
    configuration: &'a str,
    effect: &'a str,
    failure: &'a str,
    obstruction: &'a str,
    node_type: &'a str,
    attachment_type: &'a str,
    authority: &'a str,
}

const ALPHA: FixtureNames<'static> = FixtureNames {
    application: "examples.alpha@1",
    intent: "createGreeting",
    alias: "cell",
    effect_member: "createIfAbsent",
    lawpack: "causal.cell@1",
    lawpack_id: "causal.cell",
    lawpack_version: "1",
    exports: "causal.cell.exports/v1",
    adapter: "causal.cell.echo-adapter/v1",
    configuration: "causal.cell.echo-create-configuration/v1",
    effect: "causal.cell@1.createIfAbsent",
    failure: "alreadyExists",
    obstruction: "causal.cell@1.AlreadyExists",
    node_type: "examples.alpha.node.greeting/v1",
    attachment_type: "examples.alpha.attachment.message/v1",
    authority: "examples.alpha.authority.local/v1",
};

const BETA: FixtureNames<'static> = FixtureNames {
    application: "notes.beta@7",
    intent: "createNote",
    alias: "portable",
    effect_member: "createIfAbsent",
    lawpack: "portable.cell@3",
    lawpack_id: "portable.cell",
    lawpack_version: "3",
    exports: "portable.cell.exports/v3",
    adapter: "portable.cell.echo-adapter/v3",
    configuration: "portable.cell.echo-create-configuration/v3",
    effect: "portable.cell@3.createIfAbsent",
    failure: "occupied",
    obstruction: "portable.cell@3.Occupied",
    node_type: "notes.beta.node.note/v7",
    attachment_type: "notes.beta.attachment.body/v7",
    authority: "notes.beta.authority.local/v7",
};

#[test]
fn one_provider_binary_lowers_two_unrelated_application_vocabularies() {
    let alpha = lower(fixture_request(ALPHA)).expect("the alpha application lowers");
    let beta = lower(fixture_request(BETA)).expect("the beta application lowers");

    let alpha_package = &alpha.outputs[0];
    let beta_package = &beta.outputs[0];
    assert_eq!(alpha_package.role, PACKAGE_ROLE);
    assert_eq!(beta_package.role, PACKAGE_ROLE);
    assert_eq!(alpha_package.artifact.domain, PACKAGE_DOMAIN);
    assert_eq!(beta_package.artifact.domain, PACKAGE_DOMAIN);
    assert_ne!(alpha_package.artifact.bytes, beta_package.artifact.bytes);

    let alpha_decoded = decode_canonical_cbor_v1(&alpha_package.artifact.bytes)
        .expect("the alpha output is canonical");
    let beta_decoded = decode_canonical_cbor_v1(&beta_package.artifact.bytes)
        .expect("the beta output is canonical");
    assert_eq!(
        text_field(&alpha_decoded, "operation_coordinate"),
        Some("examples.alpha@1.createGreeting")
    );
    assert_eq!(
        text_field(&beta_decoded, "operation_coordinate"),
        Some("notes.beta@7.createNote")
    );
    assert_eq!(
        text_field(&alpha_decoded, "obstruction_coordinate"),
        Some(ALPHA.obstruction)
    );
    assert_eq!(
        text_field(&beta_decoded, "obstruction_coordinate"),
        Some(BETA.obstruction)
    );
}

#[test]
fn generic_lowering_matches_the_independent_warp_core_package_model() {
    for names in [ALPHA, BETA] {
        let request = fixture_request(names);
        let expected = expected_package(&request, names)
            .to_canonical_bytes()
            .expect("warp-core encodes the independent package model");

        let success = lower(request).expect("the generic fixture lowers");

        assert!(success.diagnostics.is_empty());
        let [output] = success.outputs.as_slice() else {
            panic!("generic lowering must emit exactly one package");
        };
        assert_eq!(output.kind, LoweringOutputKind::GeneratedArtifact);
        assert_eq!(output.artifact.bytes, expected);
        assert_eq!(output.logical_path, None);
    }
}

#[test]
fn semantic_roles_do_not_encode_application_names() {
    let mut request = fixture_request(ALPHA);
    for (index, input) in request.semantic_inputs.iter_mut().enumerate() {
        input.role = format!("closure-member-{index}");
    }

    let success = lower(request).expect("structural input kinds and bindings are authoritative");

    assert_eq!(success.outputs.len(), 1);
}

fn fixture_request(names: FixtureNames<'_>) -> LoweringRequestV1 {
    let target_profile = bound(
        "echo.dpo@1",
        "edict.target-profile/v1",
        TARGET_PROFILE.to_vec(),
    );
    let exports = bound(
        names.exports,
        "edict.lawpack-exports/v1",
        canonical_bytes(&exports(names)),
    );
    let configuration = bound(
        names.configuration,
        "echo.operation-lowering-configuration/v1",
        canonical_bytes(&configuration(names)),
    );
    let adapter = bound(
        names.adapter,
        "edict.lawpack-adapter/v1",
        canonical_bytes(&adapter(names, &configuration)),
    );
    let lawpack = bound(
        names.lawpack,
        "edict.lawpack/v1",
        canonical_bytes(&lawpack(names, &exports, &adapter, &target_profile)),
    );
    let core = bound(
        names.application,
        "edict.core.module/v1",
        canonical_bytes(&core(names)),
    );
    let source = bound(
        names.application,
        "edict.source/v1",
        canonical_bytes(&CanonicalValueV1::Bytes(
            format!(
                "package {};\n\nuse lawpack {} digest \"{}\" as {};\n\nintent {} uses {};",
                names.application,
                names.lawpack,
                digest_review(&lawpack.reference.digest.bytes),
                names.alias,
                names.intent,
                names.effect
            )
            .into_bytes(),
        )),
    );
    let target_ir = bound(
        "echo.span-ir/v1",
        "edict.target-ir.artifact/v1",
        canonical_bytes(&target_ir(names, &core, &lawpack, &target_profile)),
    );

    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core,
        target_profile,
        semantic_inputs: vec![
            semantic_input(
                "adapter",
                SemanticInputKind::Auxiliary("lawpack-adapter".to_owned()),
                adapter,
            ),
            semantic_input(
                "exports",
                SemanticInputKind::Auxiliary("lawpack-exports".to_owned()),
                exports,
            ),
            semantic_input("lawpack", SemanticInputKind::Lawpack, lawpack),
            semantic_input(
                "source",
                SemanticInputKind::Auxiliary("edict-source".to_owned()),
                source,
            ),
            semantic_input(
                "configuration",
                SemanticInputKind::Auxiliary("target-configuration".to_owned()),
                configuration,
            ),
            semantic_input(
                "target-ir",
                SemanticInputKind::Auxiliary("target-ir".to_owned()),
                target_ir,
            ),
        ],
        requested_outputs: vec![LoweringOutputRequest {
            role: PACKAGE_ROLE.to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE_DOMAIN.to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 1,
            max_diagnostic_count: 0,
            max_total_response_bytes: 16 * 1024,
        },
    }
}

fn core(names: FixtureNames<'_>) -> CanonicalValueV1 {
    owned_map([
        ("apiVersion", text("edict.core/v1")),
        ("coordinate", text(names.application)),
        (
            "intents",
            dynamic_map([(
                names.intent,
                owned_map([
                    ("input", text(format!("{}.Input", names.application))),
                    ("output", text(format!("{}.Output", names.application))),
                    (
                        "requiredOperationProfile",
                        text("continuum.profile.create/v1"),
                    ),
                    (
                        "body",
                        owned_map([(
                            "nodes",
                            CanonicalValueV1::Array(vec![owned_map([
                                ("kind", text("effect")),
                                (
                                    "effect",
                                    text(format!("{}.{}", names.alias, names.effect_member)),
                                ),
                                (
                                    "obstructionMap",
                                    dynamic_map([(
                                        names.failure,
                                        owned_map([("callee", text(names.obstruction))]),
                                    )]),
                                ),
                            ])]),
                        )]),
                    ),
                ]),
            )]),
        ),
    ])
}

fn digest_review(bytes: &[u8]) -> String {
    let mut value = String::from("sha256:");
    for byte in bytes {
        use std::fmt::Write as _;
        write!(value, "{byte:02x}").expect("writing to a String cannot fail");
    }
    value
}

fn exports(names: FixtureNames<'_>) -> CanonicalValueV1 {
    owned_map([(
        "effects",
        CanonicalValueV1::Array(vec![owned_map([
            ("coordinate", text(names.effect)),
            ("executionClass", text("runtime")),
            (
                "effectFailures",
                dynamic_map([(
                    names.failure,
                    owned_map([("authorityClass", text("domainMappable"))]),
                )]),
            ),
        ])]),
    )])
}

fn configuration(names: FixtureNames<'_>) -> CanonicalValueV1 {
    owned_map([
        (
            "apiVersion",
            text("echo.operation-lowering-configuration/v1"),
        ),
        (
            "programKind",
            text("anchored-node-attachment-create-if-absent/v1"),
        ),
        ("requiredNodeTypeProfile", text(names.node_type)),
        ("requiredAttachmentTypeProfile", text(names.attachment_type)),
        ("maxReplacementBytes", integer(256)),
        ("authorityProfile", text(names.authority)),
        (
            "budgetCeiling",
            owned_map([
                ("steps", integer(16)),
                ("readBytes", integer(64)),
                ("writeBytes", integer(320)),
            ]),
        ),
        (
            "invocationBinding",
            owned_map([
                ("nodeKeyField", text("key")),
                ("replacementField", text("value")),
                ("nodeIdDerivation", text("sha256-utf8/v1")),
                ("warpIdSource", text("action-lane/v1")),
            ]),
        ),
    ])
}

fn adapter(names: FixtureNames<'_>, configuration: &BoundArtifact) -> CanonicalValueV1 {
    owned_map([
        ("apiVersion", text("edict.lawpack-adapter/v1")),
        ("class", text("declarative")),
        (
            "effectImplementations",
            dynamic_map([(
                names.effect,
                owned_map([
                    ("targetIntrinsic", text(TARGET_INTRINSIC)),
                    (
                        "targetConfiguration",
                        resource_ref(&configuration.reference),
                    ),
                    ("writeClass", text("create")),
                    (
                        "failureMappings",
                        dynamic_map([(names.failure, text(PRECONDITION_MISMATCH))]),
                    ),
                ]),
            )]),
        ),
    ])
}

fn lawpack(
    names: FixtureNames<'_>,
    exports: &BoundArtifact,
    adapter: &BoundArtifact,
    target_profile: &BoundArtifact,
) -> CanonicalValueV1 {
    owned_map([
        ("apiVersion", text("edict.lawpack/v1")),
        ("id", text(names.lawpack_id)),
        ("version", text(names.lawpack_version)),
        ("exports", owner_resource_ref(exports)),
        (
            "targetAdapters",
            CanonicalValueV1::Array(vec![owned_map([
                ("adapter", owner_resource_ref(adapter)),
                (
                    "acceptedTargetProfile",
                    resource_ref(&target_profile.reference),
                ),
            ])]),
        ),
    ])
}

fn owner_resource_ref(artifact: &BoundArtifact) -> CanonicalValueV1 {
    let value =
        decode_canonical_cbor_v1(&artifact.artifact.bytes).expect("owner artifact is canonical");
    let digest = digest_canonical_value_bytes_v1(&artifact.reference.coordinate, &value)
        .expect("owner-framed digest is computable");
    owned_map([
        ("id", text(&artifact.reference.coordinate)),
        (
            "digest",
            CanonicalValueV1::Array(vec![
                text("sha256"),
                CanonicalValueV1::Bytes(digest.to_vec()),
            ]),
        ),
    ])
}

fn target_ir(
    names: FixtureNames<'_>,
    core: &BoundArtifact,
    lawpack: &BoundArtifact,
    target_profile: &BoundArtifact,
) -> CanonicalValueV1 {
    owned_map([
        ("kind", text("targetIrArtifact")),
        ("domain", text("echo.span-ir/v1")),
        ("targetProfile", resource_ref(&target_profile.reference)),
        ("sourceCoreCoordinate", text(names.application)),
        (
            "semanticClosure",
            owned_map([
                ("sourceCore", resource_ref(&core.reference)),
                (
                    "lawpacks",
                    CanonicalValueV1::Array(vec![resource_ref(&lawpack.reference)]),
                ),
            ]),
        ),
        (
            "intents",
            dynamic_map([(
                names.intent,
                owned_map([
                    ("operationProfile", text("continuum.profile.create/v1")),
                    (
                        "steps",
                        CanonicalValueV1::Array(vec![owned_map([
                            ("targetIntrinsic", text(TARGET_INTRINSIC)),
                            (
                                "obstructionFailures",
                                CanonicalValueV1::Array(vec![text(PRECONDITION_MISMATCH)]),
                            ),
                        ])]),
                    ),
                ]),
            )]),
        ),
    ])
}

fn expected_package(
    request: &LoweringRequestV1,
    names: FixtureNames<'_>,
) -> ExecutableOperationPackageV1 {
    let input = |kind: &SemanticInputKind| {
        &request
            .semantic_inputs
            .iter()
            .find(|input| &input.kind == kind)
            .expect("fixture semantic input exists")
            .artifact
    };
    let source = input(&SemanticInputKind::Auxiliary("edict-source".to_owned()));
    let exports = input(&SemanticInputKind::Auxiliary("lawpack-exports".to_owned()));
    let lawpack = input(&SemanticInputKind::Lawpack);
    let target_ir = input(&SemanticInputKind::Auxiliary("target-ir".to_owned()));
    let core_identity = hash(&request.core.reference.digest);
    ExecutableOperationPackageV1::new(
        format!("{}.{}", names.application, names.intent),
        EchoOperationSemanticClosureV1::new(
            hash(&source.reference.digest),
            core_identity,
            core_identity,
            hash(&target_ir.reference.digest),
            &exports.reference.coordinate,
            hash(&exports.reference.digest),
            &lawpack.reference.coordinate,
            hash(&lawpack.reference.digest),
        ),
        echo_operation_create_if_absent_target_profile_identity_v1(),
        profile_digest(names.authority),
        EchoOperationBudgetV1::new(16, 64, 320),
        EchoOperationProgramV1::anchored_node_attachment_create_if_absent(
            TypeId(profile_digest(names.node_type)),
            TypeId(profile_digest(names.attachment_type)),
            256,
        ),
    )
}

fn semantic_input(role: &str, kind: SemanticInputKind, artifact: BoundArtifact) -> SemanticInput {
    SemanticInput {
        role: role.to_owned(),
        kind,
        artifact,
    }
}

fn bound(coordinate: &str, domain: &str, bytes: Vec<u8>) -> BoundArtifact {
    let value = decode_canonical_cbor_v1(&bytes).expect("fixture is canonical");
    let digest =
        digest_canonical_value_bytes_v1(domain, &value).expect("fixture identity is computable");
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: digest.to_vec(),
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

fn resource_ref(reference: &ResourceRef) -> CanonicalValueV1 {
    owned_map([
        ("id", text(&reference.coordinate)),
        (
            "digest",
            CanonicalValueV1::Array(vec![
                text("sha256"),
                CanonicalValueV1::Bytes(reference.digest.bytes.clone()),
            ]),
        ),
    ])
}

fn canonical_bytes(value: &CanonicalValueV1) -> Vec<u8> {
    encode_canonical_cbor_v1(value).expect("fixture is canonical")
}

fn owned_map<const N: usize>(entries: [(&str, CanonicalValueV1); N]) -> CanonicalValueV1 {
    dynamic_map(entries)
}

fn dynamic_map<'a>(
    entries: impl IntoIterator<Item = (&'a str, CanonicalValueV1)>,
) -> CanonicalValueV1 {
    let mut entries = entries
        .into_iter()
        .map(|(key, value)| (text(key), value))
        .collect::<Vec<_>>();
    entries.sort_by_cached_key(|(key, _)| canonical_bytes(key));
    CanonicalValueV1::Map(entries)
}

fn text(value: impl Into<String>) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.into())
}

fn integer(value: u64) -> CanonicalValueV1 {
    CanonicalValueV1::Integer(i128::from(value))
}

fn hash(digest: &Digest) -> [u8; 32] {
    digest
        .bytes
        .as_slice()
        .try_into()
        .expect("fixture digest is 32 bytes")
}

fn profile_digest(label: &str) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"echo:operation-profile:v1\0");
    hasher.update(&(label.len() as u64).to_le_bytes());
    hasher.update(label.as_bytes());
    hasher.finalize().into()
}

fn text_field<'a>(value: &'a CanonicalValueV1, field: &str) -> Option<&'a str> {
    let CanonicalValueV1::Map(entries) = value else {
        return None;
    };
    entries.iter().find_map(|(key, value)| {
        if key == &text(field) {
            let CanonicalValueV1::Text(value) = value else {
                return None;
            };
            Some(value.as_str())
        } else {
            None
        }
    })
}
