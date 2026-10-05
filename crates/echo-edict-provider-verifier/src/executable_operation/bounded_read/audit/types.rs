// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned bounded type expansion and local declaration checking.

use super::super::{exact, get, members, number, string, text, Check, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Schema {
    Bytes { lower: u64, upper: u64 },
    Word(u64),
    Row(BTreeMap<String, Schema>),
}

impl Schema {
    pub(super) fn address(&self) -> bool {
        let Self::Row(fields) = self else {
            return false;
        };
        fields.len() == 3
            && ["nodeId", "warpId", "typeId"].iter().all(|key| {
                fields.get(*key)
                    == Some(&Self::Bytes {
                        lower: 32,
                        upper: 32,
                    })
            })
    }
    fn includes(&self, actual: &Self) -> bool {
        match (self, actual) {
            (Self::Bytes { lower, upper }, Self::Bytes { lower: a, upper: b }) => {
                lower <= a && b <= upper
            }
            _ => self == actual,
        }
    }
}

pub(super) struct Symbols<'a> {
    core: &'a Value,
    declared: BTreeMap<String, Value>,
    live: BTreeSet<String>,
    claimed: BTreeSet<String>,
    work: usize,
}

impl<'a> Symbols<'a> {
    pub(super) fn new(core: &'a Value, declarations: &[Value], input: &str) -> Check<Self> {
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
            return Err(());
        }
        let prefix = format!("{}.", text(self.core, "coordinate")?);
        let value = get(
            get(self.core, "types")?,
            name.strip_prefix(&prefix).unwrap_or(name),
        )?;
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
            _ => Err(()),
        }
    }

    pub(super) fn predicate(&mut self, value: &Value) -> Check<()> {
        exact(value, &["kind", "op", "left", "right"])?;
        if text(value, "kind")? != "compare" {
            return Err(());
        }
        let left = self.expression(get(value, "left")?, 0)?;
        let right = self.expression(get(value, "right")?, 0)?;
        match (text(value, "op")?, left, right) {
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
