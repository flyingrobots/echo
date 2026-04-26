<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Docs Inventory Audit - 2026-04-26

This is the active docs cleanup ledger. We are auditing tracked, human-facing
documents in `docs/` five at a time. Generated assets, VitePress config,
vendored files, and binary/image outputs are not part of the document sequence
unless an audit batch explicitly pulls them in.

## Scoring

- `5/5`: accurate, current, code-backed, and correctly placed.
- `4/5`: mostly accurate and code-backed, with small placement or wording fixes.
- `3/5`: useful, but materially stale or split across the wrong surface.
- `2/5`: mostly stale; keep only after rewrite.
- `1/5`: misleading or superseded; delete unless there is a strong reason.
- `0/5`: junk, duplicate, or generated noise; delete.

## Batch 1

### `docs/BEARING.md`

About: current direction signpost for humans and agents.

Code/doc evidence checked:

- `crates/warp-core/src/neighborhood.rs` now has `NeighborhoodSiteService`.
- `crates/warp-core/src/settlement.rs` now has `SettlementService` and
  `SettlementPlan`.
- `crates/warp-wasm/src/warp_kernel.rs` exposes neighborhood and settlement
  surfaces through the WASM kernel boundary.
- `crates/echo-wasm-abi/src/kernel_port.rs` declares `ABI_VERSION` as 6 and
  carries `ReadingEnvelope`.
- `docs/spec/SPEC-0009-wasm-abi.md` documents the current WASM ABI contract.

Accuracy as found: `2/5`.

Decision: keep, but rewrite in place. `BEARING.md` is useful as a top-level
signpost, but it was stale: it described neighborhood publication and strand
settlement as next steps even though both now have runtime/ABI shape.

Destination: `docs/BEARING.md`.

Action taken: refreshed the signpost to name the current WARP optics,
reading-envelope, docs-audit, and local-iteration hills.

### `docs/BENCHMARK_GUIDE.md`

About: operational guide for adding Criterion benchmarks, baking JSON
artifacts, and updating benchmark dashboards.

Code/doc evidence checked:

- `crates/warp-benches/Cargo.toml` uses Criterion bench targets.
- `Makefile` exposes `bench-bake`, `bench-serve`, and report helpers.
- `xtask/src/main.rs` contains `BENCH_CORE_GROUP_KEYS` and benchmark baking.
- `docs/benchmarks/index.html` consumes Criterion output.
- `.github/workflows/det-gates.yml` and `scripts/check_perf_regression.cjs`
  implement the G3 perf regression gate.

Accuracy as found: `4/5`.

Decision: keep and relocate. The contents are mostly accurate, but the top
level was the wrong place for a benchmark-specific procedure.

Destination: `docs/benchmarks/BENCHMARK_GUIDE.md`.

Action taken: moved into `docs/benchmarks/` and updated docs index links.

### `docs/DETERMINISTIC_MATH.md`

About: non-normative hazard catalog for floating-point determinism risks and
Echo mitigations.

Code/doc evidence checked:

- `crates/warp-core/src/math/scalar.rs` enforces `F32Scalar`
  canonicalization, NaN handling, signed-zero handling, and subnormal flush.
- `crates/warp-core/src/math/trig.rs` provides deterministic trig helpers.
- `crates/warp-core/src/math/fixed_q32_32.rs` implements deterministic Q32.32
  conversion helpers.
- `docs/SPEC_DETERMINISTIC_MATH.md` remains the normative policy.

Accuracy as found: `4/5`.

Decision: keep and relocate. The document is useful and accurate enough, but
it belongs with the determinism claim register and release policy.

Destination: `docs/determinism/DETERMINISTIC_MATH.md`.

Action taken: moved into `docs/determinism/` and fixed relative links.

### `docs/DOCS_AUDIT.md`

About: older whole-corpus docs audit from the pre-Method cleanup era.

Code/doc evidence checked:

- The file named deleted/moved docs and old roadmap structures that no longer
  match the tracked tree.
- Live Method docs still pointed at it as the canonical audit source.
- A newer, dated ledger is needed for this five-at-a-time process.

Accuracy as found: `1/5`.

Decision: trash. It is superseded, stale enough to mislead agents, and already
preserved in git history.

Destination: none.

Action taken: deleted `docs/DOCS_AUDIT.md` and replaced its live references
with this dated audit ledger.

### `docs/RELEASE_POLICY.md`

About: TTD/determinism release gate policy, including G1-G4 blocker states and
nondeterminism allowlist governance.

Code/doc evidence checked:

- `.ban-nondeterminism-allowlist` and `scripts/ban-nondeterminism.sh` implement
  the allowlist path described by the policy.
- `det-policy.yaml` names crate ownership/approval roles.
- `.github/workflows/det-gates.yml` implements G1-G4 gates.
- `scripts/check_perf_regression.cjs` backs the G3 perf regression gate.
- `scripts/check_task_lists.sh` exists; the policy correctly says it does not
  cover allowlist auditing.

Accuracy as found: `4/5`.

Decision: keep and relocate. The policy is accurate enough and enforceable,
but it belongs under determinism, not the top-level docs namespace.

Destination: `docs/determinism/RELEASE_POLICY.md`.

Action taken: moved into `docs/determinism/` and updated live references.

## Out-of-Batch Findings

- `docs/.vitepress/config.ts` still pointed "Docs Map" at the removed
  `/meta/docs-index` route. That was fixed to `/`, because `docs/index.md`
  declares itself the live docs map.
- `docs/index.md` still linked to removed `/ROADMAP` and `/METHODOLOGY`
  routes. Those were corrected to the Method index while touching the same
  live docs map for moved-file links.
- `pnpm docs:build` exposed a preexisting VitePress compiler blocker in
  `docs/assets/dags/tasks-dag-source.md`: a bare `Vec<u8>` generic in prose
  was parsed as an HTML tag. That file is append-only source data for generated
  diagrams, not a reader-facing page, so it is now excluded from VitePress page
  compilation instead of editing historical content.
- After that compiler fix, the build advanced to dead-link checking and exposed
  a larger preexisting dead-link set, especially stale `ROADMAP` links inside
  Method backlog items. That should become its own cleanup batch instead of
  being hidden.
