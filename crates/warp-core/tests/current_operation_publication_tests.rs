// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! One current operation package and program codec for every program kind.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used, clippy::panic)]
#[path = "common/edict_effect_publication.rs"]
mod fixture;
use fixture::*;
use warp_core::{ExecutableOperationPackage, TypeId};
#[test]
fn current_package_codec_prepares_effects_without_a_parallel_package_definition() {
    let mut program = program();
    set(&mut program, "schema", text("echo.operation-program/v1"));
    let mut publication = package(&program);
    set(
        &mut publication,
        "schema",
        text("echo.operation-package/v1"),
    );
    let bytes = encoded(&publication);
    let prepared =
        ExecutableOperationPackage::prepare(&bytes, hash(b"echo:operation-package:v1\0", &bytes))
            .expect("one current decoder accepts the effect program");
    assert_eq!(prepared.to_canonical_bytes().expect("canonical"), bytes);
    let effect = prepared
        .program()
        .as_effect_program()
        .expect("effect program kind");
    assert_eq!(effect.compiled_program(), [0xa0]);
    assert!(effect
        .graph_schema()
        .allows_atom(TypeId([1; 32]), TypeId([2; 32])));
    assert_eq!(prepared.lawpack_identity(), None);
}

#[test]
fn current_package_uses_program_kind_to_require_or_forbid_a_real_lawpack() {
    use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as V};
    use warp_core::{EchoOperationBudgetV1, EchoOperationProgram, EchoOperationSemanticClosure};
    let fixed = ExecutableOperationPackage::new(
        "app.fixed@1",
        "app.Refusal",
        EchoOperationSemanticClosure::new(
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            "app.Schema",
            [5; 32],
            "app.Law",
            [6; 32],
        ),
        warp_core::echo_operation_target_profile_identity_v1(),
        [8; 32],
        EchoOperationBudgetV1::new(4, 64, 32),
        EchoOperationProgram::anchored_node_attachment_compare_and_set(
            TypeId([1; 32]),
            TypeId([2; 32]),
            16,
        ),
    );
    let fixed_bytes = fixed.to_canonical_bytes().expect("fixed publication");
    assert!(ExecutableOperationPackage::prepare(
        &fixed_bytes,
        hash(b"echo:operation-package:v1\0", &fixed_bytes)
    )
    .is_ok());
    let mut fixed_value = decode(&fixed_bytes).expect("canonical fixed publication");
    let V::Map(fields) = &mut fixed_value else {
        panic!("map")
    };
    let (_, V::Map(closure)) = fields
        .iter_mut()
        .find(|(k, _)| k == &text("semantic_closure"))
        .expect("closure")
    else {
        panic!("closure map")
    };
    closure
        .retain(|(key, _)| key != &text("lawpack_coordinate") && key != &text("lawpack_identity"));
    refused(&fixed_value);

    let mut effect = package(&program());
    let V::Map(fields) = &mut effect else {
        panic!("map")
    };
    let (_, closure) = fields
        .iter_mut()
        .find(|(k, _)| k == &text("semantic_closure"))
        .expect("closure");
    add(closure, "lawpack_coordinate", text("app.Law"));
    add(closure, "lawpack_identity", id(6));
    refused(&effect);
}

#[test]
fn historical_package_and_program_versions_are_refused_even_with_matching_pins() {
    let mut value = package(&program());
    set(&mut value, "schema", text("echo.operation-package/v2"));
    refused(&value);
    let mut old_program = program();
    set(
        &mut old_program,
        "schema",
        text("echo.operation-program/v2"),
    );
    refused(&package(&old_program));
}
