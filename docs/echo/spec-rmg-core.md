# Recursive Meta Graph (RMG) Core Specification

## 1. Overview
Recursive Meta Graph (RMG) is the unified data model, binary format, and runtime substrate for Echo. It replaces disparate ECS tables, serialization formats, asset bundles, and build pipelines with a single deterministic graph-of-graphs representation.

RMG is designed for:
- Deterministic execution and replay.
- Zero-copy access to data, including large binary payloads.
- Efficient diffing/merging for branching timelines and networking.
- Self-describing schemas so tools and runtime inspect the same structures.

## 2. Core Principles
| Principle | Description |
| --- | --- |
| Everything is a Graph | Nodes, edges, and relationships are themselves graphs. |
| Recursive | Graphs can contain other graphs arbitrarily deep. |
| Content-Addressed | Every record uses a BLAKE3 hash of canonical bytes. |
| Zero-Copy | Binary payloads (meshes, shaders, etc.) stored directly and memory-mapped. |
| Deterministic | Traversal order, hashing, and rewrites are canonical across platforms. |
| Atomic Rewrites | Mutations apply via deterministic graph rewrite rules (P-O DPO). |
| Snapshot Isolation | Snapshots emitted from live graph state without halting execution. |
| Self-Describing | Nodes carry type hashes pointing to their schema graphs. |

## 3. Binary Layout
```
| RMGHeader | NodeTable | EdgeTable | PayloadArena |
```

### 3.1 RMGHeader
```rust
struct RMGHeader {
    u32 magic;              // "RMG\0"
    u16 version_major;
    u16 version_minor;
    u64 node_count;
    u64 edge_count;
    u64 payload_bytes;
    Hash root_hash;         // hash of root graph node
}
```

### 3.2 NodeRecord
```rust
struct NodeRecord {
    Hash id;                // BLAKE3 of canonical bytes
    Hash type_id;           // schema or behavior type
    u64 child_count;
    u64 edge_count;
    u64 payload_offset;     // 0 if none
    u64 payload_size;
    u64 subgraph_offset;    // offset into NodeTable for embedded subgraph (0 if none)
}
```

### 3.3 EdgeRecord
```rust
struct EdgeRecord {
    Hash id;                // hash(from || to || edge_type)
    Hash from;
    Hash to;
    Hash edge_type;
    u64 data_offset;        // optional payload
    u64 data_size;
}
```

### 3.4 PayloadArena
Byte ranges for binary data (meshes, textures, SPIR-V, serialized structs). Node/edge payload offsets reference this arena directly → zero-copy views.

## 4. Canonical Encoding
- All numeric fields little-endian.
- Maps/lists sorted lexicographically by hash.
- Floats canonicalized (`Math.fround`, NaN → 0x7FC00000).
- Identical logical graphs ⇒ identical hashes on any platform.

## 5. Runtime API (Rust)
```rust
pub struct RmgStore { /* memory-mapped */ }

impl RmgStore {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn node(&self, id: &Hash) -> Option<&NodeRecord>;
    pub fn edges_from(&self, id: &Hash) -> EdgeIter;
    pub fn payload<T>(&self, node: &NodeRecord) -> &T;
    pub fn diff(&self, other: &RmgStore) -> DiffGraph;
    pub fn merge(&self, base: &RmgStore, other: &RmgStore) -> MergeResult;
}
```

## 5.1 Deterministic Local Scheduler
- Rewrites scheduled in deterministic order based on (rule priority, target hash).
- Parallel rewrites allowed when pattern matches disjoint subgraphs.
- Conflicts resolved via canonical ordering; last writer wins only when rules declare commutativity.
- Execution emits commit entries:
  ```rust
  struct CommitEntry {
      rule_id: Hash;
      target: Hash;
      snapshot_before: Hash;
      snapshot_after: Hash;
      timestamp: u64; // logical tick
  }
  ```

Execution = deterministic traversal:
```rust
fn execute(node: &NodeRecord, store: &RmgStore) {
    for edge in store.edges_from(&node.id).sorted() {
        let target = store.node(&edge.to).unwrap();
        execute(target, store);
    }
}
```

## 6. Graph Semantics
| Domain concept | RMG realization |
| --- | --- |
| World | Node type `World`; edges to `System` subgraphs |
| System | Node with traversal function + query graph |
| Entity | Node with edges to component nodes |
| Component | Leaf node containing POD payload or refs |
| Timeline/Branch | Graph of graph-diffs; each tick a subgraph |
| Asset bundle | Graph with leaves storing binary payloads |
| Import/Export pipeline | Graph describing conversion steps |

## 7. Rewrite & Confluence
- Rewrites expressed as typed DPO rules over subgraphs.
- Deterministic scheduler orders concurrent rewrites by rule priority and hash to guarantee confluence.
- Commit log stores rewrite transactions; snapshots derive from committed state.
- Confluent semantics: independent rewrites yielding same effect converge to identical canonical graph.

## 8. Determinism Guarantees
1. Traversal order sorted by `(edge_type_hash, to_hash)`.
2. Rewrite scheduler resolves concurrently matched rules deterministically.
3. Snapshots hashed from live graph guarantee identical replay views.
4. Replaying traversal over identical graph and commit log yields identical results.

## 9. Language Bindings
| Language | Role | Binding |
| --- | --- | --- |
| Rust | Core implementation | `rmg-core` crate |
| C / Lua | FFI for scripting, adapters | `rmg-ffi` |
| TypeScript | WASM build for tooling/editor | `rmg-wasm` |

## 10. Tooling Hooks
CLI:
```
rmg diff A.rmg B.rmg
rmg merge base.rmg A.rmg B.rmg
rmg verify world.rmg
```

Editor (Echo Studio): graph viewer, timeline scrubber, diff overlay via WASM API.
Inspector protocol emits JSONL frames with node/edge stats and traversal traces.

## 11. Security & Capabilities
- Immutable graph segments signed with Ed25519; signatures stored in envelopes.
- Capability tokens restrict mutation to authorized node types (aligned with Echo security spec).
- Validation pipeline checks signatures and hash chains before mount.

## 12. Roadmap
| Phase | Deliverable |
| --- | --- |
| 1.5 | `rmg-core` crate: memory-mapped store, hashing, diff engine |
| 2 | Map Echo ECS/timeline to RMG schemas; scheduler as graph executor |
| 3 | Networking/persistence via graph deltas |
| 4 | Echo Studio tooling as RMG visualizers/editors |

**TL;DR**: RMG is a recursive, content-addressed graph-of-graphs. Echo’s runtime, assets, timelines, and tools operate on the same deterministic substrate.
