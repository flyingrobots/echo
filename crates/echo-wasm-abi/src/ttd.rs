// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Time Travel Debugging (TTD) types and FFI helpers.

extern crate alloc;
use alloc::string::String;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Privacy mask for field-level redaction in TTD logs.
///
/// Used to prevent PII or sensitive state from leaking into debug recordings
/// while preserving the causal structure of the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PrivacyMask {
    /// No redaction. Field is included as-is.
    Public = 0,
    /// Partial redaction. Value is hashed or truncated.
    Pseudonymized = 1,
    /// Full redaction. Value is replaced with a constant placeholder or dropped.
    Private = 2,
}

/// Opaque token representing a TTD session context across the FFI boundary.
///
/// Prevents raw pointer leakage and provides a handle for session-scoped resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SessionToken(pub u64);

impl From<u64> for SessionToken {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

/// Errors related to TTD session management and FFI.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TtdError {
    /// The session token is invalid or has expired.
    #[error("invalid session token")]
    InvalidToken,
    /// A buffer provided via FFI was too small for the requested data.
    #[error("buffer overflow")]
    BufferOverflow,
    /// A capability was requested that the current session does not possess.
    #[error("permission denied")]
    PermissionDenied,
    /// An internal failure occurred in the TTD controller.
    #[error("internal error: {0}")]
    Internal(String),
}
