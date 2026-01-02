<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Issues Matrix (Active Plan)

This table mirrors the current state of active issues in Project 9 with our plan-aligned milestones and relationships. Native GitHub dependencies represent "blocked by"/"blocking"; we no longer use custom text fields for these. The Project board remains the live system of record for status.

| Issue Name | Issue # | Milestone | Priority | Estimate | Blocked By | Blocking | Parent | Children | Remarks |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| Benchmarks & CI Regression Gates | 22 | M1 – Golden Tests | P1 | 13h+ |  | #42,#43,#44,#45,#46 |  | 42,43,44,45,46 | Umbrella for perf pipeline |
| Create benches crate | 42 | M1 – Golden Tests | P1 | 3h | #22 | #43,#44,#45,#46 | #22 |  | Criterion + scaffolding |
| Snapshot hash microbench | 43 | M1 – Golden Tests | P1 | 5h | #22,#42 |  | #22 |  | Reachable hash microbench |
| Scheduler drain microbench | 44 | M1 – Golden Tests | P1 | 5h | #22,#42 |  | #22 |  | Deterministic rule‑order/drain |
| JSON report + CI upload | 45 | M1 – Golden Tests | P2 | 3h | #22,#42 | #46 | #22 |  | Upload Criterion JSON |
| Regression thresholds gate | 46 | M1 – Golden Tests | P1 | 8h | #22,#42,#45 |  | #22 |  | Fail on P50/P95/P99 regress |
| CLI: verify/bench/inspect | 23 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  | Grouping placeholder; break down in PRs |
| Scaffold CLI subcommands | 47 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Implement 'verify' | 48 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Implement 'bench' | 49 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Implement 'inspect' | 50 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Docs/man pages | 51 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  | Tie docs to CLI UX |
| README+docs (defaults & toggles) | 41 | M4 – Determinism Proof & Publish 0.1 | P2 | 3h |  |  |  |  | Docs polish before 0.1 |
| Deterministic trig: pin error budget + deterministic oracle for audit test | 177 | M4 – Determinism Proof & Publish 0.1 |  |  |  |  |  |  | Cross-OS determinism gate; keep oracle host-independent |
| T2: Embedded tooling UI baseline (Open Props + screenshot regen) | 168 | T2 – Embedded Tooling UI Baseline |  |  |  |  |  |  | Embedded dashboard baseline + Playwright evidence |
| TT0: Time model spec lock (TimeStreams + admission digests) | 166 | TT0 – Time Model Spec Lock |  |  |  |  |  |  | Spec lock for time model primitives (streams/cursors/admission) |
| TT1: StreamsFrame inspector support (backlog + cursors + admission decisions) | 170 | TT1 – Streams Inspector Frame |  |  |  |  |  |  | Inspector scaffolding for stream backlogs and admission decisions |
| TT2: Time Travel MVP (pause/rewind/buffer/catch-up) | 171 | TT2 – Time Travel MVP |  |  |  |  |  |  | Pause/rewind UX + buffering policies |
| TT3: Rulial diff / worldline compare MVP | 172 | TT3 – Rulial Diff / Worldline Compare |  |  |  |  |  |  | Side-by-side run comparison tooling |
| S1: Deterministic Rhai surface (sandbox + claims/effects) | 173 | S1 – Deterministic Rhai Surface |  |  |  |  |  |  | Deterministic sandbox boundary for scripts |
| W1: Wesley as a boundary grammar (hashable view artifacts) | 174 | W1 – Wesley as a Boundary Grammar |  |  |  |  |  |  | Hashable grammar + pinned semantics for replay integrity |

Backlog issues are labeled `backlog` and kept visible in the Project; they will be prioritized into milestones as needed.
