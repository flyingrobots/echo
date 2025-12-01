// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical digests and constants used across the engine.
use std::sync::LazyLock;

use crate::ident::Hash;

/// BLAKE3 digest of an empty byte slice.
///
/// Used where canonical empty input semantics are required.
pub static BLAKE3_EMPTY: LazyLock<Hash> = LazyLock::new(|| blake3::hash(&[]).into());

/// Canonical digest representing an empty length-prefix list: BLAKE3 of
/// `0u64.to_le_bytes()`.
///
/// Used for plan/decision/rewrites digests when the corresponding list is empty.
pub static DIGEST_LEN0_U64: LazyLock<Hash> = LazyLock::new(|| {
    let mut h = blake3::Hasher::new();
    h.update(&0u64.to_le_bytes());
    h.finalize().into()
});
