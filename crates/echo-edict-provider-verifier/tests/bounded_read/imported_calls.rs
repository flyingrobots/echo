// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic mixed read/pure authority; retained export body, not public compilation.
use super::*;

fn retained_unsigned_export() -> Value {
    let raw =
        hex::decode(include_str!("../fixtures/compiler-produced-pure/package.cbor.hex").trim())
            .expect("retained public compiler package");
    let package = decode(&raw).expect("package");
    let Value::Bytes(program) = field(&package, "program") else {
        panic!("program")
    };
    let program = decode(program).expect("program");
    let Value::Bytes(exports) = field(&program, "lawpack_exports_artifact") else {
        panic!("exports")
    };
    let exports = decode(exports).expect("exports");
    let Value::Array(functions) = field(&exports, "pureFunctions") else {
        panic!("functions")
    };
    let [function] = functions.as_slice() else {
        panic!("one retained helper")
    };
    assert_eq!(
        field(function, "coordinate"),
        &text("jedit.text@1.maxCreatedLeafCount")
    );
    assert_eq!(field(function, "returnType"), &text("U64"));
    assert_eq!(field(function, "parameterTypes"), &Value::Array(vec![]));
    function.clone()
}

fn add_imported_authority(
    source: &mut LoweringRequestV1,
    core: &mut Value,
    target: &mut Value,
    authored: &mut String,
) {
    let mut exports =
        decode(&source.semantic_inputs[1].artifact.artifact.bytes).expect("read exports");
    let Value::Array(functions) = field_mut(&mut exports, "pureFunctions") else {
        panic!("functions")
    };
    assert!(functions.is_empty());
    functions.push(retained_unsigned_export());
    let exports_id = source.semantic_inputs[1]
        .artifact
        .reference
        .coordinate
        .clone();
    let exports_owner = digest(&exports_id, &exports).expect("exports owner identity");
    source.semantic_inputs[1].artifact = bound(
        &exports_id,
        "edict.lawpack-exports/v1",
        encode(&exports).expect("exports bytes"),
    );
    let mut lawpack = decode(&source.semantic_inputs[2].artifact.artifact.bytes).expect("lawpack");
    *field_mut(field_mut(&mut lawpack, "exports"), "digest") =
        Value::Array(vec![text("sha256"), Value::Bytes(exports_owner.into())]);
    let old = format!(
        "sha256:{}",
        hex::encode(&source.semantic_inputs[2].artifact.reference.digest.bytes)
    );
    source.semantic_inputs[2].artifact = bound(
        "jedit.text@1",
        "edict.lawpack/v1",
        encode(&lawpack).expect("lawpack bytes"),
    );
    let identity = source.semantic_inputs[2]
        .artifact
        .reference
        .digest
        .bytes
        .clone();
    let lawpack_ref = map([
        ("id", text("jedit.text@1")),
        (
            "digest",
            Value::Array(vec![text("sha256"), Value::Bytes(identity.clone())]),
        ),
    ]);
    let Value::Array(imports) = field_mut(core, "imports") else {
        panic!("imports")
    };
    *field_mut(&mut imports[0], "ref") = lawpack_ref.clone();
    *field_mut(field_mut(target, "semanticClosure"), "lawpacks") = Value::Array(vec![lawpack_ref]);
    assert_eq!(authored.matches(&old).count(), 1);
    *authored = authored.replace(&old, &format!("sha256:{}", hex::encode(identity)));
}

fn imported_call_request(same_package: bool, unknown: bool) -> LoweringRequestV1 {
    let mut source = read_helper();
    let mut core = decode(&source.core.artifact.bytes).expect("core");
    let mut target = decode(&source.semantic_inputs[5].artifact.artifact.bytes).expect("target");
    let mut projection =
        decode(&source.semantic_inputs[6].artifact.artifact.bytes).expect("projection");
    let Value::Bytes(authored) =
        decode(&source.semantic_inputs[3].artifact.artifact.bytes).expect("source")
    else {
        panic!("source bytes")
    };
    let mut authored = String::from_utf8(authored).expect("source UTF-8");
    add_imported_authority(&mut source, &mut core, &mut target, &mut authored);
    let member = if unknown {
        "unknownImportedHelper"
    } else {
        "maxCreatedLeafCount"
    };
    let binding = local("source.imported", "U64");
    let call = map([
        ("kind", text("call")),
        ("callee", text(&format!("jedit.text@1.{member}"))),
        ("args", Value::Array(vec![])),
        ("typeArgs", Value::Array(vec![])),
    ]);
    let body = field_mut(intent(&mut core), "body");
    let Value::Array(locals) = field_mut(body, "locals") else {
        panic!("locals")
    };
    locals.push(binding.clone());
    let Value::Array(nodes) = field_mut(body, "nodes") else {
        panic!("nodes")
    };
    nodes.push(map([
        ("kind", text("let")),
        ("binding", binding.clone()),
        ("value", call.clone()),
    ]));
    let target_intent = intent(&mut target);
    let Value::Array(bindings) = field_mut(target_intent, "pureBindings") else {
        panic!("bindings")
    };
    bindings.push(map([
        ("id", text("replaceRange.binding.imported")),
        ("binding", binding),
        ("value", call),
    ]));
    let Value::Array(order) = field_mut(target_intent, "executionOrder") else {
        panic!("order")
    };
    order.push(text("replaceRange.binding.imported"));
    assert_eq!(authored.matches("intent replaceRange(").count(), 1);
    authored = authored.replace("intent replaceRange(",
        "fn retain(value: text.FactBytes) -> text.FactBytes { let bytes = value; return bytes; }\nintent replaceRange(");
    assert_eq!(authored.matches("  return actual;").count(), 1);
    authored = authored.replace(
        "  return actual;",
        &format!(
        "  let retained = retain(actual);\n  let imported = text.{member}();\n  return retained;"),
    );
    let previous = source.core.reference.coordinate.clone();
    let coordinate = if same_package {
        "jedit.text@1"
    } else {
        &previous
    };
    if same_package {
        for value in [&mut core, &mut target, &mut projection] {
            rename_owned_coordinates(value, &previous, coordinate);
        }
        // This isolates call ownership using the supported relative Core table
        // spelling. It does not establish exact-key-first type lookup or public
        // compiler compatibility for a same-package imported type closure.
        let Value::Map(types) = field_mut(&mut core, "types") else {
            panic!("types")
        };
        let prefix = format!("{coordinate}.");
        let mut names = std::collections::BTreeSet::new();
        for (key, definition) in types {
            let Value::Text(name) = key else {
                panic!("type name")
            };
            if let Some(relative) = name.strip_prefix(&prefix) {
                assert_ne!(field(definition, "kind"), &text("Nominal"));
                *name = relative.to_owned();
            }
            assert!(names.insert(name.clone()));
        }
        let header = format!("package {previous};");
        assert_eq!(authored.matches(&header).count(), 1);
        authored = authored.replace(&header, &format!("package {coordinate};"));
    }
    source.core = bound(
        coordinate,
        "edict.core.module/v1",
        encode(&core).expect("core bytes"),
    );
    *field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore") = map([
        ("id", text(coordinate)),
        (
            "digest",
            Value::Array(vec![
                text("sha256"),
                Value::Bytes(source.core.reference.digest.bytes.clone()),
            ]),
        ),
    ]);
    source.semantic_inputs[3].artifact = bound(
        coordinate,
        "edict.source/v1",
        encode(&Value::Bytes(authored.into_bytes())).expect("source bytes"),
    );
    source.semantic_inputs[5].artifact = bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        encode(&target).expect("target bytes"),
    );
    source.semantic_inputs[6].artifact = bound(
        &format!("{coordinate}.replaceRange"),
        "edict.result-projection.artifact/v1",
        encode(&projection).expect("projection bytes"),
    );
    source
}

fn independently_rebound_request(candidate: LoweringRequestV1) -> verifier::VerificationRequestV1 {
    let admitted = imported_call_request(false, false);
    for index in [0, 1, 2, 4] {
        assert_eq!(
            admitted.semantic_inputs[index].artifact, candidate.semantic_inputs[index].artifact,
            "package naming or unknown call cannot change imported authority"
        );
    }
    // Only the separate-package positive is lowered. Same-package verification
    // must remain observable even while its independent lowerer control is RED.
    let mut verification = request(admitted);
    verification.core = transport(candidate.core.clone());
    verification.target_ir = transport(candidate.semantic_inputs[5].artifact.clone());
    for input in &mut verification.semantic_inputs {
        if let Some(replacement) = candidate
            .semantic_inputs
            .iter()
            .find(|value| value.role == input.role)
        {
            input.artifact = transport(replacement.artifact.clone());
        }
    }
    change_package(&mut verification, |package| {
        *field_mut(package, "operation_coordinate") = text(&format!(
            "{}.replaceRange",
            candidate.core.reference.coordinate
        ));
        let closure = field_mut(package, "semantic_closure");
        for (key, artifact) in [
            ("canonical_meaning_identity", &candidate.core),
            ("core_identity", &candidate.core),
            ("target_ir_identity", &candidate.semantic_inputs[5].artifact),
            (
                "edict_source_identity",
                &candidate.semantic_inputs[3].artifact,
            ),
        ] {
            *field_mut(closure, key) = Value::Bytes(artifact.reference.digest.bytes.clone());
        }
        let Value::Bytes(raw) = field(package, "program") else {
            panic!("program bytes")
        };
        let mut program = decode(raw).expect("program");
        for (key, artifact) in [
            ("core_artifact", &candidate.core),
            ("target_ir_artifact", &candidate.semantic_inputs[5].artifact),
            ("source_artifact", &candidate.semantic_inputs[3].artifact),
            (
                "result_projection_artifact",
                &candidate.semantic_inputs[6].artifact,
            ),
        ] {
            *field_mut(&mut program, key) = Value::Bytes(artifact.artifact.bytes.clone());
        }
        *field_mut(package, "program") = Value::Bytes(encode(&program).expect("program bytes"));
    });
    verification
}

#[test]
fn separate_package_direct_imported_pure_call_is_admitted() {
    let source = imported_call_request(false, false);
    lower(source.clone()).expect("separate-package direct pure call lowers");
    let response = verifier::verify(request(source)).expect("separate-package independent report");
    let report = decode(&response.outputs[0].artifact.bytes).expect("report");
    assert_eq!(field(&report, "outcome"), &text("accepted"));
}

#[test]
fn lowerer_same_package_imported_pure_call_is_not_claimed_by_source_prefix() {
    lower(imported_call_request(true, false))
        .expect("exact source membership permits disjoint imported helper");
}

#[test]
fn verifier_same_package_imported_pure_call_is_not_claimed_by_source_prefix() {
    let verification = independently_rebound_request(imported_call_request(true, false));
    let response = verifier::verify(verification).expect("coherently rebound same-package report");
    let report = decode(&response.outputs[0].artifact.bytes).expect("report");
    assert_eq!(field(&report, "outcome"), &text("accepted"));
}

#[test]
fn an_unknown_qualified_pure_call_remains_rejected_by_both_judgments() {
    let source = imported_call_request(true, true);
    let refusal = lower(source.clone()).expect_err("unknown imported coordinate is not authority");
    assert_eq!(
        refusal.kind,
        echo_edict_provider_lowerer::ProviderRefusalKind::UnsupportedSemantics
    );
    assert_rejected(independently_rebound_request(source));
}
