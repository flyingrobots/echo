// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The authored effect-program verdict corpus Echo's provider lowerer and
//! verifier judge independently of this runtime.
//!
//! Each entry is a whole operation package and the verdict all three
//! judgments must reach. The runtime's own verdict is asserted here; the
//! lowerer and verifier read the same retained bytes in their own tests.
//! Regenerate with the ignored `regenerate_the_retained_provider_corpus` test.
use super::abi_conformance::rename_read_failure;
use super::mutations::{fixture, fixture_with, Shape};
use super::proven_ranges::{
    at_most, declared, input, len, local, ok_binding, package, prefix_is_utf8, slice_program, Node,
};
use super::*;

const GRAPH_EFFECTS: &str = "continuum.profile.graph-effects/v1";
const READ: &str = "jedit.text@1.readFact";

/// What the runtime says when it prepares the package. `None` means the
/// refusal is a static judgment the runtime makes only when it executes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Runtime {
    Accepts,
    Refuses,
    Executes,
}

struct Entry {
    name: &'static str,
    accept: bool,
    runtime: Runtime,
    package: V,
}

fn decode(raw: &V) -> V {
    let V::Bytes(raw) = raw else { panic!("bytes") };
    echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("canonical")
}

/// Rebinds a package after editing its compiled Core, Target IR and exports,
/// keeping its lowering artifacts, so a forgery reaches the check it targets.
fn rebuild(package: &V, edit: impl FnOnce(&mut V, &mut V, &mut V)) -> V {
    let program = decode(get(package, "program"));
    let bundle = decode(get(&program, "compiled_program"));
    let mut core = decode(get(&bundle, "core_artifact"));
    let mut target = decode(get(&bundle, "target_ir_artifact"));
    let mut exports = decode(get(&bundle, "lawpack_exports_artifact"));
    edit(&mut core, &mut target, &mut exports);
    set(
        value_field_mut(
            value_field_mut(&mut target, "semanticClosure"),
            "sourceCore",
        ),
        "digest",
        V::Array(vec![
            text("sha256"),
            artifact_identity("edict.core.module/v1", &core),
        ]),
    );
    let V::Text(intent) = get(&bundle, "intent") else {
        panic!("intent")
    };
    let mut rebound = bound_compiler_package(&core, &target, &exports, intent);
    let mut program = decode(get(&rebound, "program"));
    let mut fresh = compiled_bundle(&core, &target, &exports, intent);
    set(
        &mut fresh,
        "lowering_artifacts",
        get(&bundle, "lowering_artifacts").clone(),
    );
    set(&mut program, "compiled_program", V::Bytes(encoded(&fresh)));
    set(&mut rebound, "program", V::Bytes(encoded(&program)));
    rebound
}

fn first_intent_mut(core: &mut V) -> &mut V {
    let V::Map(intents) = value_field_mut(core, "intents") else {
        panic!("intents")
    };
    &mut intents[0].1
}

/// Sets only the declared step budget, in Core and its lowered intent alike.
fn with_budget(package: &V, steps: u64) -> V {
    rebuild(package, |core, target, _| {
        let steps = V::Integer(i128::from(steps));
        set(
            value_field_mut(first_intent_mut(core), "coreEvaluationBudget"),
            "maxSteps",
            steps.clone(),
        );
        set(
            value_field_mut(selected_target_mut(target), "coreEvaluationBudget"),
            "maxSteps",
            steps,
        );
    })
}

const AMPLE: u64 = 1_000_000;
/// Covers the worst case of reading a 1 MiB fact and scanning it. The
/// captured read program declares 1 MiB of steps, which Echo's providers
/// find too small statically; the runtime meters it during execution.
const READ_AMPLE: u64 = 4_194_304;

fn proven_slice() -> V {
    with_budget(
        &slice_program(vec![at_most(input("at"), len(input("text")))], "text"),
        AMPLE,
    )
}

fn read_with_adapter(edit: impl FnOnce(&mut V), catalog: V) -> V {
    let mut adapter = catalog_named_adapter(&captured_adapter());
    edit(value_field_mut(
        value_field_mut(value_field_mut(&mut adapter, "effectImplementations"), READ),
        "failureMappings",
    ));
    with_budget(
        &publication_for_lowering(&lowering_fixture_inputs(Some(catalog), None, Some(adapter))),
        READ_AMPLE,
    )
}

fn entries() -> Vec<Entry> {
    let read_capture = publication_for_lowering(&repinned_lowering_artifacts());
    let read_control = with_budget(&read_capture, READ_AMPLE);
    let mut wrong_input = Shape::conforming("ensure-node");
    wrong_input.fields[1] = ("nodeId", "Bytes<max=32>");
    let mut tag_spelled = catalog_fixture();
    for (tag, name) in READ_FAILURE_NAMES {
        rename_read_failure(&mut tag_spelled, name, tag);
    }
    let mut unknown = catalog_fixture();
    rename_read_failure(&mut unknown, "atomMissing", "atomAbsent");
    let entry = |name, accept, runtime, package| Entry {
        name,
        accept,
        runtime,
        package,
    };
    vec![
        entry("read-control", true, Runtime::Accepts, read_control.clone()),
        entry(
            "ensure-node-control",
            true,
            Runtime::Accepts,
            fixture("ensure-node", GRAPH_EFFECTS).0,
        ),
        entry("slice-proven", true, Runtime::Accepts, proven_slice()),
        entry(
            "slice-unproven",
            false,
            Runtime::Refuses,
            with_budget(&slice_program(Vec::new(), "text"), AMPLE),
        ),
        entry(
            "slice-other-operand",
            false,
            Runtime::Refuses,
            with_budget(
                &slice_program(vec![at_most(input("at"), len(input("other")))], "text"),
                AMPLE,
            ),
        ),
        entry(
            "slice-require-only",
            false,
            Runtime::Refuses,
            with_budget(
                &package(
                    Vec::new(),
                    vec![
                        Node::Require(at_most(input("at"), len(input("text")))),
                        Node::Let(ok_binding(), prefix_is_utf8("text", input("at"))),
                    ],
                    local(&ok_binding()),
                ),
                AMPLE,
            ),
        ),
        entry(
            "forged-node-order",
            false,
            Runtime::Refuses,
            rebuild(
                &with_budget(
                    &package(
                        vec![at_most(input("at"), len(input("text")))],
                        vec![
                            Node::Let(declared("local.size", "U64"), len(input("text"))),
                            Node::Let(ok_binding(), prefix_is_utf8("text", input("at"))),
                        ],
                        local(&ok_binding()),
                    ),
                    AMPLE,
                ),
                |_, target, _| {
                    let V::Array(order) =
                        value_field_mut(selected_target_mut(target), "executionOrder")
                    else {
                        panic!("order")
                    };
                    order.swap(0, 1);
                },
            ),
        ),
        entry(
            "wrong-intrinsic-input-type",
            false,
            Runtime::Refuses,
            fixture_with("ensure-node", GRAPH_EFFECTS, &wrong_input).0,
        ),
        entry(
            "undeclared-obstruction",
            false,
            Runtime::Refuses,
            rebuild(&read_control, |_, _, exports| {
                let V::Array(obstructions) = value_field_mut(exports, "obstructions") else {
                    panic!("obstructions")
                };
                obstructions.remove(0);
            }),
        ),
        entry(
            "failure-mapping-tag-spelled",
            false,
            Runtime::Refuses,
            with_budget(
                &publication_for_lowering(&lowering_fixture_inputs(
                    Some(tag_spelled),
                    None,
                    Some(captured_adapter()),
                )),
                READ_AMPLE,
            ),
        ),
        entry(
            "failure-mapping-unknown",
            false,
            Runtime::Refuses,
            read_with_adapter(
                |mappings| {
                    let V::Map(mappings) = mappings else {
                        panic!("mappings")
                    };
                    for (_, target) in mappings.iter_mut() {
                        if target == &text("atomMissing") {
                            *target = text("atomAbsent");
                        }
                    }
                },
                unknown,
            ),
        ),
        entry(
            "failure-mapping-duplicate",
            false,
            Runtime::Refuses,
            read_with_adapter(
                |mappings| {
                    let V::Map(mappings) = mappings else {
                        panic!("mappings")
                    };
                    for (_, target) in mappings.iter_mut() {
                        if target == &text("atomRequired") {
                            *target = text("atomMissing");
                        }
                    }
                },
                catalog_fixture(),
            ),
        ),
        entry(
            "over-budget",
            false,
            Runtime::Executes,
            with_budget(&proven_slice(), 1),
        ),
        entry(
            "read-captured-budget",
            false,
            Runtime::Executes,
            read_capture,
        ),
    ]
}

/// The retained corpus bytes, embedded so the check reads no files.
macro_rules! retained {
    ($($name:literal),* $(,)?) => {
        &[$((
            $name,
            include_str!(concat!(
                "../../../echo-edict-provider-lowerer/tests/fixtures/effect-programs/",
                $name,
                ".hex"
            )),
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
const VERDICTS: &str = include_str!(
    "../../../echo-edict-provider-lowerer/tests/fixtures/effect-programs/verdicts.txt"
);

fn manifest(entries: &[Entry]) -> String {
    let mut manifest = String::new();
    for entry in entries {
        manifest.push_str(entry.name);
        manifest.push_str(if entry.accept {
            " accept\n"
        } else {
            " refuse\n"
        });
    }
    manifest
}

#[test]
fn the_retained_provider_corpus_matches_the_runtime_verdicts() {
    let entries = entries();
    assert_eq!(
        entries.iter().map(|entry| entry.name).collect::<Vec<_>>(),
        RETAINED.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "every corpus entry is retained, in order"
    );
    assert_eq!(VERDICTS, manifest(&entries), "verdict manifest drifted");
    for (entry, (_, retained)) in entries.iter().zip(RETAINED) {
        let prepared = prepare(&entry.package);
        match entry.runtime {
            Runtime::Accepts => {
                prepared.unwrap_or_else(|e| panic!("{}: runtime refused: {e:?}", entry.name));
            }
            Runtime::Refuses => assert!(prepared.is_err(), "{}: runtime accepted", entry.name),
            // The static budget is a provider judgment; the runtime meters it
            // during execution instead.
            Runtime::Executes => {
                prepared.unwrap_or_else(|e| panic!("{}: runtime refused: {e:?}", entry.name));
            }
        }
        assert_eq!(
            retained.trim(),
            hex::encode(encoded(&entry.package)),
            "{}: retained bytes drifted; run the ignored regeneration test",
            entry.name
        );
    }
}

/// Rewrites the retained corpus. Run explicitly after changing an entry:
/// `cargo test ... regenerate_the_retained_provider_corpus -- --ignored`.
#[test]
#[ignore = "explicit fixture regeneration only"]
fn regenerate_the_retained_provider_corpus() {
    let entries = entries();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../echo-edict-provider-lowerer/tests/fixtures/effect-programs");
    std::fs::create_dir_all(&dir).expect("corpus directory");
    for entry in &entries {
        std::fs::write(
            dir.join(format!("{}.hex", entry.name)),
            format!("{}\n", hex::encode(encoded(&entry.package))),
        )
        .expect("corpus entry");
    }
    std::fs::write(dir.join("verdicts.txt"), manifest(&entries)).expect("manifest");
}
