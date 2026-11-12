// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The public lowerer and verifier routes for an authored program that calls
//! all five `echo.dpo@2` mutation intrinsics without a failure map (#985,
//! amblicit#311).
//!
//! The package, report, lawpack, adapter and configuration under
//! `crates/warp-core/tests/fixtures/edict-effect-step` are unchanged public
//! compiler output (Amblicit `0034605`): the program ensures and creates a
//! node, replaces an atom, ensures an edge and re-targets one, in that order. The lowering request is rebuilt from them and
//! from the retained source; Core, Target IR and exports come from the
//! package. The compiler does not publish its result projection, which the
//! effect route binds by digest but never reads, so the request carries a
//! stand-in.
//!
//! Oracles: the public compiler's own package and report bytes (class 4, the
//! components it ran), and the verifier's binding law that a package is
//! accepted only for the request that determines it.
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use echo_edict_provider_lowerer::{
    lower, Artifact, BoundArtifact, Digest, DigestAlgorithm, LoweringOutputKind,
    LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1, ResourceRef, ResponseLimitsV1,
    SemanticInput, SemanticInputKind,
};
use echo_edict_provider_verifier as verifier;

macro_rules! retained {
    ($name:literal) => {
        hex::decode(
            include_str!(concat!(
                "../../warp-core/tests/fixtures/edict-effect-mutations/",
                $name
            ))
            .trim(),
        )
        .expect("retained public bytes")
    };
}

const SOURCE: &str =
    include_str!("../../warp-core/tests/fixtures/edict-effect-mutations/source/WriteGraph.edict");
const EFFECT_PROFILE: &[u8] = include_bytes!("../resources/target-profile.echo-dpo-2.cbor");
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const MODULE: &str = "consumer.writes.program@1";
const TARGET_IR: &str = "edict.target-ir.artifact/v1";

fn bound(coordinate: &str, domain: &str, bytes: Vec<u8>) -> BoundArtifact {
    let value = decode(&bytes).expect("canonical artifact");
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: digest(domain, &value).expect("identity").to_vec(),
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    &entries
        .iter()
        .find(|(key, _)| key == &Value::Text(name.to_owned()))
        .expect("field")
        .1
}

fn bytes(value: &Value) -> &[u8] {
    let Value::Bytes(bytes) = value else {
        panic!("bytes")
    };
    bytes
}

fn public_package() -> Vec<u8> {
    retained!("write-executable-operation-package.hex")
}

/// The compiler bundle embedded in the public package.
fn bundle() -> Value {
    let package = decode(&public_package()).expect("package");
    let program = decode(bytes(field(&package, "program"))).expect("program");
    decode(bytes(field(&program, "compiled_program"))).expect("bundle")
}

/// The lowering request the public compiler sent.
fn lowering_request() -> LoweringRequestV1 {
    let bundle = bundle();
    let embedded = |key| bytes(field(&bundle, key)).to_vec();
    let auxiliary = |role: &str, kind: &str, coordinate: &str, domain: &str, raw| SemanticInput {
        role: role.to_owned(),
        kind: SemanticInputKind::Auxiliary(kind.to_owned()),
        artifact: bound(coordinate, domain, raw),
    };
    let source = encode(&Value::Bytes(SOURCE.as_bytes().to_vec())).expect("source");
    let projection = encode(&Value::Text("stand-in".to_owned())).expect("stand-in");
    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: bound(MODULE, "edict.core.module/v1", embedded("core_artifact")),
        target_profile: bound(
            "echo.dpo@2",
            "edict.target-profile/v1",
            EFFECT_PROFILE.to_vec(),
        ),
        semantic_inputs: vec![
            auxiliary(
                "01-lawpack-adapter",
                "lawpack-adapter",
                "consumer.writes.echo-adapter/v1",
                "edict.lawpack-adapter/v1",
                retained!("write-adapter.hex"),
            ),
            auxiliary(
                "02-lawpack-exports",
                "lawpack-exports",
                "consumer.writes.exports/v1",
                "edict.lawpack-exports/v1",
                embedded("lawpack_exports_artifact"),
            ),
            SemanticInput {
                role: "03-lawpack".to_owned(),
                kind: SemanticInputKind::Lawpack,
                artifact: bound(
                    "consumer.writes@1",
                    "edict.lawpack/v1",
                    retained!("write-lawpack.hex"),
                ),
            },
            auxiliary(
                "04-source",
                "edict-source",
                MODULE,
                "edict.source/v1",
                source,
            ),
            auxiliary(
                "05-target-configuration",
                "target-configuration",
                "echo.operation-lowering-configuration/v1",
                "echo.operation-lowering-configuration/v1",
                retained!("write-configuration.hex"),
            ),
            auxiliary(
                "06-target-ir",
                "target-ir",
                "echo.span-ir/v2",
                TARGET_IR,
                embedded("target_ir_artifact"),
            ),
            auxiliary(
                "07-result-projection",
                "result-projection",
                "consumer.writes.program@1.write",
                "edict.result-projection.artifact/v1",
                projection,
            ),
        ],
        requested_outputs: vec![LoweringOutputRequest {
            role: "executable-operation-package.echo".to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE_DOMAIN.to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 1,
            max_total_response_bytes: 16 * 1024 * 1024,
            max_diagnostic_count: 8,
        },
    }
}

fn transport(artifact: BoundArtifact) -> verifier::BoundArtifact {
    verifier::BoundArtifact {
        reference: verifier::ResourceRef {
            coordinate: artifact.reference.coordinate,
            digest: verifier::Digest {
                algorithm: verifier::DigestAlgorithm::Sha256,
                bytes: artifact.reference.digest.bytes,
            },
        },
        artifact: verifier::Artifact {
            domain: artifact.artifact.domain,
            bytes: artifact.artifact.bytes,
        },
    }
}

/// The verification request for `package` against `source`.
fn verification_request(
    mut source: LoweringRequestV1,
    package: Vec<u8>,
) -> verifier::VerificationRequestV1 {
    let target = source.semantic_inputs.remove(5).artifact;
    let mut inputs: Vec<_> = source
        .semantic_inputs
        .into_iter()
        .map(|input| verifier::SemanticInput {
            role: input.role,
            kind: match input.kind {
                SemanticInputKind::Lawpack => verifier::SemanticInputKind::Lawpack,
                SemanticInputKind::Auxiliary(kind) => verifier::SemanticInputKind::Auxiliary(kind),
                _ => panic!("unexpected compiler semantic kind"),
            },
            artifact: transport(input.artifact),
        })
        .collect();
    inputs.push(verifier::SemanticInput {
        role: "executable-operation-package.echo".to_owned(),
        kind: verifier::SemanticInputKind::Auxiliary("executable-operation-package".to_owned()),
        artifact: transport(bound(
            "executable-operation-package.echo",
            PACKAGE_DOMAIN,
            package,
        )),
    });
    verifier::VerificationRequestV1 {
        protocol_version: verifier::ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: transport(source.core),
        target_profile: transport(source.target_profile),
        target_ir: transport(target),
        semantic_inputs: inputs,
        requested_outputs: vec![verifier::VerificationOutputRequest {
            role: "verifier-report.echo-operation".to_owned(),
            kind: verifier::VerificationOutputKind::VerifierReport,
            domain: "echo.operation-package-verifier-report/v1".to_owned(),
        }],
        limits: verifier::ResponseLimitsV1 {
            max_output_count: 1,
            max_diagnostic_count: 8,
            max_total_response_bytes: 16 * 1024 * 1024,
        },
    }
}

/// The report's outcome and diagnostic code for `package`.
fn verdict(package: Vec<u8>) -> (String, String, Vec<u8>) {
    let response =
        verifier::verify(verification_request(lowering_request(), package)).expect("bound request");
    let bytes_out = response.outputs[0].artifact.bytes.clone();
    let report = decode(&bytes_out).expect("report");
    let Value::Text(outcome) = field(&report, "outcome") else {
        panic!("outcome")
    };
    let code = String::from_utf8(bytes(field(&report, "diagnosticBytes")).to_vec())
        .expect("diagnostic code");
    (outcome.clone(), code, bytes_out)
}

fn refusal(result: Result<impl Sized, Vec<String>>) -> String {
    result.err().expect("refused").join("; ")
}

fn lowerer_refusal(request: LoweringRequestV1) -> String {
    refusal(lower(request).map_err(|refusal| {
        refusal
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }))
}

fn verifier_refusal(request: verifier::VerificationRequestV1) -> String {
    refusal(verifier::verify(request).map_err(|refusal| {
        refusal
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }))
}

const MUTATIONS: [&str; 5] = [
    "echo.dpo@2.ensure-node",
    "echo.dpo@2.create-node",
    "echo.dpo@2.replace-atom",
    "echo.dpo@2.ensure-edge",
    "echo.dpo@2.replace-edge",
];

#[test]
fn the_program_calls_each_mutation_in_source_order_with_empty_failure_sets() {
    let bundle = bundle();
    let core = decode(bytes(field(&bundle, "core_artifact"))).expect("Core");
    let target = decode(bytes(field(&bundle, "target_ir_artifact"))).expect("IR");
    let Value::Map(intents) = field(&target, "intents") else {
        panic!("intents")
    };
    let Value::Array(steps) = field(&intents[0].1, "steps") else {
        panic!("steps")
    };
    let Value::Array(order) = field(&intents[0].1, "executionOrder") else {
        panic!("order")
    };
    let empty_map = Value::Map(vec![]);
    let names: Vec<_> = order
        .iter()
        .map(|id| {
            let step = steps
                .iter()
                .find(|step| field(step, "id") == id)
                .expect("ordered step");
            // Present and empty, not omitted.
            assert_eq!(field(step, "obstructionArms"), &empty_map);
            assert_eq!(field(step, "obstructionFailures"), &Value::Array(vec![]));
            let Value::Text(name) = field(step, "targetIntrinsic") else {
                panic!("intrinsic")
            };
            name.as_str()
        })
        .collect();
    assert_eq!(names, MUTATIONS);
    let Value::Map(core_intents) = field(&core, "intents") else {
        panic!("Core intents")
    };
    let Value::Array(nodes) = field(field(&core_intents[0].1, "body"), "nodes") else {
        panic!("nodes")
    };
    let effects: Vec<_> = nodes
        .iter()
        .filter(|node| field(node, "kind") == &Value::Text("effect".to_owned()))
        .collect();
    assert_eq!(effects.len(), MUTATIONS.len());
    for node in effects {
        assert_eq!(field(node, "obstructionMap"), &empty_map);
    }
}

#[test]
fn the_native_lowerer_reproduces_the_public_mutation_package() {
    let mut response = lower(lowering_request()).expect("the public route lowers the program");
    let output = response.outputs.remove(0);
    assert_eq!(output.artifact.domain, PACKAGE_DOMAIN);
    assert_eq!(output.artifact.bytes, public_package());
}

#[test]
fn the_separate_verifier_accepts_the_public_mutation_package() {
    // The retained report names the compiler's own result projection, which
    // this request replaces with a stand-in, so only the outcome is compared.
    let (outcome, code, _) = verdict(public_package());
    assert_eq!((outcome.as_str(), code.as_str()), ("accepted", ""));
    let retained = decode(&retained!("write-verification-report.hex")).expect("report");
    assert_eq!(
        field(&retained, "outcome"),
        &Value::Text("accepted".to_owned())
    );
}

#[test]
fn the_mutations_are_refused_under_the_echo_dpo_1_profile() {
    let profile = include_bytes!("../resources/target-profile.echo-dpo.cbor").to_vec();
    let mut lowering = lowering_request();
    lowering.target_profile = bound("echo.dpo@1", "edict.target-profile/v1", profile.clone());
    assert!(lowerer_refusal(lowering).contains("must match the selected target profile"));
    let mut verification = verification_request(lowering_request(), public_package());
    verification.target_profile =
        transport(bound("echo.dpo@1", "edict.target-profile/v1", profile));
    assert!(
        verifier_refusal(verification).contains("catalog is not the one the target profile pins")
    );
}
