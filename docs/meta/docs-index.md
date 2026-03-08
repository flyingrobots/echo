<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Docs Map

This page is a curated map of the docs: a few "golden paths", plus links to the most-used specs.
If you want the full inventory, use repo search (`rg`) and follow links outward from the core specs.

| Document                                                       | Purpose                                                                                                                |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `architecture-outline.md`                                      | High-level architecture vision and principles                                                                          |
| `workflows.md`                                                 | Contributor workflows, policies, and blessed repo entry points                                                         |
| `guide/warp-primer.md`                                         | Start here: newcomer-friendly primer for WARP in Echo                                                                  |
| `guide/wvp-demo.md`                                            | Demo: run the session hub + 2 viewers (publisher/subscriber)                                                           |
| `guide/tumble-tower.md`                                        | Demo 3 scenario: deterministic physics ladder ("Tumble Tower")                                                         |
| `spec-warp-core.md`                                            | WARP core format and runtime                                                                                           |
| `spec-warp-tick-patch.md`                                      | Tick patch boundary artifact (delta ops, in/out slots, patch_digest)                                                   |
| `spec-mwmr-concurrency.md`                                     | WARP MWMR Concurrency Spec (Footprints, Ports, Factor Masks)                                                           |
| `spec-merkle-commit.md`                                        | Snapshot Commit Spec (v2)                                                                                              |
| `spec-temporal-bridge.md`                                      | Cross-branch event lifecycle                                                                                           |
| `warp-two-plane-law.md`                                        | Project law: define SkeletonGraph vs attachment plane, π(U), depth-0 atoms, and "no hidden edges"                      |
| `adr/ADR-0001-warp-two-plane-skeleton-and-attachments.md`      | ADR: formalize two-plane representation (SkeletonGraph + Attachment Plane) and the core invariants                     |
| `adr/ADR-0002-warp-instances-descended-attachments.md`         | ADR: WarpInstances and descended attachments via flattened indirection (no hidden edges, no recursive hot path)        |
| `spec/SPEC-0001-attachment-plane-v0-atoms.md`                  | Spec: attachment plane v0 (typed atoms), codec boundary, and deterministic decode failure semantics                    |
| `spec/SPEC-0002-descended-attachments-v1.md`                   | Spec: descended attachments v1 (WarpInstances, SlotId::Attachment, descent-chain footprint law, worldline slicing)     |
| `spec/SPEC-0004-worldlines-playback-truthbus.md`               | Spec: Worldlines, PlaybackCursor, ViewSession, TruthBus                                                                |
| `architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md` | Canonical terminology: WarpState vs SkeletonGraph, instances/portals, and wormholes (reserved for history compression) |
| `scheduler.md`                                                 | Doc map: warp-core rewrite scheduler vs planned system scheduler                                                       |
| `scheduler-warp-core.md`                                       | Canonical doc: warp-core rewrite scheduler (`reserve()` / drain)                                                       |
| `scheduler-performance-warp-core.md`                           | Canonical doc: warp-core scheduler benchmarks                                                                          |
| `determinism/DETERMINISM_CLAIMS_v0.1.md`                       | Verified determinism claims (DET-001 through DET-005)                                                                  |
| `guide/configuration-reference.md`                             | Engine parameters, protocol constants, environment variables                                                           |
| `guide/cargo-features.md`                                      | Cargo feature flags across the workspace                                                                               |

## Start Here

- Echo (ELI5 spiral on-ramp): [/guide/eli5](/guide/eli5)
- Start Here guide: [/guide/start-here](/guide/start-here)
- WARP primer (newcomer-friendly): [/guide/warp-primer](/guide/warp-primer)
- Architecture overview (draft, but the intent source of truth): [/architecture-outline](/architecture-outline)

## Learn By Doing

- WARP View Protocol demo: [/guide/wvp-demo](/guide/wvp-demo)
- Tumble Tower scenario (deterministic physics ladder): [/guide/tumble-tower](/guide/tumble-tower)

## Core WARP Specs (High Leverage)

- WARP core format + runtime (`warp-core`): [/spec-warp-core](/spec-warp-core)
- Tick patches (delta artifact boundary): [/spec-warp-tick-patch](/spec-warp-tick-patch)
- MWMR Concurrency (footprints, ports): [/spec-mwmr-concurrency](/spec-mwmr-concurrency)
- Merkle commit (snapshot hashing): [/spec-merkle-commit](/spec-merkle-commit)

## Determinism + Math

- Policy (normative): [/SPEC_DETERMINISTIC_MATH](/SPEC_DETERMINISTIC_MATH)
- Hazards + mitigations (background): [/DETERMINISTIC_MATH](/DETERMINISTIC_MATH)
- Current claims / error budgets: [/warp-math-claims](/warp-math-claims)
- Determinism claims: [/determinism/DETERMINISM_CLAIMS_v0.1](/determinism/DETERMINISM_CLAIMS_v0.1)

## Reference / Deep Dives

- Two-plane law ("no hidden edges"): [/warp-two-plane-law](/warp-two-plane-law)
- Warp instances / portals terminology: [/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES](/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES)
- DIND harness: [/dind-harness](/dind-harness)
- Golden vectors (ABI): [/golden-vectors](/golden-vectors)
- JS CBOR mapping: [/js-cbor-mapping](/js-cbor-mapping)
- Dependency DAGs: [/dependency-dags](/dependency-dags)
- Benchmark guide: [/BENCHMARK_GUIDE](/BENCHMARK_GUIDE)
- Release policy: [/RELEASE_POLICY](/RELEASE_POLICY)
- Roadmap: [/ROADMAP](/ROADMAP)

## Procedures

- PR submission + review loop: [/procedures/PR-SUBMISSION-REVIEW-LOOP](/procedures/PR-SUBMISSION-REVIEW-LOOP)
- Issue dependencies: [/procedures/ISSUE-DEPENDENCIES](/procedures/ISSUE-DEPENDENCIES)
- Extract PR comments: [/procedures/EXTRACT-PR-COMMENTS](/procedures/EXTRACT-PR-COMMENTS)

## Vision Specs (Unimplemented)

These specs describe planned features that are not yet implemented. They represent
design intent and are kept for reference, but should not be treated as current behavior.

| Spec                                 | Topic                                                  |
| ------------------------------------ | ------------------------------------------------------ |
| `spec-branch-tree.md`                | Branch tree, diffs, and timeline persistence           |
| `spec-canonical-inbox-sequencing.md` | Canonical inbox sequencing, idempotent ingress         |
| `spec-capabilities-and-security.md`  | Capability tokens and signatures                       |
| `spec-concurrency-and-authoring.md`  | Parallel core + single-threaded scripting model        |
| `spec-ecs-storage.md`                | ECS storage (archetypes, chunks, COW)                  |
| `spec-editor-and-inspector.md`       | Inspector frame protocol + tooling transport           |
| `spec-entropy-and-paradox.md`        | Entropy metrics and paradox handling                   |
| `spec-knots-in-time.md`              | Time knots for Echo                                    |
| `spec-networking.md`                 | Deterministic event replication modes                  |
| `spec-plugin-system.md`              | Plugin discovery, namespace isolation                  |
| `spec-runtime-config.md`             | Deterministic configuration schema                     |
| `spec-scheduler.md`                  | Planned ECS/system scheduler (not warp-core scheduler) |
| `spec-serialization-protocol.md`     | Canonical encoding and hashing                         |
| `spec-time-streams-and-wormholes.md` | Multi-clock time, cursors, wormholes                   |
| `spec-timecube.md`                   | Chronos × Kairos × Aion                                |
| `spec-warp-confluence.md`            | Global WARP graph synchronization                      |
| `spec-warp-view-protocol.md`         | WARP View Protocol (WVP)                               |
| `spec-world-api.md`                  | Stable public façade for external modules              |

## ADRs

| ADR                     | Title                                                      |
| ----------------------- | ---------------------------------------------------------- |
| `adr/ADR-0001-*`        | Two-plane WARP representation                              |
| `adr/ADR-0002-*`        | WarpInstances + descended attachments                      |
| `adr/ADR-0003-*`        | Causality-first API (MaterializationPort)                  |
| `adr/ADR-0004-*`        | No global state (DI only)                                  |
| `adr/ADR-0005-*`        | Physics as deterministic scheduled rewrites                |
| `adr/ADR-0006-*`        | Ban non-determinism                                        |
| `adr/ADR-0007-*`        | Parallel execution storage + scheduling                    |
| `adr/PLAN-PHASE-6B-*`   | Virtual shards (complete)                                  |
| `adr/TECH-DEBT-BOAW.md` | Parallel execution tech debt tracker (historical filename) |

## Archive

Superseded and stale documents live in [`docs/archive/`](../archive/).

See `archive/README.md` for the archive policy. Archived categories include:

- **Session artifacts:** notes, plans, tasks, RFCs, memorials
- **Study materials:** LaTeX papers, tour-de-code booklets, visual atlas
- **Completed missions:** DIND missions, determinism audit, mat-bus RFC
- **Stale docs:** agents, issues matrix, code map, phase 1 plan, demo roadmaps
- **Dead redirects:** collision tour (targets never created)
