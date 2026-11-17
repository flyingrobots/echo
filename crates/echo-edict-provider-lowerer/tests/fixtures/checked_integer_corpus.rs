// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared corpus for Amblicit's checked integer calls and `min`/`max` in
//! Echo's three judgments.
//!
//! warp-core, the provider lowerer and the provider verifier each include this
//! file and must reach every case's verdict; warp-core also executes every
//! accepted case's `main` helper on each listed run. Oracle: Amblicit
//! `7dbcbc23` (which contains #321), the "Integer Safety" section and prelude
//! table of `docs/SPEC_edict-language-v1.md`, and native Target's
//! `target_ir/integer_operations.rs`: `core.integer.checked{Add,Sub,Mul,Div,Rem}`
//! and `core.integer.{min,max}` take exactly one width type argument `U8`,
//! `U32` or `U64` and two operands of exactly that width. A checked call
//! returns `Option<W>` and needs `core.option/v1`; overflow, underflow and a
//! zero divisor are its `none` case. `min` and `max` return `W` and need no
//! capability. Division truncates; the remainder takes the dividend's sign.
//!
//! Every case is one module `integers.example@1` whose intent
//! `evaluate(input: { value: U64 })` returns `{ value: main(input.value) }`.
use echo_edict_canonical::CanonicalValueV1 as Value;

const MODULE: &str = "integers.example@1";
const CAPABILITY: &str = "core.option/v1";

/// One corpus entry: a Core module, its lawpack exports, the verdict all three
/// judgments must reach and, for an accepted case, `main(input) == output`.
pub(crate) struct Case {
    pub(crate) name: String,
    pub(crate) core: Value,
    pub(crate) exports: Value,
    pub(crate) accepted: bool,
    pub(crate) runs: Vec<(u64, Value)>,
}

/// The coordinate of the helper every case's intent calls.
pub(crate) fn main_coordinate() -> String {
    format!("{MODULE}.main")
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
fn named(name: &str) -> String {
    format!("{MODULE}.{name}")
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(&format!("${id}"))),
        ("type", text(ty)),
    ])
}
/// A parameter as Amblicit spells it: id `arg.N`, alpha name `$argN`.
fn param(index: usize, ty: &str) -> Value {
    map([
        ("id", text(&format!("arg.{index}"))),
        ("alphaName", text(&format!("$arg{index}"))),
        ("type", text(ty)),
    ])
}
fn read(local: &Value) -> Value {
    map([("kind", text("local")), ("ref", local.clone())])
}
fn arg(index: usize, ty: &str) -> Value {
    read(&param(index, ty))
}
fn int(width: &str, value: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text(width)),
                ("value", Value::Integer(i128::from(value))),
            ]),
        ),
    ])
}
fn u64v(value: u64) -> Value {
    int("U64", value)
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
fn field(base: Value, name: &str) -> Value {
    map([
        ("kind", text("field")),
        ("base", base),
        ("field", text(name)),
    ])
}
fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    map([("kind", text("record")), ("fields", map(fields))])
}
fn call(name: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(&named(name))),
        ("typeArgs", Value::Array(Vec::new())),
        ("args", Value::Array(args)),
    ])
}
/// `core.option.<op><item>(args)`, the compiler's Option encoding.
fn option(op: &str, item: &str, args: Vec<Value>) -> Value {
    typed(&format!("core.option.{op}"), vec![item], args)
}
fn typed(callee: &str, type_args: Vec<&str>, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        (
            "typeArgs",
            Value::Array(type_args.into_iter().map(text).collect()),
        ),
        ("args", Value::Array(args)),
    ])
}
fn default(item: &str, value: Value, fallback: Value) -> Value {
    option("default", item, vec![value, fallback])
}

/// A source function with `bindings` as `(local, value)` pairs.
fn function_with(
    params: Vec<Value>,
    returns: &str,
    bindings: Vec<(Value, Value)>,
    result: Value,
) -> Value {
    let locals = bindings.iter().map(|(local, _)| local.clone()).collect();
    let bindings = bindings
        .into_iter()
        .map(|(binding, value)| {
            map([
                ("kind", text("let")),
                ("binding", binding),
                ("value", value),
            ])
        })
        .collect();
    map([
        ("params", Value::Array(params)),
        ("returnType", text(returns)),
        (
            "body",
            map([
                ("locals", Value::Array(locals)),
                ("bindings", Value::Array(bindings)),
                ("result", result),
            ]),
        ),
    ])
}
fn function(params: Vec<Value>, returns: &str, result: Value) -> Value {
    function_with(params, returns, Vec::new(), result)
}
/// `fn main(v: U64) -> <returns> { return <result>; }`.
fn main_fn(returns: &str, result: Value) -> (&'static str, Value) {
    ("main", function(vec![param(0, "U64")], returns, result))
}
fn v() -> Value {
    arg(0, "U64")
}

fn types(extra: Vec<(&'static str, Value)>, value_type: &str) -> Value {
    let mut entries = vec![
        (
            "Input",
            map([
                ("kind", text("Record")),
                ("fields", map([("value", text("U64"))])),
            ]),
        ),
        (
            "Output",
            map([
                ("kind", text("Record")),
                ("fields", map([("value", text(value_type))])),
            ]),
        ),
    ];
    entries.extend(extra);
    map(entries)
}

/// The one-intent module of every case.
fn module(
    functions: Vec<(&'static str, Value)>,
    extra_types: Vec<(&'static str, Value)>,
    value_type: &str,
    capabilities: Vec<&str>,
) -> (Value, Value) {
    let input = local("arg.0", &named("Input"));
    let value_local = local("local.0", value_type);
    let intent = map([
        ("input", text(&named("Input"))),
        ("output", text(&named("Output"))),
        (
            "requiredOperationProfile",
            text("continuum.profile.read-only/v1"),
        ),
        ("inputConstraints", Value::Array(Vec::new())),
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
                    Value::Array(vec![input.clone(), value_local.clone()]),
                ),
                (
                    "nodes",
                    Value::Array(vec![map([
                        ("kind", text("let")),
                        ("binding", value_local.clone()),
                        ("value", call("main", vec![field(read(&input), "value")])),
                    ])]),
                ),
                ("result", record([("value", read(&value_local))])),
            ]),
        ),
    ]);
    (
        map([
            ("coordinate", text(MODULE)),
            ("types", types(extra_types, value_type)),
            ("intents", map([("evaluate", intent)])),
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
    fn push(
        &mut self,
        name: &str,
        runs: Option<Vec<(u64, u64)>>,
        functions: Vec<(&'static str, Value)>,
        value_type: &str,
        capabilities: Vec<&str>,
    ) {
        let (core, exports) = module(functions, Vec::new(), value_type, capabilities);
        self.0.push(Case {
            name: name.to_owned(),
            core,
            exports,
            accepted: runs.is_some(),
            runs: runs
                .unwrap_or_default()
                .into_iter()
                .map(|(input, output)| (input, Value::Integer(i128::from(output))))
                .collect(),
        });
    }
    fn accept(&mut self, name: &str, width: &str, runs: &[(u64, u64)], result: Value) {
        self.push(
            name,
            Some(runs.to_vec()),
            vec![main_fn(width, result)],
            width,
            vec![CAPABILITY],
        );
    }
    fn refuse(&mut self, name: &str, width: &str, result: Value) {
        self.push(
            name,
            None,
            vec![main_fn(width, result)],
            width,
            vec![CAPABILITY],
        );
    }
}

const SENTINEL: u64 = 42;
fn integer(callee: &str, width: &str, left: Value, right: Value) -> Value {
    typed(
        &format!("core.integer.{callee}"),
        vec![width],
        vec![left, right],
    )
}
/// `default(core.integer.<op><W>(a, b), 42)`: the value when present.
fn checked_or_sentinel(op: &str, width: &str, left: Value, right: Value) -> Value {
    default(width, integer(op, width, left, right), int(width, SENTINEL))
}
const fn maximum(width: &str) -> u64 {
    match width.as_bytes() {
        b"U8" => u8::MAX as u64,
        b"U32" => u32::MAX as u64,
        _ => u64::MAX,
    }
}

/// Every case, in a fixed order.
#[allow(
    clippy::too_many_lines,
    reason = "One flat table keeps every case and its verdict side by side"
)]
pub(crate) fn cases() -> Vec<Case> {
    let mut cases = Cases(Vec::new());

    // Each checked form at each width's bounds: (op, left, right, Some(result)).
    for width in ["U8", "U32", "U64"] {
        let top = maximum(width);
        let table: [(&str, u64, u64, Option<u64>); 14] = [
            ("checkedAdd", top, 0, Some(top)),
            ("checkedAdd", top, 1, None),
            ("checkedAdd", top - 1, 1, Some(top)),
            ("checkedSub", 5, 5, Some(0)),
            ("checkedSub", 0, 1, None),
            ("checkedSub", top, top, Some(0)),
            ("checkedMul", top, 1, Some(top)),
            ("checkedMul", top, 2, None),
            ("checkedMul", 0, top, Some(0)),
            ("checkedDiv", 7, 2, Some(3)),
            ("checkedDiv", top, 0, None),
            ("checkedRem", 7, 3, Some(1)),
            ("checkedRem", 7, 0, None),
            ("checkedDiv", top, top, Some(1)),
        ];
        for (op, left, right, expected) in table {
            cases.accept(
                &format!("{op}_{width}_{left}_{right}"),
                width,
                &[(0, expected.unwrap_or(SENTINEL))],
                checked_or_sentinel(op, width, int(width, left), int(width, right)),
            );
        }
        for (op, expected) in [("min", 0), ("max", top)] {
            cases.accept(
                &format!("{op}_{width}_at_the_bounds"),
                width,
                &[(0, expected)],
                integer(op, width, int(width, top), int(width, 0)),
            );
        }
    }

    // Operands that depend on the input.
    cases.accept(
        "checked_add_of_the_input",
        "U64",
        &[(5, 6), (u64::MAX, SENTINEL), (u64::MAX - 1, u64::MAX)],
        checked_or_sentinel("checkedAdd", "U64", v(), u64v(1)),
    );
    cases.accept(
        "checked_sub_of_the_input",
        "U64",
        &[(5, 2), (2, SENTINEL), (3, 0)],
        checked_or_sentinel("checkedSub", "U64", v(), u64v(3)),
    );
    cases.accept(
        "checked_div_by_the_input",
        "U64",
        &[(0, SENTINEL), (4, 25), (u64::MAX, 0)],
        checked_or_sentinel("checkedDiv", "U64", u64v(100), v()),
    );
    cases.accept(
        "checked_rem_by_the_input",
        "U64",
        &[(0, SENTINEL), (7, 2)],
        checked_or_sentinel("checkedRem", "U64", u64v(100), v()),
    );
    cases.accept(
        "min_and_max_of_the_input",
        "U64",
        &[(3, 3), (u64::MAX, 10)],
        integer("min", "U64", v(), u64v(10)),
    );

    // `min` keeps its upper bound for a later unchecked addition; `max` does not.
    cases.accept(
        "min_bounds_a_later_addition",
        "U64",
        &[(3, 4), (u64::MAX, 11)],
        typed(
            "core.integer.add",
            vec!["U64"],
            vec![integer("min", "U64", v(), u64v(10)), u64v(1)],
        ),
    );
    cases.refuse(
        "max_does_not_bound_a_later_addition",
        "U64",
        typed(
            "core.integer.add",
            vec!["U64"],
            vec![integer("max", "U64", v(), u64v(10)), u64v(1)],
        ),
    );

    // Authority: checked forms need core.option/v1; min and max do not.
    cases.push(
        "min_needs_no_capability",
        Some(vec![(3, 3)]),
        vec![main_fn("U64", integer("min", "U64", v(), u64v(10)))],
        "U64",
        Vec::new(),
    );
    cases.push(
        "a_checked_call_without_the_option_capability",
        None,
        vec![main_fn(
            "U64",
            checked_or_sentinel("checkedAdd", "U64", v(), u64v(1)),
        )],
        "U64",
        Vec::new(),
    );

    // Forged names, widths, operand types, arities and result uses.
    for (name, result) in [
        (
            "an_unknown_checked_name",
            checked_or_sentinel("checkedPow", "U64", v(), u64v(1)),
        ),
        (
            "an_unknown_saturating_name",
            integer("saturatingAdd", "U64", v(), u64v(1)),
        ),
        (
            "an_unknown_minimum_name",
            integer("minimum", "U64", v(), u64v(1)),
        ),
        (
            "a_u16_width",
            default(
                "U16",
                integer("checkedAdd", "U16", int("U16", 1), int("U16", 1)),
                int("U16", 0),
            ),
        ),
        (
            "a_bool_width",
            integer("min", "Bool", boolean(true), boolean(false)),
        ),
        (
            "an_operand_of_another_width",
            checked_or_sentinel("checkedAdd", "U64", int("U32", 1), u64v(1)),
        ),
        (
            "a_min_whose_operands_are_wider_than_its_width",
            integer("min", "U8", v(), u64v(1)),
        ),
        (
            "a_checked_call_with_one_operand",
            default(
                "U64",
                typed("core.integer.checkedAdd", vec!["U64"], vec![v()]),
                u64v(0),
            ),
        ),
        (
            "a_max_with_three_operands",
            typed("core.integer.max", vec!["U64"], vec![v(), v(), v()]),
        ),
        (
            "a_min_without_a_type_argument",
            typed("core.integer.min", Vec::new(), vec![v(), v()]),
        ),
        (
            "a_min_with_two_type_arguments",
            typed("core.integer.min", vec!["U64", "U64"], vec![v(), v()]),
        ),
        (
            "a_checked_result_used_as_its_width",
            integer("checkedAdd", "U64", v(), u64v(1)),
        ),
        (
            "a_min_result_used_as_an_option",
            default("U64", integer("min", "U64", v(), u64v(1)), u64v(0)),
        ),
        (
            "a_byte_operand",
            checked_or_sentinel(
                "checkedAdd",
                "U64",
                map([
                    ("kind", text("const")),
                    (
                        "value",
                        map([("kind", text("bytes")), ("value", Value::Bytes(vec![1]))]),
                    ),
                ]),
                u64v(1),
            ),
        ),
    ] {
        cases.refuse(name, "U64", result);
    }
    // An unchecked sum whose operand bounds come from separate `&&`
    // conjuncts, as Amblicit emits `a <= 10 && b <= 10` (a binary `all`).
    let le = |left: Value, right: Value| {
        map([
            ("kind", text("compare")),
            ("op", text("<=")),
            ("left", left),
            ("right", right),
        ])
    };
    let sum = |a: Value, b: Value| typed("core.integer.add", vec!["U64"], vec![a, b]);
    let both = map([
        ("kind", text("all")),
        (
            "values",
            Value::Array(vec![
                le(arg(0, "U64"), u64v(10)),
                le(arg(1, "U64"), u64v(10)),
            ]),
        ),
    ]);
    let guarded = map([
        ("kind", text("if")),
        ("predicate", both),
        ("then", sum(arg(0, "U64"), arg(1, "U64"))),
        ("else", u64v(0)),
    ]);
    cases.push(
        "a_sum_bounded_by_two_conjuncts",
        Some(vec![(3, 6), (11, 0)]),
        vec![
            (
                "pair",
                function(vec![param(0, "U64"), param(1, "U64")], "U64", guarded),
            ),
            main_fn("U64", call("pair", vec![v(), v()])),
        ],
        "U64",
        vec![CAPABILITY],
    );
    {
        let both = || {
            map([
                ("kind", text("all")),
                (
                    "values",
                    Value::Array(vec![
                        le(arg(0, "U64"), u64v(10)),
                        le(arg(1, "U64"), u64v(10)),
                    ]),
                ),
            ])
        };
        let mut pair = function(
            vec![param(0, "U64"), param(1, "U64")],
            "U64",
            sum(arg(0, "U64"), arg(1, "U64")),
        );
        if let Value::Map(fields) = &mut pair {
            fields.push((text("preconditions"), Value::Array(vec![both()])));
        }
        let guarded_main = map([
            ("kind", text("if")),
            (
                "predicate",
                map([
                    ("kind", text("all")),
                    (
                        "values",
                        Value::Array(vec![le(v(), u64v(10)), le(v(), u64v(10))]),
                    ),
                ]),
            ),
            ("then", call("pair", vec![v(), v()])),
            ("else", u64v(0)),
        ]);
        cases.push(
            "a_precondition_sum_bounded_by_two_conjuncts",
            Some(vec![(3, 6), (11, 0)]),
            vec![("pair", pair), main_fn("U64", guarded_main)],
            "U64",
            vec![CAPABILITY],
        );
    }
    // Upper bounds from strict and mirrored comparisons (#1010), as native
    // Target derives them: `x < n` bounds x by n - 1 (0 when n = 0, a branch
    // that never runs), and `n > x`, `n >= x` bound x like `x < n`, `x <= n`.
    {
        let cmp = |op: &str, left: Value, right: Value| {
            map([
                ("kind", text("compare")),
                ("op", text(op)),
                ("left", left),
                ("right", right),
            ])
        };
        let both =
            |a: Value, b: Value| map([("kind", text("all")), ("values", Value::Array(vec![a, b]))]);
        let guarded = |width: &str, predicate: Value| {
            map([
                ("kind", text("if")),
                ("predicate", predicate),
                (
                    "then",
                    typed(
                        "core.integer.add",
                        vec![width],
                        vec![arg(0, width), arg(1, width)],
                    ),
                ),
                ("else", int(width, 0)),
            ])
        };
        let a = || arg(0, "U64");
        let b = || arg(1, "U64");
        let strict = [
            (
                "a_strict_bound_inside_a_conjunction",
                both(cmp("<=", a(), u64v(224)), cmp("<", b(), u64v(24))),
                vec![(3, 6), (30, 0)],
            ),
            (
                "two_strict_bounds",
                both(cmp("<", a(), u64v(225)), cmp("<", b(), u64v(24))),
                vec![(3, 6), (30, 0)],
            ),
            (
                "mirrored_strict_and_non_strict_bounds",
                both(cmp(">=", u64v(224), a()), cmp(">", u64v(24), b())),
                vec![(3, 6), (30, 0)],
            ),
            (
                "a_strict_bound_of_zero_guards_a_branch_that_never_runs",
                both(cmp("<", b(), u64v(0)), cmp("<", b(), u64v(0))),
                vec![(3, 0), (u64::MAX, 0)],
            ),
        ];
        for (name, predicate, runs) in strict {
            cases.push(
                name,
                Some(runs),
                vec![
                    (
                        "pair",
                        function(
                            vec![param(0, "U64"), param(1, "U64")],
                            "U64",
                            guarded("U64", predicate),
                        ),
                    ),
                    main_fn("U64", call("pair", vec![v(), v()])),
                ],
                "U64",
                vec![CAPABILITY],
            );
        }
        // At the U32 maximum: a < MAX and b < 2 fit; a < MAX and b < 3 do not.
        let top = u64::from(u32::MAX);
        for (name, limit, accepted) in [
            ("strict_bounds_that_fit_the_width_exactly", 2, true),
            ("strict_bounds_one_past_the_width", 3, false),
        ] {
            let predicate = both(
                cmp("<", arg(0, "U32"), int("U32", top)),
                cmp("<", arg(1, "U32"), int("U32", limit)),
            );
            cases.push(
                name,
                accepted.then(|| vec![(0, 0)]),
                vec![
                    (
                        "pair",
                        function(
                            vec![param(0, "U32"), param(1, "U32")],
                            "U32",
                            guarded("U32", predicate),
                        ),
                    ),
                    main_fn("U32", call("pair", vec![int("U32", top), int("U32", 1)])),
                ],
                "U32",
                vec![CAPABILITY],
            );
        }
    }
    cases.0
}
