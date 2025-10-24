# Branch Tree Persistence Specification (Phase 0)

Echo’s temporal sandbox relies on a persistent simulation tree to support branching, rewinding, and merging. This document defines the data model, hashing, diff encoding, and algorithms that guarantee determinism while enabling rich tooling.

---

## Goals
- Represent the multiverse as a persistent structure with structural sharing and content-addressed storage.
- Support O(1) branching and efficient diff capture without scanning entire worlds.
- Provide three-way, per-component merges with deterministic conflict resolution hooks (including CRDT strategies).
- Track entropy/paradox metrics through read/write sets and deterministic math scopes.
- Support deterministic GC and inspector tooling.

---

## Core Structures

### Dirty Chunk Index (ECS → Timeline)
On every tick the ECS emits a `DirtyChunkIndex` containing only modified chunks. This is the sole source of diff data—no full archetype scans.

```ts
interface DirtyChunkEntry {
  chunkId: string;
  archetypeId: number;
  versionBefore: number;
  versionAfter: number;
  dirtyByComponent: Map<ComponentTypeId, RoaringBitmap>;
  readSet: Map<ComponentTypeId, readonly ReadKey[]>;
  writeSet: Map<ComponentTypeId, readonly WriteKey[]>;
}

type DirtyChunkIndex = Map<string, DirtyChunkEntry>;
```

- `versionBefore` / `versionAfter` are epoch counters incremented by the ECS whenever the chunk mutates.
- `RoaringBitmap` tracks dirty slots for the component within the chunk.
- `ReadKey` / `WriteKey` are canonical keys (e.g., `{ slot: number, field?: string }`) used for paradox detection.

### Diff Record
Three-way, chunk-local diffs keyed by `(archetypeId, chunkId, componentType)`.

```ts
interface ChunkDiff {
  archetypeId: number;
  chunkId: string;
  componentType: number;
  versionBefore: number;
  versionAfter: number;
  dirty: RoaringBitmap;
  readSet: ReadKey[];
  writeSet: WriteKey[];
  mergeStrategy: MergeStrategyId; // recorded decision for replay
  payloadRef: Hash;               // content-addressed component data
}

interface DiffRecord {
  readonly id: Hash;
  readonly parentSnapshotId: Hash;
  readonly chunkDiffs: readonly ChunkDiff[];
  readonly decisionsDigest: Hash; // hash of per-component merge decisions
  readonly entropyDelta: number;
  readonly metadata: DiffMetadata;
}
```

- `mergeStrategy` defaults to `lastWriteWins`, but components can specify `sum`, `max`, `min`, `setUnion`, `domainResolver`, etc. CRDT-friendly components can provide custom merge functions.
- `payloadRef` points to serialized component data stored in the block store.
- `readSet`/`writeSet` enable paradox detection.

### Snapshot Record
Rolling base snapshots with delta chains capped at depth `K`.

```ts
interface SnapshotRecord {
  readonly id: Hash;
  readonly parentId: Hash | null;
  readonly schemaVersion: number;
  readonly endianness: "le";
  readonly chunkRefs: readonly ChunkRef[]; // content-addressed chunk payloads
  readonly cumulativeDiffSize: number;
  readonly depth: number; // distance from last full base
}
```

Policy:
- Take a full snapshot every N ticks or when cumulative diff bytes > X% of the base snapshot.
- Limit delta chains to length `K` (e.g., 5). On commit, if chain length exceeds `K`, materialize a new base snapshot.

### Timeline Node
```ts
interface TimelineNode {
  readonly id: Hash;              // content-addressed
  readonly parentId: Hash | null;
  readonly branchId: KairosBranchId;
  readonly chronos: ChronosTick;
  readonly aionWeight: number;
  readonly snapshotId: Hash;
  readonly diffId: Hash | null;
  readonly entropyDelta: number;
  readonly mergeParents?: [Hash, Hash]; // present for merge nodes
  readonly metadata: TimelineMetadata;
}
```

`id = BLAKE3( parentId || branchId || chronos || diffId || mergeDecisionDigest )` using canonical byte encoding (sorted keys, little-endian numeric fields).

### Branch Record
```ts
type BranchStatus = "active" | "collapsed" | "abandoned";

interface BranchRecord {
  readonly id: KairosBranchId;
  readonly rootNodeId: Hash;
  headNodeId: Hash;
  entropy: number;
  status: BranchStatus;
  ancestry: readonly Hash[]; // cached path from root to head
}
```

- `collapsed`: branch intentionally merged into another branch/root.
- `abandoned`: orphaned draft/proposal; subject to auto-expiry policies.

---

## Persistence: Block Store
All persistent artifacts live in a content-addressed block store, enabling pluggable backends (memory, IndexedDB, SQLite).

```ts
interface BlockStore {
  put(kind: "node" | "snapshot" | "diff" | "payload", bytes: Uint8Array): Hash;
  get(hash: Hash): Promise<Uint8Array | null>;
  pin(hash: Hash): void;   // inspector / user pins
  unpin(hash: Hash): void;
}
```

Pins must be recorded in the timeline so replays reflect identical liveness.

---

## Algorithms

### Fork Branch (O(1))
1. Retrieve head node `H` of branch `α`.
2. Create new branch record `β`: `rootNodeId = head(α)`, `headNodeId = head(α)`.
3. Increment snapshot/diff reference counts (epoch-aware API).

### Commit Branch (O(touched slots + metadata))
1. ECS provides `DirtyChunkIndex`.
2. For each dirty chunk:
   - Validate `versionBefore` matches snapshot version.
   - Serialize component payloads using canonical encoding.
   - Build `ChunkDiff` with roaring bitmap, read/write sets, merge strategy (default `lastWriteWins`).
3. Compute cumulative diff size; decide whether to create new base snapshot.
4. Write diff and optional snapshot to block store.
5. Create new TimelineNode with hashed ID; update branch head.
6. Update branch entropy using formula:
   `entropyDelta = wF*forks + wC*conflicts + wP*paradoxes + wM*crossMsgs − wX*collapses` (clamped [0,1]).

### Merge Branches (α ← β)
1. Find lowest common ancestor `L` via binary lifting (store `up[k]` tables and depths on nodes).
2. Walk diff chains from `head(α)` and `head(β)` back to `L`, collecting chunk diffs.
3. For each `(chunk, component)` in lexicographic order:
   - Combine roaring bitmaps to identify slots touched by either branch.
   - Perform three-way merge using snapshot at `L` as base.
   - If both branches changed the same slot and results differ (`!equals(A', B')`), register conflict.
   - Apply merge strategy (policy, CRDT, manual). Record decision digest.
4. Build merged diff & optional snapshot, commit new node as head of α.
5. Mark branch β `collapsed` or `abandoned` depending on workflow.

### Paradox Detection
- For each diff, track `readSet`/`writeSet`.
- On merge or commit, paradox exists if `writesB` intersects with any `readsA` where operation A precedes B in Chronos.
- Paradoxes increment entropy and may block merge depending on policy.

### Random Determinism
- Each diff that samples randomness records `{ seedStart, count }`. Branch forks derive new seeds via `seed' = BLAKE3(seed || branchId || chronos)`.
- Replay consumes exactly `count` draws to maintain determinism.

### Garbage Collection (Deterministic)
- GC runs only at fixed intervals (e.g., every 256 ticks) and processes nodes in sorted `Hash` order.
- When GC disabled (deterministic mode), reference counts accumulate but release occurs at predetermined checkpoints.
- Inspector pins are recorded in timeline to keep GC behavior replayable.

---

## Data Structure Enhancements
- `TimelineNode.mergeParents` captures the two node IDs merged, aiding inspector and proofs.
- `DiffRecord.decisionsDigest` stores hash of merge decisions for deterministic replay.
- `SnapshotRecord` includes `schemaVersion` & `endianness` for portability.
- `DirtyChunkIndex` is the authoritative source for chunk mutations (no fallbacks).

---

## Block Hashing & Canonical Encoding
- All persisted data encoded little-endian, with sorted keys for maps.
- Use canonical NaN encoding to avoid float hash drift.
- No timestamps feed into IDs; timestamps remain metadata only.

---

## Inspector Roadmap (Future)
- Conflict heatmaps by archetype/component across Chronos.
- Causality lens: click a component to reveal diffs that read it before mutation.
- Entropy graph: visualize branch stability; warn when nearing paradox thresholds.
- Scrub & splice: preview merges over a selected node range before committing.

---

## Minimal API (MVP)
```ts
type NodeId = string;
type BranchId = string;

type MergeResult = {
  node: NodeId;
  conflicts: readonly MergeConflict[];
};

interface EchoTimeline {
  head(branch: BranchId): NodeId;
  fork(from: NodeId, newBranch?: BranchId): BranchId;
  commit(branch: BranchId, worldView: WorldView, dirtyIndex: DirtyChunkIndex): NodeId;
  merge(into: BranchId, from: BranchId): MergeResult;
  collapse(branch: BranchId): void;
  materialize(node: NodeId): SnapshotRecord;
  gc(policy: GCPolicy): void;
}
```

Ship MVP with roaring bitmaps, chunk epochs, rolling snapshots, deterministic hashing, three-way merges (default LWW), paradox detection, and entropy accumulation.

---

## Test Plan
1. **Replay Identity:** Fork → no writes → commit → world equals parent snapshot byte-for-byte.
2. **Order Independence:** Two systems write disjoint slots; merged diff identical regardless of execution order.
3. **Three-Way Merge:** Synthetic 1M-slot scenario with 1% overlap; conflicts deterministic, merge sub-second.
4. **GC Determinism:** Same action sequence with GC on/off → materialized world identical.
5. **Paradox Scanner:** Inject read/write overlaps → paradox count stable across replays.
6. **Hash Stability:** Different JS runtimes, same seeds → identical node IDs across N ticks.
7. **Entropy Regression:** Validate entropy formula per branch with known events.

---

## Open Questions
- Which roaring bitmap implementation offers best balance of size/perf in JS? (Possibly WebAssembly bridge.)
- Should we expose plugin hooks for domain-specific merge strategies? (e.g., geometry vs inventory.)
- Best policy for auto-expiring abandoned branches? (Time-based vs depth-based.)
- Inspector pin semantics: how to surface pinned nodes to users without threatening determinism.
- CRDT component library: identify candidate components (counters, sets) for conflict-free merges.


---

## Phase 0.5 Addendum — Causality & Determinism Layer

This addendum extends the branch tree specification with causal tracking, schema safety, replay guarantees, and the public API boundary.

### Causality Graph
Each node may store a causal DAG linking events to their effects.

```ts
interface CausalEdge {
  readonly causeId: string;
  readonly effectId: string;
  readonly relation: "reads" | "writes" | "spawns" | "resolves";
}

interface CausalityGraph {
  readonly nodeId: string;
  readonly edges: readonly CausalEdge[];
}
```

The diff generator populates the graph from `readSet` / `writeSet`. Causal graphs are persisted as deterministic blocks to enable “why” queries and paradox prevention.

### Component Schema Ledger
Keep a ledger of component layouts to ensure cross-branch compatibility.

```ts
interface ComponentSchemaRecord {
  readonly typeId: number;
  readonly layoutHash: string;
  readonly version: number;
}

interface SchemaLedgerSnapshot {
  readonly id: string;
  readonly schemas: readonly ComponentSchemaRecord[];
}
```

Snapshots reference their ledger ID. Layout hashes (BLAKE3 over canonical schema JSON) must match before merges occur.

### Inspector Data Protocol
Expose structured telemetry per tick for UI or headless consumers.

```ts
interface InspectorFrame {
  readonly tick: ChronosTick;
  readonly branches: KairosBranchId[];
  readonly entropy: number;
  readonly metrics: Record<string, number>;
  readonly diffsApplied: number;
  readonly conflicts: number;
  readonly paradoxes: number;
  readonly worldHash: string;
}
```

Inspector frames are serialized alongside timeline events; inspector UIs subscribe to the feed without direct runtime mutation.

### Diff Compaction & Compression
- Deduplicate identical `ChunkDiff` entries via content hashes.
- Pack small diffs into 64 KB pages to reduce block overhead.
- Compress pages with Zstandard (level 3 default).
- Compaction runs in deterministic order: sort `(chunkId, componentType, versionBefore)`.

### Deterministic Replay Contract
Expose a replay command (conceptually `echo replay --from nodeId --until nodeId --verify`) with guarantees:
1. Identical diff sequences yield identical `worldHash`.
2. Event order and PRNG consumption counts are identical.
3. GC, compression, or inspector hooks do not affect semantics.

Verification mode re-hashes snapshots/diffs and flags divergence.

### Entropy Observers
Provide hooks for gameplay systems to respond to stability changes.

```ts
interface EntropyObserver {
  onEntropyChange(node: TimelineNode, delta: number, total: number): void;
}
```

Observers subscribe to branch-level entropy updates to trigger narrative or mechanical responses.

### Security Envelope & Capability Tokens
Wrap persistent blocks with a security envelope.

```ts
interface SecurityEnvelope {
  readonly hash: string;
  readonly signature?: string;
  readonly signerId?: string;
}
```

Diffs, snapshots, and merges carry envelopes. Capability tokens assign which adapters can mutate which component domains; violations raise deterministic errors (e.g., `ERR_CAPABILITY_DENIED`).

### Determinism Invariants
1. **World Equivalence:** identical diff sequences ⇒ identical `worldHash`.
2. **Merge Determinism:** identical inputs + merge decisions ⇒ identical output.
3. **Temporal Stability:** GC, compression, inspector activity do not affect logical state.
4. **Schema Consistency:** mismatched layout hashes block merges.
5. **Causal Integrity:** writes do not modify values they transitively read earlier in Chronos.
6. **Entropy Reproducibility:** entropy delta derives solely from recorded events.

Violations terminate the tick and record deterministic error nodes.

### Error Model & Recovery
| Failure | Detection | Recovery | Status |
| ------- | --------- | -------- | ------ |
| Diff apply fails | checksum mismatch | discard node, mark branch `corrupted` | deterministic |
| Snapshot corrupted | hash mismatch | rebuild from last base snapshot | deterministic |
| Capability violation | runtime guard | abort tick, log error | deterministic |
| Merge unresolved | conflict count | require manual merge node | deterministic |
| Paradox | read/write overlap | isolate branch, emit paradox node | deterministic |

Recovery operations emit synthetic nodes so replay matches origin.

### Public API Boundary
Expose a stable façade while internals remain replaceable.

```ts
interface EchoWorldAPI {
  createEntity(archetype: ArchetypeDef): EntityId;
  destroyEntity(id: EntityId): void;
  query<Q extends QuerySpec>(q: Q): QueryResult<Q>;
  emit<E extends Event>(event: E): void;
  fork(from?: NodeId): BranchId;
  merge(into: BranchId, from: BranchId): MergeResult;
  replay(options: ReplayOpts): VerificationReport;
  inspect(tick?: ChronosTick): InspectorFrame;
}
```

All mutating operations route through Codex’s Baby; determinism invariants enforced at this boundary. Internal systems (storage, scheduler, adapters) remain swappable under the same contract.

---
