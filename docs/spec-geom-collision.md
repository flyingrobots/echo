<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Geometry & Collision (Spec Stub)

**Status: not yet re-specified.** This repo currently carries an interactive DPO tour and diagram assets, but the full written spec for Echo’s geometry/collision subsystem is pending re-homing into the Rust-first era.

What exists today:
- Interactive tour: `/collision-dpo-tour.html` (source: `docs/public/collision-dpo-tour.html`)
- Guide entrypoint: `docs/guide/collision-tour.md`
- Diagram assets: `docs/public/assets/collision/`

What this spec should eventually cover:
- Deterministic broad phase + narrow phase modeled as graph rewrites (DPO).
- Canonical IDs, stable ordering, and hashing inputs/outputs for replay.
- Temporal proxies, CCD workflow, and event emission in a timeline-aware world.
- Deterministic math constraints for collision (no platform transcendentals; quantized policies; fixed-point audits).

Until the full spec is written, treat the tour as an **illustrative artifact**, not a normative contract.
