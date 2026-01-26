<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Course Glossary (Progressive Vocabulary)

This glossary is ordered from “public intuition” to “internal/formal name”.
The course tries to introduce concepts in this same order.

## Friendly Terms (Use First)

- **Tick**: one step of time in the simulation.
- **Input**: what a player chooses on a tick (move / place balloon).
- **State**: everything that’s true right now (positions, timers, pickups, etc.).
- **Fingerprint**: a compact check that two states match.
- **Replay**: re-running the simulation from the same start + same inputs.
- **Desync**: when two peers disagree (fingerprints diverge).

## Formal / Engine Terms (Introduce After the Idea Lands)

- **Hash**: the cryptographic “fingerprint” of a state (or history artifact).
- **Canonical encoding**: the rule that “the same meaning must serialize to the same bytes”.
- **Determinism**: same start + same inputs => same outputs.
- **Graph**: a set of nodes and edges representing structure/relationships.
- **Attachment**: data stored “on” graph elements (payloads).
- **Rewrite rule**: a rule that matches a pattern and applies edits to state.
- **Footprint**: the declared read set and write set of graph resources (nodes, edges, attachments) a rewrite rule may access.
- **Footprint independence**: the property that two rewrites are independent only if their footprints do not overlap on any combination of accesses (read/write, write/read, or write/write), enabling safe parallel execution.

## Echo / WARP Vocabulary (When the Reader Is Ready)

- **WARP**: Echo’s graph‑rewrite simulation model (state evolves via deterministic rewrites).
- **Two-plane law**: keep structure (graph) visible; don’t hide edges inside opaque bytes.
- **Tick patch**: a canonical delta artifact representing a tick's edits + read/write footprint.
- **FootprintGuard**: runtime enforcement module (`FootprintGuard`) validating every graph access and emitted op against the declared footprint. Active by default in debug builds (`debug_assertions`), additionally gated by the `footprint_enforce_release` feature for release builds, and disabled when the `unsafe_graph` escape hatch is used.
- **FootprintViolation**: a typed panic payload emitted when a rule breaches its declared footprint. Carries structured info: the offending op, the declared sets, and a `ViolationKind`.
- **ViolationKind**: enum classifying footprint breaches — `NodeRead`, `NodeWrite`, `EdgeRead`, `EdgeWrite`, `AttachmentRead`, `AttachmentWrite`, `CrossWarpEmission`, `UnauthorizedInstanceOp`, `AdjacencyViolation`.
- **ExecItemKind**: `System` or `User` — gates whether an executor may emit warp-instance-level ops (create/delete instances, open portals). User rules are always `User`; only internal engine code creates `System` items. Unauthorized attempts raise `UnauthorizedInstanceOp`.

## Demo-Specific Terms

- **Splash**: the set of tiles affected when a balloon bursts.
- **Fuse**: a countdown in ticks until a balloon bursts.
