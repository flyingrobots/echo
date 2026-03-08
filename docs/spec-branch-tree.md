<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Branch Tree Persistence Specification (Phase 0)

> **Background:** For a gentler introduction, see [WARP Primer](guide/warp-primer.md).

Echo's temporal sandbox relies on a persistent simulation tree to support branching, rewinding, and merging. This document defines the data model, hashing, diff encoding, and algorithms that guarantee determinism while enabling rich tooling.

> **Terminology — strain vs. entropy.** The Aion papers define _entropy_ in the Boltzmann sense: an observer-relative measure of observational indistinguishability (`S(τ) = log |Π⁻¹(τ)|`). This spec uses **branch strain** — a configurable gameplay heuristic that tracks stability via weighted event counts. Same conceptual family, different quantities. Where this document says "strain," it means the heuristic; where it says "entropy," it means the Aion/Boltzmann quantity.

---

## Goals

- Represent the multiverse as a persistent structure with structural sharing and content-addressed storage.
- Support O(1) branching and efficient diff capture without scanning entire worlds.
- Provide three-way, per-component merges with deterministic conflict resolution hooks (including CRDT strategies).
- Track branch strain through read/write sets and deterministic math scopes.
- Support deterministic GC and inspector tooling.

---

## Core Structures

### Foundational Types

These types are used throughout this spec.

#### ECS-Layer Access Keys

The Aion papers define footprints at the rewrite level (`Del`/`Use` on graph matches); the network confluence paper generalizes to patch-level `R(π)`/`W(π)`/`D(π)`/`A(π)`. This spec's keys are an **ECS-layer specialization** — component addresses inside chunks — not a replacement for either higher-level abstraction.

```ts
/** Dot-separated field path within a component (e.g., "position.x"). */
type CanonicalFieldPath = string;

/**
 * ECS-local access key identifying a slot (and optionally a field) within
 * a single component column of a chunk. ReadKey and WriteKey share the
 * same shape; the distinction is contextual (which set they appear in).
 *
 * componentType is NOT included here because these keys already live
 * inside a Map<ComponentTypeId, ...> — repeating it would be redundant.
 */
interface AccessKey {
    readonly slot: u32;
    readonly fieldPath?: CanonicalFieldPath;
}

type ReadKey = AccessKey;
type WriteKey = AccessKey;

/**
 * Lifted canonical address for contexts that leave the per-component map
 * scope: paradox detection across chunks, merge conflict reporting, and
 * inspector queries.
 */
interface QualifiedKey {
    readonly chunkId: string;
    readonly componentType: ComponentTypeId;
    readonly slot: u32;
    readonly fieldPath?: CanonicalFieldPath;
}
```

> **Caveat:** If slots are reused or entities migrate archetypes, pure slot-based keys become ambiguous. In that case `QualifiedKey` will eventually need a stable `entityId`. Do not introduce that field until the migration path demands it.

#### Dirty Slot Sets

```ts
/**
 * Abstract set of dirty slot indices within a component column.
 * Required semantics: membership test, iteration in deterministic
 * (ascending) order, set union/intersection, and canonical encoding
 * when crossing a persistence boundary.
 *
 * Phase 0 default implementation: roaring-family bitmap. The specific
 * JS/WASM library is implementation-defined.
 */
type DirtySlotSet = /* roaring-family bitmap */;
```

#### Merge Strategy IDs

```ts
/**
 * Namespaced string identifying a merge strategy.
 *
 * Built-in strategies use the `core:` namespace:
 *   - core:lww          (last-write-wins — the default)
 *   - core:sum
 *   - core:max
 *   - core:min
 *   - core:set-union
 *
 * Domain-specific strategies use an application namespace:
 *   - game:inventory-merge@1
 *   - physics:contact-union@2
 *
 * Non-core strategies MUST record a resolver manifest digest in the
 * DiffRecord so that replays can verify the same resolver was used.
 * The runtime plugin loading/registration ABI is out of scope for
 * Phase 0; what matters is that the strategy ID and decision outputs
 * are recorded deterministically.
 */
type MergeStrategyId = string;
```

### Dirty Chunk Index (ECS → Timeline)

On every tick the ECS emits a `DirtyChunkIndex` containing only modified chunks. This is the sole source of diff data — no full archetype scans.

```ts
interface DirtyChunkEntry {
    chunkId: string;
    archetypeId: number;
    versionBefore: number;
    versionAfter: number;
    dirtyByComponent: Map<ComponentTypeId, DirtySlotSet>;
    readSet: Map<ComponentTypeId, readonly ReadKey[]>;
    writeSet: Map<ComponentTypeId, readonly WriteKey[]>;
}

type DirtyChunkIndex = Map<string, DirtyChunkEntry>;
```

- `versionBefore` / `versionAfter` are epoch counters incremented by the ECS whenever the chunk mutates.
- `DirtySlotSet` tracks dirty slots for the component within the chunk. Iteration order MUST be deterministic (ascending slot index).
- `ReadKey` / `WriteKey` are ECS-local access keys (see above) used for paradox detection.

### Diff Record

Three-way, chunk-local diffs keyed by `(archetypeId, chunkId, componentType)`.

```ts
interface ChunkDiff {
    archetypeId: number;
    chunkId: string;
    componentType: number;
    versionBefore: number;
    versionAfter: number;
    dirty: DirtySlotSet;
    readSet: ReadKey[];
    writeSet: WriteKey[];
    mergeStrategy: MergeStrategyId; // recorded decision for replay
    payloadRef: Hash; // content-addressed component data
}

/**
 * The hashable core of a DiffRecord. These fields define diff identity.
 * id = BLAKE3(canonicalEncode(DiffRecordCore)).
 */
interface DiffRecordCore {
    readonly parentSnapshotId: Hash;
    readonly chunkDiffs: readonly ChunkDiff[];
    readonly decisionsDigest: Hash; // hash of per-component merge decisions
}

interface DiffRecord extends DiffRecordCore {
    readonly id: Hash; // = BLAKE3(canonicalEncode(DiffRecordCore))
    readonly strainDelta: number;
    readonly metadata: DiffMetadata;
}
```

- `mergeStrategy` defaults to `core:lww`. Components can specify any registered strategy. CRDT-friendly components can provide custom merge functions registered under an application namespace.
- `payloadRef` points to serialized component data stored in the block store.
- `readSet`/`writeSet` enable paradox detection.
- `strainDelta` is derived metadata (not part of the hashable core).

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
/**
 * The hashable core of a TimelineNode. Only these fields define node
 * identity. Derived metrics (strain, aionWeight) live in metadata so
 * that tuning them does not change node hashes.
 */
interface TimelineNodeCore {
    readonly parents: Hash[]; // [] = genesis, [p] = linear, [p1, p2] = merge
    readonly branchId: KairosBranchId;
    readonly chronos: ChronosTick;
    readonly snapshotId: Hash;
    readonly diffId: Hash | null;
}

interface TimelineNode extends TimelineNodeCore {
    readonly id: Hash; // = BLAKE3(canonicalEncode(TimelineNodeCore))
    readonly metadata: TimelineMetadata;
}

/** Non-hashed sidecar for derived/tunable metrics. */
interface TimelineMetadata {
    readonly aionWeight: number;
    readonly strainDelta: number;
    // ... additional metadata as needed
}
```

`id = BLAKE3(canonicalEncode(TimelineNodeCore))` using canonical byte encoding (sorted keys, little-endian numeric fields). The `parents` array is encoded in order; its length implicitly distinguishes genesis (0), linear (1), and merge (2) nodes.

> **Design note:** `aionWeight` and `strainDelta` are intentionally excluded from the hashable core. If they were inside the core, every formula tweak would change node identity and break content-addressed storage. Keep derived/tunable metrics in metadata.

### Branch Record

```ts
type BranchStatus = "active" | "collapsed" | "abandoned";

interface BranchRecord {
    readonly id: KairosBranchId;
    readonly rootNodeId: Hash;
    headNodeId: Hash;
    strain: number; // cached strain total of current head node
    status: BranchStatus;
    ancestry: readonly Hash[]; // cached path from root to head
}
```

- `collapsed`: branch intentionally merged into another branch/root.
- `abandoned`: logical workflow status marking an orphaned draft/proposal. This is a status flag only — it does **not** imply automatic deletion. Retention and expiry are a separate governance/policy layer (see [Open Questions](#open-questions)). In Phase 0, abandoned branches are retained indefinitely unless an explicit, deterministic expiry policy is configured and recorded as a control input.

---

## Persistence: Block Store

All persistent artifacts live in a content-addressed block store, enabling pluggable backends (memory, IndexedDB, SQLite).

```ts
interface BlockStore {
    put(
        kind: "node" | "snapshot" | "diff" | "payload",
        bytes: Uint8Array,
    ): Hash;
    get(hash: Hash): Promise<Uint8Array | null>;
    pin(hash: Hash): void; // inspector / user pins
    unpin(hash: Hash): void;
}
```

Pins must be recorded in the timeline so replays reflect identical liveness.

---

## Algorithms

### Fork Branch (O(1))

1. Retrieve head node `H` of branch `α`.
2. Create new branch record `β`: `rootNodeId = head(α)`, `headNodeId = head(α)`.
3. New branch inherits the strain total of node `H`.
4. Increment snapshot/diff reference counts (epoch-aware API).

### Commit Branch (O(touched slots + metadata))

1. ECS provides `DirtyChunkIndex`.
2. For each dirty chunk:
    - Validate `versionBefore` matches snapshot version. If `versionBefore` does not match the snapshot version, the commit aborts and emits a `ParadoxTimelineNode` referencing the conflicting chunk IDs. This ensures deterministic behavior on replay.
    - Serialize component payloads using canonical encoding.
    - Build `ChunkDiff` with dirty slot set, read/write sets, merge strategy (default `core:lww`).
3. Compute cumulative diff size; decide whether to create new base snapshot.
4. Write diff and optional snapshot to block store.
5. Create new `TimelineNode` with hashed ID; update branch head.
6. Update branch strain using the formula below.

#### Branch Strain Formula

```text
strainDelta = wF·forks + wC·conflicts + wP·paradoxes + wM·imports − wX·collapses
```

The running total is `strain = max(0, parentStrain + strainDelta)` — floored at zero, unbounded above. Normalize only for display or gameplay threshold checks.

- **`imports`**: cross-branch messages/imports (not cross-warp).
- **Weights** are configurable per-world and stored in the deterministic world config. They MUST be encoded as fixed-point integers (not floats) to avoid platform-dependent rounding.
- **Defaults** (for worlds that do not specify custom weights):

| Weight | Value | Rationale                    |
| ------ | ----- | ---------------------------- |
| `wF`   | 5     | Forks: small structural cost |
| `wC`   | 25    | Conflicts: serious           |
| `wP`   | 50    | Paradoxes: heavy             |
| `wM`   | 15    | Imports: moderate            |
| `wX`   | 20    | Collapses: resolving         |

A UI saturation threshold of 100 provides a natural scale where forks alone are gentle and paradoxes quickly push a branch toward instability.

### Merge Branches (α ← β)

1. Find lowest common ancestor `L` via binary lifting (store `up[k]` tables and depths on nodes).
2. Walk diff chains from `head(α)` and `head(β)` back to `L`, collecting chunk diffs.
3. For each `(chunk, component)` in lexicographic order:
    - Combine dirty slot sets to identify slots touched by either branch.
    - Perform three-way merge using snapshot at `L` as base.
    - If both branches changed the same slot and results differ (`!equals(A', B')`), register conflict. Equality (`equals`) is defined as bitwise comparison of canonical serialized payloads — no epsilon or component-specific comparison is used, preserving determinism.
    - Apply merge strategy (policy, CRDT, manual). Record decision digest.
4. Build merged diff & optional snapshot, commit new node as head of α. The merge node's `parents` array contains `[head(α), head(β)]`.
5. Strain continues from the **target branch's** total plus the merge delta. Do not sum both branch totals; that would double-count shared history.
6. Mark branch β `collapsed` or `abandoned` depending on workflow.

### Paradox Detection

- For each diff, track `readSet`/`writeSet`.
- On merge or commit, paradox exists if `writesB` intersects with any `readsA` where operation A precedes B in Chronos.
- Paradoxes increment strain and may block merge depending on policy.

### Random Determinism

Each diff that samples randomness records `{ seedStart, count }`. Branch forks derive new seeds via domain-separated canonical encoding:

```text
seed' = BLAKE3(canonicalEncode({
    domain:   "echo.branch-seed.v1",
    seed:     [32 raw bytes],
    branchId: [length-prefixed UTF-8, no NUL terminator],
    chronos:  [u64, little-endian]
}))
```

If `KairosBranchId` is a string, it MUST be valid UTF-8. Ideally constrain branch IDs to printable ASCII to avoid Unicode normalization ambiguities.

Replay consumes exactly `count` draws to maintain determinism.

### Garbage Collection (Deterministic)

Phase 0 supports three GC modes:

| Mode         | Behavior                                                                                                                     |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| `periodic`   | Runs at fixed tick intervals (e.g., every 256 ticks). Processes nodes in sorted `Hash` order. Deterministic by construction. |
| `checkpoint` | Runs only at predetermined checkpoints. Reference counts accumulate between checkpoints; release is batched.                 |
| `none`       | No GC. All blocks retained indefinitely. Use for archival, replay verification, or debugging.                                |

There is no adaptive/memory-pressure mode. Memory-pressure-driven GC would be nondeterminism in disguise.

**Pin semantics:** Pinning a node preserves the **full reachable closure** needed to materialize or verify that node:

- Ancestor nodes (transitively via `parents`)
- Referenced snapshots and diffs
- Payload blocks referenced by diffs
- Schema ledger blocks
- Capability assertion records needed for verification

If a pin does not preserve ancestors transitively, it is not a pin — it is a dangling reference. Inspector pins are recorded in the timeline so GC behavior is replayable.

---

## Data Structure Enhancements

- `TimelineNode.parents` captures all parent node IDs (linear and merge), aiding inspector and proofs.
- `DiffRecord.decisionsDigest` stores hash of merge decisions for deterministic replay.
- `SnapshotRecord` includes `schemaVersion` & `endianness` for portability.
- `DirtyChunkIndex` is the authoritative source for chunk mutations (no fallbacks).

---

## Block Hashing & Canonical Encoding

- All persisted data encoded little-endian, with sorted keys for maps.
- Use canonical NaN encoding to avoid float hash drift.
- No timestamps feed into IDs; timestamps remain metadata only.
- `TimelineNode.id` and `DiffRecord.id` are computed over their respective `Core` structs only. Derived metrics (`strainDelta`, `aionWeight`) are excluded.

---

## Inspector Roadmap (Future)

- Conflict heatmaps by archetype/component across Chronos.
- Causality lens: click a component to reveal diffs that read it before mutation.
- Strain graph: visualize branch stability; warn when nearing instability thresholds.
- Scrub & splice: preview merges over a selected node range before committing.

---

## Minimal API (MVP)

### WorldView

A lightweight read-only handle providing canonical access to chunk state during the commit barrier. The `DirtyChunkIndex` tells `commit` _what_ changed; `WorldView` provides the _bytes and versions_ for those locations.

```ts
interface WorldView {
    readonly chronos: ChronosTick;
    readonly schemaLedgerId: Hash;
    getChunkVersion(chunkId: string): number;
    readComponentCanonical(
        chunkId: string,
        componentType: ComponentTypeId,
        slots?: readonly u32[],
    ): Uint8Array;
}
```

### GCPolicy

```ts
type GCMode = "periodic" | "checkpoint" | "none";

interface GCPolicy {
    readonly mode: GCMode;
    readonly intervalTicks?: number; // periodic only
    readonly retainDepth?: number; // max ancestor chain length to keep
    readonly retainBaseSnapshots?: boolean; // keep full base snapshots even if unreferenced
    readonly respectPins: true; // always true; listed for explicitness
}
```

Avoid knobs like `targetBytes` or `freeWhenMemoryHigh`. Those are nondeterministic unless fully recorded as control inputs.

### EchoTimeline

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
    commit(
        branch: BranchId,
        worldView: WorldView,
        dirtyIndex: DirtyChunkIndex,
    ): NodeId;
    merge(into: BranchId, from: BranchId): MergeResult;
    collapse(branch: BranchId): void;
    materialize(node: NodeId): SnapshotRecord;
    gc(policy: GCPolicy): void;
}
```

Ship MVP with dirty slot sets, chunk epochs, rolling snapshots, deterministic hashing, three-way merges (default `core:lww`), paradox detection, and strain accumulation.

---

## Test Plan

1. **Replay Identity:** Fork → no writes → commit → world equals parent snapshot byte-for-byte.
2. **Order Independence:** Two systems write disjoint slots; merged diff identical regardless of execution order.
3. **Three-Way Merge:** Synthetic 1M-slot scenario with 1% overlap; conflicts deterministic, merge sub-second.
4. **GC Determinism:** Same action sequence with GC on/off → materialized world identical.
5. **Paradox Scanner:** Inject read/write overlaps → paradox count stable across replays.
6. **Hash Stability:** Different JS runtimes, same seeds → identical node IDs across N ticks.
7. **Strain Regression:** Validate strain formula per branch with known events and configurable weights.

---

## Open Questions

- **Dirty slot set implementation.** The spec requires deterministic set semantics, ascending iteration order, and canonical encoding at persistence boundaries. Phase 0 defaults to a roaring-family bitmap. The specific JS package or WASM bridge is implementation-defined and does not affect protocol semantics.

- **Plugin hook API for merge strategies.** The replay-relevant artifacts — `MergeStrategyId` per chunk diff, resolver manifest digest for non-core strategies, and `decisionsDigest` in the `DiffRecord` — are standardized. The runtime plugin loading and registration ABI is out of scope for Phase 0; it does not affect deterministic replay as long as recorded strategy IDs and decision outputs are stable.

- **Auto-expiry policy for abandoned branches.** `abandoned` is a logical workflow status, not an implicit deletion trigger. Retention and expiry are a separate governance layer. In Phase 0, abandoned branches are retained indefinitely unless an explicit, deterministic expiry policy is configured and recorded as a control input. Wall-clock-based expiry is forbidden in deterministic replay contexts.

- **Inspector pin semantics:** how to surface pinned nodes to users without threatening determinism.

- **CRDT component library:** identify candidate components (counters, sets) for conflict-free merges.

---

## Phase 0.5 Addendum — Causality & Determinism Layer

This addendum extends the branch tree specification with causal tracking, schema safety, replay guarantees, and the public API boundary.

### Causality Graph

Each node may store a **local** causal DAG linking events to their effects within a single tick/commit. This is the Paper II tick-event poset specialized to the ECS commit layer.

Cross-tick causality is captured by the timeline's `parents`/`chronos` ancestry. Cross-branch causality is captured by merge `parents`. Network-level frontier-relative causality (as defined in the network confluence paper) is out of scope for this spec and belongs to the replication/import layer.

A global causal DAG spanning ticks, branches, and replicas can be **derived** for inspector tooling, but is not the persisted primitive.

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

**Edge relation semantics:**

| Relation   | Meaning                                                                            |
| ---------- | ---------------------------------------------------------------------------------- |
| `reads`    | Effect read a location/version written by cause.                                   |
| `writes`   | Effect overwrote or depends on a prior write at the same address.                  |
| `spawns`   | Cause created an identity, event, or branch later used by effect.                  |
| `resolves` | Effect resolves a prior conflict, paradox, or merge condition introduced by cause. |

The diff generator populates the graph from `readSet` / `writeSet`. Causal graphs are persisted as deterministic blocks to enable "why" queries and paradox prevention.

> **Important:** Chronos is a per-branch tick counter, not a global cross-branch clock. Within one branch lineage, tick N+1 happens after tick N. Across divergent branches, numeric Chronos comparison alone is not sufficient to establish causal ordering.

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
    readonly strain: number;
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

### Stability Observers

Provide hooks for gameplay systems to respond to branch strain changes.

```ts
interface StabilityObserver {
    onStrainChange(node: TimelineNode, delta: number, total: number): void;
}
```

Observers subscribe to branch-level strain updates to trigger narrative or mechanical responses (e.g., visual instability effects, forced branch resolution above a threshold).

**Branch strain lifecycle:**

- Genesis branch starts at strain `0`.
- Forked branch inherits the strain total of the head node it forks from.
- Merge continues from the **target branch's** total plus the merge delta. Do not sum both branch totals.
- `BranchRecord.strain` caches the total of the current head node.
- No hard reset on collapse or merge. Strain is a running total.
- Saturation at a configured threshold means "unstable" — but consequences are defined by gameplay policy, not by the scalar itself.

### Security Envelope & Capability Assertions

Wrap persistent blocks with a security envelope.

```ts
interface SecurityEnvelope {
    readonly hash: string;
    readonly signature?: string;
    readonly signerId?: string;
}
```

Diffs, snapshots, and merges carry envelopes. Capability enforcement determines which adapters can mutate which component domains; violations raise deterministic errors (e.g., `ERR_CAPABILITY_DENIED`).

**Capability token format** is defined in [spec-capabilities-and-security.md](spec-capabilities-and-security.md). This spec records only what the branch tree needs for audit and replay:

```ts
/** What branch-tree persists for capability audit. */
interface CapabilityAssertion {
    readonly tokenDigest: Hash; // hash of the full token (format defined elsewhere)
    readonly scope: string; // e.g., "components:physics", "branch:fork"
}
```

The full token grammar, issuance, revocation, and scoping rules live in the capabilities spec. Branch-tree stores assertion digests and scope labels; violations emit deterministic error nodes.

### Determinism Invariants

1. **World Equivalence:** identical diff sequences ⇒ identical `worldHash`.
2. **Merge Determinism:** identical inputs + merge decisions ⇒ identical output.
3. **Temporal Stability:** GC, compression, inspector activity do not affect logical state.
4. **Schema Consistency:** mismatched layout hashes block merges.
5. **Causal Integrity:** writes do not modify values they transitively read earlier in Chronos (within a single branch lineage).
6. **Strain Reproducibility:** strain delta derives solely from recorded events and deterministic world config weights.

Violations terminate the tick and record deterministic error nodes.

### Error Model & Recovery

| Failure              | Detection          | Recovery                              | Status                            |
| -------------------- | ------------------ | ------------------------------------- | --------------------------------- |
| Diff apply fails     | checksum mismatch  | discard node, mark branch `corrupted` | deterministic                     |
| Snapshot corrupted   | hash mismatch      | rebuild from last base snapshot       | deterministic                     |
| Capability violation | runtime guard      | abort tick, log error                 | deterministic                     |
| Merge unresolved     | conflict count     | require manual merge node             | deterministic (decision recorded) |
| Paradox              | read/write overlap | isolate branch, emit paradox node     | deterministic                     |

Recovery operations emit synthetic nodes so replay matches origin. In particular, manual merge resolutions are persisted as canonical decision nodes in the timeline, making replays deterministic even when human intervention was required.

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

All mutating operations route through Codex's Baby; determinism invariants enforced at this boundary. Internal systems (storage, scheduler, adapters) remain swappable under the same contract.

---
