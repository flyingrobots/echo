<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP Tick Patch Spec (v2)

This document defines the **tick patch** boundary artifact used for Paper III-style replay and slicing.

In Paper III terms:
- a worldline boundary encoding is `(U0, P)`, where `P = (μ0, …, μn-1)` is a linear sequence of **tick patches**.
- the patch is the **prescriptive** “what happened” artifact required for deterministic replay.
- receipts/traces are **descriptive** “why this outcome happened” artifacts and are not required for replay.

Echo policy (locked decisions):
- **Delta patch first:** a tick patch is a *delta* (canonical edits), not a “recipe” (re-run rule executors).
- **Unversioned slots in the patch:** the patch contains `in_slots` / `out_slots` as *unversioned* slot identifiers.
  - SSA-style value identity is recovered by interpretation along the worldline:
    - `ValueVersionId := (slot_id, tick_index)` where `tick_index` is the patch’s position in `P`.
- **Commit hash v2 commits only to the patch digest:** `commit_id` commits to the replayable delta, not to planner/scheduler narration.

Patch format note:
- The Rust struct name is still `WarpTickPatchV1`, but the canonical byte encoding committed by
  `patch_digest` is versioned. This spec defines patch encoding **version 2**.

---

## 1. Data Model

### 1.1 SlotId (unversioned)

`SlotId` identifies a location whose value can change over ticks.

V2 slots:
- `Node(NodeKey)` — the skeleton node record at `NodeKey` (instance-scoped).
- `Edge(EdgeKey)` — the skeleton edge record at `EdgeKey` (instance-scoped).
- `Attachment(AttachmentKey)` — an attachment slot value (Atom payload or `Descend` link).
- `Port(PortKey)` — a boundary port value (opaque key).

Notes:
- `NodeKey = { warp_id: WarpId, local_id: NodeId }`.
- `EdgeKey = { warp_id: WarpId, local_id: EdgeId }`.
- Attachment-plane payload atoms are typed: `AtomPayload { type_id: TypeId, bytes: Bytes }`.
- `NodeRecord.ty` / `EdgeRecord.ty` are **skeleton type ids** (schema typing for structure).
- `AtomPayload.type_id` is an **attachment-plane type id** (meaning tag for the bytes).
- V2 treats node/edge records and attachment slot values as atomic “values” at their slots.
  Finer-grained slots (component fields, attachment fragments) can be introduced later.
- The patch does **not** embed version identifiers. Versioning is derived from tick index at slice time.

### 1.2 WarpOp (delta edits)

`WarpOp` is a canonical edit operation on the WARP state.

V2 op set (minimal but recursion-ready):
- `OpenPortal { key: AttachmentKey, child_warp: WarpId, child_root: NodeId, init: PortalInit }`
- `UpsertWarpInstance { instance: WarpInstance }`
- `DeleteWarpInstance { warp_id: WarpId }`
- `UpsertNode { node: NodeKey, record: NodeRecord }`
- `DeleteNode { node: NodeKey }`
- `UpsertEdge { warp_id: WarpId, record: EdgeRecord }`
- `DeleteEdge { warp_id: WarpId, from: NodeId, edge_id: EdgeId }`
- `SetAttachment { key: AttachmentKey, value: Option<AttachmentValue> }`

Note: the order of ops in this list is semantic only; the canonical encoding tag order is defined in section 3.2 and is authoritative.

Semantic intent:
- Ops are deterministic edits that, when applied in order, transform `U_i` into `U_{i+1}`.
- Ops are a replay contract and must be stable across languages.
- Attachment-plane edits must be explicit ops (`SetAttachment`), never “hidden” inside node/edge record bytes.
- Descended attachments should be authored atomically via `OpenPortal` (so portal creation and instance creation cannot be separated across ticks).

### 1.3 WarpTickPatchV1 (μ)

Canonical patch fields:
- `version: u16 = 2`
- `policy_id: u32`
- `rule_pack_id: Hash` (pin for the producing rule-pack; does not affect replay semantics)
- `commit_status: u8`
  - `1` = Committed
  - `2` = Aborted (reserved for future transactional semantics)
- `in_slots: Set<SlotId>` (sorted, deduped)
- `out_slots: Set<SlotId>` (sorted, deduped)
- `ops: Vec<WarpOp>` (canonical order; deduped by canonical sort key)

#### rule_pack_id (v1)

V1 defines `rule_pack_id` as a BLAKE3 digest over the set of registered rule ids:

- `version: u16 = 1`
- `count: u64` number of rule ids
- `rule_ids: Vec<Hash>` where:
  - rule ids are sorted ascending (lexicographic over 32-byte values)
  - duplicates are removed
  - each rule id is encoded as raw 32 bytes

This pins the producing rule registry for auditability while keeping replay semantics rule-engine independent (replay executes `ops` only).

Non-canonical optional metadata (not part of `patch_digest` unless explicitly upgraded):
- tick receipts / traces (e.g., Paper II receipts with blocking-causality witness)
- applied rewrite keys (“how we got here”)

---

## 2. Canonical Ordering

### 2.1 Slot ordering

Slots must be sorted in ascending order with a stable type tag ordering:
1) Node
2) Edge
3) Attachment
4) Port

Within each tag:
- `WarpId`, `NodeId`, `EdgeId`, and `TypeId` compare lexicographically on their 32-byte values.
- `NodeKey` sorts by (`warp_id`, `local_id`).
- `EdgeKey` sorts by (`warp_id`, `local_id`).
- `AttachmentKey` sorts by (`owner_tag`, `plane_tag`, `owner.warp_id`, `owner.local_id`).
- `PortKey` compares as `u64` in little-endian numeric order.

### 2.2 Op ordering

`ops` must be emitted in canonical order (`WarpOp::sort_key`):
1) `OpenPortal` by (`owner_tag`, `plane_tag`, `owner.warp_id`, `owner.local_id`)
2) `UpsertWarpInstance` by (`warp_id`)
3) `DeleteWarpInstance` by (`warp_id`)
4) `DeleteEdge` by (`warp_id`, `from`, `edge_id`)
5) `DeleteNode` by (`node.warp_id`, `node.local_id`)
6) `UpsertNode` by (`node.warp_id`, `node.local_id`)
7) `UpsertEdge` by (`warp_id`, `record.from`, `record.id`)
8) `SetAttachment` by (`owner_tag`, `plane_tag`, `owner.warp_id`, `owner.local_id`)

Rationale:
- Portals are authored atomically (instance creation + `Descend` link) and must be applied before any ops that depend on the child instance.
- Instance metadata is written before instance-scoped content ops.
- Deletes occur before upserts (safe for replay).
- Nodes are created/updated before edges that reference them.
- Attachment writes occur after skeleton writes for the same owners.
- Deterministic, position-independent patch digests require a canonical op order.

---

## 3. patch_digest

`patch_digest` is the BLAKE3 digest of the canonical byte encoding of the patch **core**:

- `patch_version: u16 = 2`
- `policy_id: u32`
- `rule_pack_id: 32 bytes`
- `commit_status: u8`
- `in_slots: Vec<SlotId>` as a length-prefixed list
- `out_slots: Vec<SlotId>` as a length-prefixed list
- `ops: Vec<WarpOp>` as a length-prefixed list

Encoding rules:
- All list lengths are `u64` little-endian.
- All ids (`WarpId`, `NodeId`, `EdgeId`, `TypeId`, and `Hash`) are raw 32-byte values.

### 3.1 Slot encoding

- `tag: u8` then tag-specific bytes:
  - Node (tag `1`): `warp_id: 32`, `node_id: 32`
  - Edge (tag `2`): `warp_id: 32`, `edge_id: 32`
  - Attachment (tag `3`): `attachment_key` (defined below)
  - Port (tag `4`): `port_key: u64 LE`

### 3.2 Op encoding

- `tag: u8` then tag-specific bytes:
  - UpsertWarpInstance (tag `1`):
    - `warp_id: 32`, `root_node: 32`, `parent_key_opt`
  - DeleteWarpInstance (tag `2`):
    - `warp_id: 32`
  - UpsertNode (tag `3`):
    - `warp_id: 32`, `node_id: 32`, `node_type_id: 32`
  - DeleteNode (tag `4`):
    - `warp_id: 32`, `node_id: 32`
  - UpsertEdge (tag `5`):
    - `warp_id: 32`, `from: 32`, `edge_id: 32`, `to: 32`, `edge_type_id: 32`
  - DeleteEdge (tag `6`):
    - `warp_id: 32`, `from: 32`, `edge_id: 32`
  - SetAttachment (tag `7`):
    - `attachment_key`
    - `attachment_value_opt`
  - OpenPortal (tag `8`):
    - `attachment_key` (the portal slot)
    - `child_warp_id: 32`
    - `child_root_node_id: 32`
    - `portal_init`

Implementation note:
- These op tag bytes are part of the patch format for hashing and do not define replay ordering.
  Replay ordering is defined separately by canonical op ordering (section 2.2).

### 3.3 attachment_key

`attachment_key` encodes the identity of an attachment slot:

- `owner_tag: u8` (`1` = Node, `2` = Edge)
- `plane_tag: u8` (`1` = Alpha, `2` = Beta)
- `owner_warp_id: 32`
- `owner_local_id: 32` (raw `NodeId` or `EdgeId` bytes)

### 3.4 parent_key_opt

`parent_key_opt`:

- `present: u8` (`0` = None, `1` = Some)
- when present: `attachment_key`

### 3.5 attachment_value_opt

`attachment_value_opt`:

- `present: u8` (`0` = None, `1` = Some)
- when present:
  - `value_tag: u8` (`1` = Atom, `2` = Descend)
  - if Atom:
    - `payload_type_id: 32`
    - `payload_len: u64 LE`
    - `payload_bytes`
  - if Descend:
    - `child_warp_id: 32`

### 3.6 portal_init

`portal_init` encodes the initialization policy for `OpenPortal`:

- `init_tag: u8`
  - `0` = RequireExisting (require child instance + root already exist)
  - `1` = Empty (create root if missing)
- if `init_tag == 1`:
  - `root_node_type_id: 32` (the `NodeRecord.ty` for the created root node)

---

## 4. Relationship to commit_id (v2)

Commit hash v2 must commit to the replay boundary artifact:
- parents
- state_root
- patch_digest
- policy_id / header version tags

Plan/decision/rewrites digests and receipts are **diagnostics** and must not be committed into `commit_id` unless explicitly upgraded in a later version.

---

## 5. Slicing (Paper III) with unversioned slots

Given a worldline payload `P = (μ0, …, μn-1)` and a target slot `s` in the final state:
- interpretive value identity is `s@i` (“slot s after tick i”).
- producer uniqueness holds by construction: `s@i` is produced by `μi` iff `s ∈ out_slots(μi)`.

Slice extraction for a target value version `s@n`:
1) Find the producing tick `i` (the last tick where `s ∈ out_slots(μi)`; boundary values are from `U0`).
2) Add `μi` to the slice.
3) Enqueue all slots in `in_slots(μi)` and repeat.
4) Emit the collected tick indices in increasing order and take that subsequence of `P`.

Correctness requires conservatism:
- Over-approximate `in_slots` is acceptable (slices get larger but remain correct).
- Under-approximate `in_slots` is forbidden (slices become incorrect).

Descended attachments (Stage B1) integrate by construction:
- execution inside a descended instance reads the attachment keys in its descent chain,
  so the slice for a descendant-produced slot pulls in the portal producers via those reads.
