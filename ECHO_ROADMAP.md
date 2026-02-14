<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ECHO_ROADMAP — Phased Plan (Post-ADR Alignment)

## Active Sprint: TTD-HARDENING-S1 (2026-02-14 to 2026-02-21)

**Goal:** Formalize the TTD (Time-Travel Determinism) hardening gates and evidence integrity.

- [ ] **G1 (DET):** Multi-platform determinism matrix (macOS/Linux + wasm).
- [ ] **G2 (SEC):** Explicit negative test mapping for decoder controls.
- [ ] **G3 (PRF):** Criterion baseline + regression threshold for materialization path.
- [ ] **G4 (REP):** Enforce artifact-backed VERIFIED claims and path-aware gates.
- [ ] **GOV:** Publish release policy and commit-ordered rollback playbooks.

---

This roadmap re-syncs active work with recent ADRs:

- ADR-0003: Causality-first API + MaterializationBus/Port
- ADR-0004: No global state / explicit dependency injection
- ADR-0005: Physics as deterministic scheduled rewrites
- ADR-0006: Ban non-determinism via CI guards

It also incorporates the latest DIND status from `GEMINI_CONTINUE_NOTES.md`.

---

## Phase 0 — Repo Hygiene & Ownership

Goal: eliminate structural drift and restore correct ownership boundaries.

- Move `crates/echo-dind-harness/` to the Echo repo (submodule) where it belongs.
    - Remove the crate from this workspace once moved.
    - Ensure any references/scripts in this repo point to the Echo submodule path.
- Audit for other Echo-owned crates/docs accidentally mirrored here.
- Update docs to reflect the correct location of DIND tooling.

Exit criteria:

- `crates/echo-dind-harness/` no longer exists in this repo.
- A clear pointer exists for where to run DIND locally (Echo repo).

---

## Phase 1 — Determinism Guardrails (ADR-0004 + ADR-0006)

Goal: codify the “no global state / no nondeterminism” doctrine and enforce it in CI.

- Add CI scripts:
    - `scripts/ban-globals.sh` (ADR-0004)
    - `scripts/ban-nondeterminism.sh` and `scripts/ban-unordered-abi.sh` (ADR-0006)
- Wire scripts into CI for core crates (warp-core, warp-wasm, app wasm).
- Add minimal allowlist files (empty by default).
- Document determinism rules in README / doctrine doc.

Exit criteria:

- CI fails on banned patterns.
- No global init (`install_*` style) in runtime core.

---

## Phase 2 — Causality-First Boundary (ADR-0003)

Goal: enforce ingress-only writes and bus-first reads.

- Define/confirm canonical intent envelopes for ingress (bytes-only).
- Ensure all write paths use ingress; remove any public “direct mutation” APIs.
- Implement MaterializationBus + MaterializationPort boundary:
    - `view_subscribe`, `view_drain`, `view_replay_last`, `view_unsubscribe`
    - channel IDs are byte-based (TypeId-derived), no strings in ABI
- Ensure UI uses materializations rather than direct state reads (except inspector).
- Define InspectorPort as a gated, separate API (optional).

Exit criteria:

- No direct mutation path exposed to tools/UI.
- UI can run solely on materialized channels (or has a plan to get there).

---

## Phase 3 — Physics Pipeline (ADR-0005)

Goal: implement deterministic physics as scheduled rewrites.

- Implement tick phases:
    1. Integrate (predict)
    2. Candidate generation (broadphase + narrowphase)
    3. Solver iterations with footprint scheduling
    4. Finalize (commit)
- Canonical ordering:
    - candidate keys: `(toi_q, min_id, max_id, feature_id)`
    - deterministic iteration order for bodies and contacts
- Add optional trace channels for physics (debug materializations).
- Ensure physics outputs only emit post-commit.

Exit criteria:

- Physics determinism across wasm/native with fixed seeds and inputs.
- No queue-based “micro-inbox” for derived physics work.

---

## Phase 4 — DIND Mission Continuation (from GEMINI_CONTINUE_NOTES)

Goal: complete Mission 3 polish and Mission 4 performance envelope.

### Mission 3 (Polish / Verification)

- Badge scoping: clarify scope (“PR set”) and platforms.
- Badge truth source: generate from CI artifacts only.
- Matrix audit: confirm explicit aarch64 coverage needs.

### Mission 4 (Performance Envelope)

- Add `perf` command to DIND harness:
    - `perf <scenario> --baseline <file> --tolerance 15%`
    - track `time_ms`, `steps`, `time_per_step`
    - optional: max nodes/edges, allocations
- Add baseline: `testdata/dind/perf_baseline.json`
- CI:
    - PR: core scenarios, release build, fail on >15% regression
    - Nightly: full suite, upload perf artifacts

Exit criteria:

- DIND perf regressions fail CI.
- Stable baseline file committed.

---

## Phase 5 — App-Repo Integration (flyingrobots.dev specific)

Goal: keep app-specific wasm boundary clean and deterministic.

- Ensure TS encoders are the source of truth for binary protocol.
- Keep WASM as a thin bridge (no placeholder exports).
- Verify handshake matches registry version / codec / schema hash.
- Add or update tests verifying canonical ordering and envelope bytes.

Exit criteria:

- ABI tests use TS encoders, not wasm placeholder exports.
- wasm build + vitest pass.

---

## Open Questions / Dependencies

- Precise target crates for determinism guardrails in this repo vs Echo repo.
- Whether InspectorPort needs to exist in flyingrobots.dev or only in Echo.
- Final home for DIND artifacts: Echo repo or shared tooling repo.

---

## Suggested Execution Order

1. Phase 0 (move DIND harness) to prevent ownership drift.
2. Phase 1 guardrails to lock determinism.
3. Phase 2 boundary enforcement (ingress + bus).
4. Phase 3 physics pipeline.
5. Phase 4 DIND polish/perf.
6. Phase 5 app integration clean-up.
