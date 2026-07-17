// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Executable contract for the first pure Echo Edict provider lowerer.

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1, CanonicalValueV1,
};
use echo_edict_provider_lowerer::{
    lower, Artifact, BoundArtifact, Digest, DigestAlgorithm, LoweringOutputArtifact,
    LoweringOutputKind, LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1,
    ProviderRefusalKind, ResourceRef, ResponseLimitsV1, SemanticInput, SemanticInputKind,
};
use sha2::{Digest as ShaDigest, Sha256};
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const TARGET_PROFILE: &[u8] = include_bytes!("../resources/target-profile.echo-dpo.cbor");
const LAWPACK: &[u8] = include_bytes!("../resources/lawpack.echo-dpo.cbor");
const TARGET_AUTHORITY: &[u8] = include_bytes!("../resources/authority-facts.echo-dpo.cbor");
const LAWPACK_AUTHORITY: &[u8] = include_bytes!("../resources/authority-facts.echo-lawpack.cbor");

const CORE_DOMAIN: &str = "edict.core.module/v1";
const TARGET_PROFILE_DOMAIN: &str = "edict.target-profile/v1";
const LAWPACK_DOMAIN: &str = "edict.lawpack/v1";
const AUTHORITY_DOMAIN: &str = "edict.authority-facts/v1";
const LOWERABILITY_DOMAIN: &str = "edict.lowering-requirements/v1";
const OUTER_TARGET_IR_DOMAIN: &str = "edict.target-ir.artifact/v1";
const INNER_TARGET_IR_DOMAIN: &str = "echo.span-ir/v1";
const GENERATED_ARTIFACT_DOMAIN: &str = "echo.generated-artifact/v1";
const REVIEW_PAYLOAD_DOMAIN: &str = "echo.review-payload/v1";
const GENERATED_ARTIFACT_ROLE: &str = "generated.echo-dpo";
const REVIEW_PAYLOAD_ROLE: &str = "review.echo-dpo";
const TARGET_IR_ROLE: &str = "target-ir.echo-dpo";
const GENERATED_ARTIFACT_PROFILE: &str = "echo.dpo.registration/v1";
const GENERATED_ARTIFACT_PROFILE_DIGEST: &str =
    "sha256:7b2d8216222e95dbcc9310f7aac924938545665aded578e060af13cbd79d7ac9";
const TARGET_BUNDLE_PROFILE: &str = "echo.dpo.bundle/v1";
const TARGET_BUNDLE_PROFILE_DIGEST: &str =
    "sha256:aa0438bcc6ef14ee6cb6d4976622f6080381d731459dcb7b9102595c9bed92c0";
const GENERATED_SOURCE_MEDIA_TYPE: &str = "text/rust; charset=utf-8";
const REVIEW_MEDIA_TYPE: &str = "application/json";
const GENERATED_SOURCE_PATH: &str = "generated/echo_dpo.rs";
const REVIEW_PATH: &str = "review/echo_dpo.json";
const EXPECTED_PROVIDER_SCHEMA_SHA256_HEX: &str =
    "dcf2cc739bb855cb4c9578c2fbc35f0c99b58f8c83d5f290c51498dd658c8232";
const EXPECTED_OPERATION_ID_LAW: &str = "echo.semantic-operation-id.fnv1-32/v1";
const EXPECTED_OPERATION_ID: u32 = 3_389_142_194;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

const EDICT_ORACLE_CORE_HEX: &str = concat!(
    "a6657479706573a665496e707574a2646b696e64665265636f7264666669656c6473a16269646e612e6240312e496e70",
    "75742e6964664f7574707574a2646b696e64665265636f7264666669656c6473a16269646f612e6240312e4f75747075",
    "742e69646752656365697074a2646b696e64665265636f7264666669656c6473a162696470612e6240312e5265636569",
    "70742e696468496e7075742e6964a3636d617810646b696e6466537472696e676963616e6f6e6963616c687261772d75",
    "746638694f75747075742e6964a3636d617810646b696e6466537472696e676963616e6f6e6963616c687261772d7574",
    "66386a526563656970742e6964a3636d617810646b696e6466537472696e676963616e6f6e6963616c687261772d7574",
    "663867696d706f7274738067696e74656e7473a16174a664626f6479a3656e6f64657381a5646b696e64666566666563",
    "7465696e707574a36462617365a263726566a3626964656172672e3064747970656b612e6240312e496e70757469616c",
    "7068614e616d65652461726730646b696e64656c6f63616c646b696e64656669656c64656669656c6462696466656666",
    "6563746e7461726765742e7265706c6163656762696e64696e67a3626964676c6f63616c2e3064747970656d612e6240",
    "312e5265636569707469616c7068614e616d6567246c6f63616c306e6f62737472756374696f6e4d6170a16872656a65",
    "63746564a26576616c7565a4646172677380646b696e646463616c6c6663616c6c656574646f6d61696e2e5772697465",
    "52656a6563746564687479706541726773806662696e646572a36269646d6f62737472756374696f6e2e306474797065",
    "777461726765742e7265706c6163652e72656a656374656469616c7068614e616d656d246f62737472756374696f6e30",
    "666c6f63616c7383a3626964656172672e3064747970656b612e6240312e496e70757469616c7068614e616d65652461",
    "726730a3626964676c6f63616c2e3064747970656d612e6240312e5265636569707469616c7068614e616d6567246c6f",
    "63616c30a36269646d6f62737472756374696f6e2e306474797065777461726765742e7265706c6163652e72656a6563",
    "74656469616c7068614e616d656d246f62737472756374696f6e3066726573756c74a2646b696e64667265636f726466",
    "6669656c6473a1626964a36462617365a263726566a3626964656172672e3064747970656b612e6240312e496e707574",
    "69616c7068614e616d65652461726730646b696e64656c6f63616c646b696e64656669656c64656669656c6462696465",
    "696e7075746b612e6240312e496e707574666f75747075746c612e6240312e4f757470757470696e707574436f6e7374",
    "7261696e74738074636f72654576616c756174696f6e427564676574a3686d61785374657073086e6d61784f75747075",
    "744279746573190100716d6178416c6c6f63617465644279746573190400781872657175697265644f7065726174696f",
    "6e50726f66696c65781a636f6e74696e75756d2e70726f66696c652e77726974652f76316a61706956657273696f6e6d",
    "65646963742e636f72652f76316a636f6f7264696e61746565612e62403178187265717569726564436f726543617061",
    "62696c697469657380",
);

const EDICT_ORACLE_TARGET_IR_HEX: &str = concat!(
    "a5646b696e64707461726765744972417274696661637466646f6d61696e6f6563686f2e7370616e2d69722f76316769",
    "6e74656e7473a16174a665737465707381a762696468742e737465702e3065696e707574a36462617365a263726566a3",
    "626964656172672e3064747970656b612e6240312e496e70757469616c7068614e616d65652461726730646b696e6465",
    "6c6f63616c646b696e64656669656c64656669656c64626964666566666563746e7461726765742e7265706c61636567",
    "62696e64696e67a3626964676c6f63616c2e3064747970656d612e6240312e5265636569707469616c7068614e616d65",
    "67246c6f63616c306f6f62737472756374696f6e41726d73a16872656a6563746564a26576616c7565a4646172677380",
    "646b696e646463616c6c6663616c6c656574646f6d61696e2e577269746552656a656374656468747970654172677380",
    "6662696e646572a36269646d6f62737472756374696f6e2e306474797065777461726765742e7265706c6163652e7265",
    "6a656374656469616c7068614e616d656d246f62737472756374696f6e306f746172676574496e7472696e7369637265",
    "63686f2e64706f40312e7265706c616365736f62737472756374696f6e4661696c75726573816872656a656374656466",
    "726573756c74a2646b696e64667265636f7264666669656c6473a1626964a36462617365a263726566a3626964656172",
    "672e3064747970656b612e6240312e496e70757469616c7068614e616d65652461726730646b696e64656c6f63616c64",
    "6b696e64656669656c64656669656c646269646c726571756972656d656e74738070696e707574436f6e73747261696e",
    "747380706f7065726174696f6e50726f66696c65781a636f6e74696e75756d2e70726f66696c652e77726974652f7631",
    "74636f72654576616c756174696f6e427564676574a3686d61785374657073086e6d61784f7574707574427974657319",
    "0100716d6178416c6c6f636174656442797465731904006d74617267657450726f66696c65a26269646a6563686f2e64",
    "706f40316664696765737482667368613235365820f41df38156625a05c1ee8bce652ffddf04e71b54fe027eeab9d255",
    "d0d8322db074736f75726365436f7265436f6f7264696e61746565612e624031",
);

fn text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn integer(value: u64) -> CanonicalValueV1 {
    CanonicalValueV1::Integer(i128::from(value))
}

fn map(entries: impl IntoIterator<Item = (&'static str, CanonicalValueV1)>) -> CanonicalValueV1 {
    let mut entries = entries
        .into_iter()
        .map(|(key, value)| (text(key), value))
        .collect::<Vec<_>>();
    entries.sort_by_cached_key(|(key, _)| canonical_bytes(key));
    CanonicalValueV1::Map(entries)
}

fn string_map(
    entries: impl IntoIterator<Item = (&'static str, CanonicalValueV1)>,
) -> CanonicalValueV1 {
    map(entries)
}

fn local(id: &str, alpha_name: &str, ty: &str) -> CanonicalValueV1 {
    map([
        ("id", text(id)),
        ("alphaName", text(alpha_name)),
        ("type", text(ty)),
    ])
}

fn local_expr(reference: CanonicalValueV1) -> CanonicalValueV1 {
    map([("kind", text("local")), ("ref", reference)])
}

fn field_expr(base: CanonicalValueV1, field: &str) -> CanonicalValueV1 {
    map([
        ("kind", text("field")),
        ("base", base),
        ("field", text(field)),
    ])
}

fn record_expr(id: CanonicalValueV1) -> CanonicalValueV1 {
    map([
        ("kind", text("record")),
        ("fields", string_map([("id", id)])),
    ])
}

fn call_expr(callee: &str) -> CanonicalValueV1 {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        ("typeArgs", CanonicalValueV1::Array(Vec::new())),
        ("args", CanonicalValueV1::Array(Vec::new())),
    ])
}

fn canonical_bytes(value: &CanonicalValueV1) -> Vec<u8> {
    encode_canonical_cbor_v1(value).expect("test value has a canonical encoding")
}

fn bound(coordinate: &str, domain: &str, bytes: impl Into<Vec<u8>>) -> BoundArtifact {
    let bytes = bytes.into();
    let value = decode_canonical_cbor_v1(&bytes).expect("fixture is canonical CBOR");
    let review_digest =
        digest_canonical_value_v1(domain, &value).expect("fixture has a domain-framed digest");
    let digest_bytes = hex::decode(
        review_digest
            .strip_prefix("sha256:")
            .expect("review digest uses sha256"),
    )
    .expect("review digest is hexadecimal");
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: digest_bytes,
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

fn core_types() -> CanonicalValueV1 {
    string_map([
        (
            "Input",
            map([
                ("kind", text("Record")),
                ("fields", string_map([("id", text("a.b@1.Input.id"))])),
            ]),
        ),
        (
            "Output",
            map([
                ("kind", text("Record")),
                ("fields", string_map([("id", text("a.b@1.Output.id"))])),
            ]),
        ),
        (
            "Receipt",
            map([
                ("kind", text("Record")),
                ("fields", string_map([("id", text("a.b@1.Receipt.id"))])),
            ]),
        ),
        (
            "Input.id",
            map([
                ("kind", text("String")),
                ("max", integer(16)),
                ("canonical", text("raw-utf8")),
            ]),
        ),
        (
            "Output.id",
            map([
                ("kind", text("String")),
                ("max", integer(16)),
                ("canonical", text("raw-utf8")),
            ]),
        ),
        (
            "Receipt.id",
            map([
                ("kind", text("String")),
                ("max", integer(16)),
                ("canonical", text("raw-utf8")),
            ]),
        ),
    ])
}

fn core_value(result: CanonicalValueV1, effect: Option<&str>) -> CanonicalValueV1 {
    let input = local("local:0", "input", "a.b@1.Input");
    let receipt = local("local:1", "receipt", "a.b@1.Receipt");
    let reason = local("local:2", "reason", "target.replace.rejected");
    let nodes = effect.map_or_else(Vec::new, |effect| {
        vec![map([
            ("kind", text("effect")),
            ("binding", receipt.clone()),
            ("effect", text(effect)),
            ("input", field_expr(local_expr(input.clone()), "id")),
            (
                "obstructionMap",
                string_map([(
                    "rejected",
                    map([
                        ("binder", reason.clone()),
                        ("value", call_expr("domain.WriteRejected")),
                    ]),
                )]),
            ),
        ])]
    });
    let intent = map([
        ("input", text("a.b@1.Input")),
        ("output", text("a.b@1.Output")),
        (
            "requiredOperationProfile",
            text("continuum.profile.write/v1"),
        ),
        ("inputConstraints", CanonicalValueV1::Array(Vec::new())),
        (
            "coreEvaluationBudget",
            map([
                ("maxSteps", integer(8)),
                ("maxAllocatedBytes", integer(1024)),
                ("maxOutputBytes", integer(256)),
            ]),
        ),
        (
            "body",
            map([
                (
                    "locals",
                    CanonicalValueV1::Array(vec![input, receipt, reason]),
                ),
                ("nodes", CanonicalValueV1::Array(nodes)),
                ("result", result),
            ]),
        ),
    ]);
    map([
        ("apiVersion", text("edict.core/v1")),
        ("coordinate", text("a.b@1")),
        ("imports", CanonicalValueV1::Array(Vec::new())),
        ("types", core_types()),
        ("intents", string_map([("t", intent)])),
        (
            "requiredCoreCapabilities",
            CanonicalValueV1::Array(Vec::new()),
        ),
    ])
}

fn ordinary_result() -> CanonicalValueV1 {
    record_expr(field_expr(
        local_expr(local("local:0", "input", "a.b@1.Input")),
        "id",
    ))
}

fn lowerability_value() -> CanonicalValueV1 {
    map([
        ("apiVersion", text(LOWERABILITY_DOMAIN)),
        ("operationProfile", text("continuum.profile.write/v1")),
        (
            "semanticEffects",
            CanonicalValueV1::Array(vec![map([
                ("coordinate", text("target.replace")),
                ("writeClass", text("replace")),
                (
                    "guardKinds",
                    CanonicalValueV1::Array(vec![text("precommit-atomic")]),
                ),
                (
                    "obstructionCoordinates",
                    CanonicalValueV1::Array(vec![text("rejected")]),
                ),
                (
                    "footprintObligations",
                    CanonicalValueV1::Array(vec![text("target.replace.footprint")]),
                ),
                (
                    "costObligations",
                    CanonicalValueV1::Array(vec![text("target.replace.cost")]),
                ),
            ])]),
        ),
        (
            "requiredWriteClasses",
            CanonicalValueV1::Array(vec![text("replace")]),
        ),
        (
            "guardKinds",
            CanonicalValueV1::Array(vec![text("precommit-atomic")]),
        ),
        ("atomicity", text("atomic")),
        ("postconditionSupport", CanonicalValueV1::Bool(true)),
        (
            "obstructionCoordinates",
            CanonicalValueV1::Array(vec![text("rejected")]),
        ),
        (
            "footprintObligations",
            CanonicalValueV1::Array(vec![text("target.replace.footprint")]),
        ),
        (
            "costObligations",
            CanonicalValueV1::Array(vec![text("target.replace.cost")]),
        ),
        ("opticContract", text("replace-point")),
    ])
}

fn request_with_core(core: CanonicalValueV1) -> LoweringRequestV1 {
    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: bound("a.b@1", CORE_DOMAIN, canonical_bytes(&core)),
        target_profile: bound("echo.dpo@1", TARGET_PROFILE_DOMAIN, TARGET_PROFILE),
        semantic_inputs: vec![
            SemanticInput {
                role: "authority-facts.echo-dpo".to_owned(),
                kind: SemanticInputKind::AuthorityFacts,
                artifact: bound(
                    "echo.dpo-authority-facts@1",
                    AUTHORITY_DOMAIN,
                    TARGET_AUTHORITY,
                ),
            },
            SemanticInput {
                role: "authority-facts.echo-lawpack".to_owned(),
                kind: SemanticInputKind::AuthorityFacts,
                artifact: bound(
                    "echo.dpo-lawpack-authority-facts@1",
                    AUTHORITY_DOMAIN,
                    LAWPACK_AUTHORITY,
                ),
            },
            SemanticInput {
                role: "lawpack.echo-dpo".to_owned(),
                kind: SemanticInputKind::Lawpack,
                artifact: bound("echo.dpo-lawpack@1", LAWPACK_DOMAIN, LAWPACK),
            },
            SemanticInput {
                role: "lowerability.echo-dpo".to_owned(),
                kind: SemanticInputKind::LowerabilityFacts,
                artifact: bound(
                    "echo.dpo-lowerability@1",
                    LOWERABILITY_DOMAIN,
                    canonical_bytes(&lowerability_value()),
                ),
            },
        ],
        requested_outputs: vec![LoweringOutputRequest {
            role: TARGET_IR_ROLE.to_owned(),
            kind: LoweringOutputKind::TargetIr,
            domain: OUTER_TARGET_IR_DOMAIN.to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 8,
            max_diagnostic_count: 8,
            max_total_response_bytes: 64 * 1024,
        },
    }
}

fn request() -> LoweringRequestV1 {
    request_with_core(core_value(ordinary_result(), Some("target.replace")))
}

fn output_request(role: &str, kind: LoweringOutputKind, domain: &str) -> LoweringOutputRequest {
    LoweringOutputRequest {
        role: role.to_owned(),
        kind,
        domain: domain.to_owned(),
    }
}

fn generated_artifact_request() -> LoweringOutputRequest {
    output_request(
        GENERATED_ARTIFACT_ROLE,
        LoweringOutputKind::GeneratedArtifact,
        GENERATED_ARTIFACT_DOMAIN,
    )
}

fn review_payload_request() -> LoweringOutputRequest {
    output_request(
        REVIEW_PAYLOAD_ROLE,
        LoweringOutputKind::ReviewPayload,
        REVIEW_PAYLOAD_DOMAIN,
    )
}

fn target_ir_request() -> LoweringOutputRequest {
    output_request(
        TARGET_IR_ROLE,
        LoweringOutputKind::TargetIr,
        OUTER_TARGET_IR_DOMAIN,
    )
}

fn assert_exact_outputs(
    outputs: &[LoweringOutputArtifact],
    expected: &[(&str, LoweringOutputKind, &str)],
) {
    assert_eq!(outputs.len(), expected.len());
    for (output, (role, kind, domain)) in outputs.iter().zip(expected) {
        assert_eq!(output.role, *role);
        assert_eq!(output.kind, *kind);
        assert_eq!(output.artifact.domain, *domain);
    }
}

fn output_by_role<'a>(
    outputs: &'a [LoweringOutputArtifact],
    role: &str,
) -> &'a LoweringOutputArtifact {
    outputs
        .iter()
        .find(|output| output.role == role)
        .unwrap_or_else(|| panic!("output role `{role}` is absent"))
}

fn output_envelope(output: &LoweringOutputArtifact) -> CanonicalValueV1 {
    decode_canonical_cbor_v1(&output.artifact.bytes)
        .unwrap_or_else(|_| panic!("output role `{}` is not canonical CBOR", output.role))
}

fn bytes_field<'a>(value: &'a CanonicalValueV1, field: &str) -> &'a [u8] {
    let CanonicalValueV1::Bytes(bytes) = map_field(value, field) else {
        panic!("map field `{field}` is not bytes");
    };
    bytes
}

fn assert_sha256_resource_ref(value: &CanonicalValueV1, id: &str, digest: &str) {
    assert_eq!(text_value(map_field(value, "id")), id);
    let digest_bytes = hex::decode(
        digest
            .strip_prefix("sha256:")
            .expect("expected review digest uses sha256"),
    )
    .expect("expected review digest is hexadecimal");
    assert_eq!(
        map_field(value, "digest"),
        &CanonicalValueV1::Array(vec![text("sha256"), CanonicalValueV1::Bytes(digest_bytes),])
    );
}

fn generated_source() -> String {
    let mut request = request();
    request.requested_outputs = vec![generated_artifact_request()];
    let success = lower(request).expect("generated source lowers");
    let envelope = output_envelope(output_by_role(&success.outputs, GENERATED_ARTIFACT_ROLE));
    String::from_utf8(bytes_field(&envelope, "bytes").to_vec()).expect("generated source is UTF-8")
}

fn compile_and_run_generated_consumer(source: &str) -> std::process::Output {
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "echo-edict-provider-lowerer-binding-proof-{}-{}-{}",
        std::process::id(),
        nanos,
        unique
    ));
    create_dir_all(&dir).expect("failed to create generated-consumer temp dir");

    let src_path: PathBuf = dir.join("proof.rs");
    let out_path: PathBuf = dir.join("proof");
    write(&src_path, source).expect("failed to write generated-consumer source");

    let compile = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            src_path.to_str().expect("non-utf8 source path"),
            "-o",
            out_path.to_str().expect("non-utf8 output path"),
        ])
        .output()
        .expect("failed to invoke rustc");
    if !compile.status.success() {
        remove_dir_all(&dir).expect("failed to remove generated-consumer temp dir");
        panic!(
            "generated consumer failed to compile: {}",
            String::from_utf8_lossy(&compile.stderr)
        );
    }

    let execution = Command::new(&out_path)
        .output()
        .expect("failed to execute generated consumer");
    remove_dir_all(&dir).expect("failed to remove generated-consumer temp dir");
    execution
}

fn review_json() -> String {
    let mut request = request();
    request.requested_outputs = vec![review_payload_request()];
    let success = lower(request).expect("review payload lowers");
    let envelope = output_envelope(output_by_role(&success.outputs, REVIEW_PAYLOAD_ROLE));
    String::from_utf8(bytes_field(&envelope, "bytes").to_vec()).expect("review payload is UTF-8")
}

fn map_field<'a>(value: &'a CanonicalValueV1, field: &str) -> &'a CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("value is not a map");
    };
    entries
        .iter()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .unwrap_or_else(|| panic!("map field `{field}` is absent"))
}

fn text_value(value: &CanonicalValueV1) -> &str {
    let CanonicalValueV1::Text(value) = value else {
        panic!("value is not text");
    };
    value
}

#[test]
fn minimal_echo_mutation_lowers_from_explicit_semantics() {
    let request = request();
    let target_profile_digest = request.target_profile.reference.digest.clone();
    let success = lower(request).expect("supported explicit closure lowers");

    assert!(success.diagnostics.is_empty());
    assert_eq!(success.outputs.len(), 1);
    let output = &success.outputs[0];
    assert_eq!(output.role, TARGET_IR_ROLE);
    assert_eq!(output.kind, LoweringOutputKind::TargetIr);
    assert_eq!(output.artifact.domain, OUTER_TARGET_IR_DOMAIN);
    assert_eq!(output.logical_path, None);

    let artifact = decode_canonical_cbor_v1(&output.artifact.bytes)
        .expect("provider output is canonical CBOR");
    assert_eq!(text_value(map_field(&artifact, "kind")), "targetIrArtifact");
    assert_eq!(
        text_value(map_field(&artifact, "domain")),
        INNER_TARGET_IR_DOMAIN
    );
    assert_eq!(
        text_value(map_field(&artifact, "sourceCoreCoordinate")),
        "a.b@1"
    );
    let target_profile = map_field(&artifact, "targetProfile");
    assert_eq!(text_value(map_field(target_profile, "id")), "echo.dpo@1");
    assert_eq!(
        map_field(target_profile, "digest"),
        &CanonicalValueV1::Array(vec![
            text("sha256"),
            CanonicalValueV1::Bytes(target_profile_digest.bytes),
        ])
    );

    let intent = map_field(map_field(&artifact, "intents"), "t");
    assert_eq!(
        text_value(map_field(intent, "operationProfile")),
        "continuum.profile.write/v1"
    );
    assert_eq!(
        map_field(intent, "coreEvaluationBudget"),
        map_field(
            map_field(
                map_field(
                    &core_value(ordinary_result(), Some("target.replace")),
                    "intents"
                ),
                "t"
            ),
            "coreEvaluationBudget"
        )
    );
    assert_eq!(map_field(intent, "result"), &ordinary_result());

    let CanonicalValueV1::Array(steps) = map_field(intent, "steps") else {
        panic!("steps is not an array");
    };
    assert_eq!(steps.len(), 1);
    assert_eq!(text_value(map_field(&steps[0], "id")), "t.step.0");
    assert_eq!(text_value(map_field(&steps[0], "effect")), "target.replace");
    assert_eq!(
        text_value(map_field(&steps[0], "targetIntrinsic")),
        "echo.dpo@1.replace"
    );
    assert_eq!(
        map_field(&steps[0], "binding"),
        &local("local:1", "receipt", "a.b@1.Receipt")
    );
    assert_eq!(
        map_field(&steps[0], "input"),
        &field_expr(local_expr(local("local:0", "input", "a.b@1.Input")), "id")
    );
    assert_eq!(
        map_field(map_field(&steps[0], "obstructionArms"), "rejected"),
        &map([
            (
                "binder",
                local("local:2", "reason", "target.replace.rejected")
            ),
            ("value", call_expr("domain.WriteRejected")),
        ])
    );
}

#[test]
fn reviewed_edict_fixture_has_exact_builtin_wrapper_parity() {
    let core_bytes = hex::decode(EDICT_ORACLE_CORE_HEX).expect("oracle Core hex is valid");
    assert_eq!(core_bytes.len(), 1209);
    let mut request = request();
    request.core = bound("a.b@1", CORE_DOMAIN, core_bytes);
    let target_profile_digest = request.target_profile.reference.digest.bytes.clone();
    assert_eq!(
        hex::encode(&request.core.reference.digest.bytes),
        "c3dbe413c78a82f6120e64c9a04bc94e2d79505f9e4b8a65c2bc26b408d775de"
    );

    let success = lower(request).expect("reviewed Edict fixture lowers");
    assert!(success.diagnostics.is_empty());
    assert_eq!(success.outputs.len(), 1);
    let output = &success.outputs[0];
    let prior_oracle =
        hex::decode(EDICT_ORACLE_TARGET_IR_HEX).expect("oracle Target IR hex is valid");
    assert_eq!(prior_oracle.len(), 848);
    let mut expected = decode_canonical_cbor_v1(&prior_oracle)
        .expect("reviewed Edict oracle is canonical Target IR");
    *map_field_mut(map_field_mut(&mut expected, "targetProfile"), "digest") =
        CanonicalValueV1::Array(vec![
            text("sha256"),
            CanonicalValueV1::Bytes(target_profile_digest),
        ]);
    let expected =
        encode_canonical_cbor_v1(&expected).expect("rebound Edict oracle remains canonical");
    assert_ne!(prior_oracle, expected);
    assert_eq!(output.artifact.bytes, expected);

    let output_value = decode_canonical_cbor_v1(&output.artifact.bytes)
        .expect("oracle-parity output is canonical CBOR");
    assert_eq!(
        digest_canonical_value_v1(OUTER_TARGET_IR_DOMAIN, &output_value)
            .expect("oracle-parity output has a domain-framed digest"),
        "sha256:e55c4980841efaaaed510425e29b011f232d3c66e52ed05221c114d85877e341"
    );
}

#[test]
fn vendored_wit_is_the_frozen_edict_lowerer_contract() {
    let bytes = include_bytes!("../wit/edict-target-provider.wit");
    assert_eq!(bytes.len(), 7392);
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        "2971fe44def7e51d5271dfc0f04f3088aa58754cffdc847681a587605aac749e"
    );
}

#[test]
fn target_ir_only_remains_an_exact_single_output() {
    let success = lower(request()).expect("the existing Target IR-only request remains supported");
    assert_exact_outputs(
        &success.outputs,
        &[(
            TARGET_IR_ROLE,
            LoweringOutputKind::TargetIr,
            OUTER_TARGET_IR_DOMAIN,
        )],
    );
}

#[test]
fn generated_artifact_only_emits_the_exact_declared_envelope() {
    let mut request = request();
    request.requested_outputs = vec![generated_artifact_request()];

    let success = lower(request).expect("the declared generated-artifact role is supported");
    assert_exact_outputs(
        &success.outputs,
        &[(
            GENERATED_ARTIFACT_ROLE,
            LoweringOutputKind::GeneratedArtifact,
            GENERATED_ARTIFACT_DOMAIN,
        )],
    );
}

#[test]
fn review_payload_only_emits_the_exact_declared_envelope() {
    let mut request = request();
    request.requested_outputs = vec![review_payload_request()];

    let success = lower(request).expect("the declared review-payload role is supported");
    assert_exact_outputs(
        &success.outputs,
        &[(
            REVIEW_PAYLOAD_ROLE,
            LoweringOutputKind::ReviewPayload,
            REVIEW_PAYLOAD_DOMAIN,
        )],
    );
}

#[test]
fn all_declared_outputs_emit_exact_sorted_correspondence() {
    let mut request = request();
    request.requested_outputs = vec![
        generated_artifact_request(),
        review_payload_request(),
        target_ir_request(),
    ];

    let success = lower(request).expect("the exact declared output set is supported");
    assert_exact_outputs(
        &success.outputs,
        &[
            (
                GENERATED_ARTIFACT_ROLE,
                LoweringOutputKind::GeneratedArtifact,
                GENERATED_ARTIFACT_DOMAIN,
            ),
            (
                REVIEW_PAYLOAD_ROLE,
                LoweringOutputKind::ReviewPayload,
                REVIEW_PAYLOAD_DOMAIN,
            ),
            (
                TARGET_IR_ROLE,
                LoweringOutputKind::TargetIr,
                OUTER_TARGET_IR_DOMAIN,
            ),
        ],
    );
}

#[test]
fn generated_and_review_envelopes_bind_exact_projection_metadata() {
    let mut request = request();
    request.requested_outputs = vec![
        generated_artifact_request(),
        review_payload_request(),
        target_ir_request(),
    ];
    let success = lower(request).expect("the exact declared output set lowers");

    let generated = output_by_role(&success.outputs, GENERATED_ARTIFACT_ROLE);
    assert_eq!(
        generated.logical_path.as_deref(),
        Some(GENERATED_SOURCE_PATH)
    );
    let generated_envelope = output_envelope(generated);
    let CanonicalValueV1::Map(generated_fields) = &generated_envelope else {
        panic!("generated artifact envelope is not a map");
    };
    assert_eq!(generated_fields.len(), 5);
    assert_eq!(
        text_value(map_field(&generated_envelope, "apiVersion")),
        GENERATED_ARTIFACT_DOMAIN
    );
    assert_sha256_resource_ref(
        map_field(&generated_envelope, "profile"),
        GENERATED_ARTIFACT_PROFILE,
        GENERATED_ARTIFACT_PROFILE_DIGEST,
    );
    assert_eq!(
        text_value(map_field(&generated_envelope, "operation")),
        "a.b@1.t"
    );
    assert_eq!(
        text_value(map_field(&generated_envelope, "mediaType")),
        GENERATED_SOURCE_MEDIA_TYPE
    );
    assert!(bytes_field(&generated_envelope, "bytes")
        .starts_with(b"// SPDX-License-Identifier: Apache-2.0\n"));

    let review = output_by_role(&success.outputs, REVIEW_PAYLOAD_ROLE);
    assert_eq!(review.logical_path.as_deref(), Some(REVIEW_PATH));
    let review_envelope = output_envelope(review);
    let CanonicalValueV1::Map(review_fields) = &review_envelope else {
        panic!("review payload envelope is not a map");
    };
    assert_eq!(review_fields.len(), 5);
    assert_eq!(
        text_value(map_field(&review_envelope, "apiVersion")),
        REVIEW_PAYLOAD_DOMAIN
    );
    assert_eq!(
        map_field(&review_envelope, "authoritative"),
        &CanonicalValueV1::Bool(false)
    );
    let generated_digest =
        digest_canonical_value_v1(GENERATED_ARTIFACT_DOMAIN, &generated_envelope)
            .expect("the corresponding generated artifact has a domain-framed digest");
    assert_sha256_resource_ref(
        map_field(&review_envelope, "subject"),
        GENERATED_ARTIFACT_ROLE,
        &generated_digest,
    );
    assert_eq!(
        text_value(map_field(&review_envelope, "mediaType")),
        REVIEW_MEDIA_TYPE
    );
    let review_bytes = bytes_field(&review_envelope, "bytes");
    assert!(review_bytes.starts_with(b"{\"apiVersion\":\"echo.generated-helper-review/v1\""));
    let target_ir = output_by_role(&success.outputs, TARGET_IR_ROLE);
    let target_ir_value = decode_canonical_cbor_v1(&target_ir.artifact.bytes)
        .expect("the corresponding Target IR output is canonical CBOR");
    let target_ir_digest = digest_canonical_value_v1(OUTER_TARGET_IR_DOMAIN, &target_ir_value)
        .expect("the corresponding Target IR has a domain-framed digest");
    assert!(String::from_utf8_lossy(review_bytes).contains(&target_ir_digest));
    assert!(String::from_utf8_lossy(review_bytes).contains(&generated_digest));
    assert!(review_bytes.ends_with(b"}\n"));
}

#[test]
fn generated_and_review_bytes_are_invariant_under_requested_output_supersets() {
    let mut generated_request = request();
    generated_request.requested_outputs = vec![generated_artifact_request()];
    let generated_only = lower(generated_request).expect("generated-only request lowers");

    let mut review_request = request();
    review_request.requested_outputs = vec![review_payload_request()];
    let review_only = lower(review_request).expect("review-only request lowers");

    let mut all_request = request();
    all_request.requested_outputs = vec![
        generated_artifact_request(),
        review_payload_request(),
        target_ir_request(),
    ];
    let all = lower(all_request).expect("full sorted request lowers");

    assert_eq!(
        output_by_role(&generated_only.outputs, GENERATED_ARTIFACT_ROLE),
        output_by_role(&all.outputs, GENERATED_ARTIFACT_ROLE)
    );
    assert_eq!(
        output_by_role(&review_only.outputs, REVIEW_PAYLOAD_ROLE),
        output_by_role(&all.outputs, REVIEW_PAYLOAD_ROLE)
    );
}

#[test]
fn generated_source_exposes_explicit_post_assembly_bundle_binding() {
    let source = generated_source();
    for required_surface in [
        "pub struct ContractBundleIdentityV1",
        "pub struct RegistrationDescriptorV1",
        "pub enum BindingMismatchKind",
        "pub fn bind_contract_bundle",
    ] {
        assert!(
            source.contains(required_surface),
            "generated source is missing `{required_surface}`"
        );
    }
}

#[test]
fn generated_source_binds_the_expected_provider_schema_identity() {
    let source = generated_source();

    assert!(
        source.contains(&format!("\"{EXPECTED_PROVIDER_SCHEMA_SHA256_HEX}\"")),
        "generated source did not bind expected provider schema SHA-256 \
         {EXPECTED_PROVIDER_SCHEMA_SHA256_HEX}"
    );
}

#[test]
fn generated_source_exposes_the_expected_profile_owned_operation_identity() {
    let source = generated_source();

    assert!(
        source.contains(&format!(
            "pub const OPERATION_ID_LAW: &str = \"{EXPECTED_OPERATION_ID_LAW}\";"
        )),
        "generated source did not expose the expected operation-id law"
    );
    assert!(
        source.contains(&format!(
            "pub const OPERATION_ID: u32 = {EXPECTED_OPERATION_ID};"
        )),
        "generated source did not expose the expected operation id"
    );
}

#[test]
fn generated_bundle_binding_is_a_bounded_executable_consumer_contract() {
    let source = generated_source();
    let consumer = r#"
use echo_dpo::{
    bind_contract_bundle, BindingMismatchKind, ContractBundleIdentityV1,
    ExpectedContractBundleIdentityV1, CONTRACT_HOST_HELPER_API_VERSION,
    ECHO_CONTRACT_ABI_VERSION, EFFECT_FAILURE_SCHEMA, FOOTPRINT_ALGEBRA,
    FOOTPRINT_ALGEBRA_DIGEST, FOOTPRINT_ALGEBRA_DIGEST_DOMAIN, FOOTPRINT_OBLIGATION,
    GENERATED_ARTIFACT_PROFILE, GENERATED_ARTIFACT_PROFILE_DIGEST,
    GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN, INPUT_SCHEMA, OBSTRUCTION_COORDINATE,
    OBSTRUCTION_DOMAIN, OBSTRUCTION_PAYLOAD_SCHEMA, OPERATION_COORDINATE, OPERATION_DOMAIN,
    OPERATION_ID, OPERATION_ID_LAW, OPERATION_PROFILE, OPERATION_PROFILES_COORDINATE,
    OPERATION_PROFILES_DIGEST,
    OPERATION_PROFILES_DIGEST_DOMAIN, OPERATION_PROFILE_DOMAIN, OUTPUT_SCHEMA,
    PROVIDER_SCHEMA_COORDINATE, PROVIDER_SCHEMA_SHA256_HEX,
    RELEASE_BUNDLE_DIGEST_DOMAIN, SEMANTIC_BUNDLE_DIGEST_DOMAIN,
    TARGET_BUNDLE_PROFILE_COORDINATE, TARGET_BUNDLE_PROFILE_DIGEST,
    TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN, TARGET_IR_COORDINATE, TARGET_IR_DIGEST,
    TARGET_IR_DIGEST_DOMAIN, TARGET_PROFILE_COORDINATE, TARGET_PROFILE_DIGEST,
    TARGET_PROFILE_DIGEST_DOMAIN, TYPE_SCHEMA_DOMAIN,
};

const SEMANTIC_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const RELEASE_DIGEST: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const OTHER_DIGEST: &str =
    "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn exact_pin() -> ExpectedContractBundleIdentityV1<'static> {
    ExpectedContractBundleIdentityV1 {
        semantic_digest_domain: SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        semantic_digest: SEMANTIC_DIGEST,
        release_digest_domain: RELEASE_BUNDLE_DIGEST_DOMAIN,
        release_digest: RELEASE_DIGEST,
    }
}

fn matching_identity() -> ContractBundleIdentityV1<'static> {
    ContractBundleIdentityV1 {
        semantic_digest_domain: SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        semantic_digest: SEMANTIC_DIGEST,
        release_digest_domain: RELEASE_BUNDLE_DIGEST_DOMAIN,
        release_digest: RELEASE_DIGEST,
        operation_coordinate: OPERATION_COORDINATE,
        operation_domain: OPERATION_DOMAIN,
        operation_id_law: OPERATION_ID_LAW,
        operation_id: OPERATION_ID,
        target_ir_coordinate: TARGET_IR_COORDINATE,
        target_ir_digest_domain: TARGET_IR_DIGEST_DOMAIN,
        target_ir_digest: TARGET_IR_DIGEST,
        target_profile_coordinate: TARGET_PROFILE_COORDINATE,
        target_profile_digest_domain: TARGET_PROFILE_DIGEST_DOMAIN,
        target_profile_digest: TARGET_PROFILE_DIGEST,
        target_bundle_profile_coordinate: TARGET_BUNDLE_PROFILE_COORDINATE,
        target_bundle_profile_digest_domain: TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN,
        target_bundle_profile_digest: TARGET_BUNDLE_PROFILE_DIGEST,
        echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION,
        helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
        provider_schema_coordinate: PROVIDER_SCHEMA_COORDINATE,
        provider_schema_sha256_hex: PROVIDER_SCHEMA_SHA256_HEX,
        input_schema: INPUT_SCHEMA,
        output_schema: OUTPUT_SCHEMA,
        type_schema_domain: TYPE_SCHEMA_DOMAIN,
        obstruction_coordinate: OBSTRUCTION_COORDINATE,
        obstruction_domain: OBSTRUCTION_DOMAIN,
        effect_failure_schema: EFFECT_FAILURE_SCHEMA,
        obstruction_payload_schema: OBSTRUCTION_PAYLOAD_SCHEMA,
        generated_artifact_profile: GENERATED_ARTIFACT_PROFILE,
        generated_artifact_profile_digest_domain: GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN,
        generated_artifact_profile_digest: GENERATED_ARTIFACT_PROFILE_DIGEST,
        operation_profile: OPERATION_PROFILE,
        operation_profile_domain: OPERATION_PROFILE_DOMAIN,
        operation_profiles_coordinate: OPERATION_PROFILES_COORDINATE,
        operation_profiles_digest_domain: OPERATION_PROFILES_DIGEST_DOMAIN,
        operation_profiles_digest: OPERATION_PROFILES_DIGEST,
        footprint_obligation: FOOTPRINT_OBLIGATION,
        footprint_algebra: FOOTPRINT_ALGEBRA,
        footprint_algebra_digest_domain: FOOTPRINT_ALGEBRA_DIGEST_DOMAIN,
        footprint_algebra_digest: FOOTPRINT_ALGEBRA_DIGEST,
    }
}

fn expect_mismatch(
    identity: ContractBundleIdentityV1<'static>,
    expected: BindingMismatchKind,
) {
    assert_eq!(bind_contract_bundle(exact_pin(), identity), Err(expected));
}

fn main() {
    let matching = matching_identity();
    let descriptor = bind_contract_bundle(exact_pin(), matching)
        .expect("bundle claims match the independent host pin");
    assert_eq!(*descriptor.contract_bundle(), matching);
    assert_eq!(descriptor.operation_id(), OPERATION_ID);

    expect_mismatch(
        ContractBundleIdentityV1 {
            semantic_digest_domain: "wrong.bundle.semantic/v1",
            ..matching_identity()
        },
        BindingMismatchKind::BundleDigestDomain,
    );

    expect_mismatch(
        ContractBundleIdentityV1 {
            semantic_digest: "sha256:not-hex",
            ..matching_identity()
        },
        BindingMismatchKind::BundleDigest,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            release_digest: "sha256:ABCDEF",
            ..matching_identity()
        },
        BindingMismatchKind::BundleDigest,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            semantic_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        BindingMismatchKind::SemanticBundleDigest,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            release_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        BindingMismatchKind::ReleaseBundleDigest,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            operation_coordinate: "a.b@1.other",
            ..matching_identity()
        },
        BindingMismatchKind::Operation,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            operation_domain: "wrong.operation/v1",
            ..matching_identity()
        },
        BindingMismatchKind::Operation,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            operation_id_law: "wrong.operation-id-law/v1",
            ..matching_identity()
        },
        BindingMismatchKind::OperationId,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            operation_id: OPERATION_ID + 1,
            ..matching_identity()
        },
        BindingMismatchKind::OperationId,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_ir_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        BindingMismatchKind::TargetIr,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_ir_coordinate: "wrong.target-ir/v1",
            ..matching_identity()
        },
        BindingMismatchKind::TargetIr,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_ir_digest_domain: "wrong.target-ir-artifact/v1",
            ..matching_identity()
        },
        BindingMismatchKind::TargetIr,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_profile_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        BindingMismatchKind::TargetProfile,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_profile_coordinate: "wrong.target-profile@1",
            ..matching_identity()
        },
        BindingMismatchKind::TargetProfile,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_profile_digest_domain: "wrong.target-profile/v1",
            ..matching_identity()
        },
        BindingMismatchKind::TargetProfile,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_bundle_profile_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        BindingMismatchKind::TargetBundleProfile,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_bundle_profile_coordinate: "wrong.bundle-profile/v1",
            ..matching_identity()
        },
        BindingMismatchKind::TargetBundleProfile,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            target_bundle_profile_digest_domain: "wrong.bundle-profile-domain/v1",
            ..matching_identity()
        },
        BindingMismatchKind::TargetBundleProfile,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION + 1,
            ..matching_identity()
        },
        BindingMismatchKind::EchoAbi,
    );
    expect_mismatch(
        ContractBundleIdentityV1 {
            helper_api_version: CONTRACT_HOST_HELPER_API_VERSION + 1,
            ..matching_identity()
        },
        BindingMismatchKind::HelperApi,
    );
    for schema_mismatch in [
        ContractBundleIdentityV1 {
            provider_schema_coordinate: "wrong.provider-schema@1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            provider_schema_sha256_hex: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            input_schema: "a.b@1.WrongInput",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            output_schema: "a.b@1.WrongOutput",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            type_schema_domain: "wrong.value/v1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            obstruction_coordinate: "domain.WrongObstruction",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            obstruction_domain: "wrong.obstruction/v1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            effect_failure_schema: "target.wrong.rejected",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            obstruction_payload_schema: "domain.WrongObstruction.Payload",
            ..matching_identity()
        },
    ] {
        expect_mismatch(schema_mismatch, BindingMismatchKind::Schema);
    }
    for profile_mismatch in [
        ContractBundleIdentityV1 {
            generated_artifact_profile: "wrong.registration/v1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            generated_artifact_profile_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            generated_artifact_profile_digest_domain: "wrong.generated-profile/v1",
            ..matching_identity()
        },
    ] {
        expect_mismatch(
            profile_mismatch,
            BindingMismatchKind::GeneratedArtifactProfile,
        );
    }
    for profile_mismatch in [
        ContractBundleIdentityV1 {
            operation_profile: "wrong.operation-profile/v1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            operation_profiles_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            operation_profile_domain: "wrong.operation-profile-member/v1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            operation_profiles_coordinate: "wrong.operation-profiles/v1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            operation_profiles_digest_domain: "wrong.operation-profiles-domain/v1",
            ..matching_identity()
        },
    ] {
        expect_mismatch(profile_mismatch, BindingMismatchKind::OperationProfile);
    }
    for footprint_mismatch in [
        ContractBundleIdentityV1 {
            footprint_obligation: "wrong.footprint",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            footprint_algebra: "wrong.footprint/v1",
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            footprint_algebra_digest: OTHER_DIGEST,
            ..matching_identity()
        },
        ContractBundleIdentityV1 {
            footprint_algebra_digest_domain: "wrong.footprint-domain/v1",
            ..matching_identity()
        },
    ] {
        expect_mismatch(footprint_mismatch, BindingMismatchKind::Footprint);
    }
}
"#;
    let execution = compile_and_run_generated_consumer(&format!("{source}\n{consumer}"));
    assert!(
        execution.status.success(),
        "generated consumer failed at runtime: {}",
        String::from_utf8_lossy(&execution.stderr)
    );

    let forbidden_public_symbols = [
        "rewriterule",
        "scheduler",
        "tick",
        "wal",
        "recovery",
        "install_contract",
        "install_package",
        "register_contract_package",
        "register_rule",
    ];
    for public_line in source
        .lines()
        .filter(|line| line.trim_start().starts_with("pub "))
    {
        let normalized = public_line.to_ascii_lowercase();
        for forbidden in forbidden_public_symbols {
            assert!(
                !normalized.contains(forbidden),
                "generated source exposes forbidden public runtime symbol `{forbidden}` in `{public_line}`"
            );
        }
    }
}

#[test]
fn contract_bundle_binding_separates_semantic_and_release_digests() {
    let source = generated_source();
    for line in source
        .lines()
        .filter(|line| line.contains(TARGET_BUNDLE_PROFILE_DIGEST))
    {
        assert!(
            !line.contains("CONTRACT_BUNDLE_DIGEST"),
            "the target-bundle-profile digest was mislabeled as a final contract-bundle digest"
        );
    }
    assert!(source.contains(TARGET_BUNDLE_PROFILE));
    assert!(source.contains("TARGET_BUNDLE_PROFILE_DIGEST"));
    assert!(
        source.contains("semantic_digest"),
        "contract-bundle identity lacks its semantic digest"
    );
    assert!(
        source.contains("release_digest"),
        "contract-bundle identity lacks its release digest"
    );
}

#[test]
fn review_declares_explicit_post_assembly_contract_bundle_binding() {
    let review = review_json();
    assert!(review.contains(&format!(
        "\"targetBundleProfile\":{{\"id\":\"{TARGET_BUNDLE_PROFILE}\",\"digest\":\"{TARGET_BUNDLE_PROFILE_DIGEST}\"}}"
    )));
    assert!(
        review.contains("\"contractBundle\":"),
        "review does not distinguish the final contract bundle from its target-bundle profile"
    );
    assert!(
        review.contains("\"binding\":\"explicit-after-assembly\""),
        "review does not state the explicit post-assembly binding law"
    );
}

#[test]
fn invalid_output_requests_refuse_with_the_stable_first_subject() {
    let cases = [
        (
            "wrong domain",
            vec![output_request(
                TARGET_IR_ROLE,
                LoweringOutputKind::TargetIr,
                "wrong.target-ir/v1",
            )],
            TARGET_IR_ROLE,
        ),
        (
            "wrong kind",
            vec![output_request(
                TARGET_IR_ROLE,
                LoweringOutputKind::GeneratedArtifact,
                OUTER_TARGET_IR_DOMAIN,
            )],
            TARGET_IR_ROLE,
        ),
        (
            "unknown role",
            vec![output_request(
                "unknown.echo-dpo",
                LoweringOutputKind::TargetIr,
                OUTER_TARGET_IR_DOMAIN,
            )],
            "unknown.echo-dpo",
        ),
        (
            "duplicate role",
            vec![target_ir_request(), target_ir_request()],
            TARGET_IR_ROLE,
        ),
        (
            "out-of-order roles",
            vec![target_ir_request(), generated_artifact_request()],
            GENERATED_ARTIFACT_ROLE,
        ),
    ];

    for (label, requested_outputs, expected_subject) in cases {
        let mut request = request();
        request.requested_outputs = requested_outputs;
        let refusal = lower(request).expect_err(label);
        assert_eq!(
            refusal.kind,
            ProviderRefusalKind::UnsupportedOutputRole,
            "{label}"
        );
        assert_eq!(
            refusal.subject.as_deref(),
            Some(expected_subject),
            "{label}"
        );
    }
}

#[test]
fn unrecognized_lowerability_obligation_refuses_instead_of_being_ignored() {
    let mut lowerability = lowerability_value();
    *map_field_mut(&mut lowerability, "footprintObligations") =
        CanonicalValueV1::Array(vec![text("unexpected.footprint")]);
    let mut request = request();
    request.semantic_inputs[3].artifact = bound(
        "echo.dpo-lowerability@1",
        LOWERABILITY_DOMAIN,
        canonical_bytes(&lowerability),
    );

    let refusal = lower(request).expect_err("unrecognized obligations cannot be discharged");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(refusal.subject.as_deref(), Some("lowerability.echo-dpo"));
}

#[test]
fn no_requested_outputs_is_an_empty_success() {
    let mut request = request();
    request.requested_outputs.clear();
    let success = lower(request).expect("zero requested roles is valid");
    assert!(success.outputs.is_empty());
    assert!(success.diagnostics.is_empty());
}

#[test]
fn provider_output_is_independent_of_host_response_limits() {
    let first = lower(request()).expect("baseline lowers");
    let mut changed = request();
    changed.limits = ResponseLimitsV1 {
        max_output_count: 0,
        max_diagnostic_count: 0,
        max_total_response_bytes: 0,
    };
    let second = lower(changed).expect("provider does not reinterpret host limits");
    assert_eq!(first, second);
}

#[test]
fn protocol_target_and_output_mismatches_refuse_with_stable_kinds() {
    let mut protocol = request();
    protocol.protocol_version.patch = 1;
    assert_eq!(
        lower(protocol).expect_err("protocol mismatch refuses").kind,
        ProviderRefusalKind::UnsupportedCoreAbi
    );

    let mut profile = request();
    profile.target_profile.reference.coordinate = "echo.other@1".to_owned();
    assert_eq!(
        lower(profile).expect_err("profile mismatch refuses").kind,
        ProviderRefusalKind::UnsupportedTargetProfile
    );

    let mut profile_domain = request();
    let coordinate = profile_domain.target_profile.reference.coordinate.clone();
    let bytes = profile_domain.target_profile.artifact.bytes.clone();
    profile_domain.target_profile = bound(&coordinate, "wrong.target-profile/v1", bytes);
    assert_eq!(
        lower(profile_domain)
            .expect_err("profile domain mismatch refuses")
            .kind,
        ProviderRefusalKind::UnsupportedTargetProfile
    );

    let mut output = request();
    output.requested_outputs[0].role = "generated.echo-dpo".to_owned();
    assert_eq!(
        lower(output).expect_err("unserved output refuses").kind,
        ProviderRefusalKind::UnsupportedOutputRole
    );
}

#[test]
fn malformed_or_incomplete_semantic_closure_refuses() {
    let mut malformed = request();
    malformed.semantic_inputs[0].artifact.artifact.bytes.push(0);
    assert_eq!(
        lower(malformed)
            .expect_err("malformed authority facts refuse")
            .kind,
        ProviderRefusalKind::InvalidSemanticArtifact
    );

    let mut missing = request();
    missing
        .semantic_inputs
        .retain(|input| input.role != "authority-facts.echo-lawpack");
    assert_eq!(
        lower(missing).expect_err("incomplete closure refuses").kind,
        ProviderRefusalKind::UnsupportedSemantics
    );
}

#[test]
fn unsupported_core_abi_and_semantics_refuse_without_artifacts() {
    let mut wrong_abi = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(&mut wrong_abi, "apiVersion") = text("edict.core/v2");
    assert_eq!(
        lower(request_with_core(wrong_abi))
            .expect_err("unknown Core ABI refuses")
            .kind,
        ProviderRefusalKind::UnsupportedCoreAbi
    );

    let read = core_value(ordinary_result(), None);
    assert_eq!(
        lower(request_with_core(read))
            .expect_err("effect-free reads are not synthetic mutations")
            .kind,
        ProviderRefusalKind::UnsupportedSemantics
    );
}

#[test]
fn fully_qualified_core_intent_key_refuses_instead_of_broadening_the_boundary() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    let CanonicalValueV1::Map(intents) = map_field_mut(&mut core, "intents") else {
        panic!("Core intents is not a map");
    };
    intents[0].0 = text("a.b@1.t");

    assert_eq!(
        lower(request_with_core(core))
            .expect_err("the canonical Core intent key is package-local")
            .kind,
        ProviderRefusalKind::UnsupportedSemantics
    );
}

#[test]
fn rebound_core_coordinate_refuses_as_unsupported_semantics() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(&mut core, "coordinate") = text("x.y@1");
    let mut request = request_with_core(core);
    request.core.reference.coordinate = "x.y@1".to_owned();

    let refusal = lower(request).expect_err("a rebound Core module is not the reviewed operation");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(refusal.subject.as_deref(), Some("x.y@1"));
}

#[test]
fn authored_core_optic_refuses_instead_of_being_silently_discarded() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    let CanonicalValueV1::Map(intents) = map_field_mut(&mut core, "intents") else {
        panic!("Core intents is not a map");
    };
    let CanonicalValueV1::Map(intent) = &mut intents[0].1 else {
        panic!("Core intent is not a map");
    };
    intent.push((
        text("optic"),
        map([
            ("opticKind", text("affectReintegration")),
            ("boundaryKind", text("affect")),
            (
                "apertureRequirement",
                map([
                    ("kind", text("footprintCeiling")),
                    ("ref", text("echo.dpo@1.replace-footprint")),
                ]),
            ),
            ("supportPolicy", text("echo.dpo@1.replace-support")),
            ("lossDisposition", text("refuse")),
        ]),
    ));
    intent.sort_by_cached_key(|(key, _)| canonical_bytes(key));

    let refusal = lower(request_with_core(core))
        .expect_err("an authored optic cannot disappear across lowering");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(refusal.subject.as_deref(), Some("a.b@1.t"));
}

#[test]
fn unsupported_intent_type_bindings_refuse_instead_of_being_ignored() {
    for field in ["input", "output"] {
        let mut core = core_value(ordinary_result(), Some("target.replace"));
        let CanonicalValueV1::Map(intents) = map_field_mut(&mut core, "intents") else {
            panic!("Core intents is not a map");
        };
        *map_field_mut(&mut intents[0].1, field) = text("x.y@1.Other");

        let refusal = lower(request_with_core(core))
            .expect_err("unsupported operation type bindings cannot disappear across lowering");
        assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
        assert_eq!(refusal.subject.as_deref(), Some("a.b@1.t"));
    }
}

#[test]
fn altered_core_type_definitions_refuse_instead_of_disappearing() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    let input_id = map_field_mut(map_field_mut(&mut core, "types"), "Input.id");
    *map_field_mut(input_id, "max") = integer(17);

    let refusal = lower(request_with_core(core))
        .expect_err("changed Core type semantics cannot disappear across lowering");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(refusal.subject.as_deref(), Some("a.b@1"));
    assert_eq!(refusal.diagnostics.len(), 1);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.provider.unsupported-semantics"
    );
}

#[test]
fn changed_evaluation_budget_refuses_instead_of_broadening_the_closure() {
    for (field, value) in [
        ("maxSteps", 0),
        ("maxAllocatedBytes", 2048),
        ("maxOutputBytes", 512),
    ] {
        let mut core = core_value(ordinary_result(), Some("target.replace"));
        let intent = map_field_mut(map_field_mut(&mut core, "intents"), "t");
        *map_field_mut(map_field_mut(intent, "coreEvaluationBudget"), field) = integer(value);

        let refusal = lower(request_with_core(core))
            .expect_err("a different evaluation budget is outside the reviewed closure");
        assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
        assert_eq!(refusal.subject.as_deref(), Some("a.b@1.t"));
        assert_eq!(refusal.diagnostics.len(), 1);
        assert_eq!(
            refusal.diagnostics[0].code,
            "echo.provider.unsupported-semantics"
        );
    }
}

#[test]
fn nonempty_input_constraints_refuse_instead_of_crossing_unchecked() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    let CanonicalValueV1::Array(constraints) =
        map_field_mut(operation_intent_mut(&mut core), "inputConstraints")
    else {
        panic!("Core input constraints is not an array");
    };
    constraints.push(map([
        ("coordinate", text("a.b@1.t.where.0")),
        ("source", text("where")),
        (
            "predicate",
            map([
                ("kind", text("call")),
                ("predicate", text("domain.Unreviewed")),
                (
                    "args",
                    CanonicalValueV1::Array(vec![local_expr(local(
                        "local:99",
                        "ghost",
                        "a.b@1.Input",
                    ))]),
                ),
            ]),
        ),
    ]));

    assert_unsupported_semantics(
        core,
        "input constraints need explicit lowering and scope validation",
    );
}

#[test]
fn unreviewed_effect_input_call_refuses_as_unsupported_semantics() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(operation_node_mut(&mut core), "input") = call_expr("domain.Unreviewed");

    assert_unsupported_semantics(
        core,
        "an unreviewed effect-input call cannot cross lowering",
    );
}

#[test]
fn unreviewed_intent_result_call_refuses_as_unsupported_semantics() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(operation_body_mut(&mut core), "result") =
        record_expr(call_expr("domain.Unreviewed"));

    assert_unsupported_semantics(
        core,
        "an unreviewed intent-result call cannot cross lowering",
    );
}

#[test]
fn nested_undeclared_local_reference_refuses_with_stable_details() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(operation_body_mut(&mut core), "result") = record_expr(field_expr(
        local_expr(local("local:99", "ghost", "a.b@1.Input")),
        "id",
    ));
    assert_out_of_scope(core, "a nested undeclared result local cannot lower");
}

#[test]
fn effect_result_binding_is_not_visible_in_its_own_input() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(operation_node_mut(&mut core), "input") =
        local_expr(local("local:1", "receipt", "a.b@1.Receipt"));
    assert_out_of_scope(core, "an effect cannot consume its own result binding");
}

#[test]
fn obstruction_binder_does_not_escape_its_arm() {
    let mut core = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(operation_body_mut(&mut core), "result") =
        local_expr(local("local:2", "reason", "target.replace.rejected"));
    assert_out_of_scope(core, "an obstruction binder cannot escape its arm");
}

#[test]
fn local_reference_must_match_the_declared_identity_triple() {
    for changed_reference in [
        local("local:0", "other", "a.b@1.Input"),
        local("local:0", "input", "a.b@1.Output"),
    ] {
        let mut core = core_value(ordinary_result(), Some("target.replace"));
        *map_field_mut(operation_body_mut(&mut core), "result") =
            record_expr(field_expr(local_expr(changed_reference), "id"));
        assert_out_of_scope(core, "a local reference cannot alter its declaration");
    }
}

#[test]
fn duplicate_or_conflicting_local_ids_refuse() {
    for duplicate in [
        local("local:0", "input", "a.b@1.Input"),
        local("local:0", "other", "a.b@1.Output"),
    ] {
        let mut core = core_value(ordinary_result(), Some("target.replace"));
        let CanonicalValueV1::Array(locals) =
            map_field_mut(operation_body_mut(&mut core), "locals")
        else {
            panic!("Core locals is not an array");
        };
        locals.push(duplicate);
        assert_invalid_local_declarations(core);
    }
}

#[test]
fn local_inventory_is_exactly_the_reviewed_binding_closure() {
    for missing_id in ["local:1", "local:2"] {
        let mut core = core_value(ordinary_result(), Some("target.replace"));
        operation_locals_mut(&mut core)
            .retain(|local| text_value(map_field(local, "id")) != missing_id);
        assert_invalid_local_declarations(core);
    }

    let mut core = core_value(ordinary_result(), Some("target.replace"));
    operation_locals_mut(&mut core).push(local("local:99", "ghost", "a.b@1.Input"));
    assert_invalid_local_declarations(core);
}

#[test]
fn local_binding_roles_authenticate_their_reviewed_types() {
    let mut input = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(local_by_id_mut(&mut input, "local:0"), "type") = text("a.b@1.Output");
    assert_invalid_local_declarations(input);

    let mut receipt = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(local_by_id_mut(&mut receipt, "local:1"), "type") = text("a.b@1.Output");
    *map_field_mut(
        map_field_mut(operation_node_mut(&mut receipt), "binding"),
        "type",
    ) = text("a.b@1.Output");
    assert_invalid_local_declarations(receipt);

    let mut reason = core_value(ordinary_result(), Some("target.replace"));
    *map_field_mut(local_by_id_mut(&mut reason, "local:2"), "type") = text("a.b@1.Input");
    *map_field_mut(
        map_field_mut(obstruction_arm_mut(&mut reason), "binder"),
        "type",
    ) = text("a.b@1.Input");
    assert_invalid_local_declarations(reason);
}

#[test]
fn obstruction_constructor_is_exactly_the_reviewed_mapping() {
    for (field, changed) in [
        ("callee", text("domain.Unreviewed")),
        ("typeArgs", CanonicalValueV1::Array(vec![text("T")])),
        (
            "args",
            CanonicalValueV1::Array(vec![local_expr(local("local:0", "input", "a.b@1.Input"))]),
        ),
    ] {
        let mut core = core_value(ordinary_result(), Some("target.replace"));
        *map_field_mut(
            map_field_mut(obstruction_arm_mut(&mut core), "value"),
            field,
        ) = changed;
        assert_unsupported_semantics(
            core,
            "an unreviewed obstruction constructor cannot cross lowering",
        );
    }
}

#[test]
fn renamed_lowerability_artifact_refuses_as_an_invalid_closure_member() {
    let mut request = request();
    request.semantic_inputs[3].artifact.reference.coordinate =
        "echo.dpo-renamed-lowerability@1".to_owned();

    let refusal = lower(request).expect_err("lowerability identity includes its coordinate");
    assert_eq!(refusal.kind, ProviderRefusalKind::InvalidSemanticArtifact);
    assert_eq!(refusal.subject.as_deref(), Some("lowerability.echo-dpo"));
}

#[test]
fn lowering_uses_core_values_instead_of_replaying_static_bytes() {
    let baseline = lower(request()).expect("baseline lowers").outputs.remove(0);
    let changed_result = record_expr(map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("string")), ("value", text("changed"))]),
        ),
    ]));
    let changed = lower(request_with_core(core_value(
        changed_result.clone(),
        Some("target.replace"),
    )))
    .expect("supported semantic variant lowers")
    .outputs
    .remove(0);
    assert_ne!(baseline.artifact.bytes, changed.artifact.bytes);
    let value = decode_canonical_cbor_v1(&changed.artifact.bytes).expect("output is canonical");
    let intent = map_field(map_field(&value, "intents"), "t");
    assert_eq!(map_field(intent, "result"), &changed_result);
}

fn map_field_mut<'a>(value: &'a mut CanonicalValueV1, field: &str) -> &'a mut CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("value is not a map");
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .unwrap_or_else(|| panic!("map field `{field}` is absent"))
}

fn operation_body_mut(core: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    map_field_mut(operation_intent_mut(core), "body")
}

fn operation_intent_mut(core: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    map_field_mut(map_field_mut(core, "intents"), "t")
}

fn operation_locals_mut(core: &mut CanonicalValueV1) -> &mut Vec<CanonicalValueV1> {
    let CanonicalValueV1::Array(locals) = map_field_mut(operation_body_mut(core), "locals") else {
        panic!("Core locals is not an array");
    };
    locals
}

fn local_by_id_mut<'a>(
    core: &'a mut CanonicalValueV1,
    expected_id: &str,
) -> &'a mut CanonicalValueV1 {
    operation_locals_mut(core)
        .iter_mut()
        .find(|local| text_value(map_field(local, "id")) == expected_id)
        .unwrap_or_else(|| panic!("Core local `{expected_id}` is absent"))
}

fn operation_node_mut(core: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    let CanonicalValueV1::Array(nodes) = map_field_mut(operation_body_mut(core), "nodes") else {
        panic!("Core nodes is not an array");
    };
    nodes.first_mut().expect("reviewed closure has one node")
}

fn obstruction_arm_mut(core: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    map_field_mut(
        map_field_mut(operation_node_mut(core), "obstructionMap"),
        "rejected",
    )
}

fn assert_out_of_scope(core: CanonicalValueV1, message: &str) {
    let refusal = lower(request_with_core(core)).expect_err(message);
    assert_eq!(refusal.kind, ProviderRefusalKind::InvalidSemanticArtifact);
    assert_eq!(refusal.subject.as_deref(), Some("a.b@1.t"));
    assert_eq!(refusal.diagnostics.len(), 1);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.provider.local-reference-out-of-scope"
    );
}

fn assert_invalid_local_declarations(core: CanonicalValueV1) {
    let refusal = lower(request_with_core(core)).expect_err("ambiguous Core locals cannot lower");
    assert_eq!(refusal.kind, ProviderRefusalKind::InvalidSemanticArtifact);
    assert_eq!(refusal.subject.as_deref(), Some("a.b@1.t"));
    assert_eq!(refusal.diagnostics.len(), 1);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.provider.invalid-semantic-artifact"
    );
}

fn assert_unsupported_semantics(core: CanonicalValueV1, message: &str) {
    let refusal = lower(request_with_core(core)).expect_err(message);
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(refusal.subject.as_deref(), Some("a.b@1.t"));
    assert_eq!(refusal.diagnostics.len(), 1);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.provider.unsupported-semantics"
    );
}
