// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Read audit: each case is consumed once and payload declarations are restored.
use super::{exact, get, text, Check, Schema, Symbols, Value};
impl Symbols<'_> {
    pub(super) fn match_schema(&mut self, value: &Value, depth: usize) -> Check<Schema> {
        exact(value, &["kind", "scrutinee", "arms"])?;
        let Schema::Variant {
            mut alternatives, ..
        } = self.expression(get(value, "scrutinee")?, depth + 1)?
        else {
            return Err(());
        };
        let arms = super::super::sequence(value, "arms")?;
        let mut result = None;
        for arm in arms {
            self.charge(depth + 1)?;
            let payload = alternatives.remove(text(arm, "case")?).ok_or(())?;
            let actual = self.arm_schema(arm, payload.as_ref(), depth)?;
            result = Some(match (result, actual) {
                (None, actual) => actual,
                (
                    Some(Schema::Bytes { lower: a, upper: b }),
                    Schema::Bytes { lower: c, upper: d },
                ) => Schema::Bytes {
                    lower: a.min(c),
                    upper: b.max(d),
                },
                (Some(previous), actual) if previous == actual => actual,
                _ => return Err(()),
            });
        }
        if !alternatives.is_empty() {
            return Err(());
        }
        result.ok_or(())
    }
    fn arm_schema(&mut self, arm: &Value, payload: Option<&Schema>, depth: usize) -> Check<Schema> {
        let Ok(binder) = get(arm, "binder") else {
            exact(arm, &["case", "body"])?;
            return self.expression(get(arm, "body")?, depth + 1);
        };
        exact(arm, &["case", "body", "binder"])?;
        exact(binder, &["id", "alphaName", "type"])?;
        let id = text(binder, "id")?;
        text(binder, "alphaName")?;
        let expected = payload.ok_or(())?;
        let declared = self.schema(text(binder, "type")?, 0)?;
        if self.live.contains(id) || !expected.includes(&declared) || !declared.includes(expected) {
            return Err(());
        }
        let body = get(arm, "body")?;
        let previous = self.declared.insert(id.to_owned(), binder.clone());
        self.live.insert(id.to_owned());
        let result = self.expression(body, depth + 1);
        self.live.remove(id);
        if let Some(previous) = previous {
            self.declared.insert(id.to_owned(), previous);
        } else {
            self.declared.remove(id);
        }
        result
    }
}
