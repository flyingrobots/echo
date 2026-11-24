// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the verifier's audit of Core ordered records
//! (`orderedRecord`): fields evaluated in authored order, charged as the
//! unordered `record` that builds the same canonical value. A `fail` field
//! takes its schema from the declared type of the enclosing helper result or
//! binding, and is refused anywhere that type is not known.
use super::{
    exact, get, sequence, string, Arm, Charge, Check, Checker, Expression, Frame, Schema, Value,
};
use std::collections::BTreeMap;

impl<'a> Checker<'a, '_> {
    /// Audits a helper result or binding against its declared schema, which
    /// types a `fail`, including a `fail` field of an ordered record.
    pub(super) fn declared_arm(
        &mut self,
        value: &'a Value,
        frame: &Frame<'a>,
        declared: &Schema,
    ) -> Check<Arm> {
        if string(get(value, "kind")?)? != "orderedRecord" {
            return self.arm(value, frame, 0);
        }
        self.types.step(0)?;
        let record = self.ordered_record(value, frame, 0, Some(declared))?;
        Ok(Arm {
            schema: Some(record.schema),
            charge: record.charge,
            height: record.height,
        })
    }

    /// Audits an `orderedRecord` node, whose step the caller has taken;
    /// `expected` types its `fail` fields.
    pub(super) fn ordered_record(
        &mut self,
        value: &'a Value,
        frame: &Frame<'a>,
        depth: usize,
        expected: Option<&Schema>,
    ) -> Check<Expression> {
        exact(value, &["kind", "fields"])?;
        let entries = sequence(get(value, "fields")?)?;
        if entries.is_empty() {
            return Err(());
        }
        let expected = match expected {
            Some(Schema::Nominal { representation, .. }) => Some(representation.as_ref()),
            other => other,
        };
        let mut charge = Charge {
            ticks: 1,
            storage: 64,
        };
        let mut height = 0;
        let mut fields = BTreeMap::new();
        for entry in entries {
            exact(entry, &["name", "value"])?;
            let name = string(get(entry, "name")?)?;
            let want = match expected {
                Some(Schema::Fields(types)) => types.get(name),
                _ => None,
            };
            let child = get(entry, "value")?;
            let (schema, child_charge, child_height) =
                if string(get(child, "kind")?)? == "orderedRecord" {
                    self.types.step(depth + 1)?;
                    let record = self.ordered_record(child, frame, depth + 1, want)?;
                    (record.schema, record.charge, record.height)
                } else {
                    let arm = self.arm(child, frame, depth + 1)?;
                    let schema = match arm.schema {
                        Some(schema) => schema,
                        None => self.types.copy_type(want.ok_or(())?)?,
                    };
                    (schema, arm.charge, arm.height)
                };
            height = height.max(child_height + 1);
            charge.append(child_charge)?;
            charge.append(Charge {
                ticks: 0,
                storage: 64 + name.len() as u128,
            })?;
            if fields.insert(name.to_owned(), schema).is_some() {
                return Err(());
            }
        }
        Ok(Expression {
            schema: Schema::Fields(fields),
            charge,
            height,
        })
    }
}
