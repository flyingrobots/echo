<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0001 - Attachment Plane v0 Typed Atoms

_Define the depth-0 attachment contract: typed bytes may ride with structure, but they do not become hidden structure._

Legend: KERNEL

Depends on:

- [Echo Runtime Model](../architecture/outline.md)
- [warp-core](warp-core.md)
- [Merkle Commit](merkle-commit.md)

## Why this packet exists

Echo's hot path works over witnessed causal structure. Attachments are allowed because real applications need payloads, but payload bytes cannot secretly change matching, scheduling, replay, or observation. This packet keeps the attachment plane useful while preserving the distinction between causal history and observer-relative readings.

## Human users / jobs / hills

Human users need payloads to carry domain state without making the runtime uninspectable.

The hill: a reviewer can look at a commit hash, replay patch, or scheduler footprint and know that no hidden edge was smuggled through an attachment blob.

## Agent users / jobs / hills

Agent users need stable rules for decoding payloads only when a declared rule, view, or inspector asks for that interpretation.

The hill: an agent can audit attachment usage by checking type ids, slot ids, and footprint declarations without reverse-engineering opaque bytes.

## Decision 1: Attachments are typed atoms at depth 0

Depth 0 admits one attachment value shape:

```text
AtomPayload { type_id: TypeId, bytes: Bytes }
```

The `type_id` is part of the value. Same bytes under different type ids are different payloads.

Implementation evidence: `crates/warp-core/src/attachment.rs`, `crates/warp-core/src/ident.rs`, `crates/warp-core/tests/atom_payload_digest_tests.rs`.

## Decision 2: Skeleton records stay skeleton-only

Node and edge records define structure and type. Attachment bytes live in the attachment plane: node attachments on alpha, edge attachments on beta.

Matching and indexing forget attachments unless a rule explicitly reads an attachment slot. The skeleton is the causal carrier; decoded payload meaning is part of a reading.

Implementation evidence: `crates/warp-core/src/graph.rs`, `crates/warp-core/src/footprint.rs`, `docs/invariants/warp-two-plane-law.md`.

## Decision 3: Payload bytes never create hidden edges

If structure matters to matching, scheduling, slicing, replay, or causality, it must be represented as skeleton structure, a boundary port, or an explicit attachment slot read/write.

## Decision 4: Decode failure means "rule does not apply"

Rules may decode atoms through a deterministic codec boundary. Type mismatch or strict decode failure means no match. Partial mutation after decode failure is forbidden.

## Decision 5: Hashes commit to type and bytes

Any canonical state or patch encoding that includes an attachment atom commits to presence, value tag, payload type id, payload length, and payload bytes.

## Consequences

The rewrite hot path stays fast and deterministic. Rules that need payload semantics must declare their reads and decode at a typed boundary. Tools may present rich payload views, but those views are observer-relative readings over witnessed state.

## Open work

Descended attachments are handled by [SPEC-0002](SPEC-0002-descended-attachments-v1.md). Finer-grained payload slicing requires explicit slots rather than treating bytes as discoverable substructure.
