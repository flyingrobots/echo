// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native, synthetically rebound read-provider coverage; not public compilation.
use super::*;

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(name, value)| (text(name), value))
            .collect(),
    )
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(local: &Value) -> Value {
    map([("kind", text("local")), ("ref", local.clone())])
}
fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    &mut entries
        .iter_mut()
        .find(|(key, _)| key == &text(name))
        .expect("field")
        .1
}
fn insert(value: &mut Value, name: &str, replacement: Value) {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    assert!(!entries.iter().any(|(key, _)| key == &text(name)));
    entries.push((text(name), replacement));
}
fn intent(value: &mut Value) -> &mut Value {
    field_mut(field_mut(value, "intents"), "replaceRange")
}
fn read_helper() -> LoweringRequestV1 {
    read_helper_with_budget(4_194_304)
}
fn read_helper_with_budget(max_steps: u64) -> LoweringRequestV1 {
    let mut source = fixture();
    let mut core = decode(&source.core.artifact.bytes).expect("core");
    let mut target = decode(&source.semantic_inputs[5].artifact.artifact.bytes).expect("target");
    let mut projection =
        decode(&source.semantic_inputs[6].artifact.artifact.bytes).expect("projection");
    let Value::Text(coordinate) = field(&core, "coordinate") else {
        panic!("coordinate")
    };
    let coordinate = coordinate.clone();
    let Value::Text(output_type) = field(intent(&mut core), "output") else {
        panic!("output type")
    };
    let output_type = output_type.clone();
    let parameter = local("arg.0", &output_type);
    let helper_local = local("local.0", &output_type);
    insert(
        &mut core,
        "functions",
        map([(
            "retain",
            map([
                ("params", Value::Array(vec![parameter.clone()])),
                ("returnType", text(&output_type)),
                (
                    "body",
                    map([
                        ("locals", Value::Array(vec![helper_local.clone()])),
                        (
                            "bindings",
                            Value::Array(vec![map([
                                ("kind", text("let")),
                                ("binding", helper_local.clone()),
                                ("value", reference(&parameter)),
                            ])]),
                        ),
                        ("result", reference(&helper_local)),
                    ]),
                ),
            ]),
        )]),
    );
    let saved = local("source.return", &output_type);
    let body = field_mut(intent(&mut core), "body");
    let call = map([
        ("kind", text("call")),
        ("callee", text(&format!("{coordinate}.retain"))),
        ("args", Value::Array(vec![field(body, "result").clone()])),
        ("typeArgs", Value::Array(vec![])),
    ]);
    let Value::Array(locals) = field_mut(body, "locals") else {
        panic!("locals")
    };
    locals.push(saved.clone());
    let Value::Array(nodes) = field_mut(body, "nodes") else {
        panic!("nodes")
    };
    nodes.push(map([
        ("kind", text("let")),
        ("binding", saved.clone()),
        ("value", call.clone()),
    ]));
    *field_mut(body, "result") = reference(&saved);
    let target_intent = intent(&mut target);
    let binding = map([
        ("id", text("replaceRange.binding.0")),
        ("binding", saved.clone()),
        ("value", call),
    ]);
    let Value::Map(entries) = target_intent else {
        panic!("intent")
    };
    match entries
        .iter_mut()
        .find(|(key, _)| key == &text("pureBindings"))
    {
        Some((_, Value::Array(bindings))) => bindings.push(binding),
        None => entries.push((text("pureBindings"), Value::Array(vec![binding]))),
        _ => panic!("pure bindings"),
    }
    let Value::Array(order) = field_mut(target_intent, "executionOrder") else {
        panic!("order")
    };
    order.push(text("replaceRange.binding.0"));
    *field_mut(target_intent, "result") = reference(&saved);
    *field_mut(&mut projection, "expression") = map([
        ("kind", text("source")),
        (
            "source",
            map([
                ("kind", text("pureBinding")),
                ("bindingId", text("replaceRange.binding.0")),
            ]),
        ),
        ("path", Value::Array(vec![])),
    ]);
    // This synthetic fixture explicitly increases its declared step ceiling.
    // At FactBytes.max=1 MiB, read work plus the required byte equality already
    // exceeds the legacy 1 MiB step ceiling. Allocation stays at its old 16 MiB.
    for artifact in [&mut core, &mut target] {
        *field_mut(
            field_mut(intent(artifact), "coreEvaluationBudget"),
            "maxSteps",
        ) = Value::Integer(i128::from(max_steps));
    }
    fork_adapter_budget(&mut source, &mut core, &mut target, max_steps);
    source.core = bound(
        &coordinate,
        "edict.core.module/v1",
        encode(&core).expect("core bytes"),
    );
    *field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore") = map([
        ("id", text(&coordinate)),
        (
            "digest",
            Value::Array(vec![
                text("sha256"),
                Value::Bytes(source.core.reference.digest.bytes.clone()),
            ]),
        ),
    ]);
    for (index, value) in [(5, target), (6, projection)] {
        let artifact = &mut source.semantic_inputs[index].artifact;
        *artifact = bound(
            &artifact.reference.coordinate,
            &artifact.artifact.domain,
            encode(&value).expect("changed artifact"),
        );
    }
    source
}

#[test]
fn bounded_read_source_function_is_admitted_after_a_real_read_producer() {
    let source = read_helper();
    let lowered = lower(source.clone()).expect("read plus pure source helper lowers");
    assert_eq!(
        lowered,
        lower(source.clone()).expect("repeat native lowering")
    );
    let verification = request(source);
    let response =
        verifier::verify(verification.clone()).expect("independent read helper verification");
    let report = decode(&response.outputs[0].artifact.bytes).expect("report");
    assert_eq!(field(&report, "outcome"), &text("accepted"));
    assert_eq!(
        response,
        verifier::verify(verification).expect("repeat independent judgment")
    );
}

#[test]
fn bounded_read_source_call_target_substitution_rejects_after_coherent_rebinding() {
    let mut verification = request(read_helper());
    changed_target(&mut verification, |intent| {
        let Value::Array(bindings) = field_mut(intent, "pureBindings") else {
            panic!("bindings")
        };
        // Wrong arity is rehashed into the package and Target, while authenticated
        // Core still calls retain(actual). The lowerer never endorses this change.
        *field_mut(field_mut(&mut bindings[0], "value"), "args") = Value::Array(vec![]);
    });
    if let Ok(response) = verifier::verify(verification) {
        let report = decode(&response.outputs[0].artifact.bytes).expect("report");
        assert_ne!(field(&report, "outcome"), &text("accepted"));
    }
}

// A budget is adapter-owned authority, not a free Core number. This synthetic
// fixture forks the complete adapter/lawpack/source/import closure explicitly.
fn fork_adapter_budget(
    source: &mut LoweringRequestV1,
    core: &mut Value,
    target: &mut Value,
    max_steps: u64,
) {
    let mut adapter = decode(&source.semantic_inputs[0].artifact.artifact.bytes).expect("adapter");
    let Value::Map(profiles) = field(&adapter, "operationProfiles") else {
        panic!("profiles")
    };
    let Value::Text(obligation) = field(&profiles[0].1, "budgetObligation") else {
        panic!("budget obligation")
    };
    let obligation = obligation.clone();
    *field_mut(
        field_mut(field_mut(&mut adapter, "budgets"), &obligation),
        "maxSteps",
    ) = Value::Integer(i128::from(max_steps));
    let adapter_id = source.semantic_inputs[0]
        .artifact
        .reference
        .coordinate
        .clone();
    let owner_identity = digest(&adapter_id, &adapter)
        .expect("adapter owner identity")
        .to_vec();
    source.semantic_inputs[0].artifact = bound(
        &adapter_id,
        "edict.lawpack-adapter/v1",
        encode(&adapter).expect("adapter bytes"),
    );
    let mut lawpack = decode(&source.semantic_inputs[2].artifact.artifact.bytes).expect("lawpack");
    let Value::Array(adapters) = field_mut(&mut lawpack, "targetAdapters") else {
        panic!("adapters")
    };
    *field_mut(field_mut(&mut adapters[0], "adapter"), "digest") =
        Value::Array(vec![text("sha256"), Value::Bytes(owner_identity)]);
    let old_digest = source.semantic_inputs[2]
        .artifact
        .reference
        .digest
        .bytes
        .clone();
    source.semantic_inputs[2].artifact = bound(
        "jedit.text@1",
        "edict.lawpack/v1",
        encode(&lawpack).expect("lawpack bytes"),
    );
    let new_digest = source.semantic_inputs[2]
        .artifact
        .reference
        .digest
        .bytes
        .clone();
    let lawpack_ref = map([
        ("id", text("jedit.text@1")),
        (
            "digest",
            Value::Array(vec![text("sha256"), Value::Bytes(new_digest.clone())]),
        ),
    ]);
    let Value::Array(imports) = field_mut(core, "imports") else {
        panic!("imports")
    };
    *field_mut(&mut imports[0], "ref") = lawpack_ref.clone();
    *field_mut(field_mut(target, "semanticClosure"), "lawpacks") = Value::Array(vec![lawpack_ref]);
    let Value::Bytes(authored) =
        decode(&source.semantic_inputs[3].artifact.artifact.bytes).expect("source")
    else {
        panic!("source bytes")
    };
    let authored = String::from_utf8(authored).expect("UTF-8 source");
    let old = format!("sha256:{}", hex::encode(old_digest));
    assert_eq!(
        authored.matches(&old).count(),
        1,
        "one imported lawpack pin"
    );
    let authored = authored.replace(&old, &format!("sha256:{}", hex::encode(new_digest)));
    source.semantic_inputs[3].artifact = bound(
        "jedit.text.replace_range@1",
        "edict.source/v1",
        encode(&Value::Bytes(authored.into_bytes())).expect("source bytes"),
    );
}

#[test]
fn bounded_read_source_helper_refuses_the_original_insufficient_step_ceiling() {
    let rejected = read_helper_with_budget(1_048_576);
    let refusal =
        lower(rejected.clone()).expect_err("whole-operation cost exceeds the original authority");
    assert_eq!(
        refusal.kind,
        echo_edict_provider_lowerer::ProviderRefusalKind::UnsupportedSemantics
    );
    // Build only the valid positive through the lowerer, then independently
    // replace every affected budget/authority carrier from the rejected request.
    let mut verification = request(read_helper());
    verification.core = transport(rejected.core.clone());
    verification.target_ir = transport(rejected.semantic_inputs[5].artifact.clone());
    for input in &mut verification.semantic_inputs {
        if let Some(replacement) = rejected
            .semantic_inputs
            .iter()
            .find(|candidate| candidate.role == input.role)
        {
            input.artifact = transport(replacement.artifact.clone());
        }
    }
    change_package(&mut verification, |package| {
        *field_mut(field_mut(package, "budget_ceiling"), "max_steps") = Value::Integer(1_048_576);
        let closure = field_mut(package, "semantic_closure");
        for (key, artifact) in [
            ("canonical_meaning_identity", &rejected.core),
            ("core_identity", &rejected.core),
            ("target_ir_identity", &rejected.semantic_inputs[5].artifact),
            ("lawpack_identity", &rejected.semantic_inputs[2].artifact),
            (
                "edict_source_identity",
                &rejected.semantic_inputs[3].artifact,
            ),
        ] {
            *field_mut(closure, key) = Value::Bytes(artifact.reference.digest.bytes.clone());
        }
        let Value::Bytes(raw) = field(package, "program") else {
            panic!("program bytes")
        };
        let mut program = decode(raw).expect("program");
        for (key, artifact) in [
            ("core_artifact", &rejected.core),
            ("target_ir_artifact", &rejected.semantic_inputs[5].artifact),
            (
                "lawpack_adapter_artifact",
                &rejected.semantic_inputs[0].artifact,
            ),
            ("lawpack_artifact", &rejected.semantic_inputs[2].artifact),
            ("source_artifact", &rejected.semantic_inputs[3].artifact),
        ] {
            *field_mut(&mut program, key) = Value::Bytes(artifact.artifact.bytes.clone());
        }
        *field_mut(package, "program") = Value::Bytes(encode(&program).expect("program bytes"));
    });
    if let Ok(response) = verifier::verify(verification) {
        let report = decode(&response.outputs[0].artifact.bytes).expect("report");
        assert_ne!(field(&report, "outcome"), &text("accepted"));
    }
}

fn rename_owned_coordinates(value: &mut Value, previous: &str, replacement: &str) {
    match value {
        Value::Text(name) => {
            if name == previous {
                replacement.clone_into(name);
            } else if let Some(member) = name.strip_prefix(&format!("{previous}.")) {
                *name = format!("{replacement}.{member}");
            }
        }
        Value::Array(values) => {
            for value in values {
                rename_owned_coordinates(value, previous, replacement);
            }
        }
        Value::Map(entries) => {
            for (key, value) in entries {
                rename_owned_coordinates(key, previous, replacement);
                rename_owned_coordinates(value, previous, replacement);
            }
        }
        _ => {}
    }
}

// Only source-owned coordinates move. The actual imported read effect and its
// authenticated exports, adapter, lawpack and configuration stay byte-identical.
fn same_package_source_function(collision: bool) -> LoweringRequestV1 {
    let mut source = read_helper();
    let exports = decode(&source.semantic_inputs[1].artifact.artifact.bytes).expect("exports");
    let Value::Array(effects) = field(&exports, "effects") else {
        panic!("effects")
    };
    let Value::Text(effect) = field(&effects[0], "coordinate") else {
        panic!("effect coordinate")
    };
    let (coordinate, effect_name) = effect.rsplit_once('.').expect("qualified read effect");
    assert_eq!(effect, "jedit.text@1.readFact");
    let previous = source.core.reference.coordinate.clone();
    let mut core = decode(&source.core.artifact.bytes).expect("core");
    let mut target = decode(&source.semantic_inputs[5].artifact.artifact.bytes).expect("target");
    let mut projection =
        decode(&source.semantic_inputs[6].artifact.artifact.bytes).expect("projection");
    for value in [&mut core, &mut target, &mut projection] {
        rename_owned_coordinates(value, &previous, coordinate);
    }
    // Core permits module-relative named-type table keys. Once the module is
    // in the imported package, move those definitions to the relative spelling
    // expected by this runtime aperture; keep their qualified references and
    // exact definitions. Do not create duplicate aliases or fork the lawpack.
    let Value::Map(types) = field_mut(&mut core, "types") else {
        panic!("type inventory")
    };
    let prefix = format!("{coordinate}.");
    let mut names = std::collections::BTreeSet::new();
    for (key, definition) in types {
        let Value::Text(name) = key else {
            panic!("type name")
        };
        if let Some(relative) = name.strip_prefix(&prefix) {
            assert_ne!(
                field(definition, "kind"),
                &text("Nominal"),
                "this fixture must not move a nominal authority definition"
            );
            *name = relative.to_owned();
        }
        assert!(names.insert(name.clone()), "type inventory remains unique");
    }
    let unused_name = if collision { effect_name } else { "unusedRead" };
    let functions = field_mut(&mut core, "functions");
    let unused = field(functions, "retain").clone();
    insert(functions, unused_name, unused);
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
    let Value::Bytes(authored) =
        decode(&source.semantic_inputs[3].artifact.artifact.bytes).expect("source")
    else {
        panic!("source bytes")
    };
    let mut authored = String::from_utf8(authored).expect("UTF-8 source");
    let package_header = format!("package {previous};");
    assert_eq!(authored.matches(&package_header).count(), 1);
    authored = authored.replace(&package_header, &format!("package {coordinate};"));
    let helpers = format!(
        "fn retain(value: text.FactBytes) -> text.FactBytes {{ let bytes = value; return bytes; }}\n\
         fn {unused_name}(value: text.FactBytes) -> text.FactBytes {{ let bytes = value; return bytes; }}\n"
    );
    assert_eq!(authored.matches("intent replaceRange(").count(), 1);
    authored = authored.replace(
        "intent replaceRange(",
        &format!("{helpers}intent replaceRange("),
    );
    assert_eq!(authored.matches("  return actual;").count(), 1);
    authored = authored.replace(
        "  return actual;",
        "  let retained = retain(actual);\n  return retained;",
    );
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

fn collision_verification() -> verifier::VerificationRequestV1 {
    let positive = same_package_source_function(false);
    let negative = same_package_source_function(true);
    for index in [0, 1, 2, 4, 6] {
        assert_eq!(
            positive.semantic_inputs[index].artifact, negative.semantic_inputs[index].artifact,
            "changing an unused source name must preserve imported authority and result projection"
        );
    }
    // Only the disjoint control goes through the lowerer. The collision package
    // is assembled independently, so a future lowerer refusal cannot mask a
    // verifier that still accepts the same authenticated ownership ambiguity.
    let mut verification = request(positive);
    verification.core = transport(negative.core.clone());
    verification.target_ir = transport(negative.semantic_inputs[5].artifact.clone());
    verification
        .semantic_inputs
        .iter_mut()
        .find(|input| input.role == "04-source")
        .expect("source input")
        .artifact = transport(negative.semantic_inputs[3].artifact.clone());
    change_package(&mut verification, |package| {
        let closure = field_mut(package, "semantic_closure");
        for (key, artifact) in [
            ("canonical_meaning_identity", &negative.core),
            ("core_identity", &negative.core),
            ("target_ir_identity", &negative.semantic_inputs[5].artifact),
            (
                "edict_source_identity",
                &negative.semantic_inputs[3].artifact,
            ),
        ] {
            *field_mut(closure, key) = Value::Bytes(artifact.reference.digest.bytes.clone());
        }
        let Value::Bytes(raw) = field(package, "program") else {
            panic!("program bytes")
        };
        let mut program = decode(raw).expect("program");
        for (key, artifact) in [
            ("core_artifact", &negative.core),
            ("target_ir_artifact", &negative.semantic_inputs[5].artifact),
            ("source_artifact", &negative.semantic_inputs[3].artifact),
        ] {
            *field_mut(&mut program, key) = Value::Bytes(artifact.artifact.bytes.clone());
        }
        *field_mut(package, "program") = Value::Bytes(encode(&program).expect("program bytes"));
    });
    verification
}

#[test]
fn bounded_read_disjoint_source_functions_share_the_imported_effect_package() {
    let source = same_package_source_function(false);
    lower(source.clone()).expect("same-package disjoint called and unused source functions lower");
    let verification = request(source);
    let response = verifier::verify(verification).expect("same-package independent verification");
    let report = decode(&response.outputs[0].artifact.bytes).expect("report");
    assert_eq!(field(&report, "outcome"), &text("accepted"));
}

#[test]
fn lowerer_source_function_names_cannot_shadow_imported_read_effects() {
    let source = same_package_source_function(true);
    let refusal =
        lower(source).expect_err("unused source function collides with imported read effect");
    assert_eq!(
        refusal.kind,
        echo_edict_provider_lowerer::ProviderRefusalKind::UnsupportedSemantics
    );
}

#[test]
fn verifier_source_function_names_cannot_shadow_imported_read_effects() {
    assert_rejected(collision_verification());
}

#[path = "imported_calls.rs"]
mod imported_calls;
