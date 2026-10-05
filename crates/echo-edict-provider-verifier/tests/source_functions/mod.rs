// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deliberately rebound native-provider tests, not compiler acceptance evidence.
use super::*;

fn fixture_with_functions(
    functions: CanonicalValueV1,
) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    edited_fixture(|core, _| add_field(core, "functions", functions))
}

fn edited_fixture(
    edit: impl FnOnce(&mut CanonicalValueV1, &mut CanonicalValueV1),
) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    let (names, mut fixture, package) = pure_relation::compiler_fixture();
    let mut core = decode_canonical_cbor_v1(&fixture.core).expect("Core");
    let mut target = decode_canonical_cbor_v1(&fixture.target_ir).expect("Target");
    edit(&mut core, &mut target);
    fixture.core = canonical_bytes(&core);
    let core_reference = raw_ref(names.application, "edict.core.module/v1", &fixture.core);
    *map_field_mut(map_field_mut(&mut target, "semanticClosure"), "sourceCore") = map([
        ("id", text(names.application)),
        (
            "digest",
            CanonicalValueV1::Array(vec![
                text("sha256"),
                CanonicalValueV1::Bytes(core_reference.digest.clone()),
            ]),
        ),
    ]);
    fixture.target_ir = canonical_bytes(&target);
    // Construct the independent verifier request without asking the lowerer to
    // endorse this changed Core. Rebind the retained package's exact carriers.
    let mut package = decode_canonical_cbor_v1(&package).expect("package");
    let CanonicalValueV1::Bytes(bytes) = map_field(&package, "program") else {
        panic!("program")
    };
    let mut program = decode_canonical_cbor_v1(bytes).expect("program");
    for (name, bytes) in [
        ("core_artifact", &fixture.core),
        ("target_ir_artifact", &fixture.target_ir),
    ] {
        *map_field_mut(&mut program, name) = CanonicalValueV1::Bytes(bytes.clone());
    }
    *map_field_mut(&mut package, "program") = CanonicalValueV1::Bytes(canonical_bytes(&program));
    let budget = map_field(
        map_field(map_field(&core, "intents"), names.intent),
        "coreEvaluationBudget",
    );
    for (package_key, core_key) in [
        ("max_steps", "maxSteps"),
        ("max_allocated_bytes", "maxAllocatedBytes"),
        ("max_output_bytes", "maxOutputBytes"),
    ] {
        *map_field_mut(map_field_mut(&mut package, "budget_ceiling"), package_key) =
            map_field(budget, core_key).clone();
    }
    let closure = map_field_mut(&mut package, "semantic_closure");
    *map_field_mut(closure, "canonical_meaning_identity") =
        CanonicalValueV1::Bytes(core_reference.digest.clone());
    *map_field_mut(closure, "core_identity") = CanonicalValueV1::Bytes(core_reference.digest);
    *map_field_mut(closure, "target_ir_identity") = CanonicalValueV1::Bytes(
        raw_ref(
            "echo.span-ir/v1",
            "edict.target-ir.artifact/v1",
            &fixture.target_ir,
        )
        .digest,
    );
    (names, fixture, canonical_bytes(&package))
}

#[test]
fn lowerer_rejects_unused_malformed_source_function_authority() {
    let (names, fixture, _) = fixture_with_functions(map([("unused", CanonicalValueV1::Null)]));
    assert!(
        lowerer::lower(lowering_request(names, &fixture)).is_err(),
        "lowerer ignored authenticated malformed source function"
    );
}

#[test]
fn verifier_independently_rejects_unused_malformed_source_function_authority() {
    let (names, fixture, package) =
        fixture_with_functions(map([("unused", CanonicalValueV1::Null)]));
    match verifier::verify(verification_request(names, &fixture, package)) {
        Err(_) => {}
        Ok(success) => {
            let report =
                decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
            assert_ne!(
                text_field(&report, "outcome"),
                Some("accepted"),
                "verifier ignored authenticated malformed source function"
            );
        }
    }
}

#[test]
fn valid_unused_source_function_has_coherent_package_carriers() {
    let function = map([
        ("params", CanonicalValueV1::Array(vec![])),
        ("returnType", text("U32")),
        (
            "body",
            map([
                ("locals", CanonicalValueV1::Array(vec![])),
                ("bindings", CanonicalValueV1::Array(vec![])),
                (
                    "result",
                    map([
                        ("kind", text("const")),
                        (
                            "value",
                            map([
                                ("kind", text("int")),
                                ("width", text("U32")),
                                ("value", integer(1)),
                            ]),
                        ),
                    ]),
                ),
            ]),
        ),
    ]);
    let (names, fixture, package) = fixture_with_functions(map([("unused", function)]));
    let lower =
        lowerer::lower(lowering_request(names, &fixture)).expect("valid unused definition lowers");
    assert_eq!(
        lower.outputs[0].artifact.bytes, package,
        "independently rebound carriers match real lowering"
    );
    let success = verifier::verify(verification_request(names, &fixture, package))
        .expect("valid source authority");
    let report = decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
    assert_eq!(text_field(&report, "outcome"), Some("accepted"));
}

#[test]
fn stale_canonical_meaning_identity_is_a_carrier_disagreement_control() {
    let (names, fixture, package) =
        fixture_with_functions(map([("unused", CanonicalValueV1::Null)]));
    let mut package = decode_canonical_cbor_v1(&package).expect("package");
    *map_field_mut(
        map_field_mut(&mut package, "semantic_closure"),
        "canonical_meaning_identity",
    ) = CanonicalValueV1::Bytes(vec![0; 32]);
    if let Ok(success) = verifier::verify(verification_request(
        names,
        &fixture,
        canonical_bytes(&package),
    )) {
        let report = decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
        assert_ne!(text_field(&report, "outcome"), Some("accepted"));
    }
}

fn add_field(value: &mut CanonicalValueV1, name: &str, replacement: CanonicalValueV1) {
    let CanonicalValueV1::Map(fields) = value else {
        panic!("map")
    };
    assert!(!fields.iter().any(|(key, _)| key == &text(name)));
    fields.push((text(name), replacement));
}
fn list(values: Vec<CanonicalValueV1>) -> CanonicalValueV1 {
    CanonicalValueV1::Array(values)
}
fn local(id: &str, ty: &str) -> CanonicalValueV1 {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(local: &CanonicalValueV1) -> CanonicalValueV1 {
    map([("kind", text("local")), ("ref", local.clone())])
}
fn call(name: &str, args: Vec<CanonicalValueV1>) -> CanonicalValueV1 {
    map([
        ("kind", text("call")),
        ("callee", text(name)),
        ("args", list(args)),
        ("typeArgs", list(vec![])),
    ])
}
fn literal(width: &str, n: i128) -> CanonicalValueV1 {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text(width)),
                ("value", CanonicalValueV1::Integer(n)),
            ]),
        ),
    ])
}
fn function(
    params: Vec<CanonicalValueV1>,
    ty: &str,
    locals: Vec<CanonicalValueV1>,
    bindings: Vec<CanonicalValueV1>,
    result: CanonicalValueV1,
) -> CanonicalValueV1 {
    map([
        ("params", list(params)),
        ("returnType", text(ty)),
        (
            "body",
            map([
                ("locals", list(locals)),
                ("bindings", list(bindings)),
                ("result", result),
            ]),
        ),
    ])
}
fn intent(artifact: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    map_field_mut(map_field_mut(artifact, "intents"), "replaceRange")
}
fn called_fixture(
    edit: impl FnOnce(&mut CanonicalValueV1, &mut CanonicalValueV1),
) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    edited_fixture(|core, target| {
        let coordinate = text_field(core, "coordinate")
            .expect("coordinate")
            .to_owned();
        let a = local("arg.0", "U64");
        let b = local("arg.1", "U64");
        let value = local("local.0", "U64");
        let binding = map([
            ("kind", text("let")),
            ("binding", value.clone()),
            (
                "value",
                call(&format!("{coordinate}.passNext"), vec![reference(&a)]),
            ),
        ]);
        let condition = map([
            ("kind", text("if")),
            (
                "predicate",
                map([
                    ("kind", text("compare")),
                    ("op", text("==")),
                    ("left", reference(&a)),
                    ("right", reference(&b)),
                ]),
            ),
            ("then", literal("U32", 1)),
            ("else", literal("U32", 0)),
        ]);
        add_field(
            core,
            "functions",
            map([
                (
                    "pass",
                    function(
                        vec![a.clone()],
                        "U64",
                        vec![value.clone()],
                        vec![binding],
                        reference(&value),
                    ),
                ),
                (
                    "passNext",
                    function(vec![a.clone()], "U64", vec![], vec![], reference(&a)),
                ),
                (
                    "empty",
                    function(vec![a, b], "U32", vec![], vec![], condition),
                ),
            ]),
        );
        for (artifact, key) in [(&mut *core, "nodes"), (&mut *target, "pureBindings")] {
            let body = if key == "nodes" {
                map_field_mut(intent(artifact), "body")
            } else {
                intent(artifact)
            };
            let CanonicalValueV1::Array(bindings) = map_field_mut(body, key) else {
                panic!("bindings")
            };
            let original = map_field(&bindings[0], "value").clone();
            *map_field_mut(&mut bindings[0], "value") =
                call(&format!("{coordinate}.pass"), vec![original]);
            let predicate = map_field(map_field(&bindings[1], "value"), "predicate");
            let arguments = vec![
                map_field(predicate, "left").clone(),
                map_field(predicate, "right").clone(),
            ];
            *map_field_mut(&mut bindings[1], "value") =
                call(&format!("{coordinate}.empty"), arguments);
        }
        edit(core, target);
    })
}
fn verifier_accepts(names: FixtureNames<'_>, fixture: &RawFixture, package: Vec<u8>) -> bool {
    verifier::verify(verification_request(names, fixture, package)).is_ok_and(|success| {
        let report = decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
        text_field(&report, "outcome") == Some("accepted")
    })
}
fn both_refuse(names: FixtureNames<'_>, fixture: &RawFixture, package: Vec<u8>, case: &str) {
    let lower = lowerer::lower(lowering_request(names, fixture)).is_ok();
    let verify = verifier_accepts(names, fixture, package);
    assert!(
        !lower && !verify,
        "{case}: lowerer accepted={lower}, verifier accepted={verify}"
    );
}

#[test]
fn source_functions_called_forward_bindings_conditions_and_imported_arguments_are_admitted() {
    let (names, fixture, package) = called_fixture(|_, _| {});
    let lowered =
        lowerer::lower(lowering_request(names, &fixture)).expect("called source functions lower");
    assert_eq!(
        lowered.outputs[0].artifact.bytes, package,
        "independent carrier construction"
    );
    assert!(verifier_accepts(names, &fixture, package));
}

#[test]
fn source_functions_unused_bodies_have_fresh_complete_typed_lexical_frames() {
    for case in [
        "duplicate_parameter",
        "capture",
        "return_type",
        "unknown_callee",
        "recursion",
        "unsupported_bool",
        "unused_local",
        "forward_local",
        "partial_subtract",
    ] {
        let (names, fixture, package) = called_fixture(|core, _| {
            let coordinate = text_field(core, "coordinate")
                .expect("coordinate")
                .to_owned();
            let a = local("arg.0", "U64");
            let b = local("arg.1", "U64");
            let saved = local("local.0", "U64");
            let mut definition = function(vec![a.clone()], "U64", vec![], vec![], reference(&a));
            match case {
                "duplicate_parameter" => {
                    *map_field_mut(&mut definition, "params") = list(vec![a.clone(), a]);
                }
                "capture" => {
                    *map_field_mut(map_field_mut(&mut definition, "body"), "result") =
                        reference(&b);
                }
                "return_type" => *map_field_mut(&mut definition, "returnType") = text("U32"),
                "unknown_callee" | "recursion" => {
                    let callee = if case == "recursion" {
                        "unused"
                    } else {
                        "missing"
                    };
                    *map_field_mut(map_field_mut(&mut definition, "body"), "result") =
                        call(&format!("{coordinate}.{callee}"), vec![reference(&a)]);
                }
                "unsupported_bool" => *map_field_mut(&mut definition, "returnType") = text("Bool"),
                "unused_local" => {
                    *map_field_mut(map_field_mut(&mut definition, "body"), "locals") =
                        list(vec![saved]);
                }
                "forward_local" => {
                    *map_field_mut(map_field_mut(&mut definition, "body"), "locals") =
                        list(vec![saved.clone()]);
                    *map_field_mut(map_field_mut(&mut definition, "body"), "bindings") =
                        list(vec![map([
                            ("kind", text("let")),
                            ("binding", saved.clone()),
                            ("value", reference(&saved)),
                        ])]);
                }
                "partial_subtract" => {
                    *map_field_mut(&mut definition, "params") = list(vec![a.clone(), b.clone()]);
                    let mut subtraction =
                        call("core.integer.subtract", vec![reference(&a), reference(&b)]);
                    *map_field_mut(&mut subtraction, "typeArgs") = list(vec![text("U64")]);
                    *map_field_mut(map_field_mut(&mut definition, "body"), "result") = subtraction;
                }
                _ => panic!("case"),
            }
            add_field(map_field_mut(core, "functions"), "unused", definition);
        });
        both_refuse(names, &fixture, package, case);
    }
}

#[test]
fn source_functions_core_calls_require_the_exact_declared_signature() {
    for case in ["arity", "argument_type", "generic"] {
        let (names, fixture, package) = called_fixture(|core, target| {
            for (artifact, key) in [(core, "nodes"), (target, "pureBindings")] {
                let body = if key == "nodes" {
                    map_field_mut(intent(artifact), "body")
                } else {
                    intent(artifact)
                };
                let CanonicalValueV1::Array(bindings) = map_field_mut(body, key) else {
                    panic!("bindings")
                };
                let expression = map_field_mut(&mut bindings[0], "value");
                match case {
                    "arity" => *map_field_mut(expression, "args") = list(vec![]),
                    "argument_type" => {
                        *map_field_mut(expression, "args") = list(vec![literal("U32", 1)]);
                    }
                    "generic" => *map_field_mut(expression, "typeArgs") = list(vec![text("U64")]),
                    _ => panic!("case"),
                }
            }
        });
        both_refuse(names, &fixture, package, case);
    }
}

#[test]
fn source_functions_independent_relation_rejects_coherently_rebound_target_call_substitution() {
    let (names, fixture, package) = called_fixture(|_, target| {
        let CanonicalValueV1::Array(bindings) = map_field_mut(intent(target), "pureBindings")
        else {
            panic!("bindings")
        };
        *map_field_mut(map_field_mut(&mut bindings[0], "value"), "args") =
            list(vec![literal("U64", 7)]);
    });
    assert!(
        !verifier_accepts(names, &fixture, package),
        "valid hashes cannot endorse a different call argument"
    );
}

#[test]
fn source_functions_pure_intent_inventory_is_complete_and_unique() {
    for case in ["missing", "extra", "duplicate"] {
        let (names, fixture, package) = called_fixture(|core, _| {
            let CanonicalValueV1::Array(locals) =
                map_field_mut(map_field_mut(intent(core), "body"), "locals")
            else {
                panic!("locals")
            };
            match case {
                "missing" => {
                    locals.pop().expect("declared let");
                }
                "extra" => locals.push(local("ghost", "U64")),
                "duplicate" => locals.push(locals[0].clone()),
                _ => panic!("case"),
            }
        });
        both_refuse(names, &fixture, package, case);
    }
}

#[test]
fn source_functions_combined_source_import_depth_counts_shared_suffixes_in_both_orders() {
    for count in [63, 64] {
        for shallow_first in [false, true] {
            let (names, fixture, package) = edited_fixture(|core, _| {
                let coordinate = text_field(core, "coordinate")
                    .expect("coordinate")
                    .to_owned();
                let CanonicalValueV1::Array(nodes) =
                    map_field(map_field(intent(core), "body"), "nodes")
                else {
                    panic!("nodes")
                };
                let imported = map_field(&nodes[0], "value").clone();
                let name = |index: usize| {
                    format!(
                        "f{:03}",
                        if shallow_first {
                            count - 1 - index
                        } else {
                            index
                        }
                    )
                };
                let mut functions = Vec::new();
                for index in 0..count {
                    let result = if index + 1 == count {
                        imported.clone()
                    } else {
                        call(&format!("{coordinate}.{}", name(index + 1)), vec![])
                    };
                    functions.push((
                        text(name(index)),
                        function(vec![], "U64", vec![], vec![], result),
                    ));
                }
                add_field(core, "functions", CanonicalValueV1::Map(functions));
            });
            if count == 63 {
                lowerer::lower(lowering_request(names, &fixture)).expect("exact combined 64 depth");
                assert!(
                    verifier_accepts(names, &fixture, package),
                    "exact depth with shallow_first={shallow_first}"
                );
            } else {
                both_refuse(
                    names,
                    &fixture,
                    package,
                    "over combined source/import depth",
                );
            }
        }
    }
}

fn diamond(core: &mut CanonicalValueV1, layers: usize) -> String {
    let coordinate = text_field(core, "coordinate")
        .expect("coordinate")
        .to_owned();
    let mut functions = Vec::new();
    for layer in 0..=layers {
        let body = if layer == layers {
            function(vec![], "U64", vec![], vec![], literal("U64", 1))
        } else {
            let a = local("local.0", "U64");
            let b = local("local.1", "U64");
            let binding = |local: &CanonicalValueV1| {
                map([
                    ("kind", text("let")),
                    ("binding", local.clone()),
                    (
                        "value",
                        call(&format!("{coordinate}.f{:03}", layer + 1), vec![]),
                    ),
                ])
            };
            function(
                vec![],
                "U64",
                vec![a.clone(), b.clone()],
                vec![binding(&a), binding(&b)],
                reference(&a),
            )
        };
        functions.push((text(format!("f{layer:03}")), body));
    }
    add_field(core, "functions", CanonicalValueV1::Map(functions));
    format!("{coordinate}.f000")
}

#[test]
fn source_functions_diamond_cost_counts_occurrences_and_refuses_u64_storage_overflow() {
    // Helper storage B(n)=2*B(n-1)+64, B(0)=64. B(57)=2^64-64
    // fits, B(58)=2^65-64 does not. Both graphs fit the depth aperture.
    for layers in [57, 58] {
        let (names, fixture, package) = edited_fixture(|core, _| {
            diamond(core, layers);
        });
        if layers == 57 {
            lowerer::lower(lowering_request(names, &fixture))
                .expect("checked u64 storage just below overflow");
            assert!(verifier_accepts(names, &fixture, package));
        } else {
            both_refuse(names, &fixture, package, "diamond cost overflow");
        }
    }
}

#[test]
fn source_functions_called_diamond_must_fit_the_whole_operation_budget() {
    for layers in [0, 20] {
        let (names, fixture, package) = edited_fixture(|core, target| {
            let root = diamond(core, layers);
            for (artifact, key) in [(core, "nodes"), (target, "pureBindings")] {
                let body = if key == "nodes" {
                    map_field_mut(intent(artifact), "body")
                } else {
                    intent(artifact)
                };
                let CanonicalValueV1::Array(bindings) = map_field_mut(body, key) else {
                    panic!("bindings")
                };
                *map_field_mut(&mut bindings[0], "value") = call(&root, vec![]);
            }
        });
        if layers == 0 {
            lowerer::lower(lowering_request(names, &fixture))
                .expect("small called body fits operation");
            assert!(verifier_accepts(names, &fixture, package));
        } else {
            both_refuse(
                names,
                &fixture,
                package,
                "each repeated call occurrence must spend budget",
            );
        }
    }
}

#[test]
fn source_functions_coherent_declared_budgets_cannot_omit_entry_and_result_materialization() {
    for key in ["maxSteps", "maxAllocatedBytes"] {
        let (names, fixture, package) = called_fixture(|core, target| {
            for artifact in [core, target] {
                *map_field_mut(map_field_mut(intent(artifact), "coreEvaluationBudget"), key) =
                    integer(1);
            }
        });
        both_refuse(names, &fixture, package, key);
    }
}

#[test]
fn source_functions_signature_value_depth_is_part_of_the_call_aperture() {
    for layers in [63, 64] {
        let (names, fixture, package) = edited_fixture(|core, _| {
            let coordinate = text_field(core, "coordinate")
                .expect("coordinate")
                .to_owned();
            for layer in 0..layers {
                let child = if layer + 1 == layers {
                    "U64".to_owned()
                } else {
                    format!("{coordinate}.Deep{}", layer + 1)
                };
                add_field(
                    map_field_mut(core, "types"),
                    &format!("Deep{layer}"),
                    map([
                        ("kind", text("Record")),
                        ("fields", map([("child", text(child))])),
                    ]),
                );
            }
            let ty = format!("{coordinate}.Deep0");
            let parameter = local("arg.0", &ty);
            add_field(
                core,
                "functions",
                map([(
                    "identity",
                    function(
                        vec![parameter.clone()],
                        &ty,
                        vec![],
                        vec![],
                        reference(&parameter),
                    ),
                )]),
            );
        });
        if layers == 63 {
            lowerer::lower(lowering_request(names, &fixture))
                .expect("63 record levels plus one call frame");
            assert!(verifier_accepts(names, &fixture, package));
        } else {
            both_refuse(
                names,
                &fixture,
                package,
                "record validation would exceed call aperture",
            );
        }
    }
}

#[test]
fn source_functions_pure_input_identity_does_not_depend_on_declaration_inventory_order() {
    let (names, fixture, package) = called_fixture(|core, _| {
        let CanonicalValueV1::Array(locals) =
            map_field_mut(map_field_mut(intent(core), "body"), "locals")
        else {
            panic!("locals")
        };
        assert!(locals.len() > 1);
        locals.reverse();
    });
    let lowered = lowerer::lower(lowering_request(names, &fixture))
        .expect("declaration inventory order carries no execution order");
    assert_eq!(lowered.outputs[0].artifact.bytes, package);
    assert!(verifier_accepts(names, &fixture, package));
}

#[test]
fn source_functions_pure_basis_has_complete_input_only_call_authority() {
    for case in ["none", "valid", "null", "unknown", "arity", "capture"] {
        let (names, fixture, package) = called_fixture(|core, target| {
            let coordinate = text_field(core, "coordinate")
                .expect("coordinate")
                .to_owned();
            let parameter = local("arg.0", "Bytes<exact=32>");
            add_field(
                map_field_mut(core, "functions"),
                "basisValue",
                function(
                    vec![parameter.clone()],
                    "Bytes<exact=32>",
                    vec![],
                    vec![],
                    reference(&parameter),
                ),
            );
            let original = map_field(intent(core), "basis").clone();
            let expression = match case {
                "none" | "null" => CanonicalValueV1::Null,
                "valid" => call(&format!("{coordinate}.basisValue"), vec![original]),
                "unknown" => call(&format!("{coordinate}.missing"), vec![]),
                "arity" => call(&format!("{coordinate}.basisValue"), vec![]),
                "capture" => reference(&local("local.0", "U64")),
                _ => panic!("case"),
            };
            for artifact in [core, target] {
                if case == "none" {
                    let CanonicalValueV1::Map(fields) = intent(artifact) else {
                        panic!("intent map")
                    };
                    fields.retain(|(name, _)| name != &text("basis"));
                } else {
                    *map_field_mut(intent(artifact), "basis") = expression.clone();
                }
            }
        });
        if ["none", "valid"].contains(&case) {
            let lowered = lowerer::lower(lowering_request(names, &fixture))
                .expect("coherent valid optional basis");
            assert_eq!(lowered.outputs[0].artifact.bytes, package);
            assert!(verifier_accepts(names, &fixture, package));
        } else {
            both_refuse(names, &fixture, package, case);
        }
    }
}

mod nominal_types;
