// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned closed-union identity, constructor and conservative storage proof.
use super::{
    exact, get, identifier, includes, members, string, Check, Cost, Fact, Judgment, Scope, Ty,
    Value,
};
use std::collections::BTreeMap;

impl<'a> Judgment<'a> {
    pub(super) fn variant_type(
        &mut self,
        definition: &Value,
        key: &str,
        prefix: &str,
        depth: usize,
    ) -> Check<Ty> {
        exact(definition, &["kind", "cases"])?;
        let alternatives = members(get(definition, "cases")?)?;
        if alternatives.is_empty() {
            return Err(());
        }
        let mut cases = BTreeMap::new();
        for (tag, body) in alternatives {
            self.charge(depth)?;
            let tag = string(tag)?;
            if !identifier(tag) {
                return Err(());
            }
            let payload = if let Some(payload) = super::super::map_field(body, "payload") {
                exact(body, &["payload"])?;
                Some(self.ty(string(payload)?, depth + 1)?)
            } else {
                exact(body, &[])?;
                None
            };
            if cases.insert(tag.to_owned(), payload).is_some() {
                return Err(());
            }
        }
        let identity = if key.contains('@') {
            key.to_owned()
        } else {
            format!("{prefix}{key}")
        };
        let ty = Ty::Variant { identity, cases };
        super::shape(&ty)?;
        Ok(ty)
    }

    pub(super) fn variant_constructor(
        &mut self,
        value: &'a Value,
        scope: &Scope<'a>,
        source: bool,
        depth: usize,
    ) -> Check<Fact> {
        let payload = super::super::map_field(value, "payload");
        exact(
            value,
            if payload.is_some() {
                &["kind", "type", "case", "payload"]
            } else {
                &["kind", "type", "case"]
            },
        )?;
        let ty = self.ty(string(get(value, "type")?)?, 0)?;
        let Ty::Variant { identity, cases } = &ty else {
            return Err(());
        };
        let tag = string(get(value, "case")?)?;
        let expected = cases.get(tag).ok_or(())?;
        let metadata = if payload.is_some() {
            6_u64 * 64 + 15
        } else {
            5_u64 * 64 + 8
        };
        let mut cost = Cost {
            steps: 3,
            bytes: metadata
                .checked_add(identity.len() as u64)
                .and_then(|n| n.checked_add(tag.len() as u64))
                .ok_or(())?,
        };
        let mut height = 1;
        match (expected, payload) {
            (None, None) => {}
            (Some(expected), Some(payload)) => {
                let child = self.expression(payload, scope, source, depth + 1)?;
                if !includes(expected, &child.ty) {
                    return Err(());
                }
                cost = cost.plus(child.cost)?;
                height = height.max(child.depth + 1);
            }
            _ => return Err(()),
        }
        Ok(Fact {
            ty,
            cost,
            depth: height,
        })
    }
}

pub(super) fn shape(
    identity: &str,
    cases: &BTreeMap<String, Option<Ty>>,
) -> Check<(Cost, u64, u64, usize)> {
    // Reserve payload key and one null-sized cell even for an omitted payload.
    let mut payload = Cost {
        steps: 1,
        bytes: 64,
    };
    let mut checks = 1_u64;
    let mut encoded = 1_u64;
    let mut height = 0;
    let mut label = 0;
    for (tag, ty) in cases {
        label = label.max(tag.len() as u64);
        if let Some(ty) = ty {
            let (copy, validations, bytes, depth) = super::shape(ty)?;
            payload = payload.maximum(copy);
            checks = checks.max(validations);
            encoded = encoded.max(bytes);
            height = height.max(depth);
        }
    }
    let metadata_bytes = (6_u64 * 64 + 15)
        .checked_add(identity.len() as u64)
        .and_then(|n| n.checked_add(label))
        .ok_or(())?;
    let encoded = encoded
        .checked_add(69)
        .and_then(|n| n.checked_add(identity.len() as u64))
        .and_then(|n| n.checked_add(label))
        .ok_or(())?;
    Ok((
        payload.plus(Cost {
            steps: 6,
            bytes: metadata_bytes,
        })?,
        checks.checked_add(3).ok_or(())?,
        encoded,
        height + 1,
    ))
}

// A proof clone retains all alternatives; a runtime copy contains only one.
pub(super) fn type_nodes(ty: &Ty) -> Check<usize> {
    let mut pending = vec![ty];
    let mut count = 0_usize;
    while let Some(ty) = pending.pop() {
        count = count.checked_add(1).ok_or(())?;
        match ty {
            Ty::Record(fields) => {
                count = count.checked_add(fields.len()).ok_or(())?;
                pending.extend(fields.values());
            }
            Ty::List(Some(inner), _) | Ty::Nominal(_, inner) => pending.push(inner),
            Ty::Variant { cases, .. } => {
                count = count.checked_add(cases.len()).ok_or(())?;
                pending.extend(cases.values().filter_map(Option::as_ref));
            }
            _ => {}
        }
        if count > super::WORK {
            return Err(());
        }
    }
    Ok(count)
}
