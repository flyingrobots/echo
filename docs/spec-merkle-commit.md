<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Snapshot Commit Spec (v2)

This document precisely defines the two hashes produced by the engine when recording state and provenance.

- state_root: BLAKE3 of the canonical encoding of the reachable WARP state under the current root (including attachments and descended instances).
- commit hash (commit_id): BLAKE3 of a header that includes state_root, parent commit(s), the tick patch digest (boundary delta), plus a policy id.

## 1. Canonical State Encoding (state_root)

Inputs: `WarpState`, root `NodeKey`.

Deterministic traversal:
- Reachability: deterministic BFS from the root `NodeKey` across instances:
  - within an instance: follow outbound skeleton edges
  - across instances: follow any `AttachmentValue::Descend(child_warp)` encountered on:
    - reachable nodes (α plane), and
    - reachable edges (β plane)
  - descending enqueues the `WarpInstance.root_node` of `child_warp`
- Warp order: reachable `WarpId` values in ascending order (lexicographic over 32-byte ids).
- Node order: ascending `NodeId` within each warp (lexicographic over 32-byte ids), filtered to reachable nodes in that warp.
- Edge order: per reachable source bucket, sort edges by ascending `EdgeId`, and include only edges whose destination node is reachable.

Encoding (little-endian where applicable):
- Root key:
  - `root_warp_id: 32`
  - `root_node_id: 32`
- For each reachable warp (in order):
  - Instance header:
    - `warp_id: 32`
    - `root_node_id: 32` (the instance-local root)
    - `parent_key_opt`:
      - `present: u8` (`0` = None, `1` = Some)
      - when present: `attachment_key` (see below)
  - For each reachable node in that warp (in order):
    - `node_id: 32`
    - `node_type_id: 32`
    - `attachment_value_opt` for the node’s α-plane slot (see below)
  - For each reachable source bucket in that warp (in order):
    - `from_node_id: 32`
    - `edge_count: u64 LE` (number of included edges after reachability filter)
    - For each included edge (in order):
      - `edge_id: 32`
      - `edge_type_id: 32`
      - `to_node_id: 32`
      - `attachment_value_opt` for the edge’s β-plane slot (see below)

Where:
- `attachment_key` encodes the identity of an attachment slot:
  - `owner_tag: u8` (`1` = Node, `2` = Edge)
  - `plane_tag: u8` (`1` = Alpha, `2` = Beta)
  - `owner_warp_id: 32`
  - `owner_local_id: 32` (raw `NodeId` or `EdgeId` bytes)
- `attachment_value_opt` encodes the value stored in an attachment slot:
  - `present: u8` (`0` = None, `1` = Some)
  - when present:
    - `value_tag: u8` (`1` = Atom, `2` = Descend)
    - if Atom: `payload_type_id: 32`, `payload_len: u64 LE`, then raw bytes
    - if Descend: `child_warp_id: 32`

Hash: blake3(encoding) → 32-byte digest.

## 2. Commit Header (commit_id)

Header fields (v2):
- version: u16 = 2
- parents: `Vec<Hash>` (length u64 LE, then each 32-byte hash). Genesis commits
  have zero parents (length = 0).
- state_root: 32 bytes (from section 1)
- patch_digest: 32 bytes (digest of the tick patch boundary delta)
- policy_id: u32 (version pin for Aion policy)

Hash: blake3(encode(header)) → commit_id.

### 2.1 patch_digest (Tick patch digest)

`patch_digest` commits to the tick patch boundary artifact: a replayable delta
patch with canonical ops and conservative in/out slot sets.

Canonical encoding for the tick patch (v2) is defined in `docs/spec-warp-tick-patch.md`.

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

### 3.4 admission_digest (Stream admission decisions)

`admission_digest` is a deterministic digest of the `StreamAdmissionDecision`
records produced for the tick (see `docs/spec-time-streams-and-wormholes.md`).

Purpose:
- Make stream admission part of HistoryTime (foldable, replay-safe) without
  changing commit hash v2.
- Provide an integrity pin for time travel tooling (rewind/catch-up/merge UX),
  so “why/how was this admitted?” is auditable from history rather than
  re-derived from HostTime.

Canonical encoding (v1) for `admission_digest`:

- If there are **0** admission decisions for the tick, `admission_digest` is the
  canonical empty digest: `blake3(0u64.to_le_bytes())` (matches `DIGEST_LEN0_U64`).
- Otherwise, compute `blake3(encoding)` where `encoding` is:
  - `version: u16 = 1`
  - `count: u64` number of decision records
  - For each decision, in canonical order:
    - Primary sort key: `(view_id, stream_id)` (lexicographic on canonical bytes)
    - Secondary: `admit_at_tick` (u64 LE), then `admitted_range.from_seq` (u64 LE)
  - Each decision record encodes:
    - `view_id_len: u64`, then `view_id_bytes`
    - `stream_id_len: u64`, then `stream_id_bytes`
    - `admit_at_tick: u64`
    - `policy_hash: 32`
    - `budget_max_events_present: u8` then `budget_max_events: u64` if present
    - `budget_max_bytes_present: u8` then `budget_max_bytes: u64` if present
    - `budget_max_work_present: u8` then `budget_max_work: u64` if present
    - `fairness_order_digest_present: u8` then `fairness_order_digest: 32` if present
    - `admitted_set_tag: u8`
      - `1` = Range (inclusive): `from_seq: u64`, `to_seq: u64`
      - `2` = Sparse list: `count: u64`, then each `seq: u64`
    - `admitted_digest: 32`

Notes:
- `view_id` and `stream_id` are treated as opaque identifiers at this layer; the
  canonical encoding is length-prefixed bytes so we can use stable string or
  binary identifiers without ambiguity.
- The `worldline_ref` (universe/branch) is already pinned by the snapshot header
  (`commit_id` ancestry + branch metadata in higher layers); `admit_at_tick`
  anchors the decision into Chronos.
- The `decision_id` used for cross-references in `StreamAdmissionDecision` is
  derived (not separately encoded) to avoid circularity and to keep the
  admission digest stable. Compute it as:
  - `decision_id = blake3("echo:stream_admission_decision_id:v1\0" || decision_record_bytes_v1)`
  - where `decision_record_bytes_v1` is exactly the per-decision record encoding
    bytes listed above (from `view_id_len` through `admitted_digest`).

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
