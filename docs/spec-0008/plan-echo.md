<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0008 Echo Track (Derived)

## CAS + Engine + BOAW + Privacy (from Editors Edition)

**Source of truth:** `docs/spec-0008/SPEC-0008-Wesley-Echo-CAS-BOAW-Privacy-Editors-Edition.md`

This document extracts the Echo responsibilities and requirements from the Editors Edition. It is a derived view only.

---

## 1. Purpose and scope

Echo/Janus owns the deterministic engine, CAS substrate integration, BOAW storage model, receipt semantics, privacy enforcement, and runtime footprint enforcement.

**Depends on Wesley:** canonical encoders/decoders, schema/registry blobs, stable identities (`schema_hash`, `registry_hash`, `type_id`, `layout_hash`).

---

## 2. Shared contracts Echo must honor

### 2.1 Hash32 (one hash type everywhere)

```
Hash32 = BLAKE3(bytes) = [u8; 32]
```

### 2.2 Canonicality rule (MUST)

Echo MUST NOT hash non-canonical bytes. All hashed values must be encoded via Wesley-generated canonical codecs.

### 2.3 TypedRef decode gating (MUST)

Echo decoders MUST refuse to decode a `TypedRef` unless:

- schema/registry for `schema_hash` is present
- decoder for `layout_hash` exists
- `BLAKE3(blob_bytes) == value_hash` before decode
- registry confirms `layout_hash` belongs to `type_id`

---

## 3. CAS substrate (storage + ref-first transport)

### 3.1 BlobStore (strict, dumb, verifiable)

BlobStore is bytes-by-hash, nothing more.

```rust
pub trait BlobStore {
    fn put_verified(&mut self, hash: Hash32, bytes: Vec<u8>) -> Result<(), StoreError>;
    fn get(&self, hash: &Hash32) -> Option<&[u8]>;
    fn has(&self, hash: &Hash32) -> bool;

    fn pin(&mut self, hash: &Hash32);
    fn unpin(&mut self, hash: &Hash32);
}
```

Rules:

- `put_verified` MUST reject if `BLAKE3(bytes) != hash`.
- Receivers of PROVIDE MUST verify each entry before storing.

### 3.2 Two-domain storage model (public vs private)

- **Public CAS (ledger-safe):** schema/registry blobs, patch blobs (redacted if needed), manifests, commitments, ZK proofs (or proof hashes), opaque refs, policy metadata.
- **Private Vault (erasable):** encrypted private bytes retrievable only via authorization.

The public ledger MUST remain publishable in mind mode: no sensitive raw bytes.

### 3.3 SchemaUniverse layer (SHOULD exist)

Schema/registry concerns SHOULD live above BlobStore as a `SchemaUniverse` cache:

- fetch schema/registry blobs by hash
- map `layout_hash` to decoder
- enforce TypedRef decode gating

BlobStore SHOULD NOT grow schema-specific APIs.

---

## 4. BOAW storage integration

### 4.1 Immutable snapshots + segment-level structural sharing

Snapshots are immutable tables. Copy-on-write requires structural sharing.

Storage MUST be segment-addressed:

- tables stored as fixed-size segments
- each segment is content-addressed by hash
- commits reference segment hashes rather than rewriting whole snapshots

Segment boundaries MUST be deterministic for a given canonical table byte stream.

### 4.2 Snapshot manifest (segment directory)

Each commit MUST include a SnapshotManifest (canonical bytes) that references:

- format_version
- schema_hash / registry_hash (or references thereto)
- segment hashes per table (nodes, edges, indexes, attachments, blob arenas)
- root metadata (worldline_id, tick, parents, etc.) as needed

`manifest_hash = BLAKE3(manifest_bytes)`

### 4.3 WSC packaging as distribution artifact

WSC MAY remain a single-file distribution format by packing referenced segments.
Packing MUST NOT change meaning. Canonical storage is manifest + segments.

---

## 5. Worldlines + receipts (CAS enablement)

### 5.1 Patch blobs (MVP bridge)

Tick patches MUST be canonical bytes stored as CAS blobs:

- `patch_bytes = encode(WorldlineTickPatchV*)`
- `patch_digest = BLAKE3(patch_bytes)`
- store under `patch_digest`

In PROOF/LIGHT modes, receipts MAY omit bodies and include `patch_digest` only.

### 5.2 Receipt semantics

Receipt hashes become operational references:

- client can `WANT(patch_digest)`
- provider `PROVIDE(patch_bytes)`
- client verifies hash and decodes/applies/replays

This enables TTD without bodies: send hashes, fetch bodies when needed.

### 5.3 Snapshot/checkpoint blobs

If checkpoints are used, checkpoint manifests and segments MUST be addressable by hash and retrievable via CAS.

---

## 6. Privacy mode (ZK proof + opaque pointer)

### 6.1 Modes: mind vs diagnostics

- **Mind mode (publishable):** ledger MUST NOT contain sensitive raw bytes. Allowed: commitments, ZK proofs (or proof hashes), opaque refs, policy hashes, canonical metadata.
- **Diagnostics mode:** richer data MAY be permitted but remains policy-gated.

### 6.2 ClaimRecord (canonical)

ClaimRecord is the deterministic, ledger-safe privacy carrier containing only publishable forms:

- `claim_key`
- `scheme_id`
- `statement_hash`
- `commitment`
- `proof_bytes` OR `proof_hash`
- `private_ref` (optional opaque pointer)
- `policy_hash`
- `issuer`, tick, etc.

### 6.3 PrivateAtomRefV1 (recommended unified reference)

```
PrivateAtomRefV1:
  commit:         Hash32        # commitment to canonical plaintext or ciphertext
  policy_hash:    Hash32
  statement_hash: Hash32        # what the proof attests (or 0-hash if not used)
  zk_evidence:    [Hash32]      # public CAS refs (proof blobs, verifier params, etc.)
  opaque_ref:     Option[Hash32]# public CAS ref to opaque pointer blob
```

Rules:

- If `opaque_ref=None`, it is ZK-only (or commitment-only).
- If `zk_evidence=[]`, it is opaque-pointer-only.
- Hybrid is allowed and recommended for "prove publicly, inspect privately."

### 6.4 Opaque pointer format (MUST be binding + tamper-evident)

Recommended opaque pointer blob:

```
OpaqueRefV1:
  vault_id:    Hash32 or short string id (canonicalized)
  locator:     bytes          # opaque to CAS
  commit:      Hash32         # binds pointer to commitment
  alg_id:      Hash32         # encryption scheme id
  policy_hash: Hash32
```

OpaqueRefV1 blob MUST be stored in public CAS and referenced by hash, so it cannot be silently altered.

### 6.5 Private vault interface (optional / pluggable)

```rust
pub trait VaultResolver {
    type Ref; // opaque (bytes blob or structured)

    fn put(&mut self, ciphertext: Vec<u8>) -> Result<Self::Ref, VaultError>;
    fn get(&self, r: &Self::Ref) -> Result<Vec<u8>, VaultError>;
    fn has(&self, r: &Self::Ref) -> bool;
}
```

Requirements:

- Vault storage MUST be treated as erasable (it may delete data).
- Authorization is vault-specific and out of scope.
- Ledger MUST remain valid even if vault data disappears.

---

## 7. Footprint enforcement

### 7.1 Footprint rule (BOAW compatibility)

If something can be mutated, it MUST be representable in the footprint, including adjacency/index targets and persisted caches.

### 7.2 Runtime enforcement (MUST)

Runtime enforcement MUST remain enabled as defense-in-depth:

- catches unsafe/FFI/WASM bypasses
- catches generator bugs
- catches dynamic access outside static bounds

Acceptable strategies:

- Plan-to-Apply fusion, or
- FootprintGuard checks on reads/writes/emissions

### 7.3 Crate boundary requirement (MUST)

User rules MUST NOT depend on `echo-core` (or any crate that exposes unrestricted graph access). Rules MUST depend only on `echo-rule-api` + generated artifacts.

---

## 8. CAS wire v1 (WANT / PROVIDE / FRAME)

### 8.1 Required messages

- WANT v1: request missing blobs by hash
- PROVIDE v1: provide blobs (hash + bytes)
- FRAME v1 (CFRM): advertise relevant refs (raw + typed refs)

Optional:

- HAVE v1 (hints)
- FRAME_PLUS (frame + bundled provides)

### 8.2 Canonical ordering (MUST)

- WANT hashes MUST be sorted and deduped.
- PROVIDE entries MUST be sorted by hash.
- FRAME sections MUST be sorted and deduped by canonical keys.

Violations MUST be rejected deterministically.

---

## 9. Retention + pin/GC

Pin roots SHOULD include:

- active worldline heads
- active cursors/forks (LRU)
- last N receipts per subscription
- latest checkpoints per worldline
- schema/registry blobs required to decode any still-pinned receipts

GC MAY delete unpinned, unreachable segments/blobs deterministically. Missing blobs are missing; corrupt blobs cannot pass hash verification.

---

## 10. Rollout plan (Echo responsibilities)

From the Editors Edition rollout plan:

- **Wave 1 (parallel):** CAS BlobStore + WANT/PROVIDE + strict verification
- **Wave 2 (MVP value):** store patch bytes under `patch_digest`; PROOF/LIGHT clients fetch bodies on demand
- **Wave 3 (typed safety):** schema/registry distribution as blobs + TypedRef decode gating
- **Wave 4 (BOAW segmentation):** SnapshotManifest + deterministic chunking; WSC pack/unpack
- **Wave 5 (privacy hardening):** ClaimRecord canonicalization, ZK evidence blobs, OpaqueRefV1 + VaultResolver plugin

Echo work is dependent on Wesley outputs where noted in Sections 2 and 3.
