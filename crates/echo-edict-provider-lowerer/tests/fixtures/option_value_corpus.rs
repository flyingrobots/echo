// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared corpus for Amblicit `Option<T>` values in Echo's three judgments.
//!
//! warp-core, the provider lowerer and the provider verifier each include this
//! file and must reach every case's verdict; warp-core also executes every
//! accepted case's `main` helper on each listed run. Oracle: Amblicit `b9a6e734`
//! (which contains #316), the "Option Refinement" and "Standard Functions"
//! sections of `docs/SPEC_edict-language-v1.md`, and native Target's
//! `target_ir/option_operations.rs`. The Core shapes are those the compiler
//! emits: `core.option.<op>` calls with the item coordinate as the only type
//! argument under `core.option/v1`, an `isSome` test in predicate position as
//! `call == true`, and a `Bool` body as `if p then true else false`.
//!
//! `unwrap` is legal only on a local that a dominating fact proves present:
//! the predicate of an enclosing `then` arm, the negation of an enclosing
//! `else` arm's predicate, an earlier `all` conjunct, or the negation of an
//! earlier `any` disjunct. Amblicit's compiler refuses `match` on an Option, so
//! every Option `match` here is forged and refused.
//!
//! Every case is one module `options.example@1` whose intent
//! `evaluate(input: { value: U64 })` returns `{ value: main(input.value) }`.
//! Echo admits Options only inside helpers: an Option-valued intent result,
//! which Amblicit also does not lower, is refused.
use echo_edict_canonical::CanonicalValueV1 as Value;

const MODULE: &str = "options.example@1";
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
fn param(index: usize, ty: &str) -> Value {
    local(&format!("arg.{index}"), ty)
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
fn compare(op: &str, left: Value, right: Value) -> Value {
    map([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
fn not(value: Value) -> Value {
    map([("kind", text("not")), ("value", value)])
}
fn all(values: Vec<Value>) -> Value {
    map([("kind", text("all")), ("values", Value::Array(values))])
}
fn any(values: Vec<Value>) -> Value {
    map([("kind", text("any")), ("values", Value::Array(values))])
}
fn branch(predicate: Value, then: Value, otherwise: Value) -> Value {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", then),
        ("else", otherwise),
    ])
}
/// A `Bool` result as Amblicit emits it: `if p then true else false`.
fn truth(predicate: Value) -> Value {
    branch(predicate, boolean(true), boolean(false))
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
fn some(item: &str, value: Value) -> Value {
    option("some", item, vec![value])
}
fn none(item: &str) -> Value {
    option("none", item, Vec::new())
}
fn default(item: &str, value: Value, fallback: Value) -> Value {
    option("default", item, vec![value, fallback])
}
fn unwrap(item: &str, value: Value) -> Value {
    option("unwrap", item, vec![value])
}
/// `isSome(value)` in predicate position: `isSome(value) == true`.
fn present(item: &str, value: Value) -> Value {
    compare("==", option("isSome", item, vec![value]), boolean(true))
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
const OPT: &str = "Option<U64>";
/// `fn main(v: U64) -> <returns> { return <result>; }`.
fn main_fn(returns: &str, result: Value) -> (&'static str, Value) {
    ("main", function(vec![param(0, "U64")], returns, result))
}
fn v() -> Value {
    arg(0, "U64")
}
fn o() -> Value {
    arg(0, OPT)
}
/// `fn wrap(v: U64) -> Option<U64> { return some<U64>(v); }`.
fn wrap() -> (&'static str, Value) {
    (
        "wrap",
        function(vec![param(0, "U64")], OPT, some("U64", v())),
    )
}
/// `fn nothing() -> Option<U64> { return none<U64>(); }`.
fn nothing() -> (&'static str, Value) {
    ("nothing", function(Vec::new(), OPT, none("U64")))
}
/// `fn maybe(v: U64) -> Option<U64> { return if v > 3u64 then some(v) else none(); }`.
fn maybe() -> (&'static str, Value) {
    (
        "maybe",
        function(
            vec![param(0, "U64")],
            OPT,
            branch(compare(">", v(), u64v(3)), some("U64", v()), none("U64")),
        ),
    )
}
/// `fn <name>(o: Option<U64>) -> <returns> { return <result>; }`.
fn on_option(name: &'static str, returns: &str, result: Value) -> (&'static str, Value) {
    (name, function(vec![param(0, OPT)], returns, result))
}
/// `main(v) = <name>(maybe(v))`.
fn main_through(name: &str, returns: &str) -> (&'static str, Value) {
    main_fn(returns, call(name, vec![call("maybe", vec![v()])]))
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
fn record_type(fields: Vec<(&'static str, &str)>) -> Value {
    map([
        ("kind", text("Record")),
        (
            "fields",
            map(fields.into_iter().map(|(name, ty)| (name, text(ty)))),
        ),
    ])
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
        runs: Option<Vec<(u64, Value)>>,
        functions: Vec<(&'static str, Value)>,
        extra_types: Vec<(&'static str, Value)>,
        value_type: &str,
        capabilities: Vec<&str>,
    ) {
        let (core, exports) = module(functions, extra_types, value_type, capabilities);
        self.0.push(Case {
            name: name.to_owned(),
            core,
            exports,
            accepted: runs.is_some(),
            runs: runs.unwrap_or_default(),
        });
    }
    /// An accepted `U64`-valued case and its runs.
    fn accept(&mut self, name: &str, runs: &[(u64, u64)], functions: Vec<(&'static str, Value)>) {
        let runs = runs
            .iter()
            .map(|(input, output)| (*input, Value::Integer(i128::from(*output))))
            .collect();
        self.push(
            name,
            Some(runs),
            functions,
            Vec::new(),
            "U64",
            vec![CAPABILITY],
        );
    }
    /// An accepted `Bool`-valued case and its runs.
    fn accept_bool(
        &mut self,
        name: &str,
        runs: &[(u64, bool)],
        functions: Vec<(&'static str, Value)>,
    ) {
        let runs = runs
            .iter()
            .map(|(input, output)| (*input, Value::Bool(*output)))
            .collect();
        self.push(
            name,
            Some(runs),
            functions,
            Vec::new(),
            "Bool",
            vec![CAPABILITY],
        );
    }
    /// A refused `U64`-valued case.
    fn refuse(&mut self, name: &str, functions: Vec<(&'static str, Value)>) {
        self.push(name, None, functions, Vec::new(), "U64", vec![CAPABILITY]);
    }
}

/// `fn guarded(o) -> U64 { return if isSome(o) then unwrap(o) else 0u64; }`.
fn guarded() -> (&'static str, Value) {
    on_option(
        "guarded",
        "U64",
        branch(present("U64", o()), unwrap("U64", o()), u64v(0)),
    )
}

/// Every case, in a fixed order.
#[allow(
    clippy::too_many_lines,
    reason = "One flat table keeps every case and its verdict side by side"
)]
pub(crate) fn cases() -> Vec<Case> {
    let mut cases = Cases(Vec::new());

    // Construction and `default`.
    cases.accept(
        "some_then_default_returns_the_value",
        &[(5, 5), (0, 0)],
        vec![
            wrap(),
            main_fn("U64", default("U64", call("wrap", vec![v()]), u64v(9))),
        ],
    );
    cases.accept(
        "none_then_default_returns_the_fallback",
        &[(5, 7)],
        vec![
            nothing(),
            main_fn("U64", default("U64", call("nothing", Vec::new()), u64v(7))),
        ],
    );
    cases.accept(
        "a_conditional_option_takes_none_from_its_typed_peer",
        &[(5, 5), (2, 1)],
        vec![
            maybe(),
            main_fn("U64", default("U64", call("maybe", vec![v()]), u64v(1))),
        ],
    );

    // `isSome` refinement of `unwrap`.
    cases.accept(
        "unwrap_in_the_then_arm_of_is_some",
        &[(5, 5), (2, 0)],
        vec![maybe(), guarded(), main_through("guarded", "U64")],
    );
    cases.accept(
        "unwrap_in_the_else_arm_of_a_negated_is_some",
        &[(5, 5), (1, 0)],
        vec![
            maybe(),
            on_option(
                "pick",
                "U64",
                branch(not(present("U64", o())), u64v(0), unwrap("U64", o())),
            ),
            main_through("pick", "U64"),
        ],
    );
    cases.accept_bool(
        "unwrap_in_a_later_conjunct_of_is_some",
        &[(5, true), (2, false)],
        vec![
            maybe(),
            on_option(
                "large",
                "Bool",
                truth(all(vec![
                    present("U64", o()),
                    compare(">", unwrap("U64", o()), u64v(4)),
                ])),
            ),
            main_through("large", "Bool"),
        ],
    );
    cases.accept_bool(
        "unwrap_in_a_later_disjunct_after_a_negated_is_some",
        &[(5, true), (1, true)],
        vec![
            maybe(),
            on_option(
                "absent_or_large",
                "Bool",
                truth(any(vec![
                    not(present("U64", o())),
                    compare(">", unwrap("U64", o()), u64v(4)),
                ])),
            ),
            main_through("absent_or_large", "Bool"),
        ],
    );
    cases.accept_bool(
        "is_some_as_a_value",
        &[(5, true), (1, false)],
        vec![
            maybe(),
            on_option("has", "Bool", option("isSome", "U64", vec![o()])),
            main_through("has", "Bool"),
        ],
    );
    {
        let held = local("local.1", OPT);
        cases.accept(
            "unwrap_of_a_let_bound_local",
            &[(5, 5), (2, 0)],
            vec![
                maybe(),
                (
                    "main",
                    function_with(
                        vec![param(0, "U64")],
                        "U64",
                        vec![(held.clone(), call("maybe", vec![v()]))],
                        branch(
                            present("U64", read(&held)),
                            unwrap("U64", read(&held)),
                            u64v(0),
                        ),
                    ),
                ),
            ],
        );
    }

    // Options inside records, of records, of Options and behind a name.
    {
        let pair = named("Pair");
        let both = (
            "both",
            function(
                vec![param(0, "U64")],
                &pair,
                record([("left", some("U64", v())), ("right", none("U64"))]),
            ),
        );
        let main = main_fn(
            "U64",
            default(
                "U64",
                field(call("both", vec![v()]), "left"),
                default("U64", field(call("both", vec![v()]), "right"), u64v(9)),
            ),
        );
        cases.push(
            "option_record_fields",
            Some(vec![(5, Value::Integer(5))]),
            vec![both, main],
            vec![("Pair", record_type(vec![("left", OPT), ("right", OPT)]))],
            "U64",
            vec![CAPABILITY],
        );
    }
    {
        let visit = named("Visit");
        let visit_of = |value: Value| record([("lo", value.clone()), ("hi", value)]);
        let main = main_fn(
            "U64",
            field(
                default(&visit, call("visit", vec![v()]), visit_of(u64v(0))),
                "hi",
            ),
        );
        let maker = (
            "visit",
            function(
                vec![param(0, "U64")],
                &format!("Option<{visit}>"),
                some(&visit, visit_of(v())),
            ),
        );
        cases.push(
            "an_option_of_a_named_record",
            Some(vec![(5, Value::Integer(5))]),
            vec![maker, main],
            vec![("Visit", record_type(vec![("lo", "U64"), ("hi", "U64")]))],
            "U64",
            vec![CAPABILITY],
        );
    }
    cases.accept(
        "a_nested_option",
        &[(5, 5), (1, 0)],
        vec![
            maybe(),
            (
                "inner",
                function(
                    vec![param(0, "U64")],
                    "Option<Option<U64>>",
                    some(OPT, call("maybe", vec![v()])),
                ),
            ),
            main_fn(
                "U64",
                default(
                    "U64",
                    default(OPT, call("inner", vec![v()]), none("U64")),
                    u64v(0),
                ),
            ),
        ],
    );
    {
        let maybe_count = named("MaybeCount");
        cases.push(
            "an_option_named_in_the_type_table",
            Some(vec![(5, Value::Integer(5))]),
            vec![
                (
                    "wrap",
                    function(vec![param(0, "U64")], &maybe_count, some("U64", v())),
                ),
                main_fn("U64", default("U64", call("wrap", vec![v()]), u64v(0))),
            ],
            vec![(
                "MaybeCount",
                map([("kind", text("Option")), ("item", text("U64"))]),
            )],
            "U64",
            vec![CAPABILITY],
        );
    }

    // Authority.
    cases.push(
        "an_option_call_without_the_capability",
        None,
        vec![
            wrap(),
            main_fn("U64", default("U64", call("wrap", vec![v()]), u64v(9))),
        ],
        Vec::new(),
        "U64",
        Vec::new(),
    );
    cases.push(
        "an_option_valued_intent_result",
        None,
        vec![wrap(), main_fn(OPT, call("wrap", vec![v()]))],
        Vec::new(),
        OPT,
        vec![CAPABILITY],
    );

    // Unproven presence.
    let unwrapping = |name: &str, result: Value| {
        (
            name.to_owned(),
            vec![
                maybe(),
                on_option("pick", "U64", result),
                main_through("pick", "U64"),
            ],
        )
    };
    let unproven = vec![
        unwrapping("unwrap_without_a_fact", unwrap("U64", o())),
        unwrapping(
            "unwrap_in_the_else_arm_of_is_some",
            branch(present("U64", o()), u64v(0), unwrap("U64", o())),
        ),
        unwrapping(
            "unwrap_in_the_then_arm_of_a_negated_is_some",
            branch(not(present("U64", o())), unwrap("U64", o()), u64v(0)),
        ),
        unwrapping(
            "unwrap_under_is_some_equal_false",
            branch(
                compare("==", option("isSome", "U64", vec![o()]), boolean(false)),
                unwrap("U64", o()),
                u64v(0),
            ),
        ),
        unwrapping(
            "unwrap_after_a_disjunction",
            branch(
                any(vec![present("U64", o()), compare(">", u64v(1), u64v(0))]),
                unwrap("U64", o()),
                u64v(0),
            ),
        ),
        unwrapping(
            "unwrap_in_an_earlier_conjunct",
            branch(
                all(vec![
                    compare(">", unwrap("U64", o()), u64v(4)),
                    present("U64", o()),
                ]),
                u64v(1),
                u64v(0),
            ),
        ),
    ];
    for (name, functions) in unproven {
        cases.refuse(&name, functions);
    }
    {
        let p = arg(1, OPT);
        cases.refuse(
            "unwrap_of_a_different_local",
            vec![
                maybe(),
                (
                    "pick",
                    function(
                        vec![param(0, OPT), param(1, OPT)],
                        "U64",
                        branch(present("U64", p), unwrap("U64", o()), u64v(0)),
                    ),
                ),
                main_fn(
                    "U64",
                    call(
                        "pick",
                        vec![call("maybe", vec![v()]), call("maybe", vec![v()])],
                    ),
                ),
            ],
        );
    }
    {
        let alias = local("local.1", OPT);
        cases.refuse(
            "unwrap_through_an_alias",
            vec![
                maybe(),
                (
                    "pick",
                    function_with(
                        vec![param(0, OPT)],
                        "U64",
                        vec![(alias.clone(), o())],
                        branch(present("U64", o()), unwrap("U64", read(&alias)), u64v(0)),
                    ),
                ),
                main_through("pick", "U64"),
            ],
        );
    }
    {
        let boxed = named("Box");
        let inner = || field(arg(0, &boxed), "inner");
        cases.push(
            "unwrap_of_a_field_path",
            None,
            vec![
                maybe(),
                (
                    "pick",
                    function(
                        vec![param(0, &boxed)],
                        "U64",
                        branch(present("U64", inner()), unwrap("U64", inner()), u64v(0)),
                    ),
                ),
                main_fn(
                    "U64",
                    call("pick", vec![record([("inner", call("maybe", vec![v()]))])]),
                ),
            ],
            vec![("Box", record_type(vec![("inner", OPT)]))],
            "U64",
            vec![CAPABILITY],
        );
    }

    // Ill-typed calls.
    let wrapped = |name: &str, body: Value| {
        (
            name.to_owned(),
            vec![
                maybe(),
                on_option("pick", "U64", body),
                main_through("pick", "U64"),
            ],
        )
    };
    let ill_typed = vec![
        wrapped(
            "some_of_a_mismatched_payload",
            default("U64", some("U64", boolean(true)), u64v(0)),
        ),
        wrapped(
            "some_of_a_narrower_payload",
            default("U64", some("U64", int("U32", 1)), u64v(0)),
        ),
        wrapped(
            "default_of_a_mismatched_fallback",
            default("U64", o(), boolean(true)),
        ),
        wrapped(
            "is_some_of_a_non_option",
            branch(
                compare("==", option("isSome", "U64", vec![u64v(1)]), boolean(true)),
                u64v(1),
                u64v(0),
            ),
        ),
        wrapped("default_of_a_non_option", default("U64", u64v(1), u64v(0))),
        wrapped(
            "some_without_a_payload",
            default("U64", option("some", "U64", Vec::new()), u64v(0)),
        ),
        wrapped(
            "none_with_a_payload",
            default("U64", option("none", "U64", vec![u64v(1)]), u64v(0)),
        ),
        wrapped(
            "default_without_a_fallback",
            option("default", "U64", vec![o()]),
        ),
        wrapped(
            "is_some_of_two_operands",
            branch(
                compare("==", option("isSome", "U64", vec![o(), o()]), boolean(true)),
                u64v(1),
                u64v(0),
            ),
        ),
        wrapped(
            "an_option_call_without_a_type_argument",
            default(
                "U64",
                typed("core.option.some", Vec::new(), vec![u64v(1)]),
                u64v(0),
            ),
        ),
        wrapped(
            "an_option_call_with_two_type_arguments",
            default(
                "U64",
                typed("core.option.some", vec!["U64", "U64"], vec![u64v(1)]),
                u64v(0),
            ),
        ),
        wrapped(
            "an_unknown_option_operation",
            default("U64", option("map", "U64", vec![o()]), u64v(0)),
        ),
        wrapped(
            "an_option_variant_constructor",
            default(
                "U64",
                map([
                    ("kind", text("variant")),
                    ("type", text(OPT)),
                    ("case", text("some")),
                    ("payload", u64v(1)),
                ]),
                u64v(0),
            ),
        ),
    ];
    for (name, functions) in ill_typed {
        cases.refuse(&name, functions);
    }
    // Type arguments that disagree with the operand's item, in `Bool` helpers
    // so that no result-type mismatch can be the reason for the refusal.
    for (name, result) in [
        (
            "default_whose_type_argument_is_not_the_operand_item",
            truth(compare(
                "==",
                default("U32", o(), int("U32", 0)),
                int("U32", 0),
            )),
        ),
        (
            "unwrap_whose_type_argument_is_not_the_operand_item",
            truth(all(vec![
                present("U64", o()),
                compare("==", unwrap("U32", o()), int("U32", 1)),
            ])),
        ),
        (
            "is_some_whose_type_argument_is_not_the_operand_item",
            truth(present("U32", o())),
        ),
    ] {
        cases.push(
            name,
            None,
            vec![
                maybe(),
                on_option("pick", "Bool", result),
                main_through("pick", "Bool"),
            ],
            Vec::new(),
            "Bool",
            vec![CAPABILITY],
        );
    }
    cases.refuse(
        "a_body_whose_option_item_is_not_its_return_item",
        vec![
            maybe(),
            on_option("same", "Option<U32>", o()),
            main_fn("U64", int("U64", 0)),
        ],
    );
    cases.refuse(
        "a_malformed_option_coordinate",
        vec![
            (
                "wrap",
                function(vec![param(0, "U64")], "Option<U64", some("U64", v())),
            ),
            main_fn("U64", u64v(0)),
        ],
    );

    // Forged Option matches: Amblicit names no Option case labels.
    let arm = |case: &str, binder: Option<Value>, body: Value| {
        let mut fields = vec![("case", text(case)), ("body", body)];
        if let Some(binder) = binder {
            fields.push(("binder", binder));
        }
        map(fields)
    };
    let held = local("local.1", "U64");
    let matching = |arms: Vec<Value>| {
        map([
            ("kind", text("match")),
            ("scrutinee", o()),
            ("arms", Value::Array(arms)),
        ])
    };
    for (label, arms) in [
        (
            "an_exhaustive_lowercase_option_match",
            vec![
                arm("none", None, u64v(0)),
                arm("some", Some(held.clone()), read(&held)),
            ],
        ),
        (
            "an_exhaustive_capitalised_option_match",
            vec![
                arm("None", None, u64v(0)),
                arm("Some", Some(held.clone()), read(&held)),
            ],
        ),
        (
            "a_non_exhaustive_option_match",
            vec![arm("some", Some(held.clone()), read(&held))],
        ),
        (
            "an_ill_typed_option_match_binder",
            vec![
                arm("none", None, u64v(0)),
                arm("some", Some(local("local.1", "Bool")), u64v(1)),
            ],
        ),
    ] {
        cases.refuse(
            label,
            vec![
                maybe(),
                on_option("pick", "U64", matching(arms)),
                main_through("pick", "U64"),
            ],
        );
    }
    cases.0
}
