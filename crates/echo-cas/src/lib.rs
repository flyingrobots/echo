// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Content-addressed blob store for Echo.
//!
//! `echo-cas` provides a [`BlobStore`] trait for content-addressed storage keyed by
//! BLAKE3 hash. Phase 1 ships [`MemoryTier`] — sufficient for the in-browser website
//! demo. Disk/cold tiers, wire protocol, and GC come in Phase 3.
//!
//! # Hash Domain Policy
//!
//! CAS hash is content-only: `BLAKE3(bytes)` with no domain prefix. Two blobs with
//! identical bytes are the same CAS blob regardless of semantic type. This is by
//! design — deduplication is a feature, not a bug. Domain separation happens at the
//! typed-reference layer above (`TypedRef`: `schema_hash` + `type_id` + `layout_hash` +
//! `value_hash`).
//!
//! # Determinism Invariant
//!
//! No public API exposes store iteration order. CAS determinism is content-level
//! (same bytes → same hash), not collection-level. Any future `list`/`iter` API must
//! return results sorted by [`BlobHash`].
#![forbid(unsafe_code)]
#![deny(missing_docs, rust_2018_idioms, unused_must_use)]
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro,
    clippy::print_stdout,
    clippy::print_stderr
)]
#![allow(
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::unreadable_literal,
    clippy::missing_const_for_fn,
    clippy::suboptimal_flops,
    clippy::redundant_pub_crate,
    clippy::many_single_char_names,
    clippy::module_name_repetitions,
    clippy::use_self
)]

mod memory;
pub use memory::MemoryTier;

use std::sync::Arc;

/// A 32-byte BLAKE3 content hash.
///
/// Thin newtype over `[u8; 32]` following the `NodeId`/`TypeId` pattern from
/// `warp-core`. The inner bytes are public for zero-cost access; the `Display`
/// impl renders lowercase hex for logging and error messages.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BlobHash(pub [u8; 32]);

impl BlobHash {
    /// View the hash as a byte slice.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for BlobHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Compute the BLAKE3 content hash of `bytes`.
///
/// No domain prefix — the content IS the identity. See module-level docs for
/// hash domain policy.
pub fn blob_hash(bytes: &[u8]) -> BlobHash {
    let hash = blake3::hash(bytes);
    BlobHash(*hash.as_bytes())
}

/// Errors that can occur during CAS operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CasError {
    /// Blob bytes did not match the declared hash.
    #[error("[CAS_HASH_MISMATCH] expected {expected}, computed {computed}")]
    HashMismatch {
        /// The hash that was declared/expected.
        expected: BlobHash,
        /// The hash actually computed from the bytes.
        computed: BlobHash,
    },
}

/// Content-addressed blob store.
///
/// Implementations store opaque byte blobs keyed by their BLAKE3 hash. The trait
/// is intentionally synchronous and object-safe for Phase 1. Async methods will be
/// added (likely as a separate `AsyncBlobStore` trait) when disk/network tiers
/// demand it.
///
/// # Absence Semantics
///
/// [`get`](BlobStore::get) returns `None` for missing blobs — this is **not** an
/// error. CAS is a lookup table: missing blobs are expected (not-yet-fetched,
/// GC'd, never stored). Error variants are reserved for integrity violations.
pub trait BlobStore {
    /// Compute hash and store. Returns the content hash.
    fn put(&mut self, bytes: &[u8]) -> BlobHash;

    /// Store with a pre-computed hash. Rejects if `BLAKE3(bytes) != expected`.
    ///
    /// On mismatch the store is unchanged and a [`CasError::HashMismatch`] is
    /// returned. This method exists for receivers of `WANT`/`PROVIDE` messages
    /// who already possess the hash.
    ///
    /// # Errors
    ///
    /// Returns [`CasError::HashMismatch`] if the computed hash differs from
    /// `expected`.
    fn put_verified(&mut self, expected: BlobHash, bytes: &[u8]) -> Result<(), CasError>;

    /// Retrieve blob by hash. Returns `None` if not stored — absence is not an
    /// error.
    fn get(&self, hash: &BlobHash) -> Option<Arc<[u8]>>;

    /// Check existence without retrieving.
    fn has(&self, hash: &BlobHash) -> bool;

    /// Mark hash as a retention root.
    ///
    /// Legal on missing blobs (pre-pin intent). Pin semantics are set-based (not
    /// reference-counted) in Phase 1.
    fn pin(&mut self, hash: &BlobHash);

    /// Remove retention root. No-op if not pinned or not stored.
    fn unpin(&mut self, hash: &BlobHash);
}
