// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the lowerer's judgment of Core ordered records
//! (`orderedRecord`): fields evaluated in authored order and costed as the
//! unordered `record` that builds the same canonical value. A `fail` field
//! takes its type from the declared type of the enclosing helper result or
//! binding, and is refused anywhere that type is not known.
use super::{exact, get, list, string, Arm, Check, Cost, Fact, Judgment, Scope, Ty, Value};
use std::collections::BTreeMap;

impl<'a> Judgment<'a> {
    /// Judges a helper result or binding whose declared type is `declared`,
    /// which types a `fail`, including a `fail` field of an ordered record.
    pub(super) fn declared_arm(
        &mut self,
        value: &'a Value,
        scope: &Scope<'a>,
        source: bool,
        prefix: usize,
        declared: &str,
    ) -> Check<Arm> {
        if string(get(value, "kind")?)? != "orderedRecord" {
            return self.arm(value, scope, source, prefix);
        }
        let expected = self.ty(declared, 0)?;
        self.charge(prefix)?;
        let fact = self.ordered_record(value, scope, source, prefix, Some(&expected))?;
        Ok(Arm {
            ty: Some(fact.ty),
            cost: fact.cost,
            depth: fact.depth,
        })
    }

    /// Judges an `orderedRecord` node, whose step the caller has charged;
    /// `expected` types its `fail` fields.
    pub(super) fn ordered_record(
        &mut self,
        value: &'a Value,
        scope: &Scope<'a>,
        source: bool,
        prefix: usize,
        expected: Option<&Ty>,
    ) -> Check<Fact> {
        exact(value, &["kind", "fields"])?;
        let entries = list(get(value, "fields")?)?;
        if entries.is_empty() {
            return Err(());
        }
        let expected = match expected.map(Ty::representation) {
            Some(Ty::Record(fields)) => Some(fields),
            _ => None,
        };
        let mut cost = Cost {
            steps: 1,
            bytes: 64,
        };
        let mut depth = 0;
        let mut fields = BTreeMap::new();
        for entry in entries {
            exact(entry, &["name", "value"])?;
            let name = string(get(entry, "name")?)?;
            let want = expected.and_then(|types| types.get(name));
            let child = get(entry, "value")?;
            let (ty, child_cost, child_depth) = if string(get(child, "kind")?)? == "orderedRecord" {
                self.charge(prefix + 1)?;
                let fact = self.ordered_record(child, scope, source, prefix + 1, want)?;
                (fact.ty, fact.cost, fact.depth)
            } else {
                let arm = self.arm(child, scope, source, prefix + 1)?;
                let ty = match arm.ty {
                    Some(ty) => ty,
                    None => want.cloned().ok_or(())?,
                };
                (ty, arm.cost, arm.depth)
            };
            cost = cost.plus(child_cost)?.plus(Cost {
                steps: 0,
                bytes: 64_u64.checked_add(name.len() as u64).ok_or(())?,
            })?;
            depth = depth.max(child_depth + 1);
            if fields.insert(name.to_owned(), ty).is_some() {
                return Err(());
            }
        }
        Ok(Fact {
            ty: Ty::Record(fields),
            cost,
            depth,
        })
    }
}
