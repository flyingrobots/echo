// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The verifier judges authored effect packages itself, independently of the
//! lowerer and the runtime, and reaches the runtime's verdict on the shared
//! retained corpus.
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V,
};
use echo_edict_provider_verifier::effect_package::{verify_effect_package, EffectPackageRefusal};

/// The retained corpus, embedded so the judgment reads no files.
macro_rules! retained {
    ($($name:literal),* $(,)?) => {
        &[$((
            $name,
            include_str!(concat!("../../echo-edict-provider-lowerer/tests/fixtures/effect-programs/", $name, ".hex")),
        )),*]
    };
}
const RETAINED: &[(&str, &str)] = retained![
    "read-control",
    "ensure-node-control",
    "slice-proven",
    "slice-unproven",
    "slice-other-operand",
    "slice-require-only",
    "forged-node-order",
    "wrong-intrinsic-input-type",
    "undeclared-obstruction",
    "failure-mapping-tag-spelled",
    "failure-mapping-unknown",
    "failure-mapping-duplicate",
    "over-budget",
    "read-captured-budget",
];
const VERDICTS: &str =
    include_str!("../../echo-edict-provider-lowerer/tests/fixtures/effect-programs/verdicts.txt");

fn package_bytes(name: &str) -> Vec<u8> {
    let hex = RETAINED
        .iter()
        .find_map(|(entry, hex)| (*entry == name).then_some(*hex))
        .expect(name);
    hex::decode(hex.trim()).expect("hex")
}
fn field_mut<'a>(value: &'a mut V, key: &str) -> &'a mut V {
    let V::Map(fields) = value else { panic!("map") };
    &mut fields
        .iter_mut()
        .find(|(k, _)| k == &V::Text(key.into()))
        .unwrap_or_else(|| panic!("field {key}"))
        .1
}
fn verdicts() -> Vec<(String, bool)> {
    VERDICTS
        .lines()
        .map(|line| {
            let (name, verdict) = line.split_once(' ').expect("entry");
            (name.to_owned(), verdict == "accept")
        })
        .collect()
}

#[test]
fn the_verifier_reaches_every_retained_corpus_verdict() {
    let verdicts = verdicts();
    assert_eq!(
        verdicts
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        RETAINED.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "every verdict has its embedded package, in order"
    );
    assert!(verdicts.iter().filter(|(_, accept)| *accept).count() >= 3);
    assert!(verdicts.iter().filter(|(_, accept)| !*accept).count() >= 10);
    for (name, accept) in verdicts {
        let judged = verify_effect_package(&package_bytes(&name));
        if accept {
            judged.unwrap_or_else(|e| panic!("{name}: verifier refused an accepted package: {e}"));
        } else {
            assert_eq!(
                judged,
                Err(EffectPackageRefusal::UnsupportedProgram),
                "{name}: verifier accepted a refused package"
            );
        }
    }
}

/// A valid program cannot be republished under another package's identities:
/// the verifier rebinds every identity the package claims to the program it
/// actually carries.
#[test]
fn package_identities_must_name_the_program_they_carry() {
    let accepted = package_bytes("read-control");
    verify_effect_package(&accepted).expect("rebinding control");
    let original = decode(&accepted).expect("package");
    for key in [
        "core_identity",
        "target_ir_identity",
        "application_schema_identity",
    ] {
        let mut package = original.clone();
        let V::Bytes(identity) = field_mut(field_mut(&mut package, "semantic_closure"), key) else {
            panic!("identity")
        };
        identity[0] ^= 1;
        assert_eq!(
            verify_effect_package(&encode(&package).expect("package")),
            Err(EffectPackageRefusal::InvalidBinding),
            "{key}"
        );
    }
    let mut package = original.clone();
    *field_mut(&mut package, "operation_coordinate") = V::Text("jedit.text@1.otherIntent".into());
    assert_eq!(
        verify_effect_package(&encode(&package).expect("package")),
        Err(EffectPackageRefusal::InvalidBinding)
    );
    let mut package = original;
    let V::Bytes(program) = field_mut(&mut package, "program").clone() else {
        panic!("program")
    };
    let mut program = decode(&program).expect("program");
    *field_mut(&mut program, "output_type") = V::Text("app.Other".into());
    *field_mut(&mut package, "program") = V::Bytes(encode(&program).expect("program"));
    assert_eq!(
        verify_effect_package(&encode(&package).expect("package")),
        Err(EffectPackageRefusal::InvalidBinding)
    );
}

#[test]
fn only_authored_effect_programs_are_judged_here() {
    let mut package = decode(&package_bytes("slice-proven")).expect("package");
    let V::Bytes(program) = field_mut(&mut package, "program").clone() else {
        panic!("program")
    };
    let mut program = decode(&program).expect("program");
    *field_mut(&mut program, "kind") = V::Text("edict-pure-program/v1".into());
    *field_mut(&mut package, "program") = V::Bytes(encode(&program).expect("program"));
    assert_eq!(
        verify_effect_package(&encode(&package).expect("package")),
        Err(EffectPackageRefusal::InvalidEnvelope)
    );
    assert_eq!(
        verify_effect_package(b"not canonical"),
        Err(EffectPackageRefusal::InvalidEnvelope)
    );
}
