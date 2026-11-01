// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned list signatures and strict domains, independent of providers.
mod evaluate;
use super::{
    functions::accepts,
    model::{Expr, RuntimeType, MAX_DEPTH},
    proven_order,
    syntax::Parser,
    values::{array, field, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
pub(super) use evaluate::evaluate;

pub(super) fn is_operation(name: &str) -> bool {
    matches!(
        name,
        "core.list.length" | "core.list.at" | "core.list.append" | "core.list.concat"
    )
}

// All module bodies, including unused helpers, require the capability. This
// bounded scan also covers intent-only operations and imported helper bodies.
pub(super) fn without_capability(
    value: &Value,
    remaining: &mut usize,
    depth: usize,
) -> Result<(), Error> {
    *remaining = remaining.checked_sub(1).ok_or(Error::UnsupportedProgram)?;
    if depth > MAX_DEPTH {
        return Err(Error::UnsupportedProgram);
    }
    match value {
        Value::Map(entries) => {
            if text_field(value, "kind") == Ok("call")
                && text_field(value, "callee").is_ok_and(is_operation)
            {
                return Err(Error::UnsupportedProgram);
            }
            for (key, value) in entries {
                without_capability(key, remaining, depth + 1)?;
                without_capability(value, remaining, depth + 1)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                without_capability(value, remaining, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn parse(
    parser: &mut Parser<'_>,
    name: &str,
    args: &[Value],
    coordinates: &[Value],
    depth: usize,
) -> Result<Expr, Error> {
    let mut types = coordinates
        .iter()
        .map(|v| {
            let ty = parser.ty(text(v)?, depth + 1)?;
            if !matches!(ty, RuntimeType::List { item: Some(_), .. }) {
                return Err(Error::InvalidArtifact);
            }
            Ok(ty)
        })
        .collect::<Result<Vec<_>, _>>()?;
    match (name, args, types.as_slice()) {
        ("core.list.length", [value], [_]) => Ok(Expr::ListLength {
            ty: types.remove(0),
            value: Box::new(parser.expr(value, depth + 1)?),
        }),
        ("core.list.at", [value, index], [_]) => Ok(Expr::ListAt {
            ty: types.remove(0),
            value: Box::new(parser.expr(value, depth + 1)?),
            index: Box::new(parser.expr(index, depth + 1)?),
        }),
        ("core.list.append", [value, item], [_]) => Ok(Expr::ListAppend {
            ty: types.remove(0),
            value: Box::new(parser.expr(value, depth + 1)?),
            item: Box::new(parser.expr(item, depth + 1)?),
        }),
        (
            "core.list.concat",
            [left, right],
            [RuntimeType::List { item: a, max: n }, RuntimeType::List { item: b, max: m }],
        ) => {
            if a != b
                || item_coordinate(parser, text(&coordinates[0])?)?
                    != item_coordinate(parser, text(&coordinates[1])?)?
            {
                return Err(Error::InvalidArtifact);
            }
            n.checked_add(*m).ok_or(Error::InvalidArtifact)?;
            Ok(Expr::ListConcat {
                left_type: Box::new(types.remove(0)),
                right_type: Box::new(types.remove(0)),
                left: Box::new(parser.expr(left, depth + 1)?),
                right: Box::new(parser.expr(right, depth + 1)?),
            })
        }
        _ => Err(Error::InvalidArtifact),
    }
}

pub(super) fn infer(
    call: &Value,
    actual: &[RuntimeType],
    declared: &[RuntimeType],
    facts: &[&Value],
    path: &[(&Value, bool)],
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<RuntimeType, Error> {
    let name = text_field(call, "callee")?;
    let coordinates = array(field(call, "typeArgs")?)?;
    let args = array(field(call, "args")?)?;
    match (name, actual, declared) {
        ("core.list.length", [value], [ty @ RuntimeType::List { item: Some(_), .. }])
            if accepts(ty, value) =>
        {
            Ok(RuntimeType::Unsigned(u64::MAX))
        }
        (
            "core.list.at",
            [value, RuntimeType::Unsigned(u64::MAX)],
            [ty @ RuntimeType::List {
                item: Some(item), ..
            }],
        ) if accepts(ty, value) => {
            domain(&coordinates[0], args, None, facts, path, parser, depth + 1)?;
            Ok(*item.clone())
        }
        (
            "core.list.append",
            [value, entry],
            [ty @ RuntimeType::List {
                item: Some(item),
                max,
            }],
        ) if ty == value && accepts(item, entry) => {
            domain(
                &coordinates[0],
                args,
                Some(*max),
                facts,
                path,
                parser,
                depth + 1,
            )?;
            Ok(ty.clone())
        }
        (
            "core.list.concat",
            [left, right],
            [a @ RuntimeType::List {
                item: Some(item),
                max: n,
            }, b @ RuntimeType::List {
                item: Some(other),
                max: m,
            }],
        ) if item == other && accepts(a, left) && accepts(b, right) => {
            if item_coordinate(parser, text(&coordinates[0])?)?
                != item_coordinate(parser, text(&coordinates[1])?)?
            {
                return Err(Error::InvalidArtifact);
            }
            Ok(RuntimeType::List {
                item: Some(item.clone()),
                max: n.checked_add(*m).ok_or(Error::InvalidArtifact)?,
            })
        }
        _ => Err(Error::InvalidArtifact),
    }
}

fn item_coordinate<'a>(parser: &Parser<'a>, coordinate: &'a str) -> Result<&'a str, Error> {
    if let Some(body) = coordinate
        .strip_prefix("List<")
        .and_then(|s| s.strip_suffix('>'))
    {
        return body
            .rsplit_once(",max=")
            .map(|(item, _)| item)
            .ok_or(Error::InvalidArtifact);
    }
    let prefix = format!("{}.", parser.coordinate);
    let ty = field(
        parser.types,
        coordinate.strip_prefix(&prefix).unwrap_or(coordinate),
    )?;
    if text_field(ty, "kind")? != "List" {
        return Err(Error::InvalidArtifact);
    }
    text_field(ty, "item")
}

fn domain(
    coordinate: &Value,
    args: &[Value],
    capacity: Option<u64>,
    facts: &[&Value],
    path: &[(&Value, bool)],
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<(), Error> {
    let [value, operand] = args else {
        return Err(Error::InvalidArtifact);
    };
    let length = map([
        ("kind", string("call")),
        ("callee", string("core.list.length")),
        ("typeArgs", Value::Array(vec![coordinate.clone()])),
        ("args", Value::Array(vec![value.clone()])),
    ]);
    let capacity = capacity.map(|n| {
        map([
            ("kind", string("const")),
            (
                "value",
                map([
                    ("kind", string("int")),
                    ("width", string("U64")),
                    ("value", Value::Integer(n.into())),
                ]),
            ),
        ])
    });
    let (lower, upper) = capacity
        .as_ref()
        .map_or((operand, &length), |capacity| (&length, capacity));
    let lower = proven_order::key(lower, parser)?;
    let upper = proven_order::key(upper, parser)?;
    for (predicate, truth) in facts.iter().map(|p| (*p, true)).chain(path.iter().copied()) {
        if proven_order::strict_fact(predicate, truth, &lower, &upper, parser, depth)? {
            return Ok(());
        }
    }
    Err(Error::UnsupportedProgram)
}
fn string(value: &str) -> Value {
    Value::Text(value.into())
}
fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (string(key), value))
            .collect(),
    )
}
