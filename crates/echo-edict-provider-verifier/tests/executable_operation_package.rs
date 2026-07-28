// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Independent generic package-verifier witnesses.

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, encode_canonical_cbor_v1,
    CanonicalValueV1,
};
use echo_edict_provider_lowerer as lowerer;
use echo_edict_provider_verifier as verifier;

const TARGET_PROFILE: &[u8] = include_bytes!("../resources/target-profile.echo-dpo.cbor");
const PACKAGE_ROLE: &str = "executable-operation-package.echo";
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const REPORT_ROLE: &str = "verifier-report.echo-operation";
const REPORT_DOMAIN: &str = "echo.operation-package-verifier-report/v1";
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

const FIXTURES: [FixtureNames<'static>; 2] = [
    FixtureNames {
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
    },
    FixtureNames {
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
    },
];

struct RawFixture {
    core: Vec<u8>,
    target_profile: Vec<u8>,
    adapter: Vec<u8>,
    exports: Vec<u8>,
    lawpack: Vec<u8>,
    source: Vec<u8>,
    configuration: Vec<u8>,
    target_ir: Vec<u8>,
}

#[test]
fn verifier_accepts_generic_lowerer_output_for_two_application_vocabularies() {
    for names in FIXTURES {
        let fixture = raw_fixture(names);
        let package = lower_package(names, &fixture);
        let package_value =
            decode_canonical_cbor_v1(&package).expect("the lowered package is canonical");
        assert_eq!(
            text_field(&package_value, "obstruction_coordinate"),
            Some(names.obstruction)
        );

        let verified = verifier::verify(verification_request(names, &fixture, package))
            .expect("the independent verifier completes");

        assert!(verified.diagnostics.is_empty());
        let [report] = verified.outputs.as_slice() else {
            panic!("verification emits exactly one report");
        };
        assert_eq!(report.role, REPORT_ROLE);
        assert_eq!(
            report.kind,
            verifier::VerificationOutputKind::VerifierReport
        );
        assert_eq!(report.artifact.domain, REPORT_DOMAIN);
        let report = decode_canonical_cbor_v1(&report.artifact.bytes)
            .expect("the relation report is canonical");
        assert_eq!(text_field(&report, "outcome"), Some("accepted"));
    }
}

#[test]
fn verifier_rejects_a_canonical_package_with_a_rebound_operation_coordinate() {
    let names = FIXTURES[0];
    let fixture = raw_fixture(names);
    let package = lower_package(names, &fixture);
    let mut package_value =
        decode_canonical_cbor_v1(&package).expect("the lowered package is canonical");
    *map_field_mut(&mut package_value, "operation_coordinate") =
        text("forged.application@1.createAnything");
    let package = encode_canonical_cbor_v1(&package_value).expect("the mutation remains canonical");

    let verified = verifier::verify(verification_request(names, &fixture, package))
        .expect("semantic mismatch is a completed verification");

    assert_eq!(verified.diagnostics.len(), 1);
    assert_eq!(
        verified.diagnostics[0].code,
        "echo.verifier.executable-operation-package-mismatch"
    );
    let report = decode_canonical_cbor_v1(&verified.outputs[0].artifact.bytes)
        .expect("the rejection report is canonical");
    assert_eq!(text_field(&report, "outcome"), Some("rejected"));
}

#[test]
fn verifier_rejects_a_canonical_package_with_a_rebound_obstruction_coordinate() {
    let names = FIXTURES[0];
    let fixture = raw_fixture(names);
    let package = lower_package(names, &fixture);
    let mut package_value =
        decode_canonical_cbor_v1(&package).expect("the lowered package is canonical");
    *map_field_mut(&mut package_value, "obstruction_coordinate") =
        text("forged.domain@9.NotTheAuthoredObstruction");
    let package = encode_canonical_cbor_v1(&package_value).expect("the mutation remains canonical");

    let verified = verifier::verify(verification_request(names, &fixture, package))
        .expect("semantic mismatch is a completed verification");

    assert_eq!(verified.diagnostics.len(), 1);
    assert_eq!(
        verified.diagnostics[0].code,
        "echo.verifier.executable-operation-package-mismatch"
    );
    let report = decode_canonical_cbor_v1(&verified.outputs[0].artifact.bytes)
        .expect("the rejection report is canonical");
    assert_eq!(text_field(&report, "outcome"), Some("rejected"));
}

fn lower_package(names: FixtureNames<'_>, fixture: &RawFixture) -> Vec<u8> {
    let success =
        lowerer::lower(lowering_request(names, fixture)).expect("the generic lowerer completes");
    let [package] = success.outputs.as_slice() else {
        panic!("lowering emits exactly one package");
    };
    package.artifact.bytes.clone()
}

fn lowering_request(names: FixtureNames<'_>, fixture: &RawFixture) -> lowerer::LoweringRequestV1 {
    lowerer::LoweringRequestV1 {
        protocol_version: lowerer::ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: lowerer_bound(names.application, "edict.core.module/v1", &fixture.core),
        target_profile: lowerer_bound(
            "echo.dpo@1",
            "edict.target-profile/v1",
            &fixture.target_profile,
        ),
        semantic_inputs: vec![
            lowerer_input(
                lowerer::SemanticInputKind::Auxiliary("lawpack-adapter".to_owned()),
                lowerer_bound(names.adapter, "edict.lawpack-adapter/v1", &fixture.adapter),
            ),
            lowerer_input(
                lowerer::SemanticInputKind::Auxiliary("lawpack-exports".to_owned()),
                lowerer_bound(names.exports, "edict.lawpack-exports/v1", &fixture.exports),
            ),
            lowerer_input(
                lowerer::SemanticInputKind::Lawpack,
                lowerer_bound(names.lawpack, "edict.lawpack/v1", &fixture.lawpack),
            ),
            lowerer_input(
                lowerer::SemanticInputKind::Auxiliary("edict-source".to_owned()),
                lowerer_bound(names.application, "edict.source/v1", &fixture.source),
            ),
            lowerer_input(
                lowerer::SemanticInputKind::Auxiliary("target-configuration".to_owned()),
                lowerer_bound(
                    names.configuration,
                    "echo.operation-lowering-configuration/v1",
                    &fixture.configuration,
                ),
            ),
            lowerer_input(
                lowerer::SemanticInputKind::Auxiliary("target-ir".to_owned()),
                lowerer_bound(
                    "echo.span-ir/v1",
                    "edict.target-ir.artifact/v1",
                    &fixture.target_ir,
                ),
            ),
        ],
        requested_outputs: vec![lowerer::LoweringOutputRequest {
            role: PACKAGE_ROLE.to_owned(),
            kind: lowerer::LoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE_DOMAIN.to_owned(),
        }],
        limits: lowerer::ResponseLimitsV1 {
            max_output_count: 1,
            max_diagnostic_count: 0,
            max_total_response_bytes: 16 * 1024,
        },
    }
}

fn verification_request(
    names: FixtureNames<'_>,
    fixture: &RawFixture,
    package: Vec<u8>,
) -> verifier::VerificationRequestV1 {
    verifier::VerificationRequestV1 {
        protocol_version: verifier::ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: verifier_bound(names.application, "edict.core.module/v1", &fixture.core),
        target_profile: verifier_bound(
            "echo.dpo@1",
            "edict.target-profile/v1",
            &fixture.target_profile,
        ),
        target_ir: verifier_bound(
            "echo.span-ir/v1",
            "edict.target-ir.artifact/v1",
            &fixture.target_ir,
        ),
        semantic_inputs: vec![
            verifier_input(
                verifier::SemanticInputKind::Auxiliary("lawpack-adapter".to_owned()),
                verifier_bound(names.adapter, "edict.lawpack-adapter/v1", &fixture.adapter),
            ),
            verifier_input(
                verifier::SemanticInputKind::Auxiliary("executable-operation-package".to_owned()),
                verifier_bound(PACKAGE_ROLE, PACKAGE_DOMAIN, &package),
            ),
            verifier_input(
                verifier::SemanticInputKind::Auxiliary("lawpack-exports".to_owned()),
                verifier_bound(names.exports, "edict.lawpack-exports/v1", &fixture.exports),
            ),
            verifier_input(
                verifier::SemanticInputKind::Lawpack,
                verifier_bound(names.lawpack, "edict.lawpack/v1", &fixture.lawpack),
            ),
            verifier_input(
                verifier::SemanticInputKind::Auxiliary("edict-source".to_owned()),
                verifier_bound(names.application, "edict.source/v1", &fixture.source),
            ),
            verifier_input(
                verifier::SemanticInputKind::Auxiliary("target-configuration".to_owned()),
                verifier_bound(
                    names.configuration,
                    "echo.operation-lowering-configuration/v1",
                    &fixture.configuration,
                ),
            ),
        ],
        requested_outputs: vec![verifier::VerificationOutputRequest {
            role: REPORT_ROLE.to_owned(),
            kind: verifier::VerificationOutputKind::VerifierReport,
            domain: REPORT_DOMAIN.to_owned(),
        }],
        limits: verifier::ResponseLimitsV1 {
            max_output_count: 1,
            max_diagnostic_count: 8,
            max_total_response_bytes: 16 * 1024,
        },
    }
}

fn raw_fixture(names: FixtureNames<'_>) -> RawFixture {
    let target_profile = TARGET_PROFILE.to_vec();
    let target_profile_ref = raw_ref("echo.dpo@1", "edict.target-profile/v1", &target_profile);
    let exports = canonical_bytes(&exports(names));
    let exports_ref = raw_ref(names.exports, "edict.lawpack-exports/v1", &exports);
    let configuration = canonical_bytes(&configuration(names));
    let configuration_ref = raw_ref(
        names.configuration,
        "echo.operation-lowering-configuration/v1",
        &configuration,
    );
    let adapter = canonical_bytes(&adapter(names, &configuration_ref));
    let adapter_ref = raw_ref(names.adapter, "edict.lawpack-adapter/v1", &adapter);
    let lawpack = canonical_bytes(&lawpack(
        names,
        &exports_ref,
        &exports,
        &adapter_ref,
        &adapter,
        &target_profile_ref,
    ));
    let lawpack_ref = raw_ref(names.lawpack, "edict.lawpack/v1", &lawpack);
    let core = canonical_bytes(&core(names));
    let core_ref = raw_ref(names.application, "edict.core.module/v1", &core);
    let source = canonical_bytes(&CanonicalValueV1::Bytes(
        format!(
            "package {};\n\nuse lawpack {} digest \"{}\" as {};\n\nintent {};",
            names.application,
            names.lawpack,
            digest_review(&lawpack_ref.digest),
            names.alias,
            names.intent
        )
        .into_bytes(),
    ));
    let target_ir = canonical_bytes(&target_ir(
        names,
        &core_ref,
        &lawpack_ref,
        &target_profile_ref,
    ));
    RawFixture {
        core,
        target_profile,
        adapter,
        exports,
        lawpack,
        source,
        configuration,
        target_ir,
    }
}

#[derive(Clone)]
struct RawRef {
    coordinate: String,
    digest: Vec<u8>,
}

fn raw_ref(coordinate: &str, domain: &str, bytes: &[u8]) -> RawRef {
    let value = decode_canonical_cbor_v1(bytes).expect("fixture is canonical");
    RawRef {
        coordinate: coordinate.to_owned(),
        digest: digest_canonical_value_bytes_v1(domain, &value)
            .expect("fixture identity is computable")
            .to_vec(),
    }
}

fn core(names: FixtureNames<'_>) -> CanonicalValueV1 {
    map([
        ("apiVersion", text("edict.core/v1")),
        ("coordinate", text(names.application)),
        (
            "intents",
            dynamic_map([(
                names.intent,
                map([
                    (
                        "requiredOperationProfile",
                        text("continuum.profile.create/v1"),
                    ),
                    (
                        "body",
                        map([(
                            "nodes",
                            CanonicalValueV1::Array(vec![map([
                                ("kind", text("effect")),
                                (
                                    "effect",
                                    text(format!("{}.{}", names.alias, names.effect_member)),
                                ),
                                (
                                    "obstructionMap",
                                    dynamic_map([(
                                        names.failure,
                                        map([("callee", text(names.obstruction))]),
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
    map([(
        "effects",
        CanonicalValueV1::Array(vec![map([
            ("coordinate", text(names.effect)),
            ("executionClass", text("runtime")),
            (
                "effectFailures",
                dynamic_map([(
                    names.failure,
                    map([("authorityClass", text("domainMappable"))]),
                )]),
            ),
        ])]),
    )])
}

fn configuration(names: FixtureNames<'_>) -> CanonicalValueV1 {
    map([
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
            map([
                ("steps", integer(16)),
                ("readBytes", integer(64)),
                ("writeBytes", integer(320)),
            ]),
        ),
        (
            "invocationBinding",
            map([
                ("nodeKeyField", text("key")),
                ("replacementField", text("value")),
                ("nodeIdDerivation", text("sha256-utf8/v1")),
                ("warpIdSource", text("action-lane/v1")),
            ]),
        ),
    ])
}

fn adapter(names: FixtureNames<'_>, configuration: &RawRef) -> CanonicalValueV1 {
    map([
        ("apiVersion", text("edict.lawpack-adapter/v1")),
        ("class", text("declarative")),
        (
            "effectImplementations",
            dynamic_map([(
                names.effect,
                map([
                    ("targetIntrinsic", text(TARGET_INTRINSIC)),
                    ("targetConfiguration", resource_ref(configuration)),
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
    exports: &RawRef,
    exports_bytes: &[u8],
    adapter: &RawRef,
    adapter_bytes: &[u8],
    target_profile: &RawRef,
) -> CanonicalValueV1 {
    map([
        ("apiVersion", text("edict.lawpack/v1")),
        ("id", text(names.lawpack_id)),
        ("version", text(names.lawpack_version)),
        ("exports", owner_resource_ref(exports, exports_bytes)),
        (
            "targetAdapters",
            CanonicalValueV1::Array(vec![map([
                ("adapter", owner_resource_ref(adapter, adapter_bytes)),
                ("acceptedTargetProfile", resource_ref(target_profile)),
            ])]),
        ),
    ])
}

fn owner_resource_ref(reference: &RawRef, bytes: &[u8]) -> CanonicalValueV1 {
    let value = decode_canonical_cbor_v1(bytes).expect("owner artifact is canonical");
    let digest = digest_canonical_value_bytes_v1(&reference.coordinate, &value)
        .expect("owner-framed digest is computable");
    map([
        ("id", text(&reference.coordinate)),
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
    core: &RawRef,
    lawpack: &RawRef,
    target_profile: &RawRef,
) -> CanonicalValueV1 {
    map([
        ("kind", text("targetIrArtifact")),
        ("domain", text("echo.span-ir/v1")),
        ("targetProfile", resource_ref(target_profile)),
        ("sourceCoreCoordinate", text(names.application)),
        (
            "semanticClosure",
            map([
                ("sourceCore", resource_ref(core)),
                (
                    "lawpacks",
                    CanonicalValueV1::Array(vec![resource_ref(lawpack)]),
                ),
            ]),
        ),
        (
            "intents",
            dynamic_map([(
                names.intent,
                map([
                    ("operationProfile", text("continuum.profile.create/v1")),
                    (
                        "steps",
                        CanonicalValueV1::Array(vec![map([
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

fn lowerer_input(
    kind: lowerer::SemanticInputKind,
    artifact: lowerer::BoundArtifact,
) -> lowerer::SemanticInput {
    lowerer::SemanticInput {
        role: "semantic-input".to_owned(),
        kind,
        artifact,
    }
}

fn verifier_input(
    kind: verifier::SemanticInputKind,
    artifact: verifier::BoundArtifact,
) -> verifier::SemanticInput {
    verifier::SemanticInput {
        role: "semantic-input".to_owned(),
        kind,
        artifact,
    }
}

fn lowerer_bound(coordinate: &str, domain: &str, bytes: &[u8]) -> lowerer::BoundArtifact {
    let reference = raw_ref(coordinate, domain, bytes);
    lowerer::BoundArtifact {
        reference: lowerer::ResourceRef {
            coordinate: reference.coordinate,
            digest: lowerer::Digest {
                algorithm: lowerer::DigestAlgorithm::Sha256,
                bytes: reference.digest,
            },
        },
        artifact: lowerer::Artifact {
            domain: domain.to_owned(),
            bytes: bytes.to_vec(),
        },
    }
}

fn verifier_bound(coordinate: &str, domain: &str, bytes: &[u8]) -> verifier::BoundArtifact {
    let reference = raw_ref(coordinate, domain, bytes);
    verifier::BoundArtifact {
        reference: verifier::ResourceRef {
            coordinate: reference.coordinate,
            digest: verifier::Digest {
                algorithm: verifier::DigestAlgorithm::Sha256,
                bytes: reference.digest,
            },
        },
        artifact: verifier::Artifact {
            domain: domain.to_owned(),
            bytes: bytes.to_vec(),
        },
    }
}

fn resource_ref(reference: &RawRef) -> CanonicalValueV1 {
    map([
        ("id", text(&reference.coordinate)),
        (
            "digest",
            CanonicalValueV1::Array(vec![
                text("sha256"),
                CanonicalValueV1::Bytes(reference.digest.clone()),
            ]),
        ),
    ])
}

fn canonical_bytes(value: &CanonicalValueV1) -> Vec<u8> {
    encode_canonical_cbor_v1(value).expect("fixture is canonical")
}

fn map<const N: usize>(entries: [(&str, CanonicalValueV1); N]) -> CanonicalValueV1 {
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

fn map_field_mut<'a>(value: &'a mut CanonicalValueV1, field: &str) -> &'a mut CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("expected map");
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .unwrap_or_else(|| panic!("missing field {field}"))
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
