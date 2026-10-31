// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Retain the identity and bounded payload graph of variant-valued read locals.
use super::{
    entries, field, invalid, require_exact_fields, text, BTreeMap, ProviderRefusalV1, ReadType,
    Scope, Value, SUBJECT,
};
impl Scope<'_> {
    pub(super) fn variant_type(
        &self,
        value: &Value,
        key: &str,
        depth: usize,
    ) -> Result<ReadType, ProviderRefusalV1> {
        require_exact_fields(value, &["kind", "cases"], SUBJECT)?;
        let mut cases = BTreeMap::new();
        for (name, body) in entries(field(value, "cases")?)? {
            let remaining = self
                .remaining_type_work
                .get()
                .checked_sub(1)
                .ok_or_else(invalid)?;
            self.remaining_type_work.set(remaining);
            let Value::Text(name) = name else {
                return Err(invalid());
            };
            if name.is_empty()
                || name.len() > 1024
                || !name.bytes().enumerate().all(|(i, b)| {
                    b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                })
            {
                return Err(invalid());
            }
            let payload = if let Ok(payload) = field(body, "payload") {
                require_exact_fields(body, &["payload"], SUBJECT)?;
                let Value::Text(payload) = payload else {
                    return Err(invalid());
                };
                if payload.len() > 1024 {
                    return Err(invalid());
                }
                Some(self.resolve(payload, depth + 1)?)
            } else {
                require_exact_fields(body, &[], SUBJECT)?;
                None
            };
            if cases.insert(name.clone(), payload).is_some() {
                return Err(invalid());
            }
        }
        if cases.is_empty() {
            return Err(invalid());
        }
        let identity = if key.contains('@') {
            key.to_owned()
        } else {
            format!("{}.{key}", text(self.core, "coordinate")?)
        };
        Ok(ReadType::Variant(identity, cases))
    }
}
