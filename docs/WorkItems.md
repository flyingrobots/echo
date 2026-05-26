<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Work Items

Last audited: 2026-05-25.

This is an inventory, not a replacement for the repo's planning system. When
there is disagreement, prefer the specific backlog card, design packet, issue,
or executable test over this summary.

Sources checked during this audit:

- [`docs/BEARING.md`](BEARING.md)
- [`docs/method/backlog/`](method/backlog/)
- [`backlog/`](../backlog/)
- open GitHub issues in `flyingrobots/echo`

## Summary

| Source                            | Open count | Notes                                      |
| --------------------------------- | ---------: | ------------------------------------------ |
| `docs/method/backlog/asap/`       |         13 | Immediate filesystem backlog.              |
| `docs/method/backlog/v0.1.0/`     |         21 | Release-bar lane.                          |
| `docs/method/backlog/up-next/`    |         39 | Planned follow-on work.                    |
| `docs/method/backlog/inbox/`      |         16 | Triage queue and older issue mirrors.      |
| `docs/method/backlog/bad-code/`   |          3 | Known local structural debt.               |
| `docs/method/backlog/cool-ideas/` |         29 | Deliberately not current release work.     |
| `backlog/bad-code/`               |          5 | Older RE-series debt cards still present.  |
| `backlog/cool-ideas/`             |          2 | Older CI-series idea cards still present.  |
| GitHub open issues                |         46 | After closing completed `#281` and `#285`. |

## Current Execution Gravity

The active direction remains:

```text
prove Echo with jedit as a real external app
without moving app nouns into Echo
and without giving application code tick, WAL, or trusted runtime authority
```

Current active signposts:

- Echo's release feature bar:
  [`docs/design/v0.1.0-release-plan.md`](design/v0.1.0-release-plan.md)
- sequencing and prioritization filter:
  [`docs/design/work-item-sequencing-and-prioritization.md`](design/work-item-sequencing-and-prioritization.md)
- jedit external release gate:
  [`docs/design/v0.1.0-jedit-release-gate.md`](design/v0.1.0-jedit-release-gate.md)
- next ten jedit/Echo release-gate slices:
  [`docs/design/v0.1.0-jedit-next-ten-slices.md`](design/v0.1.0-jedit-next-ten-slices.md)
- causal WAL doctrine:
  [`docs/design/causal-wal-end-to-end.md`](design/causal-wal-end-to-end.md)

Progress bars from the current work stream:

```text
[##########] Echo/jedit retained-evidence release-gate batch [10/10 slices]
[##########] PR checkpoint batch [10/10 slices before next PR]
[##########] Echo WAL truth boundary and runtime ACK plumbing [95/95 slices]
```

Current batch status: complete; open paired Echo and jedit PRs before starting
the next implementation batch.

## Known Cross-Repo And Storage Doctrine Gaps

This inventory is Echo-local unless a row explicitly names another repository.
The following mission-critical gaps were not fully represented by the current
backlog lanes when this audit started:

- `[Echo][jedit]` WSC causal-history persistence, export, and recovery.
- `[Echo]` WAL/WSC storage relationship and recovery authority.
- `[warp-ttd][Echo]` WAL-backed causal commit evidence read model.
- `[Echo]` JS/WASM/browser client release surface.
- `[Echo][Wesley]` package publish, generated package compatibility, and
  versioning.
- `[Echo][Graft][Think]` post-jedit application portability checklist.
- `[Echo]` retained evidence posture versus durable recovery evidence.

The Echo-owned follow-up cards are now in the `v0.1.0` lane:

- [WAL/WSC Storage Relationship](method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md)
- [WSC Causal-History Storage](method/backlog/v0.1.0/PLATFORM_wsc-causal-history-storage.md)
- [Retained Evidence Durability Boundary](method/backlog/v0.1.0/PLATFORM_retained-evidence-durability-boundary.md)
- [JS/WASM/Browser Client Release Surface](method/backlog/v0.1.0/PLATFORM_js-wasm-browser-client-release-surface.md)
- [Package Publish And Versioning](method/backlog/v0.1.0/RELEASE_package-publish-and-versioning.md)

## ASAP Backlog

- [Docs cleanup](method/backlog/asap/DOCS_docs-cleanup.md)
- [Echo and git-warp compatibility sanity check](method/backlog/asap/KERNEL_echo-git-warp-compatibility-sanity-check.md)
- [Deterministic Trig Oracle](method/backlog/asap/MATH_deterministic-trig.md)
- [CI det-policy hardening](method/backlog/asap/PLATFORM_ci-det-policy-hardening.md)
- [CLI Scaffold (#47)](method/backlog/asap/PLATFORM_cli-scaffold.md)
- [Contract-Hosted File History Substrate](method/backlog/asap/PLATFORM_contract-hosted-file-history-substrate.md)
- [Contract QueryView Observer Bridge](method/backlog/asap/PLATFORM_contract-queryview-observer-bridge.md)
- [Explicit negative test mapping for decoder controls](method/backlog/asap/PLATFORM_decoder-negative-test-map.md)
- [Echo Contract Hosting Roadmap](method/backlog/asap/PLATFORM_echo-contract-hosting-roadmap.md)
- [Installed Wesley Contract Host Dispatch](method/backlog/asap/PLATFORM_installed-wesley-contract-host-dispatch.md)
- [Commit-ordered rollback playbooks for TTD integration](method/backlog/asap/PLATFORM_ttd-rollback-playbooks.md)
- [Reconcile TTD protocol schemas with warp-ttd](method/backlog/asap/PLATFORM_ttd-schema-reconciliation.md)
- [Wesley Compiled Contract Hosting Doctrine](method/backlog/asap/PLATFORM_wesley-compiled-contract-hosting-doctrine.md)

## v0.1.0 Lane

- [Release-Grade Quickstart](method/backlog/v0.1.0/DOCS_release-grade-quickstart.md)
- [Contract-Aware Receipts And Readings](method/backlog/v0.1.0/KERNEL_contract-aware-receipts-and-readings.md)
- [Contract Obstruction Taxonomy](method/backlog/v0.1.0/KERNEL_contract-obstruction-taxonomy.md)
- [Contract Reading Identity And Bounded Payloads](method/backlog/v0.1.0/KERNEL_contract-reading-identity-and-bounded-payloads.md)
- [Witnessed Intent Submission Persistence](method/backlog/v0.1.0/KERNEL_witnessed-intent-submission-persistence.md)
- [App-Safe Client Surface](method/backlog/v0.1.0/PLATFORM_app-safe-client-surface.md)
- [Contract Artifact Retention In echo-cas](method/backlog/v0.1.0/PLATFORM_contract-artifact-retention-in-echo-cas.md)
- [Contract Retention And Semantic Lookup Seams](method/backlog/v0.1.0/PLATFORM_contract-retention-and-semantic-lookup-seams.md)
- [External Contract Proof Fixture](method/backlog/v0.1.0/PLATFORM_external-contract-proof-fixture.md)
- [JS/WASM/Browser Client Release Surface](method/backlog/v0.1.0/PLATFORM_js-wasm-browser-client-release-surface.md)
- [jedit Real Echo Release Gate](method/backlog/v0.1.0/PLATFORM_jedit-real-echo-release-gate.md)
- [Package Publish And Versioning](method/backlog/v0.1.0/RELEASE_package-publish-and-versioning.md)
- [Product-Facing Intent Outcome API](method/backlog/v0.1.0/PLATFORM_product-facing-intent-outcome-api.md)
- [Reference Trusted Runtime Host Loop](method/backlog/v0.1.0/PLATFORM_reference-trusted-runtime-host-loop.md)
- [Retained Evidence Durability Boundary](method/backlog/v0.1.0/PLATFORM_retained-evidence-durability-boundary.md)
- [Versioned Contract And API Compatibility](method/backlog/v0.1.0/PLATFORM_versioned-contract-api-compatibility.md)
- [WAL/WSC Storage Relationship](method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md)
- [WSC Causal-History Storage](method/backlog/v0.1.0/PLATFORM_wsc-causal-history-storage.md)
- [v0.1.0 Release Candidate](method/backlog/v0.1.0/RELEASE_v0.1.0-release-candidate.md)
- [Authority Boundary Audit](method/backlog/v0.1.0/SECURITY_authority-boundary-audit.md)
- [v0.1.0 Replay And DIND Proof](method/backlog/v0.1.0/TEST_v0.1.0-replay-dind-proof.md)

## Up Next

- [KERNEL - Admission Outcome Family](method/backlog/up-next/KERNEL_admission-outcome-family.md)
- [KERNEL - Bounded Site and Admission Policy](method/backlog/up-next/KERNEL_bounded-site-and-admission-policy.md)
- [KERNEL - Braid and Settlement Admission Unification](method/backlog/up-next/KERNEL_braid-settlement-admission-unification.md)
- [Compliance reporting as a TTD protocol extension](method/backlog/up-next/KERNEL_compliance-protocol-envelope.md)
- [Contract Inverse Admission Hook](method/backlog/up-next/KERNEL_contract-inverse-admission-hook.md)
- [Contract Strands And Counterfactuals](method/backlog/up-next/KERNEL_contract-strands-and-counterfactuals.md)
- [KERNEL - Determinism escape hatches audit and closure](method/backlog/up-next/KERNEL_determinism-escape-hatches.md)
- [Dynamic Footprint Binding Runtime](method/backlog/up-next/KERNEL_dynamic-footprint-binding-runtime.md)
- [Generic Contract Braid Substrate](method/backlog/up-next/KERNEL_generic-contract-braid-substrate.md)
- [Intent-Only Contract Runtime Mutations](method/backlog/up-next/KERNEL_intent-only-contract-runtime-mutations.md)
- [SHA-256 to BLAKE3 Coordination](method/backlog/up-next/KERNEL_sha256-blake3.md)
- [Strand Runtime Graph Ontology](method/backlog/up-next/KERNEL_strand-runtime-graph-ontology.md)
- [Security/capabilities for fork/rewind/merge](method/backlog/up-next/KERNEL_time-travel-capabilities.md)
- [WARP optic boundary audit for topology and history operations](method/backlog/up-next/KERNEL_topology-mutation-intent-boundary-audit.md)
- [Authenticated Wesley Intent Admission Posture](method/backlog/up-next/PLATFORM_authenticated-wesley-intent-admission-posture.md)
- [Braid and settlement Intent paths](method/backlog/up-next/PLATFORM_braid-settlement-intent-paths.md)
- [In-Browser Visualization](method/backlog/up-next/PLATFORM_browser-visualization.md)
- [PLATFORM - Continuum admission family cutover](method/backlog/up-next/PLATFORM_continuum-admission-family-cutover.md)
- [Continuum Proof Family Runtime Cutover](method/backlog/up-next/PLATFORM_continuum-proof-family-runtime-cutover.md)
- [Add an explicit Echo CLI and MCP agent surface](method/backlog/up-next/PLATFORM_echo-agent-surface-cli-and-mcp.md)
- [echo-cas JS Bindings](method/backlog/up-next/PLATFORM_echo-cas-js-bindings.md)
- [Echo / git-warp witnessed suffix sync](method/backlog/up-next/PLATFORM_echo-git-warp-witnessed-suffix-sync.md)
- [Split echo-session-proto into retained bridge contracts vs legacy transport residue](method/backlog/up-next/PLATFORM_echo-session-proto-split.md)
- [Footprint Honesty Rewrite Proof Slice](method/backlog/up-next/PLATFORM_footprint-honesty-rewrite-proof-slice.md)
- [Graft Live Frontier Structural Readings](method/backlog/up-next/PLATFORM_graft-live-frontier-structural-readings.md)
- [Import outcome idempotence and loop law](method/backlog/up-next/PLATFORM_import-outcome-idempotence-and-loop-law.md)
- [Import outcome retention and novelty index](method/backlog/up-next/PLATFORM_import-outcome-retention-novelty-index.md)
- [Inverse operation Intent path](method/backlog/up-next/PLATFORM_inverse-operation-intent-path.md)
- [jedit Optic Intent / Observation Handoff](method/backlog/up-next/PLATFORM_jedit-hot-text-runtime-host-surface.md)
- [jedit Text Contract Hosting MVP](method/backlog/up-next/PLATFORM_jedit-text-contract-mvp.md)
- [Triage METHOD drift against ~/git/method](method/backlog/up-next/PLATFORM_method-sync-and-doctor-triage.md)
- [PLATFORM - Neighborhood publication stack documentation](method/backlog/up-next/PLATFORM_neighborhood-publication-stack.md)
- [Strand and support Intent paths](method/backlog/up-next/PLATFORM_strand-and-support-intent-paths.md)
- [WASM Runtime Integration](method/backlog/up-next/PLATFORM_wasm-runtime.md)
- [Wesley Footprint Honesty Artifact Attestation](method/backlog/up-next/PLATFORM_wesley-footprint-honesty-artifact-attestation.md)
- [Wesley Go Public](method/backlog/up-next/PLATFORM_wesley-go-public.md)
- [Wesley Migration Planning Phase B](method/backlog/up-next/PLATFORM_wesley-migration.md)
- [Wesley QIR Phase C](method/backlog/up-next/PLATFORM_wesley-qir-phase-c.md)
- [Wesley Type Pipeline in Browser](method/backlog/up-next/PLATFORM_wesley-type-pipeline-browser.md)

## Inbox

- [Wesley Docs](method/backlog/inbox/DOCS_wesley-docs.md)
- [Deterministic Rhai](method/backlog/inbox/KERNEL_deterministic-rhai.md)
- [First-class invariant documents](method/backlog/inbox/KERNEL_invariants-as-docs.md)
- [Security](method/backlog/inbox/KERNEL_security.md)
- [ABI nested evidence strictness](method/backlog/inbox/PLATFORM_abi-nested-evidence-strictness.md)
- [Editor Hot-Reload](method/backlog/inbox/PLATFORM_editor-hot-reload.md)
- [git-mind NEXUS](method/backlog/inbox/PLATFORM_git-mind-nexus.md)
- [Importer](method/backlog/inbox/PLATFORM_importer.md)
- [Legend progress in method status](method/backlog/inbox/PLATFORM_method-status-legend-progress.md)
- [Reconcile Relocated Wesley Echo Schemas](method/backlog/inbox/PLATFORM_reconcile-relocated-wesley-echo-schemas.md)
- [Runtime-Owned Footprint Directive Migration](method/backlog/inbox/PLATFORM_runtime-owned-footprint-directive-migration.md)
- [Signing Pipeline](method/backlog/inbox/PLATFORM_signing-pipeline.md)
- [Tooling & Misc](method/backlog/inbox/PLATFORM_tooling-misc.md)
- [TTD Hardening & Future](method/backlog/inbox/PLATFORM_ttd-hardening.md)
- [Wesley Boundary Grammar](method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md)
- [Wesley Future](method/backlog/inbox/PLATFORM_wesley-future.md)

## Known Bad Code

- [RED/GREEN can't be separate commits](method/backlog/bad-code/red-green-lint-friction.md)
- [WASM control intent authority boundary is too implicit](method/backlog/bad-code/wasm-control-intent-authority-boundary.md)
- [xtask main.rs is a god file](method/backlog/bad-code/xtask-god-file.md)
- [RE-028 — Merkle-Tree Memoization in Snapshot Accumulator](../backlog/bad-code/RE-028-snapshot-accumulator-memoization.md)
- [RE-029 — Enforce det_fixed by Default](../backlog/bad-code/RE-029-concurrent-snapshot-fetching.md)
- [RE-030 — Converge QueryView Reads onto Optics](../backlog/bad-code/RE-030-queryview-optic-convergence.md)
- [RE-031 Capability Grant Validation Admission Integration](../backlog/bad-code/RE-031-capability-grant-validation-admission-integration.md)
- [RE-032: Publish Durable Scheduler Fault Evidence](../backlog/bad-code/RE-032-durable-scheduler-fault-evidence.md)

## Cool Ideas

- [Enforce Echo design vocabulary](method/backlog/cool-ideas/DOCS_glossary-enforcement.md)
- [Course Material](method/backlog/cool-ideas/DOCS_splash-guy-course-material.md)
- [Course Material](method/backlog/cool-ideas/DOCS_tumble-tower-course-material.md)
- [Expose parallel execution counterfactuals](method/backlog/cool-ideas/KERNEL_parallel-execution-counterfactuals.md)
- [TT3 — Rulial Diff](method/backlog/cool-ideas/KERNEL_rulial-diff.md)
- [Controlled Desync](method/backlog/cool-ideas/KERNEL_splash-guy-controlled-desync.md)
- [Lockstep Protocol](method/backlog/cool-ideas/KERNEL_splash-guy-lockstep-protocol.md)
- [Rules & State Model](method/backlog/cool-ideas/KERNEL_splash-guy-rules-and-state.md)
- [TT2 — Time Travel MVP](method/backlog/cool-ideas/KERNEL_time-travel-mvp.md)
- [Desync Breakers](method/backlog/cool-ideas/KERNEL_tumble-tower-desync-breakers.md)
- [Lockstep Harness](method/backlog/cool-ideas/KERNEL_tumble-tower-lockstep-harness.md)
- [Worldline Convergence Suite](method/backlog/cool-ideas/KERNEL_worldline-convergence.md)
- [Stage 0: AABB](method/backlog/cool-ideas/MATH_tumble-tower-stage-0-aabb.md)
- [Stage 1: Rotation](method/backlog/cool-ideas/MATH_tumble-tower-stage-1-rotation.md)
- [Stage 2: Friction](method/backlog/cool-ideas/MATH_tumble-tower-stage-2-friction.md)
- [Stage 3: Sleeping](method/backlog/cool-ideas/MATH_tumble-tower-stage-3-sleeping.md)
- [Continuum Contract Artifact Interchange](method/backlog/cool-ideas/PLATFORM_continuum-contract-artifact-interchange.md)
- [Cross-repo METHOD dashboard](method/backlog/cool-ideas/PLATFORM_cross-repo-method-dashboard.md)
- [API Evolution](method/backlog/cool-ideas/PLATFORM_deep-storage-api-evolution.md)
- [DiskTier](method/backlog/cool-ideas/PLATFORM_deep-storage-disk-tier.md)
- [GC Sweep & Eviction](method/backlog/cool-ideas/PLATFORM_deep-storage-gc-sweep-eviction.md)
- [Wire Protocol](method/backlog/cool-ideas/PLATFORM_deep-storage-wire-protocol.md)
- [Extract method crate to its own repo](method/backlog/cool-ideas/PLATFORM_method-crate-extract.md)
- [Method drift check as pre-push hook](method/backlog/cool-ideas/PLATFORM_method-drift-as-pre-push-hook.md)
- [Proof-Carrying Apertures](method/backlog/cool-ideas/PLATFORM_proof-carrying-apertures.md)
- [Reading envelope inspector](method/backlog/cool-ideas/PLATFORM_reading-envelope-inspector.md)
- [Visualization](method/backlog/cool-ideas/PLATFORM_splash-guy-visualization.md)
- [Visualization](method/backlog/cool-ideas/PLATFORM_tumble-tower-visualization.md)
- [WARPDrive POSIX Materialization Optic](method/backlog/cool-ideas/PLATFORM_warpdrive-posix-optic.md)
- [CI-001 — Causal "Multiverse" Puzzle Engine](../backlog/cool-ideas/CI-001-causal-puzzle-engine.md)
- [CI-002 — Deterministic Rule Profiling (Flamegraphs)](../backlog/cool-ideas/CI-002-deterministic-flamegraphs.md)

## Open GitHub Issues

| Issue                                                   | Title                                                            | Recommendation                                               |
| ------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------ |
| [#370](https://github.com/flyingrobots/echo/issues/370) | Track Echo v0.1.0 release bar                                    | Keep open; top-level release tracker.                        |
| [#286](https://github.com/flyingrobots/echo/issues/286) | CI: Add unit tests for `classify_changes.cjs` and `matches()`    | Keep open; current CI-hardening follow-up.                   |
| [#284](https://github.com/flyingrobots/echo/issues/284) | CI: Per-crate gate overrides in det-policy classification system | Keep open; current CI-hardening follow-up.                   |
| [#282](https://github.com/flyingrobots/echo/issues/282) | Commit-ordered rollback playbooks for TTD integration            | Keep open; mirrored by ASAP rollback-playbook card.          |
| [#279](https://github.com/flyingrobots/echo/issues/279) | Explicit negative test mapping for decoder controls              | Keep open; partially implemented but exhaustiveness remains. |
| [#246](https://github.com/flyingrobots/echo/issues/246) | TT1: Security/capabilities for fork/rewind/merge in multiplayer  | Keep open; capability policy remains future work.            |
| [#239](https://github.com/flyingrobots/echo/issues/239) | Tooling: Reliving debugger UX                                    | Keep open; mirrored in inbox/task DAG.                       |
| [#238](https://github.com/flyingrobots/echo/issues/238) | Demo 3: Tumble Tower docs course                                 | Keep open as demo/course idea work.                          |
| [#237](https://github.com/flyingrobots/echo/issues/237) | Demo 3: Tumble Tower visualization                               | Keep open as demo/course idea work.                          |
| [#236](https://github.com/flyingrobots/echo/issues/236) | Demo 3: Tumble Tower controlled desync breakers                  | Keep open as demo/course idea work.                          |
| [#235](https://github.com/flyingrobots/echo/issues/235) | Demo 3: Tumble Tower lockstep harness                            | Keep open as demo/course idea work.                          |
| [#234](https://github.com/flyingrobots/echo/issues/234) | Demo 3: Tumble Tower Stage 3 physics                             | Keep open as demo/course idea work.                          |
| [#233](https://github.com/flyingrobots/echo/issues/233) | Demo 3: Tumble Tower Stage 2 physics                             | Keep open as demo/course idea work.                          |
| [#232](https://github.com/flyingrobots/echo/issues/232) | Demo 3: Tumble Tower Stage 1 physics                             | Keep open as demo/course idea work.                          |
| [#231](https://github.com/flyingrobots/echo/issues/231) | Demo 3: Tumble Tower Stage 0 physics                             | Keep open as demo/course idea work.                          |
| [#226](https://github.com/flyingrobots/echo/issues/226) | Demo 2: Splash Guy docs course                                   | Keep open as demo/course idea work.                          |
| [#225](https://github.com/flyingrobots/echo/issues/225) | Demo 2: Splash Guy visualization                                 | Keep open as demo/course idea work.                          |
| [#224](https://github.com/flyingrobots/echo/issues/224) | Demo 2: Splash Guy controlled desync lessons                     | Keep open as demo/course idea work.                          |
| [#223](https://github.com/flyingrobots/echo/issues/223) | Demo 2: Splash Guy lockstep harness                              | Keep open as demo/course idea work.                          |
| [#222](https://github.com/flyingrobots/echo/issues/222) | Demo 2: Splash Guy deterministic rules and state model           | Keep open as demo/course idea work.                          |
| [#207](https://github.com/flyingrobots/echo/issues/207) | Backlog: Run noisy-line test for naming                          | Keep open; mirrored in tooling/misc inbox.                   |
| [#205](https://github.com/flyingrobots/echo/issues/205) | TT2: Reliving debugger MVP                                       | Keep open; time-travel/debugger work remains future.         |
| [#204](https://github.com/flyingrobots/echo/issues/204) | TT3: Provenance heatmap                                          | Keep open; future debugger/provenance idea.                  |
| [#202](https://github.com/flyingrobots/echo/issues/202) | Spec: Provenance Payload v1                                      | Keep open; mirrored in security inbox.                       |
| [#199](https://github.com/flyingrobots/echo/issues/199) | TT3: Wesley worldline diff                                       | Keep open; future diff tool.                                 |
| [#198](https://github.com/flyingrobots/echo/issues/198) | W1: Provenance as query semantics                                | Keep open; mirrored in Wesley boundary grammar work.         |
| [#195](https://github.com/flyingrobots/echo/issues/195) | Backlog: JS-ABI packet checksum v2                               | Keep open; protocol-version follow-up.                       |
| [#194](https://github.com/flyingrobots/echo/issues/194) | W1: SchemaDelta vocabulary                                       | Keep open; mirrored in Wesley boundary grammar work.         |
| [#193](https://github.com/flyingrobots/echo/issues/193) | W1: Schema hash chain pinning                                    | Keep open; mirrored in Wesley boundary grammar work.         |
| [#190](https://github.com/flyingrobots/echo/issues/190) | M4: Determinism torture harness                                  | Keep open; still useful as broader stress suite.             |
| [#187](https://github.com/flyingrobots/echo/issues/187) | M4: Worldline convergence property suite                         | Keep open; generalized convergence suite remains open.       |
| [#174](https://github.com/flyingrobots/echo/issues/174) | W1: Wesley as a boundary grammar                                 | Keep open; mirrored in Wesley boundary grammar work.         |
| [#173](https://github.com/flyingrobots/echo/issues/173) | S1: Deterministic Rhai surface                                   | Keep open; mirrored in deterministic Rhai inbox.             |
| [#172](https://github.com/flyingrobots/echo/issues/172) | TT3: Rulial diff / worldline compare MVP                         | Keep open; future compare tooling.                           |
| [#171](https://github.com/flyingrobots/echo/issues/171) | TT2: Time Travel MVP                                             | Keep open; time-travel core remains future.                  |
| [#79](https://github.com/flyingrobots/echo/issues/79)   | Docs/logging                                                     | Keep open; mirrored in tooling/misc inbox.                   |
| [#76](https://github.com/flyingrobots/echo/issues/76)   | File watcher/debounce                                            | Keep open; mirrored in hot-reload inbox.                     |
| [#75](https://github.com/flyingrobots/echo/issues/75)   | Draft hot-reload spec                                            | Keep open; mirrored in hot-reload inbox.                     |
| [#36](https://github.com/flyingrobots/echo/issues/36)   | CI: verify signatures                                            | Keep open; mirrored in signing pipeline.                     |
| [#35](https://github.com/flyingrobots/echo/issues/35)   | Key management doc                                               | Keep open; mirrored in signing pipeline.                     |
| [#34](https://github.com/flyingrobots/echo/issues/34)   | CLI verify path                                                  | Keep open; mirrored in signing pipeline.                     |
| [#33](https://github.com/flyingrobots/echo/issues/33)   | CI: sign release artifacts                                       | Keep open; mirrored in signing pipeline.                     |
| [#25](https://github.com/flyingrobots/echo/issues/25)   | Importer: TurtlGraph to Echo store                               | Keep open; mirrored in importer inbox.                       |
| [#24](https://github.com/flyingrobots/echo/issues/24)   | Editor Hot-Reload                                                | Keep open; mirrored in hot-reload inbox.                     |
| [#21](https://github.com/flyingrobots/echo/issues/21)   | Spec: Security Contexts                                          | Keep open; mirrored in security inbox.                       |
| [#20](https://github.com/flyingrobots/echo/issues/20)   | Spec: Commit/Manifest Signing                                    | Keep open; mirrored in signing pipeline.                     |

## Issues Closed During This Audit

| Issue                                                   | Disposition                     | Evidence                                                                                                                                               |
| ------------------------------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [#285](https://github.com/flyingrobots/echo/issues/285) | Closed as completed.            | `.github/workflows/det-gates.yml` computes `DETERMINISM_PATHS` from `det-policy.yaml`; `PLATFORM_ci-det-policy-hardening.md` marks the item completed. |
| [#281](https://github.com/flyingrobots/echo/issues/281) | Closed as completed/superseded. | `docs/determinism/RELEASE_POLICY.md` contains the staging/production blocker matrix and recommendation rules.                                          |

## Notes For Future Audits

- Do not close open issues just because they are old. Several old issues are
  mirrored in `docs/method/task-matrix.md`, backlog cards, and docs audits.
- The GitHub issue tracker contains both execution work and intentional idea
  parking lots. Keep the issue open when the repo still carries a matching
  backlog card or task-DAG node.
- Favor moving stale issue text into filesystem backlog cards before closing
  the issue, unless the work is already clearly completed by merged code/docs.
