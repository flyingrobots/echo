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
fn package(name: &str) -> V {
    let hex = RETAINED
        .iter()
        .find_map(|(entry, hex)| (*entry == name).then_some(*hex))
        .expect(name);
    decode(&hex::decode(hex.trim()).expect("hex")).expect("canonical package")
}
fn compiled_program(name: &str) -> Vec<u8> {
    let program = decode(bytes(get(&package(name), "program"))).expect("program");
    bytes(get(&program, "compiled_program")).to_vec()
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
        let expected = if accept || name == LAW_REFUSED {
            Ok(())
        } else {
            Err(EffectPublicationError::UnsupportedProgram)
        };
        if judged != expected {
            disagreements.push(format!("{name}: expected {expected:?}, got {judged:?}"));
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

/// Publication runs the same judgment, so a refused program is never
/// serialized into a package.
#[test]
fn publication_refuses_what_the_judgment_refuses() {
    let mut disagreements = Vec::new();
    for (name, accept) in verdicts() {
        let published = publish(&name, None);
        match (accept, &published) {
            (true, Ok(_)) | (false, Err(EffectPublicationError::UnsupportedProgram)) => {}
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
