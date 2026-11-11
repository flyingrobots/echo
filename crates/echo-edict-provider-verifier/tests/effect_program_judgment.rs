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
    "two-let-control",
    "forged-node-order",
    "wrong-intrinsic-input-type",
    "read-exports-control",
    "undeclared-obstruction",
    "read-adapter-control",
    "failure-mapping-tag-spelled",
    "failure-mapping-unknown",
    "failure-mapping-duplicate",
    "constraint-proven-by-constraint",
    "constraint-conjunct-proven",
    "over-budget",
    "mutation-allocation-exact",
    "mutation-allocation-short",
    "require-payload-over-output",
    "mutation-count-over",
    "local-name-empty",
    "constraint-without-coordinate",
    "mutation-input-nominal",
    "mutation-result-nominal",
    "nominal-extra-field",
    "failure-declared-twice",
    "basis-control",
    "basis-not-bytes",
    "type-table-width",
    "type-table-int",
    "require-payload-fits",
    "mutation-count-control",
    "read-and-return-control",
    "read-allocation-short",
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
    // Every disagreement is reported at once, not only the first.
    let mut disagreements = Vec::new();
    for (name, accept) in verdicts {
        let judged = verify_effect_package(&package_bytes(&name));
        let expected = if accept {
            Ok(())
        } else {
            Err(EffectPackageRefusal::UnsupportedProgram)
        };
        if judged != expected {
            disagreements.push(format!("{name}: expected {expected:?}, got {judged:?}"));
        }
    }
    assert_eq!(disagreements, Vec::<String>::new());
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
        "canonical_meaning_identity",
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

/// Rewrites one field of the package's embedded program.
fn with_program(package: &V, edit: impl FnOnce(&mut V)) -> V {
    let mut package = package.clone();
    let V::Bytes(program) = field_mut(&mut package, "program").clone() else {
        panic!("program")
    };
    let mut program = decode(&program).expect("program");
    edit(&mut program);
    *field_mut(&mut package, "program") = V::Bytes(encode(&program).expect("program"));
    package
}
fn insert(value: &mut V, key: &str, field: V) {
    let V::Map(fields) = value else { panic!("map") };
    fields.push((V::Text(key.into()), field));
}
fn flip(value: &mut V) {
    let V::Bytes(bytes) = value else {
        panic!("bytes")
    };
    bytes[0] ^= 1;
}

/// The program's own Target IR claim and the footprint contract name the
/// program the package carries.
#[test]
fn the_program_target_ir_and_footprint_claims_are_bound() {
    let original = decode(&package_bytes("read-control")).expect("package");
    let program_target = with_program(&original, |program| {
        flip(field_mut(program, "target_ir_identity"));
    });
    let mut footprint = original;
    flip(field_mut(&mut footprint, "footprint_contract_identity"));
    for (name, package) in [
        ("program target_ir_identity", program_target),
        ("footprint_contract_identity", footprint),
    ] {
        assert_eq!(
            verify_effect_package(&encode(&package).expect("package")),
            Err(EffectPackageRefusal::InvalidBinding),
            "{name}"
        );
    }
}

/// A package spelled other than canonically is not verifiable: the same
/// package spelled two ways would have two identities. The canonical decoder
/// already refuses this non-minimal map head; the verifier's re-encoding
/// check is a second guard this input does not reach.
#[test]
fn decodable_non_canonical_bytes_are_refused() {
    let canonical = package_bytes("read-control");
    // A map head with its entry count in a one-byte argument it does not
    // need: the same map, spelled at a longer length.
    let head = canonical[0];
    assert!((0xa0..0xb8).contains(&head), "a short map head");
    let mut long = vec![0xb8, head - 0xa0];
    long.extend_from_slice(&canonical[1..]);
    assert_eq!(
        verify_effect_package(&long),
        Err(EffectPackageRefusal::InvalidEnvelope)
    );
}

/// What Echo's runtime refuses when it admits a package envelope, the
/// verifier refuses before installation: the exact field sets, the fixed
/// schema constants, the effect-program limits and a nonzero step ceiling.
#[test]
fn package_envelopes_the_runtime_cannot_admit_are_refused() {
    let original = decode(&package_bytes("read-control")).expect("package");
    verify_effect_package(&encode(&original).expect("package")).expect("envelope control");
    let package = |edit: &dyn Fn(&mut V)| {
        let mut package = original.clone();
        edit(&mut package);
        package
    };
    let program = |edit: &dyn Fn(&mut V)| with_program(&original, edit);
    let cases = [
        (
            "an unknown package field",
            package(&|p| insert(p, "unexpected", V::Null)),
        ),
        (
            "a result projection on an effect package",
            package(&|p| insert(p, "application_result_projection", V::Null)),
        ),
        (
            "a zero step ceiling",
            package(&|p| {
                *field_mut(field_mut(p, "budget_ceiling"), "steps") = V::Integer(0);
            }),
        ),
        (
            "a lawpack in an effect closure",
            package(&|p| {
                let closure = field_mut(p, "semantic_closure");
                insert(closure, "lawpack_coordinate", V::Text("app.Lawpack".into()));
                insert(closure, "lawpack_identity", V::Bytes(vec![7; 32]));
            }),
        ),
        (
            "an unknown program field",
            program(&|p| insert(p, "unexpected", V::Null)),
        ),
        (
            "another identity law",
            program(&|p| *field_mut(p, "identity_law") = V::Integer(2)),
        ),
        (
            "another interpreter profile",
            program(&|p| flip(field_mut(p, "interpreter_profile_identity"))),
        ),
        (
            "another intrinsic profile",
            program(&|p| flip(field_mut(p, "intrinsic_profile_identity"))),
        ),
        (
            "an input ceiling above 64 KiB",
            program(&|p| *field_mut(p, "max_input_bytes") = V::Integer(65_537)),
        ),
        (
            "a result ceiling above 64 KiB",
            program(&|p| *field_mut(p, "max_result_bytes") = V::Integer(65_537)),
        ),
        (
            "an allocation ceiling above 16 MiB",
            program(&|p| *field_mut(p, "max_allocated_bytes") = V::Integer(16_777_217)),
        ),
        (
            "an empty application schema coordinate",
            package(&|p| {
                *field_mut(
                    field_mut(p, "semantic_closure"),
                    "application_schema_coordinate",
                ) = V::Text(String::new());
            }),
        ),
        (
            "an undecodable graph schema",
            program(&|p| *field_mut(p, "graph_schema") = V::Bytes(Vec::new())),
        ),
        (
            "an undecodable footprint law",
            program(&|p| *field_mut(p, "footprint_law") = V::Bytes(Vec::new())),
        ),
        (
            "an unknown budget field",
            package(&|p| insert(field_mut(p, "budget_ceiling"), "unexpected", V::Integer(0))),
        ),
        // The verifier binds neither identity to a value, but each must be
        // the 32-byte hash the runtime decodes.
        (
            "an authority profile that is not a hash",
            package(&|p| *field_mut(p, "authority_profile_identity") = V::Null),
        ),
        (
            "a source identity that is not a hash",
            package(&|p| {
                *field_mut(field_mut(p, "semantic_closure"), "edict_source_identity") =
                    V::Bytes(vec![7; 31]);
            }),
        ),
    ];
    let mut cases = cases.to_vec();
    // Every fixed package profile and schema identity names the effect
    // profile the runtime supports.
    for field in [
        "target_profile_identity",
        "interpreter_profile_identity",
        "intrinsic_profile_identity",
        "input_schema_identity",
        "result_schema_identity",
        "obstruction_schema_identity",
        "result_interpretation_identity",
        "obstruction_interpretation_identity",
        "application_basis_schema_identity",
        "evaluation_basis_schema_identity",
    ] {
        cases.push((field, package(&|p| flip(field_mut(p, field)))));
    }
    let admitted: Vec<_> = cases
        .into_iter()
        .filter(|(_, package)| {
            verify_effect_package(&encode(package).expect("package"))
                != Err(EffectPackageRefusal::InvalidEnvelope)
        })
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        admitted,
        Vec::<&str>::new(),
        "envelopes the verifier did not refuse"
    );
}
