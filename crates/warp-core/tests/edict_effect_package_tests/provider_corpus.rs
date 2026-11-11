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
use super::execution::{host, limits};
use super::mutations::{fixture, fixture_with, repeat_emission, Shape};
use super::proven_ranges::{
    all, at_most, declared, input, is_true, len, local, ok_binding, package, prefix_is_utf8,
    slice_program, Node,
};
use super::*;
use warp_core::edict_effects::EffectFrameError;
use warp_core::edict_pure::EvaluationError;
use warp_core::edict_read::ReadError;
use warp_core::{EchoOperationApplicationBasisV1, EchoOperationBudgetV1};

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

/// Sets only the declared allocation budget, in Core and its lowered intent.
fn with_allocation(package: &V, bytes: u64) -> V {
    rebuild(package, |core, target, _| {
        let bytes = V::Integer(i128::from(bytes));
        set(
            value_field_mut(first_intent_mut(core), "coreEvaluationBudget"),
            "maxAllocatedBytes",
            bytes.clone(),
        );
        set(
            value_field_mut(selected_target_mut(target), "coreEvaluationBudget"),
            "maxAllocatedBytes",
            bytes,
        );
    })
}

/// What the runtime allocates to run `ensure-node-control` on its largest
/// input, a 1,024-byte atom. Pinned by execution in
/// `ensure_node_needs_exactly_its_worst_case_allocation`.
const ENSURE_NODE_WORST_ALLOCATION: u64 = 4258;

/// The `ensure-node` fixture's input with its atom at the type's 1,024-byte cap.
fn largest_ensure_node_input() -> Vec<u8> {
    let (_, input) = fixture("ensure-node", GRAPH_EFFECTS);
    let mut input = echo_edict_canonical::decode_canonical_cbor_v1(&input).expect("input");
    set(&mut input, "atomBytes", V::Bytes(vec![0x61; 1024]));
    encoded(&input)
}

/// Runs an `ensure-node` package on its largest input under frame limits far
/// above its own Core budget, so only that budget can refuse it.
fn run_largest_ensure_node(package: &V) -> Result<Vec<u8>, EffectFrameError> {
    run(package, &largest_ensure_node_input())
}

/// Sets only the declared output ceiling, in Core and its lowered intent.
fn with_output(package: &V, bytes: u64) -> V {
    rebuild(package, |core, target, _| {
        let bytes = V::Integer(i128::from(bytes));
        set(
            value_field_mut(first_intent_mut(core), "coreEvaluationBudget"),
            "maxOutputBytes",
            bytes.clone(),
        );
        set(
            value_field_mut(selected_target_mut(target), "coreEvaluationBudget"),
            "maxOutputBytes",
            bytes,
        );
    })
}

/// Replaces every occurrence of one text value, wherever it appears.
fn replace_text(value: &mut V, from: &str, to: &str) {
    match value {
        V::Text(found) if found == from => *value = text(to),
        V::Array(values) => values
            .iter_mut()
            .for_each(|value| replace_text(value, from, to)),
        V::Map(fields) => fields
            .iter_mut()
            .for_each(|(_, value)| replace_text(value, from, to)),
        _ => {}
    }
}

/// Removes `key` from every input constraint, in Core and its lowered intent.
fn without_constraint_field(package: &V, key: &str) -> V {
    let strip = |constraints: &mut V| {
        let V::Array(constraints) = constraints else {
            panic!("constraints")
        };
        for constraint in constraints {
            let V::Map(fields) = constraint else {
                panic!("constraint")
            };
            fields.retain(|(name, _)| name != &text(key));
        }
    };
    rebuild(package, |core, target, _| {
        strip(value_field_mut(first_intent_mut(core), "inputConstraints"));
        strip(value_field_mut(
            selected_target_mut(target),
            "inputConstraints",
        ));
    })
}

/// The captured read with its exports replaced, and every lowering pin
/// re-derived from them, so the program reaches the checks on its exports.
fn read_with_exports(exports: V) -> V {
    let lowering = lowering_fixture_inputs(None, Some(exports.clone()), None);
    let (core, target, _) = compiler_fixture(&lowering);
    with_budget(
        &publication_from_parts(&core, &target, &exports, &lowering),
        READ_AMPLE,
    )
}

fn captured_exports() -> V {
    compiler_fixture(&repinned_lowering_artifacts()).2
}

/// A body whose `require` fails with a 64-byte text payload, over a `Bool`
/// result. The payload, not the result, needs the output ceiling.
fn require_with_text_payload() -> V {
    with_budget(
        &package(
            vec![at_most(input("at"), len(input("text")))],
            vec![
                Node::RequireWith(
                    at_most(input("at"), len(input("other"))),
                    map([("text", input("text"))]),
                ),
                Node::Let(ok_binding(), prefix_is_utf8("text", input("at"))),
            ],
            local(&ok_binding()),
        ),
        AMPLE,
    )
}

/// The output ceiling the corpus gives `require_with_text_payload`: room for
/// any `Bool` result, not for the failure payload.
const REQUIRE_PAYLOAD_OUTPUT: u64 = 16;

/// An input whose constraint holds and whose `require` fails, with the
/// payload text at its 64-byte cap.
fn failing_require_input() -> Vec<u8> {
    encoded(&map([
        ("at", V::Integer(5)),
        ("other", V::Bytes(Vec::new())),
        ("text", V::Bytes(vec![0x61; 64])),
    ]))
}

/// Runs a package's effect program once under frame limits far above any
/// Core budget in this corpus, so only the package's own budget can refuse it.
fn run(package: &V, input: &[u8]) -> Result<Vec<u8>, EffectFrameError> {
    let prepared = prepare(package).expect("package prepares");
    let program = prepared.program().as_effect_program().expect("effect");
    let (host, head, _) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10_000, 4096, 4096),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    frame
        .evaluate_effect(program, input, &[])
        .map(<[u8]>::to_vec)
}

/// Names the package's Core as its canonical meaning, as Echo's lowerer
/// publishes every package. The shared test package names an unrelated one.
fn as_published(mut package: V) -> V {
    let core = get(get(&package, "semantic_closure"), "core_identity").clone();
    set(
        value_field_mut(&mut package, "semantic_closure"),
        "canonical_meaning_identity",
        core,
    );
    package
}

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
    let two_lets = with_budget(
        &package(
            vec![at_most(input("at"), len(input("text")))],
            vec![
                Node::Let(declared("local.size", "U64"), len(input("text"))),
                Node::Let(ok_binding(), prefix_is_utf8("text", input("at"))),
            ],
            local(&ok_binding()),
        ),
        AMPLE,
    );
    let mut undeclared = captured_exports();
    let V::Array(obstructions) = value_field_mut(&mut undeclared, "obstructions") else {
        panic!("obstructions")
    };
    // The `missing` failure arm maps to this obstruction.
    obstructions
        .retain(|obstruction| get(obstruction, "coordinate") != &text("jedit.text@1.FactMissing"));
    let entry = |name, accept, runtime, package: V| Entry {
        name,
        accept,
        runtime,
        package: as_published(package),
    };
    vec![
        entry("read-control", true, Runtime::Accepts, read_control),
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
        // The same two-`let` body in its own order, then with its lowered
        // execution order swapped.
        entry(
            "two-let-control",
            true,
            Runtime::Accepts,
            rebuild(&two_lets, |_, _, _| {}),
        ),
        entry(
            "forged-node-order",
            false,
            Runtime::Refuses,
            rebuild(&two_lets, |_, target, _| {
                let V::Array(order) =
                    value_field_mut(selected_target_mut(target), "executionOrder")
                else {
                    panic!("order")
                };
                order.swap(0, 1);
            }),
        ),
        entry(
            "wrong-intrinsic-input-type",
            false,
            Runtime::Refuses,
            fixture_with("ensure-node", GRAPH_EFFECTS, &wrong_input).0,
        ),
        // The captured exports with every lowering pin re-derived, then the
        // same exports missing one obstruction a failure arm names.
        entry(
            "read-exports-control",
            true,
            Runtime::Accepts,
            read_with_exports(captured_exports()),
        ),
        entry(
            "undeclared-obstruction",
            false,
            Runtime::Refuses,
            read_with_exports(undeclared),
        ),
        // The catalog-named adapter's unedited failure table. The unknown and
        // duplicate entries below edit only that table; the tag-spelled entry
        // spells the same table by runtime tag instead of by name.
        entry(
            "read-adapter-control",
            true,
            Runtime::Accepts,
            read_with_adapter(|_| {}, catalog_fixture()),
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
        // Each input constraint is judged with no facts, so one constraint
        // cannot prove another; the slice in the second is unproven.
        entry(
            "constraint-proven-by-constraint",
            false,
            Runtime::Refuses,
            with_budget(
                &slice_program(
                    vec![
                        at_most(input("at"), len(input("text"))),
                        is_true(prefix_is_utf8("text", input("at"))),
                    ],
                    "text",
                ),
                AMPLE,
            ),
        ),
        // Within one `all`, an earlier conjunct does prove a later one.
        entry(
            "constraint-conjunct-proven",
            true,
            Runtime::Accepts,
            with_budget(
                &slice_program(
                    vec![all(vec![
                        at_most(input("at"), len(input("text"))),
                        is_true(prefix_is_utf8("text", input("at"))),
                    ])],
                    "text",
                ),
                AMPLE,
            ),
        ),
        entry(
            "over-budget",
            false,
            Runtime::Executes,
            with_budget(&proven_slice(), 1),
        ),
        // Exactly the runtime's worst case, which includes the emitted
        // mutation's row reservation, and then one byte short of it.
        entry(
            "mutation-allocation-exact",
            true,
            Runtime::Accepts,
            with_allocation(
                &fixture("ensure-node", GRAPH_EFFECTS).0,
                ENSURE_NODE_WORST_ALLOCATION,
            ),
        ),
        entry(
            "mutation-allocation-short",
            false,
            Runtime::Executes,
            with_allocation(
                &fixture("ensure-node", GRAPH_EFFECTS).0,
                ENSURE_NODE_WORST_ALLOCATION - 1,
            ),
        ),
        // A failing `require` encodes its payload under the result's output
        // ceiling; this one leaves room for the `Bool` result only.
        entry(
            "require-payload-over-output",
            false,
            Runtime::Executes,
            with_output(&require_with_text_payload(), REQUIRE_PAYLOAD_OUTPUT),
        ),
        // The runtime emits at most 1,024 mutations per evaluation.
        entry(
            "mutation-count-over",
            false,
            Runtime::Executes,
            with_budget(
                &repeat_emission(&fixture("ensure-node", GRAPH_EFFECTS).0, 1025),
                AMPLE,
            ),
        ),
        // A Core local name must be present.
        entry(
            "local-name-empty",
            false,
            Runtime::Refuses,
            rebuild(
                &fixture("ensure-node", GRAPH_EFFECTS).0,
                |core, target, _| {
                    replace_text(core, "$local0", "");
                    replace_text(target, "$local0", "");
                },
            ),
        ),
        // The runtime names a failed input constraint by its coordinate.
        entry(
            "constraint-without-coordinate",
            false,
            Runtime::Executes,
            without_constraint_field(&proven_slice(), "coordinate"),
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
            " accept
"
        } else {
            " refuse
"
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
            format!(
                "{}
",
                hex::encode(encoded(&entry.package))
            ),
        )
        .expect("corpus entry");
    }
    std::fs::write(dir.join("verdicts.txt"), manifest(&entries)).expect("manifest");
}

#[test]
fn ensure_node_needs_exactly_its_worst_case_allocation() {
    let control = fixture("ensure-node", GRAPH_EFFECTS).0;
    run_largest_ensure_node(&with_allocation(&control, ENSURE_NODE_WORST_ALLOCATION))
        .expect("the worst case fits its exact allocation");
    assert_eq!(
        run_largest_ensure_node(&with_allocation(&control, ENSURE_NODE_WORST_ALLOCATION - 1)),
        Err(EffectFrameError::BoundedRead(ReadError::Evaluation(
            EvaluationError::AllocationBudgetExceeded
        ))),
        "one byte less than the worst case must not be enough"
    );
}

/// The runtime's own refusals for the corpus entries it refuses only when it
/// executes: each one fails on an input its types and constraints admit.
#[test]
fn the_runtime_refuses_each_executed_entry_when_it_runs() {
    let output = |package: &V| run(package, &failing_require_input());
    assert!(
        matches!(
            output(&with_output(&require_with_text_payload(), 65536)),
            Err(EffectFrameError::AuthoredObstruction { .. })
        ),
        "with room, the failing require reports its own obstruction"
    );
    assert_eq!(
        output(&with_output(
            &require_with_text_payload(),
            REQUIRE_PAYLOAD_OUTPUT
        )),
        Err(EffectFrameError::BoundedRead(ReadError::Evaluation(
            EvaluationError::OutputBudgetExceeded
        ))),
        "the payload does not fit the output ceiling the corpus declares"
    );
    let slice_input = encoded(&map([
        ("at", V::Integer(0)),
        ("other", V::Bytes(Vec::new())),
        ("text", V::Bytes(Vec::new())),
    ]));
    run(&proven_slice(), &slice_input).expect("the coordinated control runs");
    assert!(
        run(
            &without_constraint_field(&proven_slice(), "coordinate"),
            &slice_input
        )
        .is_err(),
        "a constraint without a coordinate cannot run"
    );
}
