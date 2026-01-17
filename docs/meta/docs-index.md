<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Docs Map

This page is a curated map of the docs: a few “golden paths”, plus links to the most-used specs.
If you want the full inventory, use repo search (`rg`) and follow links outward from the core specs.

| Document | Purpose |
| -------- | ------- |
| `architecture-outline.md` | High-level architecture vision and principles |
| `workflows.md` | Contributor workflows, policies, and blessed repo entry points |
| `guide/warp-primer.md` | Start here: newcomer-friendly primer for WARP in Echo |
| `guide/wvp-demo.md` | Demo: run the session hub + 2 viewers (publisher/subscriber) |
| `guide/tumble-tower.md` | Demo 3 scenario: deterministic physics ladder (“Tumble Tower”) |
| `spec-branch-tree.md` | Branch tree, diffs, and timeline persistence |
| `spec-temporal-bridge.md` | Cross-branch event lifecycle |
| `spec-serialization-protocol.md` | Canonical encoding and hashing |
| `spec-canonical-inbox-sequencing.md` | Canonical inbox sequencing, idempotent ingress, and deterministic tie-breaks |
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
| `WARP-GRAPH.md` | WSC (Write-Streaming Columnar) snapshot format design spec |
| `scheduler.md` | Doc map: warp-core rewrite scheduler vs planned system scheduler |
| `scheduler-warp-core.md` | Canonical doc: warp-core rewrite scheduler (`reserve()` / drain) |
| `scheduler-performance-warp-core.md` | Canonical doc: warp-core scheduler benchmarks |
| `spec-scheduler.md` | Planned ECS/system scheduler spec (not yet implemented) |
| `spec-warp-tick-patch.md` | Tick patch boundary artifact (delta ops, in/out slots, patch_digest) |
| `spec-warp-confluence.md` | Global WARP graph synchronization (Confluence) |
| `spec-ecs-storage.md` | ECS storage (archetypes, chunks, COW) |
| `math-validation-plan.md` | Deterministic math coverage |
| `ISSUES_MATRIX.md` | Table view of active issues, milestones, and relationships |
| `dependency-dags.md` | Visual dependency sketches across issues and milestones (confidence-styled DAGs) |
| `scheduler-benchmarks.md` | Redirect: scheduler benchmark plan split (see `scheduler-performance-warp-core.md`) |
| `scheduler-reserve-validation.md` | Redirect: merged into `scheduler-warp-core.md` |
| `scheduler-reserve-complexity.md` | Redirect: merged into `scheduler-warp-core.md` |
| `testing-and-replay-plan.md` | Replay, golden hashes, entropy tests |
| `runtime-diagnostics-plan.md` | Logging, tracing, inspector streams |
| `meta/docs-audit.md` | Docs hygiene memo: purge/merge/splurge candidates |
| `meta/docs-index.md` | This index |
| `hash-graph.md` | Hash relationships across subsystems |
| `meta/legacy-excavation.md` | Historical artifact log |
| `memorial.md` | Tribute to Caverns |
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

## Reference / Deep Dives

- Two-plane law (“no hidden edges”): [/warp-two-plane-law](/warp-two-plane-law)
- Warp instances / portals terminology: [/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES](/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES)
- Confluence (global sync): [/spec-warp-confluence](/spec-warp-confluence)
- Scheduler spec: [/spec-scheduler](/spec-scheduler)
- Scheduler benchmarks: [/scheduler-benchmarks](/scheduler-benchmarks)

## Orphaned Docs (Linked)

These docs had zero or one inbound references from other docs. They are linked here to keep the map complete.

| Document | Purpose |
| --- | --- |
| [`BENCHMARK_GUIDE.md`](/BENCHMARK_GUIDE) | How to Add Benchmarks to Echo |
| [`ISSUES_MATRIX.md`](/ISSUES_MATRIX) | Echo Issues Matrix (Active Plan) |
| [`METHODOLOGY.md`](/METHODOLOGY) | JITOS Engineering Standard: The Living Specification |
| [`ROADMAP.md`](/ROADMAP) | Echo Roadmap (Milestones + Issue Map) |
| [`THEORY.md`](/THEORY) | Echo: Theoretical Foundations |
| [`adr/ADR-0001-warp-two-plane-skeleton-and-attachments.md`](/adr/ADR-0001-warp-two-plane-skeleton-and-attachments) | ADR-0001: Two-plane WARP representation in Echo (SkeletonGraph + Attachment Plane) |
| [`adr/ADR-0002-warp-instances-descended-attachments.md`](/adr/ADR-0002-warp-instances-descended-attachments) | ADR-0002: WarpInstances + Descended Attachments via Flattened Indirection |
| [`adr/ADR-0003-Materialization-Bus.md`](/adr/ADR-0003-Materialization-Bus) | ADR-000X: Causality-First API — Ingress + MaterializationPort, No Direct Graph Writes |
| [`adr/ADR-0004-No-Global-State.md`](/adr/ADR-0004-No-Global-State) | ADR-000Y: No Global State in Echo — Dependency Injection Only |
| [`adr/ADR-0005-Physics.md`](/adr/ADR-0005-Physics) | ADR-0005: Physics as Deterministic Scheduled Rewrites (Footprints + Phases) |
| [`adr/ADR-0006-Ban-Non-Determinism.md`](/adr/ADR-0006-Ban-Non-Determinism) | T2000 on 'em |
| [`aion-papers-bridge.md`](/aion-papers-bridge) | AIΩN Foundations → Echo: Bridge |
| [`architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md`](/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES) | Terms: WARP State, SkeletonGraph, Instances, Portals, Wormholes |
| [`benchmarks/RESERVE_BENCHMARK.md`](/benchmarks/RESERVE_BENCHMARK) | Reserve Independence Benchmark |
| [`branch-merge-playbook.md`](/branch-merge-playbook) | Branch Merge Conflict Playbook |
| [`capability-ownership-matrix.md`](/capability-ownership-matrix) | Capability Ownership Matrix |
| [`code-map.md`](/code-map) | Echo Code Map |
| [`dependency-dags.md`](/dependency-dags) | Dependency DAGs (Issues + Milestones) |
| [`determinism-invariants.md`](/determinism-invariants) | Determinism Invariants |
| [`diagrams.md`](/diagrams) | Echo Diagram Vault |
| [`dind-harness.md`](/dind-harness) | DIND Harness (Deterministic Ironclad Nightmare Drills) |
| [`meta/docs-audit.md`](/meta/docs-audit) | Docs Audit — Purge / Merge / Splurge |
| [`golden-vectors.md`](/golden-vectors) | ABI Golden Vectors (v1) |
| [`guide/course/00-orientation.md`](/guide/course/00-orientation) | 00 — Orientation: Shared Truth, Not Vibes |
| [`guide/course/01-lockstep.md`](/guide/course/01-lockstep) | 01 — Lockstep, Explained (Inputs‑Only Networking) |
| [`guide/course/README.md`](/guide/course/README) | Course Notes (Authoring) |
| [`guide/course/glossary.md`](/guide/course/glossary) | Course Glossary (Progressive Vocabulary) |
| [`guide/course/index.md`](/guide/course/index) | Echo Course: Networking‑First (Build “Splash Guy”) |
| [`guide/tumble-tower.md`](/guide/tumble-tower) | Demo 3 Scenario: “Tumble Tower” (Deterministic Physics) |
| [`hash-graph.md`](/hash-graph) | Hash Graph Overview |
| [`index.md`](/index) | Echo |
| [`jitos/spec-0000.md`](/jitos/spec-0000) | SPEC-000: Everything Is a Rewrite |
| [`js-cbor-mapping.md`](/js-cbor-mapping) | JS → Canonical CBOR Mapping Rules (ABI v1) |
| [`meta/legacy-excavation.md`](/meta/legacy-excavation) | Legacy Excavation Log (Placeholder) |
| [`notes/aion-papers-bridge.md`](/notes/aion-papers-bridge) | Moved: AIΩN Foundations → Echo Bridge |
| [`notes/f32scalar-deterministic-trig-implementation-guide.md`](/notes/f32scalar-deterministic-trig-implementation-guide) | Implementation Guide — Deterministic `sin/cos` for `F32Scalar` (LUT-backed) |
| [`notes/project-tour-2025-12-28.md`](/notes/project-tour-2025-12-28) | Echo Project Tour (2025-12-28) |
| [`notes/scheduler-optimization-followups.md`](/notes/scheduler-optimization-followups) | Scheduler Optimization Follow-up Tasks |
| [`notes/scheduler-radix-optimization-2.md`](/notes/scheduler-radix-optimization-2) | From $O(n \\log n)$ to $O(n)$: Optimizing Echo’s Deterministic Scheduler |
| [`notes/scheduler-radix-optimization.md`](/notes/scheduler-radix-optimization) | From $O(n log n)$ to $O(n)$: Optimizing Echo's Deterministic Scheduler |
| [`notes/xtask-wizard.md`](/notes/xtask-wizard) | xtask "workday wizard" — concept note |
| [`phase1-plan.md`](/phase1-plan) | Phase 1 – Core Ignition Plan |
| [`procedures/EXTRACT-PR-COMMENTS.md`](/procedures/EXTRACT-PR-COMMENTS) | Procedure: Extract Actionable Comments from PR Review Threads (CodeRabbitAI + Humans) |
| [`procedures/ISSUE-DEPENDENCIES.md`](/procedures/ISSUE-DEPENDENCIES) | Procedure: GitHub Issue Dependencies (“blocked by” / “blocking”) |
| [`procedures/PR-SUBMISSION-REVIEW-LOOP.md`](/procedures/PR-SUBMISSION-REVIEW-LOOP) | Procedure: PR Submission + CodeRabbitAI Review Loop |
| [`public/assets/collision/README.md`](public/assets/collision/README.md) | Collision/CCD DPO Diagrams |
| [`release-criteria.md`](/release-criteria) | Release Criteria — Phase 0.5 → Phase 1 |
| [`roadmap-mwmr-mini-epic.md`](/roadmap-mwmr-mini-epic) | MWMR Concurrency Mini‑Epic Roadmap (Footprints, Reserve Gate, Telemetry) |
| [`runtime-diagnostics-plan.md`](/runtime-diagnostics-plan) | Runtime Diagnostics Plan (Phase 0.5) |
| [`rust-rhai-ts-division.md`](/rust-rhai-ts-division) | Language & Responsibility Map (Phase 1) |
| [`scheduler-benchmarks.md`](/scheduler-benchmarks) | Scheduler Benchmark Plan (Phase 0) |
| [`scheduler-performance-warp-core.md`](/scheduler-performance-warp-core) | Scheduler Performance (warp-core) |
| [`scheduler-reserve-complexity.md`](/scheduler-reserve-complexity) | Scheduler `reserve()` Time Complexity Analysis |
| [`scheduler-reserve-validation.md`](/scheduler-reserve-validation) | Scheduler `reserve()` Implementation Validation |
| [`scheduler-warp-core.md`](/scheduler-warp-core) | WARP Rewrite Scheduler (warp-core) |
| [`scheduler.md`](/scheduler) | Scheduling in Echo (Doc Map) |
| [`spec-branch-tree.md`](/spec-branch-tree) | Branch Tree Persistence Specification (Phase 0) |
| [`spec-canonical-inbox-sequencing.md`](/spec-canonical-inbox-sequencing) | Spec: Canonical Inbox Sequencing + Deterministic Scheduler Tie-Break |
| [`spec-capabilities-and-security.md`](/spec-capabilities-and-security) | Capabilities & Security Specification (Phase 0.5) |
| [`spec-concurrency-and-authoring.md`](/spec-concurrency-and-authoring) | Concurrency & Authoring Specification (Phase 0.75) |
| [`spec-deterministic-math.md`](/spec-deterministic-math) | Deterministic Math Module Specification (Phase 0) |
| [`spec-ecs-storage.md`](/spec-ecs-storage) | Echo ECS Storage Blueprint (Phase 0) |
| [`spec-editor-and-inspector.md`](/spec-editor-and-inspector) | Inspector & Editor Protocol Specification (Phase 0.75) |
| [`spec-entropy-and-paradox.md`](/spec-entropy-and-paradox) | Entropy & Paradox Specification (Phase 0.75) |
| [`spec-geom-collision.md`](/spec-geom-collision) | Geometry & Collision (Spec Stub) |
| [`spec-knots-in-time.md`](/spec-knots-in-time) | Knots In (and Over) Graphs — Time Knots for Echo |
| [`spec-merkle-commit.md`](/spec-merkle-commit) | Snapshot Commit Spec (v2) |
| [`spec-mwmr-concurrency.md`](/spec-mwmr-concurrency) | WARP MWMR Concurrency Spec (Footprints, Ports, Factor Masks) |
| [`spec-networking.md`](/spec-networking) | Networking Specification (Phase 0.75) |
| [`spec-plugin-system.md`](/spec-plugin-system) | Plugin System Specification (Phase 0.75) |
| [`spec-runtime-config.md`](/spec-runtime-config) | Runtime Configuration Specification (Phase 0.75) |
| [`spec-temporal-bridge.md`](/spec-temporal-bridge) | Temporal Bridge Specification (Phase 0.5) |
| [`spec-time-streams-and-wormholes.md`](/spec-time-streams-and-wormholes) | TimeStreams, Cursors, and Wormholes (Multi-Clock Time for Echo) |
| [`spec-timecube.md`](/spec-timecube) | TimeCube: Chronos × Kairos × Aion |
| [`spec-warp-confluence.md`](/spec-warp-confluence) | WARP Confluence Specification (Phase 0.75) |
| [`spec-warp-tick-patch.md`](/spec-warp-tick-patch) | WARP Tick Patch Spec (v2) |
| [`spec-warp-view-protocol.md`](/spec-warp-view-protocol) | WARP View Protocol (WVP) |
| [`spec-world-api.md`](/spec-world-api) | World API Specification (Phase 0.5) |
| [`spec/SPEC-0001-attachment-plane-v0-atoms.md`](/spec/SPEC-0001-attachment-plane-v0-atoms) | SPEC-0001: Attachment Plane v0 — Typed Atoms (Depth-0) |
| [`spec/SPEC-0002-descended-attachments-v1.md`](/spec/SPEC-0002-descended-attachments-v1) | SPEC-0002: Descended Attachments v1 — WarpInstances + Flattened Indirection |
| [`spec/SPEC-0003-dpo-concurrency-litmus-v0.md`](/spec/SPEC-0003-dpo-concurrency-litmus-v0) | SPEC-0003: DPO Concurrency Litmus (v0) |
| [`tasks.md`](/tasks) | WARP View Protocol Tasks |
| [`tasks/issue-canonical-f32.md`](/tasks/issue-canonical-f32) | Title: feat(warp-core): Implement strict determinism for F32Scalar (NaNs, Subnormals) |
| [`telemetry-graph-replay.md`](/telemetry-graph-replay) | Telemetry: Graph Snapshot for Repro/Replay (Design Note) |
| [`testing-and-replay-plan.md`](/testing-and-replay-plan) | Testing & Replay Plan (Phase 0.5) |
| [`two-lane-abi.md`](/two-lane-abi) | Two-Lane ABI Design (Control Plane vs. Data Plane) |
| [`warp-demo-roadmap.md`](/warp-demo-roadmap) | WARP Demo Roadmap (Phase 1 Targets) |
| [`warp-runtime-architecture.md`](/warp-runtime-architecture) | WARP Runtime Architecture (Phase 1 Blueprint) |
| [`workflows.md`](/workflows) | Workflows (Contributor Playbook) |
