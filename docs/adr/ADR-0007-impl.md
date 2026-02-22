<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0007-impl: warp-core Implementation of BOAW Storage + Parallel Execution

- **Status:** Accepted
- **Date:** 2026-01-19
- **Parent:** ADR-0007 (BOAW Storage + Execution + Merge + Privacy)
- **Crate:** `crates/warp-core` (36 source modules)

---

## 0) Scope

ADR-0007 defines **what** the BOAW architecture is and **why** it exists:
immutable snapshots, COW overlays, lockless parallel execution, deterministic
commits, typed merge/collapse, and privacy-safe provenance.

This companion ADR documents **how** and **where** those decisions are
realized in `crates/warp-core`. It maps every ADR-0007 section to concrete
modules, data structures, and algorithms so that anyone implementing or
maintaining warp-core can find the canonical reference in one place.

Anything not yet implemented (collapse/merge with typed registry, privacy
enforcement, segment-level COW) is called out explicitly in §11.

---

## 1) Module Organization

The crate contains 36 source modules organized into five architectural layers.
Line counts are as of 2026-01-19.

### Core execution

| Module           |  LOC | Responsibility                                                   |
| ---------------- | ---: | ---------------------------------------------------------------- |
| `engine_impl.rs` | 2472 | Tick pipeline orchestration, rule registration, intent lifecycle |
| `scheduler.rs`   | 1427 | Radix-sort admission, `GenSet` conflict detection, `PendingTx`   |
| `tick_patch.rs`  | 1768 | `WarpOp` enum, `WarpTickPatchV1`, delta encoding/digest          |
| `tick_delta.rs`  |    — | Thread-local `TickDelta` (append-only op buffer)                 |

### Graph storage

| Module              |  LOC | Responsibility                                          |
| ------------------- | ---: | ------------------------------------------------------- |
| `graph.rs`          |  631 | `GraphStore` — BTreeMap-based in-memory store           |
| `graph_view.rs`     |    — | `GraphView` — compile-time read-only projection         |
| `snapshot.rs`       | 1216 | `Snapshot`, state-root hashing, commit-hash computation |
| `snapshot_accum.rs` |    — | Accumulator for building snapshots across instances     |
| `warp_state.rs`     |    — | Per-warp instance state tracking                        |

### BOAW parallel execution

| Module          | LOC | Responsibility                                                 |
| --------------- | --: | -------------------------------------------------------------- |
| `boaw/exec.rs`  | 526 | `ExecItem`, serial/parallel/sharded execution, `PoisonedDelta` |
| `boaw/merge.rs` | 244 | Canonical delta merge, `MergeError`, `MergeConflict`           |
| `boaw/shard.rs` | 265 | Virtual shard routing (`NUM_SHARDS = 256`), `shard_of()`       |

### WSC format (Write-Streaming Columnar)

| Module            | LOC | Responsibility                                                             |
| ----------------- | --: | -------------------------------------------------------------------------- |
| `wsc/types.rs`    | 300 | Fixed-size row types (`repr(C)`, `Pod`): `WscHeader`, `NodeRow`, `EdgeRow` |
| `wsc/build.rs`    | 580 | `GraphStore` → `OneWarpInput` bridge, canonical ordering                   |
| `wsc/view.rs`     | 503 | `WscFile`, `WarpView` — zero-copy read via `bytemuck`                      |
| `wsc/write.rs`    | 301 | Binary serialization of `OneWarpInput` → WSC bytes                         |
| `wsc/read.rs`     | 311 | Low-level read primitives, `ReadError` enum                                |
| `wsc/validate.rs` | 655 | Structural validation (ordering, alignment, bounds, tags)                  |

### Materialization

| Module                              | LOC | Responsibility                                             |
| ----------------------------------- | --: | ---------------------------------------------------------- |
| `materialization/bus.rs`            | 490 | `MaterializationBus` — order-independent output collection |
| `materialization/emit_key.rs`       | 164 | `EmitKey` — lexicographic `(scope_hash, rule_id, subkey)`  |
| `materialization/scoped_emitter.rs` | 207 | `ScopedEmitter` — key-forgery-proof emission handle        |

### Supporting modules

| Module                | LOC | Responsibility                                                          |
| --------------------- | --: | ----------------------------------------------------------------------- |
| `ident.rs`            | 197 | `Hash`, `NodeId`, `EdgeId`, `WarpId`, `TypeId`, domain-separated BLAKE3 |
| `record.rs`           |  50 | `NodeRecord`, `EdgeRecord` (skeleton-plane only)                        |
| `rule.rs`             | 108 | `RewriteRule`, `MatchFn`, `ExecuteFn`, `FootprintFn`, `ConflictPolicy`  |
| `footprint.rs`        | 495 | `Footprint` — warp-scoped read/write sets with `factor_mask`            |
| `footprint_guard.rs`  | 537 | `FootprintGuard`, `ViolationKind` — runtime enforcement                 |
| `attachment.rs`       | 457 | `AttachmentPlane`, `AttachmentOwner`, `AttachmentKey`                   |
| `receipt.rs`          | 241 | `TickReceipt` — causality poset witness per tick                        |
| `inbox.rs`            |   — | Canonical inbox management for deterministic intent sequencing          |
| `playback.rs`         |   — | Cursor-based playback and seek                                          |
| `retention.rs`        |   — | Snapshot retention policies                                             |
| `domain.rs`           |   — | Domain types for cross-crate API                                        |
| `cmd.rs`              |   — | Command types                                                           |
| `payload.rs`          |   — | Typed payload encoding                                                  |
| `tx.rs`               |   — | Transaction identity (`TxId`)                                           |
| `provenance_store.rs` |   — | Provenance tracking                                                     |
| `telemetry.rs`        |   — | Internal telemetry hooks                                                |

---

## 2) Key Data Structures

### 2.1 GraphStore (`src/graph.rs`)

BTreeMap-based in-memory graph with reverse edge indices. BTreeMap provides
deterministic iteration order at the cost of ~2x lookup vs HashMap (see §9.1).

```rust
pub struct GraphStore {
    pub(crate) warp_id: WarpId,
    pub(crate) nodes: BTreeMap<NodeId, NodeRecord>,
    pub(crate) edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>,
    pub(crate) edges_to: BTreeMap<NodeId, Vec<EdgeId>>,
    pub(crate) node_attachments: BTreeMap<NodeId, AttachmentValue>,
    pub(crate) edge_attachments: BTreeMap<EdgeId, AttachmentValue>,
    pub(crate) edge_index: BTreeMap<EdgeId, NodeId>,       // edge → source
    pub(crate) edge_to_index: BTreeMap<EdgeId, NodeId>,    // edge → target
}
```

The `canonical_state_hash()` method computes a BLAKE3 digest over a
deterministic byte stream (V2 format):

- Header: `b"DIND_STATE_HASH_V2\0"`
- Node count as `u64`, then nodes in ascending `NodeId` order
- Edge count as `u64`, then edges in ascending `EdgeId` order
- Attachments: ATOM → `(type_id, blob_len as u64, bytes)`, DESC → `(warp_id)`

### 2.2 Footprint (`src/footprint.rs`)

Warp-scoped read/write sets with a `u64` coarse prefilter. All resource
identifiers are `(WarpId, LocalId)` tuples, preventing false conflicts when
different warps touch resources with identical local IDs.

```rust
pub struct Footprint {
    pub n_read:  NodeSet,        // warp-scoped node reads
    pub n_write: NodeSet,        // warp-scoped node writes
    pub e_read:  EdgeSet,        // warp-scoped edge reads
    pub e_write: EdgeSet,        // warp-scoped edge writes
    pub a_read:  AttachmentSet,  // attachment reads
    pub a_write: AttachmentSet,  // attachment writes
    pub b_in:    PortSet,        // boundary input ports
    pub b_out:   PortSet,        // boundary output ports
    pub factor_mask: u64,        // coarse partition prefilter
}
```

Port keys are packed into 64 bits for efficient set operations:

```text
bits 63..32: lower 32 bits of node_id[0..8] as LE u64
bits 31..2:  port_id (u30)
bit 1:       reserved (0)
bit 0:       direction (1=input, 0=output)
```

### 2.3 ActiveFootprints + GenSet (`src/scheduler.rs`)

`GenSet` provides O(1) per-key conflict tracking without clearing hash tables
between transactions. A generation counter stamps each mark; `contains()` is
a simple `seen[key] == gen` check.

```rust
pub(crate) struct GenSet<K> {
    gen: u32,
    seen: FxHashMap<K, u32>,
}
```

`PendingTx` holds candidates awaiting admission:

```rust
struct PendingTx<P> {
    next_nonce: u32,
    index: FxHashMap<([u8; 32], u32), usize>,  // last-wins dedupe
    thin: Vec<RewriteThin>,     // 24B + 4B handle, radix-sortable
    fat: Vec<Option<P>>,        // payload storage
    scratch: Vec<RewriteThin>,  // radix sort double-buffer
    counts16: Vec<u32>,         // 65536-bucket histogram
}
```

### 2.4 WarpOp + WarpTickPatchV1 (`src/tick_patch.rs`)

Eight operation variants define all state mutations:

```rust
pub enum WarpOp {
    OpenPortal    { key, child_warp, child_root, init },
    UpsertWarpInstance { instance },
    DeleteWarpInstance { warp_id },
    UpsertNode    { node, record },
    DeleteNode    { node },
    UpsertEdge    { warp_id, record },
    DeleteEdge    { warp_id, from, edge_id },
    SetAttachment { key, value },
}
```

Each op carries a `WarpOpKey` for canonical merge ordering.

```rust
pub enum SlotId {
    Node(NodeKey),
    Edge(EdgeKey),
    Attachment(AttachmentKey),
    Port(WarpScopedPortKey),
}
```

### 2.5 MaterializationBus + EmitKey + ScopedEmitter (`src/materialization/`)

Order-independent output collection using BTreeMap for deterministic
finalization. Duplicate detection rejects identical `(channel, EmitKey)` pairs.
Thread-unsafe by design (single-threaded tick execution).

```rust
pub struct MaterializationBus {
    pending: RefCell<BTreeMap<ChannelId, BTreeMap<EmitKey, Vec<u8>>>>,
    policies: BTreeMap<ChannelId, ChannelPolicy>,
}

pub struct EmitKey {
    pub scope_hash: Hash,   // content hash of scope node
    pub rule_id: u32,       // compact rule ID
    pub subkey: u32,        // multi-emission differentiator
}
```

`ScopedEmitter` prevents key forgery: the engine fills `scope_hash` and
`rule_id`; rules cannot construct `EmitKey` directly.

### 2.6 WSC Format Types (`src/wsc/types.rs`)

Fixed-size columnar rows, all `#[repr(C)]` + `Pod` + `Zeroable` for
zero-copy access via `bytemuck::try_cast_slice()`.

```rust
#[repr(C)]
pub struct WscHeader {       // 128 bytes
    pub magic: [u8; 8],      // b"WSC\x00\x01\x00\x00\x00"
    pub schema_hash: Hash,   // 32
    pub tick_le: u64,
    pub warp_count_le: u64,
    pub warp_dir_off_le: u64,
    pub reserved: [u8; 64],
}

#[repr(C)]
pub struct NodeRow {         // 64 bytes
    pub node_id: Hash,       // 32
    pub node_type: Hash,     // 32
}

#[repr(C)]
pub struct EdgeRow {         // 128 bytes
    pub edge_id: Hash,       // 32
    pub from_node_id: Hash,  // 32
    pub to_node_id: Hash,    // 32
    pub edge_type: Hash,     // 32
}

#[repr(C)]
pub struct AttRow {          // 56 bytes
    pub tag: u8,             // 1=Atom, 2=Descend
    pub reserved0: [u8; 7],
    pub type_or_warp: Hash,  // TypeId or WarpId
    pub blob_off_le: u64,
    pub blob_len_le: u64,
}
```

### 2.7 Snapshot + TickReceipt (`src/snapshot.rs`, `src/receipt.rs`)

```rust
pub struct Snapshot {
    pub root: NodeKey,
    pub hash: Hash,
    pub state_root: Hash,
    pub parents: Vec<Hash>,
    pub plan_digest: Hash,
    pub decision_digest: Hash,
    pub rewrites_digest: Hash,
    pub patch_digest: Hash,
    pub policy_id: u32,
    pub tx: TxId,
}
```

`TickReceipt` records the causality poset witness for each tick:

```rust
pub struct TickReceipt {
    tx: TxId,
    entries: Vec<TickReceiptEntry>,  // canonical plan order
    blocked_by: Vec<Vec<u32>>,      // causality poset witness
    digest: Hash,                   // commits to entries only
}
```

---

## 3) Tick Execution Pipeline

The pipeline runs once per tick, transforming ingress into a committed
snapshot.

```text
  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
  │ 1.Ingest │───►│ 2.Match  │───►│ 3.Radix  │───►│ 4.Reserve│
  │ + Dispatch│    │ + Plan   │    │  Drain   │    │ (GenSet) │
  └──────────┘    └──────────┘    └──────────┘    └──────────┘
                                                       │
  ┌──────────┐    ┌──────────┐                         ▼
  │ 6.Commit │◄───│ 5.Execute│◄────────────────────────┘
  │ + Hash   │    │ Parallel │
  └──────────┘    └──────────┘
```

**Step 1 — Ingest + Dispatch.** Intents arrive via the inbox subsystem.
`ingest_inbox_event()` creates event nodes under the `sim/inbox` path and
attaches pending edges from opaque intent bytes. `dispatch_inbox_rule()`
drains pending intents and acks them.

**Step 2 — Rule Matching + Footprint.** For each candidate rule/scope pair,
the engine calls `MatchFn` to test applicability and `FootprintFn` to compute
the warp-scoped resource set. Each accepted match becomes a `PendingRewrite`
with fields `(scope_hash, rule_id, compact_rule, footprint, origin)`.

**Step 3 — Radix Drain.** `PendingTx::drain_in_order()` produces candidates
in a canonical byte-lexicographic order. The ordering key is
`(scope_hash[32B BE], rule_id[4B], nonce[4B])` — see §4 for algorithm details.

**Step 4 — Independence Check + Reservation.** Candidates are tested against
`ActiveFootprints` using `GenSet`-based O(1) conflict detection. Independent
candidates are atomically reserved (mark-all-or-nothing). Conflicting
candidates are deferred or rejected per `ConflictPolicy`.

**Step 5 — Parallel Execution.** Admitted rewrites are dispatched to workers.
Each worker receives a `GraphView` (read-only snapshot projection) and emits
ops into a thread-local `&mut TickDelta`. There is no shared mutable state —
determinism is enforced by the merge step, not execution order.

**Step 6 — Canonical Merge + Commit.** Thread-local deltas are merged
canonically (§6.3). The merged ops are applied to produce the next snapshot.
The engine computes `state_root`, `patch_digest`, and `commit_hash`
(§7), then stores the `Snapshot` and `TickReceipt`.

---

## 4) Radix Scheduler Algorithm

**Ref:** `src/scheduler.rs`

The scheduler uses LSD (least-significant-digit) radix sort to produce a
canonical ordering of pending rewrites. This is O(n) in the number of
candidates with zero comparisons.

### Sort Parameters

- **Key:** 40 bytes = `scope_hash[32B] || rule_id[4B] || nonce[4B]`
- **20 LSD passes** with 16-bit digits (65,536 buckets per pass)
- **Pass order:** nonce bytes [0,1], rule bytes [0,1], scope pairs [15..0]
- **Result:** byte-lexicographic order `(scope_be32, rule_id, nonce)`

### Small-Batch Fallback

When the candidate count is below `SMALL_SORT_THRESHOLD = 1024`, the scheduler
falls back to comparison sort, which is faster for small n due to lower
constant factors.

### Deduplication

`PendingTx` uses an `FxHashMap` keyed on `(scope_hash, compact_rule)` with
last-wins semantics. `FxHashMap` is safe here because it is internal to the
scheduler and never crosses a determinism boundary — iteration order over
the map is irrelevant since all entries are drained through the radix sort.

---

## 5) Conflict Detection

**Ref:** `src/footprint.rs`, `src/scheduler.rs`, `src/footprint_guard.rs`

### Independence Check

`Footprint::independent()` performs 5 resource checks with early exit:

1. **Factor mask (O(1)):** `(self.factor_mask & other.factor_mask) == 0` →
   independent, skip remaining checks
2. **Boundary ports:** any `b_in`/`b_out` intersection → conflict
3. **Edges:** `e_write ∩ e_write`, `e_write ∩ e_read`, or
   reverse → conflict
4. **Attachments:** `a_write ∩ a_write`, `a_write ∩ a_read`, or
   reverse → conflict
5. **Nodes:** `n_write ∩ n_write`, `n_write ∩ n_read`, or
   reverse → conflict

If all five checks pass, the rewrites are independent.

### GenSet Mechanism

`GenSet` provides O(1) per-key conflict detection without clearing hash
tables between transactions. Each `mark(key)` stamps the current generation;
`contains(key)` checks `seen[key] == gen`. Calling `advance()` increments the
generation counter, logically clearing all marks without touching the map.

### Atomic Reservation

Admission is all-or-nothing per candidate: all resource keys in the footprint
are marked, or none are (if any conflict is found, the reservation is aborted).

### FootprintGuard (Runtime Enforcement)

`FootprintGuard` validates that executors stay within their declared footprint.
Active in debug builds and when `footprint_enforce_release` feature is enabled
(disabled by `unsafe_graph`). Violations produce a `FootprintViolation` with
one of 9 `ViolationKind` variants:

```rust
pub enum ViolationKind {
    NodeReadNotDeclared(NodeId),
    EdgeReadNotDeclared(EdgeId),
    AttachmentReadNotDeclared(AttachmentKey),
    NodeWriteNotDeclared(NodeId),
    EdgeWriteNotDeclared(EdgeId),
    AttachmentWriteNotDeclared(AttachmentKey),
    CrossWarpEmission { op_warp: WarpId },
    UnauthorizedInstanceOp,
    OpWarpUnknown,
}
```

---

## 6) BOAW Parallel Execution (Phase 6B)

**Ref:** `src/boaw/shard.rs`, `src/boaw/exec.rs`, `src/boaw/merge.rs`

### 6.1 Virtual Shard Partitioning

```rust
pub const NUM_SHARDS: usize = 256;
const SHARD_MASK: u64 = 255;

pub fn shard_of(scope: &NodeId) -> usize {
    let first_8 = &scope.0[0..8];
    let val = u64::from_le_bytes(first_8.try_into().unwrap());
    (val & SHARD_MASK) as usize
}
```

`NUM_SHARDS = 256` is a **frozen protocol constant** — changing it requires a
version bump. The formula takes the first 8 bytes of the NodeId's 32-byte hash,
interprets them as a little-endian u64, and masks to 255. This provides:

- **Load balance:** 256 virtual shards vs 8–64 typical workers
- **Cache locality:** scope-adjacent rewrites land in the same shard
- **Platform independence:** LE interpretation is explicit

### 6.2 Worker Execution

`execute_parallel_sharded()` dispatches `ExecItem` work units across workers.
Workers atomically claim shards and execute all items within a claimed shard.

```rust
pub struct ExecItem {
    pub exec: ExecuteFn,
    pub scope: NodeId,
    pub origin: OpOrigin,
}

pub enum WorkerResult {
    Success(TickDelta),
    Poisoned(PoisonedDelta),
    MissingStore(WarpId),
}
```

If a rule panics during execution, `catch_unwind` captures the panic and wraps
it as `PoisonedDelta`. The panic is surfaced at merge time as a typed error,
not an uncontrolled crash.

When `footprint_enforce_release` is enabled, `GraphView::new_guarded(store, guard)`
wraps reads, and `check_op()` validates each emitted op post-execution.

### 6.3 Canonical Merge

The merge algorithm guarantees deterministic output regardless of worker count
or execution order:

1. **Flatten:** Collect all ops from all worker deltas with their `OpOrigin`
2. **Sort:** Order by `(WarpOpKey, OpOrigin)` for canonical ordering
3. **Validate new warps:** Collect `OpenPortal` ops with `PortalInit::Empty`;
   reject same-tick writes to newly created warps
4. **Dedupe:** Identical ops from different origins are collapsed to one
5. **Conflict detection:** Divergent ops on the same `WarpOpKey` →
   `MergeConflict`

```rust
pub enum MergeError {
    Conflict(Box<MergeConflict>),
    PoisonedDelta(PoisonedDelta),
    WriteToNewWarp { warp_id: WarpId, op_origin: OpOrigin, op_kind: &'static str },
}
```

**Policy:** Merge conflicts are bugs — they mean the footprint model lied.
There is no recovery path; the tick is aborted.

---

## 7) State Root Hashing

**Ref:** `src/snapshot.rs`, `src/graph.rs`, `src/tick_patch.rs`

All hashing uses BLAKE3.

### State Root (`canonical_state_hash()`)

Deterministic BFS to collect the reachable graph across warp instances:

- Header: `b"DIND_STATE_HASH_V2\0"`
- Per-instance: `warp_id`, `root_node_id`, parent attachment key
- Nodes in ascending `NodeId` order, each contributing `(node_id, type_id)`
- Edges per source node, sorted by `EdgeId`, filtered to reachable targets
- Attachments: ATOM → `(type_id, blob_len as u64, bytes)`, DESC → `(warp_id)`

Node and edge counts are encoded as `u64` for version stability.

### Commit Hash

```rust
pub fn compute_commit_hash_v2(
    state_root: &Hash,
    parents: &[Hash],
    patch_digest: &Hash,
    policy_id: u32,
) -> Hash
```

The commit hash is:
`H(version_tag || parents_len || parents || state_root || patch_digest || policy_id)`

This commits to the full causal chain: the prior state (parents), the
resulting state (state_root), and the operations that produced it
(patch_digest), under a specific policy.

### Patch Digest

Computed from the versioned encoding of merged ops + slots + metadata in the
`WarpTickPatchV1`. This is separate from `state_root` so that two commits
can be compared either by "did they reach the same state?" or "did they apply
the same operations?"

---

## 8) WSC Build Process

**Ref:** `src/wsc/build.rs`, `src/wsc/write.rs`, `src/wsc/view.rs`

### Canonical Ordering

The build process in `build_one_warp_input()` enforces strict ordering:

1. **Nodes** — iterated from `GraphStore`'s `BTreeMap`, already sorted by
   `NodeId`
2. **Edges (global)** — all edges collected, then sorted by `EdgeId` via
   `.sort_by_key(|e| e.id)`
3. **Per-node outbound edges** — for each node (in `NodeId` order), outgoing
   edges are sorted by `EdgeId` and appended with an index `Range { start, len }`
4. **Attachments** — node attachments follow node order; edge attachments
   follow global edge order

### Blob Arena Alignment

All blob payloads are 8-byte aligned before appending: `(len + 7) & !7`.
This prepares the blob section for mmap and SIMD consumers. All `Range`,
`AttRow`, and index fields are stored little-endian.

### Zero-Copy Read

`WscFile` owns the raw `Vec<u8>`. `WarpView<'a>` holds lifetime-bound
slices directly into that buffer — no heap allocations at access time:

```rust
pub struct WarpView<'a> {
    pub nodes: &'a [NodeRow],
    pub edges: &'a [EdgeRow],
    pub out_index: &'a [Range],
    pub out_edges: &'a [OutEdgeRef],
    pub node_atts_index: &'a [Range],
    pub node_atts: &'a [AttRow],
    pub edge_atts_index: &'a [Range],
    pub edge_atts: &'a [AttRow],
    pub blobs: &'a [u8],
}
```

All slice casts use `bytemuck::try_cast_slice()`, which is a safe transmutation
requiring `T: Pod`. Node/edge lookup is O(log n) via `binary_search_by_key`.

### Structural Validation (`src/wsc/validate.rs`)

`validate_wsc()` runs 6 checks per warp instance:

1. Index range bounds (all three index tables within data table bounds)
2. Node ordering (strict ascending by `NodeId`)
3. Edge ordering (strict ascending by `EdgeId`)
4. Root node present (binary search; zero root OK when nodes is empty)
5. Attachment validity (tag, reserved bytes, blob bounds, DESCEND invariants)
6. Out-edge reference validity (edge index within bounds, safe u64→usize cast)

---

## 9) Design Decisions

Nine implementation-level decisions with rationale.

### 9.1 BTreeMap Everywhere

`GraphStore` uses `BTreeMap` for all collections. This gives deterministic
iteration order at ~2x lookup cost vs `HashMap`. The trade-off is accepted
because iteration order affects `canonical_state_hash()`, WSC build ordering,
and snapshot construction. Using `HashMap` would require a separate sort pass
at every materialization point.

### 9.2 RefCell in MaterializationBus

`MaterializationBus` uses `RefCell<BTreeMap<...>>` for pending emissions.
This is intentionally single-threaded: tick execution on a single warp is
sequential by design, and `RefCell` makes the borrow rules visible at compile
time without the overhead of `Mutex`.

### 9.3 Factor Mask as u64 Coarse Prefilter

`Footprint::factor_mask` is a 64-bit bloom-like field. One bitwise AND
(`mask_a & mask_b == 0`) can short-circuit the entire independence check
before examining per-resource sets. On workloads with many independent
rewrites touching disjoint partitions, this eliminates >90% of set
intersection work.

### 9.4 FxHashMap in PendingTx

`PendingTx` uses Rustc's `FxHashMap` (non-deterministic iteration order) for
deduplication. This is safe because the map is internal to the scheduler and
all entries are drained through the radix sort before any
determinism-sensitive operation. The map's iteration order is never observed.

### 9.5 20-Pass LSD Radix Sort

A 40-byte key requires 20 passes with 16-bit digits (65,536 buckets). This
is O(n) with zero comparisons, ideal for the scheduling workload where n can
be large and keys are uniformly distributed (BLAKE3 hashes). The fallback to
comparison sort below 1,024 items avoids the per-pass overhead for small
batches.

### 9.6 Pod + bytemuck for WSC

All WSC row types are `#[repr(C)]` + `Pod` + `Zeroable`. This allows
zero-copy deserialization via `bytemuck::try_cast_slice()` — a safe
transmutation that requires no `unsafe` in user code. The constraint is
that all fields must be plain-old-data (no pointers, no padding with
uninitialized bytes).

### 9.7 Separate GraphView Type

`GraphView` is a compile-time read-only projection of `GraphStore`. Executors
receive `GraphView<'a>` instead of `&GraphStore`, preventing accidental
writes. When `FootprintGuard` is active, `GraphView::new_guarded()` also
validates reads against the declared footprint.

### 9.8 PoisonedDelta on Panic

When a rule panics during parallel execution, `catch_unwind` captures the
panic payload and wraps it as `PoisonedDelta`. This ensures the panic is
surfaced as a typed `MergeError::PoisonedDelta` at merge time rather than
crashing the entire tick. The delta's ops are never applied.

### 9.9 NUM_SHARDS = 256 Frozen Protocol Constant

The shard count is a protocol constant, not a tuning knob. Changing it would
alter `shard_of()` routing and break determinism for any graph built under the
old constant. 256 provides a good balance: enough shards for load distribution
across 8–64 workers, small enough that the shard metadata overhead is trivial.

---

## 10) Test Strategy

### 10.1 Integration Test Categories

warp-core has 50+ integration tests in `crates/warp-core/tests/`. Key
categories:

| Category           | Representative Files                                               | What They Validate                                                          |
| ------------------ | ------------------------------------------------------------------ | --------------------------------------------------------------------------- |
| Determinism        | `boaw_determinism.rs`, `boaw_end_to_end.rs`                        | Snapshot hash invariance under permutations, serial-vs-parallel equivalence |
| Worker invariance  | `boaw_engine_worker_invariance.rs`                                 | Identical results for 1/2/4/8/16/32 workers                                 |
| Parallel execution | `boaw_parallel_exec.rs`                                            | Sharded partitioning, canonical merge ordering                              |
| Footprints         | `boaw_footprints.rs`, `boaw_footprint_warp_scoping.rs`             | Independence symmetry, warp-scoped conflict isolation                       |
| Merge safety       | `boaw_merge_tripwire.rs`, `boaw_merge_warpopkey.rs`                | Footprint violations caught at merge, cross-warp key isolation              |
| Portal rules       | `boaw_openportal_rules.rs`                                         | No same-tick writes to newly created warps                                  |
| Materialization    | `materialization_determinism.rs`, `materialization_spec_police.rs` | Order-independence, permutation invariance, wire stability                  |
| Snapshot           | `snapshot_reachability_tests.rs`                                   | Unreachable nodes excluded from hash                                        |
| Slice theorem      | `slice_theorem_proof.rs`                                           | Seven-phase executable proof of parallel correctness                        |
| Math determinism   | `deterministic_sin_cos_tests.rs`, `nan_exhaustive_tests.rs`        | ULP-budget trig, NaN canonicalization to `0x7fc00000`                       |
| Playback           | `outputs_playback_tests.rs`, `playback_cursor_tests.rs`            | Cursor seek, corrupt-hash detection                                         |

### 10.2 Testing Patterns

- **Exhaustive permutation:** `materialization_spec_police.rs` uses Heap's
  algorithm to test all N! orderings (N ≤ 6) and assert byte-identical output
- **Pinned seeds:** Proptest with deterministic seeds for reproducible CI
  (`proptest_seed_pinning.rs`)
- **1-ULP float sensitivity:** `determinism_audit.rs` checks whether f32
  bit-flips affect canonical hashes
- **Serial-vs-parallel equivalence:** `boaw_determinism.rs` runs the same
  workload with 1 worker and N workers, asserts identical `state_root`,
  `patch_digest`, and `commit_hash`
- **Worker-count invariance:** `boaw_engine_worker_invariance.rs` iterates
  over `{1, 2, 4, 8, 16, 32}` workers on independent workloads
- **Byte-identical assertion:** All hash comparisons use exact `[u8; 32]`
  equality, not approximate matching

### 10.3 Future Tests (Marked `#[ignore]`)

Several test files contain `#[ignore]` tests for features not yet
implemented:

- `boaw_cow.rs` — COW overlay semantics (pending overlay implementation)
- `boaw_merge.rs` — Multi-parent commutative merge (pending collapse/merge)
- `boaw_privacy.rs` — Mind-mode enforcement (pending privacy implementation)

---

## 11) ADR-0007 Alignment

Mapping from parent ADR sections to concrete implementation modules.

| ADR-0007 Section                   | Implementation                                                          | Status                                                  |
| ---------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------- |
| §3 Atom & Attachment policy        | `attachment.rs` — `AttachmentPlane`, `AttachmentOwner`, `AttachmentKey` | Partial — typed storage exists, no registry enforcement |
| §4 WARP snapshots (reachable-only) | `snapshot.rs` — reachable BFS in state-root computation                 | Done                                                    |
| §4.2 WSC zero-copy IO              | `wsc/` — full read/write/validate/view pipeline                         | Done                                                    |
| §5 COW mechanics                   | `tick_delta.rs`, `graph_view.rs` — delta-based writes, read-only views  | Partial — no segment-level structural sharing           |
| §6 Footprints & enforcement        | `footprint.rs`, `footprint_guard.rs` — warp-scoped sets + guard         | Done                                                    |
| §7 Scheduling (virtual shards)     | `boaw/shard.rs`, `scheduler.rs` — `NUM_SHARDS=256`, radix sort          | Done                                                    |
| §7.1 Shard routing (frozen)        | `boaw/shard.rs::shard_of()`                                             | Done — frozen protocol constant                         |
| §8 Tick pipeline                   | `engine_impl.rs`, `scheduler.rs`, `boaw/exec.rs`, `boaw/merge.rs`       | Done                                                    |
| §8.4 Parallel execute              | `boaw/exec.rs` — `execute_parallel_sharded()`                           | Done (Phase 6B)                                         |
| §8.5 Merge deltas                  | `boaw/merge.rs` — canonical sort + conflict detection                   | Done                                                    |
| §8.6 Commit                        | `snapshot.rs` — `compute_commit_hash_v2()`, `compute_state_root()`      | Done                                                    |
| §9 Collapse/merge                  | —                                                                       | Not started — `boaw_merge.rs` tests are `#[ignore]`     |
| §10 Privacy (mind/diagnostics)     | —                                                                       | Not started — `boaw_privacy.rs` tests are `#[ignore]`   |

### Not Yet Implemented

- **Collapse/merge with typed registry:** ADR-0007 §9 defines multi-parent
  merge with typed merge rules (commutative, LWW, ConflictOnly). No merge
  implementation exists beyond single-tick delta merge.
- **Privacy enforcement:** ADR-0007 §10 defines mind mode (no secrets in
  ledger) and diagnostics mode. The `ClaimRecord` type and enforcement gates
  are not yet implemented.
- **Segment-level COW:** ADR-0007 §5.5 defines segment-level structural
  sharing for WSC snapshots. Current implementation rebuilds full snapshots
  each tick.

---

## 12) Consequences

### Benefits

- **Lockless parallel execution** — workers emit to thread-local deltas;
  no shared mutable state during execution
- **Deterministic across platforms** — canonical ordering (radix sort),
  fixed shard topology (256), explicit LE encoding, BLAKE3 hashing
- **Zero-copy snapshots** — WSC format with `bytemuck` provides read access
  without deserialization
- **Typed conflict handling** — `PoisonedDelta`, `MergeConflict`, and
  `FootprintViolation` surface errors as structured data, not panics
- **Comprehensive test coverage** — 50+ integration tests including
  exhaustive permutation, worker-count invariance, and executable proofs

### Costs

- **BTreeMap overhead** — ~2x lookup cost vs HashMap, accepted for
  deterministic iteration
- **Radix sort memory** — 65,536-bucket histogram + scratch buffer per sort
  pass, amortized across ticks
- **Full snapshot rebuild** — each tick rebuilds the entire snapshot (no
  segment-level sharing yet)
- **Feature-gated enforcement** — `FootprintGuard` is debug-only by default;
  release enforcement requires opt-in feature flag

### Open Items

- Implement collapse/merge with typed registry (ADR-0007 §9)
- Implement privacy enforcement / mind mode (ADR-0007 §10)
- Add segment-level COW for snapshot structural sharing (ADR-0007 §5.5)
- Migrate `GraphStore` from BTreeMap to immutable snapshot + overlay model
- Add benchmark suites (snapshot hashing, motion throughput, scheduler drain)
