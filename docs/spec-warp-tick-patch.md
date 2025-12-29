<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP Tick Patch Spec (v1)

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

---

## 1. Data Model

### 1.1 SlotId (unversioned)

`SlotId` identifies a location whose value can change over ticks.

V1 slots:
- `Node(NodeId)` — the full node record at `NodeId` (type id + payload bytes).
- `Edge(EdgeId)` — the full edge record at `EdgeId` (from/to/type/payload).
- `Port(PortKey)` — a boundary port value (opaque key).

Notes:
- V1 treats node/edge records as atomic “values” at their slots. Finer-grained slots (component fields, attachment fragments) can be introduced later.
- The patch does **not** embed version identifiers. Versioning is derived from tick index at slice time.

### 1.2 WarpOp (delta edits)

`WarpOp` is a canonical edit operation on the graph store.

Minimal V1 op set:
- `UpsertNode { node: NodeId, record: NodeRecord }`
- `DeleteNode { node: NodeId }`
- `UpsertEdge { record: EdgeRecord }`
- `DeleteEdge { from: NodeId, id: EdgeId }`

Semantic intent:
- Ops are deterministic edits that, when applied in order, transform `U_i` into `U_{i+1}`.
- Ops are a replay contract and must be stable across languages.

### 1.3 WarpTickPatchV1 (μ)

Canonical patch fields:
- `version: u16 = 1`
- `policy_id: u32`
- `rule_pack_id: Hash` (pin for the producing rule-pack; does not affect replay semantics)
- `commit_status: u8`
  - `1` = Committed
  - `2` = Aborted (reserved for future transactional semantics)
- `in_slots: Set<SlotId>` (sorted, deduped)
- `out_slots: Set<SlotId>` (sorted, deduped)
- `ops: Vec<WarpOp>` (canonical order)

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
3) Port

Within each tag:
- NodeId / EdgeId compare lexicographically on their 32-byte hash values.
- PortKey compares as `u64` in little-endian numeric order.

### 2.2 Op ordering

`ops` must be emitted in canonical order:
1) `DeleteEdge` by (`from`, `id`)
2) `DeleteNode` by (`node`)
3) `UpsertNode` by (`node`)
4) `UpsertEdge` by (`record.from`, `record.id`)

Rationale:
- Deletes occur before upserts (safe for replay).
- Nodes are created/updated before edges that reference them.
- Deterministic, position-independent patch digests require a canonical op order.

---

## 3. patch_digest

`patch_digest` is the BLAKE3 digest of the canonical byte encoding of the patch **core**:

- `patch_version: u16 = 1`
- `policy_id: u32`
- `rule_pack_id: 32 bytes`
- `commit_status: u8`
- `in_slots: Vec<SlotId>` as a length-prefixed list
- `out_slots: Vec<SlotId>` as a length-prefixed list
- `ops: Vec<WarpOp>` as a length-prefixed list

Encoding rules:
- All list lengths are `u64` little-endian.
- All ids (`NodeId`, `EdgeId`, `TypeId`, and `Hash`) are raw 32-byte values.
- Payload bytes are encoded as: `len: u64 LE` then raw bytes.
- Slot encoding:
  - `tag: u8` then tag-specific bytes:
    - Node: `node_id: 32`
    - Edge: `edge_id: 32`
    - Port: `port_key: u64 LE`
- Op encoding:
  - `tag: u8` then tag-specific bytes:
    - UpsertNode: `node_id: 32`, `type_id: 32`, `payload_len: u64`, `payload_bytes`
    - DeleteNode: `node_id: 32`
    - UpsertEdge: `edge_id: 32`, `from: 32`, `to: 32`, `type_id: 32`, `payload_len: u64`, `payload_bytes`
    - DeleteEdge: `from: 32`, `edge_id: 32`

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
