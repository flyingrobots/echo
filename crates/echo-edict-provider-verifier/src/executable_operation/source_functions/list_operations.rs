// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned list signatures, strict domains and conservative charges.
use super::{byte_at, get, sequence, string, Charge, Check, Checker, Schema, Value};

pub(super) fn is_operation(name: &str) -> bool {
    [
        "core.list.length",
        "core.list.at",
        "core.list.append",
        "core.list.concat",
    ]
    .contains(&name)
}

pub(super) fn authority(core: &Value, name: &str, coordinates: &[Value]) -> Check<()> {
    let capabilities = sequence(get(core, "requiredCoreCapabilities")?)?;
    if !capabilities.iter().any(|v| string(v) == Ok("core.list/v1")) {
        return Err(());
    }
    if name == "core.list.concat" {
        let [left, right] = coordinates else {
            return Err(());
        };
        if item(core, string(left)?)? != item(core, string(right)?)? {
            return Err(());
        }
    }
    Ok(())
}

pub(super) fn infer(
    name: &str,
    actual: &[Schema],
    declared: &[Schema],
    proof: bool,
) -> Check<(Schema, Charge)> {
    let mut checks = Vec::new();
    let output = match (name, actual, declared) {
        (
            "core.list.length",
            [value],
            [sequence @ Schema::Sequence {
                element: Some(_), ..
            }],
        ) if sequence.encloses(value) => {
            checks.push(sequence);
            Schema::Word(u64::MAX)
        }
        (
            "core.list.at",
            [value, index @ Schema::Word(u64::MAX)],
            [sequence @ Schema::Sequence {
                element: Some(element),
                ..
            }],
        ) if sequence.encloses(value) && proof => {
            checks.extend([sequence, index]);
            *element.clone()
        }
        (
            "core.list.append",
            [value, entry],
            [sequence @ Schema::Sequence {
                element: Some(element),
                ..
            }],
        ) if sequence == value && element.encloses(entry) && proof => {
            checks.extend([sequence, element.as_ref()]);
            sequence.clone()
        }
        (
            "core.list.concat",
            [left, right],
            [a @ Schema::Sequence {
                element: Some(element),
                bound: n,
            }, b @ Schema::Sequence {
                element: Some(other),
                bound: m,
            }],
        ) if element == other && a == left && b == right => {
            checks.extend([a, b]);
            Schema::Sequence {
                element: Some(element.clone()),
                bound: n.checked_add(*m).ok_or(())?,
            }
        }
        _ => return Err(()),
    };
    let mut charge = output.measure()?.copy;
    for schema in checks {
        charge.append(Charge {
            ticks: schema.measure()?.checks,
            storage: 0,
        })?;
    }
    Ok((output, charge))
}

impl Checker<'_, '_> {
    pub(super) fn list_domain(&mut self, call: &Value, declared: &[Schema]) -> Check<bool> {
        let [coordinate] = sequence(get(call, "typeArgs")?)? else {
            return Err(());
        };
        let [value, operand] = sequence(get(call, "args")?)? else {
            return Err(());
        };
        let length = map([
            ("kind", text("call")),
            ("callee", text("core.list.length")),
            ("typeArgs", Value::Array(vec![coordinate.clone()])),
            ("args", Value::Array(vec![value.clone()])),
        ]);
        let capacity = if string(get(call, "callee")?)? == "core.list.append" {
            let [Schema::Sequence { bound, .. }] = declared else {
                return Err(());
            };
            Some(map([
                ("kind", text("const")),
                (
                    "value",
                    map([
                        ("kind", text("int")),
                        ("width", text("U64")),
                        ("value", Value::Integer((*bound).into())),
                    ]),
                ),
            ]))
        } else {
            None
        };
        let (lower, upper) = match &capacity {
            Some(capacity) => (&length, capacity),
            None => (operand, &length),
        };
        let lower = byte_at::canonical(lower, self.types)?;
        let upper = byte_at::canonical(upper, self.types)?;
        for (fact, holds) in self
            .proven
            .iter()
            .map(|p| (*p, true))
            .chain(self.path.iter().copied())
        {
            if byte_at::establishes(fact, holds, &lower, &upper, self.types, 0)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn item<'a>(core: &'a Value, coordinate: &'a str) -> Check<&'a str> {
    if let Some(body) = coordinate
        .strip_prefix("List<")
        .and_then(|s| s.strip_suffix('>'))
    {
        return body
            .rsplit_once(",max=")
            .map(|(element, _)| element)
            .ok_or(());
    }
    let module = format!("{}.", string(get(core, "coordinate")?)?);
    let definition = get(
        get(core, "types")?,
        coordinate.strip_prefix(&module).unwrap_or(coordinate),
    )?;
    if string(get(definition, "kind")?)? != "List" {
        return Err(());
    }
    string(get(definition, "item")?)
}
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}
