// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic runtime controls. Public compiler/provider evidence is a separate witness.
use super::execution::{host, limits};
use super::*;
use warp_core::edict_effects::EffectMutation;
use warp_core::{EchoOperationApplicationBasisV1, EchoOperationBudgetV1};

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

struct Parts {
    core: V,
    target: V,
    exports: V,
    lowering: V,
}
impl Parts {
    fn publication(&mut self) -> V {
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
        text("echo.span-ir/v3"),
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
        ("id", text("jedit.text@1")),
        ("digest", digest("edict.lawpack/v1", &lawpack)),
    ]);
    let V::Array(imports) = value_field_mut(&mut parts.core, "imports") else {
        panic!("imports")
    };
    set(&mut imports[0], "ref", lawpack_ref.clone());
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
    set(&mut parts.target, "domain", text("echo.span-ir/v3"));
    set(
        &mut parts.target,
        "kind",
        text("structuredTargetIrArtifact"),
    );

    parts
}

fn loop_fixture() -> (Parts, V) {
    let (publication, input) =
        super::mutations::fixture("ensure-node", "continuum.profile.graph-effects/v1");
    let mut parts = structured_parts(&publication);
    let arg = local("arg.0", "List<MutationInput,max=3>");
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
    let bound = map([("kind", text("literal")), ("value", V::Integer(3))]);
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

#[test]
fn structured_loop_repeats_one_effect_site_with_fresh_items_and_no_frontier_mutation() {
    let (mut parts, item) = loop_fixture();
    let package = prepare(&parts.publication()).expect("structured loop must prepare");
    let program = package.program().as_effect_program().expect("effect");
    for count in [0, 3] {
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
                EchoOperationBudgetV1::new(10000, 4096, 4096),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        let items = (0..count)
            .map(|index| {
                let mut item = item.clone();
                set(&mut item, "nodeId", id(index + 20));
                item
            })
            .collect();
        assert_eq!(
            frame
                .evaluate_effect(program, &encoded(&V::Array(items)), &[])
                .expect("loop"),
            encoded(&map([]))
        );
        let mutations = frame.buffered_mutations();
        assert_eq!(mutations.len(), usize::from(count));
        for (index, mutation) in mutations.iter().enumerate() {
            let EffectMutation::EnsureNode { node, .. } = mutation else {
                panic!("mutation")
            };
            assert_eq!(
                node.local_id.0,
                [u8::try_from(index).expect("index") + 20; 32]
            );
        }
        assert!(frame.readings().is_empty());
    }
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
fn branch(parts: &mut Parts, taken: bool) {
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

#[test]
fn structured_branch_skips_its_unchosen_loop_and_emissions() {
    for taken in [false, true] {
        let (mut parts, item) = loop_fixture();
        branch(&mut parts, taken);
        let (success, effects, used, reads) = evaluate(
            &parts.publication(),
            &V::Array(vec![item; 3]),
            EchoOperationBudgetV1::new(10000, 4096, 4096),
        );
        assert!(success);
        assert_eq!(effects.len(), if taken { 3 } else { 0 });
        assert_eq!(used.write_bytes(), if taken { 213 } else { 0 });
        assert_eq!(reads, 0);
    }
}

#[test]
fn structured_nested_loops_preserve_group_and_item_order() {
    let (mut parts, item) = loop_fixture();
    let arg = local("arg.0", "List<List<MutationInput,max=3>,max=2>");
    let group = local("local.group", "List<MutationInput,max=3>");
    let body = core_body(&mut parts);
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
    let input = V::Array(
        (0..2)
            .map(|group| {
                V::Array(
                    (0..3)
                        .map(|index| {
                            let mut value = item.clone();
                            set(&mut value, "warpId", id(3));
                            set(&mut value, "nodeId", id(20 + group * 3 + index));
                            value
                        })
                        .collect(),
                )
            })
            .collect(),
    );
    let (success, effects, used, _) = evaluate(
        &parts.publication(),
        &input,
        EchoOperationBudgetV1::new(10000, 4096, 4096),
    );
    assert!(success);
    assert_eq!(effects.len(), 6);
    assert_eq!(used.write_bytes(), 6 * 71);
    for (index, effect) in effects.iter().enumerate() {
        let EffectMutation::EnsureNode { node, .. } = effect else {
            panic!("ensure")
        };
        assert_eq!(
            node.local_id.0,
            [20 + u8::try_from(index).expect("index"); 32]
        );
    }
}

#[test]
fn structured_failed_iteration_discards_the_whole_buffer_but_retains_prior_charges() {
    let (mut parts, item) = loop_fixture();
    let publication = parts.publication();
    let input = V::Array(vec![item; 3]);
    let (success, effects, full, _) = evaluate(
        &publication,
        &input,
        EchoOperationBudgetV1::new(10000, 4096, 213),
    );
    assert!(success);
    assert_eq!(effects.len(), 3);
    for (steps, writes, charged_writes) in [(10000, 212, 142), (full.steps() - 1, 213, 213)] {
        let (success, effects, used, _) = evaluate(
            &publication,
            &input,
            EchoOperationBudgetV1::new(steps, 4096, writes),
        );
        assert!(!success);
        assert!(effects.is_empty());
        assert_eq!(used.write_bytes(), charged_writes);
        assert!(used.steps() > 0 && used.steps() < full.steps());
    }
}

#[test]
fn structured_refuses_invalid_bounds_scope_leaks_and_substituted_order_under_fresh_pins() {
    for mode in 0..7 {
        let (mut parts, _) = loop_fixture();
        match mode {
            0 | 1 => {
                let bound = if mode == 0 {
                    map([("kind", text("literal")), ("value", V::Integer(2))])
                } else {
                    map([
                        ("kind", text("coordinate")),
                        ("ref", text("untrusted.bound")),
                    ])
                };
                set(
                    first(value_field_mut(core_body(&mut parts), "nodes")),
                    "bound",
                    bound.clone(),
                );
                set(
                    first(value_field_mut(
                        selected_target_mut(&mut parts.target),
                        "executionOrder",
                    )),
                    "bound",
                    bound,
                );
            }
            2 => {
                let effect = first(value_field_mut(
                    value_field_mut(
                        first(value_field_mut(core_body(&mut parts), "nodes")),
                        "body",
                    ),
                    "nodes",
                ));
                let escaped = reference(get(effect, "binding"));
                set(core_body(&mut parts), "result", escaped.clone());
                set(selected_target_mut(&mut parts.target), "result", escaped);
            }
            3 => set(
                first(value_field_mut(
                    selected_target_mut(&mut parts.target),
                    "executionOrder",
                )),
                "id",
                text("effect.0"),
            ),
            4 => set(
                first(value_field_mut(
                    selected_target_mut(&mut parts.target),
                    "executionOrder",
                )),
                "body",
                V::Array(vec![]),
            ),
            5 => {
                set(&mut parts.target, "domain", text("echo.span-ir/v2"));
                set(&mut parts.target, "kind", text("orderedTargetIrArtifact"));
            }
            _ => set(
                value_field_mut(
                    first(value_field_mut(core_body(&mut parts), "nodes")),
                    "body",
                ),
                "result",
                map([("kind", text("record")), ("fields", map([]))]),
            ),
        }
        assert!(
            prepare(&parts.publication()).is_err(),
            "mutation {mode} must refuse"
        );
    }
}

#[test]
fn structured_unchosen_branch_performs_no_atom_reads() {
    let mut parts = structured_parts(&super::execution::publication());
    let body = core_body(&mut parts);
    let input = get(body, "locals");
    let V::Array(locals) = input else {
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
