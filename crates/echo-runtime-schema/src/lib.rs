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

use core::fmt;

macro_rules! logical_counter {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);

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
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct WorldlineId(pub RuntimeIdBytes);

impl WorldlineId {
    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub const fn as_bytes(&self) -> &RuntimeIdBytes {
        &self.0
    }
}

/// Opaque stable identifier for a head.
#[repr(transparent)]
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
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
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct WriterHeadKey {
    /// The worldline this head targets.
    pub worldline_id: WorldlineId,
    /// The head identity within that worldline.
    pub head_id: HeadId,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{GlobalTick, HeadId, RunId, WorldlineId, WorldlineTick, WriterHeadKey};

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
        let worldline = WorldlineId([3u8; 32]);
        let head = HeadId::from_bytes([7u8; 32]);
        assert_eq!(*worldline.as_bytes(), [3u8; 32]);
        assert_eq!(*head.as_bytes(), [7u8; 32]);
    }

    #[test]
    fn writer_head_key_preserves_typed_components() {
        let key = WriterHeadKey {
            worldline_id: WorldlineId([1u8; 32]),
            head_id: HeadId::from_bytes([2u8; 32]),
        };
        assert_eq!(*key.worldline_id.as_bytes(), [1u8; 32]);
        assert_eq!(*key.head_id.as_bytes(), [2u8; 32]);
    }
}
