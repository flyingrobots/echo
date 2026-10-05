// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned bounded representations and independently accumulated charges.
use super::{exact, get, members, string, unsigned, Check};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Schema {
    Word(u64),
    Blob {
        low: u64,
        high: u64,
    },
    Fields(BTreeMap<String, Schema>),
    Nominal {
        identity: String,
        representation: Box<Schema>,
    },
}
#[derive(Clone, Copy, Default)]
pub(super) struct Charge {
    pub ticks: u128,
    pub storage: u128,
}
impl Charge {
    pub(super) fn append(&mut self, other: Self) -> Check<()> {
        self.ticks = self.ticks.checked_add(other.ticks).ok_or(())?;
        self.storage = self.storage.checked_add(other.storage).ok_or(())?;
        if self.ticks > u128::from(u64::MAX) || self.storage > u128::from(u64::MAX) {
            return Err(());
        }
        Ok(())
    }
    pub(super) fn choice(a: Self, b: Self) -> Self {
        Self {
            ticks: a.ticks.max(b.ticks),
            storage: a.storage.max(b.storage),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Size {
    pub copy: Charge,
    pub checks: u128,
    pub encoded: u128,
    pub height: usize,
}
impl Schema {
    pub(super) fn representation(&self) -> &Self {
        match self {
            Self::Nominal { representation, .. } => representation.representation(),
            _ => self,
        }
    }
    pub(super) fn encloses(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Blob { low, high }, Self::Blob { low: a, high: b }) => low <= a && b <= high,
            (Self::Fields(a), Self::Fields(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(name, ty)| b.get(name).is_some_and(|value| ty.encloses(value)))
            }
            _ => self == other,
        }
    }
    pub(super) fn choice(self, other: Self) -> Check<Self> {
        match (self, other) {
            (Self::Blob { low: a, high: b }, Self::Blob { low: c, high: d }) => Ok(Self::Blob {
                low: a.min(c),
                high: b.max(d),
            }),
            (a, b) if a == b => Ok(a),
            _ => Err(()),
        }
    }
    pub(super) fn measure(&self) -> Check<Size> {
        let mut size = Size {
            copy: Charge {
                ticks: 1,
                storage: 64,
            },
            checks: 1,
            encoded: 9,
            height: 0,
        };
        match self {
            // Nominal types retain identity without adding physical value tags.
            Self::Nominal { representation, .. } => return representation.measure(),
            Self::Word(_) => {}
            Self::Blob { high, .. } => {
                size.copy.storage += u128::from(*high);
                size.encoded += u128::from(*high);
            }
            Self::Fields(fields) => {
                for (name, ty) in fields {
                    let child = ty.measure()?;
                    size.copy.append(child.copy)?;
                    size.copy.append(Charge {
                        ticks: 1,
                        storage: 64 + name.len() as u128,
                    })?;
                    size.checks = size.checks.checked_add(child.checks).ok_or(())?;
                    size.encoded = size
                        .encoded
                        .checked_add(9 + name.len() as u128)
                        .and_then(|value| value.checked_add(child.encoded))
                        .ok_or(())?;
                    size.height = size.height.max(child.height + 1);
                }
            }
        }
        if size.copy.storage > u128::from(u64::MAX) || size.encoded > u128::from(u64::MAX) {
            return Err(());
        }
        Ok(size)
    }
}

pub(super) struct Types<'a> {
    pub module: &'a Value,
    remaining: usize,
}
impl<'a> Types<'a> {
    pub(super) fn new(module: &'a Value) -> Self {
        Self {
            module,
            remaining: 65_536,
        }
    }
    pub(super) fn step(&mut self, depth: usize) -> Check<()> {
        if depth > 64 {
            return Err(());
        }
        self.remaining = self.remaining.checked_sub(1).ok_or(())?;
        Ok(())
    }
    pub(super) fn copy_type(&mut self, schema: &Schema) -> Check<Schema> {
        let count = usize::try_from(schema.measure()?.copy.ticks).map_err(|_| ())?;
        self.remaining = self.remaining.checked_sub(count).ok_or(())?;
        Ok(schema.clone())
    }
    pub(super) fn resolve(&mut self, name: &str, depth: usize) -> Check<Schema> {
        self.step(depth)?;
        if let Some(width) = match name {
            "U8" => Some(u8::MAX.into()),
            "U16" => Some(u16::MAX.into()),
            "U32" => Some(u32::MAX.into()),
            "U64" => Some(u64::MAX),
            _ => None,
        } {
            return Ok(Schema::Word(width));
        }
        if let Some(spec) = name
            .strip_prefix("Bytes<")
            .and_then(|s| s.strip_suffix('>'))
        {
            let decimal = |s: &str| -> Check<u64> {
                let value = s.parse::<u64>().map_err(|_| ())?;
                if value.to_string() != s {
                    return Err(());
                }
                Ok(value)
            };
            if let Some(s) = spec.strip_prefix("max=") {
                return Ok(Schema::Blob {
                    low: 0,
                    high: decimal(s)?,
                });
            }
            if let Some(s) = spec.strip_prefix("exact=") {
                let value = decimal(s)?;
                return Ok(Schema::Blob {
                    low: value,
                    high: value,
                });
            }
            if let Some((a, b)) = spec
                .strip_prefix("min=")
                .and_then(|s| s.split_once(",max="))
            {
                let (low, high) = (decimal(a)?, decimal(b)?);
                if low < high {
                    return Ok(Schema::Blob { low, high });
                }
            }
            return Err(());
        }
        let prefix = format!("{}.", string(get(self.module, "coordinate")?)?);
        let resolved = name.strip_prefix(&prefix).unwrap_or(name);
        let definition = get(get(self.module, "types")?, resolved)?;
        match string(get(definition, "kind")?)? {
            "Nominal" => {
                let identity = string(get(definition, "contract")?)?;
                if identity != resolved {
                    return Err(());
                }
                let representation =
                    self.resolve(string(get(definition, "representation")?)?, depth + 1)?;
                Ok(Schema::Nominal {
                    identity: identity.to_owned(),
                    representation: Box::new(representation),
                })
            }
            "Bytes" => {
                let high = unsigned(get(definition, "max")?)?;
                let low = super::super::map_field(definition, "min")
                    .map(unsigned)
                    .transpose()?
                    .unwrap_or(0);
                if low > high {
                    return Err(());
                }
                Ok(Schema::Blob { low, high })
            }
            "Record" => {
                let mut fields = BTreeMap::new();
                for (name, coordinate) in members(get(definition, "fields")?)? {
                    if fields
                        .insert(
                            string(name)?.to_owned(),
                            self.resolve(string(coordinate)?, depth + 1)?,
                        )
                        .is_some()
                    {
                        return Err(());
                    }
                }
                Ok(Schema::Fields(fields))
            }
            _ => Err(()),
        }
    }
    pub(super) fn local(&mut self, value: &Value) -> Check<Schema> {
        exact(value, &["id", "type", "alphaName"])?;
        string(get(value, "id")?)?;
        string(get(value, "alphaName")?)?;
        self.resolve(string(get(value, "type")?)?, 0)
    }
}
