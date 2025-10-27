//! Transaction identifier types.

/// Thin wrapper around an auto-incrementing transaction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TxId(pub u64);

