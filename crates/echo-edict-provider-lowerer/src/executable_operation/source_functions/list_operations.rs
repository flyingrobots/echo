// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Forward judgment for bounded immutable list calls and their strict domains.
use super::{byte_at, get, includes, list, shape, string, Check, Cost, Fact, Judgment, Ty, Value};

pub(super) fn is_operation(name: &str) -> bool {
    matches!(
        name,
        "core.list.length" | "core.list.at" | "core.list.append" | "core.list.concat"
    )
}

impl Judgment<'_> {
    pub(super) fn list_operation(
        &mut self,
        value: &Value,
        actual: &[Ty],
        declared: &[Ty],
    ) -> Check<Fact> {
        if !list(get(self.core, "requiredCoreCapabilities")?)?
            .iter()
            .any(|v| string(v) == Ok("core.list/v1"))
        {
            return Err(());
        }
        let name = string(get(value, "callee")?)?;
        let args = list(get(value, "args")?)?;
        let coordinates = list(get(value, "typeArgs")?)?;
        let mut validations = Vec::new();
        let result = match (name, actual, declared) {
            ("core.list.length", [value], [ty @ Ty::List(Some(_), _)]) if includes(ty, value) => {
                validations.push(ty);
                Ty::Word(u64::MAX)
            }
            (
                "core.list.at",
                [value, index @ Ty::Word(u64::MAX)],
                [ty @ Ty::List(Some(item), _)],
            ) if includes(ty, value) => {
                self.list_domain(&coordinates[0], args, None)?;
                validations.extend([ty, index]);
                *item.clone()
            }
            ("core.list.append", [value, element], [ty @ Ty::List(Some(item), max)])
                if ty == value && includes(item, element) =>
            {
                self.list_domain(&coordinates[0], args, Some(*max))?;
                validations.extend([ty, item.as_ref()]);
                ty.clone()
            }
            (
                "core.list.concat",
                [left, right],
                [a @ Ty::List(Some(item), n), b @ Ty::List(Some(other), m)],
            ) if item == other && includes(a, left) && includes(b, right) => {
                if item_coordinate(self.core, string(&coordinates[0])?)?
                    != item_coordinate(self.core, string(&coordinates[1])?)?
                {
                    return Err(());
                }
                validations.extend([a, b]);
                Ty::List(Some(item.clone()), n.checked_add(*m).ok_or(())?)
            }
            _ => return Err(()),
        };
        let (mut cost, _, _, depth) = shape(&result)?;
        for operand in validations {
            cost = cost.plus(Cost {
                steps: shape(operand)?.1,
                bytes: 0,
            })?;
        }
        Ok(Fact {
            ty: result,
            cost,
            depth,
        })
    }

    fn list_domain(
        &mut self,
        coordinate: &Value,
        args: &[Value],
        capacity: Option<u64>,
    ) -> Check<()> {
        let [value, operand] = args else {
            return Err(());
        };
        let length = Value::Map(vec![
            (text("kind"), text("call")),
            (text("callee"), text("core.list.length")),
            (text("typeArgs"), Value::Array(vec![coordinate.clone()])),
            (text("args"), Value::Array(vec![value.clone()])),
        ]);
        let upper = capacity.map(|n| {
            Value::Map(vec![
                (text("kind"), text("const")),
                (
                    text("value"),
                    Value::Map(vec![
                        (text("kind"), text("int")),
                        (text("width"), text("U64")),
                        (text("value"), Value::Integer(n.into())),
                    ]),
                ),
            ])
        });
        let (a, b) = upper
            .as_ref()
            .map_or((operand, &length), |upper| (&length, upper));
        let a = byte_at::key(a, &mut self.work)?;
        let b = byte_at::key(b, &mut self.work)?;
        for (predicate, holds) in self
            .proven
            .iter()
            .map(|p| (*p, true))
            .chain(self.path.iter().copied())
        {
            if byte_at::strict(predicate, holds, &a, &b, &mut self.work, 0)? {
                return Ok(());
            }
        }
        Err(())
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn item_coordinate<'a>(core: &'a Value, name: &'a str) -> Check<&'a str> {
    if let Some(inner) = name.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        return inner.rsplit_once(",max=").map(|(item, _)| item).ok_or(());
    }
    let prefix = format!("{}.", string(get(core, "coordinate")?)?);
    let ty = get(
        get(core, "types")?,
        name.strip_prefix(&prefix).unwrap_or(name),
    )?;
    if string(get(ty, "kind")?)? != "List" {
        return Err(());
    }
    string(get(ty, "item")?)
}
