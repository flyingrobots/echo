// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Effect programs carry Edict's proven-range facts: the intent's input
//! constraints dominate its whole body, and an unsigned `let` carries its
//! proven ceiling to later nodes. A `require` guard proves nothing after it.
use super::execution::{host, limits};
use super::*;
use warp_core::edict_effects::EffectFrameError;
use warp_core::EchoOperationBudgetV1;

const BYTES: &str = "Bytes<max=64>";

fn int(value: u64) -> V {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", V::Integer(i128::from(value))),
            ]),
        ),
    ])
}
pub(super) fn declared(id: &str, ty: &str) -> V {
    map([
        ("id", text(id)),
        ("type", text(ty)),
        ("alphaName", text(&format!("${}", id.replace('.', "")))),
    ])
}
fn input_ref() -> V {
    map([
        ("id", text("arg.0")),
        ("type", text("app.Input")),
        ("alphaName", text("$arg0")),
    ])
}
pub(super) fn local(reference: &V) -> V {
    map([("kind", text("local")), ("ref", reference.clone())])
}
pub(super) fn input(name: &str) -> V {
    map([
        ("kind", text("field")),
        ("base", local(&input_ref())),
        ("field", text(name)),
    ])
}
fn call(callee: &str, types: &[&str], args: Vec<V>) -> V {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        (
            "typeArgs",
            V::Array(types.iter().map(|ty| text(ty)).collect()),
        ),
        ("args", V::Array(args)),
    ])
}
pub(super) fn len(bytes: V) -> V {
    call("core.bytes.length", &[BYTES], vec![bytes])
}
pub(super) fn prefix_is_utf8(bytes: &str, at: V) -> V {
    call(
        "core.bytes.isUtf8",
        &[BYTES],
        vec![call(
            "core.bytes.slice",
            &[BYTES],
            vec![input(bytes), int(0), at],
        )],
    )
}
fn add(left: V, right: V) -> V {
    call("core.integer.add", &["U64"], vec![left, right])
}
pub(super) fn at_most(left: V, right: V) -> V {
    map([
        ("kind", text("compare")),
        ("op", text("<=")),
        ("left", left),
        ("right", right),
    ])
}
fn is_true(value: V) -> V {
    map([
        ("kind", text("compare")),
        ("op", text("==")),
        ("left", value),
        (
            "right",
            map([
                ("kind", text("const")),
                (
                    "value",
                    map([("kind", text("bool")), ("value", V::Bool(true))]),
                ),
            ]),
        ),
    ])
}
fn all(values: Vec<V>) -> V {
    map([("kind", text("all")), ("values", V::Array(values))])
}

pub(super) enum Node {
    Let(V, V),
    Require(V),
}

/// A pure effect program over `{ text, other: Bytes<max=64>, at: U64 }`.
pub(super) fn package(constraints: Vec<V>, nodes: Vec<Node>, result: V) -> V {
    let mut core = compiled_core();
    set(
        &mut core,
        "types",
        map([
            (
                "Input",
                map([
                    ("kind", text("Record")),
                    (
                        "fields",
                        map([
                            ("at", text("U64")),
                            ("other", text(BYTES)),
                            ("text", text(BYTES)),
                        ]),
                    ),
                ]),
            ),
            ("Result", map([("kind", text("Bool"))])),
        ]),
    );
    let constraints = V::Array(
        constraints
            .into_iter()
            .enumerate()
            .map(|(index, predicate)| {
                map([
                    ("coordinate", text(&format!("app.update@1.where.{index}"))),
                    ("predicate", predicate),
                ])
            })
            .collect(),
    );
    let intent = value_field_mut(value_field_mut(&mut core, "intents"), "update@1");
    set(intent, "input", text("app.Input"));
    set(intent, "inputConstraints", constraints);
    set(
        value_field_mut(intent, "coreEvaluationBudget"),
        "maxSteps",
        V::Integer(10_000),
    );
    let mut locals = vec![input_ref()];
    let mut core_nodes = Vec::new();
    let mut bindings = Vec::new();
    let mut requirements = Vec::new();
    let mut order = Vec::new();
    for (index, node) in nodes.into_iter().enumerate() {
        match node {
            Node::Let(binding, value) => {
                locals.push(binding.clone());
                core_nodes.push(map([
                    ("kind", text("let")),
                    ("binding", binding.clone()),
                    ("value", value.clone()),
                ]));
                let id = format!("binding.{index}");
                bindings.push(map([
                    ("id", text(&id)),
                    ("binding", binding),
                    ("value", value),
                ]));
                order.push(text(&id));
            }
            Node::Require(predicate) => {
                let failure = map([
                    ("kind", text("terminal")),
                    (
                        "reason",
                        map([
                            ("reasonKind", text("app.BadOffset")),
                            ("payload", map([("at", input("at"))])),
                        ]),
                    ),
                ]);
                core_nodes.push(map([
                    ("kind", text("require")),
                    ("predicate", predicate.clone()),
                    ("onFailure", failure.clone()),
                ]));
                let id = format!("require.{index}");
                requirements.push(map([
                    ("id", text(&id)),
                    ("predicate", predicate),
                    ("onFailure", failure),
                ]));
                order.push(text(&id));
            }
        }
    }
    let body = core_body_mut(&mut core);
    set(body, "locals", V::Array(locals));
    set(body, "nodes", V::Array(core_nodes));
    set(body, "result", result);
    let mut target = compiled_target(&core);
    let selected = selected_target_mut(&mut target);
    set(selected, "pureBindings", V::Array(bindings));
    set(selected, "requirements", V::Array(requirements));
    set(selected, "executionOrder", V::Array(order));
    bound_compiler_package(&core, &target, &compiled_exports(), "update@1")
}

pub(super) fn ok_binding() -> V {
    declared("local.ok", "Bool")
}
pub(super) fn slice_program(constraints: Vec<V>, bytes: &str) -> V {
    package(
        constraints,
        vec![Node::Let(ok_binding(), prefix_is_utf8(bytes, input("at")))],
        local(&ok_binding()),
    )
}

#[test]
fn an_input_constraint_proves_a_runtime_prefix_slice() {
    prepare(&slice_program(
        vec![at_most(input("at"), len(input("text")))],
        "text",
    ))
    .expect("the constraint dominates the slice");
}

#[test]
fn a_runtime_slice_without_its_range_proof_is_refused() {
    refused(&slice_program(Vec::new(), "text"));
}

#[test]
fn a_constraint_on_another_operand_proves_nothing() {
    refused(&slice_program(
        vec![at_most(input("at"), len(input("other")))],
        "text",
    ));
}

#[test]
fn a_require_guard_does_not_prove_later_nodes() {
    // Edict's semantics: only input constraints and earlier conjuncts of the
    // same predicate dominate. A require refuses later work; it is no proof.
    refused(&package(
        Vec::new(),
        vec![
            Node::Require(at_most(input("at"), len(input("text")))),
            Node::Let(ok_binding(), prefix_is_utf8("text", input("at"))),
        ],
        local(&ok_binding()),
    ));
}

#[test]
fn a_let_carries_its_proven_ceiling_to_later_additions() {
    let count = declared("local.count", "U64");
    let total = declared("local.total", "U64");
    let program = |addend: u64| {
        package(
            Vec::new(),
            vec![
                Node::Let(
                    count.clone(),
                    call(
                        "core.bytes.count",
                        &[BYTES],
                        vec![
                            input("text"),
                            map([
                                ("kind", text("const")),
                                (
                                    "value",
                                    map([
                                        ("kind", text("bytes")),
                                        ("value", V::Bytes(vec![b'\n'])),
                                    ]),
                                ),
                            ]),
                        ],
                    ),
                ),
                Node::Let(total.clone(), add(local(&count), int(addend))),
                Node::Let(
                    ok_binding(),
                    is_true_value(at_most(local(&total), local(&total))),
                ),
            ],
            local(&ok_binding()),
        )
    };
    prepare(&program(u64::MAX - 64)).expect("the count is at most 64");
    refused(&program(u64::MAX - 63));
}

#[test]
fn an_input_constraint_bounds_a_let_and_its_later_additions() {
    let at = declared("local.at", "U64");
    let total = declared("local.total", "U64");
    let program = |addend: u64| {
        package(
            vec![at_most(input("at"), int(10))],
            vec![
                Node::Let(at.clone(), input("at")),
                Node::Let(total.clone(), add(local(&at), int(addend))),
                Node::Let(
                    ok_binding(),
                    is_true_value(at_most(local(&total), local(&total))),
                ),
            ],
            local(&ok_binding()),
        )
    };
    prepare(&program(u64::MAX - 10)).expect("at is at most 10");
    refused(&program(u64::MAX - 9));
}

/// A Bool-valued `if` of a predicate, so tests can bind a predicate result.
fn is_true_value(predicate: V) -> V {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        (
            "then",
            map([
                ("kind", text("const")),
                (
                    "value",
                    map([("kind", text("bool")), ("value", V::Bool(true))]),
                ),
            ]),
        ),
        (
            "else",
            map([
                ("kind", text("const")),
                (
                    "value",
                    map([("kind", text("bool")), ("value", V::Bool(false))]),
                ),
            ]),
        ),
    ])
}

/// Jim's character-boundary check: one conjunction whose first conjunct
/// proves the slice in the second.
fn boundary_program() -> V {
    package(
        Vec::new(),
        vec![Node::Require(all(vec![
            at_most(input("at"), len(input("text"))),
            is_true(prefix_is_utf8("text", input("at"))),
        ]))],
        map([
            ("kind", text("const")),
            (
                "value",
                map([("kind", text("bool")), ("value", V::Bool(true))]),
            ),
        ]),
    )
}

fn run(at: u64) -> Result<Vec<u8>, EffectFrameError> {
    let package = prepare(&boundary_program()).expect("the conjunct prefix proves the slice");
    let program = package.program().as_effect_program().expect("effect");
    let (host, head, _) = host();
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            warp_core::EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(10000, 4096, 0),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let request = encoded(&map([
        ("at", V::Integer(i128::from(at))),
        ("other", V::Bytes(Vec::new())),
        ("text", V::Bytes("hé".as_bytes().to_vec())),
    ]));
    frame
        .evaluate_effect(program, &request, &[])
        .map(<[u8]>::to_vec)
}

#[test]
fn a_valid_program_takes_its_authored_path_for_out_of_range_requests() {
    let accepted = encoded(&V::Bool(true));
    // "hé" is h, then é as two bytes: offsets 0, 1 and 3 are boundaries.
    for at in [0, 1, 3] {
        assert_eq!(run(at), Ok(accepted.clone()), "offset {at}");
    }
    // Inside é, past the end, and at the U64 maximum: the authored obstruction.
    for at in [2, 4, u64::MAX] {
        assert!(
            matches!(
                run(at),
                Err(EffectFrameError::AuthoredObstruction { ref reason_kind, .. })
                    if reason_kind == "app.BadOffset"
            ),
            "offset {at}"
        );
    }
}

#[test]
fn the_same_check_outside_its_conjunction_is_refused_before_it_runs() {
    refused(&package(
        Vec::new(),
        vec![Node::Require(is_true(prefix_is_utf8("text", input("at"))))],
        map([
            ("kind", text("const")),
            (
                "value",
                map([("kind", text("bool")), ("value", V::Bool(true))]),
            ),
        ]),
    ));
}

#[test]
fn an_equality_constraint_bounds_a_term_from_either_side() {
    let total = declared("local.total", "U64");
    let compare = |op: &str, left: V, right: V| {
        map([
            ("kind", text("compare")),
            ("op", text(op)),
            ("left", left),
            ("right", right),
        ])
    };
    let program = |constraint: V, addend: u64| {
        package(
            vec![constraint],
            vec![
                Node::Let(total.clone(), add(input("at"), int(addend))),
                Node::Let(
                    ok_binding(),
                    is_true_value(at_most(local(&total), local(&total))),
                ),
            ],
            local(&ok_binding()),
        )
    };
    // Echo's predicates compare with `==` and `<=` only; as in Edict, an
    // equality with a constant bounds the term on either side.
    for constraint in [
        compare("==", input("at"), int(10)),
        compare("==", int(10), input("at")),
    ] {
        prepare(&program(constraint.clone(), u64::MAX - 10)).expect("at is at most 10");
        refused(&program(constraint, u64::MAX - 9));
    }
}
