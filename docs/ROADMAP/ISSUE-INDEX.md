<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Issue Coverage Index

This index maps tracked GitHub issues (open and carry-forward references) to roadmap tasks and feature files.

| Issue | Title                                               | Task(s)              | Feature File                                                                                       |
| ----: | --------------------------------------------------- | -------------------- | -------------------------------------------------------------------------------------------------- |
|   #20 | Spec: Commit/Manifest Signing                       | T-10-2-1             | [backlog/security.md](backlog/security.md)                                                         |
|   #21 | Spec: Security Contexts (FFI/WASM/CLI)              | T-10-2-2             | [backlog/security.md](backlog/security.md)                                                         |
|   #22 | Benchmarks & CI Regression Gates                    | T-1-2-1              | [lock-the-hashes/benchmarks-cleanup.md](lock-the-hashes/benchmarks-cleanup.md)                     |
|   #23 | CLI: verify/bench/inspect (umbrella)                | F6.\*                | [developer-cli/](developer-cli/README.md)                                                          |
|   #24 | Editor Hot-Reload (spec + impl)                     | T-10-4-3             | [backlog/editor-hot-reload.md](backlog/editor-hot-reload.md)                                       |
|   #25 | Importer: TurtlGraph -> Echo store                  | T-10-5-1             | [backlog/importer.md](backlog/importer.md)                                                         |
|   #26 | Plugin ABI (C) v0 (umbrella)                        | F10.1.\*             | [backlog/plugin-abi.md](backlog/plugin-abi.md)                                                     |
|   #33 | CI: sign release artifacts (dry run)                | T-10-3-2             | [backlog/signing-pipeline.md](backlog/signing-pipeline.md)                                         |
|   #34 | CLI verify path                                     | T-10-3-3             | [backlog/signing-pipeline.md](backlog/signing-pipeline.md)                                         |
|   #35 | Key management doc                                  | T-10-3-1             | [backlog/signing-pipeline.md](backlog/signing-pipeline.md)                                         |
|   #36 | CI: verify signatures                               | T-10-3-4             | [backlog/signing-pipeline.md](backlog/signing-pipeline.md)                                         |
|   #38 | FFI limits and validation                           | T-10-2-3             | [backlog/security.md](backlog/security.md)                                                         |
|   #41 | README+docs (defaults & toggles)                    | T-9-4-1              | [proof-core/docs-polish.md](proof-core/docs-polish.md)                                             |
|   #47 | Scaffold CLI subcommands                            | T-6-1-1              | [developer-cli/cli-scaffold.md](developer-cli/cli-scaffold.md)                                     |
|   #48 | Implement verify                                    | T-6-2-1              | [developer-cli/verify.md](developer-cli/verify.md)                                                 |
|   #49 | Implement bench                                     | T-6-3-1              | [developer-cli/bench.md](developer-cli/bench.md)                                                   |
|   #50 | Implement inspect                                   | T-6-4-1              | [developer-cli/inspect.md](developer-cli/inspect.md)                                               |
|   #51 | Docs/man pages                                      | T-6-5-1              | [developer-cli/docs-man-pages.md](developer-cli/docs-man-pages.md)                                 |
|   #75 | Draft hot-reload spec                               | T-10-4-1             | [backlog/editor-hot-reload.md](backlog/editor-hot-reload.md)                                       |
|   #76 | File watcher/debounce                               | T-10-4-2             | [backlog/editor-hot-reload.md](backlog/editor-hot-reload.md)                                       |
|   #79 | Docs/logging                                        | T-10-8-1             | [backlog/tooling-misc.md](backlog/tooling-misc.md)                                                 |
|   #85 | Draft C ABI spec                                    | T-10-1-1             | [backlog/plugin-abi.md](backlog/plugin-abi.md)                                                     |
|   #86 | C header + host loader                              | T-10-1-2             | [backlog/plugin-abi.md](backlog/plugin-abi.md)                                                     |
|   #87 | Version negotiation                                 | T-10-1-3             | [backlog/plugin-abi.md](backlog/plugin-abi.md)                                                     |
|   #88 | Capability tokens                                   | T-10-1-4             | [backlog/plugin-abi.md](backlog/plugin-abi.md)                                                     |
|   #89 | Example plugin + tests                              | T-10-1-5             | [backlog/plugin-abi.md](backlog/plugin-abi.md)                                                     |
|  #170 | TT1: StreamsFrame inspector support                 | T-7-2-5              | [time-travel/streams-inspector-frame.md](time-travel/streams-inspector-frame.md)                   |
|  #171 | TT2: Time Travel MVP                                | T-7-3-1, T-7-3-2     | [time-travel/time-travel-mvp.md](time-travel/time-travel-mvp.md)                                   |
|  #172 | TT3: Rulial diff / worldline compare                | T-7-4-1              | [time-travel/rulial-diff.md](time-travel/rulial-diff.md)                                           |
|  #173 | S1: Deterministic Rhai surface                      | T-10-6-1a, T-10-6-1b | [backlog/deterministic-rhai.md](backlog/deterministic-rhai.md)                                     |
|  #174 | W1: Wesley boundary grammar                         | T-10-7-1             | [backlog/wesley-boundary-grammar.md](backlog/wesley-boundary-grammar.md)                           |
|  #177 | Deterministic trig oracle (carry-forward reference) | T-9-3-1              | [proof-core/deterministic-trig.md](proof-core/deterministic-trig.md)                               |
|  #185 | M1: Domain-separated hash contexts (core)           | T-1-1-1              | [lock-the-hashes/domain-separated-hashes.md](lock-the-hashes/domain-separated-hashes.md)           |
|  #186 | M1: Domain-separated digest (RenderGraph)           | T-1-1-2              | [lock-the-hashes/domain-separated-hashes.md](lock-the-hashes/domain-separated-hashes.md)           |
|  #187 | M4: Worldline convergence suite                     | T-9-2-1, T-9-2-2     | [proof-time-convergence/worldline-convergence.md](proof-time-convergence/worldline-convergence.md) |
|  #190 | M4: Determinism torture harness                     | T-9-1-1, T-9-1-2     | [proof-core/determinism-torture.md](proof-core/determinism-torture.md)                             |
|  #191 | TT0: Session stream time fields                     | T-7-1-1              | [time-semantics-lock/time-model-spec.md](time-semantics-lock/time-model-spec.md)                   |
|  #192 | TT0: TTL/deadline semantics                         | T-7-1-2              | [time-semantics-lock/time-model-spec.md](time-semantics-lock/time-model-spec.md)                   |
|  #193 | W1: Schema hash chain pinning                       | T-10-7-2             | [backlog/wesley-boundary-grammar.md](backlog/wesley-boundary-grammar.md)                           |
|  #194 | W1: SchemaDelta vocabulary                          | T-10-7-3             | [backlog/wesley-boundary-grammar.md](backlog/wesley-boundary-grammar.md)                           |
|  #195 | JS-ABI packet checksum v2                           | T-10-2-4             | [backlog/security.md](backlog/security.md)                                                         |
|  #198 | W1: Provenance as query semantics                   | T-10-7-4             | [backlog/wesley-boundary-grammar.md](backlog/wesley-boundary-grammar.md)                           |
|  #199 | TT3: Wesley worldline diff                          | T-7-4-2              | [time-travel/rulial-diff.md](time-travel/rulial-diff.md)                                           |
|  #202 | Spec: Provenance Payload (PP) v1                    | T-10-2-5             | [backlog/security.md](backlog/security.md)                                                         |
|  #203 | TT1: Constraint Lens panel                          | T-7-2-6              | [time-travel/streams-inspector-frame.md](time-travel/streams-inspector-frame.md)                   |
|  #204 | TT3: Provenance heatmap                             | T-7-4-3              | [time-travel/rulial-diff.md](time-travel/rulial-diff.md)                                           |
|  #205 | TT2: Reliving debugger MVP                          | T-7-3-2              | [time-travel/time-travel-mvp.md](time-travel/time-travel-mvp.md)                                   |
|  #207 | Naming test (noisy-line)                            | T-10-8-2             | [backlog/tooling-misc.md](backlog/tooling-misc.md)                                                 |
|  #222 | Splash Guy: rules + state model                     | T-8-1-1              | [splash-guy/rules-and-state.md](splash-guy/rules-and-state.md)                                     |
|  #223 | Splash Guy: lockstep protocol                       | T-8-1-2              | [splash-guy/lockstep-protocol.md](splash-guy/lockstep-protocol.md)                                 |
|  #224 | Splash Guy: controlled desync                       | T-8-1-3              | [splash-guy/controlled-desync.md](splash-guy/controlled-desync.md)                                 |
|  #225 | Splash Guy: visualization                           | T-8-1-4              | [splash-guy/visualization.md](splash-guy/visualization.md)                                         |
|  #226 | Splash Guy: docs course                             | T-8-1-5              | [splash-guy/course-material.md](splash-guy/course-material.md)                                     |
|  #231 | Tumble Tower: Stage 0 (AABB)                        | T-8-2-1              | [tumble-tower/stage-0-aabb.md](tumble-tower/stage-0-aabb.md)                                       |
|  #232 | Tumble Tower: Stage 1 (rotation)                    | T-8-2-2              | [tumble-tower/stage-1-rotation.md](tumble-tower/stage-1-rotation.md)                               |
|  #233 | Tumble Tower: Stage 2 (friction)                    | T-8-2-3              | [tumble-tower/stage-2-friction.md](tumble-tower/stage-2-friction.md)                               |
|  #234 | Tumble Tower: Stage 3 (sleeping)                    | T-8-2-4              | [tumble-tower/stage-3-sleeping.md](tumble-tower/stage-3-sleeping.md)                               |
|  #235 | Tumble Tower: lockstep harness                      | T-8-2-5              | [tumble-tower/lockstep-harness.md](tumble-tower/lockstep-harness.md)                               |
|  #236 | Tumble Tower: desync breakers                       | T-8-2-6              | [tumble-tower/desync-breakers.md](tumble-tower/desync-breakers.md)                                 |
|  #237 | Tumble Tower: visualization                         | T-8-2-7              | [tumble-tower/visualization.md](tumble-tower/visualization.md)                                     |
|  #238 | Tumble Tower: docs course                           | T-8-2-8              | [tumble-tower/course-material.md](tumble-tower/course-material.md)                                 |
|  #239 | Reliving debugger UX                                | T-10-8-3             | [backlog/tooling-misc.md](backlog/tooling-misc.md)                                                 |
|  #243 | TT1: dt policy                                      | T-7-2-1              | [time-travel/streams-inspector-frame.md](time-travel/streams-inspector-frame.md)                   |
|  #244 | TT1: TimeStream retention                           | T-7-2-2              | [time-travel/streams-inspector-frame.md](time-travel/streams-inspector-frame.md)                   |
|  #245 | TT1: Merge semantics                                | T-7-2-3              | [time-travel/streams-inspector-frame.md](time-travel/streams-inspector-frame.md)                   |
|  #246 | TT1: Security/capabilities                          | T-7-2-4              | [time-travel/streams-inspector-frame.md](time-travel/streams-inspector-frame.md)                   |
