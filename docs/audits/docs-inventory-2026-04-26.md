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

---

## Log Policy

As of this entry, this file is append-only. New audit entries are appended at
the bottom and separated with `---`. If a past decision becomes wrong, append a
new correction entry instead of rewriting earlier audit text.

---

## Batch 2

### `docs/SPEC_DETERMINISTIC_MATH.md`

About: normative deterministic-math policy for `warp-core`.

Code/doc evidence checked:

- `crates/warp-core/src/math/scalar.rs` implements `F32Scalar::new(...)`
  canonicalization for NaN, signed zero, and subnormals.
- `crates/warp-core/src/math/trig.rs` implements deterministic LUT-backed
  `sin`/`cos` and avoids platform transcendentals.
- `crates/warp-core/src/math/fixed_q32_32.rs` implements deterministic Q32.32
  conversions for the `det_fixed` lane.
- `.github/workflows/ci.yml` runs the float, fixed-point, and MUSL validation
  lanes named by the doc.
- `scripts/check_no_raw_trig.sh` backs the raw-transcendental policy.

Accuracy as found: `4/5`.

Decision: keep and relocate. The content is code-backed and should remain the
normative math policy, but it belongs with the determinism corpus.

Destination: `docs/determinism/SPEC_DETERMINISTIC_MATH.md`.

Action taken: moved into `docs/determinism/` and updated live references.

### `docs/THEORY.md`

About: AIΩN / WARP foundations paraphrase with implementation-deviation notes
for Echo.

Code/doc evidence checked:

- `crates/warp-core/src/attachment.rs` and `crates/warp-core/src/tick_patch.rs`
  back the typed-atom / `Descend(WarpId)` / `OpenPortal` model.
- `crates/warp-core/src/tick_patch.rs` rejects dangling portals and orphan
  instances during patch replay.
- `crates/warp-core/src/tick_patch.rs` and `crates/warp-core/src/receipt.rs`
  back the delta-first patch and receipt notes.
- `crates/warp-core/src/worldline_state.rs` backs linear worldline state and
  tick-history storage.
- Paper IV observer geometry is explicitly marked as not implemented in
  `warp-core`; current runtime evidence is the reading-envelope work.

Accuracy as found: `3/5`.

Decision: keep, but demote from top-level docs. It is useful north-star
material, not an implementation spec. It also pointed to missing
`docs/aion-papers-bridge.md`.

Destination: `docs/theory/THEORY.md`.

Action taken: moved into `docs/theory/` and replaced the missing bridge link
with live links to `docs/architecture/WARP_DRIFT.md` and the WARP terms doc.

### `docs/WARP_DRIFT.md`

About: current gap analysis between Echo's runtime/docs and the stronger WARP
optic/observer/strand doctrine.

Code/doc evidence checked:

- `crates/warp-core/src/strand.rs` implements `Strand::live_basis_report(...)`.
- `crates/warp-core/src/settlement.rs` implements `SettlementPlan`,
  `SettlementDecision`, `ConflictArtifactDraft`, and `SettlementResult`.
- `crates/warp-wasm/src/warp_kernel.rs` exposes `dispatch_intent(...)`,
  `observe(...)`, neighborhood publication, and settlement entrypoints.
- `crates/echo-wasm-abi/src/kernel_port.rs` exposes `ReadingEnvelope` and
  settlement/observation ABI types.
- Referenced Method items and designs for live holographic strands, observer
  plans, reading envelopes, and witnessed suffix shells exist.

Accuracy as found: `4/5`.

Decision: keep and relocate. This is a useful current architecture/drift memo,
but it belongs under architecture rather than the top-level docs namespace.

Destination: `docs/architecture/WARP_DRIFT.md`.

Action taken: moved into `docs/architecture/` and updated references.

### `docs/architecture-outline.md`

About: high-level Echo architecture draft mixing current runtime facts,
Continuum context, and future product/ECS/interface ideas.

Code/doc evidence checked:

- `crates/warp-core` backs the current hot runtime claims.
- `crates/warp-core/src/materialization/` backs the `MaterializationBus`
  claims.
- `crates/echo-scene-port`, `crates/echo-scene-codec`, and
  `packages/echo-renderer-three` back the scene boundary claims.
- `crates/ttd-browser`, `crates/echo-wesley-gen`, and `PrivacyMask` types exist.
- The doc also linked removed ADR/RFC routes and still contains large planned
  ECS/product sections.

Accuracy as found: `2/5`.

Decision: keep only as a draft context artifact and quarantine it under
`docs/architecture/`. It should not be presented as the authoritative system
map until rewritten more aggressively.

Destination: `docs/architecture/outline.md`.

Action taken: moved into `docs/architecture/`, downgraded README/index wording,
and replaced missing ADR/RFC links with code-backed materialization evidence.

### `docs/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md`

About: canonical terminology for WARP state, skeletons, attachment planes,
instances, portals, wormholes, and slicing.

Code/doc evidence checked:

- `crates/warp-core/src/attachment.rs` defines `AttachmentValue::Atom` and
  `AttachmentValue::Descend(WarpId)`.
- `crates/warp-core/src/tick_patch.rs` defines `WarpOp::OpenPortal` and
  validates dangling-portal/orphan-instance invariants.
- `crates/warp-core/src/worldline_state.rs` and `crates/warp-core/src/lib.rs`
  expose `WarpState`, `WorldlineState`, and related runtime terms.
- `docs/spec/SPEC-0002-descended-attachments-v1.md` matches the flattened
  indirection model.

Accuracy as found: `5/5`.

Decision: keep in place. This is one of the better architecture docs: precise,
code-backed, and already in the right directory.

Destination: `docs/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md`.

Action taken: none beyond recording the audit decision.

---

## Verification Note

`pnpm docs:build` now gets past the moved-file links from batch 2, but still
fails on 59 dead links. The remaining class is the known docs-site backlog:
stale `ROADMAP` links in Method backlog/graveyard files, `scheduler.md` pointing
at missing `spec-scheduler`, `benchmarks/PARALLEL_POLICY_MATRIX.md` pointing at
missing `report-inline`, and a few non-doc-root links. Do not hide this with
`ignoreDeadLinks`; fix it as a dedicated cleanup batch.

---

## Batch 2 Follow-up

A post-move stale-reference sweep found remaining live references to the old
top-level paths in `CONTRIBUTING.md`, `CLAUDE.md`, and
`docs/method/backlog/asap/DOCS_docs-cleanup.md`. These were updated to the new
destinations under `docs/architecture/`, `docs/determinism/`, and
`docs/theory/`.

Historical path references in older `docs/audits/` files were left untouched as
audit records rather than rewritten.

---

## docs/audits/docs-inventory-2026-04-26.md

| filepath                                   | description       | score | decision                           | new filepath                               | remarks                                                                                                                                        |
| ------------------------------------------ | ----------------- | ----- | ---------------------------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/audits/docs-inventory-2026-04-26.md` | Docs audit ledger | `n/a` | Use the table format going forward | `docs/audits/docs-inventory-2026-04-26.md` | Existing prose entries remain immutable under the append-only rule; new audited docs use one `## <filepath>` heading plus the requested table. |

---

## docs/continuum-foundations.md

| filepath                        | description                                                               | score | decision       | new filepath                                 | remarks                                                                                                                                                                                                        |
| ------------------------------- | ------------------------------------------------------------------------- | ----- | -------------- | -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/continuum-foundations.md` | Draft platform bridge from WARP/observer/optic theory to Continuum repos. | `3/5` | Keep and move. | `docs/architecture/continuum-foundations.md` | Accurate as a clearly labeled architecture intent memo; evidence includes `CONTINUUM.md`, design 0011, and local sibling repos, but several Wesley/`git-warp`/`warp-ttd` proof obligations remain future work. |

---

## docs/dependency-dags.md

| filepath                  | description                                                         | score | decision       | new filepath                     | remarks                                                                                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------- | ----- | -------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/dependency-dags.md` | Explainer for issue, milestone, and task dependency DAG generation. | `5/5` | Keep and move. | `docs/method/dependency-dags.md` | Code-backed by `scripts/generate-dependency-dags.js`, `scripts/generate-tasks-dag.js`, `cargo xtask dags`, `Makefile` targets, DAG assets, and `.github/workflows/refresh-dependency-dags.yml`. |

---

## docs/dind-harness.md

| filepath               | description                                                 | score | decision       | new filepath                       | remarks                                                                                                                                                                                      |
| ---------------------- | ----------------------------------------------------------- | ----- | -------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/dind-harness.md` | Determinism verification runner and DIND scenario workflow. | `3/5` | Keep and move. | `docs/determinism/dind-harness.md` | Useful and code-backed by `crates/echo-dind-harness`, `crates/echo-dind-tests`, `testdata/dind`, and DIND workflows; fixed stale direct CLI examples and overclaiming around FootprintGuard. |

---

## docs/golden-vectors.md

| filepath                 | description                                | score | decision       | new filepath                      | remarks                                                                                                                                                                                    |
| ------------------------ | ------------------------------------------ | ----- | -------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/golden-vectors.md` | ABI canonical-CBOR golden-vector examples. | `2/5` | Keep and move. | `docs/spec/abi-golden-vectors.md` | The CBOR examples are directionally useful, but the old "Phase 1 Frozen" and Rust+JS parity claim overstated current evidence; relabeled as partial and tied to the Rust-side vector test. |

---

## docs/index.md

| filepath        | description                            | score | decision       | new filepath    | remarks                                                                                                                                                                            |
| --------------- | -------------------------------------- | ----- | -------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/index.md` | Live docs map and VitePress home page. | `4/5` | Keep in place. | `docs/index.md` | Still the right root map; updated links for moved Batch 3 docs and added the DIND/ABI-vector surfaces. Residual risk remains in broader docs-site dead links outside the docs map. |

---

## docs/js-cbor-mapping.md

| filepath                  | description                                | score | decision                         | new filepath                   | remarks                                                                                                                                                                                                        |
| ------------------------- | ------------------------------------------ | ----- | -------------------------------- | ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/js-cbor-mapping.md` | JS/TS-to-canonical-CBOR ABI mapping rules. | `4/5` | Keep, move, and lightly correct. | `docs/spec/js-cbor-mapping.md` | Backed by `crates/echo-wasm-abi/src/canonical.rs`, canonical-vector tests, `echo-session-proto` framing, and generated helper names in `packages/wesley-generator-vue`; refreshed stale status/reference text. |

---

## docs/scheduler-performance-warp-core.md

| filepath                                  | description                                              | score | decision       | new filepath                                         | remarks                                                                                                                                                                                   |
| ----------------------------------------- | -------------------------------------------------------- | ----- | -------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/scheduler-performance-warp-core.md` | Benchmark guide for the implemented warp-core scheduler. | `4/5` | Keep and move. | `docs/benchmarks/scheduler-performance-warp-core.md` | Bench files exist at `crates/warp-benches/benches/scheduler_drain.rs` and `scheduler_adversarial.rs`; doc avoids hard timing claims and now points at the moved canonical scheduler spec. |

---

## docs/scheduler-warp-core.md

| filepath                      | description                                                | score | decision                         | new filepath                       | remarks                                                                                                                                                                                              |
| ----------------------------- | ---------------------------------------------------------- | ----- | -------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/scheduler-warp-core.md` | Canonical implemented scheduler semantics for `warp-core`. | `4/5` | Keep, move, and lightly correct. | `docs/spec/scheduler-warp-core.md` | Matches `crates/warp-core/src/scheduler.rs` for `PendingRewrite`, `reserve()`, `GenSet`, `SchedulerKind::Radix`, and drain ordering; removed stale links to missing scheduler map/future spec/notes. |

---

## docs/scheduler.md

| filepath            | description                  | score | decision | new filepath | remarks                                                                                                                                                                                                 |
| ------------------- | ---------------------------- | ----- | -------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/scheduler.md` | Top-level scheduler doc map. | `1/5` | Trash.   | `n/a`        | Duplicated the live docs map, pointed to missing `docs/spec-scheduler.md`, and encouraged multiple scheduler truths. Deleted in favor of `docs/spec/scheduler-warp-core.md` plus `docs/index.md` links. |

---

## docs/spec-canonical-inbox-sequencing.md

| filepath                                  | description                                                     | score | decision                      | new filepath                              | remarks                                                                                                                                                                                                                     |
| ----------------------------------------- | --------------------------------------------------------------- | ----- | ----------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/spec-canonical-inbox-sequencing.md` | Canonical inbox identity, admission order, and sequencing spec. | `3/5` | Keep, move, and mark partial. | `docs/spec/canonical-inbox-sequencing.md` | Content-addressed ingress and append-only queue maintenance are backed by `head_inbox.rs`, `engine_impl.rs`, `inbox.rs`, and tests; priority-class scheduler tie-break remains design guidance, so status now says partial. |

---

## docs/spec-merkle-commit.md

| filepath                     | description                                        | score | decision                      | new filepath                 | remarks                                                                                                                                                                                                                                                              |
| ---------------------------- | -------------------------------------------------- | ----- | ----------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/spec-merkle-commit.md` | Snapshot state-root and commit-hash encoding spec. | `3/5` | Keep, move, and mark partial. | `docs/spec/merkle-commit.md` | Core `state_root`, `commit_id` v2, patch/plan/decision/rewrites digest behavior is backed by `snapshot.rs`, `engine_impl.rs`, receipts, playback/provenance tests, and golden vectors; parent-limit and `admission_digest` claims were corrected as partial/planned. |

---

## docs/spec-runtime-config.md

| filepath                      | description                                         | score | decision | new filepath | remarks                                                                                                                                                                                                                                                                  |
| ----------------------------- | --------------------------------------------------- | ----- | -------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/spec-runtime-config.md` | Planned project-level runtime configuration schema. | `1/5` | Trash.   | `n/a`        | No `echo.config.json` loader, config CLI, schema command, config hash, or `ERR_CONFIG_HASH_MISMATCH` implementation exists; the only implemented `EchoConfig` is sandbox construction config, and the guide now preserves planned fields without promoting a ghost spec. |

---

## docs/spec-warp-core.md

| filepath                 | description                             | score | decision                         | new filepath             | remarks                                                                                                                                                                                                                 |
| ------------------------ | --------------------------------------- | ----- | -------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/spec-warp-core.md` | Broad `warp-core` runtime and API tour. | `3/5` | Keep, move, and lightly correct. | `docs/spec/warp-core.md` | Mostly backed by `lib.rs` exports, `Engine`, `GraphView`, `WarpState`, `tick_patch.rs`, `snapshot.rs`, and Stage B1 tests; stale links, the future entropy pointer, and obsolete footprint example code were corrected. |

---

## docs/spec-warp-tick-patch.md

| filepath                       | description                                                            | score | decision                         | new filepath                   | remarks                                                                                                                                                                                                                                                        |
| ------------------------------ | ---------------------------------------------------------------------- | ----- | -------------------------------- | ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/spec-warp-tick-patch.md` | Tick patch boundary artifact, canonical ops, replay, and slicing spec. | `4/5` | Keep, move, and lightly correct. | `docs/spec/warp-tick-patch.md` | Strongly backed by `WarpTickPatchV1`, `WarpOp`, `SlotId`, canonical sorting/digest code, `apply_to_state`, and slicing tests; corrected links and clarified that stream admission records are future digest design material, not current `patch_digest` input. |

---

## docs/spec-warp-view-protocol.md

| filepath                          | description                                          | score | decision                                                 | new filepath                      | remarks                                                                                                                                                                                                         |
| --------------------------------- | ---------------------------------------------------- | ----- | -------------------------------------------------------- | --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/spec-warp-view-protocol.md` | Retained older Echo-local WARP view stream contract. | `3/5` | Keep, move, and retain as historical/current-proto note. | `docs/spec/warp-view-protocol.md` | Message names and aliases are backed by `echo-session-proto`; the session hub/viewer path is retired as the doc already stated, and the stale 8 MiB payload cap was corrected against current JS-ABI/EINT code. |

---

## docs/BEARING.md

| filepath          | description                                       | score | decision                           | new filepath      | remarks                                                                                                                                                                                                                   |
| ----------------- | ------------------------------------------------- | ----- | ---------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/BEARING.md` | Current direction signpost for Echo docs/runtime. | `4/5` | Keep in place and lightly correct. | `docs/BEARING.md` | Current claims are backed by `neighborhood.rs`, `settlement.rs`, `warp_kernel.rs`, `kernel_port.rs`, SPEC-0009, Method docs, and design 0011; added the stricter live-docs corpus rule and linked it from the docs index. |

---

## docs/warp-math-claims.md

| filepath                   | description                                                                        | score | decision | new filepath | remarks                                                                                                                                                                                                                |
| -------------------------- | ---------------------------------------------------------------------------------- | ----- | -------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/warp-math-claims.md` | Theory note asserting WPP/hypergraph-to-WARP embedding and rulial-distance claims. | `1/5` | Trash.   | `n/a`        | Interesting theory, but not current repo truth: no proof pack, WPP importer, parity demo, or rulial-distance runtime exists. Removed direct links rather than preserving an archaeology/theory duplicate in live docs. |

---

## docs/warp-two-plane-law.md

| filepath                     | description                                                                | score | decision       | new filepath                            | remarks                                                                                                                                                                                                                  |
| ---------------------------- | -------------------------------------------------------------------------- | ----- | -------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/warp-two-plane-law.md` | Implemented law separating WARP skeleton structure from typed attachments. | `5/5` | Keep and move. | `docs/invariants/warp-two-plane-law.md` | Code-backed by `GraphStore`, `AttachmentValue::Atom`, `AttachmentValue::Descend`, `GraphView`, `Footprint`, `WarpTickPatchV1`, `compute_state_root`, and Stage B1 portal/slicing tests; moved into invariants ownership. |

---

## docs/workflows.md

| filepath            | description                                        | score | decision                           | new filepath        | remarks                                                                                                                                                                                                                                                             |
| ------------------- | -------------------------------------------------- | ----- | ---------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/workflows.md` | Contributor workflow and tooling entrypoint index. | `4/5` | Keep in place and lightly correct. | `docs/workflows.md` | Current commands are backed by `Makefile`, `xtask/src/main.rs`, `scripts/verify-local.sh`, and dependency-DAG/DIND workflows; removed stale ADR wording and an empty validation heading, while keeping the path because repo tests and hooks reference it directly. |

---

## Strict doctrine reconciliation after Batch 6

| filepath                                  | description                                           | score | decision                                | new filepath                              | remarks                                                                                                                                                                                                                             |
| ----------------------------------------- | ----------------------------------------------------- | ----- | --------------------------------------- | ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/workflows.md`                       | Contributor workflow and local verification doctrine. | `5/5` | Keep and correct.                       | `docs/workflows.md`                       | Added the hard ban on `cargo test -p <crate> <filter>` and documented the exact allowed alternatives: `--lib`, `--test`, and `cargo xtask test-slice`.                                                                              |
| `docs/spec/warp-view-protocol.md`         | WARP stream packet/message schema.                    | `4/5` | Keep only as current wire-schema truth. | `docs/spec/warp-view-protocol.md`         | Rewrote status and scope around implemented `echo-session-proto` types and packet encoding; removed session-hub, viewer, retired-prototype, backlog, and retained-old-contract language.                                            |
| `docs/spec/canonical-inbox-sequencing.md` | Canonical content-addressed ingress sequencing spec.  | `4/5` | Keep and correct.                       | `docs/spec/canonical-inbox-sequencing.md` | Removed partial status and priority-class scheduler design guidance; kept current `HeadInbox`, `ingress_id`, idempotent admission, and append-only queue-maintenance truth while pointing scheduler ordering to the scheduler spec. |
| `docs/spec/merkle-commit.md`              | Snapshot state-root and commit-hash encoding spec.    | `4/5` | Keep and correct.                       | `docs/spec/merkle-commit.md`              | Removed unimplemented stream-admission digest material; retained the current `Snapshot` digests and commit hash v2 semantics backed by `snapshot.rs`, `engine_impl.rs`, receipt tests, replay/provenance tests, and golden vectors. |
| `docs/spec/warp-core.md`                  | `warp-core` runtime/API tour.                         | `4/5` | Keep and correct.                       | `docs/spec/warp-core.md`                  | Removed the future entropy/event-log bullet from the determinism summary; retained the code-backed tour of `GraphStore`, `WarpState`, attachments, rules, receipts, patches, commit hashing, and Stage B1 portal/slicing behavior.  |
| `docs/guide/configuration-reference.md`   | Implemented engine configuration reference.           | `5/5` | Keep and correct.                       | `docs/guide/configuration-reference.md`   | Removed the planned `echo.config.json` section and old runtime-config-spec residue; the guide now documents only implemented `EngineBuilder`, scheduler, worker, materialization, and protocol-constant knobs.                      |
| `docs/index.md`                           | Live docs map.                                        | `5/5` | Keep and correct touched labels.        | `docs/index.md`                           | Updated WVP wording to current WARP stream wire-schema language; retained the live corpus rule and current map role.                                                                                                                |
| `docs/guide/start-here.md`                | Start-here guide.                                     | `4/5` | Keep and correct touched WVP wording.   | `docs/guide/start-here.md`                | Replaced retired-demo phrasing with current runnable-browser and WARP stream wire-schema pointers.                                                                                                                                  |
| `docs/guide/eli5.md`                      | Newcomer explainer.                                   | `4/5` | Keep and correct touched WVP wording.   | `docs/guide/eli5.md`                      | Replaced retired-demo phrasing with current runnable-browser and WARP stream wire-schema pointers.                                                                                                                                  |
