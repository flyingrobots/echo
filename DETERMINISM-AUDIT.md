<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Determinism Audit for warp-core

**Date:** 2026-01-13
**Auditor:** Claude (with human oversight)

## Executive Summary

### TL;DR
The refactor targeting "serde removal" was attacking the wrong problem. **Serde itself isn't the enemy—non-deterministic data structures (HashMap/HashSet), non-deterministic serialization formats (JSON), and platform-variant float behavior are.**

## Key Findings

### ✅ GOOD: Already Deterministic

1. **Core data structures use BTreeMap/BTreeSet throughout**
   - `tick_patch.rs`: Uses BTreeMap for op deduplication (line 331-338)
   - `snapshot.rs`: Uses BTreeSet for reachability (line 91-92)
   - `scheduler.rs`: Final ready set uses stable radix sort (20-pass LSD, lines 383-450)
   - All SlotIds and ops are explicitly sorted before hashing

2. **Explicit canonical ordering everywhere**
   - WarpOp::sort_key() provides canonical ordering (tick_patch.rs:203-283)
   - Edges sorted by EdgeId before hashing (snapshot.rs:185-194)
   - Scheduler uses deterministic radix sort for candidate ordering

3. **Clean identifier types**
   - All IDs are Blake3 hashes ([u8; 32])
   - All derive PartialOrd, Ord for stable comparison
   - No hidden nondeterminism in identity

### ⚠️ CONCERNS: Needs Investigation

1. **HashMap/HashSet Usage (Non-Critical)**
   - `engine_impl.rs`: HashMap for rule registries (NOT part of state hash)
   - `scheduler.rs`: HashMap for pending txs (intermediate, final output is sorted)
   - `attachment.rs`: HashMap for codec registry (NOT part of state)
   - **Assessment**: These are internal bookkeeping, not part of canonical encoding
   - **Action**: Audit that none of these leak into hashing/signing code paths

2. **Float Usage (f32/f64) - CRITICAL AREA**

   **Location: `math/scalar.rs`**
   - F32Scalar wraps f32 with canonicalization:
     - NaN → 0x7fc00000 (canonical quiet NaN)
     - Subnormals → +0.0
     - -0.0 → +0.0
   - **PROBLEM**: f32 arithmetic itself is NOT deterministic across platforms
     - x87 FPU vs SSE may produce different intermediate results
     - Rounding modes can vary
     - Denormal handling varies by CPU flags

   **Location: `payload.rs`**
   - Heavy f32 usage for motion payloads
   - Comments mention "deterministic quantization to Q32.32"
   - Has v0 (f32) and v2 (fixed-point?) formats
   - `decode_motion_payload_v0`: Reads f32 from bytes (line 258)
   - `encode_motion_payload_v0`: Writes f32 to bytes (line 216)
   - **CRITICAL QUESTION**: Are these f32 values EVER used in:
     - State hashing (compute_state_root)?
     - Patch digests?
     - Receipt digests?
     - Signature inputs?

   **Location: `math/quat.rs`**
   - Uses `[f32; 4]` for quaternion storage
   - Arithmetic operations on quaternions
   - **CONCERN**: Quaternion normalization/multiplication may be non-deterministic

   **ACTION REQUIRED**:
   - Grep for all places F32Scalar/f32 values flow into hash computations
   - Verify motion payloads are ONLY boundary data (not hashed)
   - If floats DO affect hashes, replace with fixed-point (Q32.32 or similar)

3. **Serde Feature Gates**
   - `receipt.rs:123`: Has `#[cfg(feature = "serde")]` for `rule_id_short()` helper
   - `snapshot.rs:77`: Has `#[cfg(feature = "serde")]` for `hash_hex()` helper
   - `math/scalar.rs:110,120`: Serde impls for F32Scalar
   - `serializable.rs`: Entire module was using serde derives
   - **Assessment**: These are UI/debug helpers, NOT canonical encoding
   - **Action**: Can keep serde feature gate for convenience IF:
     - It's ONLY used with deterministic CBOR encoder
     - NEVER used with serde_json
     - JSON is only for debug/view layer

### 🔥 CRITICAL: What Actually Needs Fixing

1. **Enforce CBOR-only wire format**
   - Make deterministic CBOR (echo-wasm-abi) the ONLY protocol boundary
   - JSON can exist for debug/viewing ONLY (never canonical)
   - Add compile-time checks/lints to prevent JSON usage

2. **Audit float usage in hash paths**
   - Search for all paths where f32/F32Scalar flows into:
     - compute_state_root
     - compute_patch_digest
     - compute_tick_receipt_digest
     - Any signature/commitment computation
   - If found, replace with fixed-point Q32.32

3. **Add determinism tests**
   - Test: Encode same patch 1000x → identical bytes
   - Test: Encode across different runs → identical bytes
   - Test: Encode same state → identical state_root
   - Bonus: Cross-compile test (native + wasm) for identical hashes

## Search Targets for Detailed Audit

```bash
# Find HashMap/HashSet in critical paths
rg "HashMap|HashSet" crates/warp-core/src/{snapshot,tick_patch,receipt,cmd}.rs

# Find float usage in critical paths
rg "\bf32\b|\bf64\b|F32Scalar" crates/warp-core/src/{snapshot,tick_patch,receipt,cmd}.rs

# Find serde_json usage (should be ZERO in warp-core)
rg "serde_json" crates/warp-core/

# Find ciborium usage (should be ZERO except in tests)
rg "ciborium::{from_reader|into_writer}" crates/warp-core/src/

# Find all hashing/digest computation sites
rg "Hasher::new|finalize\(\)" crates/warp-core/src/ -A5
```

## What to Revert from Previous Refactor

**REVERT:**
- ❌ Removal of serde from Cargo.toml dependencies (it's fine if used with CBOR)
- ❌ Removal of all `#[cfg_attr(feature = "serde", derive(...))]` annotations

**KEEP:**
1. ✅ Removal of serde_json dependency from warp-core
2. ✅ clippy.toml lint rules forbidding serde_json/ciborium
3. ✅ Manual JSON formatting in telemetry.rs
4. ✅ Use of deterministic CBOR in cmd.rs
5. ✅ Documentation about determinism requirements

## Proposed Refactor Plan (3 Commits)

### Commit 1: Revert overly-aggressive serde removal + document audit
**What:**
- Revert warp-core/Cargo.toml: Add serde back to dependencies
- Revert removed `#[cfg_attr(feature = "serde", ...)]` lines on core types
- Keep serde_json in dev-dependencies only
- Keep clippy lint rules (they prevent serde_json abuse)
- Add this DETERMINISM-AUDIT.md document
- Update CLAUDE-NOTES.md with corrected understanding

**Why:**
- Serde with deterministic CBOR is fine
- The real problem is JSON/HashMap/floats, not serde derives
- We need derives for convenience with CBOR encoding

**Files:**
- `crates/warp-core/Cargo.toml`
- `DETERMINISM-AUDIT.md` (NEW)
- `CLAUDE-NOTES.md`
- Revert cfg_attr removals in: attachment.rs, ident.rs, record.rs, receipt.rs, tx.rs, tick_patch.rs, snapshot.rs, warp_state.rs, graph.rs

**Commit message:**
```
fix(warp): revert overly-aggressive serde removal

The previous refactor incorrectly treated serde as the source of
non-determinism. The real issues are:
1. Non-deterministic data structures (HashMap/HashSet)
2. Non-deterministic formats (JSON)
3. Platform-variant floats

Serde itself is fine when used with deterministic encoders like our
canonical CBOR implementation (echo-wasm-abi).

This commit:
- Restores serde dependency (with derives on core types)
- Keeps serde_json removed from dependencies
- Keeps clippy lints forbidding serde_json
- Adds DETERMINISM-AUDIT.md documenting real risks

Next steps: Audit float usage in hash paths (see DETERMINISM-AUDIT.md)
```

### Commit 2: Complete float determinism audit + add tests
**What:**
- Grep every usage of f32/F32Scalar in snapshot.rs, tick_patch.rs, receipt.rs
- Document findings: Do floats flow into hashes? If yes, replace with Q32.32
- Add determinism tests:
  - test_patch_digest_repeatable: Encode same patch 100x → same bytes
  - test_state_root_repeatable: Compute state_root 100x → same hash
  - test_receipt_digest_repeatable: Encode same receipt 100x → same bytes
- Document in DETERMINISM-AUDIT.md whether floats are safe or need replacement

**Files:**
- `crates/warp-core/src/snapshot.rs` (tests)
- `crates/warp-core/src/tick_patch.rs` (tests)
- `crates/warp-core/src/receipt.rs` (tests)
- `DETERMINISM-AUDIT.md` (updated with audit results)

**Commit message:**
```
test(warp): add determinism audit tests for core hashing

Adds repeatability tests for:
- Patch digest computation
- State root computation
- Receipt digest computation

These tests verify byte-for-byte identical outputs across multiple
encode operations on the same input.

[If floats found in hash paths:]
CRITICAL: Audit revealed f32 usage in [X] - requires follow-up to
replace with fixed-point Q32.32 representation.

[If floats NOT in hash paths:]
Audit confirmed: f32 values are boundary-only and never flow into
canonical hash computation.
```

### Commit 3: Enforce CBOR-only boundary + cleanup
**What:**
- Update serializable.rs to use deterministic CBOR encoding
- Remove remaining #[cfg(feature = "serde")] gates that are now unnecessary
- Add module-level docs explaining: CBOR for wire, JSON for debug only
- Update any wasm boundary code to explicitly use echo-wasm-abi::encode_cbor

**Files:**
- `crates/warp-core/src/serializable.rs`
- `crates/warp-core/src/lib.rs` (docs)
- `crates/warp-wasm/` (ensure CBOR boundary)

**Commit message:**
```
refactor(warp): enforce CBOR-only protocol boundary

Makes deterministic CBOR (echo-wasm-abi) the canonical encoding for
all protocol boundaries. JSON is relegated to debug/view layer only.

Changes:
- serializable.rs uses CBOR encoding only
- Removed unnecessary serde feature gates
- Added docs: "CBOR for protocol, JSON for debug"
- warp-wasm boundary uses explicit CBOR encode/decode

Determinism guarantee: All canonical artifacts (patches, receipts,
snapshots) are encoded via echo-wasm-abi::encode_cbor with:
- Sorted map keys
- Canonical integer/float widths
- No indefinite lengths
- No CBOR tags
```

## Key Principles (Corrected)

1. **Determinism sources are data structures + formats, NOT serde**
   - HashMap/HashSet iteration order → use BTreeMap/BTreeSet ✅ (already done)
   - JSON object key order → use CBOR for wire format ✅ (in progress)
   - Float arithmetic variance → audit + replace if in hash paths ⚠️ (TODO)

2. **CBOR for wire, JSON for debug**
   - Protocol boundary: Always CBOR (echo-wasm-abi)
   - Debug/viewing: JSON is fine (never canonical)
   - No serde_json in warp-core runtime dependencies

3. **Serde is OK with deterministic encoders**
   - serde::Serialize with CBOR → deterministic ✅
   - serde::Serialize with JSON → non-deterministic ❌
   - Keep serde derives for convenience with CBOR

4. **Test everything**
   - Byte-for-byte identical encoding across runs
   - Ideally test native + wasm produce same hashes
   - Test patches, receipts, snapshots independently

## Outstanding Questions

1. **Are motion payload f32 values EVER hashed?**
   - Check: Does AtomPayload flow into any digest computation?
   - If yes: Must replace with Q32.32 fixed-point
   - If no: Boundary-only f32 is acceptable

2. **Do quaternions (Quat) flow into state hashing?**
   - Check: Are Quat values stored in AttachmentValue::Atom?
   - Check: Does snapshot.rs hash quaternion payloads?
   - If yes: Replace with fixed-point representation

3. **Is RFC 8785 (JCS - canonical JSON) needed?**
   - Current plan: No, use CBOR exclusively for wire format
   - JSON only for debug/human-readable views
   - Re-evaluate if JSON wire format is required later

## Next Actions (Immediate)

1. ✅ Create this audit document
2. ⏳ Implement 3-commit refactor plan (see above)
3. ⏳ Run full audit for f32 in hash paths
4. ⏳ Add determinism tests
5. ⏳ Update CLAUDE-NOTES.md with corrected understanding
6. ⏳ Verify full build passes
