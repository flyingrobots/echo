// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! V2 byte binding and executable-law validation, before installation.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used, clippy::panic)]
use echo_edict_canonical::CanonicalValueV1 as V;
use warp_core::edict_effects::EffectPackageError;
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
        [0xa0]
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
    let mut v = package(&p);
    set(&mut v, "operation_coordinate", text(&"x".repeat(1025)));
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
