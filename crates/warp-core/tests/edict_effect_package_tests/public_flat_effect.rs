// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A flat authored effect program published by the public compiler and run by
//! the generic runtime.
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-effect-program/`. Its identity is recorded here
//! independently of the bytes. Oracle: the expected result is the authored
//! program's own law (`return input.count` through one pure binding), written
//! by hand; the refusals are the runtime's identity and substitution laws.
use super::execution::{host, limits};
use super::*;
use warp_core::edict_effects::EffectPackageError;
use warp_core::{EchoOperationApplicationBasisV1, EchoOperationBudgetV1};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-program/flat-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "2ac246d890ee7c965878a43f4da21166f0a2dab81c677f2fe29c5e84ce5a47e8";

fn retained() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained hex");
    let identity = hash(b"echo:operation-package:v1\0", &bytes);
    assert_eq!(
        hex::encode(identity),
        PACKAGE_PIN,
        "retained package identity"
    );
    (bytes, identity)
}

fn input(count: u64) -> Vec<u8> {
    encoded(&map([
        ("anchor", V::Bytes(vec![0x5a; 32])),
        ("count", V::Integer(i128::from(count))),
    ]))
}

fn basis(input: &[u8]) -> EchoOperationApplicationBasisV1 {
    EchoOperationApplicationBasisV1::new(
        hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        hash(b"echo:operation-edict-effect-basis:v1\0", input),
    )
}

#[test]
fn the_public_flat_effect_package_runs_on_the_generic_runtime() {
    let (bytes, identity) = retained();
    let package = ExecutableOperationPackage::prepare(&bytes, identity).expect("public package");
    let program = package
        .program()
        .as_effect_program()
        .expect("effect program");
    let input = input(7);
    let (host, head, _) = host();
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(4096, 0, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    assert_eq!(
        frame
            .evaluate_effect(program, &input, &[])
            .expect("execute"),
        encoded(&V::Integer(7))
    );
    let completed = frame
        .into_effect_evaluation(64)
        .expect("completed evidence");
    assert_eq!(completed.output(), encoded(&V::Integer(7)));
    assert!(completed.mutations().is_empty());
    assert_eq!(completed.input_basis(), basis(&input));
}

#[test]
fn another_package_cannot_be_admitted_under_the_public_pin() {
    let (bytes, identity) = retained();
    // A different, well-formed effect package.
    let (other, _) = super::mutations::fixture("ensure-node", "continuum.profile.graph-effects/v1");
    let other = encoded(&other);
    assert!(
        prepare(&decode_value(&other)).is_ok(),
        "the substitute is itself valid"
    );
    assert!(matches!(
        ExecutableOperationPackage::prepare(&other, identity),
        Err(EffectPackageError::IdentityMismatch)
    ));
    // The public package re-bound to a different result: internally coherent,
    // so it prepares under its own pin, but never under the public one.
    let rebound = rebound_to_constant(&bytes);
    assert!(
        prepare(&rebound).is_ok(),
        "the re-bound package is coherent"
    );
    assert!(matches!(
        ExecutableOperationPackage::prepare(&encoded(&rebound), identity),
        Err(EffectPackageError::IdentityMismatch)
    ));
}

fn decode_value(bytes: &[u8]) -> V {
    echo_edict_canonical::decode_canonical_cbor_v1(bytes).expect("canonical")
}

fn set_bytes(value: &mut V, key: &str, edit: impl FnOnce(&mut V)) {
    let V::Bytes(raw) = value_field_mut(value, key) else {
        panic!("{key} bytes")
    };
    let mut inner = decode_value(raw);
    edit(&mut inner);
    *value_field_mut(value, key) = V::Bytes(encoded(&inner));
}

/// The public package with its result replaced by the constant 9 in Core and
/// Target IR alike, every embedded and outer identity re-derived.
fn rebound_to_constant(bytes: &[u8]) -> V {
    let constant = map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", V::Integer(9)),
            ]),
        ),
    ]);
    let mut package = decode_value(bytes);
    let mut core_id = V::Null;
    let mut target_id = V::Null;
    set_bytes(&mut package, "program", |program| {
        set_bytes(program, "compiled_program", |bundle| {
            set_bytes(bundle, "core_artifact", |core| {
                set(core_body_mut(core), "result", constant.clone());
                core_id = artifact_identity("edict.core.module/v1", core);
            });
            let core = decode_value(match get(bundle, "core_artifact") {
                V::Bytes(raw) => raw,
                _ => panic!("core bytes"),
            });
            set_bytes(bundle, "target_ir_artifact", |target| {
                set(selected_target_mut(target), "result", constant.clone());
                repin_core(&core, target);
                target_id = artifact_identity("edict.target-ir.artifact/v1", target);
            });
        });
        set(program, "target_ir_identity", target_id.clone());
    });
    let closure = value_field_mut(&mut package, "semantic_closure");
    set(closure, "core_identity", core_id.clone());
    set(closure, "canonical_meaning_identity", core_id);
    set(closure, "target_ir_identity", target_id);
    package
}
