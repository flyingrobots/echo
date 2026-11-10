// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Common read-result types preserve record shape and nominal identity.
use super::{invalid, BTreeMap, ProviderRefusalV1, ReadType};

impl ReadType {
    pub(super) fn join(self, other: Self) -> Result<Self, ProviderRefusalV1> {
        match (self, other) {
            (Self::Bytes(a, b), Self::Bytes(c, d)) => Ok(Self::Bytes(a.min(c), b.max(d))),
            (Self::List(a, n), Self::List(b, m)) => Ok(Self::List(
                match (a, b) {
                    (Some(a), Some(b)) => Some(Box::new(a.join(*b)?)),
                    (a, b) => a.or(b),
                },
                n.max(m),
            )),
            (Self::Record(a), Self::Record(mut b)) if a.keys().eq(b.keys()) => {
                let mut fields = BTreeMap::new();
                for (key, value) in a {
                    let other = b.remove(&key).ok_or_else(invalid)?;
                    fields.insert(key, value.join(other)?);
                }
                Ok(Self::Record(fields))
            }
            (a, b) if a == b => Ok(a),
            _ => Err(invalid()),
        }
    }
}
