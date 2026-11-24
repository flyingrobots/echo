// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic structured (`echo.span-ir/v3`) controls under `echo.dpo@2`.
//!
//! Every artifact here is internally repinned from the synthetic fixtures; it
//! is runtime evidence only, not public compiler or provider evidence.
use super::execution::{host, limits};
use super::mutations::{fixture_in, Shape};
use super::*;
use warp_core::edict_effects::EffectMutation;
use warp_core::{EchoOperationApplicationBasisV1, EchoOperationBudgetV1};

const STRUCTURED: &str = "echo.span-ir/v3";
const PROFILE: &str = "continuum.profile.graph-effects/v1";

fn embedded(value: &V, key: &str) -> V {
    let V::Bytes(bytes) = get(value, key) else {
        panic!("embedded bytes")
    };
    echo_edict_canonical::decode_canonical_cbor_v1(bytes).expect("canonical bytes")
}
fn local(id: &str, ty: &str) -> V {
    map([
        ("id", text(id)),
        ("type", text(ty)),
        ("alphaName", text(&format!("${id}"))),
    ])
}
fn reference(local: &V) -> V {
    map([("kind", text("local")), ("ref", local.clone())])
}
fn null() -> V {
    map([
        ("kind", text("const")),
        ("value", map([("kind", text("null"))])),
    ])
}
fn digest(domain: &str, value: &V) -> V {
    V::Array(vec![text("sha256"), artifact_identity(domain, value)])
}
fn coordinate(value: &V) -> String {
    let (V::Text(id), V::Text(version)) = (get(value, "id"), get(value, "version")) else {
        panic!("coordinate")
    };
    format!("{id}@{version}")
}

/// The four artifacts a compiled effect program embeds, rebound together.
pub(super) struct Parts {
    core: V,
    target: V,
    exports: V,
    lowering: V,
}
impl Parts {
    pub(super) fn publication(&mut self) -> V {
        set(
            value_field_mut(
                value_field_mut(&mut self.target, "semanticClosure"),
                "sourceCore",
            ),
            "digest",
            digest("edict.core.module/v1", &self.core),
        );
        publication_from_parts(&self.core, &self.target, &self.exports, &self.lowering)
    }
}

/// Declares the program's Target IR as structured `echo.span-ir/v3` and
/// rebinds every pin that follows: the profile's Target IR, the adapter that
/// accepts it, the lawpack, the source preamble and the Core import.
fn structured_parts(publication: &V) -> Parts {
    let program = embedded(publication, "program");
    let bundle = embedded(&program, "compiled_program");
    let mut parts = Parts {
        core: embedded(&bundle, "core_artifact"),
        target: embedded(&bundle, "target_ir_artifact"),
        exports: embedded(&bundle, "lawpack_exports_artifact"),
        lowering: get(&bundle, "lowering_artifacts").clone(),
    };
    let mut profile = embedded(&parts.lowering, "target_profile_artifact");
    let mut lawpack = embedded(&parts.lowering, "lawpack_artifact");
    let old_digest = echo_edict_canonical::digest_canonical_value_v1("edict.lawpack/v1", &lawpack)
        .expect("digest");
    set(
        value_field_mut(&mut profile, "targetIr"),
        "id",
        text(STRUCTURED),
    );
    let V::Array(adapters) = value_field_mut(&mut lawpack, "targetAdapters") else {
        panic!("adapters")
    };
    set(
        &mut adapters[0],
        "acceptedTargetIr",
        get(&profile, "targetIr").clone(),
    );
    set(
        value_field_mut(&mut adapters[0], "acceptedTargetProfile"),
        "digest",
        digest("edict.target-profile/v1", &profile),
    );
    set(
        &mut parts.target,
        "targetProfile",
        get(&adapters[0], "acceptedTargetProfile").clone(),
    );
    let lawpack_ref = map([
        ("id", text(&coordinate(&lawpack))),
        ("digest", digest("edict.lawpack/v1", &lawpack)),
    ]);
    let V::Array(imports) = value_field_mut(&mut parts.core, "imports") else {
        panic!("imports")
    };
    for import in imports.iter_mut() {
        if get(import, "kind") == &text("lawpack") {
            set(import, "ref", lawpack_ref.clone());
        }
    }
    set(
        value_field_mut(&mut parts.target, "semanticClosure"),
        "lawpacks",
        V::Array(vec![lawpack_ref]),
    );
    let V::Bytes(source) = embedded(&parts.lowering, "source_artifact") else {
        panic!("source")
    };
    let new_digest = echo_edict_canonical::digest_canonical_value_v1("edict.lawpack/v1", &lawpack)
        .expect("digest");
    let source = String::from_utf8(source)
        .expect("source")
        .replace(&old_digest, &new_digest);
    set(
        &mut parts.lowering,
        "source_artifact",
        V::Bytes(encoded(&V::Bytes(source.into_bytes()))),
    );
    set(
        &mut parts.lowering,
        "lawpack_artifact",
        V::Bytes(encoded(&lawpack)),
    );
    set(
        &mut parts.lowering,
        "target_profile_artifact",
        V::Bytes(encoded(&profile)),
    );
    set(&mut parts.target, "domain", text(STRUCTURED));
    set(
        &mut parts.target,
        "kind",
        text("structuredTargetIrArtifact"),
    );
    parts
}

/// One `ensure-node` effect site inside `for item in arg.0 bounded max`, over
/// `List<MutationInput,max=max>`, under the effect target profile `namespace`.
pub(super) fn loop_parts(namespace: &str, max: u16) -> (Parts, V) {
    let (publication, input) = fixture_in(
        namespace,
        "ensure-node",
        PROFILE,
        &Shape::conforming_in("ensure-node", namespace),
    );
    let mut parts = structured_parts(&publication);
    let arg = local("arg.0", &format!("List<MutationInput,max={max}>"));
    set(
        &mut parts.core,
        "requiredCoreCapabilities",
        V::Array(vec![text("core.list/v1")]),
    );
    let binder = local("local.item", "MutationInput");
    let intent = value_field_mut(value_field_mut(&mut parts.core, "intents"), "replaceRange");
    let V::Array(nodes) = get(get(intent, "body"), "nodes") else {
        panic!("nodes")
    };
    let mut effect = nodes[0].clone();
    set(&mut effect, "input", reference(&binder));
    let output = map([("kind", text("record")), ("fields", map([]))]);
    let bound = map([
        ("kind", text("literal")),
        ("value", V::Integer(i128::from(max))),
    ]);
    let body = map([
        (
            "locals",
            V::Array(vec![binder.clone(), get(&effect, "binding").clone()]),
        ),
        ("nodes", V::Array(vec![effect])),
        ("result", null()),
    ]);
    set(intent, "input", get(&arg, "type").clone());
    set(
        intent,
        "body",
        map([
            ("locals", V::Array(vec![arg.clone()])),
            (
                "nodes",
                V::Array(vec![map([
                    ("kind", text("for")),
                    ("binder", binder.clone()),
                    ("iter", reference(&arg)),
                    ("bound", bound.clone()),
                    ("body", body),
                ])]),
            ),
            ("result", output.clone()),
        ]),
    );
    let selected = selected_target_mut(&mut parts.target);
    let V::Array(steps) = value_field_mut(selected, "steps") else {
        panic!("steps")
    };
    set(&mut steps[0], "input", reference(&binder));
    set(selected, "result", output);
    set(
        selected,
        "executionOrder",
        V::Array(vec![map([
            ("kind", text("for")),
            ("id", text("for.0")),
            ("binder", binder),
            ("iterable", reference(&arg)),
            ("bound", bound),
            ("body", V::Array(vec![text("effect.0")])),
        ])]),
    );
    (
        parts,
        echo_edict_canonical::decode_canonical_cbor_v1(&input).expect("input"),
    )
}
/// `let node = arg.0.node; for tick in arg.0.ticks bounded max { ensure-node(node) }`
/// over `List<U8,max=max>`: one effect site that does not read its binder, so
/// the input stays small however many times the site runs.
pub(super) fn counting_parts(max: u16) -> (Parts, V) {
    let (publication, input) = fixture_in(
        "echo.dpo@2",
        "ensure-node",
        PROFILE,
        &Shape::conforming_in("ensure-node", "echo.dpo@2"),
    );
    let node = echo_edict_canonical::decode_canonical_cbor_v1(&input).expect("input");
    let mut parts = structured_parts(&publication);
    let ticks = format!("List<U8,max={max}>");
    add(
        value_field_mut(&mut parts.core, "types"),
        "CountInput",
        map([
            ("kind", text("Record")),
            (
                "fields",
                map([("node", text("MutationInput")), ("ticks", text(&ticks))]),
            ),
        ]),
    );
    set(
        &mut parts.core,
        "requiredCoreCapabilities",
        V::Array(vec![text("core.list/v1")]),
    );
    let arg = local("arg.0", "CountInput");
    let binder = local("local.tick", "U8");
    let node_local = local("local.node", "MutationInput");
    let field_of = |name: &str| {
        map([
            ("kind", text("field")),
            ("base", reference(&arg)),
            ("field", text(name)),
        ])
    };
    let intent = value_field_mut(value_field_mut(&mut parts.core, "intents"), "replaceRange");
    let V::Array(nodes) = get(get(intent, "body"), "nodes") else {
        panic!("nodes")
    };
    let mut effect = nodes[0].clone();
    set(&mut effect, "input", reference(&node_local));
    let node_let = map([
        ("kind", text("let")),
        ("binding", node_local.clone()),
        ("value", field_of("node")),
    ]);
    let output = map([("kind", text("record")), ("fields", map([]))]);
    let bound = map([
        ("kind", text("literal")),
        ("value", V::Integer(i128::from(max))),
    ]);
    set(intent, "input", text("CountInput"));
    set(
        intent,
        "body",
        map([
            ("locals", V::Array(vec![arg.clone(), node_local.clone()])),
            (
                "nodes",
                V::Array(vec![
                    node_let,
                    map([
                        ("kind", text("for")),
                        ("binder", binder.clone()),
                        ("iter", field_of("ticks")),
                        ("bound", bound.clone()),
                        (
                            "body",
                            map([
                                (
                                    "locals",
                                    V::Array(vec![binder.clone(), get(&effect, "binding").clone()]),
                                ),
                                ("nodes", V::Array(vec![effect])),
                                ("result", null()),
                            ]),
                        ),
                    ]),
                ]),
            ),
            ("result", output.clone()),
        ]),
    );
    let selected = selected_target_mut(&mut parts.target);
    let V::Array(steps) = value_field_mut(selected, "steps") else {
        panic!("steps")
    };
    set(&mut steps[0], "input", reference(&node_local));
    set(selected, "result", output);
    if let V::Map(fields) = selected {
        fields.retain(|(key, _)| key != &text("pureBindings"));
    }
    add(
        selected,
        "pureBindings",
        V::Array(vec![map([
            ("id", text("let.0")),
            ("binding", node_local),
            ("value", field_of("node")),
        ])]),
    );
    set(
        selected,
        "executionOrder",
        V::Array(vec![
            text("let.0"),
            map([
                ("kind", text("for")),
                ("id", text("for.0")),
                ("binder", binder),
                ("iterable", field_of("ticks")),
                ("bound", bound),
                ("body", V::Array(vec![text("effect.0")])),
            ]),
        ]),
    );
    (parts, node)
}
fn loop_fixture_in(namespace: &str) -> (Parts, V) {
    loop_parts(namespace, 3)
}
fn loop_fixture() -> (Parts, V) {
    loop_fixture_in("echo.dpo@2")
}

fn core_body(parts: &mut Parts) -> &mut V {
    value_field_mut(
        value_field_mut(value_field_mut(&mut parts.core, "intents"), "replaceRange"),
        "body",
    )
}
fn first(value: &mut V) -> &mut V {
    let V::Array(values) = value else {
        panic!("array")
    };
    &mut values[0]
}
fn condition(taken: bool) -> V {
    let integer = |value| {
        map([
            ("kind", text("const")),
            (
                "value",
                map([
                    ("kind", text("int")),
                    ("width", text("U8")),
                    ("value", V::Integer(value)),
                ]),
            ),
        ])
    };
    map([
        ("kind", text("compare")),
        ("op", text("==")),
        ("left", integer(1)),
        ("right", integer(i128::from(taken))),
    ])
}
/// Wraps the whole body in `if 1 == taken { body } else { }`.
pub(super) fn branch(parts: &mut Parts, taken: bool) {
    let body = core_body(parts);
    let V::Array(locals) = get(body, "locals") else {
        panic!("locals")
    };
    let parent = locals
        .iter()
        .find(|l| get(l, "id") == &text("arg.0"))
        .expect("input")
        .clone();
    let child = map([
        (
            "locals",
            V::Array(
                locals
                    .iter()
                    .filter(|l| get(l, "id") != &text("arg.0"))
                    .cloned()
                    .collect(),
            ),
        ),
        ("nodes", get(body, "nodes").clone()),
        ("result", null()),
    ]);
    set(body, "locals", V::Array(vec![parent]));
    set(
        body,
        "nodes",
        V::Array(vec![map([
            ("kind", text("branch")),
            ("predicate", condition(taken)),
            ("then", child),
            (
                "else",
                map([
                    ("locals", V::Array(vec![])),
                    ("nodes", V::Array(vec![])),
                    ("result", null()),
                ]),
            ),
        ])]),
    );
    let target = selected_target_mut(&mut parts.target);
    let order = get(target, "executionOrder").clone();
    set(
        target,
        "executionOrder",
        V::Array(vec![map([
            ("kind", text("branch")),
            ("id", text("branch.0")),
            ("predicate", condition(taken)),
            ("then", order),
            ("else", V::Array(vec![])),
        ])]),
    );
}
/// Runs one evaluation and checks that the store is untouched; returns
/// success, the buffered mutations, the charged budget and the reads.
fn evaluate(
    publication: &V,
    input: &V,
    budget: EchoOperationBudgetV1,
) -> (bool, Vec<EffectMutation>, EchoOperationBudgetV1, usize) {
    let package = prepare(publication).expect("prepared controls");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, root) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(basis, &[], 0, budget, limits())
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let success = frame.evaluate_effect(program, &encoded(input), &[]).is_ok();
    let state = host
        .runtime()
        .worldlines()
        .get(&head.worldline_id)
        .expect("worldline")
        .state();
    let store = state.store(&root.warp_id).expect("store");
    assert_eq!(
        store.iter_nodes().count(),
        1,
        "evaluation must not insert detached nodes either"
    );
    assert_eq!(
        store.node(&root.local_id).expect("root").ty,
        TypeId([1; 32])
    );
    let Some(warp_core::AttachmentValue::Atom(atom)) = store.node_attachment(&root.local_id) else {
        panic!("root atom")
    };
    assert_eq!(atom.bytes.as_ref(), b"before");
    (
        success,
        frame.buffered_mutations().to_vec(),
        frame.consumed(),
        frame.readings().len(),
    )
}
fn items(item: &V, count: u8) -> V {
    V::Array(
        (0..count)
            .map(|index| {
                let mut item = item.clone();
                set(&mut item, "nodeId", id(index + 20));
                item
            })
            .collect(),
    )
}
fn ensured_nodes(mutations: &[EffectMutation]) -> Vec<u8> {
    mutations
        .iter()
        .map(|mutation| {
            let EffectMutation::EnsureNode { node, .. } = mutation else {
                panic!("ensure-node mutation")
            };
            node.local_id.0[0]
        })
        .collect()
}

/// Oracle: Core `for` semantics. Each item binds a fresh binder, the one
/// effect site runs once per item in list order, and an empty list runs it
/// never. Mutations stay buffered; the store is unchanged.
#[test]
fn structured_loop_runs_one_effect_site_per_item_from_empty_to_maximum() {
    let (mut parts, item) = loop_fixture();
    let publication = parts.publication();
    for count in [0, 1, 3] {
        let (success, mutations, _, reads) = evaluate(
            &publication,
            &items(&item, count),
            EchoOperationBudgetV1::new(10000, 4096, 4096),
        );
        assert!(success, "{count} items");
        assert_eq!(
            ensured_nodes(&mutations),
            (20..20 + count).collect::<Vec<_>>()
        );
        assert_eq!(reads, 0);
    }
}

/// Oracle: a statement branch runs exactly the chosen arm. The unchosen arm,
/// here a loop with an effect site, emits nothing and charges no writes.
#[test]
fn structured_branch_skips_its_unchosen_loop_and_emissions() {
    for taken in [false, true] {
        let (mut parts, item) = loop_fixture();
        branch(&mut parts, taken);
        let (success, effects, used, reads) = evaluate(
            &parts.publication(),
            &items(&item, 3),
            EchoOperationBudgetV1::new(10000, 4096, 4096),
        );
        assert!(success);
        assert_eq!(effects.len(), if taken { 3 } else { 0 });
        assert_eq!(used.write_bytes() == 0, !taken);
        assert_eq!(reads, 0);
    }
}

/// Wraps the loop in `for group in arg.0 bounded 2`, over
/// `List<List<MutationInput,max=3>,max=2>`.
pub(super) fn nest(parts: &mut Parts) {
    let arg = local("arg.0", "List<List<MutationInput,max=3>,max=2>");
    let group = local("local.group", "List<MutationInput,max=3>");
    let body = core_body(parts);
    let inner = first(value_field_mut(body, "nodes"));
    set(inner, "iter", reference(&group));
    let child = map([
        ("locals", V::Array(vec![group.clone()])),
        ("nodes", get(body, "nodes").clone()),
        ("result", null()),
    ]);
    let bound = map([("kind", text("literal")), ("value", V::Integer(2))]);
    set(body, "locals", V::Array(vec![arg.clone()]));
    set(
        body,
        "nodes",
        V::Array(vec![map([
            ("kind", text("for")),
            ("binder", group.clone()),
            ("iter", reference(&arg)),
            ("bound", bound.clone()),
            ("body", child),
        ])]),
    );
    set(
        value_field_mut(value_field_mut(&mut parts.core, "intents"), "replaceRange"),
        "input",
        get(&arg, "type").clone(),
    );
    let target = selected_target_mut(&mut parts.target);
    set(
        first(value_field_mut(target, "executionOrder")),
        "iterable",
        reference(&group),
    );
    let inner = get(target, "executionOrder").clone();
    set(
        target,
        "executionOrder",
        V::Array(vec![map([
            ("kind", text("for")),
            ("id", text("for.1")),
            ("binder", group),
            ("iterable", reference(&arg)),
            ("bound", bound),
            ("body", inner),
        ])]),
    );
}

/// Oracle: nested loops run the inner body for each outer item in turn, so
/// mutations follow group order, then item order.
#[test]
fn structured_nested_loops_preserve_group_and_item_order() {
    let (mut parts, item) = loop_fixture();
    nest(&mut parts);
    let input = V::Array(
        (0..2)
            .map(|group| {
                V::Array(
                    (0..3)
                        .map(|index| {
                            let mut value = item.clone();
                            set(&mut value, "nodeId", id(20 + group * 3 + index));
                            value
                        })
                        .collect(),
                )
            })
            .collect(),
    );
    let (success, effects, _, _) = evaluate(
        &parts.publication(),
        &input,
        EchoOperationBudgetV1::new(10000, 4096, 4096),
    );
    assert!(success);
    assert_eq!(ensured_nodes(&effects), (20..26).collect::<Vec<_>>());
}

/// Oracle: the effect buffer is atomic. When a later iteration exceeds the
/// write or step budget, no mutation survives, while the charges show work
/// that was performed (earlier emissions) and none that was not.
#[test]
fn structured_failed_iteration_discards_the_whole_buffer_but_retains_prior_charges() {
    let (mut parts, item) = loop_fixture();
    let publication = parts.publication();
    let input = items(&item, 3);
    let (success, effects, full, _) = evaluate(
        &publication,
        &input,
        EchoOperationBudgetV1::new(10000, 4096, 4096),
    );
    assert!(success);
    assert_eq!(effects.len(), 3);
    let per_effect = full.write_bytes() / 3;
    assert_eq!(full.write_bytes(), per_effect * 3);
    // One byte short of the third emission: two were charged, the third not.
    let (success, effects, used, _) = evaluate(
        &publication,
        &input,
        EchoOperationBudgetV1::new(10000, 4096, full.write_bytes() - 1),
    );
    assert!(!success);
    assert!(effects.is_empty());
    assert_eq!(used.write_bytes(), per_effect * 2);
    assert!(used.steps() > 0 && used.steps() < full.steps());
    // One step short of the whole run: every emission was charged.
    let (success, effects, used, _) = evaluate(
        &publication,
        &input,
        EchoOperationBudgetV1::new(full.steps() - 1, 4096, 4096),
    );
    assert!(!success);
    assert!(effects.is_empty());
    assert_eq!(used.write_bytes(), full.write_bytes());
}

/// One way to forge the structured loop. Each forgery is fully repinned.
#[derive(Clone, Copy, Debug)]
pub(super) enum Forgery {
    /// A literal bound below the list type's maximum.
    BoundBelowList,
    /// A coordinate bound, which Target cannot authenticate.
    CoordinateBound,
    /// The result reads a local bound inside the loop body.
    LeakedLocal,
    /// The loop's control id reuses an instruction id.
    ControlIdReused,
    /// The order drops the loop body's instruction.
    DroppedBody,
    /// An ordered (`echo.span-ir/v2`) declaration carrying control entries.
    OrderedWithControl,
    /// The Target loop names a binder its Core loop does not.
    SubstitutedBinder,
    /// The loop body, a statement block, yields a value.
    BodyWithResult,
}
impl Forgery {
    pub(super) const ALL: [Self; 8] = [
        Self::BoundBelowList,
        Self::CoordinateBound,
        Self::LeakedLocal,
        Self::ControlIdReused,
        Self::DroppedBody,
        Self::OrderedWithControl,
        Self::SubstitutedBinder,
        Self::BodyWithResult,
    ];
}
fn loop_order(parts: &mut Parts) -> &mut V {
    first(value_field_mut(
        selected_target_mut(&mut parts.target),
        "executionOrder",
    ))
}
fn loop_node(parts: &mut Parts) -> &mut V {
    first(value_field_mut(core_body(parts), "nodes"))
}
/// Applies one forgery to a loop built by [`loop_parts`].
pub(super) fn forge(parts: &mut Parts, forgery: Forgery) {
    match forgery {
        Forgery::BoundBelowList | Forgery::CoordinateBound => {
            let bound = if matches!(forgery, Forgery::BoundBelowList) {
                map([("kind", text("literal")), ("value", V::Integer(2))])
            } else {
                map([
                    ("kind", text("coordinate")),
                    ("ref", text("untrusted.bound")),
                ])
            };
            set(loop_node(parts), "bound", bound.clone());
            set(loop_order(parts), "bound", bound);
        }
        Forgery::LeakedLocal => {
            let effect = first(value_field_mut(
                value_field_mut(loop_node(parts), "body"),
                "nodes",
            ));
            let escaped = reference(get(effect, "binding"));
            set(core_body(parts), "result", escaped.clone());
            set(selected_target_mut(&mut parts.target), "result", escaped);
        }
        Forgery::ControlIdReused => set(loop_order(parts), "id", text("effect.0")),
        Forgery::DroppedBody => set(loop_order(parts), "body", V::Array(vec![])),
        Forgery::OrderedWithControl => {
            set(&mut parts.target, "domain", text("echo.span-ir/v2"));
            set(&mut parts.target, "kind", text("orderedTargetIrArtifact"));
        }
        Forgery::SubstitutedBinder => set(
            loop_order(parts),
            "binder",
            local("local.other", "MutationInput"),
        ),
        Forgery::BodyWithResult => set(
            value_field_mut(loop_node(parts), "body"),
            "result",
            map([("kind", text("record")), ("fields", map([]))]),
        ),
    }
}

/// Oracle: v3 admits only literal bounds covering the list type, loop and
/// branch locals scoped to their block, and an order tree that corresponds
/// to Core node for node. Every case is fully repinned, and the unchanged
/// control prepares, so no stale pin explains a refusal.
#[test]
fn structured_refuses_invalid_bounds_scope_leaks_and_substituted_order_under_fresh_pins() {
    prepare(&loop_fixture().0.publication()).expect("the repinned control prepares");
    for forgery in Forgery::ALL {
        let (mut parts, _) = loop_fixture();
        forge(&mut parts, forgery);
        assert!(
            prepare(&parts.publication()).is_err(),
            "{forgery:?} must refuse"
        );
    }
}

/// Oracle: the owner decision on Echo #947. `echo.dpo@1` stays frozen, so a
/// structured program pinned to it is refused while the same program under
/// `echo.dpo@2` prepares.
#[test]
fn structured_programs_are_refused_under_the_frozen_echo_dpo_1_profile() {
    prepare(&loop_fixture().0.publication()).expect("echo.dpo@2 admits the loop");
    let (mut parts, _) = loop_fixture_in("echo.dpo@1");
    assert!(prepare(&parts.publication()).is_err());
}

/// Renames every `echo.dpo@1.` intrinsic name inside `value` to `echo.dpo@2.`.
fn renamespaced(value: &V) -> V {
    match value {
        V::Text(name) => V::Text(name.replacen("echo.dpo@1.", "echo.dpo@2.", 1)),
        V::Array(items) => V::Array(items.iter().map(renamespaced).collect()),
        V::Map(fields) => V::Map(
            fields
                .iter()
                .map(|(key, value)| (renamespaced(key), renamespaced(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}
/// The synthetic atom-read program, rebound under `echo.dpo@2`.
fn read_parts() -> Parts {
    let artifacts = repinned_lowering_artifacts();
    let decode = |key| {
        let V::Bytes(raw) = get(&artifacts, key) else {
            panic!("artifact")
        };
        echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("artifact")
    };
    let lowering = lowering_fixture_in(
        "echo.dpo@2",
        Some(renamespaced(&decode("intrinsics_artifact"))),
        None,
        Some(renamespaced(&decode("adapter_artifact"))),
    );
    let (core, target, exports) = compiler_fixture(&lowering);
    let target = renamespaced(&target);
    let mut publication = publication_from_parts(&core, &target, &exports, &lowering);
    let mut program = embedded(&publication, "program");
    let mut permissions = law();
    // A typed atom read resolves both a node slot and its attachment slot.
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
    structured_parts(&publication)
}

/// Oracle: an unchosen branch performs no reads. The read sits in the
/// unchosen arm, so the run needs no aperture and no read budget.
#[test]
fn structured_unchosen_branch_performs_no_atom_reads() {
    let mut parts = read_parts();
    prepare(&parts.publication()).expect("the read declared structured under echo.dpo@2 prepares");
    let body = core_body(&mut parts);
    let V::Array(locals) = get(body, "locals") else {
        panic!("locals")
    };
    let arg = locals
        .iter()
        .find(|l| get(l, "id") == &text("arg.0"))
        .expect("input");
    let result = map([
        ("kind", text("field")),
        ("base", reference(arg)),
        ("field", text("expected")),
    ]);
    set(body, "result", result.clone());
    set(selected_target_mut(&mut parts.target), "result", result);
    branch(&mut parts, false);
    let (_, _, node) = host();
    let bytes = super::execution::input(node, b"before");
    let input = echo_edict_canonical::decode_canonical_cbor_v1(&bytes).expect("input");
    let (success, effects, used, reads) = evaluate(
        &parts.publication(),
        &input,
        EchoOperationBudgetV1::new(10000, 0, 0),
    );
    assert!(
        success,
        "an unchosen read must not require an aperture or read budget"
    );
    assert!(effects.is_empty());
    assert_eq!(reads, 0);
    assert_eq!(used.read_bytes(), 0);
}
