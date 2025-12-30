<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# `warp-core` — WARP Core Runtime & API Tour

This document is a **tour of the `warp-core` crate**: the core data model,
deterministic boundary artifacts, and the runtime APIs that higher layers (`warp-ffi`,
`warp-wasm`, tools, and eventually the full Echo runtime) build on.

If you only remember one thing:

> `warp-core` is where “WARP is real” — deterministic state, deterministic ticks,
> deterministic hashes, and replayable deltas.

Related docs (recommended, in order):

1. `docs/warp-two-plane-law.md` — project law for SkeletonGraph vs attachment plane.
2. `docs/spec-merkle-commit.md` — state_root vs commit_id and what is committed.
3. `docs/spec-warp-tick-patch.md` — tick patch boundary artifact (Paper III).
4. `docs/spec/SPEC-0002-descended-attachments-v1.md` — WarpInstances / descended attachments (Stage B1).

---

## 1. Mental model (what the crate implements)

`warp-core` implements a **two-plane** WARP state object:

- **Skeleton plane (structure):** nodes + edges + boundary ports (matching/scheduling operates here).
- **Attachment plane (payloads):** typed atoms (depth-0) and explicit descent indirection (Stage B1).

In Paper I notation, this is “WarpState `U`” and its projection `π(U)`:

```text
U := (SkeletonGraph, Attachments)
π(U) = SkeletonGraph
```

The hot path is intentionally defined over `π(U)`:

- matching/indexing never decodes attachments
- attachments are decoded only when a rule explicitly chooses to do so

The crate also defines the deterministic “history boundary artifacts” used for audit/replay/slicing:

- **Snapshot** (state root + commit id)
- **TickReceipt** (Paper II: accept/reject outcomes + optional blocking witness)
- **WarpTickPatchV1** (Paper III: replayable delta ops + conservative in/out slots)

---

## 2. Public API surface (what `warp-core` exports)

The stable public surface is intentionally exposed via re-exports from
`crates/warp-core/src/lib.rs`. At a high level:

- **Identifiers:** `Hash`, `NodeId`, `EdgeId`, `TypeId`, `WarpId`, plus instance-scoped keys `NodeKey`, `EdgeKey`.
- **State & storage:** `GraphStore`, `WarpState`, `WarpInstance`.
- **Attachments:** `AtomPayload`, `AttachmentKey`, `AttachmentValue`, and the codec boundary (`Codec`, `CodecRegistry`, `DecodeError`).
- **Rules:** `RewriteRule`, `PatternGraph`, `ConflictPolicy`.
- **Scheduling & MWMR:** `Footprint`, `PortKey`.
- **Runtime:** `Engine`, `EngineError`, `ApplyResult`.
- **Boundary artifacts:** `Snapshot`, `TickReceipt`, `WarpTickPatchV1`, `WarpOp`, `SlotId`.
- **Utilities:** demo builders (`build_motion_demo_engine`), payload helpers (`encode_motion_atom_payload`, etc.).

This doc describes those pieces and how they fit.

---

## 3. Identifiers (stable bytes, stable meaning)

All core ids are **32-byte values** (`type Hash = [u8; 32]`) and are treated as
raw bytes at the deterministic boundary.

Key types (from `ident.rs`):

- `NodeId(Hash)` — local node identity inside one warp instance.
- `EdgeId(Hash)` — local edge identity inside one warp instance.
- `WarpId(Hash)` — namespacing identity for Stage B1 WarpInstances (“layers”).
- `TypeId(Hash)` — meaning tag for either skeleton typing (node/edge record types) or attachment atoms.

Stage B1 adds *instance-scoped keys*:

- `NodeKey { warp_id: WarpId, local_id: NodeId }`
- `EdgeKey { warp_id: WarpId, local_id: EdgeId }`

Interpretation rule:

- A `NodeId` is not globally unique across all instances; **`NodeKey` is**.

Construction helpers:

- `make_node_id(label)`, `make_edge_id(label)`, `make_type_id(label)`, `make_warp_id(label)`
  produce deterministic ids by BLAKE3 hashing domain-separated labels.

---

## 4. Storage model: `GraphStore` (one instance) + `WarpState` (many instances)

### 4.1 `GraphStore`: per-instance storage

`GraphStore` is the in-memory store for one warp instance (one `warp_id`):

- Skeleton plane:
  - `nodes: BTreeMap<NodeId, NodeRecord>`
  - `edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>` (adjacency buckets)
  - `edges_to: BTreeMap<NodeId, Vec<EdgeId>>` (reverse adjacency, used for fast deletes)
- Attachment plane (stored separately, but co-located in the struct):
  - `node_attachments: BTreeMap<NodeId, AttachmentValue>` (node-attachment plane)
  - `edge_attachments: BTreeMap<EdgeId, AttachmentValue>` (edge-attachment plane)
- Reverse indexes:
  - `edge_index: BTreeMap<EdgeId, NodeId>` (EdgeId → from)
  - `edge_to_index: BTreeMap<EdgeId, NodeId>` (EdgeId → to)

Design intent:

- `BTreeMap` is used to guarantee deterministic key iteration.
- Edge buckets preserve insertion order, but deterministic processes (hashing, patch diff)
  explicitly sort by `EdgeId` when order matters.

### 4.2 `WarpState`: multi-instance WARP state (Stage B1)

`WarpState` is the Stage B1 two-level container:

- `stores: BTreeMap<WarpId, GraphStore>`
- `instances: BTreeMap<WarpId, WarpInstance>`

`WarpInstance` is the metadata record that makes descended attachments sliceable:

- `warp_id: WarpId`
- `root_node: NodeId` (local id inside that instance’s store)
- `parent: Option<AttachmentKey>` (the attachment slot that descends into this instance; `None` for the root instance)

Important: `WarpInstance.parent` is what enables “include the portal chain” slicing
without searching the entire attachment plane.

---

## 5. Attachments: typed atoms + explicit descent

Attachments exist in a distinct plane and are addressed by first-class slot keys.

### 5.1 Slot identity: `AttachmentKey`

An attachment slot is identified by:

- `AttachmentOwner` = `Node(NodeKey)` or `Edge(EdgeKey)`
- `AttachmentPlane` = `Alpha` (node-owned) or `Beta` (edge-owned)

Invariant (project law):

- node-owned attachments use **Alpha**
- edge-owned attachments use **Beta**

### 5.2 Attachment values

`AttachmentValue` is:

- `Atom(AtomPayload)` — depth-0 typed bytes
- `Descend(WarpId)` — Stage B1 “flattened indirection” to another instance

`AtomPayload` is:

- `type_id: TypeId`
- `bytes: Bytes`

Determinism rules:

- canonical encodings and hashes include the payload `type_id` as well as the bytes
- attachment bytes must not hide structural dependencies (“no hidden edges”)

### 5.3 Codec boundary (safe typing)

The engine does not decode attachments in matching/indexing. Typed boundaries use:

- `trait Codec<T> { const TYPE_ID: TypeId; fn encode_canon(&T)->Bytes; fn decode_strict(&Bytes)->Result<T, DecodeError>; }`
- `AtomPayload::decode_for_match` encodes the v0 decode-failure policy:
  - type mismatch or decode error ⇒ “rule does not apply”

---

## 6. Rewrite model: rules + footprints + deterministic scheduling

### 6.1 `RewriteRule`

A registered rule is a `RewriteRule`:

- `id: Hash` (rule family id; stable and deterministic)
- `name: &'static str` (human name used by `Engine::apply`)
- `matcher: fn(&GraphStore, &NodeId) -> bool`
- `executor: fn(&mut GraphStore, &NodeId)`
- `compute_footprint: fn(&GraphStore, &NodeId) -> Footprint`
- `factor_mask: u64` (fast prefilter; must remain conservative)
- `conflict_policy: ConflictPolicy` (`Abort`, `Retry`, `Join`)

Today’s spike uses `matcher`/`executor` directly; the `PatternGraph` field is
kept for forward compatibility with richer pattern systems.

### 6.2 `Footprint`: MWMR independence contract

`Footprint` is the engine’s “resources touched” summary. It is used for:

- conflict detection (write/write, write/read, boundary port overlap)
- explaining rejections (TickReceipt blocker witness)
- conservative in/out slot derivation for tick patches

Stage B1 adds attachment slots to the footprint sets so descent-chain reads can be recorded.

### 6.3 Deterministic ordering: `scope_hash`

`scope_hash(rule_id, scope: &NodeKey) -> Hash` is the canonical “scheduler sort key” seed.
This is exported so tests and tooling can compute the same order as the engine.

---

## 7. Runtime: `Engine` and the tick pipeline

### 7.1 Transactions

`Engine` is a single-process deterministic engine:

1. `let tx = engine.begin();`
2. `engine.apply(tx, RULE_NAME, &scope_node_id)?;` (enqueue candidates)
3. `let snapshot = engine.commit(tx)?;` (deterministically reserve + execute + hash)

`Engine::apply_in_warp` is the Stage B1 variant that targets a specific instance (`warp_id`)
and carries a `descent_stack: &[AttachmentKey]` for the descent-chain footprint law.

### 7.2 Commit outputs: `Snapshot`, `TickReceipt`, `WarpTickPatchV1`

There are two commit APIs:

- `Engine::commit(tx) -> Result<Snapshot, EngineError>`
- `Engine::commit_with_receipt(tx) -> Result<(Snapshot, TickReceipt, WarpTickPatchV1), EngineError>`

The `commit_with_receipt` API is the “full boundary artifact” path: it produces the
delta patch and the receipt alongside the snapshot id.

---

## 8. Deterministic boundary artifacts (hashing, replay, slicing)

### 8.1 `Snapshot`: `state_root` + `commit_id`

`Snapshot.hash` is the deterministic commit id (`commit_id`).

Commit hash v2 commits to:

- explicit `parents`
- `state_root` (graph-only, canonical)
- `patch_digest` (replayable delta)
- `policy_id`

Plan/decision/rewrites digests remain deterministic diagnostics but are *not* committed by v2.
See `docs/spec-merkle-commit.md` for the canonical encoding.

### 8.2 `TickReceipt`: Paper II outcomes

`TickReceipt` records, in canonical plan order, whether each candidate was:

- `Applied`, or
- `Rejected(FootprintConflict)`

Additionally, `TickReceipt::blocked_by(i)` returns a sorted list of prior indices
that blocked candidate `i` (a minimal blocking-causality witness).

### 8.3 `WarpTickPatchV1`: Paper III replayable delta

The tick patch is the “what happened” boundary artifact:

- conservative `in_slots` / `out_slots` (unversioned `SlotId`s)
- canonical delta ops (`WarpOp`) such as `UpsertNode`, `DeleteEdge`, `SetAttachment`, `OpenPortal`, etc.
- `digest()` commits to the canonical v2 patch encoding (see `docs/spec-warp-tick-patch.md`)

Worldline slicing uses the Paper III interpretation rule:

- the patch does not embed versions; versions are recovered by patch position in `P = (μ0…μn-1)`.

---

## 9. Stage B1 recursion: WarpInstances + portals

`warp-core` supports “WARPs all the way down” without recursive traversal:

- descended attachments are explicit `AttachmentValue::Descend(WarpId)`
- the child instance is a separate namespace (`WarpInstance` + `GraphStore`)
- portals are authored atomically in patches via `WarpOp::OpenPortal`

Crucial correctness law:

- any match/exec inside a descended instance must READ the attachment keys in its descent chain
  (so changing a portal pointer deterministically invalidates matches)

---

## 10. Where to read code (module tour)

Start here (in order):

- `crates/warp-core/src/lib.rs` — public exports and crate-level invariants.
- `crates/warp-core/src/ident.rs` — id types, constructors, stable byte accessors.
- `crates/warp-core/src/record.rs` — skeleton records (`NodeRecord`, `EdgeRecord`).
- `crates/warp-core/src/graph.rs` — `GraphStore` layout and invariants.
- `crates/warp-core/src/attachment.rs` — typed atoms, descent values, codec boundary.
- `crates/warp-core/src/footprint.rs` — MWMR footprint tracking + port keys.
- `crates/warp-core/src/scheduler.rs` — deterministic ordering + reservation.
- `crates/warp-core/src/engine_impl.rs` — transaction lifecycle and commit pipeline.
- `crates/warp-core/src/snapshot.rs` — canonical state hashing + commit id encoding.
- `crates/warp-core/src/receipt.rs` — Paper II tick receipts.
- `crates/warp-core/src/tick_patch.rs` — Paper III delta patches + replay/slicing helpers.
- `crates/warp-core/src/warp_state.rs` — WarpInstances container.

---

## 11. Quickstart example (minimal)

For a working “known-good” bootstrap, use the demo helper:

```rust
use warp_core::{
    build_motion_demo_engine, encode_motion_atom_payload, make_node_id, make_type_id,
    ApplyResult, AttachmentValue, NodeRecord, MOTION_RULE_NAME,
};

let mut engine = build_motion_demo_engine();

// Insert an entity with a valid motion payload.
let entity_id = make_node_id("entity");
let entity_type = make_type_id("entity");
let payload = encode_motion_atom_payload([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
engine
    .insert_node_with_attachment(
        entity_id,
        NodeRecord { ty: entity_type },
        Some(AttachmentValue::Atom(payload)),
    )
    .unwrap();

let tx = engine.begin();
let res = engine.apply(tx, MOTION_RULE_NAME, &entity_id).unwrap();
assert!(matches!(res, ApplyResult::Applied));
let (snapshot, receipt, patch) = engine.commit_with_receipt(tx).unwrap();
assert_eq!(snapshot.patch_digest, patch.digest());
assert_eq!(snapshot.decision_digest, receipt.digest());
```

For provenance-grade outputs, use `commit_with_receipt` and store the `TickReceipt`
and `WarpTickPatchV1` alongside the snapshot hash.
