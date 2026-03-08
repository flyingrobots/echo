<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Serialization Protocol Specification (Phase 0.5)

> **Background:** For a gentler introduction, see [ELI5 Primer](guide/eli5.md) (hashing section).

Defines the canonical encoding for Echo’s snapshots, diffs, events, and block manifests. Ensures identical bytes across platforms and supports content-addressed storage.

---

> **Implementation Status Legend:**
>
> - ✅ **Implemented** — enforced in this repo today (runtime or tests)
> - ⚠️ **Partial** — some pieces exist, others are in-flight
> - 🗺️ **Planned** — vision/aspirational, not yet implemented

## Principles ⚠️

- ✅ Little-endian encoding for numeric lengths/headers in the hashing boundary.
- ⚠️ Canonical floating-point rules exist in math modules; snapshot hashing does not encode floats directly.
- ✅ Ordered iteration is explicit and stable (lexicographic ids; sorted edge ids).
- 🗺️ Strings length-prefixed (uint32) for future block formats; not used by state-root hashing today.
- ⚠️ BLAKE3 used for state root + commit hashes; block hashing is planned.

---

## Primitive Layouts ⚠️

- ✅ `uint8/16/32/64` – little-endian.
- ✅ `bool` – uint8 (0 or 1) when used in hashing tags.
- ⚠️ `int32` – two’s complement, little-endian (not used in snapshot hashing).
- ⚠️ `float32` – IEEE 754 little-endian; canonical NaN (not used in snapshot hashing).
- 🗺️ `VarUint` – LEB128 for optional compact ints where size unknown.

---

## Component Schema Encoding 🗺️

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

## Chunk Payload Encoding 🗺️

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

## Diff Encoding 🗺️

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

## Snapshot Header 🗺️

1. `schemaLedgerId (hash)`
2. `baseSnapshotId (hash | zero)`
3. `diffChainDepth (uint16)`
4. `chunkRefCount (uint32)`
5. `chunkRefs` (sorted hashes)
6. `cumulativeDiffSize (uint64)`

Snapshot hash = BLAKE3(header + chunkRefs).

---

## Event Encoding 🗺️

Events use a canonical binary encoding (typed bytes only):

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

## Block Manifest 🗺️

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

## Compression & Signing 🗺️

- Blocks optionally compressed with Zstandard; header indicates compression (e.g., `magic "ECHO" + version + compression`).
- Signature envelope per block if `signerId` configured.

---

## Determinism Notes ⚠️

- Always encode maps/sets as sorted arrays.
- Never include timestamps in block hashes.
- Re-encoding the same logical object must produce identical bytes across runtimes.

---

## Snapshot Hashing (Implemented Byte Layout) ✅

The current `warp-core` implementation defines a **canonical byte stream** for the state root and commit hashes. The layout below is the exact order used today by `crates/warp-core/src/snapshot.rs`.

```mermaid
flowchart TD
    Root[Root binding<br/>warp_id + local_id] --> Warps[Reachable Warps (sorted)]
    Warps --> Instance[Instance header<br/>warp_id + root_node + parent?]
    Instance --> Nodes[Nodes (sorted NodeId)]
    Nodes --> NodeBody[node_id + node.ty + attachment?]
    NodeBody --> Edges[Edges grouped by from-node]
    Edges --> EdgeHeader[from + edge_count]
    EdgeHeader --> EdgeBody[edge.id + edge.ty + edge.to + attachment?]
    EdgeBody --> Hash[BLAKE3(state_root bytes)]
```

### State Root (`state_root`) ✅

1. **Root binding**:
    - `root.warp_id` (32 bytes raw)
    - `root.local_id` (32 bytes raw)
2. **Per reachable warp instance**, iterated in lexicographic `WarpId` order:
    - `instance.warp_id` (32 bytes)
    - `instance.root_node` (32 bytes)
    - `instance.parent` (presence tag + bytes)
        - `0u8` if `None`
        - `1u8` then `AttachmentKey` bytes if `Some`
3. **Nodes** within the instance, iterated by ascending `NodeId`:
    - `node_id` (32 bytes)
    - `node.ty` (32 bytes)
    - `node.attachment` (presence tag + bytes)
4. **Edges** grouped by `from` node, iterated by ascending `from` `NodeId`:
    - `from` (32 bytes)
    - `edge_count` (`u64` LE)
    - for each edge sorted by `EdgeId`:
        - `edge.id` (32 bytes)
        - `edge.ty` (32 bytes)
        - `edge.to` (32 bytes)
        - `edge.attachment` (presence tag + bytes)

**AttachmentKey encoding** ✅:

- `owner_tag` (1 byte)
- `plane_tag` (1 byte)
- `owner`:
    - Node: `warp_id` (32 bytes) + `local_id` (32 bytes)
    - Edge: `warp_id` (32 bytes) + `local_id` (32 bytes)

**AttachmentValue encoding** ✅:

- `None` → `0u8`
- `Some` → `1u8` followed by:
    - Atom:
        - tag `1u8`
        - `type_id` (32 bytes)
        - `byte_len` (`u64` LE)
        - `payload_bytes` (exact bytes)
    - Descend:
        - tag `2u8`
        - `warp_id` (32 bytes)

`state_root = BLAKE3(canonical_bytes)`

```mermaid
flowchart LR
    V[version = 2 (u16 LE)] --> P[parents_len (u64 LE)]
    P --> Ps[parents hashes (32b each)]
    Ps --> SR[state_root (32b)]
    SR --> PD[patch_digest (32b)]
    PD --> PI[policy_id (u32 LE)]
    PI --> CH[BLAKE3(commit bytes)]
```

### Commit Hash v2 (`commit_id`) ✅

1. `version` (`u16` LE) = `2`
2. `parents_len` (`u64` LE)
3. `parents` (concatenated 32-byte hashes, in provided order)
4. `state_root` (32 bytes)
5. `patch_digest` (32 bytes)
6. `policy_id` (`u32` LE)

`commit_id = BLAKE3(canonical_bytes)`

**Note:** Legacy v1 commit hash includes `plan_digest`, `decision_digest`, and `rewrites_digest`, but is retained for migration tooling only.

---

This protocol underpins snapshots, diffs, events, and inspector feeds, enabling reliable persistence, replay, and replication. Sections marked 🗺️ describe the future target format and are not yet implemented in `warp-core`.
