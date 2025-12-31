// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Transaction identifier types.

/// Thin wrapper around a transaction identifier.
///
/// The engine issues monotonically increasing identifiers via
/// [`crate::Engine::begin`]. External bindings may construct `TxId` values for
/// FFI/Wasm interop using [`TxId::from_raw`].
///
/// # Invariants
/// - The underlying `u64` may wrap at `u64::MAX` (wrapping is intentional).
///   When wrapping occurs, the engine resumes at `1` (skipping zero).
/// - Zero (`TxId(0)`) is reserved as invalid. [`crate::Engine::begin`] never returns zero.
/// - External callers using [`TxId::from_raw`] must not construct `TxId(0)` unless
///   they have a valid reason (e.g., sentinel in FFI); using invalid ids with engine
///   operations returns [`crate::engine_impl::EngineError::UnknownTx`].
///
/// The `#[repr(transparent)]` attribute ensures FFI ABI compatibility: `TxId` has
/// the same memory layout as `u64` across the FFI/Wasm boundary.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TxId(u64);

impl TxId {
    /// Constructs a `TxId` from a raw `u64` value.
    ///
    /// # Note on zero
    /// Constructing `TxId(0)` is allowed, but engine operations treat it as
    /// invalid and will return [`crate::engine_impl::EngineError::UnknownTx`].
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying raw value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for TxId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}
