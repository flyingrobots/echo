// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared corpus for the branch-fact rule of Echo's three range judgments.
//!
//! warp-core, the provider lowerer and the provider verifier each include this
//! file and must reach `accepted` for every case. Each case is a Core module
//! of source helpers and no intents, so every judgment checks the same
//! bodies; all but two cases have exactly one helper. Inside the `then` arm of an `if`, the predicate is a fact for
//! additions, subtractions and slices, as in Amblicit's totality rule. The
//! `else` arm, disjunctions, negations and the predicate itself gain nothing.
//! Facts are read with the forms the judgments read before: an addition bound
//! only from `term <= c`, `term == c` or `c == term`, an order from `<=`, `<`,
//! `>=`, `>` or `==` between the two terms. Aligning the operator sets is
//! separate work (echo#960).
//!
//! The first three cases are the Core shapes of Jedit's public reproducers
//! `intent-guarded-add.edict` (as a helper), `subtraction-helper.edict` and
//! `byte-slice-helper.edict`. `&&` lowers to nested binary `all`, as Amblicit
//! emits it.
use echo_edict_canonical::CanonicalValueV1 as Value;

const BYTES: &str = "Bytes<max=8>";

/// One corpus entry: a helper-only Core module, its lawpack exports and the
/// verdict all three judgments must reach.
pub(crate) struct Case {
    pub(crate) name: &'static str,
    pub(crate) core: Value,
    pub(crate) exports: Value,
    pub(crate) accepted: bool,
}

pub(crate) fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}
pub(crate) fn text(value: &str) -> Value {
    Value::Text(value.into())
}
pub(crate) fn param(index: usize, ty: &str) -> Value {
    map([
        ("id", text(&format!("arg.{index}"))),
        ("alphaName", text(&format!("$arg{index}"))),
        ("type", text(ty)),
    ])
}
pub(crate) fn read(local: &Value) -> Value {
    map([("kind", text("local")), ("ref", local.clone())])
}
pub(crate) fn int(value: u64) -> Value {
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
pub(crate) fn boolean(value: bool) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("bool")), ("value", Value::Bool(value))]),
        ),
    ])
}
fn empty_bytes() -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("bytes")), ("value", Value::Bytes(Vec::new()))]),
        ),
    ])
}
pub(crate) fn call(callee: &str, ty: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        ("typeArgs", Value::Array(vec![text(ty)])),
        ("args", Value::Array(args)),
    ])
}
pub(crate) fn add(left: Value, right: Value) -> Value {
    call("core.integer.add", "U64", vec![left, right])
}
fn subtract(left: Value, right: Value) -> Value {
    call("core.integer.subtract", "U64", vec![left, right])
}
fn len(bytes: Value) -> Value {
    call("core.bytes.length", BYTES, vec![bytes])
}
fn slice(bytes: Value, start: Value, end: Value) -> Value {
    call("core.bytes.slice", BYTES, vec![bytes, start, end])
}
pub(crate) fn compare(op: &str, left: Value, right: Value) -> Value {
    map([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
pub(crate) fn all(values: Vec<Value>) -> Value {
    map([("kind", text("all")), ("values", Value::Array(values))])
}
fn any(values: Vec<Value>) -> Value {
    map([("kind", text("any")), ("values", Value::Array(values))])
}
fn not(value: Value) -> Value {
    map([("kind", text("not")), ("value", value)])
}
pub(crate) fn branch(predicate: Value, then: Value, otherwise: Value) -> Value {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", then),
        ("else", otherwise),
    ])
}
pub(crate) fn function(params: Vec<Value>, returns: &str, result: Value) -> Value {
    map([
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
    ])
}
/// A Core module holding exactly `functions`, with no intents or types.
pub(crate) fn module(functions: Vec<(&'static str, Value)>) -> (Value, Value) {
    (
        map([
            ("coordinate", text("generic.branch@1")),
            ("types", map([])),
            ("intents", map([])),
            ("functions", map(functions)),
        ]),
        map([
            ("effects", Value::Array(Vec::new())),
            ("pureFunctions", Value::Array(Vec::new())),
        ]),
    )
}
fn case(name: &'static str, accepted: bool, functions: Vec<(&'static str, Value)>) -> Case {
    let (core, exports) = module(functions);
    Case {
        name,
        core,
        exports,
        accepted,
    }
}

/// `fn f(x: U64, y: U64) -> U64 { return result; }`.
fn scalar(name: &'static str, accepted: bool, result: impl Fn(Value, Value) -> Value) -> Case {
    let (x, y) = (param(0, "U64"), param(1, "U64"));
    let body = result(read(&x), read(&y));
    case(
        name,
        accepted,
        vec![("f", function(vec![x, y], "U64", body))],
    )
}
/// `fn cut(bytes: Bytes<max=8>, start: U64, end: U64) -> Bytes<max=8>`.
fn cut(name: &'static str, accepted: bool, result: impl Fn(Value, Value, Value) -> Value) -> Case {
    let (bytes, start, end) = (param(0, BYTES), param(1, "U64"), param(2, "U64"));
    let body = result(read(&bytes), read(&start), read(&end));
    case(
        name,
        accepted,
        vec![("cut", function(vec![bytes, start, end], BYTES, body))],
    )
}

/// Every case, in a fixed order.
#[allow(
    clippy::too_many_lines,
    reason = "One flat table keeps every case and its verdict side by side"
)]
pub(crate) fn cases() -> Vec<Case> {
    let max = u64::MAX;
    vec![
        // `if index <= 2 then index + 1 else index`.
        scalar("guarded addition", true, |x, _| {
            branch(compare("<=", x.clone(), int(2)), add(x.clone(), int(1)), x)
        }),
        // `if start <= end then end - start else 0`.
        scalar("guarded subtraction", true, |start, end| {
            branch(
                compare("<=", start.clone(), end.clone()),
                subtract(end, start),
                int(0),
            )
        }),
        // `if start <= end && end <= len(bytes) && start <= len(bytes)
        //  then slice(bytes, start, end) else b""`.
        cut("guarded slice", true, |bytes, start, end| {
            branch(
                all(vec![
                    all(vec![
                        compare("<=", start.clone(), end.clone()),
                        compare("<=", end.clone(), len(bytes.clone())),
                    ]),
                    compare("<=", start.clone(), len(bytes.clone())),
                ]),
                slice(bytes, start, end),
                empty_bytes(),
            )
        }),
        scalar("equality with the constant on the left", true, |x, _| {
            branch(compare("==", int(2), x.clone()), add(x.clone(), int(1)), x)
        }),
        scalar(
            "a bound one below the width admits one more",
            true,
            |x, _| {
                branch(
                    compare("<=", x.clone(), int(max - 1)),
                    add(x.clone(), int(1)),
                    x,
                )
            },
        ),
        scalar(
            "a bound one below the width refuses two more",
            false,
            |x, _| {
                branch(
                    compare("<=", x.clone(), int(max - 1)),
                    add(x.clone(), int(2)),
                    x,
                )
            },
        ),
        scalar("a reversed order comparison", true, |start, end| {
            branch(
                compare(">=", end.clone(), start.clone()),
                subtract(end, start),
                int(0),
            )
        }),
        cut(
            "nested branches accumulate facts",
            true,
            |bytes, start, end| {
                branch(
                    compare("<=", start.clone(), end.clone()),
                    branch(
                        compare("<=", end.clone(), len(bytes.clone())),
                        slice(bytes, start, end),
                        empty_bytes(),
                    ),
                    empty_bytes(),
                )
            },
        ),
        scalar("a branch fact joins a conjunct prefix", true, |x, y| {
            branch(
                compare("<=", x.clone(), int(2)),
                branch(
                    all(vec![
                        compare("<=", y.clone(), int(3)),
                        compare("<=", add(x, y), int(5)),
                    ]),
                    int(1),
                    int(0),
                ),
                int(0),
            )
        }),
        // Refused by all three judgments.
        scalar("the else arm gains nothing", false, |x, _| {
            branch(compare("<=", x.clone(), int(2)), x.clone(), add(x, int(1)))
        }),
        scalar("a guard on another operand", false, |x, y| {
            branch(compare("<=", y, int(2)), add(x.clone(), int(1)), x)
        }),
        scalar("a strict bound is not read (echo#960)", false, |x, _| {
            branch(compare("<", x.clone(), int(3)), add(x.clone(), int(1)), x)
        }),
        scalar("a disjunction proves nothing", false, |x, y| {
            branch(
                any(vec![
                    compare("<=", x.clone(), int(2)),
                    compare("<=", y, int(2)),
                ]),
                add(x.clone(), int(1)),
                x,
            )
        }),
        // Reading through the `not` would find `x <= 2` and accept a sum that
        // overflows when `x` is the width maximum.
        scalar("a negation proves nothing", false, |x, _| {
            branch(
                not(compare("<=", x.clone(), int(2))),
                add(x.clone(), int(1)),
                x,
            )
        }),
        // Reading through the `not` would find `start <= end` and accept a
        // subtraction that underflows whenever the arm runs.
        scalar("a negated order proves nothing", false, |start, end| {
            branch(
                not(compare("<=", start.clone(), end.clone())),
                subtract(end, start),
                int(0),
            )
        }),
        // The later conjunct `x <= 2` would bound `x + 1` if the predicate were
        // a fact while it is itself judged.
        scalar("a predicate does not prove itself", false, |x, _| {
            branch(
                all(vec![
                    compare("<=", add(x.clone(), int(1)), int(9)),
                    compare("<=", x.clone(), int(2)),
                ]),
                x,
                int(0),
            )
        }),
        // `x + 1` is a sibling of the inner `if`, outside its `then` arm.
        scalar("a fact does not leave its branch", false, |x, _| {
            branch(
                all(vec![
                    compare(
                        "<=",
                        branch(compare("<=", x.clone(), int(2)), x.clone(), int(0)),
                        int(2),
                    ),
                    compare("<=", add(x, int(1)), int(9)),
                ]),
                int(1),
                int(0),
            )
        }),
        // An `if` inside a predicate operand proves its own `then` arm.
        scalar("a then arm inside a predicate operand", true, |x, _| {
            branch(
                compare(
                    "<=",
                    branch(compare("<=", x.clone(), int(2)), add(x, int(1)), int(0)),
                    int(3),
                ),
                int(1),
                int(0),
            )
        }),
        scalar("subtraction in the wrong order", false, |start, end| {
            branch(
                compare("<=", start.clone(), end.clone()),
                subtract(start, end),
                int(0),
            )
        }),
        cut(
            "a slice without its length bound",
            false,
            |bytes, start, end| {
                branch(
                    compare("<=", start.clone(), end.clone()),
                    slice(bytes, start, end),
                    empty_bytes(),
                )
            },
        ),
        {
            // A helper frame starts without its caller's branch facts. The caller
            // sorts first, so a judgment that checks helpers in name order meets
            // the callee first inside the caller's `then` arm.
            let x = param(0, "U64");
            let inner = function(vec![x.clone()], "U64", add(read(&x), int(1)));
            let outer = function(
                vec![x.clone()],
                "U64",
                branch(
                    compare("<=", read(&x), int(2)),
                    call_helper("generic.branch@1.z_inner", vec![read(&x)]),
                    read(&x),
                ),
            );
            case(
                "a caller's branch does not reach its helper",
                false,
                vec![("a_outer", outer), ("z_inner", inner)],
            )
        },
        {
            // The caller's branch facts are back after it first calls a
            // helper. A judgment that judges a helper at its first call (the
            // lowerer) judges `x + 1` after `z_same(x)` in the same `then` arm,
            // and only the outer `x <= 2` bounds it.
            let x = param(0, "U64");
            let same = function(vec![x.clone()], "U64", read(&x));
            let caller = function(
                vec![x.clone()],
                "U64",
                branch(
                    compare("<=", read(&x), int(2)),
                    branch(
                        compare(
                            "==",
                            call_helper("generic.branch@1.z_same", vec![read(&x)]),
                            read(&x),
                        ),
                        add(read(&x), int(1)),
                        int(0),
                    ),
                    read(&x),
                ),
            );
            case(
                "a caller keeps its branch facts after a helper call",
                true,
                vec![("a_caller", caller), ("z_same", same)],
            )
        },
    ]
}

fn call_helper(callee: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        ("typeArgs", Value::Array(Vec::new())),
        ("args", Value::Array(args)),
    ])
}

/// Where an intent's addition `index + 1` takes its bound from.
#[derive(Clone, Copy, Debug)]
pub(crate) enum IntentShape {
    /// `let index = if input.index <= 2 then input.index + 1 else input.index`,
    /// the Core of Jedit's `intent-guarded-add.edict`.
    ThenArm,
    /// The same branch with the addition in the `else` arm, which gains no fact.
    ElseArm,
    /// `where input.index <= 2` and `let index = input.index + 1`: the same
    /// module shape with the bound as an input constraint, which the
    /// judgments accepted before branch facts.
    InputConstraint,
}

/// A pure intent `evaluate({ index: U64 }) -> { index: U64 }` of `shape`.
pub(crate) fn guarded_intent(shape: IntentShape) -> (Value, Value) {
    let record = || {
        map([
            ("kind", text("Record")),
            ("fields", map([("index", text("U64"))])),
        ])
    };
    let input_local = map([
        ("id", text("arg.0")),
        ("alphaName", text("$input")),
        ("type", text("generic.branch@1.Input")),
    ]);
    let index_local = map([
        ("id", text("local.0")),
        ("alphaName", text("$index")),
        ("type", text("U64")),
    ]);
    let index = map([
        ("kind", text("field")),
        ("base", read(&input_local)),
        ("field", text("index")),
    ]);
    let bound = compare("<=", index.clone(), int(2));
    let incremented = add(index.clone(), int(1));
    let (constraints, value) = match shape {
        IntentShape::ThenArm => (Vec::new(), branch(bound, incremented, index)),
        IntentShape::ElseArm => (Vec::new(), branch(bound, index, incremented)),
        IntentShape::InputConstraint => (
            vec![map([
                ("coordinate", text("generic.branch@1.evaluate.where.0")),
                ("predicate", bound),
            ])],
            incremented,
        ),
    };
    let intent = map([
        ("input", text("generic.branch@1.Input")),
        ("output", text("generic.branch@1.Output")),
        ("inputConstraints", Value::Array(constraints)),
        (
            "coreEvaluationBudget",
            map([
                ("maxSteps", Value::Integer(10_000)),
                ("maxAllocatedBytes", Value::Integer(10_000)),
                ("maxOutputBytes", Value::Integer(10_000)),
            ]),
        ),
        (
            "body",
            map([
                (
                    "locals",
                    Value::Array(vec![input_local, index_local.clone()]),
                ),
                (
                    "nodes",
                    Value::Array(vec![map([
                        ("kind", text("let")),
                        ("binding", index_local.clone()),
                        ("value", value),
                    ])]),
                ),
                (
                    "result",
                    map([
                        ("kind", text("record")),
                        ("fields", map([("index", read(&index_local))])),
                    ]),
                ),
            ]),
        ),
    ]);
    (
        map([
            ("coordinate", text("generic.branch@1")),
            ("types", map([("Input", record()), ("Output", record())])),
            ("intents", map([("evaluate", intent)])),
        ]),
        map([
            ("effects", Value::Array(Vec::new())),
            ("pureFunctions", Value::Array(Vec::new())),
        ]),
    )
}

/// A helper `fn f(x: U64) -> Bool` whose body judges `size` sums that fit by
/// their own literals, inside an inner `if`. With `guarded`, the sums sit in
/// the `then` arm of 64 bounds on `x` that none of them needs; otherwise in
/// the `else` arm, where those bounds are not facts.
pub(crate) fn unneeded_branch_facts(guarded: bool, size: usize) -> (Value, Value) {
    let x = param(0, "U64");
    let guards = all((0..64)
        .map(|index| compare("<=", read(&x), int(10 + index)))
        .collect());
    let sums = branch(
        all(std::iter::repeat_n(compare("<=", add(int(1), int(1)), int(2)), size).collect()),
        boolean(true),
        boolean(false),
    );
    let result = if guarded {
        branch(guards, sums, boolean(false))
    } else {
        branch(guards, boolean(false), sums)
    };
    module(vec![("f", function(vec![x], "Bool", result))])
}

/// The largest `size` in `1..=limit` for which `accepts(size)` holds, by
/// bisection. `accepts(1)` must hold.
pub(crate) fn largest_accepted(limit: usize, accepts: impl Fn(usize) -> bool) -> Option<usize> {
    if !accepts(1) {
        return None;
    }
    let (mut accepted, mut refused) = (1, limit + 1);
    while refused - accepted > 1 {
        let middle = accepted + (refused - accepted) / 2;
        if accepts(middle) {
            accepted = middle;
        } else {
            refused = middle;
        }
    }
    Some(accepted)
}
