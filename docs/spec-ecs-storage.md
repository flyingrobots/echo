# Echo ECS Storage Blueprint (Phase 0)

This document specifies the data layout and algorithms for Echo’s entity/component storage. It complements the high-level architecture outline and will guide the first implementation pass in `@echo/core`.

---

## Goals
- Deterministic entity/component management with O(1) iteration order determined by archetype IDs.
- Cache-friendly traversal for systems by storing component columns contiguously.
- Support branchable timelines via copy-on-write chunking with minimal duplication.
- Enable metadata tracking for debugging, profiling, and diff generation.

## Terminology
- **Component Type ID**: Stable numeric identifier assigned during component registration (`ComponentTypeRegistry`).
- **Archetype Signature**: Bitset/hashed set of component type IDs describing the component mix for entities in a chunk.
- **Chunk**: Fixed-size container (default 16 KB) that stores columnar component data and entity bookkeeping for one archetype.
- **Slot**: Row within a chunk; each slot corresponds to one entity.
- **Entity Handle**: 64-bit value composed of generation + index enabling safe recycling.
- **Branch ID**: Kairos identifier used to differentiate timeline views; tie-ins with timeline tree.

---

## Data Structures

### ComponentTypeRegistry
```ts
interface ComponentTypeDescriptor {
  readonly id: number;
  readonly name: string;
  readonly size: number;            // bytes for POD types; 0 for managed objects
  readonly alignment: number;       // power-of-two alignment requirement
  readonly schemaHash: string;      // for serialization/diff sanity
  readonly defaultValueFactory?: () => unknown;
}
```
- Maintains `Map<string, ComponentTypeDescriptor>` and sequential ID issue counter.
- Emits deterministic IDs by sorting registration requests by lexical component name during boot.

### EntityTable
```ts
interface EntityRecord {
  readonly generation: number;
  archetypeId: number;
  chunkIndex: number;
  slotIndex: number;
}
```
- `entityTable: EntityRecord[]` sized to max entity count (grows via doubling).
- `freeList: number[]` for recycling; `generation` increments when reusing an index.

### ArchetypeGraph
- `signature -> ArchetypeId` map (signature stored as sorted array + hashed string).
- `adjacency: Map<ArchetypeId, Map<ComponentTypeId, ArchetypeId>>` for fast transitions when adding/removing components.

### Chunk Layout
```
Chunk {
  archetypeId: number
  branchId: KairosBranchId
  capacity: number      // computed from chunk size & component column sizes
  size: number          // active slots
  version: number       // incremented per mutation (for diff + branch merges)
  columnOffsets: Map<ComponentTypeId, ByteOffset>
  generation: number    // chunk recycling guard
  data: ArrayBuffer     // raw storage; reused across branches via COW
  freeList: number[]    // slot indices available (optional, for fragmentation)
}
```
- Column offsets computed at archetype creation; each column contiguous.
- Managed components (non-POD) stored as references in side arrays keyed by `componentTypeId`.
- `data` allocated via `SharedArrayBuffer` if environment permits; fallback to `ArrayBuffer`.

### Branch Metadata
- `branchRefCounts: Map<ChunkId, number>` for copy-on-write tracking.
- `branchSnapshots` record chunk version + size at fork to enable diffing.

---

## Operations

### Entity Creation
1. Pop index from `freeList` or extend `entityTable`.
2. Look up empty archetype (signature = Ø). Ensure chunk with spare capacity exists (allocate if not).
3. Write entity ID into chunk slot; initialize columns with default values.
4. Update `EntityRecord` with archetype/chunk/slot refs.
5. Increment chunk `version` and size counters.

### Add Component
1. Determine target archetype via `adjacency[currentArchetype][componentTypeId]`; compute on demand if missing.
2. Ensure destination chunk has capacity. If none, allocate new chunk from pool.
3. Copy entity data:
   - For each component present in both archetypes, copy column data from source slot to destination slot.
   - Initialize new component column from descriptor default.
4. Remove entity from source chunk (swap-remove with last slot to avoid gaps). Update entity record of swapped entity.
5. Update entity record to new chunk/slot, bump relevant chunk versions.
6. If chunk becomes empty, return to pool (retain for same branch for reuse).

### Remove Component
- Mirror of Add but using adjacency edge removing type; ensures default archetype exists (Ø).

### Mutate Component
- Mutations operate on column slices.
- For POD data, writes happen directly into chunk buffer.
- For managed data, maintain separate arrays and ensure clone-on-write semantics (structured clone or user-provided copy).
- Mutation should update chunk `version` (per branch) and emit dirty flags for diffing.

### Destroy Entity
- Remove from chunk using swap-remove.
- Push index back to `freeList`, increment generation.
- If chunk empty, release or keep in free pool.

---

## Copy-On-Write for Branches
1. When forking a branch, increment ref count for each chunk touched by the source world.
2. Mutating a chunk in a branch:
   - If ref count > 1, allocate new chunk buffer, copy column data, decrement source ref count, assign new buffer to branch chunk.
   - Update branch chunk `branchId` and reset local `version`.
3. Diff generation: compare `version` and `size` to snapshot metadata; store per-component bitmask of modified slots.
4. Garbage collection: when branch collapses/merges, decrement ref counts; if zero and chunk not referenced by other branches, return to pool.

---

## Memory & Pooling
- Chunk allocation uses slab allocator per archetype to reduce fragmentation.
- Pools keyed by `(archetypeId, capacity)` enable quick reuse after entity churn.
- Provide configuration to tune chunk size (default 16 KB) and align to cache line (64 bytes).

---

## Instrumentation
- Maintain counters: chunks allocated, chunk reuse hits, copy-on-write copies, mutation frequency.
- Provide debug API to dump archetype sizes, component occupancy, and branch divergence stats.
- Hooks for timeline inspector to visualize chunk lifecycle.

---

## Determinism Considerations
- Deterministic iteration order: iterate archetypes in sorted ID order, chunks by creation ID, slots by ascending index.
- Allocation choices (chunk selection) must be stable: use round-robin with deterministic starting index seeded per branch.
- Avoid JS object iteration order reliance; store explicit arrays for archetype/chunk registries.

---

## Open Questions
- Do we expose a streaming API for massive entity creation (batch builder) to cut down copy churn?
- How aggressively should we compress diff bitmasks for large worlds? Evaluate run-length encoding vs bitmap snapshots.
- Interaction with scripting languages (e.g., user-defined components) — need extension points for custom allocation?
- Evaluate fallback for environments lacking `SharedArrayBuffer` (maybe optional).

Document updates should flow into implementation tickets and tests (see execution-plan backlog). Once verified, record results in the decision log.
