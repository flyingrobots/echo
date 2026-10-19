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
    types.push((text(&"é".repeat(512)), map([("kind", text("U32"))])));
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
fn retained_public_capture_refuses_its_inconsistent_lawpack_artifact_pins() {
    // These are unchanged artifacts from a public Edict JSONL build attempt.
    // Their lawpack pins do not match the retained exports/adapter bytes.
    // They must not become accepted evidence through fresh package pins.
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
    let mut bundle = compiled_bundle(&core, &target, &exports, name);
    let captured = |value: &str| V::Bytes(hex::decode(value.trim()).expect("capture"));
    set(&mut bundle, "lowering_artifacts", map([
        ("source_artifact", captured(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/04-source.hex"))),
        ("lawpack_artifact", captured(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/03-lawpack.hex"))),
        ("adapter_artifact", captured(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/01-lawpack-adapter.hex"))),
        ("target_profile_artifact", captured(include_str!("../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/profile.hex"))),
        ("intrinsics_artifact", V::Bytes(include_bytes!("../../echo-edict-provider-verifier/resources/resource.target-intrinsics.cbor").to_vec())),
    ]));
    let bundle_bytes = encoded(&bundle);
    set(&mut p, "compiled_program", V::Bytes(bundle_bytes));
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
    refused(&v);
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

fn repinned_compiler_artifacts() -> (V, V, V) {
    compiler_fixture(&repinned_lowering_artifacts())
}
fn compiler_fixture(artifacts: &V) -> (V, V, V) {
    let decode = |hex: &str| {
        echo_edict_canonical::decode_canonical_cbor_v1(
            &hex::decode(hex.trim()).expect("artifact bytes"),
        )
        .expect("canonical compiler artifact")
    };
    let mut core = decode(include_str!(
        "../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/core.hex"
    ));
    let mut target = decode(include_str!(
        "../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/06-target-ir.hex"
    ));
    let exports = decode(include_str!(
        "../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/02-lawpack-exports.hex"
    ));
    let V::Bytes(bytes) = get(artifacts, "lawpack_artifact") else {
        panic!("lawpack")
    };
    let lawpack = echo_edict_canonical::decode_canonical_cbor_v1(bytes).expect("lawpack");
    let V::Array(adapters) = get(&lawpack, "targetAdapters") else {
        panic!("adapters")
    };
    set(
        &mut target,
        "targetProfile",
        get(&adapters[0], "acceptedTargetProfile").clone(),
    );
    let reference = map([
        ("id", text("jedit.text@1")),
        (
            "digest",
            V::Array(vec![
                text("sha256"),
                artifact_identity("edict.lawpack/v1", &lawpack),
            ]),
        ),
    ]);
    let V::Array(imports) = value_field_mut(&mut core, "imports") else {
        panic!("imports")
    };
    set(&mut imports[0], "ref", reference.clone());
    let closure = value_field_mut(&mut target, "semanticClosure");
    set(closure, "lawpacks", V::Array(vec![reference]));
    set(
        value_field_mut(closure, "sourceCore"),
        "digest",
        V::Array(vec![
            text("sha256"),
            artifact_identity("edict.core.module/v1", &core),
        ]),
    );
    (core, target, exports)
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
    let (core, target, exports) = repinned_compiler_artifacts();
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
    let (original_core, original_target, exports) = repinned_compiler_artifacts();
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
    let (core, target, exports) = repinned_compiler_artifacts();
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

#[test]
fn pinned_adapter_rejects_swapped_failure_tags_and_substituted_intrinsics() {
    let (core, original, exports) = repinned_compiler_artifacts();
    assert!(prepare(&bound_compiler_package(
        &core,
        &original,
        &exports,
        "replaceRange"
    ))
    .is_ok());
    for mode in 0..2 {
        let mut target = original.clone();
        let step = first_step_mut(&mut target);
        if mode == 0 {
            let V::Map(arms) = value_field_mut(step, "obstructionArms") else {
                panic!("arms")
            };
            let first = arms[0].1.clone();
            arms[0].1 = arms[1].1.clone();
            arms[1].1 = first;
        } else {
            set(step, "targetIntrinsic", text("echo.dpo@1.node-delete"));
        }
        assert!(
            prepare(&bound_compiler_package(
                &core,
                &target,
                &exports,
                "replaceRange"
            ))
            .is_err(),
            "adapter substitution {mode} survived fresh pins"
        );
    }
}

#[test]
fn fresh_bundle_pins_cannot_replace_or_omit_the_pinned_lowering_artifacts() {
    let (core, target, exports) = repinned_compiler_artifacts();
    for mode in 0..5 {
        let mut bundle = compiled_bundle(&core, &target, &exports, "replaceRange");
        let lowering = value_field_mut(&mut bundle, "lowering_artifacts");
        if mode == 0 {
            *lowering = V::Null;
        } else {
            let key = [
                "",
                "source_artifact",
                "lawpack_artifact",
                "adapter_artifact",
                "adapter_artifact",
            ][mode];
            let V::Bytes(raw) = get(lowering, key) else {
                panic!("artifact")
            };
            let mut value = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("artifact");
            match mode {
                1 => {
                    let V::Bytes(source) = &mut value else {
                        panic!("source")
                    };
                    *source = String::from_utf8(source.clone())
                        .expect("utf8")
                        .replace(" as text;", " as other;")
                        .into_bytes();
                }
                2 => set(&mut value, "version", text("2")),
                3 => {
                    let implementations = value_field_mut(&mut value, "effectImplementations");
                    let implementation = value_field_mut(implementations, "jedit.text@1.readFact");
                    set(
                        implementation,
                        "targetIntrinsic",
                        text("echo.dpo@1.node-delete"),
                    );
                }
                _ => {
                    add(&mut value, "extra", V::Null);
                }
            }
            set(lowering, key, V::Bytes(encoded(&value)));
        }
        let mut package = bound_compiler_package(&core, &target, &exports, "replaceRange");
        let V::Bytes(raw) = get(&package, "program") else {
            panic!("program")
        };
        let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
        set(&mut program, "compiled_program", V::Bytes(encoded(&bundle)));
        set(&mut package, "program", V::Bytes(encoded(&program)));
        assert!(
            prepare(&package).is_err(),
            "lowering substitution {mode} survived fresh pins"
        );
    }
}

fn repin_core(core: &V, target: &mut V) {
    set(
        value_field_mut(value_field_mut(target, "semanticClosure"), "sourceCore"),
        "digest",
        V::Array(vec![
            text("sha256"),
            artifact_identity("edict.core.module/v1", core),
        ]),
    );
}
#[test]
fn typed_contract_refuses_an_effect_input_with_the_wrong_representation() {
    let (mut core, mut target, exports) = repinned_compiler_artifacts();
    let V::Array(nodes) = value_field_mut(core_body_mut(&mut core), "nodes") else {
        panic!("nodes")
    };
    let mut input = get(&nodes[0], "input").clone();
    set(&mut input, "field", text("expected"));
    set(&mut nodes[0], "input", input.clone());
    set(first_step_mut(&mut target), "input", input);
    repin_core(&core, &mut target);
    refused(&bound_compiler_package(
        &core,
        &target,
        &exports,
        "replaceRange",
    ));
}
#[test]
fn typed_contract_refuses_wrong_result_types() {
    let mut core = compiled_core();
    set(
        core_body_mut(&mut core),
        "result",
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
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}
#[test]
fn typed_contract_refuses_unauthorized_helper_calls() {
    let mut core = compiled_core();
    set(
        core_body_mut(&mut core),
        "result",
        map([
            ("kind", text("call")),
            ("callee", text("app.unpublished")),
            ("args", V::Array(vec![])),
            ("typeArgs", V::Array(vec![])),
        ]),
    );
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}
#[test]
fn typed_contract_refuses_a_matching_core_target_let_with_the_wrong_type() {
    let mut core = compiled_core();
    let binding = map([
        ("id", text("local.0")),
        ("type", text("U32")),
        ("alphaName", text("$local0")),
    ]);
    let value = map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("bool")), ("value", V::Bool(false))]),
        ),
    ]);
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
    refused(&bound_compiler_package(
        &core,
        &target,
        &compiled_exports(),
        "update@1",
    ));
}
#[test]
fn typed_contract_refuses_mapping_an_integrity_failure_to_a_resource_obstruction() {
    let (mut core, mut target, exports) = repinned_compiler_artifacts();
    let V::Array(nodes) = value_field_mut(core_body_mut(&mut core), "nodes") else {
        panic!("nodes")
    };
    let arms = value_field_mut(&mut nodes[0], "obstructionMap");
    let arm = value_field_mut(arms, "notAtom");
    set(
        value_field_mut(arm, "value"),
        "callee",
        text("text.FactMissing"),
    );
    let arm = arm.clone();
    set(
        value_field_mut(first_step_mut(&mut target), "obstructionArms"),
        "echo.atom-required/v1",
        arm,
    );
    repin_core(&core, &mut target);
    refused(&bound_compiler_package(
        &core,
        &target,
        &exports,
        "replaceRange",
    ));
}

#[test]
fn typed_contract_accepts_checked_source_helpers_and_refuses_wrong_arguments() {
    let mut core = compiled_core();
    let parameter = map([
        ("id", text("param.0")),
        ("type", text("U32")),
        ("alphaName", text("$param0")),
    ]);
    let reference = map([("kind", text("local")), ("ref", parameter.clone())]);
    add(
        &mut core,
        "functions",
        map([(
            "identity",
            map([
                ("params", V::Array(vec![parameter])),
                ("returnType", text("U32")),
                (
                    "body",
                    map([
                        ("locals", V::Array(vec![])),
                        ("bindings", V::Array(vec![])),
                        ("result", reference),
                    ]),
                ),
            ]),
        )]),
    );
    let V::Array(locals) = get(core_body_mut(&mut core), "locals") else {
        panic!("locals")
    };
    let argument = map([("kind", text("local")), ("ref", locals[0].clone())]);
    let call = |arg| {
        map([
            ("kind", text("call")),
            ("callee", text("app.identity")),
            ("args", V::Array(vec![arg])),
            ("typeArgs", V::Array(vec![])),
        ])
    };
    set(core_body_mut(&mut core), "result", call(argument));
    assert!(prepare(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1"
    ))
    .is_ok());
    set(
        core_body_mut(&mut core),
        "result",
        call(map([
            ("kind", text("const")),
            (
                "value",
                map([("kind", text("bool")), ("value", V::Bool(false))]),
            ),
        ])),
    );
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}

#[test]
fn pinned_exports_refuse_core_redefinitions_of_imported_type_contracts() {
    let (original, target, exports) = repinned_compiler_artifacts();
    for mode in 0..4 {
        let mut core = original.clone();
        let types = value_field_mut(&mut core, "types");
        match mode {
            0 => set(
                value_field_mut(value_field_mut(types, "jedit.text@1.FactAddress"), "fields"),
                "nodeId",
                text("Bytes<exact=16>"),
            ),
            1 => set(
                value_field_mut(types, "jedit.text@1.FactBytes"),
                "max",
                V::Integer(2_097_152),
            ),
            2 => add(
                types,
                "jedit.text@1.Unpublished",
                map([("kind", text("Bool"))]),
            ),
            _ => add(
                value_field_mut(types, "jedit.text@1.FactBytes"),
                "extra",
                V::Null,
            ),
        }
        let mut target = target.clone();
        repin_core(&core, &mut target);
        assert!(
            prepare(&bound_compiler_package(
                &core,
                &target,
                &exports,
                "replaceRange"
            ))
            .is_err(),
            "Core type substitution {mode} survived the pinned exports"
        );
    }
}

fn nominal_identity_core() -> V {
    let mut core = compiled_core();
    for (name, contract) in [
        ("nominal@1.A", "nominal@1.A"),
        ("nominal@1.B", "nominal@1.B"),
    ] {
        add(
            value_field_mut(&mut core, "types"),
            name,
            map([
                ("kind", text("Nominal")),
                ("contract", text(contract)),
                ("representation", text("U32")),
            ]),
        );
    }
    let intent = value_field_mut(value_field_mut(&mut core, "intents"), "update@1");
    set(intent, "input", text("nominal@1.A"));
    set(intent, "output", text("nominal@1.A"));
    let body = value_field_mut(intent, "body");
    let V::Array(locals) = value_field_mut(body, "locals") else {
        panic!("locals")
    };
    set(&mut locals[0], "type", text("nominal@1.A"));
    let result = map([("kind", text("local")), ("ref", locals[0].clone())]);
    set(body, "result", result);
    core
}
#[test]
fn nominal_contracts_cannot_be_interchanged_through_matching_representations() {
    let mut core = nominal_identity_core();
    assert!(prepare(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1"
    ))
    .is_ok());
    set(
        value_field_mut(value_field_mut(&mut core, "intents"), "update@1"),
        "output",
        text("nominal@1.B"),
    );
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}
#[test]
fn nominal_contracts_cannot_implicitly_unwrap_into_their_representation() {
    let mut core = nominal_identity_core();
    set(
        value_field_mut(value_field_mut(&mut core, "intents"), "update@1"),
        "output",
        text("U32"),
    );
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}
#[test]
fn nominal_contracts_cannot_be_constructed_by_an_untyped_literal() {
    let mut core = nominal_identity_core();
    set(
        core_body_mut(&mut core),
        "result",
        map([
            ("kind", text("const")),
            (
                "value",
                map([
                    ("kind", text("int")),
                    ("width", text("U32")),
                    ("value", V::Integer(7)),
                ]),
            ),
        ]),
    );
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}
#[test]
fn nominal_declarations_must_bind_their_own_core_table_identity() {
    let mut core = nominal_identity_core();
    set(
        value_field_mut(value_field_mut(&mut core, "types"), "nominal@1.A"),
        "contract",
        text("nominal@1.B"),
    );
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}

#[test]
fn checked_helpers_preserve_nominal_arguments_and_return_identity() {
    let mut core = nominal_identity_core();
    let helper = |name| {
        let parameter = map([
            ("id", text("param.0")),
            ("type", text(name)),
            ("alphaName", text("$param0")),
        ]);
        map([
            ("params", V::Array(vec![parameter.clone()])),
            ("returnType", text(name)),
            (
                "body",
                map([
                    ("locals", V::Array(vec![])),
                    ("bindings", V::Array(vec![])),
                    ("result", map([("kind", text("local")), ("ref", parameter)])),
                ]),
            ),
        ])
    };
    add(
        &mut core,
        "functions",
        map([("identity", helper("nominal@1.A"))]),
    );
    let result = get(core_body_mut(&mut core), "result").clone();
    set(
        core_body_mut(&mut core),
        "result",
        map([
            ("kind", text("call")),
            ("callee", text("app.identity")),
            ("args", V::Array(vec![result])),
            ("typeArgs", V::Array(vec![])),
        ]),
    );
    assert!(prepare(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1"
    ))
    .is_ok());
    set(
        value_field_mut(&mut core, "functions"),
        "identity",
        helper("nominal@1.B"),
    );
    set(
        value_field_mut(value_field_mut(&mut core, "intents"), "update@1"),
        "output",
        text("nominal@1.B"),
    );
    refused(&bound_compiler_package(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    ));
}

#[test]
fn effect_preparation_requires_the_pinned_target_profile_and_intrinsic_catalog() {
    let (core, target, exports) = repinned_compiler_artifacts();
    let mut publication = bound_compiler_package(&core, &target, &exports, "replaceRange");
    let V::Bytes(raw) = get(&publication, "program") else {
        panic!("program")
    };
    let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
    let V::Bytes(raw) = get(&program, "compiled_program") else {
        panic!("bundle")
    };
    let mut bundle = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("bundle");
    let V::Map(fields) = value_field_mut(&mut bundle, "lowering_artifacts") else {
        panic!("lowering")
    };
    fields.retain(|(key, _)| {
        key != &text("target_profile_artifact") && key != &text("intrinsics_artifact")
    });
    set(&mut program, "compiled_program", V::Bytes(encoded(&bundle)));
    set(&mut publication, "program", V::Bytes(encoded(&program)));
    refused(&publication);
}

fn catalog_fixture() -> V {
    let lowering = repinned_lowering_artifacts();
    let V::Bytes(raw) = get(&lowering, "intrinsics_artifact") else {
        panic!("catalog")
    };
    echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("catalog")
}
fn publication_for_lowering(lowering: &V) -> V {
    let (core, target, exports) = compiler_fixture(lowering);
    publication_from_parts(&core, &target, &exports, lowering)
}
fn publication_from_parts(core: &V, target: &V, exports: &V, lowering: &V) -> V {
    let mut publication = bound_compiler_package(core, target, exports, "replaceRange");
    let V::Bytes(raw) = get(&publication, "program") else {
        panic!("program")
    };
    let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
    let mut bundle = compiled_bundle(core, target, exports, "replaceRange");
    set(&mut bundle, "lowering_artifacts", lowering.clone());
    set(&mut program, "compiled_program", V::Bytes(encoded(&bundle)));
    set(&mut publication, "program", V::Bytes(encoded(&program)));
    publication
}
#[test]
fn intrinsic_catalog_signatures_must_agree_with_the_exported_effect() {
    assert!(prepare(&publication_for_lowering(&repinned_lowering_artifacts())).is_ok());
    for mode in 0..3 {
        let mut catalog = catalog_fixture();
        let intrinsic = value_field_mut(
            value_field_mut(&mut catalog, "intrinsics"),
            "echo.dpo@1.node-atom-read",
        );
        match mode {
            0 => set(intrinsic, "argumentTypes", V::Array(vec![text("U32")])),
            1 => set(intrinsic, "returnType", text("U32")),
            _ => set(intrinsic, "typeParameters", V::Array(vec![text("T")])),
        }
        let lowering = lowering_fixture(Some(catalog));
        assert!(
            prepare(&publication_for_lowering(&lowering)).is_err(),
            "catalog signature substitution {mode} survived a fully repinned artifact chain"
        );
    }
}
#[test]
fn intrinsic_catalog_fault_contracts_must_agree_with_exported_failures() {
    for mode in 0..2 {
        let mut catalog = catalog_fixture();
        let intrinsic = value_field_mut(
            value_field_mut(&mut catalog, "intrinsics"),
            "echo.dpo@1.node-atom-read",
        );
        let failure = value_field_mut(
            value_field_mut(intrinsic, "effectFailures"),
            "echo.atom-required/v1",
        );
        if mode == 0 {
            set(failure, "authorityClass", text("resourceFault"));
        } else {
            set(failure, "payloadType", text("U32"));
        }
        let lowering = lowering_fixture(Some(catalog));
        assert!(
            prepare(&publication_for_lowering(&lowering)).is_err(),
            "catalog fault substitution {mode} survived a fully repinned artifact chain"
        );
    }
}
#[test]
fn a_profile_cannot_authorize_an_intrinsic_missing_from_its_catalog() {
    let mut catalog = catalog_fixture();
    set(&mut catalog, "intrinsics", map([]));
    refused(&publication_for_lowering(&lowering_fixture(Some(catalog))));
}

#[test]
fn runtime_read_abi_rejects_a_self_consistent_sixteen_byte_node_address() {
    let (_, _, mut exports) = repinned_compiler_artifacts();
    let V::Array(types) = value_field_mut(&mut exports, "types") else {
        panic!("types")
    };
    let address = types
        .iter_mut()
        .find(|ty| get(ty, "coordinate") == &text("jedit.text@1.FactAddress"))
        .expect("address");
    set(
        address,
        "definition",
        text("Record<nodeId:Bytes<exact=16>,typeId:Bytes<exact=32>,warpId:Bytes<exact=32>>"),
    );
    let lowering = lowering_fixture_inputs(None, Some(exports.clone()));
    let (mut core, mut target, _) = compiler_fixture(&lowering);
    set(
        value_field_mut(
            value_field_mut(
                value_field_mut(&mut core, "types"),
                "jedit.text@1.FactAddress",
            ),
            "fields",
        ),
        "nodeId",
        text("Bytes<exact=16>"),
    );
    repin_core(&core, &mut target);
    refused(&publication_from_parts(&core, &target, &exports, &lowering));
}
#[test]
fn runtime_read_abi_rejects_catalog_and_lawpack_agreement_on_wrong_fault_authority() {
    let (_, _, mut exports) = repinned_compiler_artifacts();
    let V::Array(effects) = value_field_mut(&mut exports, "effects") else {
        panic!("effects")
    };
    for failure in ["notAtom", "tooLarge", "typeMismatch"] {
        set(
            value_field_mut(value_field_mut(&mut effects[0], "effectFailures"), failure),
            "authorityClass",
            text("resourceFault"),
        );
    }
    let V::Array(obstructions) = value_field_mut(&mut exports, "obstructions") else {
        panic!("obstructions")
    };
    let malformed = obstructions
        .iter_mut()
        .find(|item| get(item, "coordinate") == &text("jedit.text@1.FactMalformed"))
        .expect("obstruction");
    set(malformed, "authorityClass", text("resourceFault"));
    let lowering = lowering_fixture_inputs(None, Some(exports.clone()));
    let (core, target, _) = compiler_fixture(&lowering);
    refused(&publication_from_parts(&core, &target, &exports, &lowering));
}

#[test]
fn outer_package_pins_cannot_replace_profile_or_catalog_bytes() {
    for key in ["target_profile_artifact", "intrinsics_artifact"] {
        let mut lowering = repinned_lowering_artifacts();
        let V::Bytes(raw) = get(&lowering, key) else {
            panic!("artifact")
        };
        let mut artifact = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("artifact");
        if key == "target_profile_artifact" {
            set(
                value_field_mut(&mut artifact, "diagnosticAbi"),
                "digest",
                V::Array(vec![text("sha256"), id(55)]),
            );
        } else {
            let declaration =
                get(get(&artifact, "intrinsics"), "echo.dpo@1.node-atom-read").clone();
            add(
                value_field_mut(&mut artifact, "intrinsics"),
                "echo.dpo@1.unused",
                declaration,
            );
        }
        set(&mut lowering, key, V::Bytes(encoded(&artifact)));
        refused(&publication_for_lowering(&lowering));
    }
}
#[cfg(feature = "native_rule_bootstrap")]
#[path = "edict_effect_package_tests/execution.rs"]
mod execution;
