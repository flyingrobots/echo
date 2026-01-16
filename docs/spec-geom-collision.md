<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Geometry & Collision (Spec Stub)
> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).


**Status: not yet re-specified.** This repo currently carries an interactive DPO tour and diagram assets, but the full written spec for Echo’s geometry/collision subsystem is pending re-homing into the Rust-first era.

## Scope (Intended)

- Deterministic broad phase and narrow phase modeled as graph rewrites.
- Canonical identifiers for bodies, shapes, and contacts.
- Collision events emitted as deterministic graph deltas.
- CCD as a deterministic, replayable sequence of rewrite steps.

## Non-Goals (For Now)

- Physics engine replacement (Box2D/Rapier integrations remain adapters).
- GPU-accelerated collision or platform-specific broad-phase shortcuts.
- Real-time authoring tools (tracked separately in editor/inspector specs).

What exists today:
- Interactive tour: `/collision-dpo-tour.html` (source: `docs/public/collision-dpo-tour.html`)
- Guide entrypoint: `docs/guide/collision-tour.md`
- Diagram assets: `docs/public/assets/collision/`

What this spec should eventually cover:
- Deterministic broad phase + narrow phase modeled as graph rewrites (DPO).
- Canonical IDs, stable ordering, and hashing inputs/outputs for replay.
- Temporal proxies, CCD workflow, and event emission in a timeline-aware world.
- Deterministic math constraints for collision (no platform transcendentals; quantized policies; fixed-point audits).

## Near-Term Deliverables

- Solidify the wire format for collision-related view ops (if any).
- Define the minimal node/edge schema for bodies, shapes, and contacts.
- Specify the canonical ordering for resolving contact sets.

Until the full spec is written, treat the tour as an **illustrative artifact**, not a normative contract.
