// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The public `ContentAddressV1` identity program under `echo.dpo@2`: the
//! native lowerer reproduces the retained public package from the request
//! the compiler sent, and the separate verifier accepts it. The runtime half,
//! against the six frozen vectors, is in warp-core's
//! `public_content_address_effect`.
#![allow(clippy::expect_used, clippy::panic)]
//! The public lowerer and verifier routes for an authored effect program with
//! an effect step under the `echo.dpo@2` effect target profile (#985).
//!
//! The package, report, lawpack, adapter and configuration under
//! `crates/warp-core/tests/fixtures/edict-effect-step` are unchanged public
//! compiler output: the program reads one atom through
//! `echo.dpo@2.node-atom-read`. The lowering request is rebuilt from them and
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
                "../../warp-core/tests/fixtures/edict-content-address/",
                $name
            ))
            .trim(),
        )
        .expect("retained public bytes")
    };
}

const SOURCE: &str = include_str!(
    "../../warp-core/tests/fixtures/edict-content-address/source/ContentAddress.edict"
);
const EFFECT_PROFILE: &[u8] = include_bytes!("../resources/target-profile.echo-dpo-2.cbor");
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const MODULE: &str = "consumer.identity.program@1";
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

fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    &mut entries
        .iter_mut()
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
    retained!("identity-executable-operation-package.hex")
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
    let target = decode(bytes(field(&bundle, "target_ir_artifact"))).expect("public Target IR");
    let Value::Text(target_coordinate) = field(&target, "domain") else {
        panic!("public Target IR domain")
    };
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
                "consumer.identity.echo-adapter/v1",
                "edict.lawpack-adapter/v1",
                retained!("identity-adapter.hex"),
            ),
            auxiliary(
                "02-lawpack-exports",
                "lawpack-exports",
                "consumer.identity.exports/v1",
                "edict.lawpack-exports/v1",
                embedded("lawpack_exports_artifact"),
            ),
            SemanticInput {
                role: "03-lawpack".to_owned(),
                kind: SemanticInputKind::Lawpack,
                artifact: bound(
                    "consumer.identity@1",
                    "edict.lawpack/v1",
                    retained!("identity-lawpack.hex"),
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
                retained!("identity-configuration.hex"),
            ),
            auxiliary(
                "06-target-ir",
                "target-ir",
                target_coordinate,
                TARGET_IR,
                embedded("target_ir_artifact"),
            ),
            auxiliary(
                "07-result-projection",
                "result-projection",
                "consumer.identity.program@1.ids",
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

#[test]
fn the_native_lowerer_reproduces_the_public_identity_package() {
    let mut response = lower(lowering_request()).expect("the public route lowers the program");
    let output = response.outputs.remove(0);
    assert_eq!(output.artifact.domain, PACKAGE_DOMAIN);
    assert_eq!(output.artifact.bytes, public_package());
}

#[test]
fn the_separate_verifier_accepts_the_public_identity_package() {
    let (outcome, code, _) = verdict(public_package());
    assert_eq!((outcome.as_str(), code.as_str()), ("accepted", ""));
}

#[test]
fn the_separate_verifier_rejects_a_substituted_package() {
    let other = hex::decode(
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-structured/structured-executable-operation-package.hex"
        )
        .trim(),
    )
    .expect("structured package");
    let (outcome, _, _) = verdict(other);
    assert_eq!(outcome, "rejected");
}

#[test]
fn both_routes_refuse_a_substituted_catalog_without_the_identity_contracts() {
    // Oracle: the catalog is pinned by the selected profile. A configuration
    // carrying a catalog without the identity contracts is a substituted
    // artifact, and both routes refuse it.
    let mut request = lowering_request();
    let configuration = &mut request.semantic_inputs[4].artifact;
    let mut value = decode(&configuration.artifact.bytes).expect("configuration");
    let mut catalog = decode(bytes(field(&value, "intrinsicCatalog"))).expect("catalog");
    let Value::Map(entries) = field_mut(&mut catalog, "intrinsics") else {
        panic!("intrinsics")
    };
    entries.retain(|(key, _)| {
        !matches!(key, Value::Text(name) if name.contains("contentAddress") || name.contains("EdgeId"))
    });
    *field_mut(&mut value, "intrinsicCatalog") =
        Value::Bytes(encode(&catalog).expect("catalog bytes"));
    *configuration = bound(
        &configuration.reference.coordinate.clone(),
        "echo.operation-lowering-configuration/v1",
        encode(&value).expect("configuration bytes"),
    );
    assert!(lower(request.clone()).is_err(), "lowerer");
    assert!(
        verifier::verify(verification_request(request, public_package())).map_or(
            true,
            |response| {
                let report = decode(&response.outputs[0].artifact.bytes).expect("report");
                field(&report, "outcome") != &Value::Text("accepted".to_owned())
            }
        ),
        "verifier"
    );
}
