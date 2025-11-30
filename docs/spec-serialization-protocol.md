<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Serialization Protocol Specification (Phase 0.5)

Defines the canonical encoding for Echo’s snapshots, diffs, events, and block manifests. Ensures identical bytes across platforms and supports content-addressed storage.

---

## Principles
- Little-endian encoding for all numeric types.
- IEEE 754 float32/float64 clamped via `Math.fround`; canonical NaN representation (0x7FC00000).
- Maps serialized as sorted arrays by key (lex order).
- Strings UTF-8 encoded with length prefix (uint32).
- All persistent blocks hashed via BLAKE3 hash of canonical bytes.

---

## Primitive Layouts
- `uint8/16/32` – little-endian.
- `int32` – two’s complement, little-endian.
- `float32` – IEEE 754 little-endian; canonical NaN.
- `bool` – uint8 (0 or 1).
- `VarUint` – LEB128 for optional compact ints where size unknown.

---

## Component Schema Encoding
```ts
interface ComponentSchemaRecord {
  typeId: number;
  version: number;
  fields: Array<{ name: string; type: string; offset: number; size: number }>;
}
```

Encoding: for each record
1. `typeId (uint32)`
2. `version (uint32)`
3. `fieldCount (uint16)`
4. For each field (sorted by `name`):
   - `name (string)`
   - `type (string)`
   - `offset (uint32)`
   - `size (uint32)`

Ledger hash = BLAKE3(concat(record bytes)). Stored in snapshot header.

---

## Chunk Payload Encoding
Per chunk:
1. `chunkId (string)`
2. `archetypeId (uint32)`
3. `version (uint64)`
4. `componentCount (uint16)`
5. For each component:
   - `componentType (uint32)`
   - `slotCount (uint32)`
   - `payloadBytesLength (uint32)`
   - `payloadBytes` (raw column data; already canonical due to Float32Array + deterministic order)

Chunk blocks stored individually; referenced by hash.

---

## Diff Encoding
For each `ChunkDiff` (sorted by `chunkId`, `componentType`):
1. `chunkId (string)`
2. `componentType (uint32)`
3. `versionBefore (uint64)`
4. `versionAfter (uint64)`
5. `dirtyBitmapLength (uint32)` + `dirtyBitmapBytes` (Roaring serialized format)
6. `readSetLength (uint32)` + sorted `ReadKey` entries (each: `slot (uint32)`, optional `field (string)`)
7. `writeSetLength (uint32)` + sorted `WriteKey` entries
8. `mergeStrategy (uint16)`
9. `payloadRef (hash)`

Diff hash = BLAKE3(header + chunk diff bytes).

---

## Snapshot Header
1. `schemaLedgerId (hash)`
2. `baseSnapshotId (hash | zero)`
3. `diffChainDepth (uint16)`
4. `chunkRefCount (uint32)`
5. `chunkRefs` (sorted hashes)
6. `cumulativeDiffSize (uint64)`

Snapshot hash = BLAKE3(header + chunkRefs).

---

## Event Encoding
Events use canonical JSON → CBOR-like binary encoding:
1. `id (uint32)`
2. `kind (string)`
3. `chronos (uint64)`
4. `kairos (string)`
5. `aionWeight (float32, optional flag)`
6. `payload` – encoded via domain serializer registered per kind.
7. `prngSpan` – optional block: seedStart (string), count (uint32)
8. `readSet` / `writeSet`
9. `causeIds`
10. `caps`
11. `metadata` (sorted key/value)

Hash → BLAKE3 of encoded bytes. Signature optional (Ed25519).

---

## Block Manifest
Used by persistence to describe relationships among blocks.

```ts
interface BlockManifest {
  nodes: Hash[];
  snapshots: Hash[];
  diffs: Hash[];
  payloads: Hash[];
}
```

Serialized as list of section headers + counts + sorted hashes.

---

## Compression & Signing
- Blocks optionally compressed with Zstandard; header indicates compression (e.g., `magic "ECHO" + version + compression`).
- Signature envelope per block if `signerId` configured.

---

## Determinism Notes
- Always encode maps/sets as sorted arrays.
- Never include timestamps in block hashes.
- Re-encoding the same logical object must produce identical bytes across runtimes.

---

This protocol underpins snapshots, diffs, events, and inspector feeds, enabling reliable persistence, replay, and replication.
