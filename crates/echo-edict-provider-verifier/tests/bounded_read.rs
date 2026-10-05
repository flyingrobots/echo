// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Verify read packages independently from their lowerer.

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

fn pair() -> LoweringRequestV1 {
    let mut request = fixture();
    macro_rules! raw {
        ($name:literal) => {
            hex::decode(
                include_str!(concat!("fixtures/node-atom-read/pair/", $name, ".hex")).trim(),
            )
            .expect("pair bytes")
        };
    }
    request.core = bound(
        "jedit.text.replace_range@1",
        "edict.core.module/v1",
        raw!("core"),
    );
    for (index, bytes) in [
        (3, raw!("04-source")),
        (5, raw!("06-target-ir")),
        (6, raw!("07-result-projection")),
    ] {
        let artifact = &mut request.semantic_inputs[index].artifact;
        *artifact = bound(
            &artifact.reference.coordinate,
            &artifact.artifact.domain,
            bytes,
        );
    }
    request
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

fn request(mut source: LoweringRequestV1) -> verifier::VerificationRequestV1 {
    let package = lower(source.clone())
        .expect("native lowerer produces witness")
        .outputs
        .remove(0)
        .artifact
        .bytes;
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
        artifact: transport(bound("executable-operation-package.echo", PACKAGE, package)),
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

fn mutate(value: &mut Value, key: &str, edit: impl FnOnce(&mut Value)) {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    edit(
        &mut entries
            .iter_mut()
            .find(|(name, _)| name == &Value::Text(key.to_owned()))
            .expect("field")
            .1,
    );
}

fn change_package(request: &mut verifier::VerificationRequestV1, edit: impl FnOnce(&mut Value)) {
    let artifact = &mut request
        .semantic_inputs
        .last_mut()
        .expect("package input")
        .artifact;
    let mut package = decode(&artifact.artifact.bytes).expect("package");
    edit(&mut package);
    *artifact = transport(bound(
        "executable-operation-package.echo",
        PACKAGE,
        encode(&package).expect("changed package"),
    ));
}

fn changed_target(request: &mut verifier::VerificationRequestV1, edit: impl FnOnce(&mut Value)) {
    let mut target = decode(&request.target_ir.artifact.bytes).expect("target");
    mutate(&mut target, "intents", |intents| {
        mutate(intents, "replaceRange", edit);
    });
    bind_target(request, &target);
}

fn bind_target(request: &mut verifier::VerificationRequestV1, target: &Value) {
    let bytes = encode(target).expect("changed target bytes");
    request.target_ir = transport(bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        bytes.clone(),
    ));
    let identity = request.target_ir.reference.digest.bytes.clone();
    change_package(request, |package| {
        mutate(package, "semantic_closure", |closure| {
            mutate(closure, "target_ir_identity", |value| {
                *value = Value::Bytes(identity);
            });
        });
        mutate(package, "program", |program| {
            let Value::Bytes(encoded) = program else {
                panic!("program bytes")
            };
            let mut body = decode(encoded).expect("program");
            mutate(&mut body, "target_ir_artifact", |target| {
                *target = Value::Bytes(bytes);
            });
            *program = Value::Bytes(encode(&body).expect("changed program"));
        });
    });
}

fn assert_rejected(request: verifier::VerificationRequestV1) {
    let response =
        verifier::verify(request).expect("well-bound supported relation receives a report");
    let report = decode(&response.outputs[0].artifact.bytes).expect("report");
    assert_eq!(
        field(&report, "outcome"),
        &Value::Text("rejected".to_owned())
    );
}

#[test]
fn independent_verifier_accepts_single_and_pair_and_binds_exact_report_subjects() {
    for source in [fixture(), pair()] {
        let request = request(source);
        let response = verifier::verify(request.clone()).expect("independent read verification");
        assert!(response.diagnostics.is_empty());
        let report = decode(&response.outputs[0].artifact.bytes).expect("report");
        assert_eq!(
            field(&report, "outcome"),
            &Value::Text("accepted".to_owned())
        );
        for (key, expected) in [
            ("package", &request.semantic_inputs[6].artifact),
            ("targetIr", &request.target_ir),
            (
                "applicationResultProjection",
                &request.semantic_inputs[5].artifact,
            ),
        ] {
            let reference = field(&report, key);
            assert_eq!(
                field(reference, "id"),
                &Value::Text(expected.reference.coordinate.clone())
            );
            assert_eq!(
                field(reference, "digest"),
                &Value::Array(vec![
                    Value::Text("sha256".to_owned()),
                    Value::Bytes(expected.reference.digest.bytes.clone())
                ])
            );
        }
        assert_eq!(
            response,
            verifier::verify(request).expect("repeat verification")
        );
    }
}

#[test]
fn self_consistent_rebound_package_and_target_cannot_rewrite_core_order_or_guards() {
    for mutation in 0..6 {
        let mut request = request(pair());
        changed_target(&mut request, |intent| match mutation {
            0 => mutate(intent, "executionOrder", |value| {
                let Value::Array(ids) = value else {
                    panic!("order")
                };
                ids.swap(1, 2);
            }),
            1 => mutate(intent, "executionOrder", |value| {
                let Value::Array(ids) = value else {
                    panic!("order")
                };
                ids[1] = ids[0].clone();
            }),
            2 => mutate(intent, "requirements", |value| {
                let Value::Array(nodes) = value else {
                    panic!("requirements")
                };
                mutate(&mut nodes[0], "predicate", |predicate| {
                    mutate(predicate, "op", |op| *op = Value::Text("<=".to_owned()));
                });
            }),
            3 => mutate(intent, "steps", |value| {
                let Value::Array(nodes) = value else {
                    panic!("steps")
                };
                mutate(&mut nodes[0], "targetIntrinsic", |intrinsic| {
                    *intrinsic = Value::Text("native.application.read".to_owned());
                });
            }),
            4 => mutate(intent, "steps", |value| {
                let Value::Array(nodes) = value else {
                    panic!("steps")
                };
                mutate(&mut nodes[1], "input", |input| {
                    mutate(input, "field", |name| {
                        *name = Value::Text("firstAddress".to_owned());
                    });
                });
            }),
            _ => mutate(intent, "steps", |value| {
                let Value::Array(nodes) = value else {
                    panic!("steps")
                };
                mutate(&mut nodes[0], "obstructionArms", |arms| {
                    mutate(arms, "echo.atom-missing/v1", |arm| {
                        mutate(arm, "value", |call| {
                            mutate(call, "callee", |callee| {
                                *callee = Value::Text("text.FactMalformed".to_owned());
                            });
                        });
                    });
                });
            }),
        });
        assert_rejected(request);
    }
}

#[test]
fn rebound_package_cannot_drop_closure_members_or_substitute_authority() {
    for field_name in [
        "authority_profile_identity",
        "footprint_contract_identity",
        "interpreter_profile_identity",
    ] {
        let mut request = request(fixture());
        change_package(&mut request, |package| {
            mutate(package, field_name, |value| {
                *value = Value::Bytes(vec![0; 32]);
            });
        });
        assert_rejected(request);
    }
    let mut request = request(fixture());
    change_package(&mut request, |package| {
        mutate(package, "program", |program| {
            let Value::Bytes(bytes) = program else {
                panic!("program")
            };
            let mut value = decode(bytes).expect("program");
            let Value::Map(entries) = &mut value else {
                panic!("map")
            };
            entries.retain(|(key, _)| key != &Value::Text("lawpack_adapter_artifact".to_owned()));
            *program = Value::Bytes(encode(&value).expect("changed program"));
        });
    });
    assert_rejected(request);
}

#[test]
fn report_respects_host_output_budget() {
    let mut request = request(fixture());
    request.limits.max_total_response_bytes = 1;
    assert!(verifier::verify(request).is_err());
}

#[test]
fn exact_response_ceiling_counts_roles_domains_and_rejection_diagnostics() {
    let accepted = request(fixture());
    let mut rejected = accepted.clone();
    change_package(&mut rejected, |package| {
        mutate(package, "authority_profile_identity", |value| {
            *value = Value::Bytes(vec![0; 32]);
        });
    });
    for mut request in [accepted, rejected] {
        let response = verifier::verify(request.clone()).expect("baseline report");
        let total: usize = response
            .outputs
            .iter()
            .map(|output| {
                output.role.len() + output.artifact.domain.len() + output.artifact.bytes.len()
            })
            .chain(response.diagnostics.iter().map(|diagnostic| {
                diagnostic.code.len()
                    + diagnostic.message.len()
                    + diagnostic.repair.as_ref().map_or(0, String::len)
            }))
            .sum();
        request.limits.max_total_response_bytes = total as u64;
        assert!(verifier::verify(request.clone()).is_ok());
        request.limits.max_total_response_bytes -= 1;
        assert!(
            verifier::verify(request).is_err(),
            "metadata bytes escaped the host ceiling"
        );
    }
}

fn changed_core(request: &mut verifier::VerificationRequestV1, edit: impl FnOnce(&mut Value)) {
    let mut core = decode(&request.core.artifact.bytes).expect("Core");
    edit(&mut core);
    let bytes = encode(&core).expect("changed Core");
    request.core = transport(bound(
        "jedit.text.replace_range@1",
        "edict.core.module/v1",
        bytes.clone(),
    ));
    let identity = request.core.reference.digest.bytes.clone();
    let mut target = decode(&request.target_ir.artifact.bytes).expect("Target");
    mutate(&mut target, "semanticClosure", |closure| {
        mutate(closure, "sourceCore", |source| {
            mutate(source, "digest", |digest| {
                *digest = Value::Array(vec![
                    Value::Text("sha256".to_owned()),
                    Value::Bytes(identity.clone()),
                ]);
            });
        });
    });
    bind_target(request, &target);
    change_package(request, |package| {
        mutate(package, "semantic_closure", |closure| {
            for key in ["core_identity", "canonical_meaning_identity"] {
                mutate(closure, key, |value| {
                    *value = Value::Bytes(identity.clone());
                });
            }
        });
        mutate(package, "program", |program| {
            let Value::Bytes(encoded) = program else {
                panic!("program")
            };
            let mut value = decode(encoded).expect("program");
            mutate(&mut value, "core_artifact", |core| {
                *core = Value::Bytes(bytes);
            });
            *program = Value::Bytes(encode(&value).expect("changed program"));
        });
    });
}

#[test]
fn coherent_core_target_and_package_cannot_promote_a_failure_local_into_success_scope() {
    let mut request = request(fixture());
    let core = decode(&request.core.artifact.bytes).expect("Core");
    let Value::Array(declarations) = field(
        field(field(field(&core, "intents"), "replaceRange"), "body"),
        "locals",
    ) else {
        panic!("locals")
    };
    let binder = declarations
        .iter()
        .find(|local| field(local, "id") == &Value::Text("obstruction.0".to_owned()))
        .expect("failure binder")
        .clone();
    // Selecting its node ID keeps the comparison well typed if this failure
    // local were incorrectly promoted into the successful execution scope.
    let escaped = Value::Map(vec![
        (
            Value::Text("kind".to_owned()),
            Value::Text("field".to_owned()),
        ),
        (
            Value::Text("field".to_owned()),
            Value::Text("nodeId".to_owned()),
        ),
        (
            Value::Text("base".to_owned()),
            Value::Map(vec![
                (
                    Value::Text("kind".to_owned()),
                    Value::Text("local".to_owned()),
                ),
                (Value::Text("ref".to_owned()), binder),
            ]),
        ),
    ]);
    changed_target(&mut request, |intent| {
        mutate(intent, "requirements", |requirements| {
            let Value::Array(requirements) = requirements else {
                panic!("requirements")
            };
            mutate(&mut requirements[0], "predicate", |predicate| {
                mutate(predicate, "left", |left| {
                    *left = escaped.clone();
                });
            });
        });
    });
    changed_core(&mut request, |core| {
        mutate(core, "intents", |intents| {
            mutate(intents, "replaceRange", |intent| {
                mutate(intent, "body", |body| {
                    mutate(body, "nodes", |nodes| {
                        let Value::Array(nodes) = nodes else {
                            panic!("nodes")
                        };
                        mutate(&mut nodes[1], "predicate", |predicate| {
                            mutate(predicate, "left", |left| {
                                *left = escaped;
                            });
                        });
                    });
                });
            });
        });
    });
    assert_rejected(request);
}

#[test]
fn coherent_type_rebinding_cannot_silently_change_an_imported_byte_bound() {
    let mut request = request(fixture());
    changed_core(&mut request, |core| {
        mutate(core, "types", |types| {
            mutate(types, "jedit.text@1.FactBytes", |ty| {
                mutate(ty, "max", |value| *value = Value::Integer(64));
            });
        });
    });
    assert_rejected(request);
}

#[test]
fn rebound_projection_cannot_return_a_different_read_result() {
    let mut request = request(pair());
    let artifact = &mut request.semantic_inputs[5].artifact;
    let mut projection = decode(&artifact.artifact.bytes).expect("projection");
    mutate(&mut projection, "expression", |expression| {
        mutate(expression, "source", |source| {
            mutate(source, "stepId", |id| {
                *id = Value::Text("replaceRange.step.0".to_owned());
            });
        });
    });
    let bytes = encode(&projection).expect("rebound projection");
    *artifact = transport(bound(
        &artifact.reference.coordinate,
        &artifact.artifact.domain,
        bytes.clone(),
    ));
    change_package(&mut request, |package| {
        mutate(package, "program", |program| {
            let Value::Bytes(encoded) = program else {
                panic!("program")
            };
            let mut value = decode(encoded).expect("program");
            mutate(&mut value, "result_projection_artifact", |projection| {
                *projection = Value::Bytes(bytes);
            });
            *program = Value::Bytes(encode(&value).expect("rebound program"));
        });
    });
    assert_rejected(request);
}

#[test]
fn coherent_core_target_and_package_budgets_must_still_match_the_adapter() {
    let mut request = request(fixture());
    changed_core(&mut request, |core| {
        mutate(core, "intents", |intents| {
            mutate(intents, "replaceRange", |intent| {
                mutate(intent, "coreEvaluationBudget", |budget| {
                    mutate(budget, "maxSteps", |value| *value = Value::Integer(1));
                });
            });
        });
    });
    changed_target(&mut request, |intent| {
        mutate(intent, "coreEvaluationBudget", |budget| {
            mutate(budget, "maxSteps", |value| *value = Value::Integer(1));
        });
    });
    change_package(&mut request, |package| {
        mutate(package, "budget_ceiling", |budget| {
            mutate(budget, "max_steps", |value| *value = Value::Integer(1));
        });
    });
    assert_rejected(request);
}

#[test]
fn input_identity_does_not_depend_on_local_declaration_order() {
    let mut request = request(fixture());
    changed_core(&mut request, |core| {
        mutate(core, "intents", |intents| {
            mutate(intents, "replaceRange", |intent| {
                mutate(intent, "body", |body| {
                    mutate(body, "locals", |locals| {
                        let Value::Array(locals) = locals else {
                            panic!("locals")
                        };
                        locals.rotate_left(1);
                    });
                });
            });
        });
    });
    let response = verifier::verify(request).expect("reordered declaration verification");
    let report = decode(&response.outputs[0].artifact.bytes).expect("report");
    assert_eq!(
        field(&report, "outcome"),
        &Value::Text("accepted".to_owned())
    );
}
