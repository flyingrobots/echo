<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# ADR-0001: Two-plane WARP representation in Echo (SkeletonGraph + Attachment Plane)

Status: Accepted  
Date: 2025-12-29

## Context

The WARP papers define a WARP state as a two-plane structure:

- a **skeleton plane** (structural plane) used for matching, rewriting, scheduling, slicing, and determinism; and
- an **attachment plane** (attachments over vertices/edges) where attachments may be atomic (depth-0) or recursively descended.

Echo historically represented nodes/edges with optional opaque byte payloads. That created ambiguity:

- Are we implementing a full WARP state, or only the skeleton projection `π(U)`?
- Are payload bytes “atoms,” or can they contain hidden structural subgraphs?
- Without strict typing, “same bytes, different meaning” can collide at the hash/digest boundary.

Echo also requires that graph rewriting remain fast: core matching/indexing must not traverse or decode attachment structures unless explicitly requested by a rule.

## Decision

1) Echo’s core in-memory store is defined as **SkeletonGraph** (the skeleton plane / `π(U)`).

2) Attachments exist but are represented as **typed atoms** by default (depth-0):

- `AtomPayload = { type_id: TypeId, bytes: Bytes }`

3) Any semantic meaning of payload bytes is enforced at **typed boundaries** via strict codecs:

- decoding is explicit and deterministic;
- decoding does not occur in the core scheduler/matcher unless a rule chooses to decode.

4) **No hidden edges**:

- If a dependency matters for matching, causality, slicing, or rewrite applicability, it must be represented explicitly in the skeleton plane.
- Payload bytes must not be used to smuggle graph structure the engine cannot see.

5) Future recursion in attachments is represented as **flattened indirection** (future work):

- recursion is modeled via explicit references (e.g., attachment-root references / skeleton-visible links), not nested Rust structs.

## Laws (Invariants)

Echo treats the following as project laws:

- Skeleton rewrites never decode attachments.
- Attachment decoding happens only in typed boundaries (rules/views), never in core matching/indexing.
- `type_id` participates in canonical encoding/digest; identical bytes with different `type_id` must not collide.
- No hidden edges in payload bytes.

See also: `docs/warp-two-plane-law.md`.

## Consequences

Pros:

- Preserves rewrite performance and determinism (hot path remains skeleton-only).
- Makes attachment typing explicit and safe.
- Clarifies terminology: SkeletonGraph vs. WarpState vs. `π(U)`.
- Establishes a clean path to “WARPs all the way down” via explicit indirection.

Cons:

- Adds `TypeId` + codec plumbing for payloads.
- Call sites must choose/define payload type identifiers (or use helper constructors).
- Full descended attachments require additional design work (Stage B1).

## Alternatives considered

- Store nested WARP graphs directly inside payload bytes: rejected (hides structure from matching/slicing and causes correctness/perf hazards).
- Represent recursion via recursive Rust types: rejected (poor fit for sharing, determinism, patch/slice semantics, and stable hashing).

## Follow-ups

- SPEC for Attachment Plane v0 (Atoms): codec boundary, deterministic error semantics, canonical encoding rules.
- Stage B1 proposal: descended attachments using explicit references/links (no hidden edges).

