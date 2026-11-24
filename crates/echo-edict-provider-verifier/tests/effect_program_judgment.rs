// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The verifier judges authored effect packages itself, independently of the
//! lowerer and the runtime, and reaches the runtime's verdict on the shared
//! retained corpus.
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V,
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
    "echo-dpo-2-ensure-node-control",
    "read-without-failures",
    "read-catalog-without-failures",
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
    "typed-failure-control",
    "typed-failure-undeclared",
    "typed-failure-unmapped",
    "typed-failure-unknown-arm",
    "typed-failure-repeated-set",
    "typed-failure-empty-set",
    "typed-failure-without-capability",
    "typed-failure-arms-differ",
    "imported-helper-total",
    "imported-helper-fails",
    "imported-helper-fails-in-arm",
    "imported-helper-fails-attempted",
    "ordered-record-z-first",
    "ordered-record-a-first",
    "ordered-record-guarded-z-first",
    "ordered-record-guarded-a-first",
    "ordered-record-duplicate-name",
    "ordered-record-empty",
    "ordered-record-total",
    "ordered-record-unordered-fallible",
    "ordered-record-extra-entry-key",
    "structured-loop-control",
    "structured-branch-control",
    "structured-nested-control",
    "structured-echo-dpo-1",
    "structured-bound-below-list",
    "structured-coordinate-bound",
    "structured-leaked-local",
    "structured-control-id-reused",
    "structured-dropped-body",
    "structured-ordered-with-control",
    "structured-substituted-binder",
    "structured-body-with-result",
    "structured-loop-allocation-exact",
    "structured-loop-allocation-short",
    "structured-loop-under-budget",
    "structured-mutation-count-control",
    "structured-mutation-count-over",
];
const VERDICTS: &str =
    include_str!("../../echo-edict-provider-lowerer/tests/fixtures/effect-programs/verdicts.txt");

/// Public compiler witnesses kept outside the shared corpus, beside the
/// runtime tests that run them.
const WITNESSES: &[(&str, &str)] = &[
    (
        "closure-read-witness",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-closure-read/closure-executable-operation-package.hex"
        ),
    ),
    (
        "identity-witness",
        include_str!(
            "../../warp-core/tests/fixtures/edict-content-address/identity-executable-operation-package.hex"
        ),
    ),
    (
        "attachment-digest-witness",
        include_str!(
            "../../warp-core/tests/fixtures/edict-attachment-digest/bump-executable-operation-package.hex"
        ),
    ),    (
        "option-value-witness",
        include_str!(
            "../../warp-core/tests/fixtures/edict-option-value/option-executable-operation-package.hex"
        ),
    ),    (
        "typed-failure-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/typed-executable-operation-package.hex"
        ),
    ),
    (
        "payload-attempt-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-attempt-executable-operation-package.hex"
        ),
    ),
    (
        "payload-require-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-require-executable-operation-package.hex"
        ),
    ),
];

fn package_bytes(name: &str) -> Vec<u8> {
    let hex = RETAINED
        .iter()
        .chain(WITNESSES)
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
fn zero_effect_package() -> Vec<u8> {
    let bundle = hex::decode(
        include_str!(
            "../../echo-edict-provider-lowerer/tests/fixtures/current-effect-wire/bundle.hex"
        )
        .trim(),
    )
    .expect("zero-effect compiler bundle");
    let source = hex::decode(
        include_str!(
            "../../echo-edict-provider-lowerer/tests/fixtures/current-effect-wire/source.hex"
        )
        .trim(),
    )
    .expect("source fixture");
    let configuration = hex::decode(
        include_str!("../../echo-edict-provider-lowerer/tests/fixtures/current-effect-wire/configuration.hex")
            .trim(),
    )
    .expect("configuration fixture");
    echo_edict_provider_lowerer::effect_publication::encode_effect_publication(
        echo_edict_provider_lowerer::effect_publication::EffectPublicationInput {
            compiler_program: &bundle,
            application_schema_coordinate: "app.Exports",
            source_artifact: &source,
            source_domain: "edict.source/v1",
            configuration: &configuration,
        },
    )
    .expect("frozen zero-effect @1 serialization control")
}

fn zero_effect_package_with_dpo_2_target_profile() -> Vec<u8> {
    let mut package = decode(&zero_effect_package()).expect("@1 package");
    let V::Bytes(program_bytes) = field_mut(&mut package, "program").clone() else {
        panic!("program bytes")
    };
    let mut program = decode(&program_bytes).expect("program");
    let V::Bytes(bundle_bytes) = field_mut(&mut program, "compiled_program").clone() else {
        panic!("compiler bundle bytes")
    };
    let mut bundle = decode(&bundle_bytes).expect("compiler bundle");
    let V::Bytes(target_bytes) = field_mut(&mut bundle, "target_ir_artifact").clone() else {
        panic!("Target IR bytes")
    };
    let mut target = decode(&target_bytes).expect("Target IR");
    let profile = decode(include_bytes!(
        "../resources/target-profile.echo-dpo-2.cbor"
    ))
    .expect("current @2 profile");
    let profile_digest = digest_canonical_value_bytes_v1("edict.target-profile/v1", &profile)
        .expect("@2 profile identity");
    *field_mut(&mut target, "targetProfile") = V::Map(vec![
        (V::Text("id".into()), V::Text("echo.dpo@2".into())),
        (
            V::Text("digest".into()),
            V::Array(vec![
                V::Text("sha256".into()),
                V::Bytes(profile_digest.to_vec()),
            ]),
        ),
    ]);
    let target_identity = digest_canonical_value_bytes_v1("edict.target-ir.artifact/v1", &target)
        .expect("Target IR identity");
    *field_mut(&mut bundle, "target_ir_artifact") = V::Bytes(encode(&target).expect("Target IR"));
    *field_mut(&mut program, "compiled_program") =
        V::Bytes(encode(&bundle).expect("compiler bundle"));
    *field_mut(&mut program, "target_ir_identity") = V::Bytes(target_identity.to_vec());
    *field_mut(&mut package, "program") = V::Bytes(encode(&program).expect("program"));
    *field_mut(
        field_mut(&mut package, "semantic_closure"),
        "target_ir_identity",
    ) = V::Bytes(target_identity.to_vec());
    encode(&package).expect("@2 zero-effect package")
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

#[test]
fn verifier_pure_judgment_keeps_the_zero_effect_echo_dpo_1_control() {
    assert_eq!(
        verify_effect_package(&zero_effect_package()),
        Ok(()),
        "the frozen ordered @1 zero-effect package remains admissible"
    );
}

#[test]
fn verifier_pure_judgment_rejects_zero_effect_echo_dpo_2_without_profile_artifacts() {
    let refusal = verify_effect_package(&zero_effect_package_with_dpo_2_target_profile());
    assert!(
        matches!(refusal, Err(EffectPackageRefusal::UnsupportedProgram)),
        "expected pure verifier to refuse unauthenticated @2 zero-effect Target; got {refusal:?}"
    );
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

/// A retained package edited and repinned: its Core requires `capability` as
/// well, when given, and its intent's Core budget (in Core and Target IR
/// alike) is set on the axes given. Every Core and Target IR pin is
/// recomputed.
fn edited(
    name: &str,
    capability: Option<&str>,
    steps: Option<u64>,
    bytes_: Option<u64>,
) -> Vec<u8> {
    let bytes_of = |value: &V| {
        let V::Bytes(bytes) = value else {
            panic!("bytes")
        };
        bytes.clone()
    };
    let mut package = decode(&package_bytes(name)).expect("package");
    let mut program = decode(&bytes_of(field_mut(&mut package, "program"))).expect("program");
    let mut bundle =
        decode(&bytes_of(field_mut(&mut program, "compiled_program"))).expect("compiler bundle");
    let V::Text(intent) = field_mut(&mut bundle, "intent").clone() else {
        panic!("intent")
    };
    let mut module = decode(&bytes_of(field_mut(&mut bundle, "core_artifact"))).expect("Core");
    if let Some(capability) = capability {
        let V::Array(capabilities) = field_mut(&mut module, "requiredCoreCapabilities") else {
            panic!("capabilities")
        };
        capabilities.push(V::Text(capability.into()));
    }
    let mut target =
        decode(&bytes_of(field_mut(&mut bundle, "target_ir_artifact"))).expect("Target IR");
    for (axis, value) in [("maxSteps", steps), ("maxAllocatedBytes", bytes_)] {
        let Some(value) = value else { continue };
        let value = V::Integer(i128::from(value));
        *field_mut(
            field_mut(
                field_mut(field_mut(&mut module, "intents"), &intent),
                "coreEvaluationBudget",
            ),
            axis,
        ) = value.clone();
        let V::Map(intents) = field_mut(&mut target, "intents") else {
            panic!("Target IR intents")
        };
        *field_mut(field_mut(&mut intents[0].1, "coreEvaluationBudget"), axis) = value;
    }
    let core_identity =
        digest_canonical_value_bytes_v1("edict.core.module/v1", &module).expect("Core identity");
    *field_mut(
        field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore"),
        "digest",
    ) = V::Array(vec![
        V::Text("sha256".into()),
        V::Bytes(core_identity.to_vec()),
    ]);
    let target_identity = digest_canonical_value_bytes_v1("edict.target-ir.artifact/v1", &target)
        .expect("Target IR identity");
    *field_mut(&mut bundle, "core_artifact") = V::Bytes(encode(&module).expect("Core"));
    *field_mut(&mut bundle, "target_ir_artifact") = V::Bytes(encode(&target).expect("Target IR"));
    *field_mut(&mut program, "compiled_program") =
        V::Bytes(encode(&bundle).expect("compiler bundle"));
    *field_mut(&mut program, "target_ir_identity") = V::Bytes(target_identity.to_vec());
    *field_mut(&mut package, "program") = V::Bytes(encode(&program).expect("program"));
    let closure = field_mut(&mut package, "semantic_closure");
    *field_mut(closure, "core_identity") = V::Bytes(core_identity.to_vec());
    *field_mut(closure, "canonical_meaning_identity") = V::Bytes(core_identity.to_vec());
    *field_mut(closure, "target_ir_identity") = V::Bytes(target_identity.to_vec());
    encode(&package).expect("package")
}

const SHARED: &str = "core.cost.shared-reads/v1";

/// Programs whose expressions all have a shared-read rule in Echo.
const SHARED_PROGRAMS: [&str; 5] = [
    "read-control",
    "ensure-node-control",
    "structured-loop-control",
    "structured-branch-control",
    "structured-nested-control",
];

/// The smallest step and allocation budgets the verifier admits, each
/// searched with the other axis ample.
fn minimum(name: &str, capability: Option<&str>) -> (u64, u64) {
    const AMPLE: u64 = 1 << 40;
    let search = |admits: &dyn Fn(u64) -> bool| {
        assert!(admits(AMPLE), "{name}: refused even with an ample budget");
        let (mut low, mut high) = (0_u64, AMPLE);
        while low < high {
            let middle = low + (high - low) / 2;
            if admits(middle) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        high
    };
    let steps = search(&|steps| {
        verify_effect_package(&edited(name, capability, Some(steps), Some(AMPLE))).is_ok()
    });
    let bytes_ = search(&|bytes_| {
        verify_effect_package(&edited(name, capability, Some(AMPLE), Some(bytes_))).is_ok()
    });
    (steps, bytes_)
}

/// Control for the repinning: an unused capability added the same way still
/// verifies.
#[test]
fn an_unused_capability_added_by_the_same_edit_still_verifies() {
    assert_eq!(
        verify_effect_package(&edited("read-control", Some("core.variant/v1"), None, None)),
        Ok(())
    );
}

/// An effect package that requires the shared-read schedule is audited under
/// it (echo#1016).
#[test]
fn effect_packages_are_audited_under_the_shared_read_schedule() {
    for name in SHARED_PROGRAMS {
        assert_eq!(
            verify_effect_package(&edited(name, Some(SHARED), None, None)),
            Ok(()),
            "{name}"
        );
    }
}

/// Byte operations under the shared-read schedule (echo#1016, family 5):
/// the verifier's minimums under both schedules equal the lowerer's, which
/// `byte_operation_minimums_under_each_schedule` in the lowerer pins
/// independently (oracle: the other judge).
#[test]
fn byte_operation_minimums_equal_the_lowerer_minimums() {
    let measured: Vec<_> = ["slice-proven", "typed-failure-control"]
        .into_iter()
        .map(|name| (name, minimum(name, None), minimum(name, Some(SHARED))))
        .collect();
    assert_eq!(
        measured,
        [
            ("slice-proven", (195, 3383), (873, 971)),
            ("typed-failure-control", (74, 3312), (374, 1231)),
        ]
    );
}

/// The verifier's minimums under both schedules equal the lowerer's, which
/// `effect_program_minimums_under_each_schedule` in the lowerer pins
/// independently (oracle: the other judge).
#[test]
fn effect_package_minimums_equal_the_lowerer_minimums() {
    let measured: Vec<_> = SHARED_PROGRAMS
        .iter()
        .map(|name| (*name, minimum(name, None), minimum(name, Some(SHARED))))
        .collect();
    assert_eq!(
        measured,
        [
            (
                "read-control",
                (2_097_226, 9_441_348),
                (2_097_261, 4_195_841)
            ),
            ("ensure-node-control", (41, 4258), (34, 2301)),
            ("structured-loop-control", (173, 18_325), (122, 6903)),
            ("structured-branch-control", (179, 18_453), (192, 7031)),
            ("structured-nested-control", (456, 48_136), (284, 13_742)),
        ]
    );
}

/// The typed-failure program's `attempt` under the shared-read schedule
/// (echo#1016, amblicit#341): the verifier's minimum equals the lowerer's,
/// which `an_attempt_is_charged_what_the_shared_runtime_charges` in the
/// lowerer pins independently (oracle: the other judge).
#[test]
fn an_attempt_minimum_equals_the_lowerer_minimum() {
    assert_eq!(
        minimum("typed-failure-v2-admission", Some(SHARED)),
        (140, 2_095)
    );
}

/// A one-field failure payload `{ at: input.start }` under the shared-read
/// schedule (echo#1016), as an `attempt` arm's and as a `require`'s: the
/// verifier's minimums equal the lowerer's, which
/// `a_failure_payload_field_is_charged_what_the_shared_runtime_charges` in
/// the lowerer pins independently (oracle: the other judge).
#[test]
fn a_failure_payload_field_minimum_equals_the_lowerer_minimum() {
    assert_eq!(
        [
            minimum("payload-attempt-v2-admission", Some(SHARED)),
            minimum("payload-require-v2-admission", Some(SHARED)),
        ],
        [(145, 2_291), (131, 2_227)]
    );
}

/// The identity witnesses (all four `echo.dpo@2` identity intrinsics) are
/// audited under the shared-read schedule (echo#1016, family 3), and the
/// verifier's minimums under both schedules equal the lowerer's, which
/// `identity_witness_minimums_under_each_schedule` in the lowerer pins
/// independently (oracle: the other judge).
#[test]
fn identity_witness_minimums_equal_the_lowerer_minimums() {
    let mut measured = Vec::new();
    for name in ["identity-witness", "attachment-digest-witness"] {
        assert_eq!(
            verify_effect_package(&edited(name, Some(SHARED), None, None)),
            Ok(()),
            "{name}: shared"
        );
        measured.push((name, minimum(name, None), minimum(name, Some(SHARED))));
    }
    assert_eq!(
        measured,
        [
            ("identity-witness", (1898, 71_061), (1262, 11_580)),
            ("attachment-digest-witness", (371, 11_334), (289, 3209)),
        ]
    );
}

/// The compiler-produced closure-read witness is audited under the
/// shared-read schedule, and the verifier's minimums under both schedules
/// equal the lowerer's, which `closure_read_witness_minimums_under_each_schedule`
/// in the lowerer pins independently (oracle: the other judge). Under the
/// shared schedule the declared selector is charged at its shared-read call
/// charge (echo#1016).
#[test]
fn closure_read_witness_minimums_equal_the_lowerer_minimums() {
    let name = "closure-read-witness";
    assert_eq!(
        verify_effect_package(&edited(name, Some(SHARED), None, None)),
        Ok(()),
        "shared"
    );
    assert_eq!(
        (minimum(name, None), minimum(name, Some(SHARED))),
        ((15_367, 932_374), (10_814, 194_487))
    );
}

/// Family 4 (`Option` constructors, ordered records whose fields `fail`,
/// called under `attempt`) is audited under the shared-read schedule
/// (echo#1016), and the verifier's minimums under both schedules equal the
/// lowerer's, which `family_four_minimums_under_each_schedule` in the
/// lowerer pins independently (oracle: the other judge).
#[test]
fn family_four_minimums_equal_the_lowerer_minimums() {
    let mut measured = Vec::new();
    for name in [
        "ordered-record-z-first",
        "ordered-record-a-first",
        "ordered-record-guarded-z-first",
        "ordered-record-guarded-a-first",
        "option-value-witness",
    ] {
        assert_eq!(
            verify_effect_package(&edited(name, Some(SHARED), None, None)),
            Ok(()),
            "{name}: shared"
        );
        measured.push((name, minimum(name, None), minimum(name, Some(SHARED))));
    }
    assert_eq!(
        measured,
        [
            ("ordered-record-z-first", (57, 2793), (111, 1425)),
            ("ordered-record-a-first", (57, 2793), (111, 1425)),
            ("ordered-record-guarded-z-first", (71, 3177), (251, 1681)),
            ("ordered-record-guarded-a-first", (71, 3177), (251, 1681)),
            ("option-value-witness", (88, 2936), (831, 942)),
        ]
    );
}
