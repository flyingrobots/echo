// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The lowerer judges authored effect programs itself, before publication,
//! and reaches the runtime's verdict on the shared retained corpus.
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as V};
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
    for (name, accept) in verdicts {
        let judged = judge_effect_program(&compiled_program(&name));
        if accept {
            judged.unwrap_or_else(|e| panic!("{name}: lowerer refused an accepted program: {e}"));
        } else {
            assert_eq!(
                judged,
                Err(EffectPublicationError::UnsupportedProgram),
                "{name}: lowerer accepted a refused program"
            );
        }
    }
}

/// Publication runs the same judgment, so a refused program is never
/// serialized into a package.
#[test]
fn publication_refuses_what_the_judgment_refuses() {
    for (name, accept) in verdicts() {
        let program = compiled_program(&name);
        let package = package(&name);
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
                ("footprintLaw", field("footprint_law")),
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
        let source =
            echo_edict_canonical::encode_canonical_cbor_v1(&V::Bytes(b"package app;\n".to_vec()))
                .expect("source");
        let published = encode_effect_publication(EffectPublicationInput {
            compiler_program: &program,
            application_schema_coordinate: "app.Exports",
            source_artifact: &source,
            source_domain: "edict.source/v1",
            configuration: &configuration,
        });
        if accept {
            assert_ne!(
                published,
                Err(EffectPublicationError::UnsupportedProgram),
                "{name}: publication refused an accepted program"
            );
        } else {
            assert_eq!(
                published,
                Err(EffectPublicationError::UnsupportedProgram),
                "{name}: publication serialized a refused program"
            );
        }
    }
}
