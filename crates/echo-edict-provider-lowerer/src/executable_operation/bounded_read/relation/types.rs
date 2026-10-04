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
            _ => self == actual,
        }
    }
}

pub(super) struct Scope<'a> {
    core: &'a Value,
    inventory: BTreeMap<&'a str, &'a Value>,
    available: BTreeMap<&'a str, &'a Value>,
    used: BTreeSet<&'a str>,
    input: &'a Value,
    remaining_type_work: Cell<usize>,
}

impl<'a> Scope<'a> {
    pub(super) fn new(
        core: &'a Value,
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
        if let Some(width) = coordinate.strip_prefix('U') {
            return match width {
                "8" => Ok(ReadType::Unsigned(u8::MAX.into())),
                "16" => Ok(ReadType::Unsigned(u16::MAX.into())),
                "32" => Ok(ReadType::Unsigned(u32::MAX.into())),
                "64" => Ok(ReadType::Unsigned(u64::MAX)),
                _ => Err(invalid()),
            };
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
            _ => Err(invalid()),
        }
    }

    pub(super) fn predicate(&self, value: &Value) -> Result<(), ProviderRefusalV1> {
        require_exact_fields(value, &["kind", "op", "left", "right"], SUBJECT)?;
        if text(value, "kind")? != "compare" {
            return Err(invalid());
        }
        let left = self.expression(field(value, "left")?, 0)?;
        let right = self.expression(field(value, "right")?, 0)?;
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
