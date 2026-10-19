// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Current compiler-artifact binding and executable-law preparation.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used, clippy::panic)]
use echo_edict_canonical::CanonicalValueV1 as V;
use warp_core::ExecutableOperationPackage;
use warp_core::TypeId;
#[path = "common/edict_effect_publication.rs"]
mod fixture;
use fixture::*;
#[test]
fn current_package_binds_program_and_executes_its_schema_relations() {
    let p = program();
    let v = package(&p);
    let b = encoded(&v);
    let package = prepare(&v).expect("valid effect package");
    assert_eq!(package.to_canonical_bytes().expect("canonical package"), b);
    assert_eq!(
        package
            .program()
            .identity()
            .expect("program identity")
            .as_hash(),
        hash(b"echo:operation-program:v1\0", &encoded(&p))
    );
    assert!(package
        .program()
        .as_effect_program()
        .expect("effect program")
        .graph_schema()
        .allows_atom(TypeId([1; 32]), TypeId([2; 32])));
    assert!(!package
        .program()
        .as_effect_program()
        .expect("effect program")
        .graph_schema()
        .allows_atom(TypeId([2; 32]), TypeId([1; 32])));
    assert_eq!(
        package
            .program()
            .as_effect_program()
            .expect("effect program")
            .compiled_program(),
        encoded(&compiler_program())
    );
}
#[test]
fn substituted_program_schema_law_and_closure_cannot_keep_the_original_pin() {
    let original = encoded(&package(&program()));
    let pin = hash(b"echo:operation-package:v1\0", &original);
    for mode in 0..4 {
        let mut p = program();
        let mut v = package(&p);
        match mode {
            0 => {
                set(&mut p, "compiled_program", V::Bytes(vec![0xa1]));
                set(&mut v, "program", V::Bytes(encoded(&p)));
            }
            1 => {
                let mut g = graph();
                set(
                    &mut g,
                    "edge_types",
                    V::Map(vec![(id(6), V::Array(vec![]))]),
                );
                set(&mut p, "graph_schema", V::Bytes(encoded(&g)));
                set(&mut v, "program", V::Bytes(encoded(&p)));
            }
            2 => {
                let mut l = law();
                set(&mut l, "max_reads", V::Integer(2));
                set(&mut p, "footprint_law", V::Bytes(encoded(&l)));
                set(&mut v, "program", V::Bytes(encoded(&p)));
                set(&mut v, "footprint_contract_identity", footprint(&l));
            }
            _ => set(&mut v, "operation_coordinate", text("app.other@1")),
        }
        assert!(ExecutableOperationPackage::prepare(&encoded(&v), pin).is_err());
    }
    assert!(ExecutableOperationPackage::prepare(
        &original,
        hash(b"echo:operation-package:v2\0", &original)
    )
    .is_err());
}
#[test]
fn exact_package_and_closure_shapes_refuse_effect_lawpacks_projection_and_extra_fields() {
    for (k, x) in [
        ("schema", text("echo.operation-package/v2")),
        ("program", program()),
    ] {
        let mut v = package(&program());
        set(&mut v, k, x);
        refused(&v);
    }
    for key in ["application_result_projection", "extra"] {
        let mut v = package(&program());
        add(&mut v, key, V::Integer(0));
        refused(&v);
    }
    for key in ["lawpack_coordinate", "lawpack_identity"] {
        let mut v = package(&program());
        let V::Map(m) = &mut v else { panic!("map") };
        let c = &mut m
            .iter_mut()
            .find(|(k, _)| k == &text("semantic_closure"))
            .expect("closure")
            .1;
        add(c, key, id(1));
        refused(&v);
    }
    let mut v = package(&program());
    let V::Map(m) = &mut v else { panic!("map") };
    m.retain(|(k, _)| k != &text("operation_coordinate"));
    refused(&v);
}
#[test]
fn all_package_profile_equalities_and_target_ir_binding_are_enforced() {
    for key in [
        "target_profile_identity",
        "application_basis_schema_identity",
        "input_schema_identity",
        "result_schema_identity",
        "obstruction_schema_identity",
        "result_interpretation_identity",
        "obstruction_interpretation_identity",
        "evaluation_basis_schema_identity",
        "interpreter_profile_identity",
        "intrinsic_profile_identity",
        "footprint_contract_identity",
    ] {
        let mut v = package(&program());
        set(&mut v, key, id(55));
        refused(&v);
    }
    let mut p = program();
    set(&mut p, "target_ir_identity", id(5));
    refused(&package(&p));
    let mut p = program();
    set(&mut p, "target_ir_identity", V::Bytes(vec![4; 31]));
    refused(&package(&p));
}
#[test]
fn program_refuses_unknown_profiles_laws_shapes_and_unbounded_declarations() {
    for (k, x) in [
        ("schema", text("echo.operation-program/v2")),
        ("kind", text("other")),
        ("identity_law", V::Integer(0)),
        ("identity_law", V::Integer(2)),
        ("max_input_bytes", V::Integer(65537)),
        ("max_result_bytes", V::Integer(65537)),
        ("max_allocated_bytes", V::Integer(16 * 1024 * 1024 + 1)),
        ("max_input_bytes", V::Integer(-1)),
        ("output_type", text("")),
        ("output_type", text(&"x".repeat(1025))),
        ("graph_schema", V::Bytes(vec![0xa0])),
        ("footprint_law", V::Bytes(vec![0xa0])),
        ("interpreter_profile_identity", id(44)),
        ("intrinsic_profile_identity", id(44)),
    ] {
        let mut p = program();
        set(&mut p, k, x);
        refused(&package(&p));
    }
    let mut p = program();
    add(&mut p, "extra", V::Integer(0));
    refused(&package(&p));
    let mut v = package(&program());
    set(
        &mut v,
        "budget_ceiling",
        map([
            ("steps", V::Integer(0)),
            ("read_bytes", V::Integer(0)),
            ("write_bytes", V::Integer(0)),
        ]),
    );
    refused(&v);
}
#[test]
fn nested_canonical_bytes_and_exact_footprint_law_digest_are_required() {
    let mut p = program();
    let mut g = encoded(&graph());
    g.push(0);
    set(&mut p, "graph_schema", V::Bytes(g));
    refused(&package(&p));
    let mut p = program();
    let mut l = law();
    set(&mut l, "max_reads", V::Integer(2));
    set(&mut p, "footprint_law", V::Bytes(encoded(&l)));
    refused(&package(&p));
    let mut v = package(&program());
    set(
        &mut v,
        "footprint_contract_identity",
        V::Bytes(hash(b"echo:operation-footprint-law:v1\0", &encoded(&law())).to_vec()),
    );
    refused(&v);
    let mut v = package(&program());
    let mut b = encoded(&program());
    b.push(0);
    set(&mut v, "program", V::Bytes(b));
    refused(&v);
    let mut b = encoded(&package(&program()));
    b.push(0);
    assert!(
        ExecutableOperationPackage::prepare(&b, hash(b"echo:operation-package:v1\0", &b)).is_err()
    );
}
#[test]
fn declaration_boundaries_accept_zero_io_and_allocation_and_preserve_current_coordinate_rules() {
    let mut p = program();
    for key in ["max_input_bytes", "max_result_bytes", "max_allocated_bytes"] {
        set(&mut p, key, V::Integer(0));
    }
    set(&mut p, "output_type", text(&"é".repeat(512)));
    let mut core = compiled_core();
    let V::Map(ref mut core_fields) = core else {
        panic!("core")
    };
    let V::Map(types) = &mut core_fields
        .iter_mut()
        .find(|(key, _)| key == &text("types"))
        .expect("types")
        .1
    else {
        panic!("types")
    };
    types.push((
        text(&"é".repeat(512)),
        map([
            ("kind", text("Nominal")),
            ("contract", text(&"é".repeat(512))),
            ("representation", text("U32")),
        ]),
    ));
    set(&mut core, "coordinate", text(&"x".repeat(1016)));
    let V::Map(ref mut fields) = core else {
        panic!("core map")
    };
    let intents = &mut fields
        .iter_mut()
        .find(|(key, _)| key == &text("intents"))
        .expect("intents")
        .1;
    let V::Map(intents) = intents else {
        panic!("intents map")
    };
    set(&mut intents[0].1, "output", text(&"é".repeat(512)));
    set(
        &mut p,
        "compiled_program",
        V::Bytes(encoded(&compiled_bundle(
            &core,
            &compiled_target(&core),
            &compiled_exports(),
            "update@1",
        ))),
    );
    set(
        &mut p,
        "target_ir_identity",
        artifact_identity("edict.target-ir.artifact/v1", &compiled_target(&core)),
    );
    let mut v = package(&p);
    set(
        &mut v,
        "operation_coordinate",
        text(&format!("{}.update@1", "x".repeat(1016))),
    );
    let prepared = prepare(&v).expect("zero byte limits and current package coordinate shape");
    assert_eq!(
        prepared
            .program()
            .as_effect_program()
            .expect("effect program")
            .max_input_bytes(),
        0
    );
    assert_eq!(
        prepared
            .program()
            .as_effect_program()
            .expect("effect program")
            .max_result_bytes(),
        0
    );
    assert_eq!(
        prepared
            .program()
            .as_effect_program()
            .expect("effect program")
            .max_allocated_bytes(),
        0
    );
    assert_eq!(
        prepared
            .program()
            .as_effect_program()
            .expect("effect program")
            .output_type()
            .len(),
        1024
    );
    assert_eq!(prepared.budget_ceiling().steps(), 1);
    assert_eq!(prepared.budget_ceiling().read_bytes(), 0);
    assert_eq!(prepared.budget_ceiling().write_bytes(), 0);
    let mut p = program();
    set(&mut p, "output_type", text(&"é".repeat(513)));
    refused(&package(&p));
}
#[test]
fn a_new_pin_can_bind_changed_laws_but_cannot_reuse_the_old_contract_identity() {
    let mut p = program();
    let mut changed_law = law();
    set(&mut changed_law, "max_reads", V::Integer(2));
    set(&mut p, "footprint_law", V::Bytes(encoded(&changed_law)));
    let mut v = package(&p);
    refused(&v);
    set(
        &mut v,
        "footprint_contract_identity",
        footprint(&changed_law),
    );
    let prepared = prepare(&v).expect("new pin binds the new exact law");
    assert_ne!(
        prepared
            .program()
            .identity()
            .expect("program identity")
            .as_hash(),
        prepare(&package(&program()))
            .expect("original")
            .program()
            .identity()
            .expect("program identity")
            .as_hash()
    );
}
#[path = "edict_effect_package_tests/invocation.rs"]
mod invocation;

#[test]
fn repinning_cannot_turn_opaque_or_noncanonical_compiler_bytes_into_an_effect_program() {
    // Removing compiled-Core validation admits these malformed programs even
    // after the surrounding publication has a newly matching package pin.
    for compiled in [
        vec![],
        vec![0xa0],
        vec![0xa1],
        vec![0xf6],
        vec![0x18, 0],
        vec![0, 0],
    ] {
        let mut p = program();
        set(&mut p, "compiled_program", V::Bytes(compiled));
        assert!(
            prepare(&package(&p)).is_err(),
            "opaque compiler bytes must refuse even with a new publication pin"
        );
    }
}

#[test]
fn compiler_core_identity_operation_and_output_cannot_be_rebound_by_a_new_package_pin() {
    let p = program();
    for (key, value) in [
        ("operation_coordinate", text("app.other@1")),
        ("operation_coordinate", text("application.update@1")),
    ] {
        let mut value_package = package(&p);
        set(&mut value_package, key, value);
        refused(&value_package);
    }
    let mut p = program();
    set(&mut p, "output_type", text("app.Other"));
    refused(&package(&p));
    let mut v = package(&program());
    let V::Map(ref mut fields) = v else {
        panic!("package")
    };
    let closure = &mut fields
        .iter_mut()
        .find(|(key, _)| key == &text("semantic_closure"))
        .expect("closure")
        .1;
    set(closure, "core_identity", id(33));
    refused(&v);
}

#[test]
fn retained_public_compiler_core_binds_without_a_bridge_specific_instruction_language() {
    // These artifacts were captured during a public Edict JSONL build attempt,
    // rather than manufactured by the bridge fixture builder. This witnesses
    // canonical artifact binding, not a successful build, execution or verifier approval.
    let bytes = hex::decode(
        include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/core.hex")
            .trim(),
    )
    .expect("retained compiler artifact");
    let core = echo_edict_canonical::decode_canonical_cbor_v1(&bytes).expect("Core");
    let V::Map(fields) = &core else {
        panic!("Core map")
    };
    let get = |key| {
        &fields
            .iter()
            .find(|(name, _)| name == &text(key))
            .expect("Core field")
            .1
    };
    let V::Text(coordinate) = get("coordinate") else {
        panic!("coordinate")
    };
    let V::Map(intents) = get("intents") else {
        panic!("intents")
    };
    let [(V::Text(name), V::Map(intent))] = intents.as_slice() else {
        panic!("one intent")
    };
    let output = intent
        .iter()
        .find(|(key, _)| key == &text("output"))
        .expect("output")
        .1
        .clone();
    let mut p = program();
    let target = echo_edict_canonical::decode_canonical_cbor_v1(
        &hex::decode(
            include_str!(
                "../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/06-target-ir.hex"
            )
            .trim(),
        )
        .expect("target bytes"),
    )
    .expect("Target IR");
    let exports=echo_edict_canonical::decode_canonical_cbor_v1(&hex::decode(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/02-lawpack-exports.hex").trim()).expect("exports bytes")).expect("exports");
    let bundle_bytes = encoded(&compiled_bundle(&core, &target, &exports, name));
    set(&mut p, "compiled_program", V::Bytes(bundle_bytes.clone()));
    set(
        &mut p,
        "target_ir_identity",
        artifact_identity("edict.target-ir.artifact/v1", &target),
    );
    set(&mut p, "output_type", output);
    let mut v = package(&p);
    set(
        &mut v,
        "operation_coordinate",
        text(&format!("{coordinate}.{name}")),
    );
    let prepared = prepare(&v).expect("real compiler Core bound to its operation and type");
    assert_eq!(
        prepared
            .program()
            .as_effect_program()
            .expect("effect")
            .compiled_program(),
        bundle_bytes
    );
}

fn bound_compiler_package(core: &V, target: &V, exports: &V, intent: &str) -> V {
    let mut p = program();
    set(
        &mut p,
        "compiled_program",
        V::Bytes(encoded(&compiled_bundle(core, target, exports, intent))),
    );
    set(
        &mut p,
        "target_ir_identity",
        artifact_identity("edict.target-ir.artifact/v1", target),
    );
    let V::Map(intents) = get(core, "intents") else {
        panic!("Core intents")
    };
    if let Some((_, selected)) = intents.iter().find(|(key, _)| key == &text(intent)) {
        set(&mut p, "output_type", get(selected, "output").clone());
    }
    let mut publication = package(&p);
    let V::Text(coordinate) = get(core, "coordinate") else {
        panic!("Core coordinate")
    };
    set(
        &mut publication,
        "operation_coordinate",
        text(&format!("{coordinate}.{intent}")),
    );
    publication
}
fn selected_target_mut(target: &mut V) -> &mut V {
    let V::Map(fields) = target else {
        panic!("target")
    };
    let V::Map(intents) = &mut fields
        .iter_mut()
        .find(|(key, _)| key == &text("intents"))
        .expect("intents")
        .1
    else {
        panic!("intent map")
    };
    &mut intents[0].1
}
#[test]
fn repinned_target_still_requires_exact_core_result_budget_profile_constraints_and_order() {
    let core = compiled_core();
    let exports = compiled_exports();
    for (key, value) in [
        (
            "result",
            map([
                ("kind", text("const")),
                (
                    "value",
                    map([
                        ("kind", text("int")),
                        ("width", text("U32")),
                        ("value", V::Integer(43)),
                    ]),
                ),
            ]),
        ),
        (
            "coreEvaluationBudget",
            map([
                ("maxSteps", V::Integer(2)),
                ("maxAllocatedBytes", V::Integer(16 * 1024 * 1024)),
                ("maxOutputBytes", V::Integer(65536)),
            ]),
        ),
        ("operationProfile", text("different.profile/v1")),
        ("inputConstraints", V::Array(vec![map([])])),
        ("executionOrder", V::Array(vec![text("unknown")])),
    ] {
        let mut target = compiled_target(&core);
        set(selected_target_mut(&mut target), key, value);
        refused(&bound_compiler_package(
            &core, &target, &exports, "update@1",
        ));
    }
    for (steps, order) in [
        (vec![map([("id", text("s"))])], vec![]),
        (vec![map([("id", text("s"))])], vec![text("s"), text("s")]),
        (
            vec![map([("id", text("s"))]), map([("id", text("s"))])],
            vec![text("s")],
        ),
        (vec![map([("id", text(""))])], vec![text("")]),
    ] {
        let mut target = compiled_target(&core);
        let selected = selected_target_mut(&mut target);
        set(selected, "steps", V::Array(steps));
        set(selected, "executionOrder", V::Array(order));
        refused(&bound_compiler_package(
            &core, &target, &exports, "update@1",
        ));
    }
    let mut target = compiled_target(&core);
    set(&mut target, "sourceCoreCoordinate", text("other"));
    refused(&bound_compiler_package(
        &core, &target, &exports, "update@1",
    ));
    let mut target = compiled_target(&core);
    set(
        &mut target,
        "semanticClosure",
        map([
            (
                "sourceCore",
                map([
                    ("id", text("app")),
                    ("digest", V::Array(vec![text("sha256"), id(7)])),
                ]),
            ),
            ("lawpacks", V::Array(vec![])),
        ]),
    );
    refused(&bound_compiler_package(
        &core, &target, &exports, "update@1",
    ));
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &exports,
        "other",
    ));
}

#[test]
fn compiled_artifact_claims_and_nested_canonical_bytes_cannot_be_substituted() {
    for key in [
        "core_identity",
        "target_ir_identity",
        "application_schema_identity",
    ] {
        let mut v = package(&program());
        let V::Map(fields) = &mut v else {
            panic!("package")
        };
        let closure = &mut fields
            .iter_mut()
            .find(|(key, _)| key == &text("semantic_closure"))
            .expect("closure")
            .1;
        set(closure, key, id(7));
        refused(&v);
    }
    for key in [
        "core_artifact",
        "target_ir_artifact",
        "lawpack_exports_artifact",
    ] {
        let mut bundle = compiler_program();
        let V::Bytes(mut bytes) = get(&bundle, key).clone() else {
            panic!("artifact")
        };
        bytes.push(0);
        set(&mut bundle, key, V::Bytes(bytes));
        let mut p = program();
        set(&mut p, "compiled_program", V::Bytes(encoded(&bundle)));
        refused(&package(&p));
    }
    let mut bundle = compiler_program();
    add(&mut bundle, "extra", V::Null);
    let mut p = program();
    set(&mut p, "compiled_program", V::Bytes(encoded(&bundle)));
    refused(&package(&p));
}

fn captured_compiler_artifacts() -> (V, V, V) {
    let decode = |hex: &str| {
        echo_edict_canonical::decode_canonical_cbor_v1(
            &hex::decode(hex.trim()).expect("artifact bytes"),
        )
        .expect("canonical compiler artifact")
    };
    (decode(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/core.hex")),decode(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/06-target-ir.hex")),decode(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/02-lawpack-exports.hex")))
}
fn first_step_mut(target: &mut V) -> &mut V {
    let selected = selected_target_mut(target);
    let V::Map(fields) = selected else {
        panic!("intent")
    };
    let V::Array(steps) = &mut fields
        .iter_mut()
        .find(|(key, _)| key == &text("steps"))
        .expect("steps")
        .1
    else {
        panic!("steps")
    };
    &mut steps[0]
}
#[test]
fn repinning_cannot_change_a_compiled_instruction_while_retaining_its_core_body() {
    // Disabling the ordered Core/Target relation admits each forged lowering,
    // despite every artifact identity and the new package pin being correct.
    let (core, target, exports) = captured_compiler_artifacts();
    assert!(prepare(&bound_compiler_package(
        &core,
        &target,
        &exports,
        "replaceRange"
    ))
    .is_ok());
    for mode in 0..4 {
        let mut target = target.clone();
        match mode {
            0 => set(
                first_step_mut(&mut target),
                "input",
                map([
                    ("kind", text("const")),
                    (
                        "value",
                        map([("kind", text("bool")), ("value", V::Bool(false))]),
                    ),
                ]),
            ),
            1 => set(
                first_step_mut(&mut target),
                "effect",
                text("text.otherRead"),
            ),
            2 => {
                let mut order = get(selected_target_mut(&mut target), "executionOrder").clone();
                let V::Array(ref mut order_values) = order else {
                    panic!("order")
                };
                order_values.reverse();
                set(selected_target_mut(&mut target), "executionOrder", order);
            }
            _ => {
                let V::Map(fields) = selected_target_mut(&mut target) else {
                    panic!("intent")
                };
                let V::Array(requirements) = &mut fields
                    .iter_mut()
                    .find(|(key, _)| key == &text("requirements"))
                    .expect("requirements")
                    .1
                else {
                    panic!("requirements")
                };
                set(
                    &mut requirements[0],
                    "onFailure",
                    map([
                        ("kind", text("terminal")),
                        (
                            "reason",
                            map([("reasonKind", text("text.Unrelated")), ("payload", map([]))]),
                        ),
                    ]),
                );
            }
        }
        assert!(
            prepare(&bound_compiler_package(
                &core,
                &target,
                &exports,
                "replaceRange"
            ))
            .is_err(),
            "forged lowering {mode} survived a fresh pin"
        );
    }
}

fn value_field_mut<'a>(value: &'a mut V, name: &str) -> &'a mut V {
    let V::Map(fields) = value else { panic!("map") };
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == &text(name))
        .expect("field")
        .1
}
fn core_body_mut(core: &mut V) -> &mut V {
    let V::Map(intents) = value_field_mut(core, "intents") else {
        panic!("intents")
    };
    value_field_mut(&mut intents[0].1, "body")
}
#[test]
fn matching_core_and_target_still_cannot_forge_local_identity_or_scope() {
    let (original_core, original_target, exports) = captured_compiler_artifacts();
    for mode in 0..3 {
        let mut core = original_core.clone();
        let mut target = original_target.clone();
        match mode {
            0 => {
                let V::Array(nodes) = value_field_mut(core_body_mut(&mut core), "nodes") else {
                    panic!("nodes")
                };
                let mut input = get(&nodes[0], "input").clone();
                set(
                    value_field_mut(&mut input, "base"),
                    "ref",
                    map([
                        ("id", text("arg.0")),
                        ("type", text("jedit.text.replace_range@1.Input")),
                        ("alphaName", text("forged")),
                    ]),
                );
                set(&mut nodes[0], "input", input.clone());
                set(first_step_mut(&mut target), "input", input);
            }
            1 => {
                let V::Array(nodes) = value_field_mut(core_body_mut(&mut core), "nodes") else {
                    panic!("nodes")
                };
                nodes.reverse();
                let V::Array(order) =
                    value_field_mut(selected_target_mut(&mut target), "executionOrder")
                else {
                    panic!("order")
                };
                order.reverse();
            }
            _ => {
                let V::Array(locals) = value_field_mut(core_body_mut(&mut core), "locals") else {
                    panic!("locals")
                };
                locals.push(map([
                    ("id", text("orphan")),
                    ("type", text("U32")),
                    ("alphaName", text("$orphan")),
                ]));
            }
        }
        let source = value_field_mut(
            value_field_mut(&mut target, "semanticClosure"),
            "sourceCore",
        );
        set(
            source,
            "digest",
            V::Array(vec![
                text("sha256"),
                artifact_identity("edict.core.module/v1", &core),
            ]),
        );
        assert!(
            prepare(&bound_compiler_package(
                &core,
                &target,
                &exports,
                "replaceRange"
            ))
            .is_err(),
            "forged scope {mode} survived matching artifact pins"
        );
    }
}

#[test]
fn pure_bindings_preserve_values_and_become_available_only_after_their_definition() {
    let mut core = compiled_core();
    let binding = map([
        ("id", text("local.0")),
        ("type", text("U32")),
        ("alphaName", text("$value")),
    ]);
    let value = map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U32")),
                ("value", V::Integer(7)),
            ]),
        ),
    ]);
    let local = map([("kind", text("local")), ("ref", binding.clone())]);
    let body = core_body_mut(&mut core);
    let V::Array(locals) = value_field_mut(body, "locals") else {
        panic!("locals")
    };
    locals.push(binding.clone());
    set(
        body,
        "nodes",
        V::Array(vec![map([
            ("kind", text("let")),
            ("binding", binding.clone()),
            ("value", value.clone()),
        ])]),
    );
    set(body, "result", local);
    let mut target = compiled_target(&core);
    let selected = selected_target_mut(&mut target);
    set(
        selected,
        "pureBindings",
        V::Array(vec![map([
            ("id", text("binding.0")),
            ("binding", binding),
            ("value", value),
        ])]),
    );
    set(
        selected,
        "executionOrder",
        V::Array(vec![text("binding.0")]),
    );
    assert!(prepare(&bound_compiler_package(
        &core,
        &target,
        &compiled_exports(),
        "update@1"
    ))
    .is_ok());
    let V::Array(bindings) = value_field_mut(selected_target_mut(&mut target), "pureBindings")
    else {
        panic!("bindings")
    };
    set(
        &mut bindings[0],
        "value",
        map([
            ("kind", text("const")),
            (
                "value",
                map([("kind", text("bool")), ("value", V::Bool(false))]),
            ),
        ]),
    );
    refused(&bound_compiler_package(
        &core,
        &target,
        &compiled_exports(),
        "update@1",
    ));
}

#[test]
fn lowered_failure_arms_preserve_the_source_binders_and_values_without_duplicates() {
    let (core, target, exports) = captured_compiler_artifacts();
    for mode in 0..3 {
        let mut target = target.clone();
        let step = first_step_mut(&mut target);
        match mode {
            0 => {
                let V::Map(arms) = value_field_mut(step, "obstructionArms") else {
                    panic!("arms")
                };
                set(
                    &mut arms[0].1,
                    "value",
                    map([
                        ("kind", text("call")),
                        ("callee", text("text.OtherFailure")),
                        ("args", V::Array(vec![])),
                        ("typeArgs", V::Array(vec![])),
                    ]),
                );
            }
            1 => {
                let V::Array(failures) = value_field_mut(step, "obstructionFailures") else {
                    panic!("failures")
                };
                failures.push(failures[0].clone());
            }
            _ => {
                let V::Map(arms) = value_field_mut(step, "obstructionArms") else {
                    panic!("arms")
                };
                arms[1].1 = arms[0].1.clone();
            }
        }
        refused(&bound_compiler_package(
            &core,
            &target,
            &exports,
            "replaceRange",
        ));
    }
}
