// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned joins for bounded sequence elements and conditional results.
use super::{BTreeMap, Check, Schema};

impl Schema {
    pub(super) fn choice(self, other: Self) -> Check<Self> {
        match (self, other) {
            (Self::Bytes { lower: a, upper: b }, Self::Bytes { lower: c, upper: d }) => {
                Ok(Self::Bytes {
                    lower: a.min(c),
                    upper: b.max(d),
                })
            }
            (
                Self::Sequence {
                    element: a,
                    maximum: n,
                },
                Self::Sequence {
                    element: b,
                    maximum: m,
                },
            ) => {
                let element = match (a, b) {
                    (Some(a), Some(b)) => Some(Box::new(a.choice(*b)?)),
                    (a, b) => a.or(b),
                };
                Ok(Self::Sequence {
                    element,
                    maximum: n.max(m),
                })
            }
            (Self::Row(a), Self::Row(b)) => {
                if !a.keys().eq(b.keys()) {
                    return Err(());
                }
                let fields = a
                    .into_iter()
                    .zip(b)
                    .map(|((key, a), (_, b))| Ok((key, a.choice(b)?)))
                    .collect::<Check<BTreeMap<_, _>>>()?;
                Ok(Self::Row(fields))
            }
            (a, b) if a == b => Ok(a),
            _ => Err(()),
        }
    }
}
