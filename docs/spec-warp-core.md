<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# `warp-core` — WARP Core Runtime & API Tour
>
> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).

This document is a **tour of the `warp-core` crate**: the core data model,
deterministic boundary artifacts, and the runtime APIs that higher layers (`warp-ffi`,
`warp-wasm`, tools, and eventually the full Echo runtime) build on.

If you only remember one thing:

> `warp-core` is where “WARP is real” — deterministic state, deterministic ticks,
> deterministic hashes, and replayable deltas.

Related docs (recommended, in order):

1. `docs/guide/warp-primer.md` — newcomer-friendly WARP overview (start here).
2. `docs/warp-two-plane-law.md` — project law for SkeletonGraph vs attachment plane.
3. `docs/spec-merkle-commit.md` — state_root vs commit_id and what is committed.
4. `docs/spec-warp-tick-patch.md` — tick patch boundary artifact (Paper III).
5. `docs/spec/SPEC-0002-descended-attachments-v1.md` — WarpInstances / descended attachments (Stage B1).

---

## 1. Mental model (what the crate implements)

`warp-core` implements a **two-plane** WARP state object:

- **Skeleton plane (structure):** nodes + edges + boundary ports (matching/scheduling operates here).
- **Attachment plane (payloads):** typed atoms (depth-0) and explicit descent indirection (Stage B1).

In Paper I notation, this is “WarpState `U`” and its projection `π(U)`:

```text
U := (SkeletonGraph, Attachments)  // full state (both planes)
π(U) = SkeletonGraph                // skeleton projection (hot path)
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
- **Utilities:** payload helpers (`encode_motion_atom_payload`, etc.).

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
- `matcher: fn(&GraphView, &NodeId) -> bool`
- `executor: fn(&GraphView, &NodeId, &mut TickDelta)` (BOAW Phase 5: read-only execution)
- `compute_footprint: fn(&GraphView, &NodeId) -> Footprint`
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

### 8.4 Determinism invariants (summary)

Echo treats these as non-negotiable invariants. Violations must abort deterministically.

1. **World equivalence:** identical diffs + merge decisions ⇒ identical world hash.
2. **Merge determinism:** same base snapshot + diffs + strategies ⇒ identical snapshot + diff hashes.
3. **Temporal stability:** GC/compression/inspector activity must not alter logical state.
4. **Schema consistency:** component layout hashes must match before merges.
5. **Causal integrity:** writes cannot modify values they transitively read earlier in Chronos.
6. **Entropy reproducibility:** branch entropy is a deterministic function of recorded events (see `/spec-entropy-and-paradox` for event log format + location).
7. **Replay integrity:** replay from A→B reproduces world hash, event order, and PRNG draw counts.

---

## 9. Stage B1 recursion: WarpInstances + portals

`warp-core` supports “WARPs all the way down” without recursive traversal:

- descended attachments are explicit `AttachmentValue::Descend(WarpId)`
- the child instance is a separate namespace (`WarpInstance` + `GraphStore`)
- portals are authored atomically in patches via `WarpOp::OpenPortal`

Crucial correctness law:

- any match/exec inside a descended instance must READ the attachment keys in its descent chain
  (so changing a portal pointer deterministically invalidates matches)

### 9.1 Worked example: descent-chain reads become `Footprint.a_read`

The engine enforces the law in `Engine::apply_in_warp` by *injecting* the descent
chain into the footprint before the candidate is enqueued:

```rust
let mut footprint = (rule.compute_footprint)(store, scope);
// Stage B1 law: any match/exec inside a descended instance must READ
// every attachment slot in the descent chain.
for key in descent_stack {
    footprint.a_read.insert(*key);
}
```

This is intentionally independent of whether the rule decodes attachments:

- it is a **reachability / meaning** dependency, not “data was parsed”
- it makes portal changes invalidate matches deterministically, even when the rule
  is otherwise “pure skeleton”

Downstream effects:

- `Footprint.a_read` contributes to the tick patch `in_slots` (as `SlotId::Attachment(key)`),
  because `commit_with_receipt` derives conservative `in_slots/out_slots` from footprints.
- This keeps Paper III slicing correct: uses within a descendant instance depend on the
  portal chain slots that establish the instance.

### 9.2 Worked example: Paper III slicing includes the portal chain

`warp-core` ships a unit test that models the intended history shape:

- tick 0: `OpenPortal` produces `SlotId::Attachment(portal_key)`
- tick 1: an op inside the child instance produces a node slot and **reads** `portal_key`

Worldline slicing for the child-produced node correctly returns both ticks (include the portal chain):

```rust
let worldline = [patch0_open_portal, patch1_child_write];
let ticks = slice_worldline_indices(&worldline, SlotId::Node(child_node_key));
assert_eq!(ticks, vec![0, 1]);
```

See: `crates/warp-core/src/tick_patch.rs` (`slice_includes_portal_chain_for_descended_instance`).

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

## 11. Quickstart examples

### 11.1 Minimal (single instance)

For a working "known-good" bootstrap, use the demo helper from `echo-dry-tests`:

```rust
use warp_core::{
    encode_motion_atom_payload, make_node_id, make_type_id,
    ApplyResult, AttachmentValue, NodeRecord,
};
use echo_dry_tests::{build_motion_demo_engine, MOTION_RULE_NAME};

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

### 11.2 Stage B1: portals + `apply_in_warp` + slicing

The minimal “B1-shaped” workflow is:

1) establish a portal (`OpenPortal`) from a node-owned attachment slot (Alpha plane) to a child `WarpId`  
2) apply a rewrite inside the child warp using `Engine::apply_in_warp` with a `descent_stack` containing that portal key  
3) verify the tick patch `in_slots` includes the portal slot, and slicing pulls in the portal-opening tick

```rust
use warp_core::{
    slice_worldline_indices, ApplyResult, AttachmentKey, ConflictPolicy, Engine, Footprint,
    GraphStore, GraphView, NodeId, NodeKey, NodeRecord, PortalInit, SchedulerKind, SlotId,
    TickCommitStatus, TickDelta, WarpOp, WarpState, WarpTickPatchV1, WarpInstance,
    POLICY_ID_NO_POLICY_V0, RewriteRule, make_node_id, make_type_id, make_warp_id,
};

fn b1_rule_match(_view: GraphView<'_>, _scope: &NodeId) -> bool {
    true
}

// BOAW Phase 5: Executors receive read-only GraphView and emit ops to TickDelta.
// No GraphStore mutations during execution.
fn b1_rule_exec(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let child_node = make_node_id("child-node");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id: view.warp_id(),
            local_id: child_node,
        },
        record: NodeRecord {
            ty: make_type_id("ChildTy"),
        },
    });
}

fn b1_rule_footprint(_view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    // Conservative: record the write so patch out_slots is slice-safe.
    fp.n_write.insert_node(&make_node_id("child-node"));
    fp
}

fn insert_child_node_rule() -> RewriteRule {
    RewriteRule {
        id: [9u8; 32],
        name: "demo/b1-insert-child-node",
        left: warp_core::PatternGraph { nodes: vec![] },
        matcher: b1_rule_match,
        executor: b1_rule_exec,
        compute_footprint: b1_rule_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

let root_warp = make_warp_id("root");
let child_warp = make_warp_id("child");

let root_node = make_node_id("root-node");
let child_root = make_node_id("child-root");
let child_node = make_node_id("child-node");

let root_key = NodeKey {
    warp_id: root_warp,
    local_id: root_node,
};
let child_root_key = NodeKey {
    warp_id: child_warp,
    local_id: child_root,
};
let child_node_key = NodeKey {
    warp_id: child_warp,
    local_id: child_node,
};

// The portal lives in the node-owned (Alpha) attachment plane.
let portal_key = AttachmentKey::node_alpha(root_key);

// Step 0: build an initial multi-instance state via patch replay (no engine internals required).
let mut state = WarpState::new();

let init_root = WarpTickPatchV1::new(
    POLICY_ID_NO_POLICY_V0,
    [0u8; 32], // demo rule_pack_id
    TickCommitStatus::Committed,
    vec![],
    vec![SlotId::Node(root_key)],
    vec![
        WarpOp::UpsertWarpInstance {
            instance: WarpInstance {
                warp_id: root_warp,
                root_node: root_node,
                parent: None,
            },
        },
        WarpOp::UpsertNode {
            node: root_key,
            record: NodeRecord {
                ty: make_type_id("RootTy"),
            },
        },
    ],
);
init_root.apply_to_state(&mut state).unwrap();

let open_portal = WarpTickPatchV1::new(
    POLICY_ID_NO_POLICY_V0,
    [0u8; 32],
    TickCommitStatus::Committed,
    vec![],
    vec![SlotId::Attachment(portal_key), SlotId::Node(child_root_key)],
    vec![WarpOp::OpenPortal {
        key: portal_key,
        child_warp,
        child_root,
        init: PortalInit::Empty {
            root_record: NodeRecord {
                ty: make_type_id("ChildRootTy"),
            },
        },
    }],
);
open_portal.apply_to_state(&mut state).unwrap();

// Step 1: initialize an engine from that multi-instance state.
let mut engine = Engine::with_state(state, root_key, SchedulerKind::Radix, POLICY_ID_NO_POLICY_V0)
    .unwrap();
engine.register_rule(insert_child_node_rule()).unwrap();

// Step 2: apply inside the child warp.
let tx = engine.begin();
let res = engine
    .apply_in_warp(tx, child_warp, "demo/b1-insert-child-node", &child_root, &[portal_key])
    .unwrap();
assert!(matches!(res, ApplyResult::Applied));
let (_snapshot, _receipt, patch1) = engine.commit_with_receipt(tx).unwrap();

// The descent stack is enforced as an attachment read.
assert!(patch1.in_slots().contains(&SlotId::Attachment(portal_key)));
assert!(patch1.out_slots().contains(&SlotId::Node(child_node_key)));

// Step 3: worldline slicing pulls in the portal chain.
let worldline = vec![open_portal, patch1];
let ticks = slice_worldline_indices(&worldline, SlotId::Node(child_node_key));
assert_eq!(ticks, vec![0, 1]);
```

Notes:

- `Engine::apply_in_warp(..., descent_stack)` is the *only* place the engine needs to “know about recursion”
  for correctness: the hot path still matches within an instance skeleton only.
- If you don’t record descent-chain reads, you can build a system that “looks right” but produces incorrect slices.
