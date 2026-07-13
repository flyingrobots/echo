<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0008: Bunny Owns Reusable Geometry

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

Echo accumulated reusable math, geometry, query, mesh, and renderer-oriented
types even though its durable responsibility is causal admission and proof.
Keeping general graphics primitives in Echo would turn a runtime boundary into
a game-engine substrate.

## Decision

Bunny owns reusable deterministic scalar, vector, matrix, quaternion,
transform, AABB, overlap, raycast, sweep, contact, broad-phase, mesh, optics,
and graphics-schema contracts. Echo owns the causal use of those primitives:
ticks, worldlines, bases, admission, intents, readings, receipts, retention,
transactions, and provenance.

The deterministic scalar contract is signed Q32.32 with ties-to-even conversion
and raw-`i64` cross-language golden vectors. `warp-math` and `warp-geom` are
staging/extraction surfaces, not permanent Echo ontology.

## Consequences

- Pure algorithms move without Echo causal identifiers or authority types.
- Echo adapters translate witnessed causal inputs into Bunny values and bind
  results back into receipts or readings.
- Scene/Three.js presentation does not move into Bunny merely because it uses
  geometry; product presentation remains application-owned.

## Evidence Anchors

- [Runtime constellation](../topics/RuntimeConstellation.md)
- `crates/warp-math`
- `crates/warp-geom`
- `crates/warp-core/src/fixed.rs`
