// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared ADR-0008 runtime schema primitives.
//!
//! This crate is the Echo-local shared owner for generated-or-generation-ready
//! runtime schema types that are not inherently ABI-only:
//!
//! - opaque runtime identifiers
//! - logical monotone counters
//! - structural runtime key types
//!
//! Adapter crates such as `echo-wasm-abi` may still wrap these types when the
//! host wire format needs a different serialization contract.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "serde")]
extern crate alloc;

use core::fmt;

#[cfg(feature = "serde")]
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};

macro_rules! logical_counter {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(u64);

        impl $name {
            /// Zero value for this logical counter.
            pub const ZERO: Self = Self(0);
            /// Largest representable counter value.
            pub const MAX: Self = Self(u64::MAX);

            /// Builds the counter from its raw logical value.
            #[must_use]
            pub const fn from_raw(raw: u64) -> Self {
                Self(raw)
            }

            /// Returns the raw logical value.
            #[must_use]
            pub const fn as_u64(self) -> u64 {
                self.0
            }

            /// Adds `rhs`, returning `None` on overflow.
            #[must_use]
            pub fn checked_add(self, rhs: u64) -> Option<Self> {
                self.0.checked_add(rhs).map(Self)
            }

            /// Subtracts `rhs`, returning `None` on underflow.
            #[must_use]
            pub fn checked_sub(self, rhs: u64) -> Option<Self> {
                self.0.checked_sub(rhs).map(Self)
            }

            /// Increments by one, returning `None` on overflow.
            #[must_use]
            pub fn checked_increment(self) -> Option<Self> {
                self.checked_add(1)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

/// Canonical 32-byte identifier payload used by shared runtime schema ids.
pub type RuntimeIdBytes = [u8; 32];

/// Opaque stable identifier for a worldline.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WorldlineId(RuntimeIdBytes);

impl WorldlineId {
    /// Reconstructs a worldline id from its canonical 32-byte representation.
    #[must_use]
    pub const fn from_bytes(bytes: RuntimeIdBytes) -> Self {
        Self(bytes)
    }

    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub const fn as_bytes(&self) -> &RuntimeIdBytes {
        &self.0
    }
}

/// Opaque stable identifier for a head.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HeadId(RuntimeIdBytes);

impl HeadId {
    /// Inclusive minimum key used by internal `BTreeMap` range queries.
    pub const MIN: Self = Self([0u8; 32]);
    /// Inclusive maximum key used by internal `BTreeMap` range queries.
    pub const MAX: Self = Self([0xff; 32]);

    /// Reconstructs a head id from its canonical 32-byte representation.
    #[must_use]
    pub const fn from_bytes(bytes: RuntimeIdBytes) -> Self {
        Self(bytes)
    }

    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub const fn as_bytes(&self) -> &RuntimeIdBytes {
        &self.0
    }
}

logical_counter!(
    /// Per-worldline append identity for committed history.
    WorldlineTick
);

logical_counter!(
    /// Runtime-cycle correlation stamp. No wall-clock semantics.
    GlobalTick
);

logical_counter!(
    /// Control-plane generation token for scheduler runs.
    ///
    /// This value is not provenance, replay state, or hash input.
    RunId
);

/// Composite key identifying a writer head within its worldline.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WriterHeadKey {
    /// The worldline this head targets.
    pub worldline_id: WorldlineId,
    /// The head identity within that worldline.
    pub head_id: HeadId,
}

#[cfg(feature = "serde")]
fn serialize_runtime_id<S>(bytes: &RuntimeIdBytes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(bytes)
}

#[cfg(feature = "serde")]
fn decode_runtime_id<E>(bytes: &[u8]) -> Result<RuntimeIdBytes, E>
where
    E: de::Error,
{
    bytes
        .try_into()
        .map_err(|_| E::invalid_length(bytes.len(), &"exactly 32 bytes"))
}

#[cfg(feature = "serde")]
struct RuntimeIdVisitor {
    type_name: &'static str,
}

#[cfg(feature = "serde")]
impl RuntimeIdVisitor {
    const fn new(type_name: &'static str) -> Self {
        Self { type_name }
    }
}

#[cfg(feature = "serde")]
impl Visitor<'_> for RuntimeIdVisitor {
    type Value = RuntimeIdBytes;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "exactly 32 bytes for {}", self.type_name)
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        decode_runtime_id(value)
    }

    fn visit_borrowed_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(value)
    }

    fn visit_byte_buf<E>(self, value: alloc::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(&value)
    }
}

#[cfg(feature = "serde")]
impl Serialize for WorldlineId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_runtime_id(self.as_bytes(), serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for WorldlineId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(RuntimeIdVisitor::new("WorldlineId"))?;
        Ok(Self::from_bytes(bytes))
    }
}

#[cfg(feature = "serde")]
impl Serialize for HeadId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_runtime_id(self.as_bytes(), serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for HeadId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_bytes(RuntimeIdVisitor::new("HeadId"))?;
        Ok(Self::from_bytes(bytes))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{GlobalTick, HeadId, RunId, WorldlineId, WorldlineTick, WriterHeadKey};
    #[cfg(feature = "serde")]
    use ciborium::value::Value;
    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize, de::DeserializeOwned};

    macro_rules! assert_logical_counter_boundaries {
        ($ty:ty) => {{
            assert_eq!(<$ty>::ZERO.as_u64(), 0);
            assert_eq!(<$ty>::MAX.as_u64(), u64::MAX);
            assert_eq!(<$ty>::from_raw(41).checked_add(1).unwrap().as_u64(), 42);
            assert_eq!(<$ty>::MAX.checked_add(1), None);
            assert_eq!(<$ty>::from_raw(42).checked_sub(1).unwrap().as_u64(), 41);
            assert_eq!(<$ty>::ZERO.checked_sub(1), None);
            assert_eq!(<$ty>::from_raw(7).checked_increment().unwrap().as_u64(), 8);
            assert_eq!(<$ty>::MAX.checked_increment(), None);
        }};
    }

    #[test]
    fn worldline_tick_checked_arithmetic_boundaries() {
        assert_logical_counter_boundaries!(WorldlineTick);
    }

    #[test]
    fn global_tick_checked_arithmetic_boundaries() {
        assert_logical_counter_boundaries!(GlobalTick);
    }

    #[test]
    fn run_id_checked_arithmetic_boundaries() {
        assert_logical_counter_boundaries!(RunId);
    }

    #[test]
    fn opaque_ids_round_trip_bytes() {
        let worldline = WorldlineId::from_bytes([3u8; 32]);
        let head = HeadId::from_bytes([7u8; 32]);
        assert_eq!(*worldline.as_bytes(), [3u8; 32]);
        assert_eq!(*head.as_bytes(), [7u8; 32]);
    }

    #[test]
    fn writer_head_key_preserves_typed_components() {
        let key = WriterHeadKey {
            worldline_id: WorldlineId::from_bytes([1u8; 32]),
            head_id: HeadId::from_bytes([2u8; 32]),
        };
        assert_eq!(*key.worldline_id.as_bytes(), [1u8; 32]);
        assert_eq!(*key.head_id.as_bytes(), [2u8; 32]);
    }

    #[cfg(feature = "serde")]
    fn encode_cbor<T: Serialize>(value: &T) -> Vec<u8> {
        let mut bytes = Vec::new();
        ciborium::into_writer(value, &mut bytes).unwrap();
        bytes
    }

    #[cfg(feature = "serde")]
    fn decode_cbor<T: DeserializeOwned>(bytes: &[u8]) -> T {
        ciborium::from_reader(bytes).unwrap()
    }

    #[cfg(feature = "serde")]
    #[test]
    fn runtime_ids_serialize_as_cbor_bytes() {
        let worldline = WorldlineId::from_bytes([3u8; 32]);
        let head = HeadId::from_bytes([7u8; 32]);

        let worldline_value: Value = decode_cbor(&encode_cbor(&worldline));
        let head_value: Value = decode_cbor(&encode_cbor(&head));

        assert_eq!(worldline_value, Value::Bytes(vec![3u8; 32]));
        assert_eq!(head_value, Value::Bytes(vec![7u8; 32]));
        assert_eq!(
            decode_cbor::<WorldlineId>(&encode_cbor(&worldline)),
            worldline
        );
        assert_eq!(decode_cbor::<HeadId>(&encode_cbor(&head)), head);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn writer_head_key_cbor_round_trip_preserves_byte_encoding() {
        let key = WriterHeadKey {
            worldline_id: WorldlineId::from_bytes([5u8; 32]),
            head_id: HeadId::from_bytes([9u8; 32]),
        };

        let value: Value = decode_cbor(&encode_cbor(&key));
        assert!(matches!(value, Value::Map(_)));
        let entries = match value {
            Value::Map(entries) => entries,
            _ => return,
        };

        let encoded_worldline = entries.iter().find_map(|(field, value)| match field {
            Value::Text(name) if name == "worldline_id" => Some(value),
            _ => None,
        });
        let encoded_head = entries.iter().find_map(|(field, value)| match field {
            Value::Text(name) if name == "head_id" => Some(value),
            _ => None,
        });

        assert_eq!(encoded_worldline, Some(&Value::Bytes(vec![5u8; 32])));
        assert_eq!(encoded_head, Some(&Value::Bytes(vec![9u8; 32])));
        assert_eq!(decode_cbor::<WriterHeadKey>(&encode_cbor(&key)), key);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn runtime_ids_reject_non_32_byte_cbor_bytes() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Wrapper {
            id: WorldlineId,
        }

        let bytes = encode_cbor(&Value::Map(vec![(
            Value::Text("id".into()),
            Value::Bytes(vec![9u8; 31]),
        )]));
        let err = ciborium::from_reader::<Wrapper, _>(&bytes[..]).unwrap_err();
        assert!(err.to_string().contains("32 bytes"));

        let bytes = encode_cbor(&Value::Map(vec![(
            Value::Text("id".into()),
            Value::Bytes(vec![9u8; 33]),
        )]));
        let err = ciborium::from_reader::<Wrapper, _>(&bytes[..]).unwrap_err();
        assert!(err.to_string().contains("32 bytes"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn runtime_ids_reject_cbor_integer_arrays() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Wrapper {
            id: WorldlineId,
        }

        let bytes = encode_cbor(&Value::Map(vec![(
            Value::Text("id".into()),
            Value::Array(
                (0u8..32)
                    .map(|value| Value::Integer(value.into()))
                    .collect(),
            ),
        )]));

        let err = ciborium::from_reader::<Wrapper, _>(&bytes[..]).unwrap_err();
        assert!(err.to_string().contains("bytes"));

        let bytes = encode_cbor(&Value::Map(vec![(
            Value::Text("id".into()),
            Value::Array(
                (0u8..33)
                    .map(|value| Value::Integer(value.into()))
                    .collect(),
            ),
        )]));

        let err = ciborium::from_reader::<Wrapper, _>(&bytes[..]).unwrap_err();
        assert!(err.to_string().contains("bytes"));
    }
}
