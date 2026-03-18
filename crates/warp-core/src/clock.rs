// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Logical clock and generation identifiers for runtime metadata.
//!
//! Echo's internal clocks are logical monotone counters only. They carry no
//! wall-clock or elapsed-time semantics.

macro_rules! logical_counter {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
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
