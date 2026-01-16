<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP Confluence Specification (Phase 0.75)
> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).


The Confluence is the global DAG formed by interconnected WARP graphs. It defines how local stores project into a shared graph, how graph deltas propagate, and how conflicts resolve deterministically.

---

## Concept
- **Local Graph** – a single WARP file representing a world, timeline, or asset bundle.
- **Confluence** – the union of all local graphs, content-addressed by hash, forming a global Merkle DAG.
- **Projection** – mapping a local graph into the Confluence by submitting diff blocks.
- **Synchronization** – pulling new diff blocks from the Confluence to update a local store.

---

## Data Model
```
Confluence (global)
├─ NodeSegments (append-only)
├─ EdgeSegments (append-only)
├─ PayloadSegments
├─ Index: Hash -> SegmentOffset
└─ Journal: ordered list of submissions (node/edge hashes, signer, capability)
```

Each submission references hashes from local diff graphs. Deduplication occurs when identical hashes already exist.

---

## Protocol

### Submit (push)
1. Client computes DiffGraph between local root and last synchronized root.
2. For each new node/edge:
   - Upload block (node, edge, payload) to Confluence store.
   - Provide capability tokens and (optional) signature.
3. Append entry to Journal: `{ root_hash, parent_hash, diff_hashes, signer }`.

### Sync (pull)
1. Client reads Journal entries since last sync.
2. Download missing blocks (by hash); append to local store.
3. Apply merges if divergent branches exist (per Echo merge rules).

---

## Conflict Resolution
- Confluence merge uses same deterministic three-way merge strategy as Echo branch tree.
- Paradox detection occurs during merge; paradox nodes recorded in Journal.
- Failed merges quarantine submitted diff until manual resolution (via capability `timeline:merge`).

---

## Security
- Journal entries signed (Ed25519) and capability-scoped.
- Confluence rejects blocks that conflict with capabilities or fail hash validation.
- Audit logs allow replay of every submission.

---

## API (Rust)
```rust
pub trait Confluence {
    fn submit(&mut self, diff: DiffGraph, signer: Signer) -> Result<RootHash>;
    fn pull(&mut self, since: JournalCursor) -> Result<Vec<DiffGraph>>;
    fn current_root(&self) -> RootHash;
}
```

Local projection uses `submit`. Synchronization uses `pull` + merge.

---

## Determinism
- Hashes guarantee identical content merges regardless of submission order (commutative under canonical merge rules).
- Journal order provides canonical history for replay.
- Every branch of the Confluence is just another WARP graph; local stores can fork/merge from any point.

---

## Tooling
- Confluence browser: visualize global DAG, submissions, and merges.
- CLI: `warp confluence submit`, `warp confluence sync`, `warp confluence log`.

---

The Confluence turns Echo’s deterministic worlds into a distributed multiverse: every branch, asset, and timeline is a first-class graph in a shared immutable history.
