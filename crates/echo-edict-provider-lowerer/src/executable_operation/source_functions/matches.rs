// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned exhaustive dispatch, lexical scope and worst-arm accounting.
use super::{
    exact, get, includes, list, shape, string, union, variants, Check, Cost, Fact, Judgment, Scope,
    Ty, Value,
};
use std::collections::BTreeSet;

impl<'a> Judgment<'a> {
    pub(super) fn variant_match(
        &mut self,
        value: &'a Value,
        scope: &Scope<'a>,
        source: bool,
        depth: usize,
    ) -> Check<Fact> {
        exact(value, &["kind", "scrutinee", "arms"])?;
        let scrutinee = self.expression(get(value, "scrutinee")?, scope, source, depth + 1)?;
        let Ty::Variant { cases, .. } = &scrutinee.ty else {
            return Err(());
        };
        let arms = list(get(value, "arms")?)?;
        if arms.len() != cases.len() {
            return Err(());
        }
        let mut seen = BTreeSet::new();
        let mut joined = None;
        let mut worst = Cost::default();
        let mut height = scrutinee.depth + 1;
        for (index, arm) in arms.iter().enumerate() {
            self.charge(depth + 1)?;
            let tag = string(get(arm, "case")?)?;
            if !seen.insert(tag) {
                return Err(());
            }
            let payload = cases.get(tag).ok_or(())?;
            let binder = super::super::map_field(arm, "binder");
            exact(
                arm,
                if binder.is_some() {
                    &["case", "binder", "body"]
                } else {
                    &["case", "body"]
                },
            )?;
            let mut copies = Cost::default();
            for (_, ty) in scope.values() {
                self.work = self.work.checked_sub(variants::type_nodes(ty)?).ok_or(())?;
                if binder.is_some() {
                    copies = copies.plus(shape(ty)?.0)?;
                }
            }
            let mut inner = scope.clone();
            if let Some(binder) = binder {
                let expected = payload.as_ref().ok_or(())?;
                let actual = self.declaration(binder, &mut inner)?;
                if !includes(expected, &actual) || !includes(&actual, expected) {
                    return Err(());
                }
            }
            let body = self.expression(get(arm, "body")?, &inner, source, depth + 1)?;
            height = height.max(body.depth + 1);
            joined = Some(match joined {
                None => body.ty,
                Some(previous) => union(previous, body.ty)?,
            });
            let dispatch = Cost {
                steps: u64::try_from(index)
                    .map_err(|_| ())?
                    .checked_add(1)
                    .ok_or(())?,
                bytes: 0,
            };
            worst = worst.maximum(body.cost.plus(copies)?.plus(dispatch)?);
        }
        Ok(Fact {
            ty: joined.ok_or(())?,
            cost: scrutinee
                .cost
                .plus(worst)?
                .plus(Cost { steps: 1, bytes: 0 })?,
            depth: height,
        })
    }
}
