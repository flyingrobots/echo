// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The compiler-facing checker needs no trusted-host feature or authority.
#![allow(clippy::expect_used, clippy::panic)]
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V,
};
use warp_core::{echo_operation_package_id_v1, EffectPackageError, ExecutableOperationPackage};

fn fixture() -> Vec<u8> {
    hex::decode(include_str!("fixtures/current-effect-publication/package.hex").trim())
        .expect("synthetic control")
}
fn replace(value: &mut V, key: &str, replacement: V) {
    let V::Map(entries) = value else {
        panic!("map")
    };
    entries
        .iter_mut()
        .find(|(k, _)| k == &V::Text(key.into()))
        .expect("field")
        .1 = replacement;
}
fn check(bytes: &[u8]) -> bool {
    ExecutableOperationPackage::prepare(bytes, echo_operation_package_id_v1(bytes).as_hash())
        .is_ok()
}

#[test]
fn current_publication_is_checked_without_host_authority_features() {
    let bytes = fixture();
    let prepared =
        ExecutableOperationPackage::prepare(&bytes, echo_operation_package_id_v1(&bytes).as_hash())
            .expect("current effect publication is data");
    assert_eq!(
        prepared.to_canonical_bytes().expect("canonical round trip"),
        bytes
    );
    assert_eq!(
        ExecutableOperationPackage::prepare(&bytes, [0; 32]),
        Err(EffectPackageError::IdentityMismatch)
    );
}

#[test]
fn fresh_outer_pin_does_not_make_invalid_compiler_bytes_valid() {
    let mut package = decode(&fixture()).expect("package");
    let V::Map(entries) = &package else {
        panic!("map")
    };
    let V::Bytes(raw) = &entries
        .iter()
        .find(|(k, _)| k == &V::Text("program".into()))
        .expect("program")
        .1
    else {
        panic!("bytes")
    };
    let mut program = decode(raw).expect("program");
    replace(&mut program, "compiled_program", V::Bytes(vec![0xa0]));
    replace(
        &mut package,
        "program",
        V::Bytes(encode(&program).expect("encode program")),
    );
    assert!(!check(&encode(&package).expect("encode package")));
}

#[test]
fn refreshed_pin_cannot_change_the_selected_core_binding() {
    let mut package = decode(&fixture()).expect("package");
    let V::Map(entries) = &package else {
        panic!("map")
    };
    let mut closure = entries
        .iter()
        .find(|(k, _)| k == &V::Text("semantic_closure".into()))
        .expect("closure")
        .1
        .clone();
    replace(&mut closure, "core_identity", V::Bytes(vec![99; 32]));
    replace(&mut package, "semantic_closure", closure);
    assert!(!check(
        &encode(&package).expect("freshly pinned changed closure")
    ));
}
