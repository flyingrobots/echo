// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the shared value representation of `edict.core-cost/v2`.
//!
//! Scalars are held inline. Byte strings, lists, records and variants are
//! immutable and reference-counted, so a clone is one reference-count update
//! whatever the value's size: reading a local, field, element or fold state
//! never copies. Record keys and variant labels borrow the program's own text.
//! Conversion to and from canonical values is the only full-size walk, and
//! happens only at the intent's input and output.
use super::super::{model::RuntimeType, EvaluationError as Error};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::rc::Rc;

/// One immutable runtime value; `clone` is O(1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Shared<'p> {
    Bool(bool),
    Integer(i128),
    Bytes(Rc<[u8]>),
    List(Rc<[Shared<'p>]>),
    Record(Rc<[(&'p str, Shared<'p>)]>),
    Variant(Rc<Tagged<'p>>),
}

/// A closed-union value: its type identity, case and optional payload.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Tagged<'p> {
    pub identity: &'p str,
    pub case: &'p str,
    pub payload: Option<Shared<'p>>,
}

/// Portable size in 64-byte value cells and bytes, as the schedule counts it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Size {
    pub cells: u64,
    pub bytes: u64,
}

/// Fixed cell and metadata units of the schedule.
pub(super) const CELL: Size = Size {
    cells: 1,
    bytes: 64,
};
/// A variant's header beyond its payload: six cells and 399 bytes, plus the
/// identity and case text.
const VARIANT_HEADER_CELLS: u64 = 6;
const VARIANT_HEADER_BYTES: u64 = 399;

impl Size {
    pub(super) fn plus(self, other: Self) -> Result<Self, Error> {
        Ok(Self {
            cells: self
                .cells
                .checked_add(other.cells)
                .ok_or(Error::StepBudgetExceeded)?,
            bytes: self
                .bytes
                .checked_add(other.bytes)
                .ok_or(Error::AllocationBudgetExceeded)?,
        })
    }

    /// A record field slot: one cell plus 64 bytes and the key text.
    pub(super) fn field(self, key: &str) -> Result<Self, Error> {
        self.plus(Size {
            cells: 1,
            bytes: 64_u64
                .checked_add(length(key.len())?)
                .ok_or(Error::AllocationBudgetExceeded)?,
        })
    }

    /// A variant header around `payload`.
    pub(super) fn variant(payload: Self, identity: &str, case: &str) -> Result<Self, Error> {
        payload.plus(Size {
            cells: VARIANT_HEADER_CELLS,
            bytes: VARIANT_HEADER_BYTES
                .checked_add(length(identity.len())?)
                .and_then(|n| n.checked_add(length(case.len()).ok()?))
                .ok_or(Error::AllocationBudgetExceeded)?,
        })
    }

    /// A list shell holding `items` shared references.
    pub(super) fn list_shell(items: usize) -> Result<Self, Error> {
        let items = length(items)?;
        CELL.plus(Size {
            cells: items,
            bytes: items
                .checked_mul(64)
                .ok_or(Error::AllocationBudgetExceeded)?,
        })
    }
}

pub(super) fn length(value: usize) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::AllocationBudgetExceeded)
}

impl Shared<'_> {
    /// The value's full size, as a comparison or the intent boundary sees it.
    /// The walk visits every cell once.
    pub(super) fn full_size(&self) -> Result<Size, Error> {
        match self {
            Self::Bool(_) | Self::Integer(_) => Ok(CELL),
            Self::Bytes(bytes) => CELL.plus(Size {
                cells: 0,
                bytes: length(bytes.len())?,
            }),
            Self::List(items) => items
                .iter()
                .try_fold(CELL, |total, item| total.plus(item.full_size()?)),
            Self::Record(fields) => fields.iter().try_fold(CELL, |total, (key, value)| {
                total.plus(value.full_size()?)?.field(key)
            }),
            Self::Variant(tagged) => {
                let payload = match &tagged.payload {
                    Some(payload) => payload.full_size()?,
                    None => CELL,
                };
                Size::variant(payload, tagged.identity, tagged.case)
            }
        }
    }
}

/// Validates a canonical input against its type while sharing it. Keys and
/// labels are taken from the type, which outlives the evaluation.
pub(super) fn from_canonical<'p>(
    value: &Value,
    ty: &'p RuntimeType,
    depth: usize,
) -> Result<Shared<'p>, Error> {
    if depth > super::super::model::MAX_DEPTH {
        return Err(Error::UnsupportedProgram);
    }
    match (value, ty.representation()) {
        (Value::Bool(value), RuntimeType::Bool) => Ok(Shared::Bool(*value)),
        (Value::Integer(value), RuntimeType::Unsigned(max))
            if *value >= 0 && *value <= i128::from(*max) =>
        {
            Ok(Shared::Integer(*value))
        }
        (Value::Bytes(bytes), RuntimeType::Bytes { min, max })
            if (*min..=*max).contains(&length(bytes.len())?) =>
        {
            Ok(Shared::Bytes(Rc::from(bytes.as_slice())))
        }
        (Value::Array(items), RuntimeType::List { item, max }) if length(items.len())? <= *max => {
            let item = item.as_deref().ok_or(Error::InvalidArtifact)?;
            let items = items
                .iter()
                .map(|value| from_canonical(value, item, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Shared::List(Rc::from(items)))
        }
        (Value::Map(entries), RuntimeType::Record(types)) if entries.len() == types.len() => {
            let fields = types
                .iter()
                .map(|(name, ty)| {
                    let value = super::super::values::field(value, name)?;
                    Ok((name.as_str(), from_canonical(value, ty, depth + 1)?))
                })
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(Shared::Record(Rc::from(fields)))
        }
        (Value::Map(_), RuntimeType::Variant { identity, cases }) => {
            let case = super::super::values::text_field(value, "case")?;
            if super::super::values::text_field(value, "type")? != identity {
                return Err(Error::InvalidArtifact);
            }
            let (case, payload_type) = cases.get_key_value(case).ok_or(Error::InvalidArtifact)?;
            let payload = if let Some(ty) = payload_type {
                super::super::values::exact_fields(value, &["type", "case", "payload"])?;
                let payload = super::super::values::field(value, "payload")?;
                Some(from_canonical(payload, ty, depth + 1)?)
            } else {
                super::super::values::exact_fields(value, &["type", "case"])?;
                None
            };
            Ok(Shared::Variant(Rc::new(Tagged {
                identity,
                case,
                payload,
            })))
        }
        _ => Err(Error::InvalidArtifact),
    }
}

/// Validates a result against its type while materializing it canonically.
pub(super) fn to_canonical(
    value: &Shared<'_>,
    ty: &RuntimeType,
    depth: usize,
) -> Result<Value, Error> {
    if depth > super::super::model::MAX_DEPTH {
        return Err(Error::UnsupportedProgram);
    }
    match (value, ty.representation()) {
        (Shared::Bool(value), RuntimeType::Bool) => Ok(Value::Bool(*value)),
        (Shared::Integer(value), RuntimeType::Unsigned(max))
            if *value >= 0 && *value <= i128::from(*max) =>
        {
            Ok(Value::Integer(*value))
        }
        (Shared::Bytes(bytes), RuntimeType::Bytes { min, max })
            if (*min..=*max).contains(&length(bytes.len())?) =>
        {
            Ok(Value::Bytes(bytes.to_vec()))
        }
        (Shared::List(items), RuntimeType::List { item, max }) if length(items.len())? <= *max => {
            let item = item.as_deref().ok_or(Error::InvalidArtifact)?;
            Ok(Value::Array(
                items
                    .iter()
                    .map(|value| to_canonical(value, item, depth + 1))
                    .collect::<Result<_, _>>()?,
            ))
        }
        (Shared::Record(fields), RuntimeType::Record(types)) if fields.len() == types.len() => {
            let entries = types
                .iter()
                .map(|(name, ty)| {
                    let value = fields
                        .iter()
                        .find_map(|(key, value)| (*key == name).then_some(value))
                        .ok_or(Error::InvalidArtifact)?;
                    Ok((
                        Value::Text(name.clone()),
                        to_canonical(value, ty, depth + 1)?,
                    ))
                })
                .collect::<Result<_, Error>>()?;
            Ok(Value::Map(entries))
        }
        (Shared::Variant(tagged), RuntimeType::Variant { identity, cases })
            if tagged.identity == identity =>
        {
            let payload_type = cases.get(tagged.case).ok_or(Error::InvalidArtifact)?;
            let mut entries = vec![
                (Value::Text("type".into()), Value::Text(identity.clone())),
                (Value::Text("case".into()), Value::Text(tagged.case.into())),
            ];
            match (payload_type, &tagged.payload) {
                (Some(ty), Some(payload)) => entries.push((
                    Value::Text("payload".into()),
                    to_canonical(payload, ty, depth + 1)?,
                )),
                (None, None) => {}
                _ => return Err(Error::InvalidArtifact),
            }
            Ok(Value::Map(entries))
        }
        _ => Err(Error::InvalidArtifact),
    }
}
