<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005 - Provenance Payload and Boundary Records

_Define provenance payloads as retained shells for replay, audit, and "show me why" queries._

Depends on:

- [SPEC-0004 - Worldlines, Playback, and Observation](SPEC-0004-worldlines-playback-truthbus.md)
- [WARP Tick Patch](warp-tick-patch.md)
- [Merkle Commit](merkle-commit.md)

## Purpose

Echo needs a way to package causal history without confusing that package with the public observation contract. A retained shell can carry replay and audit material, while an observation artifact is the observer-relative reading emitted from that material.

A payload must support a trustworthy answer to "why did this value appear?" A
consumer can replay its patches, verify the hash triplets, and inspect the
causal slot chain that produced a target value.

It is also a compact deterministic artifact for exchange: a consumer can
verify a worldline segment's boundary record and derive a slot's backward
causal cone without asking the live runtime to re-execute rules.

## Decision 1: ProvenancePayload packages a worldline segment

A provenance payload is an ordered sequence of tick patches plus expected hashes: `ProvenancePayload { worldline_id, u0, patches, expected }`. The payload is contiguous over its tick range.

## Decision 2: BoundaryTransitionRecord is the verification envelope

A boundary transition record binds input state root, output state root, initial/checkpoint reference, provenance payload digest, tick coordinate, policy id, and boundary hash.

## Decision 3: Provenance graphs are derived from slot I/O

The causal graph is derived from patch slot declarations: an edge from `mu_i` to `mu_j` exists when `out_slots(mu_i)` intersects `in_slots(mu_j)`.

## Decision 4: Partial stores expose incomplete causality

If a provenance store does not contain a producer for an input slot, the query must report an external or unavailable input. It must not fabricate a causal edge.

## Decision 5: Payloads are not the public read contract

Provenance payloads, boundary records, and derivation graphs are audit and replay material. The public read surface is the observation artifact.
