// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! In-memory content-addressed blob store.
//!
//! [`MemoryTier`] is the Phase 1 `BlobStore` implementation — sufficient for the
//! in-browser website demo (single tab, no persistence). Disk and cold tiers are
//! deferred to Phase 3.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::{blob_hash, BlobHash, BlobStore, CasError};

/// In-memory content-addressed blob store.
///
/// Stores blobs in a `HashMap<BlobHash, Arc<[u8]>>` and tracks a pin-set for
/// retention roots. An optional byte budget is advisory — `put` always succeeds
/// but [`is_over_budget`](MemoryTier::is_over_budget) reports when the budget is
/// exceeded. Enforcement (eviction of unpinned blobs) is Phase 3 GC's job.
///
/// # Pinning Invariants
///
/// - `pin` on a missing blob is legal (records intent before the blob arrives).
/// - `put` of a pre-pinned hash preserves the pin.
/// - `unpin` on a missing blob is a no-op.
/// - Pin count is set cardinality, not reference count.
pub struct MemoryTier {
    blobs: HashMap<BlobHash, Arc<[u8]>>,
    pins: HashSet<BlobHash>,
    byte_count: usize,
    max_bytes: Option<usize>,
}

impl MemoryTier {
    /// Create an empty store with no byte limit.
    pub fn new() -> Self {
        Self {
            blobs: HashMap::new(),
            pins: HashSet::new(),
            byte_count: 0,
            max_bytes: None,
        }
    }

    /// Create an empty store with an advisory byte budget.
    ///
    /// When the budget is exceeded, [`is_over_budget`](MemoryTier::is_over_budget)
    /// returns `true`. Puts still succeed — enforcement is deferred to Phase 3 GC.
    pub fn with_limits(max_bytes: usize) -> Self {
        Self {
            blobs: HashMap::new(),
            pins: HashSet::new(),
            byte_count: 0,
            max_bytes: Some(max_bytes),
        }
    }

    /// Number of blobs currently stored.
    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    /// Returns `true` if no blobs are stored.
    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    /// Returns `true` if the given hash is in the pin-set.
    pub fn is_pinned(&self, hash: &BlobHash) -> bool {
        self.pins.contains(hash)
    }

    /// Number of hashes in the pin-set.
    pub fn pinned_count(&self) -> usize {
        self.pins.len()
    }

    /// Total bytes stored across all blobs.
    pub fn byte_count(&self) -> usize {
        self.byte_count
    }

    /// Returns `true` if `byte_count` exceeds the configured budget.
    ///
    /// Always returns `false` if no budget was set.
    pub fn is_over_budget(&self) -> bool {
        self.max_bytes.is_some_and(|max| self.byte_count > max)
    }
}

impl Default for MemoryTier {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobStore for MemoryTier {
    fn put(&mut self, bytes: &[u8]) -> BlobHash {
        let hash = blob_hash(bytes);
        if self.blobs.contains_key(&hash) {
            return hash;
        }
        self.byte_count += bytes.len();
        self.blobs.insert(hash, Arc::from(bytes));
        hash
    }

    fn put_verified(&mut self, expected: BlobHash, bytes: &[u8]) -> Result<(), CasError> {
        let computed = blob_hash(bytes);
        if computed != expected {
            return Err(CasError::HashMismatch { expected, computed });
        }
        if !self.blobs.contains_key(&computed) {
            self.byte_count += bytes.len();
            self.blobs.insert(computed, Arc::from(bytes));
        }
        Ok(())
    }

    fn get(&self, hash: &BlobHash) -> Option<Arc<[u8]>> {
        self.blobs.get(hash).cloned()
    }

    fn has(&self, hash: &BlobHash) -> bool {
        self.blobs.contains_key(hash)
    }

    fn pin(&mut self, hash: &BlobHash) {
        self.pins.insert(*hash);
    }

    fn unpin(&mut self, hash: &BlobHash) {
        self.pins.remove(hash);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── 1. put + get round-trip ──────────────────────────────────────────

    #[test]
    fn put_get_round_trip() {
        let mut store = MemoryTier::new();
        let data = b"hello echo-cas";
        let hash = store.put(data);
        let got = store.get(&hash);
        assert!(got.is_some());
        assert_eq!(&*got.unwrap(), data);
    }

    // ── 2. put_verified rejects hash mismatch ───────────────────────────

    #[test]
    fn put_verified_rejects_mismatch() {
        let mut store = MemoryTier::new();
        let bad_hash = BlobHash([0xFF; 32]);
        let result = store.put_verified(bad_hash, b"some bytes");
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CasError::HashMismatch { expected, .. } => {
                assert_eq!(expected, bad_hash);
            }
        }
    }

    // ── 3. put_verified mismatch does NOT mutate store ──────────────────

    #[test]
    fn put_verified_mismatch_leaves_store_unchanged() {
        let mut store = MemoryTier::new();
        let bad_hash = BlobHash([0xFF; 32]);
        let _ = store.put_verified(bad_hash, b"should not be stored");
        assert_eq!(store.len(), 0);
        assert_eq!(store.byte_count(), 0);
    }

    // ── 4. has returns false for missing, true for stored ────────────────

    #[test]
    fn has_missing_and_present() {
        let mut store = MemoryTier::new();
        let hash = blob_hash(b"test");
        assert!(!store.has(&hash));
        store.put(b"test");
        assert!(store.has(&hash));
    }

    // ── 5. put idempotence ──────────────────────────────────────────────

    #[test]
    fn put_idempotence() {
        let mut store = MemoryTier::new();
        let h1 = store.put(b"duplicate");
        let h2 = store.put(b"duplicate");
        assert_eq!(h1, h2);
        assert_eq!(store.len(), 1);
    }

    // ── 6. pre-pin then put ─────────────────────────────────────────────

    #[test]
    fn pre_pin_then_put() {
        let mut store = MemoryTier::new();
        let hash = blob_hash(b"arriving later");
        // Pin before the blob exists.
        store.pin(&hash);
        assert!(store.is_pinned(&hash));
        assert!(!store.has(&hash));
        // Now store the blob.
        let stored_hash = store.put(b"arriving later");
        assert_eq!(hash, stored_hash);
        // Pin must survive the put.
        assert!(store.is_pinned(&hash));
        assert!(store.has(&hash));
    }

    // ── 7. pin/unpin lifecycle ──────────────────────────────────────────

    #[test]
    fn pin_unpin_lifecycle() {
        let mut store = MemoryTier::new();
        let hash = store.put(b"pinnable");
        assert!(!store.is_pinned(&hash));
        store.pin(&hash);
        assert!(store.is_pinned(&hash));
        assert_eq!(store.pinned_count(), 1);
        store.unpin(&hash);
        assert!(!store.is_pinned(&hash));
        assert_eq!(store.pinned_count(), 0);
    }

    // ── 8. unpin on missing blob = no-op ────────────────────────────────

    #[test]
    fn unpin_missing_is_noop() {
        let mut store = MemoryTier::new();
        let hash = BlobHash([0xAA; 32]);
        // Must not panic.
        store.unpin(&hash);
        assert!(!store.is_pinned(&hash));
    }

    // ── 9. get returns None for missing hash ────────────────────────────

    #[test]
    fn get_missing_returns_none() {
        let store = MemoryTier::new();
        let hash = BlobHash([0xBB; 32]);
        assert!(store.get(&hash).is_none());
    }

    // ── 10. empty store invariants ──────────────────────────────────────

    #[test]
    fn empty_store_invariants() {
        let store = MemoryTier::new();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
        assert_eq!(store.byte_count(), 0);
        assert_eq!(store.pinned_count(), 0);
        assert!(!store.is_over_budget());
    }

    // ── 11. byte_count tracks correctly across puts ─────────────────────

    #[test]
    fn byte_count_tracking() {
        let mut store = MemoryTier::new();
        store.put(b"aaaa"); // 4 bytes
        assert_eq!(store.byte_count(), 4);
        store.put(b"bbbbbb"); // 6 bytes
        assert_eq!(store.byte_count(), 10);
        // Duplicate put should NOT add bytes again.
        store.put(b"aaaa");
        assert_eq!(store.byte_count(), 10);
    }

    // ── 12. with_limits + is_over_budget ────────────────────────────────

    #[test]
    fn with_limits_and_over_budget() {
        let mut store = MemoryTier::with_limits(10);
        assert!(!store.is_over_budget());
        store.put(b"12345"); // 5 bytes, within budget
        assert!(!store.is_over_budget());
        store.put(b"1234567"); // +7 = 12, over budget
        assert!(store.is_over_budget());
        // Put still succeeds — budget is advisory.
        assert_eq!(store.len(), 2);
    }

    // ── 13. large blob smoke test ───────────────────────────────────────

    #[test]
    fn large_blob_round_trip() {
        let mut store = MemoryTier::new();
        let big = vec![0x42u8; 8 * 1024 * 1024]; // 8 MiB
        let hash = store.put(&big);
        let got = store.get(&hash);
        assert!(got.is_some());
        assert_eq!(got.unwrap().len(), 8 * 1024 * 1024);
        // Verify the hash matches the free function.
        assert_eq!(hash, blob_hash(&big));
    }

    // ── 14. put convenience returns correct hash ────────────────────────

    #[test]
    fn put_returns_correct_hash() {
        let mut store = MemoryTier::new();
        let data = b"verify hash correctness";
        let expected = blob_hash(data);
        let got = store.put(data);
        assert_eq!(got, expected);
    }
}
