// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Read relation: exhaustive cases and temporary payload scopes.
use super::{
    field, invalid, require_exact_fields, text, BTreeMap, BTreeSet, Cell, ProviderRefusalV1,
    ReadType, Scope, Value, SUBJECT,
};
impl Scope<'_> {
    pub(super) fn variant_match(
        &self,
        value: &Value,
        depth: usize,
    ) -> Result<ReadType, ProviderRefusalV1> {
        require_exact_fields(value, &["kind", "scrutinee", "arms"], SUBJECT)?;
        let ReadType::Variant(_, cases) = self.expression(field(value, "scrutinee")?, depth + 1)?
        else {
            return Err(invalid());
        };
        let arms = super::super::array(value, "arms")?;
        if arms.len() != cases.len() {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        let mut result: Option<ReadType> = None;
        for arm in arms {
            self.remaining_type_work.set(
                self.remaining_type_work
                    .get()
                    .checked_sub(1)
                    .ok_or_else(invalid)?,
            );
            let name = text(arm, "case")?;
            if !seen.insert(name) {
                return Err(invalid());
            }
            let payload = cases.get(name).ok_or_else(invalid)?;
            let actual = self.match_arm(arm, payload.as_ref(), depth)?;
            result = Some(match (result, actual) {
                (None, actual) => actual,
                (Some(previous), actual) => previous.join(actual)?,
            });
        }
        result.ok_or_else(invalid)
    }
    fn match_arm(
        &self,
        arm: &Value,
        payload: Option<&ReadType>,
        depth: usize,
    ) -> Result<ReadType, ProviderRefusalV1> {
        let Ok(binder) = field(arm, "binder") else {
            require_exact_fields(arm, &["case", "body"], SUBJECT)?;
            return self.expression(field(arm, "body")?, depth + 1);
        };
        require_exact_fields(arm, &["case", "binder", "body"], SUBJECT)?;
        require_exact_fields(binder, &["id", "type", "alphaName"], SUBJECT)?;
        let id = text(binder, "id")?;
        text(binder, "alphaName")?;
        let declared = self.ty(text(binder, "type")?)?;
        let expected = payload.ok_or_else(invalid)?;
        if self.available.contains_key(id)
            || !declared.accepts(expected)
            || !expected.accepts(&declared)
        {
            return Err(invalid());
        }
        self.remaining_type_work.set(
            self.remaining_type_work
                .get()
                .checked_sub(self.available.len())
                .ok_or_else(invalid)?,
        );
        let mut available = self.available.clone();
        available.insert(id, binder);
        let inner = Scope {
            core: self.core,
            exports: self.exports,
            inventory: BTreeMap::new(),
            available,
            used: BTreeSet::new(),
            input: self.input,
            remaining_type_work: Cell::new(self.remaining_type_work.get()),
        };
        let result = inner.expression(field(arm, "body")?, depth + 1);
        self.remaining_type_work
            .set(inner.remaining_type_work.get());
        result
    }
}
