// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent verifier case inventory, lexical frames and resource summary.
use super::{
    exact, get, sequence, string, Charge, Check, Checker, Expression, Frame, Schema, Value,
};
use std::collections::{BTreeMap, BTreeSet};

impl<'a> Checker<'a, '_> {
    pub(super) fn audit_match(
        &mut self,
        value: &'a Value,
        frame: &Frame<'a>,
        depth: usize,
    ) -> Check<Expression> {
        exact(value, &["kind", "scrutinee", "arms"])?;
        let input = self.expression(get(value, "scrutinee")?, frame, depth + 1)?;
        let Schema::Alternatives { cases, .. } = &input.schema else {
            return Err(());
        };
        let arms = sequence(get(value, "arms")?)?;
        let mut remaining: BTreeSet<_> = cases.keys().map(String::as_str).collect();
        let mut result: Option<Schema> = None;
        let mut branch_charge = Charge::default();
        let mut height = input.height + 1;
        for (index, arm) in arms.iter().enumerate() {
            self.types.step(depth + 1)?;
            let tag = string(get(arm, "case")?)?;
            if !remaining.remove(tag) {
                return Err(());
            }
            let binder = super::super::map_field(arm, "binder");
            exact(
                arm,
                if binder.is_none() {
                    &["case", "body"]
                } else {
                    &["case", "body", "binder"]
                },
            )?;
            let mut inner = Frame {
                declared: BTreeMap::new(),
                values: BTreeMap::new(),
            };
            let mut cost = Charge {
                ticks: (index as u128).checked_add(1).ok_or(())?,
                storage: 0,
            };
            for (id, (declaration, schema)) in &frame.values {
                let copy = self.types.copy_type(schema)?;
                if binder.is_some() {
                    cost.append(copy.measure()?.copy)?;
                }
                inner.declared.insert(*id, *declaration);
                inner.values.insert(*id, (*declaration, copy));
            }
            if let Some(binder) = binder {
                let expected = cases.get(tag).and_then(Option::as_ref).ok_or(())?;
                let actual = self.types.local(binder)?;
                if !expected.encloses(&actual) || !actual.encloses(expected) {
                    return Err(());
                }
                let id = string(get(binder, "id")?)?;
                if inner.declared.insert(id, binder).is_some() {
                    return Err(());
                }
                inner.define(binder, actual)?;
            }
            let body = self.expression(get(arm, "body")?, &inner, depth + 1)?;
            result = Some(match result {
                None => body.schema,
                Some(previous) => previous.choice(body.schema)?,
            });
            height = height.max(body.height + 1);
            cost.append(body.charge)?;
            branch_charge = Charge::choice(branch_charge, cost);
        }
        if !remaining.is_empty() {
            return Err(());
        }
        let mut charge = input.charge;
        charge.append(Charge {
            ticks: 1,
            storage: 0,
        })?;
        charge.append(branch_charge)?;
        Ok(Expression {
            schema: result.ok_or(())?,
            charge,
            height,
        })
    }
}
