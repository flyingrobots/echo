// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The lowerer judges authored effect programs itself, before publication,
//! and reaches the runtime's verdict on the shared retained corpus.
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as V};
use echo_edict_provider_lowerer::effect_contract::ContractRefusal;
use echo_edict_provider_lowerer::effect_publication::{
    encode_effect_publication, judge_effect_program, EffectPublicationError, EffectPublicationInput,
};

/// The retained corpus, embedded so the judgment reads no files.
macro_rules! retained {
    ($($name:literal),* $(,)?) => {
        &[$((
            $name,
            include_str!(concat!("fixtures/effect-programs/", $name, ".hex")),
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
    "source-module-control",
    "source-module-substituted",
    "source-module-unpinned",
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
    "footprint-writes-at-ceiling",
    "footprint-writes-over-ceiling",
];
/// The entry refused by its footprint law, not by its program (echo#1023).
const LAW_REFUSED: &str = "footprint-writes-over-ceiling";
const VERDICTS: &str = include_str!("fixtures/effect-programs/verdicts.txt");
/// The check each refused entry targets, from the corpus README's account of
/// the one edit that distinguishes it from its accepted sibling.
const REASONS: &str = include_str!("fixtures/effect-programs/reasons.txt");

fn get<'a>(value: &'a V, key: &str) -> &'a V {
    let V::Map(fields) = value else { panic!("map") };
    &fields
        .iter()
        .find(|(k, _)| k == &V::Text(key.into()))
        .unwrap_or_else(|| panic!("field {key}"))
        .1
}
fn bytes(value: &V) -> &[u8] {
    let V::Bytes(bytes) = value else {
        panic!("bytes")
    };
    bytes
}
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
    ),
    (
        "option-value-witness",
        include_str!(
            "../../warp-core/tests/fixtures/edict-option-value/option-executable-operation-package.hex"
        ),
    ),
    (
        "node-atom-read-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/read-executable-operation-package.hex"
        ),
    ),
    (
        "outgoing-edge-read-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/link-executable-operation-package.hex"
        ),
    ),
    (
        "mutations-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/write-executable-operation-package.hex"
        ),
    ),
    (
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
    (
        "payload-attempt-two-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-attempt-two-executable-operation-package.hex"
        ),
    ),
    (
        "payload-require-empty-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-require-empty-executable-operation-package.hex"
        ),
    ),    (
        "payload-attempt-record-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-attempt-record-executable-operation-package.hex"
        ),
    ),
    (
        "payload-require-record-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-require-record-executable-operation-package.hex"
        ),
    ),
    (
        "payload-loop-scalar-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-loop-scalar-executable-operation-package.hex"
        ),
    ),
    (
        "payload-loop-record-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-loop-record-executable-operation-package.hex"
        ),
    ),
    (
        "payload-loop-nested-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-loop-nested-executable-operation-package.hex"
        ),
    ),
    (
        "payload-loop-entry-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-loop-entry-executable-operation-package.hex"
        ),
    ),
    (
        "payload-loop-bytes-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-loop-bytes-executable-operation-package.hex"
        ),
    ),
    (
        "payload-loop-mixed-v2-admission",
        include_str!(
            "../../warp-core/tests/fixtures/edict-effect-v2-admission/payload-loop-mixed-executable-operation-package.hex"
        ),
    ),
];
fn package(name: &str) -> V {
    let hex = RETAINED
        .iter()
        .chain(WITNESSES)
        .find_map(|(entry, hex)| (*entry == name).then_some(*hex))
        .expect(name);
    decode(&hex::decode(hex.trim()).expect("hex")).expect("canonical package")
}
fn compiled_program(name: &str) -> Vec<u8> {
    let program = decode(bytes(get(&package(name), "program"))).expect("program");
    bytes(get(&program, "compiled_program")).to_vec()
}
/// The reason code each refused entry must name.
fn reasons() -> Vec<(String, String)> {
    REASONS
        .lines()
        .map(|line| {
            let (name, code) = line.split_once(' ').expect("entry");
            (name.to_owned(), code.to_owned())
        })
        .collect()
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
fn the_lowerer_reaches_every_retained_corpus_verdict() {
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
        let judged = judge_effect_program(&compiled_program(&name));
        // The law entry's program is sound; publication refuses its law.
        let expected = accept || name == LAW_REFUSED;
        if judged.is_ok() != expected {
            disagreements.push(format!(
                "{name}: expected accept={expected}, got {judged:?}"
            ));
        }
    }
    assert_eq!(disagreements, Vec::<String>::new());
}

/// Publishes entry `name` under the configuration its own package names,
/// with `law` in place of its footprint law when given.
fn publish(name: &str, law: Option<Vec<u8>>) -> Result<Vec<u8>, EffectPublicationError> {
    let program = compiled_program(name);
    let package = package(name);
    let program_value = decode(bytes(get(&package, "program"))).expect("program");
    let catalog = {
        let bundle = decode(&program).expect("bundle");
        match get(&bundle, "lowering_artifacts") {
            V::Null => echo_edict_canonical::encode_canonical_cbor_v1(&V::Map(vec![
                (
                    V::Text("apiVersion".into()),
                    V::Text("edict.target-profile.intrinsics/v1".into()),
                ),
                (V::Text("intrinsics".into()), V::Map(vec![])),
            ]))
            .expect("empty catalog"),
            lowering => bytes(get(lowering, "intrinsics_artifact")).to_vec(),
        }
    };
    let field = |key: &str| get(&program_value, key).clone();
    let configuration = echo_edict_canonical::encode_canonical_cbor_v1(&V::Map(
        [
            (
                "apiVersion",
                V::Text("echo.operation-lowering-configuration/v1".into()),
            ),
            ("programKind", V::Text("edict-effect-program/v1".into())),
            ("identityLaw", V::Integer(1)),
            ("intrinsicCatalog", V::Bytes(catalog)),
            ("graphSchema", field("graph_schema")),
            (
                "footprintLaw",
                law.map_or_else(|| field("footprint_law"), V::Bytes),
            ),
            ("maxInputBytes", field("max_input_bytes")),
            ("maxResultBytes", field("max_result_bytes")),
            ("maxAllocatedBytes", field("max_allocated_bytes")),
            (
                "budgetCeiling",
                V::Map(vec![
                    (V::Text("steps".into()), V::Integer(1)),
                    (V::Text("readBytes".into()), V::Integer(0)),
                    (V::Text("writeBytes".into()), V::Integer(0)),
                ]),
            ),
            ("authorityProfile", V::Text("app.effect-writer@1".into())),
        ]
        .into_iter()
        .map(|(k, v)| (V::Text(k.into()), v))
        .collect(),
    ))
    .expect("configuration");
    let source = echo_edict_canonical::encode_canonical_cbor_v1(&V::Bytes(
        b"package app;
"
        .to_vec(),
    ))
    .expect("source");
    encode_effect_publication(EffectPublicationInput {
        compiler_program: &program,
        application_schema_coordinate: "app.Exports",
        source_artifact: &source,
        source_domain: "edict.source/v1",
        configuration: &configuration,
    })
}

/// Every refused entry names the check its one edit targets.
#[test]
fn the_lowerer_names_the_check_each_refused_entry_targets() {
    let reasons = reasons();
    let refused = verdicts()
        .into_iter()
        .filter_map(|(name, accept)| (!accept).then_some(name))
        .collect::<Vec<_>>();
    assert_eq!(
        reasons.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        refused.iter().collect::<Vec<_>>(),
        "every refused entry has exactly one reason, in corpus order"
    );
    let mut disagreements = Vec::new();
    for (name, code) in &reasons {
        match judge_effect_program(&compiled_program(name)) {
            Err(EffectPublicationError::UnsupportedProgram(refusal))
                if refusal.reason().code() == code => {}
            // The law entry's program is sound; publication names its law.
            Ok(()) if name == LAW_REFUSED => match publish(name, None) {
                Err(EffectPublicationError::Contract(refusal)) if refusal.code() == code => {}
                other => disagreements.push(format!("{name}: expected {code}, got {other:?}")),
            },
            other => disagreements.push(format!("{name}: expected {code}, got {other:?}")),
        }
    }
    // Zero witnesses would pass vacuously.
    assert!(reasons.len() >= 10, "{} refused entries", reasons.len());
    assert_eq!(disagreements, Vec::<String>::new());
}

/// Publication runs the same judgment, so a refused program is never
/// serialized into a package.
#[test]
fn publication_refuses_what_the_judgment_refuses() {
    let mut disagreements = Vec::new();
    for (name, accept) in verdicts() {
        let published = publish(&name, None);
        match (accept, &published) {
            (true, Ok(_)) | (false, Err(EffectPublicationError::UnsupportedProgram(_))) => {}
            (false, Err(EffectPublicationError::Contract(ContractRefusal::FootprintMaxWrites)))
                if name == LAW_REFUSED => {}
            (true, Err(error)) => {
                disagreements.push(format!("{name}: publication refused: {error:?}"));
            }
            (false, other) => disagreements.push(format!(
                "{name}: publication did not refuse the program: {:?}",
                other.as_ref().map(Vec::len)
            )),
        }
    }
    assert_eq!(disagreements, Vec::<String>::new());
}

/// The footprint law with `max_writes` set to `writes`, from entry `name`.
fn law_with_writes(name: &str, writes: i128) -> Vec<u8> {
    let program = decode(bytes(get(&package(name), "program"))).expect("program");
    let V::Map(mut law) = decode(bytes(get(&program, "footprint_law"))).expect("law") else {
        panic!("law map")
    };
    law.iter_mut()
        .find(|(key, _)| key == &V::Text("max_writes".into()))
        .expect("max_writes")
        .1 = V::Integer(writes);
    echo_edict_canonical::encode_canonical_cbor_v1(&V::Map(law)).expect("law")
}

/// Runtime admission refuses a law above the effect profile's 256 writes, so
/// publication refuses it and names the check; at the ceiling it publishes
/// (echo#1023).
#[test]
fn publication_refuses_a_law_above_the_write_ceiling() {
    assert!(publish("read-control", Some(law_with_writes("read-control", 256))).is_ok());
    let refused = publish("read-control", Some(law_with_writes("read-control", 257)));
    assert_eq!(
        refused,
        Err(EffectPublicationError::Contract(
            ContractRefusal::FootprintMaxWrites
        ))
    );
    assert_eq!(
        refused.expect_err("refused").to_string(),
        "effect-program.footprint-law.max-writes"
    );
}

/// The adapter artifact embedded in `compiled`, with its class replaced.
fn with_adapter_class(compiled: &[u8], class: &str) -> Vec<u8> {
    let mut bundle = decode(compiled).expect("bundle");
    let V::Map(fields) = &mut bundle else {
        panic!("bundle")
    };
    let lowering = &mut fields
        .iter_mut()
        .find(|(k, _)| k == &V::Text("lowering_artifacts".into()))
        .expect("lowering artifacts")
        .1;
    let V::Map(lowering) = lowering else {
        panic!("lowering")
    };
    let slot = &mut lowering
        .iter_mut()
        .find(|(k, _)| k == &V::Text("adapter_artifact".into()))
        .expect("adapter")
        .1;
    let V::Bytes(bytes) = slot else {
        panic!("adapter bytes")
    };
    let mut adapter = decode(bytes).expect("adapter");
    let V::Map(entries) = &mut adapter else {
        panic!("adapter map")
    };
    entries
        .iter_mut()
        .find(|(k, _)| k == &V::Text("class".into()))
        .expect("class")
        .1 = V::Text(class.into());
    *slot = V::Bytes(echo_edict_canonical::encode_canonical_cbor_v1(&adapter).expect("adapter"));
    echo_edict_canonical::encode_canonical_cbor_v1(&bundle).expect("bundle")
}

/// An adapter outside the declarative class is refused as the adapter, at
/// the adapter artifact, not as the lawpack checked just before it.
#[test]
fn an_undeclared_adapter_class_is_named_as_the_adapter() {
    let compiled = with_adapter_class(&compiled_program("read-control"), "procedural");
    match judge_effect_program(&compiled) {
        Err(EffectPublicationError::UnsupportedProgram(refusal)) => assert_eq!(
            (refusal.reason().code(), refusal.path()),
            (
                "effect-program.relation.adapter",
                "/lowering_artifacts/adapter_artifact"
            )
        ),
        other => panic!("expected the adapter check, got {other:?}"),
    }
}

fn entry<'a>(value: &'a mut V, key: &str) -> &'a mut V {
    let V::Map(fields) = value else { panic!("map") };
    &mut fields
        .iter_mut()
        .find(|(k, _)| k == &V::Text(key.into()))
        .unwrap_or_else(|| panic!("field {key}"))
        .1
}

/// A retained program edited and repinned: its Core requires `capability`
/// as well, when given, and its intent's Core budget (in Core and in the
/// Target IR alike) is set on the axes given.
fn edited(
    name: &str,
    capability: Option<&str>,
    steps: Option<u64>,
    bytes_: Option<u64>,
) -> Vec<u8> {
    use echo_edict_canonical::{
        digest_canonical_value_bytes_v1 as digest, encode_canonical_cbor_v1 as encode,
    };
    let mut bundle = decode(&compiled_program(name)).expect("bundle");
    let V::Text(intent) = get(&bundle, "intent").clone() else {
        panic!("intent")
    };
    let mut module = decode(bytes(get(&bundle, "core_artifact"))).expect("core");
    if let Some(capability) = capability {
        let V::Array(capabilities) = entry(&mut module, "requiredCoreCapabilities") else {
            panic!("capabilities")
        };
        capabilities.push(V::Text(capability.into()));
    }
    let mut target = decode(bytes(get(&bundle, "target_ir_artifact"))).expect("target");
    for (axis, value) in [("maxSteps", steps), ("maxAllocatedBytes", bytes_)] {
        let Some(value) = value else { continue };
        let value = V::Integer(i128::from(value));
        *entry(
            entry(
                entry(entry(&mut module, "intents"), &intent),
                "coreEvaluationBudget",
            ),
            axis,
        ) = value.clone();
        let V::Map(intents) = entry(&mut target, "intents") else {
            panic!("target intents")
        };
        *entry(entry(&mut intents[0].1, "coreEvaluationBudget"), axis) = value;
    }
    let pin = digest("edict.core.module/v1", &module).expect("core digest");
    *entry(
        entry(entry(&mut target, "semanticClosure"), "sourceCore"),
        "digest",
    ) = V::Array(vec![V::Text("sha256".into()), V::Bytes(pin.to_vec())]);
    *entry(&mut bundle, "core_artifact") = V::Bytes(encode(&module).expect("core bytes"));
    *entry(&mut bundle, "target_ir_artifact") = V::Bytes(encode(&target).expect("target bytes"));
    encode(&bundle).expect("bundle bytes")
}

const SHARED: &str = "core.cost.shared-reads/v1";

/// The smallest step and allocation budgets the judgment admits for a
/// program, each searched with the other axis ample.
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
        judge_effect_program(&edited(name, capability, Some(steps), Some(AMPLE))).is_ok()
    });
    let bytes_ = search(&|bytes_| {
        judge_effect_program(&edited(name, capability, Some(AMPLE), Some(bytes_))).is_ok()
    });
    (steps, bytes_)
}

/// Programs whose expressions all have a shared-read rule in Echo.
const SHARED_PROGRAMS: [&str; 5] = [
    "read-control",
    "ensure-node-control",
    "structured-loop-control",
    "structured-branch-control",
    "structured-nested-control",
];

/// An effect program that requires the shared-read schedule is judged under
/// it: admitted when its expressions have Echo rules (echo#1016).
#[test]
fn effect_programs_are_judged_under_the_shared_read_schedule() {
    for name in SHARED_PROGRAMS {
        assert_eq!(
            judge_effect_program(&edited(name, Some(SHARED), None, None)),
            Ok(()),
            "{name}"
        );
    }
}

/// Effect programs using byte operations under the shared-read schedule
/// (echo#1016, family 5): `slice-proven` slices and measures its input and
/// `typed-failure-control` measures one under `attempt`. Each byte call keeps
/// its copying charge and is charged the value it builds (amblicit#340).
/// Change-detection goldens that the verifier must equal and the warp-core
/// test `byte_effects_run_within_the_lowerer_minimum` runs at exactly.
#[test]
fn byte_operation_minimums_under_each_schedule() {
    let measured: Vec<_> = ["slice-proven", "typed-failure-control"]
        .into_iter()
        .map(|name| (name, minimum(name, None), minimum(name, Some(SHARED))))
        .collect();
    // (name, copying (steps, bytes), shared (steps, bytes))
    assert_eq!(
        measured,
        [
            ("slice-proven", (195, 3383), (873, 971)),
            ("typed-failure-control", (74, 3312), (374, 1231)),
        ]
    );
}

/// The judgment's own minimum budgets under each schedule (change-detection
/// goldens, measured by this judgment). The runtime test
/// `effect_runs_never_exceed_the_lowerer_minimum` in warp-core asserts its
/// worst-case charge stays within the shared minimums recorded here.
#[test]
fn effect_program_minimums_under_each_schedule() {
    let measured: Vec<_> = SHARED_PROGRAMS
        .iter()
        .map(|name| (*name, minimum(name, None), minimum(name, Some(SHARED))))
        .collect();
    // (name, copying (steps, bytes), shared (steps, bytes))
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

/// Where a public witness's publication context comes from: its own
/// operation configuration, as its lawpack build wrote it, and its exports.
struct Context {
    configuration: &'static str,
    schema: &'static str,
    package: &'static [u8],
}

fn context(name: &str) -> Context {
    match name {
        "closure-read-witness" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-effect-closure-read/closure-configuration.hex"
            ),
            schema: "consumer.closures.exports/v1",
            package: b"package consumer.closures.program@1;\n",
        },
        "identity-witness" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-content-address/identity-configuration.hex"
            ),
            schema: "consumer.identity.exports/v1",
            package: b"package consumer.identity.program@1;\n",
        },
        "attachment-digest-witness" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-attachment-digest/bump-configuration.hex"
            ),
            schema: "consumer.buffers.exports/v1",
            package: b"package consumer.buffers.program@1;\n",
        },
        "option-value-witness" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-option-value/option-configuration.hex"
            ),
            schema: "consumer.effect.exports/v1",
            package: b"package consumer.effect.program@1;\n",
        },
        "node-atom-read-v2-admission" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-effect-step/read-configuration.hex"
            ),
            schema: "consumer.effects.exports/v1",
            package: b"package consumer.effects.program@1;\n",
        },
        "outgoing-edge-read-v2-admission" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-effect-graph-read/link-configuration.hex"
            ),
            schema: "consumer.links.exports/v1",
            package: b"package consumer.links.program@1;\n",
        },
        "mutations-v2-admission" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-effect-mutations/write-configuration.hex"
            ),
            schema: "consumer.writes.exports/v1",
            package: b"package consumer.writes.program@1;\n",
        },
        "typed-failure-v2-admission"
        | "payload-attempt-v2-admission"
        | "payload-require-v2-admission"
        | "payload-attempt-two-v2-admission"
        | "payload-require-empty-v2-admission"
        | "payload-attempt-record-v2-admission"
        | "payload-require-record-v2-admission"
        | "payload-loop-scalar-v2-admission"
        | "payload-loop-record-v2-admission"
        | "payload-loop-nested-v2-admission"
        | "payload-loop-entry-v2-admission"
        | "payload-loop-bytes-v2-admission"
        | "payload-loop-mixed-v2-admission" => Context {
            configuration: include_str!(
                "../../warp-core/tests/fixtures/edict-effect-typed-failure/typed-configuration.hex"
            ),
            schema: "consumer.typedfail.exports/v1",
            package: b"package consumer.typedfail.program@1;\n",
        },
        _ => panic!("no witness {name}"),
    }
}

/// Judges an edited witness through publication, which reads the witness's
/// configuration (its catalog and, for a closure, its selector).
fn judge_witness(name: &str, program: &[u8]) -> Result<(), EffectPublicationError> {
    let context = context(name);
    let configuration = hex::decode(context.configuration.trim()).expect("hex");
    let source =
        echo_edict_canonical::encode_canonical_cbor_v1(&V::Bytes(context.package.to_vec()))
            .expect("source");
    encode_effect_publication(EffectPublicationInput {
        compiler_program: program,
        application_schema_coordinate: context.schema,
        source_artifact: &source,
        source_domain: "edict.source/v1",
        configuration: &configuration,
    })
    .map(|_| ())
}

/// The smallest (steps, bytes) the witness's publication admits.
fn witness_minimum(name: &str, capability: Option<&str>) -> (u64, u64) {
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
    (
        search(&|steps| {
            judge_witness(name, &edited(name, capability, Some(steps), Some(AMPLE))).is_ok()
        }),
        search(&|bytes_| {
            judge_witness(name, &edited(name, capability, Some(AMPLE), Some(bytes_))).is_ok()
        }),
    )
}

/// The compiler-produced closure-read witness under each schedule: the
/// copying schedule charges the module's costliest helper as the selector,
/// the shared schedule the declared selector only, at its shared-read call
/// cost (echo#1016). Change-detection
/// goldens; `the_closure_witness_runs_within_the_lowerer_minimum` in warp-core
/// runs the witness at exactly the shared minimum recorded here.
#[test]
fn closure_read_witness_minimums_under_each_schedule() {
    let name = "closure-read-witness";
    assert_eq!(
        judge_witness(name, &compiled_program(name)),
        Ok(()),
        "as compiled"
    );
    assert_eq!(
        judge_witness(name, &edited(name, Some(SHARED), None, None)),
        Ok(()),
        "shared"
    );
    let copying = witness_minimum(name, None);
    let shared = witness_minimum(name, Some(SHARED));
    assert_eq!((copying, shared), ((15_367, 932_374), (10_814, 194_487)));
    // The witness also carries `costly`, a costlier helper it never selects:
    // copying charges it on every closure edge, the shared schedule charges
    // the declared `select` only, so it is cheaper on both axes.
    assert!(shared.0 < copying.0 && shared.1 < copying.1);
}

/// The public compiler's own shared-read minimum for the closure-read
/// witness, (steps, bytes): Amblicit 65dc5362 with Echo's `/v3` effect
/// templates, as `xtask --example effect_v2_admission` reports it. The
/// witness package is byte-identical to that compiler's checked output.
const COMPILER_SHARED_MINIMUM: (u64, u64) = (10_814, 194_487);
/// What Echo's lowerer charges above that compiler: nothing. Amblicit
/// 9f10fb91 omitted the effect node's step, (1, 0); 65dc5362 charges it
/// (echo#1016 term 3), so the compiler's minimum is admitted exactly.
const COMPILER_RESIDUAL: (u64, u64) = (0, 0);

/// Exact admission at the compiler's minimum (echo#1016): Echo's lowerer
/// minimum for the closure-read witness under the shared-read schedule is
/// the compiler's minimum plus only the known residual, so one unit below
/// the compiler's minimum on either axis is what Echo refuses once the
/// residual is gone. Oracle: the compiler's own report, pinned above.
#[test]
fn the_closure_witness_minimum_is_the_compiler_minimum_plus_its_residual() {
    let name = "closure-read-witness";
    let (steps, bytes_) = witness_minimum(name, Some(SHARED));
    assert_eq!(
        (
            steps - COMPILER_SHARED_MINIMUM.0,
            bytes_ - COMPILER_SHARED_MINIMUM.1
        ),
        COMPILER_RESIDUAL
    );
}

/// The public compiler's shared-read minimums for sixteen effect programs,
/// (name, (steps, bytes)), with Echo's `/v3` effect templates, as
/// `xtask --example effect_v2_admission` reports them for Amblicit
/// 2b09e4ab1ecf0041b4ff251693dc295ac2b649c2 through the checked provider
/// package, whose `echo.dpo@2` profile pins the `/v3` catalog (echo#1016).
/// Each package under `edict-effect-v2-admission/` is byte for byte that
/// compiler's output at exactly this minimum, under `edict.core-cost/v2`,
/// with Echo's packaged lowerer and verifier accepting it, and the compiler
/// refuses one unit below on either axis. The numbers are unchanged from the
/// private `/v3` evaluation route used before the catalog was packaged
/// (Amblicit e306ecc9; the first ten from 615d2e59, the first eight from
/// 80b52f95, the first three from 65dc5362); aa8c7cbf and 2b09e4ab write
/// byte-identical packages.
///
/// The ninth and tenth are the record-valued payload cross-check (echo#1016): a
/// one-field payload `{ at: { value: n } }` in place of `{ at: n }` adds
/// (6, 330) on an `attempt` arm and on a `require` (the record literal's own
/// work (4, 197) and encoding `{ value: U64 }` instead of `U64`, (2, 133)).
/// The last two are the same `require` in a loop bounded 3 (`payload-loop-*`),
/// scalar and record payload. Since amblicit#350 each iteration
/// is charged its item step, the binder's validations and the body, with no
/// binder copy or loop-body result, which the shared runtime never charges.
const COMPILER_V2_MINIMUMS: [(&str, (u64, u64)); 16] = [
    ("node-atom-read-v2-admission", (100, 1_631)),
    ("outgoing-edge-read-v2-admission", (244, 8_240)),
    ("mutations-v2-admission", (176, 6_796)),
    ("typed-failure-v2-admission", TYPED_FAILURE_SHARED_MINIMUM),
    (
        "payload-attempt-v2-admission",
        PAYLOAD_ATTEMPT_SHARED_MINIMUM,
    ),
    (
        "payload-require-v2-admission",
        PAYLOAD_REQUIRE_SHARED_MINIMUM,
    ),
    ("payload-attempt-two-v2-admission", (150, 2_493)),
    ("payload-require-empty-v2-admission", (126, 2_031)),
    ("payload-attempt-record-v2-admission", (151, 2_621)),
    ("payload-require-record-v2-admission", (137, 2_557)),
    ("payload-loop-scalar-v2-admission", (300, 3_200)),
    ("payload-loop-record-v2-admission", (318, 4_190)),
    ("payload-loop-nested-v2-admission", (783, 5_144)),
    ("payload-loop-entry-v2-admission", (336, 3_998)),
    ("payload-loop-bytes-v2-admission", (1_134, 3_608)),
    ("payload-loop-mixed-v2-admission", (339, 4_232)),
];

/// Exact admission at the compiler's minimum (echo#1016) for the node-atom
/// read, the outgoing-edge read with `ensure-edge`, the five mutation
/// intrinsics, the typed-failure `attempt` and the failure-payload programs:
/// Echo admits each program at exactly the compiler's minimum
/// and refuses it one unit below on either axis. Oracle: the compiler's own
/// reports, pinned above.
#[test]
fn effect_programs_are_admitted_at_exactly_the_compiler_minimum() {
    for (name, (steps, bytes_)) in COMPILER_V2_MINIMUMS {
        assert_eq!(
            judge_witness(name, &edited(name, Some(SHARED), Some(steps), Some(bytes_))),
            Ok(()),
            "{name}: at the compiler minimum"
        );
        for (label, below) in [
            ("steps", (steps - 1, bytes_)),
            ("bytes", (steps, bytes_ - 1)),
        ] {
            assert!(
                judge_witness(
                    name,
                    &edited(name, Some(SHARED), Some(below.0), Some(below.1))
                )
                .is_err(),
                "{name}: one {label} unit below the compiler minimum"
            );
        }
        assert_eq!(
            witness_minimum(name, Some(SHARED)),
            (steps, bytes_),
            "{name}"
        );
    }
}

/// Echo's shared-read minimum for the typed-failure program, (steps, bytes):
/// one `attempt` of a failing helper with an empty-payload arm, then one
/// `ensureNode`. The `attempt` node is charged its step, the call's shared
/// work, and the larger of what each path adds: success adds nothing (the
/// shared runtime does not validate the binding), the arm builds `{}`
/// (one expression step and its one-cell shell, (2, 64)) and encodes it
/// (`shape(record {})`, (1, 64)). That is what warp-core's shared executor
/// charges on the failure path, measured at (3, 128) after the call.
const TYPED_FAILURE_SHARED_MINIMUM: (u64, u64) = (140, 2_095);

/// The `attempt` arm is charged what the runtime charges for it (echo#1016,
/// amblicit#341): the typed-failure program's shared-read minimum. Oracle:
/// warp-core's own meter on each path, in the constant's comment.
#[test]
fn an_attempt_is_charged_what_the_shared_runtime_charges() {
    assert_eq!(
        witness_minimum("typed-failure-v2-admission", Some(SHARED)),
        TYPED_FAILURE_SHARED_MINIMUM
    );
}

/// Echo's shared-read minimums for the typed-failure program with a one-field
/// failure payload `{ at: input.start }`, (steps, bytes): first as the
/// `attempt` arm's payload, then as a `require`'s. warp-core's shared
/// executor, measured on each failing path, builds that payload as one
/// expression step, the field's own work (2, 0) and a shell of one cell plus
/// one keyed cell, (2, 64 + 64 + 2), so (5, 130); then it encodes it, one
/// cell for the record, the key and the integer, (3, 64 + 66 + 64) =
/// (3, 194). An empty payload is (2, 64) then (1, 64).
///
/// - `attempt`: the typed-failure minimum (140, 2095) less the empty arm
///   (3, 128), plus this arm (8, 324).
/// - `require`: its predicate, the field's own work and the rest of the
///   program, (125, 1903) as Amblicit 66f6ca4d charges them, plus the
///   payload's expression step and shell (3, 130) and its encoding (3, 194).
const PAYLOAD_ATTEMPT_SHARED_MINIMUM: (u64, u64) = (145, 2_291);
const PAYLOAD_REQUIRE_SHARED_MINIMUM: (u64, u64) = (131, 2_227);

/// A failure payload's fields are charged what the shared runtime charges
/// for them (echo#1016): one keyed cell each in the shell, not two.
#[test]
fn a_failure_payload_field_is_charged_what_the_shared_runtime_charges() {
    assert_eq!(
        witness_minimum("payload-attempt-v2-admission", Some(SHARED)),
        PAYLOAD_ATTEMPT_SHARED_MINIMUM
    );
    assert_eq!(
        witness_minimum("payload-require-v2-admission", Some(SHARED)),
        PAYLOAD_REQUIRE_SHARED_MINIMUM
    );
}

/// The compiler-produced identity witnesses (all four `echo.dpo@2` identity
/// intrinsics) are judged under the shared-read schedule (echo#1016, family
/// 3). Each call is charged its catalog rule plus a full copy of every
/// operand in and of its result out, as Amblicit's schedule charges a target
/// intrinsic. The copying minimums are unchanged from the copying judgment;
/// the shared minimums are change-detection goldens that the warp-core
/// identity witness test runs at exactly.
#[test]
fn identity_witness_minimums_under_each_schedule() {
    let mut measured = Vec::new();
    for name in ["identity-witness", "attachment-digest-witness"] {
        assert_eq!(
            judge_witness(name, &edited(name, Some(SHARED), None, None)),
            Ok(()),
            "{name}: shared"
        );
        measured.push((
            name,
            witness_minimum(name, None),
            witness_minimum(name, Some(SHARED)),
        ));
    }
    // (name, copying (steps, bytes), shared (steps, bytes))
    assert_eq!(
        measured,
        [
            ("identity-witness", (1898, 71_061), (1262, 11_580)),
            ("attachment-digest-witness", (371, 11_334), (289, 3209)),
        ]
    );
}

/// Family 4 under the shared-read schedule (echo#1016): `Option`
/// constructors (the public option witness builds `some` and `none` in a
/// helper), and ordered records whose fields `fail`, called under `attempt`
/// (the retained ordered-record programs). The rules are
/// Amblicit's at 26a201e7: an ordered record charges its shell as a record,
/// a `fail` one step, an `Option` constructor its copying charge plus the
/// value it builds (amblicit#340), and an `attempt` its value plus the
/// costlier failure arm's payload. The copying minimums are unchanged; the
/// shared minimums are change-detection goldens the verifier must equal and
/// the warp-core family-4 test runs at exactly.
#[test]
fn family_four_minimums_under_each_schedule() {
    let mut measured = Vec::new();
    for name in FAMILY_FOUR {
        assert_eq!(
            judge_effect_program(&edited(name, Some(SHARED), None, None)),
            Ok(()),
            "{name}: shared"
        );
        measured.push((name, minimum(name, None), minimum(name, Some(SHARED))));
    }
    let name = "option-value-witness";
    assert_eq!(
        judge_witness(name, &edited(name, Some(SHARED), None, None)),
        Ok(()),
        "{name}: shared"
    );
    measured.push((
        name,
        witness_minimum(name, None),
        witness_minimum(name, Some(SHARED)),
    ));
    // (name, copying (steps, bytes), shared (steps, bytes))
    assert_eq!(measured, FAMILY_FOUR_MINIMUMS);
}

/// The retained family-4 programs.
const FAMILY_FOUR: [&str; 4] = [
    "ordered-record-z-first",
    "ordered-record-a-first",
    "ordered-record-guarded-z-first",
    "ordered-record-guarded-a-first",
];

#[allow(clippy::type_complexity)]
const FAMILY_FOUR_MINIMUMS: [(&str, (u64, u64), (u64, u64)); 5] = [
    ("ordered-record-z-first", (57, 2793), (111, 1425)),
    ("ordered-record-a-first", (57, 2793), (111, 1425)),
    ("ordered-record-guarded-z-first", (71, 3177), (251, 1681)),
    ("ordered-record-guarded-a-first", (71, 3177), (251, 1681)),
    ("option-value-witness", (88, 2936), (831, 942)),
];
