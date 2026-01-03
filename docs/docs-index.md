<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Documentation Index

| Document | Purpose |
| -------- | ------- |
| `architecture-outline.md` | High-level architecture vision and principles |
| `execution-plan.md` | Living plan of tasks, intent, and progress |
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

## Getting Started
1. Read `guide/warp-primer.md` (if you’re new to WARP / `warp-core`).
2. Read `architecture-outline.md`.
3. Review `spec-branch-tree.md` + `spec-codex-baby.md` + `spec-temporal-bridge.md`.
4. Consult `execution-plan.md` for current focus.

## Phase Tags
- Phase 0.0 — initial skeleton
- Phase 0.5 — causality & determinism layer (current)
- Phase 1.0 — implementation kickoff
