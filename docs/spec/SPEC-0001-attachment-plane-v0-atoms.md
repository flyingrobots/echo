<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# SPEC-0001: Attachment Plane v0 — Typed Atoms (Depth-0)

Status: Accepted (Implemented)  
Owner: Echo / WARP  
Last reviewed: 2025-12-30

## Goal

Define a safe, deterministic, performance-preserving representation of attachments in Echo:

- Attachments are **typed atoms** by default (depth-0).
- The skeleton rewrite engine remains **skeleton-only** on the hot path.
- Higher layers can decode attachments via strict, deterministic codecs.

This spec is the “Stage B0” foundation for “WARPs all the way down” without slowing rewriting.

## Definitions

### SkeletonGraph

The structural graph used for matching, rewriting, scheduling, patching, hashing, and slicing.

In code today: `warp_core::GraphStore`.

### WarpState

Conceptually:

```
U := (G, A)
```

- `G` is the SkeletonGraph.
- `A` is the attachment plane.

### AtomPayload (depth-0)

Depth-0 means: all attachments are atoms. No descended substructure is represented.

In code today:

```
AtomPayload {
  type_id: TypeId,
  bytes: Bytes,
}
```

### Projection π(U)

`π` forgets attachments:

```
π(G, A) = G
```

Echo’s core rewrite hot path operates over `π(U)` unless a rule explicitly chooses to decode attachments.

## Requirements

### R1 — Typed atoms

Nodes/edges MAY carry an attachment payload. If present, it MUST be a typed atom:

- `AttachmentValue::Atom(AtomPayload)`

Representation rule:
- Node/edge *records* (`NodeRecord`, `EdgeRecord`) are skeleton-only.
- Attachments are stored separately in the attachment plane:
  - node/α plane: `GraphStore.node_attachments: BTreeMap<NodeId, AttachmentValue>`
  - edge/β plane: `GraphStore.edge_attachments: BTreeMap<EdgeId, AttachmentValue>`

### R2 — Payload type_id participates in canonical encoding

Canonical encodings and digests that include payload bytes MUST also include the payload `type_id`.

Rationale: “same bytes, different meaning” collisions are forbidden at the deterministic boundary.

### R3 — No hidden edges

Payload bytes MUST NOT be treated as skeleton structure.

If structure must be visible to rewriting, scheduling, slicing, or causality reasoning, it must be represented explicitly in the SkeletonGraph.

### R4 — Decoding is a boundary concern

Core matching/indexing MUST NOT decode payload bytes by default.

Decoding occurs only in typed boundaries (rules, views, inspectors) when explicitly requested.

### R5 — Deterministic decode failure semantics (v0)

Echo v0 chooses:

- **(A) decode failure ⇒ rule does not apply**

Meaning:

- payload `type_id` mismatch ⇒ matcher must return `false`
- strict decode failure ⇒ matcher must return `false`

This policy is conservative and prevents “partial effects” when payloads are malformed.

## Canonical encoding rules (boundary safety)

### State hashing (state_root)

`state_root` is computed from a canonical reachable-only traversal.

When encoding a node or edge payload:

- emit a 1-byte presence tag (`0` = None, `1` = Some)
- when present: emit `payload.type_id` (32 bytes), then `payload_len: u64 LE`, then raw payload bytes

See: `crates/warp-core/src/snapshot.rs`.

### Tick patch digest (patch_digest)

`patch_digest` commits to the replayable delta ops. Any op that encodes payload bytes MUST use the same typed-atom encoding rule as above (presence tag + `type_id` + length + bytes).

See: `crates/warp-core/src/tick_patch.rs` and `docs/spec-warp-tick-patch.md`.

## API surface

### Atom payload type

- `AtomPayload { type_id, bytes }`
- Helpers:
  - `AtomPayload::new(type_id, bytes)`
  - `AtomPayload::decode_with<C, T>() -> Result<T, DecodeError>`
  - `AtomPayload::decode_for_match<C, T>() -> Option<T>` (implements policy R5)

### Codec boundary

Minimal codec trait:

```
trait Codec<T> {
  const TYPE_ID: TypeId;
  fn encode_canon(value: &T) -> Bytes;
  fn decode_strict(bytes: &Bytes) -> Result<T, DecodeError>;
}
```

Notes:

- Core rewriting does not depend on decoding.
- A small dynamic registry (`CodecRegistry`) exists for tooling layers that want runtime decoding keyed by `TypeId`.

See: `crates/warp-core/src/attachment.rs`.

## Non-goals (v0)

- Descended attachments (`Descend(...)`, attachment-root references, recursive traversal).
- Automatic decoding/traversal in the core rewrite engine.
- Treating payload bytes as embedded graphs.

## Follow-ups (Stage B1)

- Stage B1 is specified in `docs/spec/SPEC-0002-descended-attachments-v1.md`.
