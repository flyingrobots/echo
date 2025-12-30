<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Snapshot Commit Spec (v2)

This document precisely defines the two hashes produced by the engine when recording state and provenance.

- state_root: BLAKE3 of the canonical encoding of the reachable graph under the current root.
- commit hash (commit_id): BLAKE3 of a header that includes state_root, parent commit(s), the tick patch digest (boundary delta), plus a policy id.

## 1. Canonical Graph Encoding (state_root)

Inputs: GraphStore, root NodeId.

Deterministic traversal:
- Reachability: BFS from root following outbound edges; only reachable nodes and edges are included.
- Node order: ascending NodeId (lexicographic over 32-byte ids).
- Edge order: for each source node, include only edges whose destination is reachable; sort by ascending EdgeId.

Encoding (little-endian where applicable):
- Root id: 32 bytes.
- For each node (in order):
  - node_id (32), node.ty (32), payload_atom.
- For each source (in order):
  - from_id (32), edge_count (u64 LE) of included edges.
    - edge_count is a 64-bit little-endian integer and may be 0 when a source
      node has no outbound edges included by reachability/ordering rules.
  - For each edge (in order):
    - edge.id (32), edge.ty (32), edge.to (32), payload_atom.

Where `payload_atom` is encoded as:
- `present: u8` (`0` = None, `1` = Some)
- when present: `payload_type_id: 32`, `payload_len: u64 LE`, then raw bytes.

Hash: blake3(encoding) → 32-byte digest.

## 2. Commit Header (commit_id)

Header fields (v2):
- version: u16 = 2
- parents: Vec<Hash> (length u64 LE, then each 32-byte hash). Genesis commits
  have zero parents (length = 0).
- state_root: 32 bytes (from section 1)
- patch_digest: 32 bytes (digest of the tick patch boundary delta)
- policy_id: u32 (version pin for Aion policy)

Hash: blake3(encode(header)) → commit_id.

### 2.1 patch_digest (Tick patch digest)

`patch_digest` commits to the tick patch boundary artifact: a replayable delta
patch with canonical ops and conservative in/out slot sets.

Canonical encoding for the tick patch (v1) is defined in `docs/spec-warp-tick-patch.md`.

---

## 3. Diagnostic Digests (not committed into commit_id v2)

Echo retains several deterministic digests on `Snapshot` for debugging and
tooling, but commit hash v2 intentionally does **not** commit to them.

### 3.1 plan_digest

`plan_digest` is a deterministic digest of the candidate ready set and its
canonical ordering (encoded as a length-prefixed list; empty list =
`blake3(0u64.to_le_bytes())`).

### 3.2 decision_digest (Tick receipt digest)

Until Aion integration lands, `decision_digest` commits to the **tick receipt**
outcomes (accepted vs rejected candidates).

Canonical encoding (v1) for the tick receipt digest:

- If the tick receipt has **0 entries**, `decision_digest` is the canonical empty
  digest: `blake3(0u64.to_le_bytes())` (matches `DIGEST_LEN0_U64`).
- Otherwise, compute `blake3(encoding)` where `encoding` is:
  - `version: u16 = 1`
  - `count: u64` number of entries
  - For each entry (in canonical plan order):
    - `rule_id: 32`
    - `scope_hash: 32`
    - `scope: 32` (raw 32-byte `NodeId` inner value: `NodeId.0`)
    - `disposition_code: u8`
      - `1` = Applied
      - `2` = Rejected(FootprintConflict)

Note: `TickReceipt` may expose additional debugging/provenance metadata (e.g. a
blocking-causality witness for rejections). `decision_digest` v1 intentionally
commits only to accepted vs rejected outcomes (and the coarse rejection code),
not to the blocker metadata.

### 3.3 rewrites_digest

`rewrites_digest` is a deterministic digest of the ordered rewrites applied
during the tick (encoded as a length-prefixed list; empty list =
`blake3(0u64.to_le_bytes())`).

---

## 4. Invariants and Notes

- Any change to ordering, lengths, or endianness breaks all prior hashes.
- The commit_id (v2) is stable across identical states and patch deltas, independent of runtime.
- The canonical empty digest for *length-prefixed list digests* is
  `blake3(0u64.to_le_bytes())` (not `blake3(b"")`). This matches the engine’s
  `DIGEST_LEN0_U64` constant and keeps empty-digest semantics consistent with the
  encoding strategy (the length prefix is part of the canonical byte stream).

## 5. Future Evolution

- v3 (and later) may add additional fields (e.g., signer, timestamp) and bump header version.
- Migrations must document how to re-compute commit_id for archival data.
