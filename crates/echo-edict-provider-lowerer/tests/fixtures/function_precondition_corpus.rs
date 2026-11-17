// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared corpus for source-function preconditions in Echo's three judgments.
//!
//! warp-core, the provider lowerer and the provider verifier each include this
//! file and must reach every case's verdict. Oracle: the verdict Amblicit's
//! Target lowering reaches for the same Core (Amblicit `1196396b`, the
//! "Function Preconditions" and "Core Source Function Shape" sections of
//! `docs/SPEC_edict-language-v1.md`). Each case is named after the test in
//! Amblicit's `function_preconditions.rs` or `function_precondition_forgery.rs`
//! that it translates, and the Core shapes are those that compiler emits:
//! `&&` lowers to left-nested binary `all`, a `Bool` call in predicate
//! position to `call == true`, and a `Bool` body to `if p then true else false`.
//!
//! Every case is one module `preconditions.example@1` with the intent
//! `evaluate(input: { bytes: Bytes<max=8>, start: U64, end: U64 })` that binds
//! `value` and returns `{ value }`, as Amblicit's own tests write it.
//!
//! Echo admits a subset of Amblicit's predicates (no `true`, `false` or call
//! predicates, ordered comparisons only between unsigned integers) and of its
//! intent bodies (only `let` nodes), so cases that need those are not here.
use echo_edict_canonical::CanonicalValueV1 as Value;

const MODULE: &str = "preconditions.example@1";
const BYTES: &str = "Bytes<max=8>";

/// One corpus entry: a Core module, its lawpack exports and the verdict all
/// three judgments must reach.
pub(crate) struct Case {
    pub(crate) name: String,
    pub(crate) core: Value,
    pub(crate) exports: Value,
    pub(crate) accepted: bool,
}

fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}
fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn local(id: &str, alpha: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(alpha)),
        ("type", text(ty)),
    ])
}
fn param(index: usize, ty: &str) -> Value {
    local(&format!("arg.{index}"), &format!("$arg{index}"), ty)
}
fn read(local: &Value) -> Value {
    map([("kind", text("local")), ("ref", local.clone())])
}
fn input_local() -> Value {
    local("arg.0", "$arg0", &format!("{MODULE}.Input"))
}
fn input_field(name: &str) -> Value {
    map([
        ("kind", text("field")),
        ("base", read(&input_local())),
        ("field", text(name)),
    ])
}
fn start() -> Value {
    input_field("start")
}
fn end() -> Value {
    input_field("end")
}
fn bytes() -> Value {
    input_field("bytes")
}
fn int(value: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(i128::from(value))),
            ]),
        ),
    ])
}
fn huge() -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(i128::from(u64::MAX) + 1)),
            ]),
        ),
    ])
}
fn boolean(value: bool) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("bool")), ("value", Value::Bool(value))]),
        ),
    ])
}
fn compare(op: &str, left: Value, right: Value) -> Value {
    map([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
fn le(left: Value, right: Value) -> Value {
    compare("<=", left, right)
}
/// `a op b op c` as Amblicit emits `&&` (`all`) and `||` (`any`): left-nested
/// binary nodes. One item is itself; no case passes none.
fn chain(kind: &str, items: Vec<Value>) -> Value {
    let binary = |left, right| {
        map([
            ("kind", text(kind)),
            ("values", Value::Array(vec![left, right])),
        ])
    };
    items
        .into_iter()
        .reduce(binary)
        .unwrap_or_else(|| map([("kind", text(kind)), ("values", Value::Array(Vec::new()))]))
}
fn and(items: Vec<Value>) -> Value {
    chain("all", items)
}
fn or(items: Vec<Value>) -> Value {
    chain("any", items)
}
fn any(values: Vec<Value>) -> Value {
    map([("kind", text("any")), ("values", Value::Array(values))])
}
fn not(value: Value) -> Value {
    map([("kind", text("not")), ("value", value)])
}
fn branch(predicate: Value, then: Value, otherwise: Value) -> Value {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", then),
        ("else", otherwise),
    ])
}
fn call(name: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(&format!("{MODULE}.{name}"))),
        ("typeArgs", Value::Array(Vec::new())),
        ("args", Value::Array(args)),
    ])
}
/// A `Bool` source call in predicate position, as Amblicit emits it.
fn holds(name: &str, args: Vec<Value>) -> Value {
    compare("==", call(name, args), boolean(true))
}
fn intrinsic(callee: &str, ty: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        ("typeArgs", Value::Array(vec![text(ty)])),
        ("args", Value::Array(args)),
    ])
}
fn subtract(left: Value, right: Value) -> Value {
    intrinsic("core.integer.subtract", "U64", vec![left, right])
}
fn len(value: Value) -> Value {
    intrinsic("core.bytes.length", BYTES, vec![value])
}
fn slice(value: Value, from: Value, to: Value) -> Value {
    intrinsic("core.bytes.slice", BYTES, vec![value, from, to])
}
fn fold(initial: Value, step: &str) -> Value {
    map([
        ("kind", text("fold")),
        ("initial", initial),
        ("step", text(&format!("{MODULE}.{step}"))),
        ("bound", Value::Integer(2)),
        ("stateType", text("U64")),
    ])
}

/// A source function. `preconditions` is omitted when empty, as Core does.
fn function(params: Vec<Value>, returns: &str, preconditions: Vec<Value>, result: Value) -> Value {
    let mut fields = vec![
        ("params", Value::Array(params)),
        ("returnType", text(returns)),
        (
            "body",
            map([
                ("locals", Value::Array(Vec::new())),
                ("bindings", Value::Array(Vec::new())),
                ("result", result),
            ]),
        ),
    ];
    if !preconditions.is_empty() {
        fields.push(("preconditions", Value::Array(preconditions)));
    }
    map(fields)
}
fn arg(index: usize) -> Value {
    read(&param(index, "U64"))
}
fn two(returns: &str, preconditions: Vec<Value>, result: Value) -> Value {
    function(
        vec![param(0, "U64"), param(1, "U64")],
        returns,
        preconditions,
        result,
    )
}
/// `fn span(start: U64, end: U64) -> U64 where start <= end { return end - start; }`.
fn span() -> Value {
    two("U64", vec![le(arg(0), arg(1))], subtract(arg(1), arg(0)))
}
/// `fn first(start: U64, end: U64) -> U64 where start <= end { return start; }`.
fn first() -> Value {
    two("U64", vec![le(arg(0), arg(1))], arg(0))
}
/// `fn first(start: U64, end: U64) -> U64 { return start; }`.
fn plain_first() -> Value {
    two("U64", Vec::new(), arg(0))
}
/// `fn ordered(start: U64, end: U64) -> Bool where start <= end
///  { return end - start <= 4u64; }`.
fn ordered() -> Value {
    two(
        "Bool",
        vec![le(arg(0), arg(1))],
        branch(
            le(subtract(arg(1), arg(0)), int(4)),
            boolean(true),
            boolean(false),
        ),
    )
}
/// `fn width(start: U64, end: U64) -> U64 [where start <= end]
///  { return span(start, end); }`, with `swapped` calling `span(end, start)`.
fn width(guarded: bool, swapped: bool) -> Value {
    let args = if swapped {
        vec![arg(1), arg(0)]
    } else {
        vec![arg(0), arg(1)]
    };
    let preconditions = if guarded {
        vec![le(arg(0), arg(1))]
    } else {
        Vec::new()
    };
    two("U64", preconditions, call("span", args))
}
/// `fn cut(bytes: Bytes<max=8>, start: U64, end: U64) -> Bytes<max=8>
///  where <preconditions> { return slice(bytes, start, end); }`.
fn cut(preconditions: impl Fn(Value, Value, Value) -> Vec<Value>) -> Value {
    let (b, s, e) = (param(0, BYTES), param(1, "U64"), param(2, "U64"));
    function(
        vec![b.clone(), s.clone(), e.clone()],
        BYTES,
        preconditions(read(&b), read(&s), read(&e)),
        slice(read(&b), read(&s), read(&e)),
    )
}
fn cut_two() -> Value {
    cut(|b, s, e| vec![le(s, e.clone()), le(e, len(b))])
}

/// The one-intent module of every case. `constraints` are the intent's
/// `where` predicates in order; `basis` is the optional basis expression.
fn module(
    functions: Vec<(&'static str, Value)>,
    constraints: Vec<Value>,
    basis: Option<Value>,
    value_type: &str,
    value: Value,
    capabilities: Vec<&str>,
) -> (Value, Value) {
    let value_local = local("local.0", "$local0", value_type);
    let constraints = constraints
        .into_iter()
        .enumerate()
        .map(|(index, predicate)| {
            map([
                ("coordinate", text(&format!("where.{index}"))),
                ("source", text("where")),
                ("predicate", predicate),
            ])
        })
        .collect();
    let mut intent = vec![
        ("input", text(&format!("{MODULE}.Input"))),
        ("output", text(&format!("{MODULE}.Output"))),
        (
            "requiredOperationProfile",
            text("continuum.profile.read-only/v1"),
        ),
        ("inputConstraints", Value::Array(constraints)),
        (
            "coreEvaluationBudget",
            map([
                ("maxSteps", Value::Integer(4096)),
                ("maxAllocatedBytes", Value::Integer(1 << 20)),
                ("maxOutputBytes", Value::Integer(1 << 20)),
            ]),
        ),
        (
            "body",
            map([
                (
                    "locals",
                    Value::Array(vec![input_local(), value_local.clone()]),
                ),
                (
                    "nodes",
                    Value::Array(vec![map([
                        ("kind", text("let")),
                        ("binding", value_local.clone()),
                        ("value", value),
                    ])]),
                ),
                (
                    "result",
                    map([
                        ("kind", text("record")),
                        ("fields", map([("value", read(&value_local))])),
                    ]),
                ),
            ]),
        ),
    ];
    if let Some(basis) = basis {
        intent.push(("basis", basis));
    }
    let types = map([
        (
            "Input",
            map([
                ("kind", text("Record")),
                (
                    "fields",
                    map([
                        ("bytes", text(BYTES)),
                        ("start", text("U64")),
                        ("end", text("U64")),
                    ]),
                ),
            ]),
        ),
        (
            "Output",
            map([
                ("kind", text("Record")),
                ("fields", map([("value", text(value_type))])),
            ]),
        ),
    ]);
    (
        map([
            ("coordinate", text(MODULE)),
            ("types", types),
            ("intents", map([("evaluate", map(intent))])),
            ("functions", map(functions)),
            (
                "requiredCoreCapabilities",
                Value::Array(capabilities.into_iter().map(text).collect()),
            ),
        ]),
        map([
            ("effects", Value::Array(Vec::new())),
            ("pureFunctions", Value::Array(Vec::new())),
        ]),
    )
}

struct Cases(Vec<Case>);
impl Cases {
    /// A case with a `U64` value and no basis.
    fn add(
        &mut self,
        name: impl Into<String>,
        accepted: bool,
        functions: Vec<(&'static str, Value)>,
        constraints: Vec<Value>,
        value: Value,
    ) {
        self.full(name, accepted, functions, constraints, None, "U64", value);
    }
    #[allow(clippy::too_many_arguments, reason = "one flat case constructor")]
    fn full(
        &mut self,
        name: impl Into<String>,
        accepted: bool,
        functions: Vec<(&'static str, Value)>,
        constraints: Vec<Value>,
        basis: Option<Value>,
        value_type: &str,
        value: Value,
    ) {
        let capabilities = if functions.iter().any(|(name, _)| *name == "step") {
            vec!["core.fold/v1"]
        } else {
            Vec::new()
        };
        let (core, exports) = module(
            functions,
            constraints,
            basis,
            value_type,
            value,
            capabilities,
        );
        self.0.push(Case {
            name: name.into(),
            core,
            exports,
            accepted,
        });
    }
}

/// Every case, in a fixed order.
#[allow(
    clippy::too_many_lines,
    reason = "One flat table keeps every case and its verdict side by side"
)]
pub(crate) fn cases() -> Vec<Case> {
    let mut cases = Cases(Vec::new());
    let ordered_input = || vec![le(start(), end())];
    let span_call = || call("span", vec![start(), end()]);
    let first_call = || call("first", vec![start(), end()]);

    // function_preconditions.rs
    {
        // Jedit's `function-precondition.amblicit`, with its three preconditions.
        let jedit = cut(|b, s, e| {
            vec![
                le(s.clone(), e.clone()),
                le(e, len(b.clone())),
                le(s, len(b)),
            ]
        });
        cases.full(
            "jedit_function_precondition_reproducer_compiles_and_lowers",
            true,
            vec![("cut", jedit)],
            vec![and(vec![
                le(start(), end()),
                le(end(), len(bytes())),
                le(start(), len(bytes())),
            ])],
            None,
            BYTES,
            call("cut", vec![bytes(), start(), end()]),
        );
    }
    cases.add(
        "preconditions_prove_subtraction_and_slice_in_the_body",
        true,
        vec![("cut", cut_two()), ("span", span())],
        vec![le(start(), end()), le(end(), len(bytes()))],
        span_call(),
    );
    cases.add(
        "bodies_without_preconditions_still_refuse_unproven_subtraction_and_slice: span",
        false,
        vec![("span", two("U64", Vec::new(), subtract(arg(1), arg(0))))],
        vec![le(start(), end()), le(end(), len(bytes()))],
        start(),
    );
    cases.add(
        "bodies_without_preconditions_still_refuse_unproven_subtraction_and_slice: cut",
        false,
        vec![("cut", cut(|_, _, _| Vec::new()))],
        vec![le(start(), end()), le(end(), len(bytes()))],
        start(),
    );
    for (label, constraints, value) in [
        ("no fact", Vec::new(), span_call()),
        ("a reversed fact", vec![le(end(), start())], span_call()),
        (
            "swapped arguments",
            ordered_input(),
            call("span", vec![end(), start()]),
        ),
        (
            "a non-order fact",
            vec![compare("!=", start(), end())],
            span_call(),
        ),
        (
            "a disjunction",
            vec![or(vec![le(start(), end()), le(end(), len(bytes()))])],
            span_call(),
        ),
    ] {
        cases.add(
            format!(
                "a_call_that_does_not_establish_a_precondition_is_refused_at_the_call: {label}"
            ),
            false,
            vec![("span", span())],
            constraints,
            value,
        );
    }
    cases.full(
        "every_conjunct_must_be_established",
        false,
        vec![("cut", cut_two())],
        ordered_input(),
        None,
        BYTES,
        call("cut", vec![bytes(), start(), end()]),
    );
    for (label, constraints, value) in [
        ("<=", ordered_input(), span_call()),
        (">=", vec![compare(">=", end(), start())], span_call()),
        ("<", vec![compare("<", start(), end())], span_call()),
        (
            "then arm",
            Vec::new(),
            branch(le(start(), end()), span_call(), int(0)),
        ),
        ("==", vec![compare("==", start(), end())], span_call()),
        (
            "same expression",
            Vec::new(),
            call("span", vec![start(), start()]),
        ),
        ("zero", Vec::new(), call("span", vec![int(0), end()])),
    ] {
        cases.add(
            format!("dominating_facts_from_every_source_establish_a_precondition: {label}"),
            true,
            vec![("span", span())],
            constraints,
            value,
        );
    }
    cases.add(
        "dominating_facts_from_every_source_establish_a_precondition: a caller's precondition",
        true,
        vec![("span", span()), ("width", width(true, false))],
        ordered_input(),
        call("width", vec![start(), end()]),
    );
    cases.add(
        "a_negated_guard_does_not_establish_a_precondition",
        false,
        vec![("span", span())],
        Vec::new(),
        branch(le(start(), end()), int(0), span_call()),
    );
    cases.add(
        "a_call_inside_an_intent_where_clause_sees_only_earlier_conjuncts: earlier",
        true,
        vec![("ordered", ordered())],
        vec![and(vec![
            le(start(), end()),
            holds("ordered", vec![start(), end()]),
        ])],
        start(),
    );
    cases.add(
        "a_call_inside_an_intent_where_clause_sees_only_earlier_conjuncts: later",
        false,
        vec![("ordered", ordered())],
        vec![and(vec![
            holds("ordered", vec![start(), end()]),
            le(start(), end()),
        ])],
        start(),
    );
    cases.add(
        "preconditions_are_never_evaluated_and_do_not_change_the_caller: without",
        true,
        vec![("first", plain_first())],
        ordered_input(),
        first_call(),
    );
    cases.add(
        "preconditions_are_never_evaluated_and_do_not_change_the_caller: with",
        true,
        vec![("first", first())],
        ordered_input(),
        first_call(),
    );
    cases.add(
        "preconditions_must_be_predicates_over_parameters_without_source_calls: not a predicate",
        false,
        vec![(
            "bad",
            function(vec![param(0, "U64")], "U64", vec![arg(0)], arg(0)),
        )],
        Vec::new(),
        start(),
    );
    cases.add(
        "preconditions_must_be_predicates_over_parameters_without_source_calls: a source call",
        false,
        vec![
            ("span", span()),
            (
                "bad",
                two(
                    "U64",
                    vec![le(call("span", vec![arg(0), arg(1)]), int(4))],
                    arg(0),
                ),
            ),
        ],
        Vec::new(),
        start(),
    );
    {
        let step = |preconditions| function(vec![param(0, "U64")], "U64", preconditions, arg(0));
        cases.add(
            "a_function_with_preconditions_cannot_be_a_fold_step: control",
            true,
            vec![("step", step(Vec::new()))],
            Vec::new(),
            fold(start(), "step"),
        );
        cases.add(
            "a_function_with_preconditions_cannot_be_a_fold_step",
            false,
            vec![("step", step(vec![le(arg(0), int(4))]))],
            Vec::new(),
            fold(start(), "step"),
        );
    }
    cases.add(
        "core_preconditions_are_hash_significant_and_omitted_when_empty: an empty list",
        false,
        vec![("first", {
            let Value::Map(mut fields) = plain_first() else {
                unreachable!("a function is a map")
            };
            fields.push((text("preconditions"), Value::Array(Vec::new())));
            Value::Map(fields)
        })],
        ordered_input(),
        first_call(),
    );
    let stranger = || read(&local("local.9", "$local9", "U64"));
    cases.add(
        "core_integrity_refuses_preconditions_outside_the_parameter_scope: a stranger",
        false,
        vec![(
            "span",
            two(
                "U64",
                vec![le(stranger(), arg(1))],
                subtract(arg(1), arg(0)),
            ),
        )],
        ordered_input(),
        span_call(),
    );
    cases.add(
        "core_integrity_refuses_preconditions_outside_the_parameter_scope: a self call",
        false,
        vec![(
            "span",
            two(
                "U64",
                vec![le(call("span", vec![arg(0), arg(1)]), arg(1))],
                subtract(arg(1), arg(0)),
            ),
        )],
        ordered_input(),
        span_call(),
    );
    cases.add(
        "target_refuses_forged_core_that_drops_a_precondition_the_body_needs",
        false,
        vec![("span", two("U64", Vec::new(), subtract(arg(1), arg(0))))],
        ordered_input(),
        span_call(),
    );
    cases.add(
        "target_refuses_forged_core_whose_call_does_not_establish_a_precondition: no constraint",
        false,
        vec![("first", first())],
        Vec::new(),
        first_call(),
    );
    cases.add(
        "target_refuses_forged_core_whose_call_does_not_establish_a_precondition: reversed",
        false,
        vec![("first", two("U64", vec![le(arg(1), arg(0))], arg(0)))],
        ordered_input(),
        first_call(),
    );
    {
        let pick_first = |precondition: Value| {
            function(
                vec![param(0, BYTES), param(1, "U64")],
                "U64",
                vec![precondition],
                read(&param(1, "U64")),
            )
        };
        let integers = le(read(&param(1, "U64")), read(&param(1, "U64")));
        let bytes_order = le(read(&param(0, BYTES)), read(&param(0, BYTES)));
        cases.add(
            "structured_target_refuses_a_non_integer_ordered_comparison_in_a_precondition: integers",
            true,
            vec![("first", pick_first(integers))],
            Vec::new(),
            call("first", vec![bytes(), start()]),
        );
        cases.add(
            "structured_target_refuses_a_non_integer_ordered_comparison_in_a_precondition: bytes",
            false,
            vec![("first", pick_first(bytes_order))],
            Vec::new(),
            call("first", vec![bytes(), start()]),
        );
    }
    {
        // 16 conjuncts: eight listed preconditions and one `&&` chain of eight.
        let capped = |count: u64| {
            let conjuncts: Vec<Value> = (1..=count)
                .map(|bound| compare("!=", arg(0), int(bound)))
                .collect();
            let (listed, chained) = conjuncts.split_at(conjuncts.len() / 2);
            let mut preconditions = listed.to_vec();
            preconditions.push(and(chained.to_vec()));
            function(vec![param(0, "U64")], "U64", preconditions, arg(0))
        };
        cases.add(
            "a_function_holds_at_most_sixteen_precondition_conjuncts: 16",
            true,
            vec![("first", capped(16))],
            Vec::new(),
            start(),
        );
        cases.add(
            "a_function_holds_at_most_sixteen_precondition_conjuncts: 17",
            false,
            vec![("first", capped(17))],
            Vec::new(),
            start(),
        );
        // An `any` of `n` comparisons `arg0 != k` is 1 + 3n Core nodes.
        let nodes = |count: u64| {
            let items = (1..=count)
                .map(|bound| compare("!=", arg(0), int(bound)))
                .collect();
            function(vec![param(0, "U64")], "U64", vec![any(items)], arg(0))
        };
        cases.add(
            "a_function_holds_at_most_256_precondition_nodes: 256",
            true,
            vec![("first", nodes(85))],
            Vec::new(),
            start(),
        );
        cases.add(
            "a_function_holds_at_most_256_precondition_nodes: 259",
            false,
            vec![("first", nodes(86))],
            Vec::new(),
            start(),
        );
    }
    {
        // The parameters swapped by name, every reference kept consistent.
        let renamed = function(
            vec![
                local("arg.1", "$arg1", "U64"),
                local("arg.0", "$arg0", "U64"),
            ],
            "U64",
            vec![le(arg(1), arg(0))],
            arg(1),
        );
        cases.add(
            "precondition_bearing_parameters_are_named_by_position",
            false,
            vec![("first", renamed)],
            ordered_input(),
            first_call(),
        );
    }

    // function_precondition_forgery.rs
    cases.add(
        "swapped_parameter_arguments_do_not_capture: control",
        true,
        vec![("span", span()), ("width", width(true, false))],
        ordered_input(),
        call("width", vec![start(), end()]),
    );
    cases.add(
        "swapped_parameter_arguments_do_not_capture",
        false,
        vec![("span", span()), ("width", width(true, true))],
        ordered_input(),
        call("width", vec![start(), end()]),
    );
    for (label, function, accepted) in [("plain", plain_first(), true), ("guarded", first(), false)]
    {
        cases.full(
            format!("a_call_in_the_basis_sees_no_where_facts: {label}"),
            accepted,
            vec![("first", function)],
            ordered_input(),
            Some(first_call()),
            "U64",
            first_call(),
        );
    }
    cases.add(
        "separate_input_constraints_do_not_justify_each_other",
        false,
        vec![("ordered", ordered())],
        vec![le(start(), end()), holds("ordered", vec![start(), end()])],
        start(),
    );
    cases.add(
        "a_disjunction_does_not_give_an_order_fact: forged",
        false,
        vec![("ordered", ordered())],
        vec![any(vec![
            le(start(), end()),
            holds("ordered", vec![start(), end()]),
        ])],
        start(),
    );
    cases.add(
        "a_disjunction_does_not_give_an_order_fact: a strict reversed left operand",
        false,
        vec![("ordered", ordered())],
        vec![or(vec![
            compare("<", end(), start()),
            holds("ordered", vec![start(), end()]),
        ])],
        start(),
    );
    cases.add(
        "a_call_moved_to_the_else_arm_is_refused",
        false,
        vec![("span", span())],
        Vec::new(),
        branch(le(start(), end()), int(0), span_call()),
    );
    for (op, flip, accepted) in [
        ("<=", false, true),
        ("<", false, true),
        ("==", false, true),
        ("==", true, true),
        (">=", true, true),
        (">", true, true),
        ("!=", false, false),
        (">=", false, false),
        (">", false, false),
        ("<=", true, false),
        ("<", true, false),
    ] {
        let (left, right) = if flip {
            (end(), start())
        } else {
            (start(), end())
        };
        cases.add(
            format!("only_order_implying_facts_prove_an_order: {op} flip={flip}"),
            accepted,
            vec![("first", first())],
            vec![compare(op, left, right)],
            first_call(),
        );
    }
    for (a, b, accepted) in [(3, 5, true), (5, 5, true), (5, 3, false)] {
        cases.add(
            format!("literal_arguments_follow_the_literal_order_rule: {a} <= {b}"),
            accepted,
            vec![("first", first())],
            Vec::new(),
            call("first", vec![int(a), int(b)]),
        );
    }
    cases.add(
        "literal_arguments_follow_the_literal_order_rule: above u64::MAX",
        false,
        vec![("first", first())],
        Vec::new(),
        call("first", vec![huge(), int(5)]),
    );
    cases.add(
        "dropping_a_calling_functions_precondition_is_refused",
        false,
        vec![("span", span()), ("width", width(false, false))],
        ordered_input(),
        call("width", vec![start(), end()]),
    );
    {
        let ordered_gap = le(arg(0), arg(1));
        let bounded_gap = le(subtract(arg(1), arg(0)), int(4));
        let gap = |preconditions| two("U64", preconditions, arg(0));
        let constraint = || {
            vec![and(vec![
                le(start(), end()),
                le(subtract(end(), start()), int(4)),
            ])]
        };
        let gap_call = || call("gap", vec![start(), end()]);
        for (label, preconditions, accepted) in [
            (
                "in order",
                vec![ordered_gap.clone(), bounded_gap.clone()],
                true,
            ),
            ("reversed", vec![bounded_gap.clone(), ordered_gap], false),
            ("first dropped", vec![bounded_gap], false),
        ] {
            cases.add(
                format!("each_precondition_is_total_only_given_the_ones_before_it: {label}"),
                accepted,
                vec![("gap", gap(preconditions))],
                constraint(),
                gap_call(),
            );
        }
    }
    {
        let step = || function(vec![param(0, "U64")], "U64", Vec::new(), arg(0));
        for (label, constraints, accepted) in [
            ("with the intent facts", ordered_input(), true),
            ("without them", Vec::new(), false),
        ] {
            cases.add(
                format!("a_call_in_a_fold_initial_value_needs_the_intent_facts: {label}"),
                accepted,
                vec![("span", span()), ("step", step())],
                constraints,
                fold(span_call(), "step"),
            );
        }
    }
    cases.add(
        "compiler_separate_where_predicates_do_not_see_each_other",
        false,
        vec![("ordered", ordered())],
        vec![le(start(), end()), holds("ordered", vec![start(), end()])],
        start(),
    );
    cases.add(
        "a_nested_conjunct_is_a_fact_at_any_depth",
        true,
        vec![("span", span())],
        vec![and(vec![
            le(end(), int(8)),
            le(start(), end()),
            le(start(), int(8)),
        ])],
        span_call(),
    );
    cases.add(
        "a_negated_left_operand_does_not_prove_the_right",
        false,
        vec![("ordered", ordered())],
        vec![and(vec![
            not(le(start(), end())),
            holds("ordered", vec![start(), end()]),
        ])],
        start(),
    );
    {
        let pick = |precondition: Value| two("U64", vec![precondition], arg(0));
        let ne = || pick(compare("!=", arg(0), arg(1)));
        let lt = || pick(compare("<", arg(0), arg(1)));
        let pick_call = || call("pick", vec![start(), end()]);
        for (label, function, constraints, accepted) in [
            ("!= same", ne(), vec![compare("!=", start(), end())], true),
            (
                "!= mirrored",
                ne(),
                vec![compare("!=", end(), start())],
                true,
            ),
            (
                "!= nested in &&",
                ne(),
                vec![and(vec![le(end(), int(4)), compare("!=", start(), end())])],
                true,
            ),
            ("!= from <", ne(), vec![compare("<", start(), end())], false),
            ("!= from nothing", ne(), Vec::new(), false),
            ("< same", lt(), vec![compare("<", start(), end())], true),
            ("< mirrored", lt(), vec![compare(">", end(), start())], true),
            (
                "< reversed",
                lt(),
                vec![compare("<", end(), start())],
                false,
            ),
            ("< from <=", lt(), ordered_input(), false),
            (
                "< from !=",
                lt(),
                vec![compare("!=", start(), end())],
                false,
            ),
        ] {
            cases.add(
                format!("non_order_preconditions_need_the_same_or_a_mirrored_fact: {label}"),
                accepted,
                vec![("pick", function)],
                constraints,
                pick_call(),
            );
        }
    }
    for (label, preconditions) in [
        (
            "two listed preconditions",
            vec![le(arg(0), arg(1)), le(arg(1), int(8))],
        ),
        (
            "one && precondition",
            vec![and(vec![le(arg(0), arg(1)), le(arg(1), int(8))])],
        ),
    ] {
        let function = || two("U64", preconditions.clone(), arg(0));
        cases.add(
            format!("a_partial_proof_is_refused: {label}: both facts"),
            true,
            vec![("first", function())],
            vec![and(vec![le(start(), end()), le(end(), int(8))])],
            first_call(),
        );
        for (index, fact) in [le(start(), end()), le(end(), int(8))]
            .into_iter()
            .enumerate()
        {
            cases.add(
                format!("a_partial_proof_is_refused: {label}: only fact {index}"),
                false,
                vec![("first", function())],
                vec![fact],
                first_call(),
            );
        }
    }
    {
        let not_eq = || two("U64", vec![not(compare("==", arg(0), arg(1)))], arg(0));
        let pick_call = || call("pick", vec![start(), end()]);
        cases.add(
            "an_else_arm_negation_proves_the_identical_negated_precondition",
            true,
            vec![("pick", not_eq())],
            Vec::new(),
            branch(compare("==", start(), end()), int(0), pick_call()),
        );
        cases.add(
            "a_later_disjunct_sees_the_negation_of_earlier_ones",
            true,
            vec![("pick", not_eq())],
            vec![or(vec![
                compare("==", start(), end()),
                compare("==", pick_call(), start()),
            ])],
            start(),
        );
        cases.add(
            "a_negation_is_not_mirrored",
            false,
            vec![("pick", not_eq())],
            Vec::new(),
            branch(compare("==", end(), start()), int(0), pick_call()),
        );
    }
    cases.0
}
