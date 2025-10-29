# Snapshot Commit Spec (v1)

This document precisely defines the two hashes produced by the engine when recording state and provenance.

- state_root: BLAKE3 of the canonical encoding of the reachable graph under the current root.
- commit hash (commit_id): BLAKE3 of a header that includes state_root, parent commit(s), and deterministic digests of plan/decisions/rewrites, plus a policy id.

## 1. Canonical Graph Encoding (state_root)

Inputs: GraphStore, root NodeId.

Deterministic traversal:
- Reachability: BFS from root following outbound edges; only reachable nodes and edges are included.
- Node order: ascending NodeId (lexicographic over 32-byte ids).
- Edge order: for each source node, include only edges whose destination is reachable; sort by ascending EdgeId.

Encoding (little-endian where applicable):
- Root id: 32 bytes.
- For each node (in order):
  - node_id (32), node.ty (32), payload_len (u64 LE), payload bytes.
- For each source (in order):
  - from_id (32), edge_count (u64 LE) of included edges.
  - For each edge (in order):
    - edge.id (32), edge.ty (32), edge.to (32), payload_len (u64 LE), payload bytes.

Hash: blake3(encoding) → 32-byte digest.

## 2. Commit Header (commit_id)

Header fields (v1):
- version: u16 = 1
- parents: Vec<Hash> (length u64 LE, then each 32-byte hash)
- state_root: 32 bytes (from section 1)
- plan_digest: 32 bytes (canonical digest of ready-set ordering; empty list = blake3 of zero bytes)
- decision_digest: 32 bytes (Aion/agency inputs; currently may be canonical empty until Aion lands)
- rewrites_digest: 32 bytes (ordered rewrites applied)
- policy_id: u32 (version pin for Aion policy)

Hash: blake3(encode(header)) → commit_id.

## 3. Invariants and Notes

- Any change to ordering, lengths, or endianness is a breaking change and invalidates previous hashes.
- The commit_id is stable across identical states and provenance, independent of runtime.
- The canonical empty digest is blake3 of zero bytes; use this for empty plan/rewrites until populated.

## 4. Future Evolution

- v2 may add additional fields (e.g., signer, timestamp) and bump header version.
- Migrations must document how to re-compute commit_id for archival data.
