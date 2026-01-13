<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Claude Development Notes

## 2026-01-13: Determinism Refactor - Corrected Direction

### CRITICAL CORRECTION: We Were Targeting the Wrong Problem

**Previous misconception:** Serde is the source of non-determinism in warp-core.

**Actual reality:** Serde is NOT the problem. The real enemies are:
1. **Non-deterministic data structures** (HashMap/HashSet iteration order)
2. **Non-deterministic serialization formats** (JSON object key ordering)
3. **Platform-variant float behavior** (f32 arithmetic varies by CPU/flags)

### Current Status: COMMIT 1 COMPLETE

**What we're doing:** Reverting the overly-aggressive serde removal while keeping the correct fixes.

**Progress so far:**
- ✅ Created DETERMINISM-AUDIT.md (comprehensive audit of all risks)
- ✅ Restored serde to Cargo.toml as optional dependency with warning comment
- ✅ Added serde feature gate with clear documentation
- ✅ Restored cfg_attr serde derives on all core files (attachment, ident, record, tx, snapshot, receipt, tick_patch, warp_state, graph)
- ✅ Fixed serializable.rs to compile and use serde correctly
- ✅ Fixed cmd_app_state.rs test to use CBOR (enforcing determinism)
- ✅ Added denied-crates enforcement via clippy lints
- ✅ Verified full build passes

### The 3-Commit Refactor Plan

#### Commit 1: Revert + Document (DONE)
**Goal:** Undo the overly-aggressive serde removal while keeping correct fixes.

**Keep (these were correct):**
- ✅ serde_json removed from warp-core dependencies
- ✅ clippy.toml lint rules forbidding serde_json/ciborium
- ✅ Manual JSON formatting in telemetry.rs (not canonical)
- ✅ Deterministic CBOR usage in cmd.rs via echo-wasm-abi

**Revert (these were overly aggressive):**
- ✅ Restore serde to dependencies (optional feature)
- ✅ Restore all `#[cfg_attr(feature = "serde", derive(...))]` lines
- ✅ Fix serializable.rs

**Add (new hardening):**
- ✅ Document in Cargo.toml: "serde feature ONLY for deterministic CBOR, NEVER JSON"
- ✅ Fix cmd_app_state.rs to respect CBOR-only contract

**Commit message template:**
```
fix(warp): revert overly-aggressive serde removal

CRITICAL CORRECTION: The previous refactor incorrectly treated serde as
the source of non-determinism. The real issues are:

1. Non-deterministic data structures (HashMap/HashSet iteration order)
2. Non-deterministic serialization formats (JSON object key ordering)
3. Platform-variant float behavior (f32 arithmetic varies by CPU)

Serde itself is fine when used with deterministic encoders like our
canonical CBOR implementation (echo-wasm-abi).

Changes in this commit:
- Restore serde dependency (optional, with derives on core types)
- Keep serde_json removed from dependencies (correct fix)
- Keep clippy lints forbidding serde_json usage (correct enforcement)
- Fix serializable.rs to compile with serde support
- Fix cmd_app_state.rs test to use CBOR instead of JSON
- Add DETERMINISM-AUDIT.md documenting real risks and audit plan

What we keep from the previous work:
- Deterministic CBOR in cmd.rs (echo-wasm-abi)
- Manual JSON formatting in telemetry.rs (debug only)
- clippy.toml lint rules

Next steps:
- Commit 2: Float determinism audit (CRITICAL - see DETERMINISM-AUDIT.md)
- Commit 3: Enforce CBOR-only protocol boundary

See DETERMINISM-AUDIT.md for detailed audit findings and action plan.
```

#### Commit 2: Float Audit + Determinism Tests (TODO)
**Goal:** Determine if floats flow into canonical hashes. If yes, replace with Q32.32.

**Critical questions to answer:**
1. Do f32/F32Scalar/Quat/motion payloads ever flow into:
   - `compute_state_root()` in snapshot.rs?
   - `compute_patch_digest()` in tick_patch.rs?
   - `compute_tick_receipt_digest()` in receipt.rs?
   - Any signature/commitment computation?

**How to audit:**
```bash
# Search for float usage in critical paths
rg "\bf32\b|\bf64\b|F32Scalar|Quat" crates/warp-core/src/{snapshot,tick_patch,receipt,cmd}.rs

# Trace AtomPayload usage in hashing
rg "AtomPayload" crates/warp-core/src/{snapshot,tick_patch,receipt}.rs -A5

# Check if attachment values (which can contain atoms) are hashed
rg "hash_attachment_value|encode_attachment_value" crates/warp-core/src/
```

**Required tests to add:**
- test_patch_digest_repeatable: Encode same patch 100x → identical bytes
- test_state_root_repeatable: Compute state_root 100x → identical hash
- test_receipt_digest_repeatable: Encode same receipt 100x → identical bytes

**Decision tree:**
- **If floats DO reach canonical hashes:** Must replace with fixed-point Q32.32 (no exceptions)
- **If floats are boundary-only:** Document the isolation and add guards to prevent future drift

**Also audit HashMap usage:**
- Verify that HashMap in engine_impl.rs, scheduler.rs, attachment.rs never leak into canonical encoding
- Either prove they're internal-only OR replace with BTreeMap in critical paths

#### Commit 3: Enforce CBOR-Only Boundary (TODO)
**Goal:** Make deterministic CBOR the ONLY protocol format. JSON is debug/view only.

**Changes:**
- Update serializable.rs: Remove serde derives, use explicit CBOR encode/decode
- Add module docs: "CBOR for wire protocol, JSON for debug only"
- Ensure warp-wasm boundary uses echo-wasm-abi::encode_cbor explicitly
- Remove unnecessary `#[cfg(feature = "serde")]` gates that add no value

### Key Audit Findings (from DETERMINISM-AUDIT.md)

**✅ GOOD: Already Deterministic**
- All core data structures use BTreeMap/BTreeSet
- WarpOp::sort_key() provides canonical ordering
- Scheduler uses stable 20-pass radix sort
- All IDs are Blake3 hashes with stable comparison

**⚠️ CONCERNS: Need Investigation**

1. **HashMap/HashSet Usage**
   - engine_impl.rs: rule registries (probably internal-only)
   - scheduler.rs: pending txs (intermediate, output is sorted)
   - attachment.rs: codec registry (probably internal-only)
   - **Action:** Verify none leak into hashing/signing code paths

2. **Float Usage - CRITICAL**
   - `math/scalar.rs`: F32Scalar canonicalizes NaN/subnormals/negative-zero
   - **BUT** f32 arithmetic itself is NOT deterministic across platforms
   - x87 FPU vs SSE may produce different results
   - `payload.rs`: Heavy f32 usage for motion payloads
   - `math/quat.rs`: Quaternions using [f32; 4]
   - **CRITICAL QUESTION:** Do these flow into state_root/patch_digest/receipt_digest?
   - **If yes:** MUST replace with Q32.32 fixed-point
   - **If no:** Must prove isolation and guard against future drift

### Files Modified So Far

**Completed:**
- `crates/warp-core/Cargo.toml` - restored serde optional dependency
- `crates/warp-core/src/attachment.rs` - restored cfg_attr (5 types)
- `crates/warp-core/src/ident.rs` - restored cfg_attr (6 types)
- `crates/warp-core/src/record.rs` - restored cfg_attr (2 types)
- `crates/warp-core/src/tx.rs` - restored cfg_attr (1 type)
- `crates/warp-core/src/snapshot.rs` - restored cfg_attr (1 type)
- `DETERMINISM-AUDIT.md` - NEW comprehensive audit document

**Still need to restore cfg_attr:**
- `crates/warp-core/src/receipt.rs` - 4 locations
- `crates/warp-core/src/tick_patch.rs` - 5 locations
- `crates/warp-core/src/warp_state.rs` - 2 locations
- `crates/warp-core/src/graph.rs` - 1 location

**Need to fix:**
- `crates/warp-core/src/serializable.rs` - currently has invalid Serialize/Deserialize usage (not imported)

### Next Immediate Actions

1. **Finish Commit 1 restoration:**
   ```bash
   # Restore cfg_attr in remaining 4 files
   # Fix serializable.rs (remove Serialize/Deserialize or add imports)
   # Verify build passes
   # Create commit with detailed message
   ```

2. **Execute Commit 2 - Float audit:**
   ```bash
   # Grep for f32 in critical paths
   # Trace AtomPayload through hashing code
   # Add determinism tests
   # Document findings
   # Replace with Q32.32 if needed
   ```

3. **Execute Commit 3 - CBOR boundary:**
   ```bash
   # Update serializable.rs for CBOR-only
   # Document protocol boundary
   # Remove unnecessary serde gates
   ```

### Search Targets for Float Audit (Commit 2)

```bash
# Find all float usage in critical hash paths
rg "\bf32\b|\bf64\b|F32Scalar" crates/warp-core/src/{snapshot,tick_patch,receipt,cmd}.rs

# Find AtomPayload in hash functions
rg "AtomPayload|hash_attachment_value|encode_attachment_value|hash_atom_payload|encode_atom_payload" crates/warp-core/src/

# Check motion payload usage in state hashing
rg "motion|Motion|velocity|position" crates/warp-core/src/{snapshot,tick_patch,receipt}.rs

# Find quaternion usage in hashing
rg "Quat|quaternion" crates/warp-core/src/{snapshot,tick_patch,receipt}.rs
```

### Key Principles (Corrected Understanding)

1. **Serde is OK with deterministic encoders**
   - serde + CBOR → deterministic ✅
   - serde + JSON → non-deterministic ❌
   - Keep serde derives for convenience, ban serde_json

2. **Determinism comes from data structures + formats, not libraries**
   - HashMap/HashSet → use BTreeMap/BTreeSet
   - JSON objects → use CBOR for wire format
   - Float arithmetic → replace with fixed-point if in hash paths

3. **CBOR for wire, JSON for debug**
   - Protocol boundary: echo-wasm-abi::encode_cbor
   - Debug/viewing: JSON is fine (never canonical)
   - No serde_json in warp-core runtime dependencies

4. **Test everything**
   - Byte-for-byte identical encoding across runs
   - Ideally test native + wasm produce same hashes
   - Test patches, receipts, snapshots independently

### References

- **DETERMINISM-AUDIT.md** - Complete audit findings and 3-commit plan
- **crates/warp-core/.clippy.toml** - Lint rules forbidding serde_json/ciborium
- **crates/echo-wasm-abi/src/canonical.rs** - Deterministic CBOR encoder
