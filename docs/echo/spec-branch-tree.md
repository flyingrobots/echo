# Branch Tree Persistence Specification (Phase 0)

Echo’s temporal sandbox relies on a persistent simulation tree to support branching, rewinding, and merging. This document details the data model and algorithms for maintaining that tree, capturing diffs, and reconciling timelines.

---

## Goals
- Represent the multiverse as a persistent structure with structural sharing to minimize memory usage.
- Support fast branching (O(1) to fork) and efficient re-simulation via diff caches.
- Provide merge tooling with deterministic conflict detection.
- Track entropy/paradox metrics to inform gameplay and diagnostics.

---

## Core Structures

### TimelineNode
```ts
interface TimelineNode {
  readonly id: string;                // stable UUID (deterministic hash of parent+chronos?)
  readonly parentId: string | null;
  readonly branchId: KairosBranchId;  // branch this node belongs to
  readonly chronos: ChronosTick;
  readonly aionWeight: number;
  readonly snapshotId: string;        // reference to persisted world snapshot
  readonly diffId: string | null;     // diff from parent snapshot
  readonly entropyDelta: number;      // change in entropy at this node
  readonly metadata: TimelineMetadata;
}

interface TimelineMetadata {
  readonly createdAt: number;         // monotonic tick counter
  readonly createdBy: string;         // system/systemID or tool
  readonly tags?: readonly string[];  // designer annotations
  readonly summary?: string;          // short description for inspector
}
```
- `snapshotId` points to persisted chunk state (see below).
- `diffId` optional: absent for root nodes or when snapshot is full state.

### Snapshot Store
- `SnapshotId -> SnapshotRecord`
```ts
interface SnapshotRecord {
  readonly id: string;
  readonly archetypeHashes: string[];      // for quick validation
  readonly chunkRefs: readonly ChunkRef[]; // references to ECS chunk buffers
  readonly sizeBytes: number;
  readonly createdFrom: string | null;     // parent snapshot
  readonly branchRefCount: number;         // number of branches sharing this snapshot
}
```
- Snapshot store deduplicates by content hash. When a branch forks without modifications, it references same snapshot with increased ref count.

### Diff Record
```ts
interface DiffRecord {
  readonly id: string;
  readonly parentSnapshotId: string;
  readonly modifiedChunks: readonly ChunkDiff[];
  readonly createdEntities: readonly EntityDiff[];
  readonly destroyedEntities: readonly EntityDiff[];
  readonly metadata: {
    readonly mutationCount: number;
    readonly impactedComponents: readonly number[];
    readonly isStructural: boolean;
  };
}
```
- `ChunkDiff` contains chunk ID, version, bitmask of modified slots, and optional column deltas.
- `EntityDiff` describes entity handles + serialized components (for create/destroy).
- `isStructural` indicates diff introduced structural change (e.g., new archetype) vs data-only.

### Branch Registry
```ts
interface BranchRecord {
  readonly id: KairosBranchId;
  readonly rootNodeId: string;
  readonly headNodeId: string;
  readonly entropy: number;
  readonly status: "active" | "collapsed" | "merged";
  readonly ancestry: readonly string[]; // node IDs from root to head
}
```
- `ancestry` may be stored as linked list to avoid duplication; cached array for quick inspector use.

---

## Operations

### Fork Branch
1. Identify current head node `H` for branch `α`.
2. Create new branch record `β`:
   - `rootNodeId = H.id` if branch diverges from current head.
   - `headNodeId = H.id`.
   - Entropy inherits from parent plus fork penalty.
3. Increment snapshot ref counts for `H.snapshotId` (and diff chain if needed).
4. Register branch in branch registry.

### Commit Tick
On each tick for branch `α`:
1. ECS diff generator compares world state against last committed snapshot/diff for branch head.
2. Build `DiffRecord`:
   - For each chunk touched (based on chunk `version` vs stored version), compute bitmask of dirty slots.
   - Serialize changed components into `ChunkDiff`.
   - Note created/destroyed entities.
3. Persist diff (store in diff store keyed by hashed content).
4. Optionally persist snapshot:
   - Policy: full snapshot every N ticks or when diff size exceeds threshold.
   - Snapshot references chunk buffers (copy-on-write ensures branch-specific data).
5. Create new `TimelineNode` with parent = `headNodeId`, update branch `headNodeId`.
6. Update entropy: `entropy += entropyDelta(diff)` (e.g., based on number of merges, cross-branch messages).

### Merge Branches
To merge branch `β` into `α`:
1. Find lowest common ancestor node `L`.
2. Collect diffs from `L -> head(α)` and `L -> head(β)`.
3. Replay diffs in deterministic order:
   - If both modify same component slot with different values, flag conflict.
   - Conflict resolution policy (initial): manual selection or prioritizing chosen branch.
4. Apply merged diff to create new snapshot/diff for `α`.
5. Mark branch `β` status `collapsed` or `merged`, adjust entropy.
6. Decrement ref counts for snapshots exclusive to `β`; free chunks where ref count hits zero.
7. Log merge metadata for inspector (list of nodes involved, conflicts resolved).

### Collapse Branch
When branch ends without merge:
1. Mark status `collapsed`.
2. Release snapshot/diff references (decrement counts).
3. Optionally keep branch nodes for history; GC old nodes based on retention policy.

### Garbage Collection
- Snapshots/diffs use ref counting. When `branchRefCount` reaches zero, mark for deletion.
- Periodic GC pass prunes nodes older than retention window if no references.
- Provide manual “pinning” to keep nodes around for analysis.

---

## Diff Encoding
- Chunk diff bitmasks stored as:
  - `uint32[]` bitset for slot mutations.
  - Optional run-length encoding if contiguous sections common.
- Component deltas encoded per data type:
  - POD types: copy bytes.
  - Managed types: hash + pointer to serialized payload stored separately.
- Entity create/destroy use component descriptors to reconstruct state deterministically.

---

## Timeline Navigation
- `TimelineIndex` structure for quick lookups:
```ts
interface TimelineIndex {
  readonly nodesByChronos: Map<ChronosTick, readonly string[]>;
  readonly nodesByBranch: Map<KairosBranchId, readonly string[]>;
  readonly nodesByAion: BalancedTree<number, string>; // sorted by significance
}
```
- Updated on each commit/merge.
- Inspector uses index to render timeline graph.

---

## Entropy & Paradox Tracking
- Each node stores `entropyDelta` (e.g., +1 for fork, +2 for paradox resolved).
- Global entropy meter = sum(branch.entropy).
- Paradox detection hooks integrate with Codex’s Baby; flagged nodes link to paradox resolution records.

---

## Persistence Backend
- Pluggable storage backend (initial: in-memory + optional JSON snapshots).
- Interface:
```ts
interface TimelinePersistence {
  saveNode(node: TimelineNode): void;
  saveSnapshot(snapshot: SnapshotRecord): void;
  saveDiff(diff: DiffRecord): void;
  loadBranch(branchId: KairosBranchId): BranchRecord | null;
  // etc.
}
```
- Design decouples in-memory runtime from eventual database-backed storage (SQLite, IndexedDB, etc.).

---

## Determinism Considerations
- Node IDs generated deterministically: e.g., `hash(parentId + chronos + branchId + diffId)`.
- Snapshot and diff hashing stable across runs; avoid including non-deterministic data (timestamps).
- Merge resolution must be deterministic given same choices; record decision path for replay.
- GC should run in deterministic order (sorted by node ID) or be disabled during deterministic runs.

---

## Open Questions
- How to expose partial timeline reload (e.g., load only last N nodes) without violating determinism?
- Should branch merges allow weighted blending (mix of two diffs) or only discrete selection?
- How to efficiently diff large numbers of chunks without scanning full structure (use dirty chunk set from ECS)?
- What retention policy suits multiplayer vs single-player use cases?

Future work: integrate with inspector, define serialization formats for save/load, and add conflict resolution strategies.
