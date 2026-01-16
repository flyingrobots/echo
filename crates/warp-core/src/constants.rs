// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical digests and constants used across the engine.
use crate::ident::Hash;

/// Placeholder policy identifier for the current `warp-core` engine spike.
///
/// This value is committed into both `patch_digest` (tick patches) and
/// `commit_id` (commit hash v2) to make the policy boundary explicit, even
/// before Aion policy semantics are implemented.
///
/// The value is intentionally non-zero and is encoded as the ASCII bytes
/// `b"NOP0"` (“NO Policy”, v0) interpreted as a little-endian `u32`.
pub const POLICY_ID_NO_POLICY_V0: u32 = u32::from_le_bytes(*b"NOP0");

/// BLAKE3 digest of an empty byte slice.
///
/// This function is pure and deterministic: it always returns the same [`Hash`]
/// for every invocation across runtimes. Use [`blake3_empty()`] when callers
/// need canonical empty-input semantics.
///
/// # Examples
/// ```
/// let empty = warp_core::blake3_empty();
/// assert_eq!(empty.len(), 32);
/// ```
#[must_use]
pub fn blake3_empty() -> Hash {
    blake3::hash(&[]).into()
}

/// Canonical digest representing an empty length-prefix list: BLAKE3 of
/// `0u64.to_le_bytes()`.
///
/// This is deterministic and produces a platform- and endian-independent,
/// byte-identical 32-byte BLAKE3 digest for the empty length-prefix list.
/// Returns a [`Hash`] derived from `blake3::Hasher::finalize().into()`.
///
/// Used for plan/decision/rewrites digests when the corresponding list is empty.
///
/// # Examples
/// ```
/// let len0_digest = warp_core::digest_len0_u64();
/// assert_eq!(len0_digest.len(), 32);
/// ```
#[must_use]
pub fn digest_len0_u64() -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(&0u64.to_le_bytes());
    h.finalize().into()
}
