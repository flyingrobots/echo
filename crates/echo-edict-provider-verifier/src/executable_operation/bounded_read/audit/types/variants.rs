// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent read-local schema expansion for closed union identities.
use super::{exact, get, members, string, text, BTreeMap, Check, Schema, Symbols, Value};
impl Symbols<'_> {
    pub(super) fn variant_schema(
        &mut self,
        definition: &Value,
        key: &str,
        depth: usize,
    ) -> Check<Schema> {
        exact(definition, &["kind", "cases"])?;
        let entries = members(get(definition, "cases")?)?;
        if entries.is_empty() {
            return Err(());
        }
        let mut alternatives = BTreeMap::new();
        for (label, entry) in entries {
            self.charge(depth)?;
            let label = string(label)?;
            let mut letters = label.bytes();
            if label.len() > 1024
                || !letters
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                || !letters.all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err(());
            }
            let body = members(entry)?;
            let payload = match body {
                [] => None,
                [(field, value)] if string(field)? == "payload" => {
                    let reference = string(value)?;
                    if reference.len() > 1024 {
                        return Err(());
                    }
                    Some(self.schema(reference, depth + 1)?)
                }
                _ => return Err(()),
            };
            if alternatives.insert(label.to_owned(), payload).is_some() {
                return Err(());
            }
        }
        let identity = if key.contains('@') {
            key.to_owned()
        } else {
            format!("{}.{key}", text(self.core, "coordinate")?)
        };
        Ok(Schema::Variant {
            identity,
            alternatives,
        })
    }
}
