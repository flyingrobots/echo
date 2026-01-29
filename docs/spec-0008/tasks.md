<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0008 Task Catalog

This catalog derives tasks from the Editors Edition spec and assigns stable IDs.
Primary spec: `docs/spec-0008/SPEC-0008-Wesley-Echo-CAS-BOAW-Privacy-Editors-Edition.md`

Task ID format: `SPEC-0008.{W|E}.W{wave}.{sequence}`

---

## Index (by track + wave)

### Wesley (W)

- **Wave 1**
    - SPEC-0008.W.W1.1 - Canonical schema IR + schema_hash
    - SPEC-0008.W.W1.2 - raw_le encoder/decoder generation
    - SPEC-0008.W.W1.3 - Option encoding (presence bitmap or tag)
    - SPEC-0008.W.W1.4 - type_id + layout_hash emission + registry.blob
    - SPEC-0008.W.W1.5 - Golden vectors harness (Rust/TS parity)
- **Wave 3**
    - SPEC-0008.W.W3.1 - Emit schema/registry blobs as build artifacts
    - SPEC-0008.W.W3.2 - Expand golden vectors to full schema
    - SPEC-0008.W.W3.3 - GuardedView generation for build-time footprint enforcement
- **Wave 5**
    - SPEC-0008.W.W5.1 - Privacy types in schema (ClaimRecord, PrivateAtomRefV1, OpaqueRefV1)

### Echo (E)

- **Wave 1**
    - SPEC-0008.E.W1.1 - BlobStore trait (verify + pin/unpin)
    - SPEC-0008.E.W1.2 - Memory BlobStore implementation
    - SPEC-0008.E.W1.3 - CAS wire v1 (WANT/PROVIDE/FRAME) with canonical ordering
    - SPEC-0008.E.W1.4 - SchemaUniverse cache above BlobStore
- **Wave 2**
    - SPEC-0008.E.W2.1 - Patch blobs stored in CAS
    - SPEC-0008.E.W2.2 - PROOF/LIGHT receipts with patch_digest
    - SPEC-0008.E.W2.3 - Patch fetch via WANT/PROVIDE in TTD tooling
- **Wave 3**
    - SPEC-0008.E.W3.1 - TypedRef decode gating (4-hash verification)
    - SPEC-0008.E.W3.2 - Schema/registry distribution via CAS + pinning
    - SPEC-0008.E.W3.3 - CasObjectStore envelope layer
    - SPEC-0008.E.W3.4 - Runtime FootprintGuard enforcement
- **Wave 4**
    - SPEC-0008.E.W4.1 - SnapshotManifest canonical storage
    - SPEC-0008.E.W4.2 - Deterministic segmentation + segment blobs
    - SPEC-0008.E.W4.3 - WSC pack/unpack over segments
- **Wave 5**
    - SPEC-0008.E.W5.1 - ClaimRecord handling in ledger
    - SPEC-0008.E.W5.2 - PrivateAtomRefV1 + OpaqueRefV1 validation
    - SPEC-0008.E.W5.3 - VaultResolver plugin + degrade path
    - SPEC-0008.E.W5.4 - Mind mode enforcement
    - SPEC-0008.E.W5.5 - GC retention rules (proofs outlive claims)

---

## Fixtures (used in examples)

### Fixture F1 (Motion, no optionals)

- Input (logical): `Motion { pos_x=1, pos_y=2, pos_z=3, vel_x=None, vel_y=None, vel_z=None }`
- Output bytes (raw_le, hex):
    - `00 01 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00 03 00 00 00 00 00 00 00`

### Fixture F2 (Motion, vel_x=10)

- Input (logical): `Motion { pos_x=1, pos_y=2, pos_z=3, vel_x=10, vel_y=None, vel_z=None }`
- Output bytes (raw_le, hex):
    - `01 01 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00 03 00 00 00 00 00 00 00 0a 00 00 00 00 00 00 00`

### Fixture F3 (Hash32 set)

- `hA = 1111111111111111111111111111111111111111111111111111111111111111`
- `hB = 2222222222222222222222222222222222222222222222222222222222222222`
- `hC = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`

### Fixture F4 (OpaqueRefV1 example)

```
vault_id = hA
locator  = 0x706174682f616263        # "path/abc"
commit   = hB
alg_id   = hC
policy   = hA
```

- Expected: `opaque_ref_hash = BLAKE3(opaque_ref_bytes)` and the opaque ref blob is stored in public CAS under that hash.

---

## Tasks (detailed)

### SPEC-0008.W.W1.1 - Canonical schema IR + schema_hash

- **Description**: Define canonical schema IR bytes and compute `schema_hash = BLAKE3(schema_ir_bytes)`.
- **Requirements**:
    - Canonical IR serialization (no host layout).
    - Deterministic output for same schema across platforms.
- **Acceptance criteria**:
    - `schema_hash` is stable across Rust/TS/WASM builds.
    - Schema IR bytes are reproducible byte-for-byte.
- **Scope**: Schema IR format and hash derivation.
- **Out of scope**: CAS storage or distribution.
- **Required tests**:
    - Golden path: same schema -> same bytes/hash across platforms.
    - Edge cases: field order changes, whitespace-only changes, schema with optional fields.
    - Known failures: reject non-canonical IR input.
    - Fuzz/stress: random schema generation for determinism drift.
- **Example (real input + output)**:
    - Input: Fixture F1 schema containing Motion fields with 3 required + 3 optional.
    - Output: `schema_hash = BLAKE3(schema_ir_bytes)` (must remain stable for the fixture).

### SPEC-0008.W.W1.2 - raw_le encoder/decoder generation

- **Description**: Generate canonical `encode_raw_le` and `decode_raw_le` for all schema types.
- **Requirements**:
    - Field-by-field encoding with explicit endianness.
    - No transmute or host layout usage.
- **Acceptance criteria**:
    - Encoders/decoders produce deterministic bytes for all types.
    - Decoders reject invalid length/format.
- **Scope**: Codegen for raw_le in Rust and TS.
- **Out of scope**: CAS storage.
- **Required tests**:
    - Golden path: Motion encodes to Fixture F1/F2 bytes.
    - Edge cases: max/min i64, empty strings, empty arrays.
    - Known failures: malformed length prefix.
    - Fuzz/stress: random value round-trip encode/decode.
- **Example**:
    - Input: Fixture F1 Motion.
    - Output: raw_le bytes = Fixture F1 hex.

### SPEC-0008.W.W1.3 - Option encoding (presence bitmap or tag)

- **Description**: Implement collision-free Option encoding in codegen.
- **Requirements**:
    - Presence bitmap or explicit tag.
    - Distinguish `Some(0)` from `None`.
- **Acceptance criteria**:
    - Fixture F2 bytes differ from Fixture F1 bytes.
- **Scope**: Option<T> encoding only.
- **Out of scope**: Container ordering rules (unless Option inside container).
- **Required tests**:
    - Golden path: Some(0) != None.
    - Edge cases: all optionals present/absent.
    - Known failures: sentinel encodings rejected.
    - Fuzz/stress: random optional combinations.
- **Example**:
    - Input: Fixture F1 vs F2 Motion.
    - Output: Different byte arrays (presence bit differs).

### SPEC-0008.W.W1.4 - type_id + layout_hash emission + registry.blob

- **Description**: Emit stable `type_id` and `layout_hash` constants and produce `registry.blob`.
- **Requirements**:
    - Deterministic IDs emitted by generator.
    - Registry blob canonical and hashable.
- **Acceptance criteria**:
    - `registry_hash = BLAKE3(registry_bytes)` stable across platforms.
    - Registry maps layout_hash -> type_id correctly.
- **Scope**: ID emission and registry serialization.
- **Out of scope**: Schema distribution via CAS.
- **Required tests**:
    - Golden path: registry hash stable; layout_hash maps to type_id.
    - Edge cases: multiple layouts per type.
    - Known failures: mismatched type_id/layout_hash mapping.
    - Fuzz/stress: random layouts; registry consistency.
- **Example**:
    - Input: Registry entries for Motion + Health types.
    - Output: `registry_hash = BLAKE3(registry_bytes)`; lookups resolve Motion layout -> Motion type_id.

### SPEC-0008.W.W1.5 - Golden vectors harness (Rust/TS parity)

- **Description**: Provide golden tests proving Rust and TS encoders match.
- **Requirements**:
    - Deterministic test runner; cross-platform output compare.
- **Acceptance criteria**:
    - Rust hex == TS hex for fixtures.
- **Scope**: Golden vector harness and initial fixtures.
- **Out of scope**: Full schema coverage (Wave 3).
- **Required tests**:
    - Golden path: Motion F1/F2 parity.
    - Edge cases: empty strings, large arrays.
    - Known failures: mismatch detection with explicit error.
    - Fuzz/stress: random vector batches.
- **Example**:
    - Input: Fixture F2 Motion JSON.
    - Output: Rust hex == TS hex == Fixture F2 bytes.

### SPEC-0008.W.W3.1 - Emit schema/registry blobs as build artifacts

- **Description**: Produce schema IR and registry blobs as build outputs for distribution.
- **Requirements**:
    - Artifacts emitted in predictable paths.
    - Embedded `schema_hash` and `registry_hash` in generated code.
- **Acceptance criteria**:
    - Build produces schema/registry blobs and hashes.
- **Scope**: Build pipeline outputs.
- **Out of scope**: CAS transport.
- **Required tests**:
    - Golden path: build emits stable artifacts.
    - Edge cases: build with no schema changes is no-op.
    - Known failures: missing artifact path fails CI.
    - Fuzz/stress: repeated builds on multiple platforms.
- **Example**:
    - Input: `schemas/ttd-protocol.graphql` (unchanged).
    - Output: `schema_ir.blob`, `registry.blob`, stable hashes.

### SPEC-0008.W.W3.2 - Expand golden vectors to full schema

- **Description**: Extend golden vectors to cover all schema types.
- **Requirements**:
    - Fixtures for each major type.
    - CI gate for parity.
- **Acceptance criteria**:
    - All types pass Rust/TS parity.
- **Scope**: Golden coverage expansion.
- **Out of scope**: CAS features.
- **Required tests**:
    - Golden path: all fixtures match.
    - Edge cases: nested structures, large arrays.
    - Known failures: parity mismatch detection.
    - Fuzz/stress: randomized fixtures per type.
- **Example**:
    - Input: Health fixture `{ hp: 100, max: 100 }`.
    - Output: Rust hex == TS hex.

### SPEC-0008.W.W3.3 - GuardedView generation for build-time footprint enforcement

- **Description**: Generate rule-specific GuardedView APIs that expose only declared reads/writes.
- **Requirements**:
    - Rule code cannot access unrestricted graph APIs.
    - Footprint declarations map to capabilities.
- **Acceptance criteria**:
    - Attempts to access undeclared components fail at compile-time.
- **Scope**: Build-time enforcement via generated APIs.
- **Out of scope**: Runtime FootprintGuard (Echo).
- **Required tests**:
    - Golden path: allowed accesses compile.
    - Edge cases: missing footprint entries.
    - Known failures: direct access to echo-core APIs rejected.
    - Fuzz/stress: generate random footprints, ensure capability surface matches.
- **Example**:
    - Input: Rule declares read Motion only.
    - Output: GuardedView exposes `view.motion()` only; `runtime.edge()` is absent.

### SPEC-0008.W.W5.1 - Privacy types in schema

- **Description**: Add ClaimRecord, PrivateAtomRefV1, OpaqueRefV1 to Wesley schema.
- **Requirements**:
    - Canonical encoding generated for all privacy types.
- **Acceptance criteria**:
    - Rust/TS parity for privacy types.
- **Scope**: Schema definitions + generated codecs.
- **Out of scope**: Policy enforcement in Echo.
- **Required tests**:
    - Golden path: ClaimRecord fixture parity.
    - Edge cases: optional proof bytes vs proof hash.
    - Known failures: invalid enum variant rejected.
    - Fuzz/stress: randomized privacy payloads.
- **Example**:
    - Input: ClaimRecord fixture with commitment `hA` and proof hash `hB`.
    - Output: Deterministic bytes, same hex in Rust/TS.

---

### SPEC-0008.E.W1.1 - BlobStore trait (verify + pin/unpin)

- **Description**: Define BlobStore trait with strict verification and pinning.
- **Requirements**:
    - `put_verified` rejects hash mismatch.
    - `pin/unpin` API present.
- **Acceptance criteria**:
    - Hash mismatch fails deterministically.
- **Scope**: Trait only.
- **Out of scope**: Disk/S3 backends.
- **Required tests**:
    - Golden path: store + retrieve known blob.
    - Edge cases: duplicate puts are idempotent.
    - Known failures: mismatched hash rejected.
    - Fuzz/stress: random blobs, random hashes.
- **Example**:
    - Input: bytes `"hello"`, hash = BLAKE3(bytes).
    - Output: `put_verified` succeeds; `has(hash)` true.

### SPEC-0008.E.W1.2 - Memory BlobStore implementation

- **Description**: Implement in-memory BlobStore with metadata and access tracking.
- **Requirements**:
    - Verified storage only.
    - Deterministic metadata updates.
- **Acceptance criteria**:
    - Tests pass for put/get/head/mark_accessed.
- **Scope**: Hot tier only.
- **Out of scope**: Disk/S3 tiers.
- **Required tests**:
    - Golden path: put/get returns bytes.
    - Edge cases: missing hash returns None.
    - Known failures: non-hot tier rejected.
    - Fuzz/stress: many blobs inserted/removed.
- **Example**:
    - Input: hash hA, bytes "abc".
    - Output: `get(hA)` returns "abc".

### SPEC-0008.E.W1.3 - CAS wire v1 (WANT/PROVIDE/FRAME)

- **Description**: Implement WANT/PROVIDE/FRAME with canonical ordering.
- **Requirements**:
    - Sorted/deduped hashes.
    - Verification on provide.
- **Acceptance criteria**:
    - Messages reject unsorted inputs.
- **Scope**: CAS wire v1 only.
- **Out of scope**: HAVE/FRAME_PLUS (optional).
- **Required tests**:
    - Golden path: WANT([hB,hA]) -> normalized [hA,hB].
    - Edge cases: duplicate hashes removed.
    - Known failures: unsorted PROVIDE rejected.
    - Fuzz/stress: random hash batches.
- **Example**:
    - Input: WANT hashes [hB, hA, hA].
    - Output: canonical WANT hashes [hA, hB].

### SPEC-0008.E.W1.4 - SchemaUniverse cache above BlobStore

- **Description**: Cache schema/registry blobs and map layout_hash -> decoder.
- **Requirements**:
    - Fetch schema/registry by hash.
    - Decoder lookup by layout_hash.
- **Acceptance criteria**:
    - Decoding refuses unknown schema/layout.
- **Scope**: SchemaUniverse only.
- **Out of scope**: CAS transport.
- **Required tests**:
    - Golden path: decode valid TypedRef.
    - Edge cases: missing registry blob.
    - Known failures: layout_hash not found.
    - Fuzz/stress: random TypedRefs; all invalid rejected.
- **Example**:
    - Input: TypedRef with schema_hash=hA, layout_hash=hB.
    - Output: decode fails if registry lacks hB.

### SPEC-0008.E.W2.1 - Patch blobs stored in CAS

- **Description**: Encode WorldlineTickPatch and store as CAS blob under patch_digest.
- **Requirements**:
    - Use Wesley canonical encoding.
    - Store with verified hash.
- **Acceptance criteria**:
    - patch_digest references stored bytes.
- **Scope**: Patch blobs only.
- **Out of scope**: Snapshot manifests.
- **Required tests**:
    - Golden path: patch encode + store + retrieve.
    - Edge cases: empty patch.
    - Known failures: hash mismatch rejected.
    - Fuzz/stress: random patches.
- **Example**:
    - Input: Tick patch with 1 Motion update.
    - Output: patch_digest = BLAKE3(patch_bytes); CAS has blob.

### SPEC-0008.E.W2.2 - PROOF/LIGHT receipts with patch_digest

- **Description**: Add PROOF/LIGHT modes to receipts, including patch_digest only.
- **Requirements**:
    - Receipts omit bodies in PROOF/LIGHT.
- **Acceptance criteria**:
    - Clients can fetch patch bodies on demand.
- **Scope**: Receipt encoding only.
- **Out of scope**: Client fetch implementation.
- **Required tests**:
    - Golden path: PROOF receipt includes patch_digest.
    - Edge cases: LIGHT mode minimal fields.
    - Known failures: receipt missing patch_digest rejected.
    - Fuzz/stress: random receipt variants.
- **Example**:
    - Input: PROOF receipt.
    - Output: `patch_digest = hA`, no patch bytes included.

### SPEC-0008.E.W2.3 - Patch fetch via WANT/PROVIDE in TTD tooling

- **Description**: Client/inspector fetches patch blobs using WANT/PROVIDE.
- **Requirements**:
    - Hash verification before decode.
- **Acceptance criteria**:
    - UI can replay tick after fetching patch.
- **Scope**: TTD tooling fetch path.
- **Out of scope**: Full CAS sync.
- **Required tests**:
    - Golden path: WANT patch_digest -> PROVIDE patch_bytes.
    - Edge cases: missing blob returns error.
    - Known failures: hash mismatch aborts.
    - Fuzz/stress: batch WANTs.
- **Example**:
    - Input: WANT([hA]).
    - Output: PROVIDE(hA, patch_bytes) -> decode succeeds.

### SPEC-0008.E.W3.1 - TypedRef decode gating (4-hash verification)

- **Description**: Enforce schema/layout/type/value hash checks before decode.
- **Requirements**:
    - Verify BLAKE3(bytes) == value_hash.
    - layout_hash belongs to type_id per registry.
- **Acceptance criteria**:
    - Invalid TypedRef always rejected.
- **Scope**: Decode gating only.
- **Out of scope**: Schema distribution.
- **Required tests**:
    - Golden path: valid TypedRef decodes.
    - Edge cases: wrong schema_hash.
    - Known failures: wrong value_hash.
    - Fuzz/stress: random TypedRefs.
- **Example**:
    - Input: TypedRef with value_hash=hA, but blob hashes to hB.
    - Output: decode fails with hash mismatch.

### SPEC-0008.E.W3.2 - Schema/registry distribution via CAS + pinning

- **Description**: Distribute schema/registry blobs via CAS and pin as roots.
- **Requirements**:
    - Use WANT/PROVIDE for schema/registry.
    - Pin blobs required for decoding receipts.
- **Acceptance criteria**:
    - Clients can decode TypedRefs after fetching schema/registry.
- **Scope**: Distribution + pinning.
- **Out of scope**: Schema evolution policy.
- **Required tests**:
    - Golden path: fetch schema blob by hash.
    - Edge cases: missing registry blob prevents decode.
    - Known failures: stale schema_hash rejected.
    - Fuzz/stress: multiple schemas.
- **Example**:
    - Input: WANT(schema_hash=hA).
    - Output: PROVIDE(hA, schema_bytes); SchemaUniverse loads.

### SPEC-0008.E.W3.3 - CasObjectStore envelope layer

- **Description**: Provide typed put/get APIs above BlobStore.
- **Requirements**:
    - Compute TypedRef on put.
    - Decode via SchemaUniverse on get.
- **Acceptance criteria**:
    - put_typed returns correct TypedRef.
- **Scope**: Envelope layer only.
- **Out of scope**: BlobStore implementation.
- **Required tests**:
    - Golden path: put_typed/get_typed round-trip.
    - Edge cases: unknown layout_hash rejected.
    - Known failures: missing schema blob.
    - Fuzz/stress: random typed values.
- **Example**:
    - Input: Motion value Fixture F1.
    - Output: TypedRef(schema_hash, type_id, layout_hash, value_hash).

### SPEC-0008.E.W3.4 - Runtime FootprintGuard enforcement

- **Description**: Enforce declared footprints at runtime as defense-in-depth.
- **Requirements**:
    - Guard read/write/emission operations.
- **Acceptance criteria**:
    - Violations trigger deterministic error/panic.
- **Scope**: Runtime enforcement only.
- **Out of scope**: Build-time GuardedView.
- **Required tests**:
    - Golden path: declared access allowed.
    - Edge cases: undeclared read/write rejected.
    - Known failures: bypass attempts fail.
    - Fuzz/stress: random access patterns.
- **Example**:
    - Input: Rule reads component not in footprint.
    - Output: FootprintGuard violation raised.

### SPEC-0008.E.W4.1 - SnapshotManifest canonical storage

- **Description**: Create canonical SnapshotManifest stored as CAS blob.
- **Requirements**:
    - Include schema_hash, registry_hash, worldline_id, tick, segments.
- **Acceptance criteria**:
    - manifest_hash = BLAKE3(manifest_bytes).
- **Scope**: Manifest only.
- **Out of scope**: Segment chunking.
- **Required tests**:
    - Golden path: deterministic manifest for same snapshot.
    - Edge cases: zero segments.
    - Known failures: non-canonical ordering rejected.
    - Fuzz/stress: large segment lists.
- **Example**:
    - Input: Snapshot with two node segments.
    - Output: manifest_hash = BLAKE3(manifest_bytes).

### SPEC-0008.E.W4.2 - Deterministic segmentation + segment blobs

- **Description**: Split tables into deterministic segments and store as CAS blobs.
- **Requirements**:
    - Segment boundaries deterministic for given byte stream.
- **Acceptance criteria**:
    - Unchanged segments reused across ticks.
- **Scope**: Segmenting + storage.
- **Out of scope**: WSC packaging.
- **Required tests**:
    - Golden path: same table bytes -> same segment boundaries.
    - Edge cases: small tables < segment size.
    - Known failures: nondeterministic boundary detection.
    - Fuzz/stress: large tables.
- **Example**:
    - Input: table bytes length 9MB, segment size 4MB.
    - Output: 3 segments with stable hashes.

### SPEC-0008.E.W4.3 - WSC pack/unpack over segments

- **Description**: Adapt WSC packaging to manifest + segment model.
- **Requirements**:
    - Packing does not alter meaning.
    - Unpacking restores identical CAS blobs.
- **Acceptance criteria**:
    - Unpacked CAS matches original hashes.
- **Scope**: Packaging only.
- **Out of scope**: Network distribution.
- **Required tests**:
    - Golden path: pack -> unpack -> hashes identical.
    - Edge cases: missing segment errors.
    - Known failures: incorrect manifest reference.
    - Fuzz/stress: large archives.
- **Example**:
    - Input: Manifest + 3 segments.
    - Output: WSC file that unpacks to identical hashes.

### SPEC-0008.E.W5.1 - ClaimRecord handling in ledger

- **Description**: Store and validate ClaimRecord objects in public CAS.
- **Requirements**:
    - Canonical encoding via Wesley.
    - Only publishable fields.
- **Acceptance criteria**:
    - ClaimRecord blobs hash-verified and retrievable.
- **Scope**: Ledger integration.
- **Out of scope**: ZK proof verification.
- **Required tests**:
    - Golden path: store + retrieve ClaimRecord.
    - Edge cases: optional proof bytes vs hash.
    - Known failures: raw secret bytes rejected.
    - Fuzz/stress: random ClaimRecords.
- **Example**:
    - Input: ClaimRecord with commitment=hA, proof_hash=hB.
    - Output: CAS blob stored under BLAKE3(bytes).

### SPEC-0008.E.W5.2 - PrivateAtomRefV1 + OpaqueRefV1 validation

- **Description**: Enforce rules for private atom refs and opaque pointer blobs.
- **Requirements**:
    - OpaqueRefV1 binds to commitment.
    - Optional ZK evidence allowed.
- **Acceptance criteria**:
    - Invalid bindings rejected.
- **Scope**: Validation logic.
- **Out of scope**: Vault implementations.
- **Required tests**:
    - Golden path: OpaqueRefV1 with commit=hB accepted.
    - Edge cases: missing opaque_ref for opaque-only flow.
    - Known failures: commit mismatch rejected.
    - Fuzz/stress: random opaque refs.
- **Example**:
    - Input: Fixture F4 opaque ref.
    - Output: `opaque_ref_hash = BLAKE3(opaque_ref_bytes)` stored in public CAS.

### SPEC-0008.E.W5.3 - VaultResolver plugin + degrade path

- **Description**: Define vault interface and behavior when vault data missing.
- **Requirements**:
    - Ledger remains valid without vault payloads.
- **Acceptance criteria**:
    - Missing vault data yields "commitment-only" behavior.
- **Scope**: Interface + error handling.
- **Out of scope**: Vault auth schemes.
- **Required tests**:
    - Golden path: put/get round-trip.
    - Edge cases: missing payload.
    - Known failures: unauthorized access.
    - Fuzz/stress: random payload sizes.
- **Example**:
    - Input: ciphertext bytes "secret".
    - Output: vault ref returned; ledger uses opaque ref hash.

### SPEC-0008.E.W5.4 - Mind mode enforcement

- **Description**: Enforce mind mode policy (no raw secret bytes in ledger).
- **Requirements**:
    - Only commitments, proofs, opaque refs permitted.
- **Acceptance criteria**:
    - Violations rejected deterministically.
- **Scope**: Policy enforcement.
- **Out of scope**: Diagnostics-mode policy details.
- **Required tests**:
    - Golden path: allowed ClaimRecord passes.
    - Edge cases: attempt to store plaintext.
    - Known failures: raw bytes rejected.
    - Fuzz/stress: random payloads.
- **Example**:
    - Input: ClaimRecord containing raw secret bytes.
    - Output: rejection with policy violation.

### SPEC-0008.E.W5.5 - GC retention rules (proofs outlive claims)

- **Description**: Enforce GC constraints: proofs must outlive referencing claims.
- **Requirements**:
    - Reference graph or index to identify proofs.
- **Acceptance criteria**:
    - GC refuses to delete proof blobs still referenced by claims.
- **Scope**: GC policy checks.
- **Out of scope**: GC scheduling/tiering policy.
- **Required tests**:
    - Golden path: proof referenced -> deletion blocked.
    - Edge cases: no references -> delete allowed.
    - Known failures: missing reverse index.
    - Fuzz/stress: large reference graphs.
- **Example**:
    - Input: ClaimRecord references proof hash hA.
    - Output: GC delete(hA) returns error.
