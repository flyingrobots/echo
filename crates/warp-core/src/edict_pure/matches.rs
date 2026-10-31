// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exhaustive closed-union dispatch and lexically scoped payloads.
use super::{
    evaluate::{expression, Meter},
    functions::{accepts, infer, join, local, Definition, Proof, Scope},
    model::{Expr, Helper, MatchArm, RuntimeType},
    syntax::Parser,
    values::{array, exact_fields, field, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

impl Parser<'_> {
    pub(super) fn match_expression(&mut self, value: &Value, depth: usize) -> Result<Expr, Error> {
        exact_fields(value, &["kind", "scrutinee", "arms"])?;
        let scrutinee = self.expr(field(value, "scrutinee")?, depth + 1)?;
        let mut arms = Vec::new();
        for arm in array(field(value, "arms")?)? {
            self.enter(depth + 1)?;
            let binder = arm_binder(arm)?;
            arms.push(MatchArm {
                case: text_field(arm, "case")?.to_owned(),
                binder: binder
                    .map(|b| text_field(b, "id").map(str::to_owned))
                    .transpose()?,
                body: self.expr(field(arm, "body")?, depth + 1)?,
            });
        }
        Ok(Expr::Match(Box::new(scrutinee), arms))
    }
}

fn arm_binder(arm: &Value) -> Result<Option<&Value>, Error> {
    let binder = field(arm, "binder").ok();
    exact_fields(
        arm,
        if binder.is_some() {
            &["case", "binder", "body"]
        } else {
            &["case", "body"]
        },
    )?;
    if let Some(binder) = binder {
        local(binder)?;
    }
    Ok(binder)
}

pub(super) fn infer_match<'v>(
    value: &'v Value,
    scope: &Scope<'_>,
    proof: Proof<'_, 'v>,
    definitions: &BTreeMap<String, Definition<'_>>,
    source: bool,
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<RuntimeType, Error> {
    exact_fields(value, &["kind", "scrutinee", "arms"])?;
    let RuntimeType::Variant { cases, .. } = infer(
        field(value, "scrutinee")?,
        scope,
        proof,
        definitions,
        source,
        parser,
        depth + 1,
    )?
    else {
        return Err(Error::InvalidArtifact);
    };
    let arms = array(field(value, "arms")?)?;
    if arms.len() != cases.len() {
        return Err(Error::InvalidArtifact);
    }
    let mut seen = BTreeSet::new();
    let mut result = None;
    for arm in arms {
        parser.enter(depth + 1)?;
        let tag = text_field(arm, "case")?;
        if !seen.insert(tag) {
            return Err(Error::InvalidArtifact);
        }
        let payload = cases.get(tag).ok_or(Error::InvalidArtifact)?;
        let mut inner = scope.clone();
        if let Some(binder) = arm_binder(arm)? {
            let expected = payload.as_ref().ok_or(Error::InvalidArtifact)?;
            let declared = parser.ty(text_field(binder, "type")?, 0)?;
            if !accepts(expected, &declared)
                || !accepts(&declared, expected)
                || inner.insert(text_field(binder, "id")?, binder).is_some()
            {
                return Err(Error::InvalidArtifact);
            }
        }
        let ty = infer(
            field(arm, "body")?,
            &inner,
            proof,
            definitions,
            source,
            parser,
            depth + 1,
        )?;
        result = Some(match result {
            None => ty,
            Some(previous) => join(previous, ty)?,
        });
    }
    result.ok_or(Error::InvalidArtifact)
}

pub(super) fn evaluate_match(
    scrutinee: &Expr,
    arms: &[MatchArm],
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    let value = expression(scrutinee, locals, helpers, meter, depth + 1)?;
    let tag = text_field(&value, "case")?;
    for arm in arms {
        meter.step(depth + 1)?;
        if arm.case != tag {
            continue;
        }
        let Some(binder) = &arm.binder else {
            return expression(&arm.body, locals, helpers, meter, depth + 1);
        };
        let mut inner = BTreeMap::new();
        // A fresh lexical frame owns its copies; every retained value is metered.
        for (id, value) in locals {
            inner.insert(*id, meter.copy(value)?);
        }
        let Value::Map(fields) = value else {
            return Err(Error::InvalidArtifact);
        };
        let payload = fields
            .into_iter()
            .find_map(|(key, value)| {
                matches!(key, Value::Text(key) if key == "payload").then_some(value)
            })
            .ok_or(Error::InvalidArtifact)?;
        if inner.insert(binder.as_str(), payload).is_some() {
            return Err(Error::InvalidArtifact);
        }
        return expression(&arm.body, &inner, helpers, meter, depth + 1);
    }
    Err(Error::InvalidArtifact)
}
