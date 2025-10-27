//! Transaction identifier types.

/// Thin wrapper around a transaction identifier.
///
/// The engine issues monotonically increasing identifiers via
/// [`crate::Engine::begin`]. External bindings may construct `TxId` values for
/// FFI/Wasm interop using [`TxId::from_raw`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TxId(u64);

impl TxId {
    /// Constructs a `TxId` from a raw `u64` value.
    #[must_use]
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying raw value.
    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for TxId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}
