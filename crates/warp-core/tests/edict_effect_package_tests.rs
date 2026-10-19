// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! V2 byte binding and executable-law validation, before installation.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used, clippy::panic)]
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V};
use warp_core::edict_effects::{EffectPackageError, ExecutableOperationPackageV2};
use warp_core::TypeId;
fn map(fields: impl IntoIterator<Item = (&'static str, V)>) -> V {
    V::Map(
        fields
            .into_iter()
            .map(|(k, v)| (V::Text(k.into()), v))
            .collect(),
    )
}
fn text(s: &str) -> V {
    V::Text(s.into())
}
fn id(n: u8) -> V {
    V::Bytes(vec![n; 32])
}
fn encoded(v: &V) -> Vec<u8> {
    encode(v).expect("fixture")
}
fn hash(domain: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(domain);
    h.update(&(data.len() as u64).to_le_bytes());
    h.update(data);
    h.finalize().into()
}
fn profile(label: &str) -> V {
    V::Bytes(hash(b"echo:operation-profile:v1\0", label.as_bytes()).to_vec())
}
fn graph() -> V {
    map([
        (
            "node_types",
            V::Map(vec![(
                id(1),
                map([
                    ("addressing", V::Integer(1)),
                    ("atom_types", V::Array(vec![id(2)])),
                ]),
            )]),
        ),
        ("edge_types", V::Map(vec![])),
    ])
}
fn law() -> V {
    map([
        ("read_node_types", V::Array(vec![id(1)])),
        ("write_node_types", V::Array(vec![id(1)])),
        ("atom_types", V::Array(vec![id(2)])),
        ("range_edge_types", V::Array(vec![])),
        ("write_edge_types", V::Array(vec![])),
        ("portal_owner_types", V::Array(vec![])),
        ("max_reads", V::Integer(1)),
        ("max_writes", V::Integer(1)),
        ("max_range_edges", V::Integer(0)),
    ])
}
fn footprint(v: &V) -> V {
    let mut h = blake3::Hasher::new();
    h.update(b"echo:operation-footprint-law:v2\0");
    h.update(&encoded(v));
    V::Bytes(h.finalize().as_bytes().to_vec())
}
fn program() -> V {
    map([
        ("schema", text("echo.operation-program/v2")),
        ("kind", text("edict-effect-program/v1")),
        (
            "interpreter_profile_identity",
            profile("echo.operation-interpreter.edict-effect/v1"),
        ),
        (
            "intrinsic_profile_identity",
            profile("echo.operation-intrinsics.edict-effect/v1"),
        ),
        ("compiled_program", V::Bytes(vec![0xa0])),
        ("target_ir_identity", id(4)),
        ("identity_law", V::Integer(1)),
        ("graph_schema", V::Bytes(encoded(&graph()))),
        ("footprint_law", V::Bytes(encoded(&law()))),
        ("max_input_bytes", V::Integer(65536)),
        ("max_result_bytes", V::Integer(65536)),
        ("max_allocated_bytes", V::Integer(16 * 1024 * 1024)),
        ("output_type", text("app.Result")),
    ])
}
fn package(program: &V) -> V {
    map([
        ("schema", text("echo.operation-package/v2")),
        ("operation_coordinate", text("app.update@1")),
        ("obstruction_coordinate", text("app.Refusal")),
        ("authority_profile_identity", id(8)),
        (
            "semantic_closure",
            map([
                ("application_schema_coordinate", text("app.Schema")),
                ("application_schema_identity", id(9)),
                ("canonical_meaning_identity", id(3)),
                ("core_identity", id(2)),
                ("edict_source_identity", id(1)),
                ("target_ir_identity", id(4)),
            ]),
        ),
        ("program", V::Bytes(encoded(program))),
        (
            "budget_ceiling",
            map([
                ("steps", V::Integer(1)),
                ("read_bytes", V::Integer(0)),
                ("write_bytes", V::Integer(0)),
            ]),
        ),
        (
            "target_profile_identity",
            profile("echo.operation-target.edict-effect/v1"),
        ),
        (
            "application_basis_schema_identity",
            profile("echo.operation.basis.edict-effect/v1"),
        ),
        (
            "input_schema_identity",
            profile("echo.operation.input.edict-effect/v1"),
        ),
        (
            "result_schema_identity",
            profile("echo.operation.result.edict-effect/v1"),
        ),
        (
            "obstruction_schema_identity",
            profile("echo.operation.obstruction.edict-effect/v1"),
        ),
        (
            "result_interpretation_identity",
            profile("echo.operation.result-interpretation.edict-effect/v1"),
        ),
        (
            "obstruction_interpretation_identity",
            profile("echo.operation.obstruction-interpretation.edict-effect/v1"),
        ),
        (
            "evaluation_basis_schema_identity",
            profile("echo.operation.evaluation-basis/v1"),
        ),
        (
            "interpreter_profile_identity",
            profile("echo.operation-interpreter.edict-effect/v1"),
        ),
        (
            "intrinsic_profile_identity",
            profile("echo.operation-intrinsics.edict-effect/v1"),
        ),
        ("footprint_contract_identity", footprint(&law())),
    ])
}
fn set(v: &mut V, k: &str, x: V) {
    let V::Map(m) = v else { panic!("map") };
    m.iter_mut()
        .find(|(key, _)| key == &text(k))
        .expect("field")
        .1 = x;
}
fn add(v: &mut V, k: &str, x: V) {
    let V::Map(m) = v else { panic!("map") };
    m.push((text(k), x));
}
fn prepare(v: &V) -> Result<ExecutableOperationPackageV2, EffectPackageError> {
    let b = encoded(v);
    let expected = hash(b"echo:operation-package:v2\0", &b);
    ExecutableOperationPackageV2::prepare(&b, expected)
}
fn refused(v: &V) {
    assert!(
        prepare(v).is_err(),
        "must refuse malformed or inconsistent package"
    );
}
#[test]
fn exact_v2_package_binds_program_and_executes_its_schema_relations() {
    let p = program();
    let v = package(&p);
    let b = encoded(&v);
    let package = prepare(&v).expect("valid effect package");
    assert_eq!(package.canonical_bytes(), b);
    assert_eq!(
        package.program_identity(),
        hash(b"echo:operation-program:v2\0", &encoded(&p))
    );
    assert!(package
        .graph_schema()
        .allows_atom(TypeId([1; 32]), TypeId([2; 32])));
    assert!(!package
        .graph_schema()
        .allows_atom(TypeId([2; 32]), TypeId([1; 32])));
    assert_eq!(package.compiled_program(), [0xa0]);
}
#[test]
fn substituted_program_schema_law_and_closure_cannot_keep_the_original_pin() {
    let original = encoded(&package(&program()));
    let pin = hash(b"echo:operation-package:v2\0", &original);
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
        assert!(ExecutableOperationPackageV2::prepare(&encoded(&v), pin).is_err());
    }
    assert!(ExecutableOperationPackageV2::prepare(
        &original,
        hash(b"echo:operation-package:v1\0", &original)
    )
    .is_err());
}
#[test]
fn exact_package_and_closure_shapes_refuse_v1_lawpacks_projection_and_extra_fields() {
    for (k, x) in [
        ("schema", text("echo.operation-package/v1")),
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
        ("schema", text("echo.operation-program/v1")),
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
        V::Bytes(hash(b"echo:operation-footprint-law:v2\0", &encoded(&law())).to_vec()),
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
        ExecutableOperationPackageV2::prepare(&b, hash(b"echo:operation-package:v2\0", &b))
            .is_err()
    );
}
#[test]
fn declaration_boundaries_accept_zero_io_and_allocation_and_preserve_v1_coordinate_rules() {
    let mut p = program();
    for key in ["max_input_bytes", "max_result_bytes", "max_allocated_bytes"] {
        set(&mut p, key, V::Integer(0));
    }
    set(&mut p, "output_type", text(&"é".repeat(512)));
    let mut v = package(&p);
    set(&mut v, "operation_coordinate", text(&"x".repeat(1025)));
    let prepared = prepare(&v).expect("zero byte limits and legacy package coordinate shape");
    assert_eq!(prepared.max_input_bytes(), 0);
    assert_eq!(prepared.max_result_bytes(), 0);
    assert_eq!(prepared.max_allocated_bytes(), 0);
    assert_eq!(prepared.output_type().len(), 1024);
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
        prepared.program_identity(),
        prepare(&package(&program()))
            .expect("original")
            .program_identity()
    );
}
#[path = "edict_effect_package_tests/invocation.rs"]
mod invocation;
