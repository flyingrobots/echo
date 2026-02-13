<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Deep Storage](README.md) | **Priority:** P2

# DiskTier

Persistent blob storage on local filesystem. The `BlobStore` trait exists from Phase 1 (`put`, `get`, `has`, `pin`, `unpin`, `put_verified`). This feature adds a filesystem-backed implementation with content-addressed paths and promotion/demotion between tiers.

---

## T-5-1-1: File-per-blob DiskTier implementation

**User Story:** As a developer, I want blobs persisted to disk so that simulation state survives process restarts.

**Requirements:**

- R1: Implement `BlobStore` for a new `DiskTier` struct that stores blobs as individual files.
- R2: Path layout: `{base_dir}/{hash[0..2]}/{hash[2..4]}/{hash}.blob` (two-level sharding by hex prefix, matching Git's loose object layout).
- R3: Write atomically: write to a temp file then `rename()` into place (crash-safe on POSIX).
- R4: `get` returns `Arc<[u8]>` by reading and caching the file contents (no mmap in Phase 3).
- R5: `has` checks file existence via `std::fs::metadata` (no read).

**Acceptance Criteria:**

- [ ] AC1: `put` + `get` round-trip through a real filesystem directory.
- [ ] AC2: After process restart (drop + re-create DiskTier pointing at same dir), blobs are still retrievable.
- [ ] AC3: Concurrent `put` of the same blob from two threads does not corrupt the file (atomic rename).
- [ ] AC4: Directory sharding produces the expected path for a known hash value.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Single-file blob storage, atomic writes, path sharding, `BlobStore` trait implementation.
**Out of Scope:** Pack files. Compression. Async I/O. LRU read cache (T-5-1-2).

**Test Plan:**

- **Goldens:** For `blob_hash(b"hello echo-cas")`, the file path must match the expected hex-sharded path.
- **Failures:** Read-only directory returns an error from `put`. Missing file on `get` returns `None`. Corrupted file (truncated) detected by hash verification on read.
- **Edges:** Empty blob (0 bytes). Blob exactly at 1 byte. Path with maximum length hash prefix.
- **Fuzz/Stress:** Write 10,000 random blobs, verify all retrievable. Concurrent 4-thread writes of overlapping blobs.

**Blocked By:** none
**Blocking:** T-5-1-2, T-5-2-1

**Est. Hours:** 6h
**Expected Complexity:** ~250 LoC

---

## T-5-1-2: Tiered promotion/demotion (Memory <-> Disk)

**User Story:** As a developer, I want hot blobs cached in memory and cold blobs on disk so that the system balances speed and memory usage.

**Requirements:**

- R1: Implement a `TieredStore` struct that composes `MemoryTier` and `DiskTier`.
- R2: `get` checks memory first, then disk. On disk hit, promote to memory (read-through cache).
- R3: `put` writes to both memory and disk.
- R4: `demote(hash)` evicts from memory tier (blob remains on disk). Called by GC (F5.2).
- R5: `TieredStore` implements `BlobStore` so callers are tier-agnostic.

**Acceptance Criteria:**

- [ ] AC1: After `put`, blob is retrievable from memory (no disk read on `get`).
- [ ] AC2: After `demote`, blob is still retrievable (disk read), and memory tier reports `has` = false.
- [ ] AC3: Promotion on disk-hit: second `get` after a `demote`+`get` sequence serves from memory.
- [ ] AC4: Pin state is consistent across tiers (pinning on tiered store pins in both tiers).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** TieredStore compositor, read-through promotion, explicit demotion, consistent pin state.
**Out of Scope:** Automatic eviction policy (F5.2). Cold tier (S3). Async promotion.

**Test Plan:**

- **Goldens:** N/A (behavioral, not byte-exact).
- **Failures:** Demote a non-existent hash (no-op). Demote a pinned hash (should demote from memory but pin remains).
- **Edges:** `put` to tiered store when disk is read-only (memory put succeeds, disk put returns error -- partial write handling).
- **Fuzz/Stress:** 10,000 put/get/demote cycles with random access patterns; verify no data loss.

**Blocked By:** T-5-1-1
**Blocking:** T-5-2-2

**Est. Hours:** 5h
**Expected Complexity:** ~200 LoC
