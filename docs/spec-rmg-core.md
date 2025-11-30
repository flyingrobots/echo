<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Recursive Meta Graph (RMG) Core Specification v2

## 1. Purpose
Recursive Meta Graph (RMG) is a typed, deterministic graph-rewriting engine implemented in Rust. It provides atomic, in-place edits of recursive meta-graphs with deterministic local scheduling and snapshot isolation. RMG is the substrate for the Echo engine: runtime, assets, networking, and tools all operate on the same living graph.

## 2. Core Principles
| Principle | Description |
| --- | --- |
| Everything is a Graph | Nodes, edges, and rewrite rules are graphs. |
| Recursive | Graphs contain subgraphs without limit. |
| Typed | Every record carries a type hash and schema metadata. |
| DPOi Graph Rewriting | Deterministic parallel-order Double Pushout rewrites. |
| Atomic In-Place Editing | Transactions mutate the live graph with snapshot isolation. |
| Confluence | Independent rewrite sequences converge to identical canonical graphs. |
| Snapshots, not Logs | Snapshots emitted from live graph; append-only history optional for archival. |
| Deterministic Scheduling | Rule application order derived from rule id + scope hash. |
| QCA-Ready | Rules can encode reversible/superposed transformations for quantum simulations. |

## 3. Runtime Model
### 3.1 Core Structures (Rust)
```rust
struct RmgEngine {
    graph: GraphStore,
    scheduler: DeterministicScheduler,
    rules: Vec<RewriteRule>,
}

struct GraphStore {
    nodes: HashMap<Hash, NodeRecord>,
    edges: HashMap<Hash, EdgeRecord>,
}

struct RewriteRule {
    id: Hash,
    left: PatternGraph,
    right: PatternGraph,
    constraint: Option<GuardExpr>,
}
```

### 3.2 Atomic Transactions
```rust
engine.begin_tx();
engine.apply(rule_set);
engine.commit();   // atomically swap pointers; emit snapshot hash
```
All reads during a transaction see a consistent snapshot (snapshot isolation).

### 3.3 Deterministic Local Scheduler
- Each rewrite computes a scope hash from affected nodes/edges.
- Scheduler orders scopes lexicographically; conflicts resolved by rule priority + hash.
- Concurrent rewrites on disjoint scopes execute in parallel, ensuring confluence.

## 4. Binary Representation
- NodeRecord/EdgeRecord/PayloadArena remain as in v1.
- Additional SnapshotHeader metadata stored alongside snapshots:
```rust
struct SnapshotHeader {
    Hash parent;
    u64 tx_id;
    u64 rule_count;
    u64 timestamp; // logical tick, not wall time
}
```

## 5. Rewriting Semantics
1. Match: find injective morphism of left pattern in host graph.
2. Check: evaluate guard constraints.
3. Delete: remove nodes/edges in left not in interface K.
4. Add: insert nodes/edges from right not in K.
5. Commit: update graph store atomically; emit new snapshot hash.

Rewrite operations are stored as `RewriteRecord` nodes for audit and replay.

## 6. Confluence Layer
- Peers apply identical ordered rule sets to local graphs.
- Rewrite transactions broadcast as `{tx_id, rule_id, scope_hash, snapshot_hash}`.
- Deterministic merge ensures all peers converge on the same snapshot hash.
- Conflicts resolved via rule precedence and canonical ordering; paradoxes quarantined if constraints fail.

## 7. Snapshots & Persistence
- Snapshots are immutable, content-addressed views of the live graph.
- Exportable via `rmg snapshot`; can be streamed over network.
- Append-only archival logs optional for audit / rollback recovery.

## 8. API Surface
### Rust
```rust
engine.register_rule(rule);
engine.begin_tx();
engine.rewrite("physics:update");
let snap = engine.snapshot();
```

### Lua
```lua
rmg.apply("update/transform", {entity = id})
```

### TypeScript (WASM)
```ts
await rmg.apply("ui/paint");
const graph = await rmg.snapshot();
```

## 9. Determinism Invariants
1. Rule application order determined solely by `(rule_id, scope_hash)`.
2. Identical initial graph + rewrite history ⇒ identical snapshot hash.
3. Snapshots/logging/network replication side-effect free; no hidden state.
4. Rewrites commute when scopes disjoint; engine enforces via DPOi scheduler.
5. Confluent peers always converge on same canonical graph.

## 10. Networking & Confluence Protocol
- Transactions streamed as rewrite packets referencing parent snapshot hash.
- Peers validate hash chains, apply rules locally via deterministic scheduler.
- Quantum/probabilistic rewrites tagged with amplitudes; simulated deterministically.

## 11. Tooling Hooks
| Tool | Role |
| --- | --- |
| `rmg` CLI | apply rules, emit snapshots, verify confluence |
| Echo Studio | visualize live graph, rewrites, merges |
| Analyzer | verify rule commutativity, determinism proofs |

## 12. Security & Capabilities
- Capability tokens restrict rule classes (e.g., `world:rewrite`, `asset:import`).
- Atomic transactions validated pre-commit.
- Snapshots optionally signed (Ed25519); peers reject invalid hashes.

## 13. Roadmap
| Phase | Deliverable |
| --- | --- |
| 1.5 | `rmg-core` with typed DPOi engine and deterministic scheduler |
| 2.0 | Integrate Echo ECS & assets as rewrite schemas |
| 2.5 | Implement Confluence networking layer |
| 3.0 | Tooling for live editing, graph diff visualization, snapshot verification |

**TL;DR:** RMG v2 is a deterministic, typed DPOi graph-rewriting engine with atomic in-place updates and confluent distributed synchronization. Echo’s runtime, assets, and tools run atop the same living graph.
