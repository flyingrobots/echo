// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Actual raw-byte lookup values, strict runtime bounds and deterministic charges.
use super::{
    evaluate::{expression, Meter},
    model::MAX_PROGRAM_NODES,
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(k, v)| (Value::Text(k.into()), v))
            .collect(),
    )
}
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn lookup(bytes: &[u8], index: i128) -> Value {
    let operand = map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bytes")),
                ("value", Value::Bytes(bytes.to_vec())),
            ]),
        ),
    ]);
    let offset = map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(index)),
            ]),
        ),
    ]);
    map([
        ("kind", text("call")),
        ("callee", text("core.bytes.at")),
        ("typeArgs", Value::Array(vec![text("Bytes<max=8>")])),
        ("args", Value::Array(vec![operand, offset])),
    ])
}
fn limits(steps: u64, allocated: u64) -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1024,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        max_steps: steps,
        max_allocated_bytes: allocated,
    }
}
fn run(bytes: &[u8], index: i128, limit: EvaluationLimits) -> Result<(Value, (u64, u64)), Error> {
    let types = map([]);
    let mut parser = Parser {
        types: &types,
        coordinate: "generic.lookup@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let expr = parser.expr(&lookup(bytes, index), 0)?;
    let mut meter = Meter::new(limit);
    let value = expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?;
    Ok((value, meter.usage()))
}
#[test]
fn byte_lookup_preserves_octets_and_exact_resource_limits() -> Result<(), Error> {
    for (bytes, index, want) in [
        (&[0, 128, 255][..], 0, 0),
        (&[0, 128, 255][..], 1, 128),
        (&[0, 128, 255][..], 2, 255),
    ] {
        let (value, (steps, allocated)) = run(bytes, index, limits(1024, 4096))?;
        assert_eq!(value, Value::Integer(want));
        // Three expression nodes + three value copies + two validations.
        // Storage is the three value cells plus the raw operand bytes.
        assert_eq!((steps, allocated), (8, 3 * 64 + bytes.len() as u64));
        assert_eq!(
            run(bytes, index, limits(steps, allocated))?,
            (value, (steps, allocated))
        );
        assert_eq!(
            run(bytes, index, limits(steps - 1, allocated)),
            Err(Error::StepBudgetExceeded)
        );
        assert_eq!(
            run(bytes, index, limits(steps, allocated - 1)),
            Err(Error::AllocationBudgetExceeded)
        );
    }
    Ok(())
}
#[test]
fn byte_lookup_refuses_empty_equal_length_and_huge_indices() {
    for (bytes, index) in [
        (&b""[..], 0),
        (&b"a"[..], 1),
        (&b"a"[..], i128::from(u64::MAX)),
    ] {
        assert_eq!(
            run(bytes, index, limits(1024, 4096)),
            Err(Error::InvalidArtifact)
        );
    }
}

fn array(v: Vec<Value>) -> Value {
    Value::Array(v)
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(v: &Value) -> Value {
    map([("kind", text("local")), ("ref", v.clone())])
}
fn integer(width: &str, n: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text(width)),
                ("value", Value::Integer(n.into())),
            ]),
        ),
    ])
}
fn call(name: &str, ty: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(name)),
        ("typeArgs", array(vec![text(ty)])),
        ("args", array(args)),
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
fn branch(p: Value, a: Value, b: Value) -> Value {
    map([
        ("kind", text("if")),
        ("predicate", p),
        ("then", a),
        ("else", b),
    ])
}
fn boolean(kind: &str, values: Vec<Value>) -> Value {
    map([("kind", text(kind)), ("values", array(values))])
}
fn fixture(op: &str, wrong_operand: bool, wrong_arm: bool, lazy: bool) -> (Value, Value) {
    let bytes = local("arg.0", "Bytes<max=8>");
    let index = local("arg.1", "U64");
    let other = local("arg.2", "Bytes<max=8>");
    let length = call(
        "core.bytes.length",
        "Bytes<max=8>",
        vec![reference(if wrong_operand { &other } else { &bytes })],
    );
    let guard = compare(op, reference(&index), length);
    let lookup = call(
        "core.bytes.at",
        "Bytes<max=8>",
        vec![reference(&bytes), reference(&index)],
    );
    let result = if lazy {
        branch(
            boolean("any", vec![guard, compare("<", lookup, integer("U8", 128))]),
            integer("U8", 0),
            integer("U8", 255),
        )
    } else if wrong_arm {
        branch(guard, integer("U8", 0), lookup)
    } else {
        branch(guard, lookup, integer("U8", 0))
    };
    let function = map([
        ("params", array(vec![bytes, index, other])),
        ("returnType", text("U8")),
        (
            "body",
            map([
                ("locals", array(vec![])),
                ("bindings", array(vec![])),
                ("result", result),
            ]),
        ),
    ]);
    (
        map([
            ("coordinate", text("generic.lookup@1")),
            ("types", map([])),
            ("intents", map([])),
            ("functions", map([("lookup", function)])),
        ]),
        map([("effects", array(vec![])), ("pureFunctions", array(vec![]))]),
    )
}

fn checked_helpers(
    core: &Value,
    exports: &Value,
) -> Result<BTreeMap<String, super::model::Helper>, Error> {
    let types = map([]);
    let mut parser = Parser {
        types: &types,
        coordinate: "generic.lookup@1",
        remaining: MAX_PROGRAM_NODES,
    };
    super::functions::decode(core, exports, &mut parser)
}
#[test]
fn byte_lookup_helper_admission_and_lazy_execution_keep_proof_scopes() -> Result<(), Error> {
    for (op, arm, lazy) in [
        ("<", false, false),
        (">=", true, false),
        (">=", false, true),
    ] {
        let (core, exports) = fixture(op, false, arm, lazy);
        let helpers = checked_helpers(&core, &exports)?;
        for (bytes, index) in [
            (&[0, 128, 255][..], 0_u64),
            (&[0, 128, 255][..], 2),
            (&b""[..], 0),
            (&b"a"[..], 1),
            (&b"a"[..], u64::MAX),
        ] {
            let expr = super::model::Expr::Call(
                "generic.lookup@1.lookup".into(),
                vec![
                    super::model::Expr::Constant(Value::Bytes(bytes.to_vec())),
                    super::model::Expr::Constant(Value::Integer(index.into())),
                    super::model::Expr::Constant(Value::Bytes(vec![])),
                ],
            );
            let mut meter = Meter::new(limits(4096, 16384));
            let value = expression(&expr, &BTreeMap::new(), &helpers, &mut meter, 0)?;
            let selected = usize::try_from(index).ok().and_then(|i| bytes.get(i));
            let expected = if lazy {
                if selected.is_none_or(|byte| *byte < 128) {
                    0
                } else {
                    255
                }
            } else {
                usize::try_from(index)
                    .ok()
                    .and_then(|i| bytes.get(i))
                    .map_or(0, |b| i128::from(*b))
            };
            assert_eq!(value, Value::Integer(expected));
        }
    }
    for (op, wrong, arm, lazy) in [
        ("<=", false, false, false),
        ("<", true, false, false),
        ("<", false, true, false),
        ("<", false, false, true),
    ] {
        let (core, exports) = fixture(op, wrong, arm, lazy);
        assert!(matches!(
            checked_helpers(&core, &exports),
            Err(Error::UnsupportedProgram)
        ));
    }
    Ok(())
}

fn replace_term(value: &mut Value, from: &Value, to: &Value) {
    if value == from {
        *value = to.clone();
        return;
    }
    match value {
        Value::Map(fields) => {
            for (_, child) in fields {
                replace_term(child, from, to);
            }
        }
        Value::Array(children) => {
            for child in children {
                replace_term(child, from, to);
            }
        }
        _ => {}
    }
}

#[test]
fn byte_lookup_mirrored_equal_terms_keep_both_strict_orientations() {
    // These strict guards are always false (or their non-strict complements
    // always true). The unreachable lookup still has a valid dominating proof.
    for (op, lookup_in_else) in [("<", false), (">", false), (">=", true), ("<=", true)] {
        let (mut core, exports) = fixture(op, false, lookup_in_else, false);
        let index = reference(&local("arg.1", "U64"));
        let length = call(
            "core.bytes.length",
            "Bytes<max=8>",
            vec![reference(&local("arg.0", "Bytes<max=8>"))],
        );
        replace_term(&mut core, &index, &length);
        assert!(checked_helpers(&core, &exports).is_ok(), "{op}");
    }
}

fn member_mut<'a>(value: &'a mut Value, name: &str) -> Result<&'a mut Value, String> {
    let Value::Map(fields) = value else {
        return Err("expected fixture map".into());
    };
    fields
        .iter_mut()
        .find(|(key, _)| key == &text(name))
        .map(|(_, value)| value)
        .ok_or_else(|| format!("missing fixture key: {name}"))
}

#[test]
fn byte_lookup_helper_cannot_borrow_a_guarded_callers_path() -> Result<(), String> {
    for guarded_helper in [false, true] {
        let (mut core, exports) = fixture("<", false, false, false);
        let functions = member_mut(&mut core, "functions")?;
        let mut caller = member_mut(functions, "lookup")?.clone();
        let bytes = reference(&local("arg.0", "Bytes<max=8>"));
        let index = reference(&local("arg.1", "U64"));
        let other = reference(&local("arg.2", "Bytes<max=8>"));
        let lookup = call(
            "core.bytes.at",
            "Bytes<max=8>",
            vec![bytes.clone(), index.clone()],
        );
        let invoke = map([
            ("kind", text("call")),
            ("callee", text("generic.lookup@1.lookup")),
            ("typeArgs", array(vec![])),
            ("args", array(vec![bytes, index, other])),
        ]);
        replace_term(&mut caller, &lookup, &invoke);
        if !guarded_helper {
            *member_mut(
                member_mut(member_mut(functions, "lookup")?, "body")?,
                "result",
            )? = lookup;
        }
        let Value::Map(entries) = functions else {
            return Err("fixture map".into());
        };
        // The caller sorts before the helper. Deliberately reuse frame-local
        // parameter IDs so leaking its proof would falsely authorize the callee.
        entries.push((text("aaCaller"), caller));
        assert_eq!(
            checked_helpers(&core, &exports).is_ok(),
            guarded_helper,
            "guarded_helper={guarded_helper}"
        );
    }
    Ok(())
}
