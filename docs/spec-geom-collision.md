<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Geometry & Collision (Spec Stub)

**Status: not yet re-specified.** Echo currently includes an interactive DPO tour and diagram assets for collision/CCD, but the full written collision spec is pending.

What exists today:
- Interactive tour: `/collision-dpo-tour.html` (source: `docs/public/collision-dpo-tour.html`)
- Guide entrypoint: `docs/guide/collision-tour.md`
- Diagram assets: `docs/public/assets/collision/`

What this spec should eventually cover:
- Deterministic broad phase + narrow phase modeled as graph rewrites (DPO).
- Canonical IDs, stable ordering, and hashing inputs/outputs for replay.
- Temporal proxies, CCD workflow, and event emission in a timeline-aware world.
- Deterministic math constraints for collision (no platform transcendentals; quantized policy; fixed-point audits).

Until this spec is written, treat the tour as an **illustrative artifact**, not a normative contract.
