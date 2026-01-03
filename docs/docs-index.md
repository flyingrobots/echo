<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Docs Map

This page is a curated map of the docs: a few “golden paths”, plus links to the most-used specs.
If you want the full inventory, use repo search (`rg`) and follow links outward from the core specs.

## Start Here

- Echo (ELI5 spiral on-ramp): [/guide/eli5](/guide/eli5)
- Start Here guide: [/guide/start-here](/guide/start-here)
- WARP primer (newcomer-friendly): [/guide/warp-primer](/guide/warp-primer)
- Architecture overview (draft, but the intent source of truth): [/architecture-outline](/architecture-outline)

## Learn By Doing

- WARP View Protocol demo: [/guide/wvp-demo](/guide/wvp-demo)
- Collision tour: [/guide/collision-tour](/guide/collision-tour)
- Interactive collision DPO tour (static HTML): [/collision-dpo-tour.html](/collision-dpo-tour.html)
- Tumble Tower scenario (deterministic physics ladder): [/guide/tumble-tower](/guide/tumble-tower)

## Core WARP Specs (High Leverage)

- WARP core format + runtime (`warp-core`): [/spec-warp-core](/spec-warp-core)
- Tick patches (delta artifact boundary): [/spec-warp-tick-patch](/spec-warp-tick-patch)
- Serialization protocol (canonical encode + hashing): [/spec-serialization-protocol](/spec-serialization-protocol)
- Branch tree (history + diffs): [/spec-branch-tree](/spec-branch-tree)
- WARP View Protocol (WVP): [/spec-warp-view-protocol](/spec-warp-view-protocol)

## Determinism + Replay

- Determinism invariants (what must never regress): [/determinism-invariants](/determinism-invariants)
- Testing + replay plan: [/testing-and-replay-plan](/testing-and-replay-plan)
- Runtime diagnostics plan: [/runtime-diagnostics-plan](/runtime-diagnostics-plan)
- Hash graph (what depends on what): [/hash-graph](/hash-graph)

## Deterministic Math

- Policy (normative): [/SPEC_DETERMINISTIC_MATH](/SPEC_DETERMINISTIC_MATH)
- Hazards + mitigations (background): [/DETERMINISTIC_MATH](/DETERMINISTIC_MATH)
- Current claims / error budgets: [/warp-math-claims](/warp-math-claims)
- Validation plan (may lag behind implementation): [/math-validation-plan](/math-validation-plan)

## Project Log (Read This Before Starting Big Work)

- Execution plan (living intent, “Today’s Intent”): [/execution-plan](/execution-plan)
- Decision log (chronological record): [/decision-log](/decision-log)

## Reference / Deep Dives

- Two-plane law (“no hidden edges”): [/warp-two-plane-law](/warp-two-plane-law)
- Warp instances / portals terminology: [/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES](/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES)
- Confluence (global sync): [/spec-warp-confluence](/spec-warp-confluence)
- Scheduler spec: [/spec-scheduler](/spec-scheduler)
- Scheduler benchmarks: [/scheduler-benchmarks](/scheduler-benchmarks)
