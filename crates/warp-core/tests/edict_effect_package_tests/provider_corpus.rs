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
use super::execution::limits;
use super::mutations::{fixture, fixture_in, fixture_with, repeat_emission, Shape};
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

/// What the runtime says about the package.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Runtime {
    /// It prepares and runs the package.
    Accepts,
    /// It refuses the package at preparation.
    Refuses,
    /// It prepares the package and refuses it only when it runs;
    /// `the_runtime_refuses_each_executed_entry_when_it_runs` and
    /// `ensure_node_needs_exactly_its_worst_case_allocation` run each one.
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

/// The `ensure-node` fixture with its Core type `name` redefined as a nominal
/// wrapper around a fresh empty-record or same-shape representation.
fn ensure_node_with_nominal(name: &str) -> V {
    rebuild(&fixture("ensure-node", GRAPH_EFFECTS).0, |core, _, _| {
        let types = value_field_mut(core, "types");
        let representation = get(types, name).clone();
        let inner = format!("{name}Representation");
        add(types, &inner, representation);
        set(
            types,
            name,
            map([
                ("kind", text("Nominal")),
                ("contract", text(name)),
                ("representation", text(&inner)),
            ]),
        );
    })
}

/// `ensure_node_with_nominal("MutationInput")` with one field the runtime's
/// nominal type entry does not have.
fn ensure_node_with_annotated_nominal() -> V {
    rebuild(&ensure_node_with_nominal("MutationInput"), |core, _, _| {
        add(
            value_field_mut(value_field_mut(core, "types"), "MutationInput"),
            "note",
            text("x"),
        );
    })
}

/// The captured read with its Target IR naming one declared failure twice.
fn read_with_repeated_failure() -> V {
    rebuild(
        &with_budget(
            &publication_for_lowering(&repinned_lowering_artifacts()),
            READ_AMPLE,
        ),
        |_, target, _| {
            let V::Array(steps) = value_field_mut(selected_target_mut(target), "steps") else {
                panic!("steps")
            };
            let V::Array(failures) = value_field_mut(&mut steps[0], "obstructionFailures") else {
                panic!("failures")
            };
            let first = failures[0].clone();
            failures.push(first);
        },
    )
}

/// The `ensure-node` fixture with an evaluation basis in Core and its
/// lowered intent.
fn ensure_node_with_basis(basis: V) -> V {
    rebuild(
        &fixture("ensure-node", GRAPH_EFFECTS).0,
        |core, target, _| {
            add(first_intent_mut(core), "basis", basis.clone());
            add(selected_target_mut(target), "basis", basis);
        },
    )
}

fn ensure_node_input_field(name: &str) -> V {
    map([
        ("kind", text("field")),
        (
            "base",
            map([
                ("kind", text("local")),
                (
                    "ref",
                    map([
                        ("id", text("arg.0")),
                        ("type", text("MutationInput")),
                        ("alphaName", text("$arg0")),
                    ]),
                ),
            ]),
        ),
        ("field", text(name)),
    ])
}

/// The two-`let` body with its `U64` local retyped through a Core type-table
/// entry `Count`, defined as `definition`.
fn two_lets_with_count(definition: V) -> V {
    rebuild(
        &with_budget(
            &package(
                vec![at_most(input("at"), len(input("text")))],
                vec![
                    Node::Let(declared("local.size", "app.Count"), len(input("text"))),
                    Node::Let(ok_binding(), prefix_is_utf8("text", input("at"))),
                ],
                local(&ok_binding()),
            ),
            AMPLE,
        ),
        |core, _, _| add(value_field_mut(core, "types"), "Count", definition),
    )
}

/// The captured read's runtime cost for an `n`-byte atom on a root-warp
/// node when its `require` fails: steps and allocated bytes. Pinned by
/// `reads_cost_a_fixed_linear_law`.
const fn read_cost(n: u64) -> (u64, u64) {
    (67 + 2 * n, 3976 + 3 * n)
}
/// `read_and_return`'s runtime cost for an `n`-byte atom on a root-warp node.
/// Pinned by `reads_cost_a_fixed_linear_law`.
const fn read_and_return_cost(n: u64) -> (u64, u64) {
    (55 + n, 3142 + 3 * n)
}
/// The atom `read-allocation-short` is run on: the largest a run can read under
/// the frame's fixed 1,048,576-step ceiling (55 + n steps), 55 bytes below the
/// read's declared 1 MiB maximum, so the providers' charge at that maximum
/// leaves the runtime's cost too little slack to hide any read charge.
const READ_WITNESS_ATOM: u64 = 1_048_521;

/// The captured read with its input's `expected` bytes narrowed to one byte,
/// so no input charge stands in for the read's own cost. The read's result
/// type, and so its declared atom maximum, is unchanged.
fn read_with_narrow_expected() -> V {
    rebuild(
        &with_budget(
            &publication_for_lowering(&repinned_lowering_artifacts()),
            READ_AMPLE,
        ),
        |core, _, _| {
            set(
                value_field_mut(
                    value_field_mut(value_field_mut(core, "types"), "Input"),
                    "fields",
                ),
                "expected",
                text("Bytes<max=1>"),
            );
        },
    )
}

/// The narrowed read with its `require` removed: it reads the atom and
/// returns it, so its cost is the read's own and the result's.
fn read_and_return() -> V {
    rebuild(&read_with_narrow_expected(), |core, target, _| {
        let V::Array(nodes) = value_field_mut(core_body_mut(core), "nodes") else {
            panic!("nodes")
        };
        nodes.retain(|node| get(node, "kind") != &text("require"));
        let selected = selected_target_mut(target);
        let V::Array(requirements) = value_field_mut(selected, "requirements") else {
            panic!("requirements")
        };
        let removed: Vec<V> = requirements
            .drain(..)
            .map(|requirement| get(&requirement, "id").clone())
            .collect();
        let V::Array(order) = value_field_mut(selected, "executionOrder") else {
            panic!("order")
        };
        order.retain(|id| !removed.contains(id));
        // Return the atom's length rather than the atom, which could not fit
        // the 64 KiB output ceiling.
        let actual = get(core_body_mut(core), "result").clone();
        let length = map([
            ("kind", text("call")),
            ("callee", text("core.bytes.length")),
            ("typeArgs", V::Array(vec![text("jedit.text@1.FactBytes")])),
            ("args", V::Array(vec![actual])),
        ]);
        set(core_body_mut(core), "result", length.clone());
        set(first_intent_mut(core), "output", text("U64"));
        set(selected_target_mut(target), "result", length);
    })
}

/// Runs a package's effect program once under frame limits far above any
/// Core budget in this corpus, so only the package's own budget can refuse it.
fn run(package: &V, input: &[u8]) -> Result<Vec<u8>, EffectFrameError> {
    run_on(package, input, &Frame::default())
}

/// The runtime frame one execution witness runs in.
struct Frame {
    /// A root-warp node `[3;32]/[4;32]` carrying an atom of this many bytes,
    /// read through a typed aperture, or no graph read at all.
    atom: Option<usize>,
    steps: u64,
    write_bytes: u64,
    limits: warp_core::edict_pure::EvaluationLimits,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            atom: None,
            steps: 10_000,
            write_bytes: 4096,
            limits: limits(),
        }
    }
}
impl Frame {
    /// The runtime frame's fixed ceilings, so only the package's own Core
    /// budget, or a cost beyond those ceilings, can refuse a witness.
    fn widest(atom: Option<usize>) -> Self {
        let mut limits = limits();
        limits.max_package_bytes = 4 * 1024 * 1024;
        limits.max_steps = 1_048_576;
        limits.max_allocated_bytes = 16 * 1024 * 1024;
        Self {
            atom,
            steps: 1_048_576,
            write_bytes: 16 * 1024 * 1024,
            limits,
        }
    }
}

/// The captured read with the footprint law a typed atom read needs: a node
/// slot and its attachment slot. The compiled program is unchanged.
fn with_read_law(mut publication: V) -> V {
    let V::Bytes(raw) = get(&publication, "program") else {
        panic!("program")
    };
    let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
    let mut permissions = law();
    set(&mut permissions, "max_reads", V::Integer(2));
    set(
        &mut program,
        "footprint_law",
        V::Bytes(encoded(&permissions)),
    );
    set(&mut publication, "program", V::Bytes(encoded(&program)));
    set(
        &mut publication,
        "footprint_contract_identity",
        footprint(&permissions),
    );
    publication
}

/// Runs a package's effect program once in `frame`, returning its result and
/// the steps and bytes it consumed.
fn run_metered(
    package: &V,
    input: &[u8],
    frame: &Frame,
) -> (Result<Vec<u8>, EffectFrameError>, u64, u64) {
    use warp_core::edict_effects::TypedNodeRead;
    use warp_core::{
        AtomPayload, AttachmentValue, GraphStore, NodeId, NodeKey, NodeRecord, WarpId,
    };
    let package = if frame.atom.is_some() {
        with_read_law(package.clone())
    } else {
        package.clone()
    };
    let prepared = prepare(&package).expect("package prepares");
    let program = prepared.program().as_effect_program().expect("effect");
    let key = NodeKey {
        warp_id: WarpId([3; 32]),
        local_id: NodeId([4; 32]),
    };
    let mut store = GraphStore::new(key.warp_id);
    store.insert_node(
        key.local_id,
        NodeRecord {
            ty: TypeId([1; 32]),
        },
    );
    store.set_node_attachment(
        key.local_id,
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: TypeId([2; 32]),
            bytes: vec![0x61; frame.atom.unwrap_or(6)].into(),
        })),
    );
    let (host, head, node) = super::execution::host_with_store(store, key);
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut evaluation = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(frame.steps, 64 * 1024 * 1024, frame.write_bytes),
            frame.limits,
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let aperture = [TypedNodeRead {
        node,
        absent_node_type: TypeId([1; 32]),
    }];
    let reads: &[TypedNodeRead] = if frame.atom.is_some() { &aperture } else { &[] };
    let result = evaluation
        .evaluate_effect(program, input, reads)
        .map(<[u8]>::to_vec);
    (
        result,
        evaluation.consumed().steps(),
        evaluation.allocated_bytes(),
    )
}

fn run_on(package: &V, input: &[u8], frame: &Frame) -> Result<Vec<u8>, EffectFrameError> {
    run_metered(package, input, frame).0
}

/// The captured read's input naming the witness node, expecting one byte, so
/// the read runs at the atom's full size and its `require` fails.
fn read_input() -> Vec<u8> {
    use warp_core::{NodeId, NodeKey, WarpId};
    super::execution::input(
        NodeKey {
            warp_id: WarpId([3; 32]),
            local_id: NodeId([4; 32]),
        },
        b"b",
    )
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

/// The read catalog with its four failures removed.
fn catalog_without_read_failures() -> V {
    let mut catalog = catalog_fixture();
    set(
        value_field_mut(
            value_field_mut(&mut catalog, "intrinsics"),
            "echo.dpo@1.node-atom-read",
        ),
        "effectFailures",
        map([]),
    );
    catalog
}

/// A read whose exported effect declares no failures and whose adapter maps
/// none, still bound to `node-atom-read`, under `catalog`. The compiler accepts
/// such a claim without a failure map (amblicit#311). With `unmapped`, the
/// Core node's obstruction map and the lowered step's arms and failures are
/// empty too, as the compiler emits them for an unmapped call.
fn read_without_failures(catalog: V, unmapped: bool) -> V {
    let mut exports = captured_exports();
    let V::Array(effects) = value_field_mut(&mut exports, "effects") else {
        panic!("effects")
    };
    for effect in effects.iter_mut() {
        set(effect, "effectFailures", map([]));
    }
    let mut adapter = catalog_named_adapter(&captured_adapter());
    set(
        value_field_mut(value_field_mut(&mut adapter, "effectImplementations"), READ),
        "failureMappings",
        map([]),
    );
    let lowering = lowering_fixture_inputs(Some(catalog), Some(exports.clone()), Some(adapter));
    let (mut core, mut target, _) = compiler_fixture(&lowering);
    if unmapped {
        let V::Array(nodes) = value_field_mut(core_body_mut(&mut core), "nodes") else {
            panic!("nodes")
        };
        for node in nodes
            .iter_mut()
            .filter(|node| get(node, "kind") == &text("effect"))
        {
            set(node, "obstructionMap", map([]));
        }
        // An unmapped call binds no obstruction payloads, so the body declares
        // no obstruction binders either.
        let V::Array(locals) = value_field_mut(core_body_mut(&mut core), "locals") else {
            panic!("locals")
        };
        locals.retain(
            |local| !matches!(get(local, "id"), V::Text(id) if id.starts_with("obstruction.")),
        );
        let step = first_step_mut(&mut target);
        set(step, "obstructionArms", map([]));
        set(step, "obstructionFailures", V::Array(vec![]));
        repin_core(&core, &mut target);
    }
    with_budget(
        &publication_from_parts(&core, &target, &exports, &lowering),
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
        // The same mutation under the echo.dpo@2 namespace, with its empty
        // failure set (#985).
        entry(
            "echo-dpo-2-ensure-node-control",
            true,
            Runtime::Accepts,
            fixture_in(
                "echo.dpo@2",
                "ensure-node",
                GRAPH_EFFECTS,
                &Shape::conforming_in("ensure-node", "echo.dpo@2"),
            )
            .0,
        ),
        entry(
            "read-without-failures",
            false,
            Runtime::Refuses,
            read_without_failures(catalog_fixture(), false),
        ),
        // The same read with every failure set empty: catalog, exports, adapter,
        // Core map and lowered arms all agree, so only Echo's fixed
        // per-intrinsic failure table (four failures for node-atom-read) refuses.
        entry(
            "read-catalog-without-failures",
            false,
            Runtime::Refuses,
            read_without_failures(catalog_without_read_failures(), true),
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
        // The catalog-named adapter's unedited failure table. The duplicate
        // entry below edits only that table; the unknown entry renames one
        // failure in both the table and the catalog; the tag-spelled entry
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
        // A nominal type may wrap an effect's input; its result must be the
        // ABI's concrete type.
        entry(
            "mutation-input-nominal",
            true,
            Runtime::Accepts,
            ensure_node_with_nominal("MutationInput"),
        ),
        entry(
            "mutation-result-nominal",
            false,
            Runtime::Refuses,
            ensure_node_with_nominal("MutationUnit"),
        ),
        // A nominal type entry has exactly its kind, contract and
        // representation.
        entry(
            "nominal-extra-field",
            false,
            Runtime::Refuses,
            ensure_node_with_annotated_nominal(),
        ),
        // Each declared failure is listed once.
        entry(
            "failure-declared-twice",
            false,
            Runtime::Refuses,
            read_with_repeated_failure(),
        ),
        // An evaluation basis is exactly 32 bytes.
        entry(
            "basis-control",
            true,
            Runtime::Accepts,
            ensure_node_with_basis(ensure_node_input_field("nodeId")),
        ),
        entry(
            "basis-not-bytes",
            false,
            Runtime::Executes,
            ensure_node_with_basis(map([
                ("kind", text("const")),
                (
                    "value",
                    map([("kind", text("bool")), ("value", V::Bool(true))]),
                ),
            ])),
        ),
        // A type table names an unsigned width directly; Core has no `Int`.
        entry(
            "type-table-width",
            true,
            Runtime::Accepts,
            two_lets_with_count(map([("kind", text("U64"))])),
        ),
        entry(
            "type-table-int",
            false,
            Runtime::Refuses,
            two_lets_with_count(map([("kind", text("Int")), ("width", text("U64"))])),
        ),
        // Siblings: the same failure payload with room for it, and the same
        // repeated emission at two mutations.
        entry(
            "require-payload-fits",
            true,
            Runtime::Accepts,
            with_output(&require_with_text_payload(), 65536),
        ),
        entry(
            "mutation-count-control",
            true,
            Runtime::Accepts,
            with_budget(
                &repeat_emission(&fixture("ensure-node", GRAPH_EFFECTS).0, 2),
                AMPLE,
            ),
        ),
        // A read that returns its atom's length, with an ample allocation
        // budget, then the same read one byte under its runtime cost on a
        // 1,048,521-byte atom. That cost includes the read's recorded
        // observations, authorizations and retained payload.
        entry(
            "read-and-return-control",
            true,
            Runtime::Accepts,
            read_and_return(),
        ),
        entry(
            "read-allocation-short",
            false,
            Runtime::Executes,
            with_allocation(
                &read_and_return(),
                read_and_return_cost(READ_WITNESS_ATOM).1 - 1,
            ),
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
    run_largest_ensure_node(&retained("mutation-allocation-exact"))
        .expect("the worst case fits its exact allocation");
    assert_eq!(
        run_largest_ensure_node(&retained("mutation-allocation-short")),
        Err(EffectFrameError::BoundedRead(ReadError::Evaluation(
            EvaluationError::AllocationBudgetExceeded
        ))),
        "one byte less than the worst case must not be enough"
    );
}

fn evaluation(error: EvaluationError) -> Result<Vec<u8>, EffectFrameError> {
    Err(EffectFrameError::BoundedRead(ReadError::Evaluation(error)))
}

/// A retained corpus package, decoded from its checked-in bytes, so a
/// witness runs exactly what the providers judge.
fn retained(name: &str) -> V {
    let hex = RETAINED
        .iter()
        .find_map(|(entry, hex)| (*entry == name).then_some(*hex))
        .unwrap_or_else(|| panic!("{name} is retained"));
    echo_edict_canonical::decode_canonical_cbor_v1(&hex::decode(hex.trim()).expect("hex"))
        .expect("canonical package")
}

/// The runtime prepares each `Executes` entry and refuses it when it runs, on
/// an input its types and constraints admit. Every run uses the retained
/// package itself. Each assertion names the exact refusal. Each refused run
/// has an accepted control beside it except `read-captured-budget`, whose
/// declared step budget equals the frame's fixed step ceiling.
#[test]
fn the_runtime_refuses_each_executed_entry_when_it_runs() {
    let slice_input = encoded(&map([
        ("at", V::Integer(0)),
        ("other", V::Bytes(Vec::new())),
        ("text", V::Bytes(Vec::new())),
    ]));
    run(&retained("slice-proven"), &slice_input).expect("the slice control runs");
    assert_eq!(
        run(&retained("over-budget"), &slice_input),
        evaluation(EvaluationError::StepBudgetExceeded),
        "over-budget"
    );
    assert_eq!(
        run(&retained("constraint-without-coordinate"), &slice_input),
        evaluation(EvaluationError::InvalidArtifact),
        "constraint-without-coordinate"
    );
    assert!(
        matches!(
            run(&retained("require-payload-fits"), &failing_require_input()),
            Err(EffectFrameError::AuthoredObstruction { .. })
        ),
        "require-payload-fits reports its own obstruction"
    );
    assert_eq!(
        run(
            &retained("require-payload-over-output"),
            &failing_require_input()
        ),
        evaluation(EvaluationError::OutputBudgetExceeded),
        "require-payload-over-output"
    );
    let (_, ensure_input) = fixture("ensure-node", GRAPH_EFFECTS);
    run(&retained("basis-control"), &ensure_input).expect("basis-control runs");
    assert_eq!(
        run(&retained("basis-not-bytes"), &ensure_input),
        evaluation(EvaluationError::InvalidArtifact),
        "basis-not-bytes"
    );
    run_on(
        &retained("mutation-count-control"),
        &ensure_input,
        &Frame::widest(None),
    )
    .expect("mutation-count-control runs");
    assert_eq!(
        run_on(
            &retained("mutation-count-over"),
            &ensure_input,
            &Frame::widest(None)
        ),
        Err(EffectFrameError::BudgetExceeded),
        "mutation-count-over"
    );
    // The captured read declares 1,048,576 steps, the same as the frame's
    // fixed step ceiling, so here the declared budget and the ceiling refuse
    // together; `read-control` runs this atom only up to the same ceiling.
    assert_eq!(
        run_on(
            &retained("read-captured-budget"),
            &read_input(),
            &Frame::widest(Some(600_000)),
        ),
        evaluation(EvaluationError::StepBudgetExceeded),
        "read-captured-budget"
    );
    let read_witness = |package: &V| {
        run_on(
            package,
            &read_input(),
            &Frame::widest(Some(usize::try_from(READ_WITNESS_ATOM).expect("atom"))),
        )
    };
    let exact = with_allocation(
        &read_and_return(),
        read_and_return_cost(READ_WITNESS_ATOM).1,
    );
    read_witness(&exact).expect("the read fits its exact cost");
    assert_eq!(
        read_witness(&retained("read-allocation-short")),
        evaluation(EvaluationError::AllocationBudgetExceeded),
        "read-allocation-short"
    );
}

/// A root-warp read's runtime cost is linear in its atom's size, for the
/// captured read whose `require` fails and for `read_and_return`.
#[test]
fn reads_cost_a_fixed_linear_law() {
    let captured = with_budget(
        &publication_for_lowering(&repinned_lowering_artifacts()),
        1_048_576,
    );
    let returned = with_budget(&read_and_return(), 1_048_576);
    // The captured read spends two steps a byte, so the frame's step ceiling
    // stops it near 524,000 bytes.
    for n in [1_u64, 4096, 65_536, 400_000] {
        let atom = usize::try_from(n).expect("atom size");
        let (result, steps, bytes) =
            run_metered(&captured, &read_input(), &Frame::widest(Some(atom)));
        assert!(
            matches!(result, Err(EffectFrameError::AuthoredObstruction { .. })),
            "n={n}: the captured read runs and its require fails"
        );
        assert_eq!((steps, bytes), read_cost(n), "captured read, n={n}");
    }
    for n in [1_u64, 4096, 65_536, READ_WITNESS_ATOM] {
        let atom = usize::try_from(n).expect("atom size");
        let (result, steps, bytes) =
            run_metered(&returned, &read_input(), &Frame::widest(Some(atom)));
        assert_eq!(result, Ok(encoded(&V::Integer(i128::from(n)))), "n={n}");
        assert_eq!(
            (steps, bytes),
            read_and_return_cost(n),
            "read_and_return, n={n}"
        );
    }
}
