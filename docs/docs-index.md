<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Docs Map

This page is a curated map of the docs: a few “golden paths”, plus links to the most-used specs.
If you want the full inventory, use repo search (`rg`) and follow links outward from the core specs.
| Document | Purpose |
| -------- | ------- |
| `architecture-outline.md` | High-level architecture vision and principles |
| `execution-plan.md` | Living plan of tasks, intent, and progress |
| `workflows.md` | Contributor workflows, policies, and blessed repo entry points |
| `guide/warp-primer.md` | Start here: newcomer-friendly primer for WARP in Echo |
| `guide/wvp-demo.md` | Demo: run the session hub + 2 viewers (publisher/subscriber) |
| `guide/tumble-tower.md` | Demo 3 scenario: deterministic physics ladder (“Tumble Tower”) |
| `spec-branch-tree.md` | Branch tree, diffs, and timeline persistence |
| `spec-codex-baby.md` | Event bus, bridges, backpressure, security |
| `spec-temporal-bridge.md` | Cross-branch event lifecycle |
| `spec-serialization-protocol.md` | Canonical encoding and hashing |
| `spec-capabilities-and-security.md` | Capability tokens and signatures |
| `spec-world-api.md` | Stable public façade for external modules |
| `spec-entropy-and-paradox.md` | Entropy metrics and paradox handling |
| `spec-editor-and-inspector.md` | Inspector frame protocol & tooling transport |
| `spec-runtime-config.md` | Deterministic configuration schema and hashing |
| `spec-plugin-system.md` | Plugin discovery, namespace isolation, capabilities |
| `spec-concurrency-and-authoring.md` | Parallel core & single-threaded scripting model |
| `spec-networking.md` | Deterministic event replication modes |
| `spec-time-streams-and-wormholes.md` | Multi-clock time as event streams (cursors + admission policies) and wormholes/checkpoints for fast catch-up/seek |
| `capability-ownership-matrix.md` | Ownership matrix across layers (determinism/provenance expectations per capability) |
| `aion-papers-bridge.md` | Map AIΩN Foundations (WARP papers) onto Echo’s backlog and document deviations |
| `warp-two-plane-law.md` | Project law: define SkeletonGraph vs attachment plane, π(U), depth-0 atoms, and “no hidden edges” |
| `adr/ADR-0001-warp-two-plane-skeleton-and-attachments.md` | ADR: formalize two-plane representation (SkeletonGraph + Attachment Plane) and the core invariants |
| `adr/ADR-0002-warp-instances-descended-attachments.md` | ADR: WarpInstances and descended attachments via flattened indirection (no hidden edges, no recursive hot path) |
| `spec/SPEC-0001-attachment-plane-v0-atoms.md` | Spec: attachment plane v0 (typed atoms), codec boundary, and deterministic decode failure semantics |
| `spec/SPEC-0002-descended-attachments-v1.md` | Spec: descended attachments v1 (WarpInstances, SlotId::Attachment, descent-chain footprint law, worldline slicing) |
| `architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md` | Canonical terminology: WarpState vs SkeletonGraph, instances/portals, and wormholes (reserved for history compression) |
| `phase1-plan.md` | Phase 1 implementation roadmap & demo targets |
| `spec-warp-core.md` | WARP core format and runtime |
| `spec-warp-tick-patch.md` | Tick patch boundary artifact (delta ops, in/out slots, patch_digest) |
| `spec-warp-confluence.md` | Global WARP graph synchronization (Confluence) |
| `spec-ecs-storage.md` | ECS storage (archetypes, chunks, COW) |
| `math-validation-plan.md` | Deterministic math coverage |
| `ISSUES_MATRIX.md` | Table view of active issues, milestones, and relationships |
| `dependency-dags.md` | Visual dependency sketches across issues and milestones (confidence-styled DAGs) |
| `scheduler-benchmarks.md` | Scheduler performance scenarios |
| `testing-and-replay-plan.md` | Replay, golden hashes, entropy tests |
| `runtime-diagnostics-plan.md` | Logging, tracing, inspector streams |
| `codex-instrumentation.md` | CB metrics and telemetry hooks |
| `docs-audit.md` | Docs hygiene memo: purge/merge/splurge candidates |
| `docs-index.md` | This index |
| `hash-graph.md` | Hash relationships across subsystems |
| `legacy-excavation.md` | Historical artifact log |
| `memorial.md` | Tribute to Caverns |
| `decision-log.md` | Chronological design decisions |
| `release-criteria.md` | Phase transition checklist |

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
