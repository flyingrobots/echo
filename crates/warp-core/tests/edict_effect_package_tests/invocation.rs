// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Invocation byte/basis binding without runtime authority admission.
use super::*;
use warp_core::edict_effects::EffectInvocationV2;
fn invocation(input: Vec<u8>) -> V {
    let package_bytes = encoded(&package(&program()));
    map([
        ("schema", text("echo.operation-invocation/v2")),
        ("authority_grant_identity", id(9)),
        (
            "delegated_budget",
            map([
                ("steps", V::Integer(1)),
                ("read_bytes", V::Integer(0)),
                ("write_bytes", V::Integer(0)),
            ]),
        ),
        ("operation_coordinate", text("app.update@1")),
        (
            "package_id",
            V::Bytes(hash(b"echo:operation-package:v1\0", &package_bytes).to_vec()),
        ),
        ("application_input_bytes", V::Bytes(input.clone())),
        (
            "evaluation_basis",
            map([
                (
                    "application_basis_schema_identity",
                    profile("echo.operation.basis.edict-effect/v1"),
                ),
                (
                    "application_basis_value_identity",
                    V::Bytes(hash(b"echo:operation-edict-effect-basis:v1\0", &input).to_vec()),
                ),
                ("commit_global_tick", V::Null),
                ("commit_id", id(5)),
                ("head_id", id(6)),
                ("state_root", id(7)),
                ("worldline_id", id(8)),
                ("worldline_tick", V::Integer(0)),
            ]),
        ),
    ])
}
fn check(v: &V) -> Result<EffectInvocationV2, EffectPackageError> {
    EffectInvocationV2::prepare(
        &encoded(v),
        &prepare(&package(&program())).expect("package"),
    )
}
fn basis_set(v: &mut V, k: &str, x: V) {
    let V::Map(m) = v else { panic!("map") };
    set(
        &mut m
            .iter_mut()
            .find(|(key, _)| key == &text("evaluation_basis"))
            .expect("basis")
            .1,
        k,
        x,
    );
}
#[test]
fn invocation_binds_canonical_input_and_retains_the_exact_v1_parent_basis() {
    let value = invocation(encoded(&V::Integer(42)));
    let bytes = encoded(&value);
    let prepared = check(&value).expect("valid V2 invocation");
    assert_eq!(prepared.canonical_bytes(), bytes);
    assert_eq!(
        prepared.identity(),
        hash(b"echo:operation-invocation:v2\0", &bytes)
    );
    assert_eq!(prepared.application_input_bytes(), encoded(&V::Integer(42)));
    assert_eq!(prepared.evaluation_basis().state_root(), [7; 32]);
    assert_eq!(prepared.evaluation_basis().commit_id(), [5; 32]);
    assert_eq!(prepared.authority_grant_identity(), [9; 32]);
}
#[test]
fn altered_input_cannot_keep_its_predecessors_basis_digest() {
    let mut value = invocation(encoded(&V::Integer(42)));
    set(
        &mut value,
        "application_input_bytes",
        V::Bytes(encoded(&V::Integer(43))),
    );
    assert!(check(&value).is_err());
    basis_set(
        &mut value,
        "application_basis_value_identity",
        V::Bytes(
            hash(
                b"echo:operation-edict-effect-basis:v1\0",
                &encoded(&V::Integer(43)),
            )
            .to_vec(),
        ),
    );
    assert!(check(&value).is_ok());
    basis_set(&mut value, "application_basis_schema_identity", id(88));
    assert!(check(&value).is_err());
}
#[test]
fn invocation_refuses_cross_version_cross_package_and_cross_operation_substitution() {
    for (k, v) in [
        ("schema", text("echo.operation-invocation/v1")),
        ("package_id", id(5)),
        ("operation_coordinate", text("app.other@1")),
        ("authority_grant_identity", V::Bytes(vec![9; 31])),
    ] {
        let mut value = invocation(encoded(&V::Integer(42)));
        set(&mut value, k, v);
        assert!(check(&value).is_err());
    }
    for k in ["node", "replacement_bytes", "extra"] {
        let mut value = invocation(encoded(&V::Integer(42)));
        add(&mut value, k, id(1));
        assert!(check(&value).is_err());
    }
}
#[test]
fn invocation_refuses_noncanonical_input_or_input_exceeding_its_declared_ceiling() {
    for input in [
        vec![],
        vec![0x18, 0],
        vec![0, 0],
        encoded(&V::Bytes(vec![0; 65536])),
    ] {
        assert!(check(&invocation(input)).is_err());
    }
    // A 65,536-byte canonical bstr fits; its CBOR header counts toward the limit.
    let input = encoded(&V::Bytes(vec![0; 65533]));
    assert_eq!(input.len(), 65536);
    assert!(check(&invocation(input)).is_ok());
}
#[test]
fn delegated_budget_cannot_be_zero_or_exceed_any_package_axis() {
    for (steps, read, write) in [(0, 0, 0), (2, 0, 0), (1, 1, 0), (1, 0, 1)] {
        let mut value = invocation(encoded(&V::Integer(42)));
        set(
            &mut value,
            "delegated_budget",
            map([
                ("steps", V::Integer(steps)),
                ("read_bytes", V::Integer(read)),
                ("write_bytes", V::Integer(write)),
            ]),
        );
        assert!(check(&value).is_err());
    }
}
#[test]
fn invocation_parent_uses_the_exact_v1_map_without_normalizing_unknown_fields() {
    let mut value = invocation(encoded(&V::Integer(42)));
    basis_set(&mut value, "worldline_tick", V::Integer(-1));
    assert!(check(&value).is_err());
    let mut value = invocation(encoded(&V::Integer(42)));
    basis_set(&mut value, "commit_id", V::Bytes(vec![0; 33]));
    assert!(check(&value).is_err());
    let mut value = invocation(encoded(&V::Integer(42)));
    let V::Map(m) = &mut value else { panic!("map") };
    add(
        &mut m
            .iter_mut()
            .find(|(k, _)| k == &text("evaluation_basis"))
            .expect("basis")
            .1,
        "extra",
        id(1),
    );
    assert!(check(&value).is_err());
}
#[test]
fn input_uses_the_selected_packages_smaller_ceiling_and_exact_outer_bytes() {
    let mut p = program();
    set(&mut p, "max_input_bytes", V::Integer(1));
    let value = package(&p);
    let selected = prepare(&value).expect("smaller ceiling package");
    for (input, accepts) in [
        (encoded(&V::Integer(0)), true),
        (encoded(&V::Integer(42)), false),
    ] {
        let mut value = invocation(input);
        set(
            &mut value,
            "package_id",
            V::Bytes(
                hash(
                    b"echo:operation-package:v1\0",
                    &selected.to_canonical_bytes().expect("selected publication"),
                )
                .to_vec(),
            ),
        );
        assert_eq!(
            EffectInvocationV2::prepare(&encoded(&value), &selected).is_ok(),
            accepts
        );
    }
    let mut bytes = encoded(&invocation(encoded(&V::Integer(42))));
    bytes.push(0);
    assert!(
        EffectInvocationV2::prepare(&bytes, &prepare(&package(&program())).expect("original"))
            .is_err()
    );
}
