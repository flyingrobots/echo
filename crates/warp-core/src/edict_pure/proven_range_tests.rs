// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime slice and subtraction operands under Edict's proven-range rules.
//!
//! A slice needs `start <= end <= len(bytes)` and a subtraction needs
//! `right <= left`, each from a dominating fact: equal terms, a zero right
//! side, ordered constants, or an earlier conjunct of the same `all`.
//! Conditional predicates, negations and later conjuncts prove nothing.
use super::{
    evaluate::{expression, Meter},
    functions::decode,
    model::{Helper, MAX_PROGRAM_NODES},
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

const BYTES: &str = "Bytes<max=32>";
const MODULE: &str = "range.example@1";

fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(k, v)| (Value::Text(k.into()), v))
            .collect(),
    )
}
fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn constant(kind: &str, width: Option<&str>, value: Value) -> Value {
    let mut literal = vec![("kind", text(kind)), ("value", value)];
    if let Some(width) = width {
        literal.push(("width", text(width)));
    }
    map([("kind", text("const")), ("value", map(literal))])
}
fn word(value: u64) -> Value {
    constant("int", Some("U64"), Value::Integer(i128::from(value)))
}
fn small(value: u32) -> Value {
    constant("int", Some("U32"), Value::Integer(i128::from(value)))
}
fn truth() -> Value {
    constant("bool", None, Value::Bool(true))
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(local: &Value) -> Value {
    map([("kind", text("local")), ("ref", local.clone())])
}
fn call(callee: &str, types: &[&str], args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        (
            "typeArgs",
            Value::Array(types.iter().map(|ty| text(ty)).collect()),
        ),
        ("args", Value::Array(args)),
    ])
}
fn length(bytes: Value) -> Value {
    call("core.bytes.length", &[BYTES], vec![bytes])
}
fn slice(bytes: Value, start: Value, end: Value) -> Value {
    call("core.bytes.slice", &[BYTES], vec![bytes, start, end])
}
fn utf8(bytes: Value) -> Value {
    call("core.bytes.isUtf8", &[BYTES], vec![bytes])
}
fn subtract(left: Value, right: Value) -> Value {
    call("core.integer.subtract", &["U64"], vec![left, right])
}
fn compare(op: &str, left: Value, right: Value) -> Value {
    map([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
fn all(values: Vec<Value>) -> Value {
    map([("kind", text("all")), ("values", Value::Array(values))])
}
fn not(value: Value) -> Value {
    map([("kind", text("not")), ("value", value)])
}
fn choose(predicate: Value, then: Value, otherwise: Value) -> Value {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", then),
        ("else", otherwise),
    ])
}
/// `1u32` when the predicate holds, else `0u32`.
fn flag(predicate: Value) -> Value {
    choose(predicate, small(1), small(0))
}
fn function(params: &[Value], result: Value) -> Value {
    map([
        ("params", Value::Array(params.to_vec())),
        ("returnType", text("U32")),
        (
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", result),
            ]),
        ),
    ])
}
fn helpers(definition: Value) -> Result<BTreeMap<String, Helper>, Error> {
    let core = map([
        ("functions", map([("probe", definition)])),
        ("intents", Value::Map(vec![])),
        ("types", Value::Map(vec![])),
    ]);
    let exports = map([
        ("pureFunctions", Value::Array(vec![])),
        ("effects", Value::Array(vec![])),
    ]);
    let types = Value::Map(vec![]);
    let mut parser = Parser {
        types: &types,
        coordinate: MODULE,
        remaining: MAX_PROGRAM_NODES,
    };
    decode(&core, &exports, &mut parser)
}

struct Locals {
    bytes: Value,
    other: Value,
    at: Value,
    elsewhere: Value,
}
fn locals() -> Locals {
    Locals {
        bytes: local("arg.0", BYTES),
        other: local("arg.1", BYTES),
        at: local("arg.2", "U64"),
        elsewhere: local("arg.3", "U64"),
    }
}
fn params(locals: &Locals) -> Vec<Value> {
    vec![
        locals.bytes.clone(),
        locals.other.clone(),
        locals.at.clone(),
        locals.elsewhere.clone(),
    ]
}
/// Jim's character-boundary test, `at <= len(bytes) && isUtf8(slice(bytes, 0, at))`.
fn boundary(l: &Locals) -> Value {
    let bytes = reference(&l.bytes);
    let at = reference(&l.at);
    flag(all(vec![
        compare("<=", at.clone(), length(bytes.clone())),
        compare("==", utf8(slice(bytes, word(0), at)), truth()),
    ]))
}

fn case(name: &str) -> Result<Value, Error> {
    let l = locals();
    let bytes = reference(&l.bytes);
    let other = reference(&l.other);
    let at = reference(&l.at);
    let elsewhere = reference(&l.elsewhere);
    let prefix = |bytes: Value| compare("<=", at.clone(), length(bytes));
    let use_prefix = || {
        compare(
            "==",
            utf8(slice(bytes.clone(), word(0), at.clone())),
            truth(),
        )
    };
    let result = match name {
        "guarded prefix" => boundary(&l),
        "zero offsets" => flag(compare(
            "==",
            utf8(slice(bytes.clone(), word(0), word(0))),
            truth(),
        )),
        "end offset" => flag(compare(
            "==",
            utf8(slice(bytes.clone(), word(0), length(bytes.clone()))),
            truth(),
        )),
        "guarded suffix" => flag(all(vec![
            prefix(bytes.clone()),
            compare(
                "==",
                utf8(slice(bytes.clone(), at.clone(), length(bytes.clone()))),
                truth(),
            ),
        ])),
        "unguarded" => flag(use_prefix()),
        "guard after use" => flag(all(vec![use_prefix(), prefix(bytes.clone())])),
        "guard on another operand" => flag(all(vec![prefix(other), use_prefix()])),
        "guard on another offset" => flag(all(vec![
            compare("<=", elsewhere, length(bytes.clone())),
            use_prefix(),
        ])),
        "guard by declared capacity" => {
            flag(all(vec![compare("<=", at.clone(), word(32)), use_prefix()]))
        }
        "disjunctive guard" => flag(all(vec![
            not(all(vec![
                not(prefix(bytes.clone())),
                not(prefix(bytes.clone())),
            ])),
            use_prefix(),
        ])),
        "guard in a conditional" => choose(prefix(bytes.clone()), flag(use_prefix()), small(0)),
        "guard in a sibling branch" => choose(prefix(bytes.clone()), small(0), flag(use_prefix())),
        "guarded subtraction" => flag(all(vec![
            compare("<=", elsewhere.clone(), at.clone()),
            compare("<=", subtract(at, elsewhere), word(10)),
        ])),
        "unguarded subtraction" => flag(compare("<=", subtract(at, elsewhere), word(10))),
        "reversed subtraction guard" => flag(all(vec![
            compare("<=", at.clone(), elsewhere.clone()),
            compare("<=", subtract(at, elsewhere), word(10)),
        ])),
        "literal subtraction" => flag(compare("<=", subtract(word(7), word(3)), word(10))),
        "underflowing literal subtraction" => {
            flag(compare("<=", subtract(word(3), word(7)), word(10)))
        }
        _ => return Err(Error::InvalidArtifact),
    };
    Ok(function(&params(&l), result))
}

#[test]
fn dominating_range_facts_admit_runtime_slices_and_subtractions() -> Result<(), Error> {
    for name in [
        "guarded prefix",
        "zero offsets",
        "end offset",
        "guarded suffix",
        "guarded subtraction",
        "literal subtraction",
    ] {
        assert!(helpers(case(name)?).is_ok(), "{name} must be admitted");
    }
    Ok(())
}

#[test]
fn missing_or_non_dominating_range_facts_refuse() -> Result<(), Error> {
    for name in [
        "unguarded",
        "guard after use",
        "guard on another operand",
        "guard on another offset",
        "guard by declared capacity",
        "disjunctive guard",
        "guard in a conditional",
        "guard in a sibling branch",
        "unguarded subtraction",
        "reversed subtraction guard",
        "underflowing literal subtraction",
    ] {
        assert_eq!(
            helpers(case(name)?).map(|_| ()),
            Err(Error::UnsupportedProgram),
            "{name} must be refused before execution"
        );
    }
    Ok(())
}

fn run(helpers: &BTreeMap<String, Helper>, bytes: &[u8], at: u64) -> Result<Value, Error> {
    let call = super::model::Expr::Call(
        format!("{MODULE}.probe"),
        vec![
            super::model::Expr::Constant(Value::Bytes(bytes.to_vec())),
            super::model::Expr::Constant(Value::Bytes(vec![])),
            super::model::Expr::Constant(Value::Integer(i128::from(at))),
            super::model::Expr::Constant(Value::Integer(0)),
        ],
    );
    let mut meter = Meter::new(EvaluationLimits {
        max_package_bytes: 1024,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        max_steps: 4096,
        max_allocated_bytes: 16_384,
    });
    expression(&call, &BTreeMap::new(), helpers, &mut meter, 0)
}

#[test]
fn an_out_of_range_offset_takes_the_authored_path_instead_of_failing() -> Result<(), Error> {
    let checked = helpers(case("guarded prefix")?)?;
    // "a" then U+00E9 as 0xC3 0xA9: offset 2 is inside the character.
    let text = b"a\xc3\xa9";
    for (at, boundary) in [(0, 1), (1, 1), (2, 0), (3, 1), (4, 0), (u64::MAX, 0)] {
        assert_eq!(
            run(&checked, text, at)?,
            Value::Integer(boundary),
            "offset {at}"
        );
    }
    Ok(())
}
