// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Lower the actual compiler request for the proposed bounded read profile.

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use echo_edict_provider_lowerer::{
    lower, Artifact, BoundArtifact, Digest, DigestAlgorithm, LoweringOutputKind,
    LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1, ResourceRef, ResponseLimitsV1,
    SemanticInput, SemanticInputKind,
};

const PACKAGE: &str = "echo.operation-package/v1";

fn bound(coordinate: &str, domain: &str, bytes: Vec<u8>) -> BoundArtifact {
    let value = decode(&bytes).expect("compiler artifact is canonical");
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: digest(domain, &value).expect("artifact identity").to_vec(),
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

fn fixture() -> LoweringRequestV1 {
    macro_rules! raw {
        ($name:literal) => {
            hex::decode(include_str!(concat!("fixtures/node-atom-read/", $name, ".hex")).trim())
                .expect("captured compiler bytes")
        };
    }
    let inputs = [
        (
            "01-lawpack-adapter",
            SemanticInputKind::Auxiliary("lawpack-adapter".to_owned()),
            "jedit.text.echo-adapter/v1",
            "edict.lawpack-adapter/v1",
            raw!("01-lawpack-adapter"),
        ),
        (
            "02-lawpack-exports",
            SemanticInputKind::Auxiliary("lawpack-exports".to_owned()),
            "jedit.text.exports/v1",
            "edict.lawpack-exports/v1",
            raw!("02-lawpack-exports"),
        ),
        (
            "03-lawpack",
            SemanticInputKind::Lawpack,
            "jedit.text@1",
            "edict.lawpack/v1",
            raw!("03-lawpack"),
        ),
        (
            "04-source",
            SemanticInputKind::Auxiliary("edict-source".to_owned()),
            "jedit.text.replace_range@1",
            "edict.source/v1",
            raw!("04-source"),
        ),
        (
            "05-target-configuration",
            SemanticInputKind::Auxiliary("target-configuration".to_owned()),
            "echo.operation-lowering-configuration/v1",
            "echo.operation-lowering-configuration/v1",
            raw!("05-target-configuration"),
        ),
        (
            "06-target-ir",
            SemanticInputKind::Auxiliary("target-ir".to_owned()),
            "echo.span-ir/v2",
            "edict.target-ir.artifact/v1",
            raw!("06-target-ir"),
        ),
        (
            "07-result-projection",
            SemanticInputKind::Auxiliary("result-projection".to_owned()),
            "jedit.text.replace_range@1.replaceRange",
            "edict.result-projection.artifact/v1",
            raw!("07-result-projection"),
        ),
    ];
    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: bound(
            "jedit.text.replace_range@1",
            "edict.core.module/v1",
            raw!("core"),
        ),
        target_profile: bound("echo.dpo@1", "edict.target-profile/v1", raw!("profile")),
        semantic_inputs: inputs
            .into_iter()
            .map(|(role, kind, coordinate, domain, bytes)| SemanticInput {
                role: role.to_owned(),
                kind,
                artifact: bound(coordinate, domain, bytes),
            })
            .collect(),
        requested_outputs: vec![LoweringOutputRequest {
            role: "executable-operation-package.echo".to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE.to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 1,
            max_total_response_bytes: 16 * 1024 * 1024,
            max_diagnostic_count: 8,
        },
    }
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(entries) = value else {
        panic!("map required")
    };
    &entries
        .iter()
        .find(|(key, _)| key == &Value::Text(name.to_owned()))
        .expect("field required")
        .1
}

#[test]
fn captured_compiler_read_request_lowers_without_native_application_semantics() {
    let request = fixture();
    let expected_core = request.core.artifact.bytes.clone();
    let expected_target = request.semantic_inputs[5].artifact.artifact.bytes.clone();
    let repeated = lower(request.clone()).expect("repeatable native lowering");
    let output = lower(request).expect("bounded read configuration is supported");
    assert_eq!(output.outputs, repeated.outputs);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.outputs.len(), 1);
    let package = decode(&output.outputs[0].artifact.bytes).expect("canonical package");
    assert_eq!(
        field(&package, "package_kind"),
        &Value::Text("compiler-produced-bounded-read/v1".to_owned())
    );
    let Value::Bytes(program) = field(&package, "program") else {
        panic!("program bytes")
    };
    let program = decode(program).expect("canonical program");
    assert_eq!(
        field(&program, "core_artifact"),
        &Value::Bytes(expected_core)
    );
    assert_eq!(
        field(&program, "target_ir_artifact"),
        &Value::Bytes(expected_target)
    );
    for name in [
        "lawpack_artifact",
        "lawpack_adapter_artifact",
        "target_configuration_artifact",
        "source_artifact",
    ] {
        assert!(matches!(field(&program, name), Value::Bytes(_)));
    }
}

fn mutate(value: &mut Value, name: &str, edit: impl FnOnce(&mut Value)) {
    let Value::Map(entries) = value else {
        panic!("map required")
    };
    edit(
        &mut entries
            .iter_mut()
            .find(|(key, _)| key == &Value::Text(name.to_owned()))
            .expect("field required")
            .1,
    );
}

fn change_target(request: &mut LoweringRequestV1, edit: impl FnOnce(&mut Value)) {
    let artifact = &mut request.semantic_inputs[5].artifact;
    let mut value = decode(&artifact.artifact.bytes).expect("target artifact");
    mutate(&mut value, "intents", |intents| {
        mutate(intents, "replaceRange", edit);
    });
    *artifact = bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        encode(&value).expect("changed target"),
    );
}

#[test]
fn correctly_rebound_target_mutations_do_not_change_authored_order_or_meaning() {
    for mutation in 0..4 {
        let mut request = fixture();
        change_target(&mut request, |intent| match mutation {
            0 => mutate(intent, "executionOrder", |order| {
                let Value::Array(ids) = order else {
                    panic!("order")
                };
                ids.reverse();
            }),
            1 => mutate(intent, "executionOrder", |order| {
                let Value::Array(ids) = order else {
                    panic!("order")
                };
                ids[1] = ids[0].clone();
            }),
            2 => mutate(intent, "steps", |steps| {
                let Value::Array(steps) = steps else {
                    panic!("steps")
                };
                mutate(&mut steps[0], "targetIntrinsic", |value| {
                    *value = Value::Text("native.application.read".to_owned());
                });
            }),
            _ => mutate(intent, "requirements", |requirements| {
                let Value::Array(requirements) = requirements else {
                    panic!("requirements")
                };
                mutate(&mut requirements[0], "predicate", |value| {
                    *value = Value::Map(vec![(
                        Value::Text("kind".to_owned()),
                        Value::Text("true".to_owned()),
                    )]);
                });
            }),
        });
        assert!(
            lower(request).is_err(),
            "target mutation {mutation} was accepted"
        );
    }
}

#[test]
fn core_byte_bounds_cannot_disagree_with_the_imported_export() {
    let mut request = fixture();
    let mut core = decode(&request.core.artifact.bytes).expect("Core artifact");
    mutate(&mut core, "types", |types| {
        mutate(types, "jedit.text@1.FactBytes", |ty| {
            mutate(ty, "max", |max| *max = Value::Integer(64));
        });
    });
    request.core = bound(
        "jedit.text.replace_range@1",
        "edict.core.module/v1",
        encode(&core).expect("changed Core"),
    );
    let reference = Value::Map(vec![
        (
            Value::Text("id".to_owned()),
            Value::Text(request.core.reference.coordinate.clone()),
        ),
        (
            Value::Text("digest".to_owned()),
            Value::Array(vec![
                Value::Text("sha256".to_owned()),
                Value::Bytes(request.core.reference.digest.bytes.clone()),
            ]),
        ),
    ]);
    let target = &mut request.semantic_inputs[5].artifact;
    let mut value = decode(&target.artifact.bytes).expect("Target artifact");
    mutate(&mut value, "semanticClosure", |closure| {
        mutate(closure, "sourceCore", |source| *source = reference);
    });
    *target = bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        encode(&value).expect("rebound Target"),
    );
    assert!(
        lower(request).is_err(),
        "imported byte bounds were silently redefined"
    );
}

#[test]
fn host_response_ceiling_prevents_publication_of_an_oversized_package() {
    let mut request = fixture();
    request.limits.max_total_response_bytes = 1;
    assert!(lower(request).is_err());
}

#[test]
fn compiler_authored_pair_keeps_independent_read_producers_and_ordered_guards() {
    let mut request = fixture();
    macro_rules! pair {
        ($name:literal) => {
            hex::decode(
                include_str!(concat!("fixtures/node-atom-read/pair/", $name, ".hex")).trim(),
            )
            .expect("captured two-read compiler bytes")
        };
    }
    request.core = bound(
        "jedit.text.replace_range@1",
        "edict.core.module/v1",
        pair!("core"),
    );
    for (index, bytes) in [
        (3, pair!("04-source")),
        (5, pair!("06-target-ir")),
        (6, pair!("07-result-projection")),
    ] {
        let artifact = &mut request.semantic_inputs[index].artifact;
        *artifact = bound(
            &artifact.reference.coordinate,
            &artifact.artifact.domain,
            bytes,
        );
    }
    let output = lower(request.clone()).expect("both authored reads lower");
    let package = decode(&output.outputs[0].artifact.bytes).expect("canonical package");
    let Value::Bytes(program) = field(&package, "program") else {
        panic!("program")
    };
    let program = decode(program).expect("canonical program");
    assert_eq!(
        field(&program, "target_ir_artifact"),
        &Value::Bytes(request.semantic_inputs[5].artifact.artifact.bytes.clone())
    );
    change_target(&mut request, |intent| {
        mutate(intent, "executionOrder", |order| {
            let Value::Array(ids) = order else {
                panic!("order")
            };
            assert_eq!(ids.len(), 4);
            ids.swap(1, 2);
        });
    });
    assert!(
        lower(request).is_err(),
        "second read moved ahead of the first guard"
    );
}
