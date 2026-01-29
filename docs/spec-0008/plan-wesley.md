<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0008 Wesley Track (Derived)

## Canonical Bytes + Schemas (from Editors Edition)

**Source of truth:** `docs/spec-0008/SPEC-0008-Wesley-Echo-CAS-BOAW-Privacy-Editors-Edition.md`

This document extracts the Wesley responsibilities and requirements from the Editors Edition. It is a derived view only.

---

## 1. Purpose and scope

Wesley is the canonical-bytes factory for SPEC-0008. It defines schemas, generates codecs, and emits stable identities so all hashable content is deterministic across Rust and TS/WASM.

**Out of scope:** CAS storage, wire transport, engine integration, privacy enforcement logic. Those are Echo track responsibilities.

---

## 2. Shared contracts Wesley must uphold

### 2.1 Hash32 (one hash type everywhere)

```
Hash32 = BLAKE3(bytes) = [u8; 32]
```

### 2.2 Canonicality rule (MUST)

A value MUST NOT be hashed unless the bytes being hashed are canonical under an explicit, versioned layout/codec contract.

### 2.3 Required identities (MUST)

Wesley must produce or derive these identities for each schema universe:

- `schema_hash` = `BLAKE3(schema_ir_bytes_canonical)` (preferred)
- `registry_hash` = `BLAKE3(registry_blob_bytes_canonical)`
- `type_id` = stable major identity for a type
- `layout_hash` = exact layout/codec identity
- `value_hash` = `BLAKE3(encode(value))` where encoding is canonical under `layout_hash`

### 2.4 TypedRef decode gating (contract)

Echo decoders MUST refuse a `TypedRef` unless:

- the schema/registry for `schema_hash` is present
- a decoder for `layout_hash` exists
- `BLAKE3(blob_bytes) == value_hash` before decode
- registry confirms `layout_hash` belongs to `type_id`

Wesley must therefore emit stable `type_id` and `layout_hash` values, plus schema/registry artifacts that allow this gating.

---

## 3. Wesley responsibilities (MUST)

For each schema universe, Wesley MUST generate:

- canonical `encode/decode` for each declared layout (raw_le default)
- `schema_hash` from canonical schema identity bytes
- `registry.blob` (canonical) and `registry_hash`
- per-type identities: `type_id`
- per-layout identities: `layout_hash`
- golden vectors proving Rust and TS/WASM bytes match

---

## 4. raw_le encoding requirements (MUST)

`raw_le` MUST mean explicit, field-by-field encoding with:

- deterministic field order
- explicit endian rules
- explicit Option encoding
- explicit container ordering rules (if any)

Wesley MUST NOT treat host memory layout as canonical. No transmute. No repr(Rust) assumptions.

---

## 5. Option encoding (MUST avoid collisions)

For `Option<T>`, Wesley MUST use a collision-free encoding, for example:

- presence bitmap + present values, or
- tag + value

Sentinel schemes like "0 means None" MUST NOT be used unless the schema forbids that value.

---

## 6. Schema and registry artifacts

Wesley MUST emit:

- `schema_ir` canonical bytes and `schema_hash`
- `registry.blob` canonical bytes and `registry_hash`

Echo will store these blobs in CAS and pin them as GC roots. Wesley must ensure their bytes are stable and canonical across platforms.

---

## 7. Required schema content (Wesley-defined types)

These types MUST be expressed in Wesley GraphQL schemas so encoders/decoders are canonical and shared across Rust/TS:

- WorldlineTickPatchV\* (patch blobs)
- SnapshotManifest (segment directory)
- ClaimRecord (ledger-safe privacy carrier)
- PrivateAtomRefV1 (privacy reference format)
- OpaqueRefV1 (opaque pointer blob)
- Any domain types whose bytes are hashed and stored in CAS

If a type participates in hashing, CAS storage, or cross-platform verification, it belongs in Wesley schemas.

---

## 8. Build-time footprint enforcement (SHOULD)

Wesley SHOULD generate rule-specific GuardedView (or capability-token) surfaces that only expose declared reads/writes. Build-time enforcement is only real if rule code cannot access unrestricted graph APIs.

---

## 9. Golden vectors (MUST)

Wesley must provide golden vectors and cross-platform tests proving:

- Rust bytes == TypeScript bytes for the same input
- Option encoding distinguishes `Some(0)` from `None`
- Decoders round-trip deterministically

These tests are the determinism proof harness for canonical encoding.

---

## 10. Interfaces to Echo

Wesley outputs consumed by Echo include:

- raw_le encoders/decoders (Rust + TS)
- schema IR blob (`schema_hash`)
- registry blob (`registry_hash`)
- `type_id` and `layout_hash` for all canonical types
- golden vector fixtures and verification tooling

Echo assumes these outputs are stable, canonical, and reproducible across platforms.

---

## 11. Rollout plan (Wesley responsibilities)

From the Editors Edition rollout plan:

- **Wave 1:** schema IR + baseline codegen scaffold (raw_le still incomplete)
- **Wave 3:** stable `schema_hash` + `registry.blob` + golden vectors
- **Wave 5:** ClaimRecord / PrivateAtomRefV1 / OpaqueRefV1 canonicalization support (schema-defined types used by privacy layer)

Wesley work is parallelizable with Echo Wave 1 and Wave 2.
