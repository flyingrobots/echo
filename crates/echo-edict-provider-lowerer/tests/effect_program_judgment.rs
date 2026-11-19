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
        let expected = if accept {
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

/// Publication runs the same judgment, so a refused program is never
/// serialized into a package.
#[test]
fn publication_refuses_what_the_judgment_refuses() {
    let mut disagreements = Vec::new();
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
        let source = echo_edict_canonical::encode_canonical_cbor_v1(&V::Bytes(
            b"package app;
"
            .to_vec(),
        ))
        .expect("source");
        let published = encode_effect_publication(EffectPublicationInput {
            compiler_program: &program,
            application_schema_coordinate: "app.Exports",
            source_artifact: &source,
            source_domain: "edict.source/v1",
            configuration: &configuration,
        });
        match (accept, &published) {
            (true, Ok(_)) | (false, Err(EffectPublicationError::UnsupportedProgram)) => {}
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

fn entry<'a>(value: &'a mut V, key: &str) -> &'a mut V {
    let V::Map(fields) = value else { panic!("map") };
    &mut fields
        .iter_mut()
        .find(|(k, _)| k == &V::Text(key.into()))
        .unwrap_or_else(|| panic!("field {key}"))
        .1
}

/// The admitted `read-control` program with one more required capability,
/// its Core re-encoded and the Target IR's Core pin recomputed.
fn with_capability(capability: &str) -> Vec<u8> {
    use echo_edict_canonical::{
        digest_canonical_value_bytes_v1 as digest, encode_canonical_cbor_v1 as encode,
    };
    let mut bundle = decode(&compiled_program("read-control")).expect("bundle");
    let mut module = decode(bytes(get(&bundle, "core_artifact"))).expect("core");
    let V::Array(capabilities) = entry(&mut module, "requiredCoreCapabilities") else {
        panic!("capabilities")
    };
    capabilities.push(V::Text(capability.into()));
    let pin = digest("edict.core.module/v1", &module).expect("core digest");
    let mut target = decode(bytes(get(&bundle, "target_ir_artifact"))).expect("target");
    *entry(
        entry(entry(&mut target, "semanticClosure"), "sourceCore"),
        "digest",
    ) = V::Array(vec![V::Text("sha256".into()), V::Bytes(pin.to_vec())]);
    *entry(&mut bundle, "core_artifact") = V::Bytes(encode(&module).expect("core bytes"));
    *entry(&mut bundle, "target_ir_artifact") = V::Bytes(encode(&target).expect("target bytes"));
    encode(&bundle).expect("bundle bytes")
}

/// The effect route has no shared-read meter, so it refuses a module that
/// requires `core.cost.shared-reads/v1`; the runtime's effect route does too.
#[test]
fn effect_programs_refuse_the_shared_read_schedule() {
    // Control: an unused capability with the same re-pinning is still admitted.
    assert_eq!(
        judge_effect_program(&with_capability("core.variant/v1")),
        Ok(())
    );
    assert_eq!(
        judge_effect_program(&with_capability("core.cost.shared-reads/v1")),
        Err(EffectPublicationError::UnsupportedProgram)
    );
}
