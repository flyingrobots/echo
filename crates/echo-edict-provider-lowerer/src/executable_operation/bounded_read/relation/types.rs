// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded structural types and exact local identities for atom-read lowering.

use super::{
    entries, field, invalid, require_exact_fields, required_u64, text, ProviderRefusalV1, Value,
    SUBJECT,
};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ReadType {
    Bytes(u64, u64),
    Unsigned(u64),
    Record(BTreeMap<String, ReadType>),
}

impl ReadType {
    pub(super) fn is_address(&self) -> bool {
        let Self::Record(fields) = self else {
            return false;
        };
        fields.len() == 3
            && ["nodeId", "typeId", "warpId"]
                .iter()
                .all(|key| fields.get(*key) == Some(&Self::Bytes(32, 32)))
    }

    fn accepts(&self, actual: &Self) -> bool {
        match (self, actual) {
            (Self::Bytes(min, max), Self::Bytes(actual_min, actual_max)) => {
                min <= actual_min && actual_max <= max
            }
            (Self::Record(expected), Self::Record(actual)) => {
                expected.len() == actual.len()
                    && expected
                        .iter()
                        .all(|(name, ty)| actual.get(name).is_some_and(|value| ty.accepts(value)))
            }
            _ => self == actual,
        }
    }
}

pub(super) struct Scope<'a> {
    core: &'a Value,
    exports: &'a Value,
    inventory: BTreeMap<&'a str, &'a Value>,
    available: BTreeMap<&'a str, &'a Value>,
    used: BTreeSet<&'a str>,
    input: &'a Value,
    remaining_type_work: Cell<usize>,
}

impl<'a> Scope<'a> {
    pub(super) fn new(
        core: &'a Value,
        exports: &'a Value,
        locals: &'a [Value],
        input_type: &str,
    ) -> Result<Self, ProviderRefusalV1> {
        let mut inventory = BTreeMap::new();
        for local in locals {
            require_exact_fields(local, &["id", "type", "alphaName"], SUBJECT)?;
            text(local, "alphaName")?;
            text(local, "type")?;
            if inventory.insert(text(local, "id")?, local).is_some() {
                return Err(invalid());
            }
        }
        let input = *inventory.get("arg.0").ok_or_else(invalid)?;
        if text(input, "type")? != input_type {
            return Err(invalid());
        }
        let scope = Self {
            core,
            exports,
            inventory,
            available: BTreeMap::from([("arg.0", input)]),
            used: BTreeSet::from(["arg.0"]),
            input,
            remaining_type_work: Cell::new(65_536),
        };
        scope.ty(input_type)?;
        Ok(scope)
    }

    pub(super) fn input(&self) -> &Value {
        self.input
    }

    pub(super) fn ty(&self, coordinate: &str) -> Result<ReadType, ProviderRefusalV1> {
        self.resolve(coordinate, 0)
    }

    fn resolve(&self, coordinate: &str, depth: usize) -> Result<ReadType, ProviderRefusalV1> {
        if depth > 64 {
            return Err(invalid());
        }
        let remaining = self
            .remaining_type_work
            .get()
            .checked_sub(1)
            .ok_or_else(invalid)?;
        self.remaining_type_work.set(remaining);
        let word = match coordinate {
            "U8" => Some(u64::from(u8::MAX)),
            "U16" => Some(u64::from(u16::MAX)),
            "U32" => Some(u64::from(u32::MAX)),
            "U64" => Some(u64::MAX),
            _ => None,
        };
        if let Some(max) = word {
            return Ok(ReadType::Unsigned(max));
        }
        if let Some(spec) = coordinate
            .strip_prefix("Bytes<")
            .and_then(|rest| rest.strip_suffix('>'))
        {
            if let Some(exact) = spec.strip_prefix("exact=") {
                let exact = canonical_number(exact)?;
                return Ok(ReadType::Bytes(exact, exact));
            }
            if let Some(max) = spec.strip_prefix("max=") {
                return Ok(ReadType::Bytes(0, canonical_number(max)?));
            }
            if let Some((left, right)) = spec
                .strip_prefix("min=")
                .and_then(|value| value.split_once(",max="))
            {
                let min = canonical_number(left)?;
                let max = canonical_number(right)?;
                if min < max {
                    return Ok(ReadType::Bytes(min, max));
                }
            }
            return Err(invalid());
        }
        let prefix = format!("{}.", text(self.core, "coordinate")?);
        let key = coordinate.strip_prefix(&prefix).unwrap_or(coordinate);
        let value = field(field(self.core, "types")?, key)?;
        match text(value, "kind")? {
            "Bytes" => {
                let max = required_u64(value, "max", SUBJECT)?;
                let min = match super::super::map_field(value, "min") {
                    Some(_) => required_u64(value, "min", SUBJECT)?,
                    None => 0,
                };
                if min > max {
                    return Err(invalid());
                }
                Ok(ReadType::Bytes(min, max))
            }
            "Int" => self.resolve(text(value, "width")?, depth + 1),
            "Nominal" => self.resolve(text(value, "representation")?, depth + 1),
            "Record" => {
                let fields = entries(field(value, "fields")?)?
                    .iter()
                    .map(|(key, coordinate)| {
                        let (Value::Text(key), Value::Text(coordinate)) = (key, coordinate) else {
                            return Err(invalid());
                        };
                        if key.len() > 1024 || coordinate.len() > 1024 {
                            return Err(invalid());
                        }
                        Ok((key.clone(), self.resolve(coordinate, depth + 1)?))
                    })
                    .collect::<Result<BTreeMap<_, _>, ProviderRefusalV1>>()?;
                Ok(ReadType::Record(fields))
            }
            _ => Err(invalid()),
        }
    }

    pub(super) fn introduce(
        &mut self,
        local: &'a Value,
        actual: Option<ReadType>,
    ) -> Result<(), ProviderRefusalV1> {
        self.claim(local)?;
        let expected = self.ty(text(local, "type")?)?;
        if actual.is_some_and(|actual| !expected.accepts(&actual)) {
            return Err(invalid());
        }
        self.available.insert(text(local, "id")?, local);
        Ok(())
    }

    fn claim(&mut self, local: &'a Value) -> Result<(), ProviderRefusalV1> {
        let id = text(local, "id")?;
        if self.inventory.get(id).copied() != Some(local) || !self.used.insert(id) {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn obstruction_binder(
        &mut self,
        local: &'a Value,
        address: &ReadType,
    ) -> Result<(), ProviderRefusalV1> {
        self.claim(local)?;
        if &self.ty(text(local, "type")?)? != address {
            return Err(invalid());
        }
        // Failure locals are checked but never introduced into the success scope.
        Ok(())
    }

    pub(super) fn expression(
        &self,
        value: &Value,
        depth: usize,
    ) -> Result<ReadType, ProviderRefusalV1> {
        if depth > 64 {
            return Err(invalid());
        }
        match text(value, "kind")? {
            "local" => {
                require_exact_fields(value, &["kind", "ref"], SUBJECT)?;
                let local = field(value, "ref")?;
                if self.available.get(text(local, "id")?).copied() != Some(local) {
                    return Err(invalid());
                }
                self.ty(text(local, "type")?)
            }
            "field" => {
                require_exact_fields(value, &["kind", "base", "field"], SUBJECT)?;
                let ReadType::Record(mut fields) =
                    self.expression(field(value, "base")?, depth + 1)?
                else {
                    return Err(invalid());
                };
                fields.remove(text(value, "field")?).ok_or_else(invalid)
            }
            "record" => {
                require_exact_fields(value, &["kind", "fields"], SUBJECT)?;
                let fields = entries(field(value, "fields")?)?
                    .iter()
                    .map(|(key, value)| {
                        let Value::Text(key) = key else {
                            return Err(invalid());
                        };
                        Ok((key.clone(), self.expression(value, depth + 1)?))
                    })
                    .collect::<Result<BTreeMap<_, _>, ProviderRefusalV1>>()?;
                Ok(ReadType::Record(fields))
            }
            "const" => {
                require_exact_fields(value, &["kind", "value"], SUBJECT)?;
                let literal = field(value, "value")?;
                require_exact_fields(literal, &["kind", "value", "width"], SUBJECT)?;
                let width = text(literal, "width")?;
                if text(literal, "kind")? != "int" || !["U32", "U64"].contains(&width) {
                    return Err(invalid());
                }
                let ReadType::Unsigned(max) = self.ty(width)? else {
                    return Err(invalid());
                };
                if required_u64(literal, "value", SUBJECT)? > max {
                    return Err(invalid());
                }
                Ok(ReadType::Unsigned(max))
            }
            "if" => {
                require_exact_fields(value, &["kind", "predicate", "then", "else"], SUBJECT)?;
                self.predicate_at(field(value, "predicate")?, depth + 1)?;
                let yes = self.expression(field(value, "then")?, depth + 1)?;
                let no = self.expression(field(value, "else")?, depth + 1)?;
                match (yes, no) {
                    (ReadType::Bytes(a, b), ReadType::Bytes(c, d)) => {
                        Ok(ReadType::Bytes(a.min(c), b.max(d)))
                    }
                    (a, b) if a == b => Ok(a),
                    _ => Err(invalid()),
                }
            }
            "call" => self.call(value, depth),
            _ => Err(invalid()),
        }
    }

    fn call(&self, value: &Value, depth: usize) -> Result<ReadType, ProviderRefusalV1> {
        require_exact_fields(value, &["kind", "callee", "args", "typeArgs"], SUBJECT)?;
        let args = super::array(value, "args")?;
        let types = super::array(value, "typeArgs")?;
        let name = text(value, "callee")?;
        let actual = args
            .iter()
            .map(|value| self.expression(value, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        let prefix = format!("{}.", text(self.core, "coordinate")?);
        let signature = if let Some(member) = name.strip_prefix(&prefix) {
            Some(field(field(self.core, "functions")?, member)?)
        } else {
            None
        };
        if let Some(function) = signature {
            let params = super::array(function, "params")?;
            if !types.is_empty() || params.len() != actual.len() {
                return Err(invalid());
            }
            for (parameter, ty) in params.iter().zip(&actual) {
                if !self.ty(text(parameter, "type")?)?.accepts(ty) {
                    return Err(invalid());
                }
            }
            return self.ty(text(function, "returnType")?);
        }
        if let Some(function) = super::array(self.exports, "pureFunctions")?
            .iter()
            .find(|function| super::text_field(function, "coordinate") == Some(name))
        {
            if !types.is_empty()
                || !args.is_empty()
                || !super::array(function, "parameterTypes")?.is_empty()
            {
                return Err(invalid());
            }
            return self.ty(text(function, "returnType")?);
        }
        let declared = types
            .iter()
            .map(|value| {
                let Value::Text(name) = value else {
                    return Err(invalid());
                };
                self.ty(name)
            })
            .collect::<Result<Vec<_>, _>>()?;
        match (name, actual.as_slice(), declared.as_slice()) {
            ("core.bytes.length", [actual], [expected @ ReadType::Bytes(_, _)])
                if expected.accepts(actual) =>
            {
                Ok(ReadType::Unsigned(u64::MAX))
            }
            (
                "core.bytes.concat",
                [left, right],
                [a @ ReadType::Bytes(a_min, a_max), b @ ReadType::Bytes(b_min, b_max)],
            ) if a.accepts(left) && b.accepts(right) => Ok(ReadType::Bytes(
                a_min.checked_add(*b_min).ok_or_else(invalid)?,
                a_max.checked_add(*b_max).ok_or_else(invalid)?,
            )),
            _ => Err(invalid()),
        }
    }

    pub(super) fn predicate(&self, value: &Value) -> Result<(), ProviderRefusalV1> {
        self.predicate_at(value, 0)
    }

    fn predicate_at(&self, value: &Value, depth: usize) -> Result<(), ProviderRefusalV1> {
        if depth > 64 {
            return Err(invalid());
        }
        require_exact_fields(value, &["kind", "op", "left", "right"], SUBJECT)?;
        if text(value, "kind")? != "compare" {
            return Err(invalid());
        }
        let left = self.expression(field(value, "left")?, depth + 1)?;
        let right = self.expression(field(value, "right")?, depth + 1)?;
        match (text(value, "op")?, left, right) {
            ("==", ReadType::Bytes(_, _), ReadType::Bytes(_, _)) => Ok(()),
            ("==" | "<=", ReadType::Unsigned(left), ReadType::Unsigned(right)) if left == right => {
                Ok(())
            }
            _ => Err(invalid()),
        }
    }

    pub(super) fn matches_output(
        &self,
        expression: &Value,
        coordinate: &str,
    ) -> Result<(), ProviderRefusalV1> {
        if !self
            .ty(coordinate)?
            .accepts(&self.expression(expression, 0)?)
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn finish(&self) -> Result<(), ProviderRefusalV1> {
        if self.used.len() != self.inventory.len() {
            return Err(invalid());
        }
        Ok(())
    }
}

fn canonical_number(value: &str) -> Result<u64, ProviderRefusalV1> {
    let number: u64 = value.parse().map_err(|_| invalid())?;
    if number.to_string() != value {
        return Err(invalid());
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
                .map(|(key, value)| (Value::Text(key.to_owned()), value))
                .collect(),
        )
    }

    #[test]
    fn named_types_starting_with_u_are_not_unsigned_widths() {
        let core = map([
            ("coordinate", Value::Text("Ucorp.data@1".to_owned())),
            (
                "types",
                map([(
                    "UserBytes",
                    map([
                        ("kind", Value::Text("Bytes".to_owned())),
                        ("max", Value::Integer(32)),
                    ]),
                )]),
            ),
        ]);
        let scope = Scope {
            core: &core,
            exports: &core,
            inventory: BTreeMap::new(),
            available: BTreeMap::new(),
            used: BTreeSet::new(),
            input: &core,
            remaining_type_work: Cell::new(65_536),
        };
        for name in ["UserBytes", "Ucorp.data@1.UserBytes"] {
            assert_eq!(scope.ty(name), Ok(ReadType::Bytes(0, 32)), "{name}");
        }
        for (name, max) in [
            ("U8", u64::from(u8::MAX)),
            ("U16", u64::from(u16::MAX)),
            ("U32", u64::from(u32::MAX)),
            ("U64", u64::MAX),
        ] {
            assert_eq!(scope.ty(name), Ok(ReadType::Unsigned(max)), "{name}");
        }
        for name in ["U0", "U128", "Ucorp.data@1.Unknown"] {
            assert!(scope.ty(name).is_err(), "{name}");
        }
    }
}
