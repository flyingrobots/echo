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

### Current Status: DONE (Commit 3 Complete)

**What we're doing:** Reverting the overly-aggressive serde removal while keeping the correct fixes.

**Progress so far:**
- ✅ Created DETERMINISM-AUDIT.md (comprehensive audit of all risks)
- ✅ Restored serde to Cargo.toml as optional dependency with warning comment
- ✅ Added serde feature gate with clear documentation
- ✅ Restored cfg_attr serde derives on all core files
- ✅ Fixed serializable.rs to compile and use serde correctly
- ✅ Fixed cmd_app_state.rs test to use CBOR (enforcing determinism)
- ✅ Added denied-crates enforcement via clippy lints
- ✅ Verified full build passes
- ✅ Audited float determinism (Confirmed: sensitive to 1 ULP)
- ✅ Added determinism tests (`crates/warp-core/tests/determinism_audit.rs`)
- ✅ Enforced CBOR-only boundary via documentation and dependency isolation

### The 3-Commit Refactor Plan

#### Commit 1: Revert + Document (DONE)
- Restored serde, fixed tests, added audit doc.

#### Commit 2: Float Audit + Determinism Tests (DONE)
- Confirmed sensitivity to float arithmetic.
- Mitigation: `det_fixed` feature for strict consensus.

#### Commit 3: Enforce CBOR-Only Boundary (DONE)
**Goal:** Make deterministic CBOR the ONLY protocol format. JSON is debug/view only.
**Changes:**
- Verified `serde_json` is removed from `warp-core` runtime dependencies.
- Verified `warp-wasm` uses `encode_cbor`.
- Added strict protocol documentation to `warp-core/src/lib.rs`.

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
