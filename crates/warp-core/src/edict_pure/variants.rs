// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Closed tagged values, with ordinary payload evaluation and portable accounting.
use super::{
    evaluate::{expression, validate, Meter, VALUE_CELL_BYTES},
    model::{Expr, Helper, RuntimeType},
    syntax::Parser,
    values::{exact_fields, field, map, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

impl Parser<'_> {
    pub(super) fn variant_type(
        &mut self,
        definition: &Value,
        key: &str,
        depth: usize,
    ) -> Result<RuntimeType, Error> {
        exact_fields(definition, &["kind", "cases"])?;
        let mut cases = BTreeMap::new();
        for (tag, body) in map(field(definition, "cases")?)? {
            self.enter(depth)?;
            let tag = text(tag)?;
            if tag.is_empty()
                || tag.len() > 1024
                || !tag.bytes().enumerate().all(|(i, b)| {
                    b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                })
            {
                return Err(Error::InvalidArtifact);
            }
            let payload = if let Ok(payload) = field(body, "payload") {
                exact_fields(body, &["payload"])?;
                Some(self.ty(text(payload)?, depth + 1)?)
            } else {
                exact_fields(body, &[])?;
                None
            };
            if cases.insert(tag.to_owned(), payload).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
        if cases.is_empty() {
            return Err(Error::InvalidArtifact);
        }
        let identity = if key.contains('@') {
            key.to_owned()
        } else {
            format!("{}.{key}", self.coordinate)
        };
        let ty = RuntimeType::Variant { identity, cases };
        storage_bound(&ty).ok_or(Error::UnsupportedProgram)?;
        Ok(ty)
    }

    pub(super) fn variant_expression(
        &mut self,
        value: &Value,
        depth: usize,
    ) -> Result<Expr, Error> {
        let (ty, tag, payload) = constructor(self, value)?;
        let RuntimeType::Variant { identity, .. } = ty else {
            return Err(Error::InvalidArtifact);
        };
        Ok(Expr::Variant {
            identity,
            case: tag.to_owned(),
            payload: payload
                .map(|p| self.expr(p, depth + 1).map(Box::new))
                .transpose()?,
        })
    }
}

pub(super) fn constructor<'v>(
    parser: &mut Parser<'_>,
    value: &'v Value,
) -> Result<(RuntimeType, &'v str, Option<&'v Value>), Error> {
    let payload = field(value, "payload").ok();
    exact_fields(
        value,
        if payload.is_some() {
            &["kind", "type", "case", "payload"]
        } else {
            &["kind", "type", "case"]
        },
    )?;
    let ty = parser.ty(text_field(value, "type")?, 0)?;
    let RuntimeType::Variant { cases, .. } = &ty else {
        return Err(Error::InvalidArtifact);
    };
    let tag = text_field(value, "case")?;
    if cases.get(tag).ok_or(Error::InvalidArtifact)?.is_some() != payload.is_some() {
        return Err(Error::InvalidArtifact);
    }
    Ok((ty, tag, payload))
}

pub(super) fn evaluate_constructor(
    labels: (&str, &str),
    payload: Option<&Expr>,
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    let (identity, tag) = labels;
    // Expression's own step was charged by the caller. Two text values add two
    // work units; their keys, cells and bytes are charged before materializing.
    meter.step(depth + 1)?;
    meter.step(depth + 1)?;
    let metadata = (5 * VALUE_CELL_BYTES + 8)
        .checked_add(identity.len() as u64)
        .and_then(|n| n.checked_add(tag.len() as u64))
        .ok_or(Error::AllocationBudgetExceeded)?;
    meter.allocate(metadata)?;
    let mut fields = vec![
        (Value::Text("type".into()), Value::Text(identity.into())),
        (Value::Text("case".into()), Value::Text(tag.into())),
    ];
    if let Some(payload) = payload {
        meter.allocate(VALUE_CELL_BYTES + 7)?;
        fields.push((
            Value::Text("payload".into()),
            expression(payload, locals, helpers, meter, depth + 1)?,
        ));
    }
    Ok(Value::Map(fields))
}

pub(super) fn validate_value(
    value: &Value,
    identity: &str,
    cases: &BTreeMap<String, Option<RuntimeType>>,
    meter: &mut Meter,
    depth: usize,
) -> Result<(), Error> {
    meter.step(depth + 1)?;
    meter.step(depth + 1)?;
    if text_field(value, "type")? != identity {
        return Err(Error::InvalidArtifact);
    }
    let expected = cases
        .get(text_field(value, "case")?)
        .ok_or(Error::InvalidArtifact)?;
    if let Some(ty) = expected {
        exact_fields(value, &["type", "case", "payload"])?;
        validate(field(value, "payload")?, ty, meter, depth + 1)
    } else {
        exact_fields(value, &["type", "case"])
    }
}

// Type bounds must be finite even when only an empty case is constructed.
fn storage_bound(ty: &RuntimeType) -> Option<u64> {
    match ty {
        RuntimeType::Nominal(_, inner) => storage_bound(inner),
        RuntimeType::Bool | RuntimeType::Unsigned(_) => Some(VALUE_CELL_BYTES),
        RuntimeType::Bytes { max, .. } => VALUE_CELL_BYTES.checked_add(*max),
        RuntimeType::List { item, max } => {
            let child = match item {
                Some(ty) => storage_bound(ty)?,
                None if *max == 0 => 0,
                None => return None,
            };
            child.checked_mul(*max)?.checked_add(VALUE_CELL_BYTES)
        }
        RuntimeType::Record(fields) => fields.iter().try_fold(VALUE_CELL_BYTES, |n, (name, ty)| {
            n.checked_add(VALUE_CELL_BYTES)?
                .checked_add(name.len() as u64)?
                .checked_add(storage_bound(ty)?)
        }),
        RuntimeType::Variant { identity, cases } => {
            let mut maximum = VALUE_CELL_BYTES;
            let mut label = 0;
            for (name, ty) in cases {
                label = label.max(name.len() as u64);
                if let Some(ty) = ty {
                    maximum = maximum.max(storage_bound(ty)?);
                }
            }
            (6 * VALUE_CELL_BYTES + 15)
                .checked_add(identity.len() as u64)?
                .checked_add(label)?
                .checked_add(maximum)
        }
    }
}
