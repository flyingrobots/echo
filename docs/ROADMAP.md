<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Roadmap (Milestones + Issue Map)

This roadmap reconciles our current plan with GitHub milestones, issues, and the Project board (Project 9). It is the single source of truth for “what’s next”.

---

## Milestones

- M1 – Golden Tests (Target: CI gates operational; bit‑exact vectors validated)
  - Bit‑exact golden vectors for state_root/commit_id (genesis, merge, empty)
  - Math kernel goldens (rotation/multiply/sin/cos)
  - CI matrix: macOS + Ubuntu (glibc) + Alpine (musl)
- M2.0 – Scalar Foundation (Target: det_fixed & det_float lanes green across OSes)
  - Scalar trait; F32Scalar deterministic wrappers; DFix64 Q32.32 (fixed‑point 32.32 format)
  - Deterministic transcendentals (LUT = lookup table + refinement); tables checked‑in
  - Motion rule → Scalar; v2 payload (6×i64 Q32.32), dual decode v1/v2
- M2.1 – Lattice Joins (Target: replay‑invariant merges under ACI properties)
  - Lattice trait; scheduler fold order (canonical)
  - Exemplar lattices: tags union, cap/max (join keys documented)
  - ACI property + replay determinism tests
- M2.2 – Playground Slice (Target: demo + CLI show identical hashes under permutations)
  - Minimal WASM demo; CLI run/diff showing replay‑identical hashes
- M2.5 – Accumulator Joins (Target: delta‑style joins pass ACI/replay tests)
  - Delta‑style joins; deterministic rounding/saturation; ACI + replay
- M3 – Sweep‑and‑Prune v1 (Target: deterministic broad‑phase replaces O(n²) baseline)
  - Integerized endpoints; stable tie‑breakers; ordering/stability property tests
- M4 – Determinism Proof & Publish 0.1 (Target: cross‑OS proof + 0.1 release)
  - Prove determinism across OSes; finalize docs; publish warp‑core/geom 0.1

### Tooling & Time Travel Track (T*/TT*/S*/W*)

These milestones are tracked in GitHub alongside the M* milestones. They are a parallel track focused on deterministic tooling, time travel semantics, and view/query surfaces.

- T2 – Embedded Tooling UI Baseline (Target: ship a stable embedded dashboard baseline + deterministic screenshot evidence)
  - Open Props baseline for embedded dashboards served from binaries
  - Playwright screenshot capture wired into docs (regenerable evidence)
- TT0 – Time Model Spec Lock (Target: formalize TimeStreams/cursors/admission as HistoryTime)
  - TimeStreams + stream cursors model
  - Admission decisions as history artifacts (admission_digest / receipts)
  - Wormholes/catch-up semantics (compression vs replay)
- TT1 – Streams Inspector Frame (Target: first tool that can inspect backlogs + admission decisions)
- TT2 – Time Travel MVP (Target: pause/rewind/buffer/catch-up loop in tooling)
- TT3 – Rulial Diff / Worldline Compare (Target: side-by-side run diff + first-divergence UX)
- S1 – Deterministic Rhai Surface (Target: replayable scripting sandbox w/ claims/effects boundary)
- W1 – Wesley as a Boundary Grammar (Target: hashable view artifacts + schema pinning for replay integrity)

---

## Issue Table (live snapshot)

Rows are GitHub issues. Priority/Estimate reflect Project 9 fields. Block/parent relationships use native GitHub issue dependencies; no custom text fields are used. Refresh cadence: update weekly or before each planning cycle.

Note:

- Some PRs (especially docs-only / repo maintenance / workflow hygiene) are intentionally **un-milestoned**.
- Those items should still be tracked in Project 9 (and linked to an issue when non-trivial), but they do not represent a “ship milestone” outcome.

| Issue Name | # | Milestone | Priority | Estimate | Blocked By | Blocking | Parent | Children | Remarks |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| Benchmarks & CI Regression Gates | 22 | M1 – Golden Tests | P1 | 13h+ |  | #42,#43,#44,#45,#46 |  | 42,43,44,45,46 | Umbrella for perf pipeline |
| Create benches crate | 42 | M1 – Golden Tests | P1 | 3h | #22 | #43,#44,#45,#46 | #22 |  | Criterion + scaffolding |
| Snapshot hash microbench | 43 | M1 – Golden Tests | P1 | 5h | #22,#42 |  | #22 |  | Reachable hash microbench |
| Scheduler drain microbench | 44 | M1 – Golden Tests | P1 | 5h | #22,#42 |  | #22 |  | Deterministic rule‑order/drain |
| JSON report + CI upload | 45 | M1 – Golden Tests | P2 | 3h | #22,#42 |  | #22 |  | Upload Criterion JSON |
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
| Spec: Commit/Manifest Signing | 20 | Backlog |  |  |  |  |  |  | Keep under Backlog until publish plan is firm |
| Spec: Security Contexts (FFI/WASM/CLI) | 21 | Backlog |  |  |  |  |  |  | Backlog (security track) |
| Plugin ABI (C) v0 | 26 | Backlog |  |  |  |  |  |  | Track in separate ABI milestone later |
| Example plugin + tests | 89 | Backlog |  |  |  |  |  |  | Depends on ABI |
| Capability tokens | 88 | Backlog |  |  |  |  |  |  | — |
| Version negotiation | 87 | Backlog |  |  |  |  |  |  | — |
| C header + host loader | 86 | Backlog |  |  |  |  |  |  | — |
| Draft C ABI spec | 85 | Backlog |  |  |  |  |  |  | — |
| Importer + store tasks | 80–84 | Backlog |  |  |  |  |  |  | Import flow (spec/loader/reader) |

Note: Backlog means “not part of the current M1/M2 trajectory”; issues remain visible in the Project with the `backlog` label and can be re‑prioritized later.

---

## Immediate Plan (Next PRs)

- PR‑11 (Closes #42): benches crate skeleton (Criterion + harness)
- PR‑12 (Closes #43): snapshot hash microbench
- PR‑13 (Closes #44): scheduler drain microbench
- PR‑14 (Closes #45): JSON artifact + upload
- PR‑15 (Closes #46): regression thresholds gate

In parallel (when ready): seed M2.0 – Scalar Foundation umbrella and child issues, then start the first scalar PR (trait + backends skeleton).

---

Maintainers: keep this file in sync when re‑prioritizing or moving issues between milestones. This roadmap complements the Project board, which carries Priority/Estimate fields and live status.
