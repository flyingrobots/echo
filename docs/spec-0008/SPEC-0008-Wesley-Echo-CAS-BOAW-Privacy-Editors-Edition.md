<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0008 — Wesley + Echo + CAS + BOAW Storage + Privacy

## Editor’s Edition™ (Canonical + Verifiable + Privacy-Safe)

**Version:** 1.0-ee  
**Date:** 2026-01-27  
**Status:** Definitive Reference (Normative + Annotated)  
**Audience:** Implementers of deterministic simulation, time-travel debugging, and content-addressed truth transport

> **Normative keywords:** MUST / MUST NOT / SHOULD / MAY.  
> **Examples:** Code blocks are illustrative unless labeled **REFERENCE IMPLEMENTATION**.

---

## 0. Executive summary

This spec defines one coherent system:

- **Wesley** defines _canonical bytes_ (schemas → codecs → stable identities).
- **Echo/Janus** defines _deterministic computation + receipts_ (ticks → patches → TTDR commitments).
- **CAS** defines _how bytes move and persist_ (hash-addressed blobs, ref-first transport).
- **BOAW storage** defines _how state snapshots scale_ (segment-addressed immutable snapshots, COW, manifest, pin/GC).
- **Privacy mode** defines _what the ledger is allowed to contain_ (claims/proofs/opaque refs), with **two supported privacy mechanisms**:
    1. **ZK proof evidence** (publicly verifiable without revealing bytes)
    2. **Opaque pointer** into an erasable private vault (resolvable by authorized parties)

The result: **deterministic, verifiable, time-travel debuggable simulation** where:

- every byte that matters is canonical,
- every claim is hash-addressed,
- every proof can be verified independently,
- and private data is never “accidentally” committed to a publishable ledger.

---

## 1. Scope

### 1.1 Goals

1. **Cross-platform determinism**: same inputs → same bytes → same hashes (Linux/macOS/Windows; Rust + TS/WASM).
2. **Ref-first truth transport**: receipts can ship hashes; bodies fetched on-demand via CAS.
3. **BOAW-scale snapshots**: immutable snapshot tables with **segment-level structural sharing** (COW) and deterministic GC.
4. **Privacy-safe provenance**: in mind mode, ledger stores only **commitments / proofs / opaque refs / policy hashes**, never secrets.
5. **Footprint enforcement**: rules cannot access undeclared state (build-time + runtime defense-in-depth).

### 1.2 Non-goals

- Replacing TTDR v2 or changing receipt shapes.
- Making the hub parse domain semantics (hub is hash router + cache + quota enforcer).
- Requiring ZK proofs for all private atoms (ZK is supported; not mandated).
- Designing a universal distributed CAS (this spec defines session transport + local stores; distribution is pluggable).

---

## 2. Core objects and identities (normative)

### 2.1 One hash type everywhere

```text
Hash32 = BLAKE3(bytes) = [u8; 32]
```

Hashes are raw 32-byte digests on the wire and in structures.

### 2.2 Canonicality rule (MUST)

A value MUST NOT be hashed unless the bytes being hashed are canonical under an explicit, versioned layout/codec contract.

If you hash non-canonical bytes, you’ve built a platform fingerprinting machine, not a deterministic substrate.

### 2.3 Required identities

- **schema_hash**: `BLAKE3(schema_ir_bytes_canonical)` (preferred)
- **registry_hash**: `BLAKE3(registry_blob_bytes_canonical)`
- **type_id**: stable major identity for a type (breaking changes → new type_id)
- **layout_hash**: exact layout/codec identity (any encoding change → new layout_hash)
- **value_hash**: `BLAKE3(encode(value))` (encode MUST be canonical under layout_hash)

### 2.4 TypedRef (decode gating)

```rust
pub struct TypedRef {
  pub schema_hash: Hash32,
  pub type_id:     Hash32,
  pub layout_hash: Hash32,
  pub value_hash:  Hash32,
}
```

A decoder MUST refuse to decode a `TypedRef` unless:

- it has the schema/registry for `schema_hash`, and
- it has a decoder for `layout_hash`, and
- it verifies `BLAKE3(blob_bytes) == value_hash` before decode, and
- the registry confirms `layout_hash` belongs to `type_id` in that schema universe.

---

## 3. Wesley: canonical bytes factory (normative)

### 3.1 Wesley responsibilities (MUST)

For each schema universe, Wesley MUST generate:

- canonical `encode/decode` for each declared layout (raw_le default)
- `schema_hash` from canonical schema identity bytes (prefer canonical IR bytes)
- `registry.blob` (canonical) and `registry_hash`
- per-type identities: `type_id`, per-layout identities: `layout_hash`
- golden vectors proving Rust and TS/WASM bytes match

### 3.2 raw_le definition (MUST NOT be host layout)

`raw_le` MUST mean explicit, field-by-field encoding with:

- deterministic field order,
- explicit endian rules,
- explicit Option encoding,
- explicit container ordering rules (if any).

Wesley MUST NOT treat host memory layout as canonical. No transmute. No repr(Rust) assumptions.

### 3.3 Option encoding (MUST avoid collisions)

For `Option<T>`, Wesley MUST use a collision-free encoding, e.g.:

- presence bitmap + present values, or
- tag + value.

Sentinel schemes like “0 means None” MUST NOT be used unless the schema forbids that value.

---

## 4. CAS substrate: storage + ref-first transport (normative)

### 4.1 BlobStore (strict, dumb, verifiable)

BlobStore is “bytes by hash,” nothing more.

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

### 4.2 Two-domain storage model (public vs private)

This spec defines two conceptual storage domains:

1. **Public CAS (ledger-safe)**  
   Allowed: schema/registry blobs, patch blobs (redacted if needed), manifests, commitments, ZK proofs (or proof hashes), opaque refs, policy metadata.

2. **Private Vault (erasable)**  
   Allowed: encrypted private bytes (“private atoms”), retrievable only via authorization.

The public ledger MUST remain publishable in mind mode: no sensitive raw bytes.

### 4.3 SchemaUniverse layer (SHOULD exist)

Schema/registry concerns SHOULD live above BlobStore as a `SchemaUniverse` cache:

- fetch schema/registry blobs by hash,
- map layout_hash → decoder,
- enforce TypedRef decode gating.

BlobStore SHOULD NOT grow schema-specific APIs.

---

## 5. BOAW storage integration (normative)

### 5.1 Immutable snapshots + segment-level structural sharing

Snapshots are immutable tables. Copy-on-write (COW) requires structural sharing.

Storage MUST be segment-addressed:

- tables are stored as fixed-size (e.g. 1–4MB) segments,
- each segment is content-addressed by hash,
- commits reference segment hashes rather than rewriting whole snapshots.

**Determinism requirement:** segment boundaries MUST be deterministic for a given canonical table byte stream.

### 5.2 Snapshot manifest (segment directory)

Commit output MUST include a **SnapshotManifest** (canonical bytes) that references:

- format_version
- schema_hash / registry_hash (or references thereto)
- segment hashes for each table (nodes, edges, indexes, attachments, blob arenas)
- root metadata (worldline_id, tick, parents, etc.) as needed for reconstruction

The manifest itself is a blob:

- `manifest_hash = BLAKE3(manifest_bytes)`

### 5.3 WSC packaging as a distribution artifact

WSC MAY remain a single-file distribution format by packing referenced segments.
However, the canonical storage model is:

- manifest + segments (CAS-native)

Packing MUST NOT change meaning; it is an I/O optimization and distribution artifact.

### 5.4 GC policy is pinning (not refcounts)

GC MUST be driven by pinning roots:

- “never delete anything” = pin commits or disable GC
- “free disk” = delete unreachable segments from unpinned commits

Pinned roots define retention; do not maintain a second mutable base graph.

---

## 6. Worldlines + receipts: CAS enablement (normative)

### 6.1 Patch blobs (MVP bridge)

Tick patches MUST be canonical bytes and stored as CAS blobs:

- `patch_bytes = encode(WorldlineTickPatchV*)`
- `patch_digest = BLAKE3(patch_bytes)`
- store under `patch_digest`

In PROOF/LIGHT modes, receipts MAY omit bodies and include `patch_digest` only.

### 6.2 Receipt semantics

Receipt hashes become operational references:

- client can `WANT(patch_digest)`
- provider `PROVIDE(patch_bytes)`
- client verifies hash and decodes/apply/replays

This unlocks “TTD without bodies”: send hashes; fetch bodies when the UI opens an object.

### 6.3 Snapshot/checkpoint blobs

If checkpoints are used, checkpoint manifests and segments MUST also be addressable by hash and retrievable via CAS.

---

## 7. Privacy mode: ZK proof + opaque pointer (normative)

### 7.1 Modes: mind vs diagnostics

- **Mind mode (publishable):** ledger MUST NOT contain sensitive raw bytes. Allowed forms: commitment, ZK proof (or proof hash), opaque private ref, policy hashes, canonical metadata.
- **Diagnostics mode:** richer data MAY be permitted, but still governed by type policy (no accidental logging of sensitive content).

### 7.2 ClaimRecord (canonical) — ledger-safe privacy carrier

A deterministic claim record MUST exist for privacy-safe provenance, containing only publishable forms:

- `claim_key` (stable identity of claim)
- `scheme_id` (ZK/verifier identity)
- `statement_hash` (public statement)
- `commitment` (to secret or ciphertext)
- `proof_bytes` OR `proof_hash` (policy-controlled)
- `private_ref` (optional opaque pointer into an erasable vault)
- `policy_hash` (redaction/disclosure/retention rules)
- `issuer` (rule/subsystem id), tick, etc.

This expresses **two supported privacy mechanisms**:

1. **ZK proof**: provide `proof_bytes` (or `proof_hash` with the proof stored as a public CAS blob)
2. **Opaque pointer**: provide `private_ref` that can resolve to encrypted bytes in a vault

### 7.3 PrivateAtomRefV1 (recommended unified reference)

For patch/truth streams, represent private atoms uniformly:

```text
PrivateAtomRefV1:
  commit:         Hash32        # commitment to canonical plaintext or ciphertext (policy-defined)
  policy_hash:    Hash32
  statement_hash: Hash32        # what the proof attests (or 0-hash if not used)
  zk_evidence:    [Hash32]      # public CAS refs (proof blobs, verifier params, etc.)
  opaque_ref:     Option[Hash32]# public CAS ref to opaque pointer blob
```

Rules:

- If `opaque_ref=None`, it is **ZK-only** (or commitment-only).
- If `zk_evidence=[]`, it is **opaque-pointer-only**.
- Hybrid (both present) is allowed and recommended for “prove publicly, inspect privately.”

### 7.4 Opaque pointer format (MUST be binding + tamper-evident)

Opaque pointers MUST bind to the commitment they claim to represent.

Recommended opaque pointer blob:

```text
OpaqueRefV1:
  vault_id:    Hash32 or short string id (canonicalized)
  locator:     bytes          # opaque to CAS
  commit:      Hash32         # binds pointer to commitment
  alg_id:      Hash32         # encryption scheme id
  policy_hash: Hash32
```

The OpaqueRefV1 blob MUST be stored in **Public CAS** and referenced by hash (`opaque_ref`),
so it cannot be silently altered.

### 7.5 Private vault interface (optional / pluggable)

Supporting external storage for private atoms is OPTIONAL but recommended as a plugin boundary.

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
- Authorization is vault-specific and out of scope; but the ledger MUST remain valid even if vault data disappears.
- If vault data disappears, the system MUST degrade to “commitment/proof remains; payload unavailable.”

---

## 8. Footprint enforcement: build-time + runtime (normative)

### 8.1 Footprint rule (BOAW compatibility)

If something can be mutated, it MUST be representable in the footprint.
This includes adjacency/index targets and any derived caches that are persisted.

### 8.2 Build-time enforcement via Wesley-generated GuardedView (SHOULD)

Wesley SHOULD generate rule-specific views (or capability tokens) that expose only declared reads/writes.

Build-time enforcement is only real if rule code cannot access unrestricted graph APIs.

### 8.3 Runtime enforcement (MUST)

Runtime enforcement MUST remain enabled as defense-in-depth:

- catches unsafe/FFI/WASM bypasses,
- catches generator bugs,
- catches dynamic access outside static bounds.

Acceptable strategies:

- **Plan→Apply fusion** (capabilities derived from footprint), OR
- **FootprintGuard** checks on reads/writes/emissions.

### 8.4 Crate boundary requirement (MUST)

User rules MUST NOT depend on `echo-core` (or any crate that exposes unrestricted graph access).
Rules MUST depend only on `echo-rule-api` (GuardedView surface) + generated artifacts.

If a rule can import raw GraphView/store methods, build-time enforcement becomes theater.

---

## 9. CAS wire v1: WANT / PROVIDE / FRAME (normative)

### 9.1 Messages

Minimum required set:

- WANT v1: request missing blobs by hash
- PROVIDE v1: provide blobs (hash + bytes)
- FRAME v1 (CFRM): advertise relevant refs (raw + typed refs)

Optional:

- HAVE v1 (hints)
- FRAME_PLUS (frame + bundled provides)

### 9.2 Canonical ordering (MUST)

- WANT hashes MUST be sorted and deduped.
- PROVIDE entries MUST be sorted by hash.
- FRAME sections MUST be sorted/deduped by canonical keys.

Violations MUST be rejected deterministically.

---

## 10. Retention + pin/GC (normative)

Pin roots SHOULD include:

- active worldline heads
- active cursors/forks (LRU)
- last N receipts per subscription
- latest checkpoints per worldline
- schema/registry blobs required to decode any still-pinned receipts

GC MAY delete unpinned, unreachable segments/blobs deterministically.
Integrity remains provable: missing blobs are missing; corrupt blobs cannot pass hash verification.

---

## 11. Rollout plan (recommended)

**Wave 1 (parallel):**

- CAS: BlobStore + WANT/PROVIDE + strict verification
- Wesley: schema IR + baseline codegen scaffold

**Wave 2 (MVP value):**

- Store tick patch bytes under `patch_digest`
- PROOF/LIGHT clients fetch patch bodies on demand

**Wave 3 (typed safety):**

- Wesley: stable schema_hash + registry.blob + golden vectors
- CAS: schema/registry distribution as blobs + decode gating

**Wave 4 (BOAW snapshot segmentation):**

- SnapshotManifest + segment hashing + deterministic chunking
- WSC packing/unpacking as distribution artifact

**Wave 5 (privacy hardening):**

- ClaimRecord canonicalization
- ZK evidence blobs in public CAS (policy-controlled)
- OpaqueRefV1 + VaultResolver plugin (optional)

---

## 12. Doctrine line (non-normative)

State is an immutable snapshot. Time is a commit DAG. Writes are patches. Truth is commitments. Privacy is policy. Determinism isn’t optional.
