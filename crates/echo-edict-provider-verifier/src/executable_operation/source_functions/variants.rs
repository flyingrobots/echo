// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned variant graph and constructor audit; no lowerer code is reused.
use super::types::Size;
use super::{
    exact, get, members, string, Charge, Check, Checker, Expression, Frame, Schema, Types, Value,
};
use std::collections::BTreeMap;

impl Types<'_> {
    pub(super) fn alternatives(
        &mut self,
        definition: &Value,
        key: &str,
        prefix: &str,
        depth: usize,
    ) -> Check<Schema> {
        exact(definition, &["kind", "cases"])?;
        let mut cases = BTreeMap::new();
        for (name, entry) in members(get(definition, "cases")?)? {
            self.proof_work(1)?;
            let name = string(name)?;
            if !name.bytes().enumerate().all(|(i, b)| {
                b == b'_' || b.is_ascii_alphabetic() || (i != 0 && b.is_ascii_digit())
            }) {
                return Err(());
            }
            let child = if let Some(reference) = super::super::map_field(entry, "payload") {
                exact(entry, &["payload"])?;
                Some(self.resolve(string(reference)?, depth + 1)?)
            } else {
                exact(entry, &[])?;
                None
            };
            if cases.insert(name.to_owned(), child).is_some() {
                return Err(());
            }
        }
        if cases.is_empty() {
            return Err(());
        }
        let identity = if key.contains('@') {
            key.to_owned()
        } else {
            format!("{prefix}{key}")
        };
        let schema = Schema::Alternatives { identity, cases };
        schema.measure()?;
        Ok(schema)
    }
}

impl<'a> Checker<'a, '_> {
    pub(super) fn variant_constructor(
        &mut self,
        value: &'a Value,
        frame: &Frame<'a>,
        depth: usize,
    ) -> Check<Expression> {
        let payload = super::super::map_field(value, "payload");
        exact(
            value,
            if payload.is_none() {
                &["kind", "type", "case"]
            } else {
                &["kind", "type", "case", "payload"]
            },
        )?;
        let schema = self.types.resolve(string(get(value, "type")?)?, 0)?;
        let Schema::Alternatives { identity, cases } = &schema else {
            return Err(());
        };
        let tag = string(get(value, "case")?)?;
        let expected = cases.get(tag).ok_or(())?;
        let mut charge = metadata(identity, tag).measure()?.copy;
        charge.ticks = 3;
        let mut height = 1;
        match (expected, payload) {
            (None, None) => {}
            (Some(ty), Some(value)) => {
                let child = self.expression(value, frame, depth + 1)?;
                if !ty.encloses(&child.schema) {
                    return Err(());
                }
                charge.append(child.charge)?;
                charge.append(Charge {
                    ticks: 0,
                    storage: 64 + 7,
                })?;
                height = height.max(child.height + 1);
            }
            _ => return Err(()),
        }
        Ok(Expression {
            schema,
            charge,
            height,
        })
    }
}

fn metadata(identity: &str, tag: &str) -> Schema {
    Schema::Fields(BTreeMap::from([
        (
            "type".to_owned(),
            Schema::Blob {
                low: identity.len() as u64,
                high: identity.len() as u64,
            },
        ),
        (
            "case".to_owned(),
            Schema::Blob {
                low: tag.len() as u64,
                high: tag.len() as u64,
            },
        ),
    ]))
}

pub(super) fn measure(identity: &str, cases: &BTreeMap<String, Option<Schema>>) -> Check<Size> {
    let mut maximum: Option<Size> = None;
    for (tag, child) in cases {
        let mut size = metadata(identity, tag).measure()?;
        let child = child.as_ref().unwrap_or(&Schema::Truth).measure()?;
        size.copy.append(child.copy)?;
        size.copy.append(Charge {
            ticks: 1,
            storage: 64 + 7,
        })?;
        size.checks = size.checks.checked_add(child.checks).ok_or(())?;
        size.encoded = size
            .encoded
            .checked_add(9 + 7)
            .and_then(|n| n.checked_add(child.encoded))
            .ok_or(())?;
        size.height = size.height.max(child.height + 1);
        maximum = Some(match maximum {
            None => size,
            Some(old) => Size {
                copy: Charge::choice(old.copy, size.copy),
                checks: old.checks.max(size.checks),
                encoded: old.encoded.max(size.encoded),
                height: old.height.max(size.height),
            },
        });
    }
    let size = maximum.ok_or(())?;
    if size.copy.storage > u128::from(u64::MAX) || size.encoded > u128::from(u64::MAX) {
        return Err(());
    }
    Ok(size)
}

pub(super) fn type_nodes(schema: &Schema) -> Check<usize> {
    fn count(schema: &Schema) -> Option<usize> {
        match schema {
            Schema::Fields(fields) => fields.values().try_fold(1_usize, |n, child| {
                n.checked_add(1)?.checked_add(count(child)?)
            }),
            Schema::Alternatives { cases, .. } => cases.values().try_fold(1_usize, |n, child| {
                n.checked_add(1)?
                    .checked_add(child.as_ref().map_or(Some(0), count)?)
            }),
            Schema::Nominal { representation, .. } => count(representation)?.checked_add(1),
            _ => Some(1),
        }
    }
    count(schema).filter(|n| *n <= 65_536).ok_or(())
}
