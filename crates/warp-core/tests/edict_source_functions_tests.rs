// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic runtime boundary tests. Rebound pins are deliberately test-owned;
//! these are not public compiler, provider verification or release witnesses.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod concat {
    use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
    use warp_core::edict_pure::EvaluationLimits;

    pub fn package() -> Vec<u8> {
        hex::decode(
            include_str!("fixtures/edict-byte-concat/executable-operation-package.cbor.hex").trim(),
        )
        .unwrap()
    }

    pub fn limits() -> EvaluationLimits {
        EvaluationLimits {
            max_package_bytes: 32_768,
            max_input_bytes: 2_097_152,
            max_steps: 10_000,
            max_allocated_bytes: 16_777_216,
            max_output_bytes: 2_097_152,
        }
    }

    pub fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
        Value::Map(
            fields
                .into_iter()
                .map(|(key, value)| (Value::Text(key.into()), value))
                .collect(),
        )
    }

    pub fn input(left: &[u8], right: &[u8]) -> Vec<u8> {
        encode(&record([
            ("firstFragment", Value::Bytes(left.into())),
            ("secondFragment", Value::Bytes(right.into())),
        ]))
        .unwrap()
    }

    pub fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
        let Value::Map(fields) = value else {
            panic!("map")
        };
        &fields
            .iter()
            .find(|(key, _)| key == &Value::Text(name.into()))
            .unwrap()
            .1
    }

    pub fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
        let Value::Map(fields) = value else {
            panic!("map")
        };
        &mut fields
            .iter_mut()
            .find(|(key, _)| key == &Value::Text(name.into()))
            .unwrap()
            .1
    }

    pub fn set_field(value: &mut Value, name: &str, replacement: Value) {
        *field_mut(value, name) = replacement;
    }
}

use concat::{field, field_mut, record, set_field};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::{evaluate, EvaluationError};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}
fn insert(value: &mut Value, key: &str, replacement: Value) {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    assert!(!entries.iter().any(|(name, _)| name == &text(key)));
    entries.push((text(key), replacement));
}
fn local(id: &str, ty: &str) -> Value {
    record([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(local: &Value) -> Value {
    record([("kind", text("local")), ("ref", local.clone())])
}
fn call(callee: &str, args: Vec<Value>) -> Value {
    record([
        ("kind", text("call")),
        ("callee", text(callee)),
        ("args", array(args)),
        ("typeArgs", array(vec![])),
    ])
}
fn function(
    params: Vec<Value>,
    ty: &str,
    locals: Vec<Value>,
    bindings: Vec<Value>,
    result: Value,
) -> Value {
    record([
        ("params", array(params)),
        ("returnType", text(ty)),
        (
            "body",
            record([
                ("locals", array(locals)),
                ("bindings", array(bindings)),
                ("result", result),
            ]),
        ),
    ])
}
fn embedded(program: &Value, name: &str) -> Value {
    let Value::Bytes(bytes) = field(program, name) else {
        panic!("artifact bytes")
    };
    decode(bytes).unwrap()
}
fn rewrite(
    raw: Vec<u8>,
    edit: impl FnOnce(&mut Value, &mut Value, &mut Value),
) -> (Vec<u8>, [u8; 32]) {
    let mut package = decode(&raw).unwrap();
    let mut program = embedded(&package, "program");
    let mut core = embedded(&program, "core_artifact");
    let mut target = embedded(&program, "target_ir_artifact");
    edit(&mut program, &mut core, &mut target);
    let core_digest = digest("edict.core.module/v1", &core).unwrap();
    set_field(
        field_mut(&mut package, "semantic_closure"),
        "canonical_meaning_identity",
        Value::Bytes(core_digest.into()),
    );
    let core_ref = record([
        ("id", field(&core, "coordinate").clone()),
        (
            "digest",
            array(vec![text("sha256"), Value::Bytes(core_digest.into())]),
        ),
    ]);
    set_field(
        field_mut(&mut target, "semanticClosure"),
        "sourceCore",
        core_ref,
    );
    for (key, closure, domain, value) in [
        (
            "core_artifact",
            "core_identity",
            "edict.core.module/v1",
            core,
        ),
        (
            "target_ir_artifact",
            "target_ir_identity",
            "edict.target-ir.artifact/v1",
            target,
        ),
    ] {
        set_field(&mut program, key, Value::Bytes(encode(&value).unwrap()));
        set_field(
            field_mut(&mut package, "semantic_closure"),
            closure,
            Value::Bytes(digest(domain, &value).unwrap().into()),
        );
    }
    set_field(
        &mut package,
        "program",
        Value::Bytes(encode(&program).unwrap()),
    );
    let pin = digest("echo.operation-package/v1", &package).unwrap();
    (encode(&package).unwrap(), pin)
}
fn factored_concat() -> (Vec<u8>, [u8; 32]) {
    rewrite(concat::package(), |_, core, target| {
        let coordinate = match field(core, "coordinate") {
            Value::Text(value) => value.clone(),
            _ => panic!("coordinate"),
        };
        let left = local("arg.0", "Bytes<max=8>");
        let right = local("arg.1", "Bytes<max=8>");
        let saved = local("local.0", "Bytes<max=16>");
        let mut intrinsic = call(
            "core.bytes.concat",
            vec![reference(&left), reference(&right)],
        );
        set_field(
            &mut intrinsic,
            "typeArgs",
            array(vec![text("Bytes<max=8>"), text("Bytes<max=8>")]),
        );
        let join = function(
            vec![left.clone(), right.clone()],
            "Bytes<max=16>",
            vec![],
            vec![],
            intrinsic,
        );
        let binding = record([
            ("kind", text("let")),
            ("binding", saved.clone()),
            (
                "value",
                call(
                    &format!("{coordinate}.join"),
                    vec![reference(&left), reference(&right)],
                ),
            ),
        ]);
        let assemble = function(
            vec![left, right],
            "Bytes<max=16>",
            vec![saved.clone()],
            vec![binding],
            reference(&saved),
        );
        insert(
            core,
            "functions",
            record([("assemble", assemble), ("join", join)]),
        );
        for (artifact, bindings) in [(core, "nodes"), (target, "pureBindings")] {
            let intent = field_mut(field_mut(artifact, "intents"), "assembleRange");
            let body = if bindings == "nodes" {
                field_mut(intent, "body")
            } else {
                intent
            };
            let Value::Array(nodes) = field_mut(body, bindings) else {
                panic!("bindings")
            };
            let args = match field(field(&nodes[0], "value"), "args") {
                Value::Array(args) => args.clone(),
                _ => panic!("args"),
            };
            set_field(
                &mut nodes[0],
                "value",
                call(&format!("{coordinate}.assemble"), args),
            );
        }
    })
}

#[test]
fn source_functions_nested_forward_calls_and_bindings_use_fresh_parameter_frames() {
    let (package, pin) = factored_concat();
    for (left, right) in [
        (b"".as_slice(), b"".as_slice()),
        (b"ab", b"cd"),
        (b"cd", b"ab"),
        (&[0, 255], &[128, 127]),
    ] {
        let result =
            evaluate(&package, pin, &concat::input(left, right), concat::limits()).unwrap();
        let expected = [left, right].concat();
        assert_eq!(
            result.output,
            encode(&record([("bytes", Value::Bytes(expected))])).unwrap()
        );
    }
}

#[test]
fn source_functions_nested_frames_share_the_same_cumulative_meter() {
    let (package, pin) = factored_concat();
    let supplied = concat::input(b"ab", b"cde");
    let result = evaluate(&package, pin, &supplied, concat::limits()).unwrap();
    let mut exact = concat::limits();
    exact.max_steps = result.steps;
    exact.max_allocated_bytes = result.allocated_bytes;
    assert_eq!(evaluate(&package, pin, &supplied, exact).unwrap(), result);
    let mut short = exact;
    short.max_steps -= 1;
    assert_eq!(
        evaluate(&package, pin, &supplied, short),
        Err(EvaluationError::StepBudgetExceeded)
    );
    short = exact;
    short.max_allocated_bytes -= 1;
    assert_eq!(
        evaluate(&package, pin, &supplied, short),
        Err(EvaluationError::AllocationBudgetExceeded)
    );
}

#[test]
fn source_functions_unused_malformed_definition_is_not_silently_ignored() {
    let (package, pin) = rewrite(concat::package(), |_, core, _| {
        insert(core, "functions", record([("unused", Value::Null)]));
    });
    assert!(
        evaluate(&package, pin, &concat::input(b"a", b"b"), concat::limits()).is_err(),
        "runtime ignored authenticated malformed source authority"
    );
}

#[test]
fn source_functions_bounded_read_passes_a_real_retained_atom_to_a_helper() {
    use warp_core::edict_read::{evaluate as read, ReadBasis, ReadLimits, ReadView};
    use warp_core::{
        make_node_id, make_type_id, AtomPayload, AttachmentValue, GraphStore, NodeKey, NodeRecord,
        WorldlineFrontier, WorldlineId, WorldlineState,
    };
    let original = hex::decode(
        include_str!("fixtures/edict-node-read/single-executable-operation-package.hex").trim(),
    )
    .unwrap();
    let (package, pin) = rewrite(original, |program, core, target| {
        let coordinate = match field(core, "coordinate") {
            Value::Text(value) => value.clone(),
            _ => panic!("coordinate"),
        };
        let output_type = match field(field(field(core, "intents"), "replaceRange"), "output") {
            Value::Text(value) => value.clone(),
            _ => panic!("output"),
        };
        let parameter = local("arg.0", &output_type);
        insert(
            core,
            "functions",
            record([(
                "retain",
                function(
                    vec![parameter.clone()],
                    &output_type,
                    vec![],
                    vec![],
                    reference(&parameter),
                ),
            )]),
        );
        let saved = local("source.return", &output_type);
        let body = field_mut(
            field_mut(field_mut(core, "intents"), "replaceRange"),
            "body",
        );
        let helper = call(
            &format!("{coordinate}.retain"),
            vec![field(body, "result").clone()],
        );
        let Value::Array(locals) = field_mut(body, "locals") else {
            panic!("locals")
        };
        locals.push(saved.clone());
        let Value::Array(nodes) = field_mut(body, "nodes") else {
            panic!("nodes")
        };
        nodes.push(record([
            ("kind", text("let")),
            ("binding", saved.clone()),
            ("value", helper.clone()),
        ]));
        set_field(body, "result", reference(&saved));
        let intent = field_mut(field_mut(target, "intents"), "replaceRange");
        let binding = record([
            ("id", text("replaceRange.binding.source")),
            ("binding", saved.clone()),
            ("value", helper),
        ]);
        if let Value::Map(entries) = intent {
            if let Some((_, Value::Array(bindings))) = entries
                .iter_mut()
                .find(|(key, _)| key == &text("pureBindings"))
            {
                bindings.push(binding);
            } else {
                entries.push((text("pureBindings"), array(vec![binding])));
            }
        }
        let Value::Array(order) = field_mut(intent, "executionOrder") else {
            panic!("order")
        };
        order.push(text("replaceRange.binding.source"));
        set_field(intent, "result", reference(&saved));
        let mut projection = embedded(program, "result_projection_artifact");
        set_field(
            &mut projection,
            "expression",
            record([
                ("kind", text("source")),
                (
                    "source",
                    record([
                        ("kind", text("pureBinding")),
                        ("bindingId", text("replaceRange.binding.source")),
                    ]),
                ),
                ("path", array(vec![])),
            ]),
        );
        set_field(
            program,
            "result_projection_artifact",
            Value::Bytes(encode(&projection).unwrap()),
        );
    });
    let mut store = GraphStore::default();
    let node = make_node_id("source-function-read");
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id("node"),
        },
    );
    store.set_node_attachment(
        node,
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: make_type_id("document"),
            bytes: b"alpha".to_vec().into(),
        })),
    );
    let key = NodeKey {
        warp_id: store.warp_id(),
        local_id: node,
    };
    let state = WorldlineState::from_root_store(store, node).unwrap();
    let frontier = WorldlineFrontier::new(WorldlineId::from_bytes([7; 32]), state);
    let before = frontier.state().state_root();
    let basis = ReadBasis::at(&frontier);
    let aperture = [key];
    let view = ReadView::new(&frontier, basis, &aperture).unwrap();
    let supplied = encode(&record([
        (
            "address",
            record([
                ("warpId", Value::Bytes(key.warp_id.0.to_vec())),
                ("nodeId", Value::Bytes(key.local_id.0.to_vec())),
                ("typeId", Value::Bytes(make_type_id("document").0.to_vec())),
            ]),
        ),
        ("expected", Value::Bytes(b"alpha".to_vec())),
    ]))
    .unwrap();
    let limits = ReadLimits {
        evaluation: concat::limits(),
        max_reads: 1,
        max_read_bytes: 5,
    };
    let result = read(&package, pin, &supplied, &view, limits).unwrap();
    assert_eq!(
        decode(&result.output).unwrap(),
        Value::Bytes(b"alpha".to_vec())
    );
    assert_eq!((result.reads, result.read_bytes), (1, 5));
    assert_eq!(result.basis, basis);
    assert_eq!(frontier.state().state_root(), before);
}

#[test]
fn source_functions_complete_graph_depth_counts_every_shared_suffix_in_both_orders() {
    for count in [64, 65] {
        for shallow_first in [false, true] {
            let (package, pin) = rewrite(concat::package(), |_, core, _| {
                let Value::Text(coordinate) = field(core, "coordinate") else {
                    panic!("coordinate")
                };
                let coordinate = coordinate.clone();
                let name = |index: usize| {
                    format!(
                        "f{:03}",
                        if shallow_first {
                            count - 1 - index
                        } else {
                            index
                        }
                    )
                };
                let mut functions = Vec::new();
                for index in 0..count {
                    let result = if index + 1 == count {
                        record([
                            ("kind", text("const")),
                            (
                                "value",
                                record([
                                    ("kind", text("int")),
                                    ("width", text("U64")),
                                    ("value", Value::Integer(7)),
                                ]),
                            ),
                        ])
                    } else {
                        call(&format!("{coordinate}.{}", name(index + 1)), vec![])
                    };
                    functions.push((
                        text(&name(index)),
                        function(vec![], "U64", vec![], vec![], result),
                    ));
                }
                insert(core, "functions", Value::Map(functions));
            });
            let result = evaluate(&package, pin, &concat::input(b"a", b"b"), concat::limits());
            if count == 64 {
                assert!(
                    result.is_ok(),
                    "exact depth, shallow first {shallow_first}: {result:?}"
                );
            } else {
                assert_eq!(
                    result,
                    Err(EvaluationError::UnsupportedProgram),
                    "over depth, shallow first {shallow_first}"
                );
            }
        }
    }
}

#[test]
fn source_functions_unused_definition_cannot_collide_with_an_intent() {
    let (package, pin) = rewrite(concat::package(), |_, core, _| {
        let result = record([
            ("kind", text("const")),
            (
                "value",
                record([
                    ("kind", text("int")),
                    ("width", text("U64")),
                    ("value", Value::Integer(7)),
                ]),
            ),
        ]);
        insert(
            core,
            "functions",
            record([(
                "assembleRange",
                function(vec![], "U64", vec![], vec![], result),
            )]),
        );
    });
    assert!(
        evaluate(&package, pin, &concat::input(b"a", b"b"), concat::limits()).is_err(),
        "a source function and intent cannot share one module member"
    );
}

// The expected counters below are derived from the public fixed-cell meter:
// input materialization S = 347 + L + R; each field selection copies S;
// repeat(N) costs 18 + 2*N steps and S + 192 + 4*N storage. The unused
// second parameter must still pay for its argument exactly once.
fn metered_arguments(reverse: bool, lazy: Option<bool>) -> (Vec<u8>, [u8; 32]) {
    rewrite(concat::package(), |_, core, target| {
        let Value::Text(coordinate) = field(core, "coordinate") else {
            panic!("coordinate")
        };
        let coordinate = coordinate.clone();
        let value = local("arg.0", "Bytes<max=8>");
        let left = local("arg.0", "Bytes<max=16>");
        let right = local("arg.1", "Bytes<max=16>");
        let mut duplicate = call(
            "core.bytes.concat",
            vec![reference(&value), reference(&value)],
        );
        set_field(
            &mut duplicate,
            "typeArgs",
            array(vec![text("Bytes<max=8>"), text("Bytes<max=8>")]),
        );
        insert(
            core,
            "functions",
            record([
                (
                    "repeat",
                    function(vec![value], "Bytes<max=16>", vec![], vec![], duplicate),
                ),
                (
                    "first",
                    function(
                        vec![left.clone(), right],
                        "Bytes<max=16>",
                        vec![],
                        vec![],
                        reference(&left),
                    ),
                ),
            ]),
        );
        for (artifact, bindings) in [(core, "nodes"), (target, "pureBindings")] {
            let intent = field_mut(field_mut(artifact, "intents"), "assembleRange");
            let body = if bindings == "nodes" {
                field_mut(intent, "body")
            } else {
                intent
            };
            let Value::Array(nodes) = field_mut(body, bindings) else {
                panic!("bindings")
            };
            let Value::Array(original) = field(field(&nodes[0], "value"), "args") else {
                panic!("args")
            };
            let repeat = |argument| call(&format!("{coordinate}.repeat"), vec![argument]);
            let first = match lazy {
                None => repeat(original[0].clone()),
                Some(expensive_else) => {
                    let predicate = record([
                        ("kind", text("compare")),
                        ("op", text("==")),
                        ("left", original[0].clone()),
                        ("right", original[1].clone()),
                    ]);
                    let cheap = original[0].clone();
                    let costly = repeat(cheap.clone());
                    let (yes, no) = if expensive_else {
                        (cheap, costly)
                    } else {
                        (costly, cheap)
                    };
                    record([
                        ("kind", text("if")),
                        ("predicate", predicate),
                        ("then", yes),
                        ("else", no),
                    ])
                }
            };
            let mut args = vec![first, repeat(original[1].clone())];
            if reverse {
                args.reverse();
            }
            set_field(
                &mut nodes[0],
                "value",
                call(&format!("{coordinate}.first"), args),
            );
        }
    })
}

#[test]
fn source_functions_evaluate_used_and_unused_arguments_once_with_fixed_cost_oracles() {
    let (package, pin) = metered_arguments(false, None);
    for (left, right) in [
        (b"".as_slice(), b"".as_slice()),
        (b"ab", b"cde"),
        (b"12345678", b"12345678"),
    ] {
        let (l, r) = (left.len() as u64, right.len() as u64);
        let mut exact = concat::limits();
        exact.max_steps = 59 + 2 * (l + r);
        exact.max_allocated_bytes = 1883 + 13 * l + 7 * r;
        let input = concat::input(left, right);
        let result =
            evaluate(&package, pin, &input, exact).expect("independently calculated exact budget");
        assert_eq!(
            (result.steps, result.allocated_bytes),
            (exact.max_steps, exact.max_allocated_bytes)
        );
        assert_eq!(
            decode(&result.output).unwrap(),
            record([("bytes", Value::Bytes([left, left].concat()))])
        );
        let mut short = exact;
        short.max_steps -= 1;
        assert_eq!(
            evaluate(&package, pin, &input, short),
            Err(EvaluationError::StepBudgetExceeded)
        );
        short = exact;
        short.max_allocated_bytes -= 1;
        assert_eq!(
            evaluate(&package, pin, &input, short),
            Err(EvaluationError::AllocationBudgetExceeded)
        );
    }
}

#[test]
fn source_functions_argument_order_is_observable_through_distinct_budget_refusals() {
    let mut limits = concat::limits();
    limits.max_steps = 30;
    limits.max_allocated_bytes = 890;
    let input = concat::input(b"12345678", b"");
    let (package, pin) = metered_arguments(false, None);
    assert_eq!(
        evaluate(&package, pin, &input, limits),
        Err(EvaluationError::StepBudgetExceeded)
    );
    let (package, pin) = metered_arguments(true, None);
    assert_eq!(
        evaluate(&package, pin, &input, limits),
        Err(EvaluationError::AllocationBudgetExceeded)
    );
}

#[test]
fn source_functions_only_the_selected_valid_branch_spends_execution_budget() {
    let input = concat::input(b"12345678", b"");
    for expensive_else in [false, true] {
        let (package, pin) = metered_arguments(false, Some(expensive_else));
        let mut exact = concat::limits();
        exact.max_steps = if expensive_else { 99 } else { 72 };
        exact.max_allocated_bytes = if expensive_else { 2697 } else { 2449 };
        let result = evaluate(&package, pin, &input, exact).expect("selected branch exact budget");
        assert_eq!(
            (result.steps, result.allocated_bytes),
            (exact.max_steps, exact.max_allocated_bytes)
        );
        let expected = if expensive_else {
            b"1234567812345678".as_slice()
        } else {
            b"12345678".as_slice()
        };
        assert_eq!(
            decode(&result.output).unwrap(),
            record([("bytes", Value::Bytes(expected.to_vec()))])
        );
    }
}

#[test]
fn source_functions_executed_call_depth_includes_surrounding_predicate_frames() {
    for count in [62, 63] {
        for shallow_first in [false, true] {
            let (package, pin) = rewrite(concat::package(), |_, core, target| {
                let Value::Text(coordinate) = field(core, "coordinate") else {
                    panic!("coordinate")
                };
                let coordinate = coordinate.clone();
                let name = |index: usize| {
                    format!(
                        "f{:03}",
                        if shallow_first {
                            count - 1 - index
                        } else {
                            index
                        }
                    )
                };
                let one = record([
                    ("kind", text("const")),
                    (
                        "value",
                        record([
                            ("kind", text("int")),
                            ("width", text("U64")),
                            ("value", Value::Integer(1)),
                        ]),
                    ),
                ]);
                let functions = (0..count)
                    .map(|index| {
                        let result = if index + 1 == count {
                            one.clone()
                        } else {
                            call(&format!("{coordinate}.{}", name(index + 1)), vec![])
                        };
                        (
                            text(&name(index)),
                            function(vec![], "U64", vec![], vec![], result),
                        )
                    })
                    .collect();
                insert(core, "functions", Value::Map(functions));
                for (artifact, key) in [(core, "nodes"), (target, "pureBindings")] {
                    let intent = field_mut(field_mut(artifact, "intents"), "assembleRange");
                    let body = if key == "nodes" {
                        field_mut(intent, "body")
                    } else {
                        intent
                    };
                    let Value::Array(nodes) = field_mut(body, key) else {
                        panic!("nodes")
                    };
                    let original = field(&nodes[0], "value").clone();
                    let predicate = record([
                        ("kind", text("compare")),
                        ("op", text("==")),
                        ("left", call(&format!("{coordinate}.{}", name(0)), vec![])),
                        ("right", one.clone()),
                    ]);
                    set_field(
                        &mut nodes[0],
                        "value",
                        record([
                            ("kind", text("if")),
                            ("predicate", predicate),
                            ("then", original.clone()),
                            ("else", original),
                        ]),
                    );
                }
            });
            let result = evaluate(&package, pin, &concat::input(b"ab", b"c"), concat::limits());
            if count == 62 {
                let result = result.expect(
                    "62 function frames plus two surrounding predicate frames execute at 64",
                );
                assert_eq!(
                    decode(&result.output).unwrap(),
                    record([("bytes", Value::Bytes(b"abc".to_vec()))])
                );
            } else {
                assert_eq!(
                    result,
                    Err(EvaluationError::UnsupportedProgram),
                    "65 total frames, shallow first {shallow_first}"
                );
            }
        }
    }
}

#[test]
fn source_functions_pure_input_uses_arg_zero_identity_after_declaration_reordering() {
    let (original, _) = factored_concat();
    let (package, pin) = rewrite(original, |_, core, _| {
        let body = field_mut(
            field_mut(field_mut(core, "intents"), "assembleRange"),
            "body",
        );
        let Value::Array(locals) = field_mut(body, "locals") else {
            panic!("locals")
        };
        assert!(locals.len() > 1);
        locals.reverse();
    });
    let result = evaluate(
        &package,
        pin,
        &concat::input(b"ab", b"cd"),
        concat::limits(),
    )
    .expect("provider-admitted declaration inventory order");
    assert_eq!(
        decode(&result.output).unwrap(),
        record([("bytes", Value::Bytes(b"abcd".to_vec()))])
    );
}

#[test]
fn source_functions_pure_basis_is_admitted_but_does_not_spend_execution_budget() {
    let (baseline, baseline_pin) = factored_concat();
    let input = concat::input(b"ab", b"cd");
    let expected = evaluate(&baseline, baseline_pin, &input, concat::limits()).unwrap();
    for case in ["none", "valid", "null", "unknown", "arity", "capture"] {
        let (package, pin) = rewrite(baseline.clone(), |_, core, target| {
            let Value::Text(coordinate) = field(core, "coordinate") else {
                panic!("coordinate")
            };
            let coordinate = coordinate.clone();
            let one = record([
                ("kind", text("const")),
                (
                    "value",
                    record([
                        ("kind", text("int")),
                        ("width", text("U64")),
                        ("value", Value::Integer(1)),
                    ]),
                ),
            ]);
            insert(
                field_mut(core, "functions"),
                "basisValue",
                function(vec![], "U64", vec![], vec![], one.clone()),
            );
            let expression = match case {
                "none" | "null" => Value::Null,
                "valid" => call(&format!("{coordinate}.basisValue"), vec![]),
                "unknown" => call(&format!("{coordinate}.missing"), vec![]),
                "arity" => call(&format!("{coordinate}.basisValue"), vec![one]),
                "capture" => reference(&local("local.0", "Bytes<max=16>")),
                _ => panic!("case"),
            };
            if case != "none" {
                for artifact in [core, target] {
                    insert(
                        field_mut(field_mut(artifact, "intents"), "assembleRange"),
                        "basis",
                        expression.clone(),
                    );
                }
            }
        });
        let result = evaluate(&package, pin, &input, concat::limits());
        if ["none", "valid"].contains(&case) {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.is_err(), "invalid basis {case} was ignored");
        }
    }
}
