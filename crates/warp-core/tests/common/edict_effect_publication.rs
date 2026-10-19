// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical publication fixtures for package and runtime-owner admission tests.
#![allow(dead_code)]
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V};
use warp_core::edict_effects::{EffectPackageError, ExecutableOperationPackageV2};
pub fn map(fields: impl IntoIterator<Item = (&'static str, V)>) -> V {
    V::Map(
        fields
            .into_iter()
            .map(|(k, v)| (V::Text(k.into()), v))
            .collect(),
    )
}
pub fn text(s: &str) -> V {
    V::Text(s.into())
}
pub fn id(n: u8) -> V {
    V::Bytes(vec![n; 32])
}
pub fn encoded(v: &V) -> Vec<u8> {
    encode(v).expect("fixture")
}
pub fn hash(domain: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(domain);
    h.update(&(data.len() as u64).to_le_bytes());
    h.update(data);
    h.finalize().into()
}
pub fn profile(label: &str) -> V {
    V::Bytes(hash(b"echo:operation-profile:v1\0", label.as_bytes()).to_vec())
}
pub fn graph() -> V {
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
pub fn law() -> V {
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
pub fn footprint(v: &V) -> V {
    let mut h = blake3::Hasher::new();
    h.update(b"echo:operation-footprint-law:v2\0");
    h.update(&encoded(v));
    V::Bytes(h.finalize().as_bytes().to_vec())
}
pub fn program() -> V {
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
pub fn package(program: &V) -> V {
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
pub fn set(v: &mut V, k: &str, x: V) {
    let V::Map(m) = v else { panic!("map") };
    m.iter_mut()
        .find(|(key, _)| key == &text(k))
        .expect("field")
        .1 = x;
}
pub fn add(v: &mut V, k: &str, x: V) {
    let V::Map(m) = v else { panic!("map") };
    m.push((text(k), x));
}
pub fn prepare(v: &V) -> Result<ExecutableOperationPackageV2, EffectPackageError> {
    let b = encoded(v);
    let expected = hash(b"echo:operation-package:v2\0", &b);
    ExecutableOperationPackageV2::prepare(&b, expected)
}
pub fn refused(v: &V) {
    assert!(
        prepare(v).is_err(),
        "must refuse malformed or inconsistent package"
    );
}
