// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned bounded type expansion and local declaration checking.

use super::super::{exact, get, members, number, string, text, Check, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Schema {
    Bytes {
        lower: u64,
        upper: u64,
    },
    Word(u64),
    Row(BTreeMap<String, Schema>),
    Nominal {
        identity: String,
        storage: Box<Schema>,
    },
}

impl Schema {
    pub(super) fn representation(&self) -> &Self {
        match self {
            Self::Nominal { storage, .. } => storage.representation(),
            _ => self,
        }
    }

    pub(super) fn address(&self) -> bool {
        let Self::Row(fields) = self else {
            return false;
        };
        fields.len() == 3
            && ["nodeId", "warpId", "typeId"].iter().all(|key| {
                fields.get(*key).is_some_and(|ty| {
                    ty.representation()
                        == &Self::Bytes {
                            lower: 32,
                            upper: 32,
                        }
                })
            })
    }
    fn includes(&self, actual: &Self) -> bool {
        match (self, actual) {
            (Self::Bytes { lower, upper }, Self::Bytes { lower: a, upper: b }) => {
                lower <= a && b <= upper
            }
            (Self::Row(expected), Self::Row(actual)) => {
                expected.len() == actual.len()
                    && expected
                        .iter()
                        .all(|(name, ty)| actual.get(name).is_some_and(|value| ty.includes(value)))
            }
            _ => self == actual,
        }
    }
}

pub(super) struct Symbols<'a> {
    core: &'a Value,
    exports: &'a Value,
    declared: BTreeMap<String, Value>,
    live: BTreeSet<String>,
    claimed: BTreeSet<String>,
    work: usize,
}

impl<'a> Symbols<'a> {
    pub(super) fn new(
        core: &'a Value,
        exports: &'a Value,
        declarations: &[Value],
        input: &str,
    ) -> Check<Self> {
        let mut declared = BTreeMap::new();
        for declaration in declarations {
            exact(declaration, &["id", "type", "alphaName"])?;
            text(declaration, "type")?;
            text(declaration, "alphaName")?;
            if declared
                .insert(text(declaration, "id")?.to_owned(), declaration.clone())
                .is_some()
            {
                return Err(());
            }
        }
        if text(declared.get("arg.0").ok_or(())?, "type")? != input {
            return Err(());
        }
        let mut symbols = Self {
            core,
            exports,
            declared,
            live: BTreeSet::from(["arg.0".to_owned()]),
            claimed: BTreeSet::from(["arg.0".to_owned()]),
            work: 65_536,
        };
        symbols.schema(input, 0)?;
        Ok(symbols)
    }

    fn charge(&mut self, depth: usize) -> Check<()> {
        self.work = self.work.checked_sub(1).ok_or(())?;
        if depth > 64 {
            return Err(());
        }
        Ok(())
    }

    pub(super) fn schema(&mut self, name: &str, depth: usize) -> Check<Schema> {
        self.charge(depth)?;
        let word = match name {
            "U8" => Some(u8::MAX.into()),
            "U16" => Some(u16::MAX.into()),
            "U32" => Some(u32::MAX.into()),
            "U64" => Some(u64::MAX),
            _ => None,
        };
        if let Some(max) = word {
            return Ok(Schema::Word(max));
        }
        if let Some(bound) = name
            .strip_prefix("Bytes<")
            .and_then(|name| name.strip_suffix('>'))
        {
            if let Some(value) = bound.strip_prefix("exact=") {
                let size = decimal(value)?;
                return Ok(Schema::Bytes {
                    lower: size,
                    upper: size,
                });
            }
            if let Some(value) = bound.strip_prefix("max=") {
                return Ok(Schema::Bytes {
                    lower: 0,
                    upper: decimal(value)?,
                });
            }
            if let Some((left, right)) = bound
                .strip_prefix("min=")
                .and_then(|value| value.split_once(",max="))
            {
                let lower = decimal(left)?;
                let upper = decimal(right)?;
                if lower < upper {
                    return Ok(Schema::Bytes { lower, upper });
                }
            }
            return Err(());
        }
        let prefix = format!("{}.", text(self.core, "coordinate")?);
        let key = name.strip_prefix(&prefix).unwrap_or(name);
        let value = get(get(self.core, "types")?, key)?;
        match text(value, "kind")? {
            "Bytes" => {
                let upper = number(get(value, "max")?)?;
                let lower = match super::super::map_field(value, "min") {
                    Some(min) => number(min)?,
                    None => 0,
                };
                if lower > upper {
                    return Err(());
                }
                Ok(Schema::Bytes { lower, upper })
            }
            "Int" => self.schema(text(value, "width")?, depth + 1),
            "Nominal" => {
                if text(value, "contract")? != key {
                    return Err(());
                }
                Ok(Schema::Nominal {
                    identity: key.to_owned(),
                    storage: Box::new(self.schema(text(value, "representation")?, depth + 1)?),
                })
            }
            "Record" => {
                let mut fields = BTreeMap::new();
                for (key, value) in members(get(value, "fields")?)? {
                    fields.insert(
                        string(key)?.to_owned(),
                        self.schema(string(value)?, depth + 1)?,
                    );
                }
                Ok(Schema::Row(fields))
            }
            _ => Err(()),
        }
    }

    pub(super) fn input(&self) -> Check<&Value> {
        self.declared.get("arg.0").ok_or(())
    }

    pub(super) fn claim_failure(&mut self, local: &Value, expected: &Schema) -> Check<()> {
        self.claim(local)?;
        if &self.schema(text(local, "type")?, 0)? != expected {
            return Err(());
        }
        Ok(())
    }

    fn claim(&mut self, local: &Value) -> Check<()> {
        let id = text(local, "id")?;
        if self.declared.get(id) != Some(local) || !self.claimed.insert(id.to_owned()) {
            return Err(());
        }
        Ok(())
    }

    pub(super) fn bind(&mut self, local: &Value, actual: Schema) -> Check<()> {
        self.claim(local)?;
        if !self.schema(text(local, "type")?, 0)?.includes(&actual) {
            return Err(());
        }
        self.live.insert(text(local, "id")?.to_owned());
        Ok(())
    }

    pub(super) fn expression(&mut self, value: &Value, depth: usize) -> Check<Schema> {
        self.charge(depth)?;
        match text(value, "kind")? {
            "local" => {
                exact(value, &["kind", "ref"])?;
                let local = get(value, "ref")?;
                let id = text(local, "id")?;
                if !self.live.contains(id) || self.declared.get(id) != Some(local) {
                    return Err(());
                }
                self.schema(text(local, "type")?, 0)
            }
            "field" => {
                exact(value, &["kind", "base", "field"])?;
                let Schema::Row(mut fields) = self.expression(get(value, "base")?, depth + 1)?
                else {
                    return Err(());
                };
                fields.remove(text(value, "field")?).ok_or(())
            }
            "record" => {
                exact(value, &["kind", "fields"])?;
                let mut fields = BTreeMap::new();
                for (key, value) in members(get(value, "fields")?)? {
                    fields.insert(string(key)?.to_owned(), self.expression(value, depth + 1)?);
                }
                Ok(Schema::Row(fields))
            }
            "const" => {
                exact(value, &["kind", "value"])?;
                let literal = get(value, "value")?;
                exact(literal, &["kind", "value", "width"])?;
                let width = text(literal, "width")?;
                if text(literal, "kind")? != "int" || !["U32", "U64"].contains(&width) {
                    return Err(());
                }
                let Schema::Word(max) = self.schema(width, 0)? else {
                    return Err(());
                };
                if number(get(literal, "value")?)? > max {
                    return Err(());
                }
                Ok(Schema::Word(max))
            }
            "if" => {
                exact(value, &["kind", "predicate", "then", "else"])?;
                self.predicate_at(get(value, "predicate")?, depth + 1)?;
                let yes = self.expression(get(value, "then")?, depth + 1)?;
                let no = self.expression(get(value, "else")?, depth + 1)?;
                match (yes, no) {
                    (
                        Schema::Bytes { lower: a, upper: b },
                        Schema::Bytes { lower: c, upper: d },
                    ) => Ok(Schema::Bytes {
                        lower: a.min(c),
                        upper: b.max(d),
                    }),
                    (a, b) if a == b => Ok(a),
                    _ => Err(()),
                }
            }
            "call" => self.call(value, depth),
            _ => Err(()),
        }
    }

    fn call(&mut self, value: &Value, depth: usize) -> Check<Schema> {
        exact(value, &["kind", "callee", "args", "typeArgs"])?;
        let name = text(value, "callee")?;
        let args = super::sequence(value, "args")?;
        let types = super::sequence(value, "typeArgs")?;
        let mut actual = Vec::new();
        for argument in args {
            actual.push(self.expression(argument, depth + 1)?);
        }
        let core = self.core;
        let prefix = format!("{}.", text(core, "coordinate")?);
        let source_definition = name.strip_prefix(&prefix).and_then(|member| {
            super::super::map_field(core, "functions")
                .and_then(|functions| super::super::map_field(functions, member))
        });
        if let Some(definition) = source_definition {
            let parameters = super::sequence(definition, "params")?;
            if !types.is_empty() || actual.len() != parameters.len() {
                return Err(());
            }
            for (parameter, argument) in parameters.iter().zip(&actual) {
                if !self.schema(text(parameter, "type")?, 0)?.includes(argument) {
                    return Err(());
                }
            }
            return self.schema(text(definition, "returnType")?, 0);
        }
        let exports = self.exports;
        if let Some(function) = super::sequence(exports, "pureFunctions")?
            .iter()
            .find(|function| text(function, "coordinate") == Ok(name))
        {
            if !types.is_empty()
                || !args.is_empty()
                || !super::sequence(function, "parameterTypes")?.is_empty()
            {
                return Err(());
            }
            return self.schema(text(function, "returnType")?, 0);
        }
        let mut declared = Vec::new();
        for coordinate in types {
            declared.push(self.schema(string(coordinate)?, 0)?);
        }
        match (name, actual.as_slice(), declared.as_slice()) {
            ("core.bytes.length", [actual], [expected @ Schema::Bytes { .. }])
                if expected.includes(actual) =>
            {
                Ok(Schema::Word(u64::MAX))
            }
            (
                "core.bytes.concat",
                [left, right],
                [a @ Schema::Bytes {
                    lower: a_min,
                    upper: a_max,
                }, b @ Schema::Bytes {
                    lower: b_min,
                    upper: b_max,
                }],
            ) if a.includes(left) && b.includes(right) => Ok(Schema::Bytes {
                lower: a_min.checked_add(*b_min).ok_or(())?,
                upper: a_max.checked_add(*b_max).ok_or(())?,
            }),
            _ => Err(()),
        }
    }

    pub(super) fn predicate(&mut self, value: &Value) -> Check<()> {
        self.predicate_at(value, 0)
    }

    fn predicate_at(&mut self, value: &Value, depth: usize) -> Check<()> {
        self.charge(depth)?;
        exact(value, &["kind", "op", "left", "right"])?;
        if text(value, "kind")? != "compare" {
            return Err(());
        }
        let left = self.expression(get(value, "left")?, depth + 1)?;
        let right = self.expression(get(value, "right")?, depth + 1)?;
        if (matches!(left, Schema::Nominal { .. }) || matches!(right, Schema::Nominal { .. }))
            && left != right
        {
            return Err(());
        }
        match (
            text(value, "op")?,
            left.representation(),
            right.representation(),
        ) {
            ("==", Schema::Bytes { .. }, Schema::Bytes { .. }) => Ok(()),
            ("==" | "<=", Schema::Word(a), Schema::Word(b)) if a == b => Ok(()),
            _ => Err(()),
        }
    }

    pub(super) fn finish(&mut self, result: &Value, output: &str) -> Check<()> {
        let actual = self.expression(result, 0)?;
        if !self.schema(output, 0)?.includes(&actual) || self.claimed.len() != self.declared.len() {
            return Err(());
        }
        Ok(())
    }
}

fn decimal(value: &str) -> Check<u64> {
    let number: u64 = value.parse().map_err(|_| ())?;
    if number.to_string() != value {
        return Err(());
    }
    Ok(number)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
        Value::Map(
            fields
                .into_iter()
                .map(|(key, value)| (Value::Text(key.into()), value))
                .collect(),
        )
    }
    #[test]
    fn distinct_nominals_without_source_functions_do_not_convert() -> Check<()> {
        let nominal = |contract: &str| {
            map([
                ("kind", Value::Text("Nominal".into())),
                ("contract", Value::Text(contract.into())),
                ("representation", Value::Text("U64".into())),
            ])
        };
        let core = map([
            ("coordinate", Value::Text("example@1".into())),
            ("types", map([("A", nominal("A")), ("B", nominal("B"))])),
        ]);
        let mut symbols = Symbols {
            core: &core,
            exports: &core,
            declared: BTreeMap::new(),
            live: BTreeSet::new(),
            claimed: BTreeSet::new(),
            work: 65_536,
        };
        let a = symbols.schema("A", 0)?;
        assert!(symbols.schema("example@1.A", 0)?.includes(&a));
        assert!(!symbols.schema("B", 0)?.includes(&a));
        assert!(!symbols.schema("U64", 0)?.includes(&a));
        Ok(())
    }
}
