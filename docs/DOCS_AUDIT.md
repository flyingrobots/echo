<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Documentation Audit

Conducted 2026-04-03 as part of METHOD adoption.

**Question for each file:** Would this exist if we were writing Echo's docs
from scratch today, based on what the code actually does?

## Legend

| Symbol      | Meaning                                                              |
| ----------- | -------------------------------------------------------------------- |
| **KEEP**    | Accurate, useful, would exist from scratch.                          |
| **REWRITE** | The topic matters but the content is stale, aspirational, or sloppy. |
| **ARCHIVE** | Historical value only. Move to `docs/archive/`.                      |
| **DELETE**  | No value. Remove.                                                    |
| **BACKLOG** | Active concern — migrate to METHOD backlog.                          |

---

## Top-level docs/

| File                         | Rec     | From scratch? | Remarks                                                                                                  |
| ---------------------------- | ------- | ------------- | -------------------------------------------------------------------------------------------------------- |
| `index.md`                   | REWRITE | Yes           | Entry point is good; links will break as we reorganize. Rewrite after restructure.                       |
| `METHODOLOGY.md`             | DELETE  | No            | 5x Duty model was never practiced. METHOD replaces this entirely.                                        |
| `ROADMAP.md`                 | DELETE  | No            | Replaced by METHOD backlog lanes. Migrate active items to backlog.                                       |
| `RELEASE_POLICY.md`          | KEEP    | Yes           | v1.1, clear gates (G1–G4). Enforced.                                                                     |
| `THEORY.md`                  | REWRITE | Maybe         | Large theory doc. Some content is foundational, much is speculative. Needs brutal trim to match reality. |
| `DETERMINISTIC_MATH.md`      | KEEP    | Yes           | Hazard catalog for IEEE 754. Non-normative but accurate.                                                 |
| `SPEC_DETERMINISTIC_MATH.md` | KEEP    | Yes           | Normative deterministic math policy. CI-enforced.                                                        |
| `BENCHMARK_GUIDE.md`         | KEEP    | Yes           | Excellent. Gold standard benchmarking guide.                                                             |
| `architecture-outline.md`    | REWRITE | Maybe         | High-level sketch, lags behind code reality. Many sections planned, not implemented.                     |
| `continuum-foundations.md`   | KEEP    | Yes           | Multi-repo context bridge (Echo, git-warp, Wesley). Current.                                             |
| `dependency-dags.md`         | KEEP    | Yes           | DAG docs and generation. Current.                                                                        |
| `dind-harness.md`            | KEEP    | Yes           | Deterministic Ironclad Nightmare Drills. Current.                                                        |
| `golden-vectors.md`          | KEEP    | Yes           | Determinism lock vectors. Current.                                                                       |
| `js-cbor-mapping.md`         | KEEP    | Yes           | JS-to-CBOR type mapping. Current.                                                                        |
| `workflows.md`               | REWRITE | Yes           | Contributor playbook. Needs update for METHOD workflow.                                                  |
| `warp-math-claims.md`        | KEEP    | Yes           | Math claims complement to hazard catalog.                                                                |
| `warp-two-plane-law.md`      | KEEP    | Yes           | Core invariant: skeleton plane vs attachment plane. Foundational.                                        |
| `march-16.plan.md`           | DELETE  | No            | Stale planning scratchpad.                                                                               |
| `macros.tex`                 | KEEP    | Yes           | LaTeX macros for book.                                                                                   |
| `refs.bib`                   | KEEP    | Yes           | Bibliography.                                                                                            |
| `warp-rulial-distance.tex`   | ARCHIVE | No            | Research artifact. Not part of active docs.                                                              |

## Top-level specs (docs/spec-\*.md)

| File                                 | Rec    | From scratch? | Remarks                                           |
| ------------------------------------ | ------ | ------------- | ------------------------------------------------- |
| `spec-warp-core.md`                  | KEEP   | Yes           | Warp-core crate tour. Stable.                     |
| `spec-warp-tick-patch.md`            | KEEP   | Yes           | Tick patch boundary spec (v2). Precise.           |
| `spec-warp-view-protocol.md`         | KEEP   | Yes           | WVP pub/sub protocol. Boring and correct.         |
| `spec-merkle-commit.md`              | KEEP   | Yes           | Snapshot commit spec (v2). Deterministic.         |
| `spec-canonical-inbox-sequencing.md` | KEEP   | Yes           | Inbox ordering spec.                              |
| `spec-runtime-config.md`             | KEEP   | Yes           | Runtime config.                                   |
| `scheduler.md`                       | KEEP   | Yes           | Doc map clarifying two scheduler concepts.        |
| `scheduler-warp-core.md`             | KEEP   | Yes           | Rewrite scheduler spec.                           |
| `scheduler-performance-warp-core.md` | KEEP   | Yes           | Scheduler benchmarks.                             |
| `spec-scheduler.md`                  | DELETE | No            | Future ECS scheduler. Unimplemented, speculative. |
| `spec-branch-tree.md`                | DELETE | No            | Future branching spec. Unimplemented.             |
| `spec-editor-and-inspector.md`       | DELETE | No            | Future tooling. Unimplemented.                    |
| `spec-entropy-and-paradox.md`        | DELETE | No            | Future paradox handling. Unimplemented.           |
| `spec-knots-in-time.md`              | DELETE | No            | Future time knots. Unimplemented.                 |
| `spec-temporal-bridge.md`            | DELETE | No            | Future temporal bridge. Unimplemented.            |
| `spec-time-streams-and-wormholes.md` | DELETE | No            | Future worldline comms. Unimplemented.            |
| `spec-timecube.md`                   | DELETE | No            | Future time model. Unimplemented.                 |
| `spec-warp-confluence.md`            | DELETE | No            | Future merge semantics. Unimplemented.            |
| `spec-ecs-storage.md`                | DELETE | No            | Future ECS storage. Unimplemented.                |
| `spec-concurrency-and-authoring.md`  | DELETE | No            | Future multi-author concurrency. Unimplemented.   |
| `spec-world-api.md`                  | DELETE | No            | Future world API. Unimplemented.                  |
| `spec-mwmr-concurrency.md`           | DELETE | No            | Future MWMR concurrency. Unimplemented.           |
| `spec-serialization-protocol.md`     | DELETE | No            | Partial. Not enforced.                            |
| `spec-capabilities-and-security.md`  | DELETE | No            | Future capability system. Unimplemented.          |
| `spec-networking.md`                 | DELETE | No            | Future networking. Unimplemented.                 |
| `spec-plugin-system.md`              | DELETE | No            | Future plugin system. Unimplemented.              |

## docs/spec/ (numbered specs)

| File                                        | Rec  | From scratch? | Remarks                                    |
| ------------------------------------------- | ---- | ------------- | ------------------------------------------ |
| `SPEC-0001-attachment-plane-v0-atoms.md`    | KEEP | Yes           | Attachment plane v0. Implemented.          |
| `SPEC-0002-descended-attachments-v1.md`     | KEEP | Yes           | Descended attachments. Implemented.        |
| `SPEC-0003-dpo-concurrency-litmus-v0.md`    | KEEP | Yes           | DPO concurrency litmus tests. Implemented. |
| `SPEC-0004-worldlines-playback-truthbus.md` | KEEP | Yes           | Provenance infrastructure. Implemented.    |
| `SPEC-0005-provenance-payload.md`           | KEEP | Yes           | Provenance payload. Implemented.           |
| `SPEC-0009-wasm-abi-v3.md`                  | KEEP | Yes           | WASM ABI v3. Implemented.                  |

## docs/adr/

| File                              | Rec     | Remarks                                                                |
| --------------------------------- | ------- | ---------------------------------------------------------------------- |
| All 11 ADRs (0001–0011)           | ARCHIVE | Historical decisions. Valuable as record, but not active process docs. |
| `adr-exceptions.md`               | ARCHIVE | Exception ledger (currently empty).                                    |
| `PLAN-PHASE-6B-VIRTUAL-SHARDS.md` | ARCHIVE | Phase 6B complete. Status report, not a plan.                          |
| `TECH-DEBT-BOAW.md`               | BACKLOG | Active tech debt. Migrate items to `bad-code/` lane.                   |

## docs/plans/

| File                                            | Rec     | Remarks                                                    |
| ----------------------------------------------- | ------- | ---------------------------------------------------------- |
| `adr-0008-and-0009.md`                          | BACKLOG | Living plan, phases 0–8 complete. Phase 9+ work → backlog. |
| `parallel-merge-and-footprint-design-review.md` | ARCHIVE | Design review with conclusions. Reference value.           |
| `parallel-merge-and-footprint-optimizations.md` | DELETE  | Superseded by design review.                               |
| `phase-8-runtime-schema-conformance.md`         | ARCHIVE | Locked audit for phase 8.                                  |
| `phase-8-runtime-schema-mapping-contract.md`    | ARCHIVE | Locked contract for phase 8.                               |
| `phase-8-schema-freeze-inventory.md`            | ARCHIVE | Locked inventory for phase 8.                              |

## docs/ROADMAP/ (37 milestone files)

| Directory                          | Rec     | Remarks                                               |
| ---------------------------------- | ------- | ----------------------------------------------------- |
| `lock-the-hashes/` (2 items)       | BACKLOG | P0, verified. Migrate to `asap/`.                     |
| `developer-cli/` (5 items)         | BACKLOG | P0, verified. Migrate to `asap/`.                     |
| `proof-core/` (3 items)            | BACKLOG | P1, in progress. Migrate to `asap/`.                  |
| `time-semantics-lock/` (1 item)    | BACKLOG | P1, planned. Migrate to `up-next/`.                   |
| `first-light/` (9 items)           | BACKLOG | P2, planned. Migrate to `up-next/`.                   |
| `backlog/` (12 items)              | BACKLOG | Unscheduled. Migrate to `inbox/` or appropriate lane. |
| `time-travel/` (3 items)           | BACKLOG | P3, planned. Migrate to `cool-ideas/`.                |
| `proof-time-convergence/` (1 item) | BACKLOG | P3, planned. Migrate to `cool-ideas/`.                |
| `splash-guy/` (5 items)            | BACKLOG | P3, planned. Migrate to `cool-ideas/`.                |
| `tumble-tower/` (8 items)          | BACKLOG | P3, planned. Migrate to `cool-ideas/`.                |
| `deep-storage/` (4 items)          | BACKLOG | P3, planned. Migrate to `cool-ideas/`.                |

## docs/guide/

| File                         | Rec     | From scratch? | Remarks                                                                         |
| ---------------------------- | ------- | ------------- | ------------------------------------------------------------------------------- |
| `start-here.md`              | REWRITE | Yes           | Good entry point but links will need updating.                                  |
| `eli5.md`                    | KEEP    | Yes           | Accessible, accurate.                                                           |
| `warp-primer.md`             | KEEP    | Yes           | Two-plane WARP intro. Stable.                                                   |
| `cargo-features.md`          | KEEP    | Yes           | Feature flags.                                                                  |
| `configuration-reference.md` | KEEP    | Yes           | Runtime config reference.                                                       |
| `splash-guy.md`              | KEEP    | Maybe         | Demo spec. P3 but still describes real intent.                                  |
| `tumble-tower.md`            | KEEP    | Maybe         | Demo spec. P3.                                                                  |
| `wvp-demo.md`                | KEEP    | Yes           | WVP demo walkthrough.                                                           |
| `course/` (5 files)          | REWRITE | Maybe         | Course material exists but incomplete. Worth keeping if course is still a goal. |

## docs/architecture/

| File                                              | Rec  | Remarks                        |
| ------------------------------------------------- | ---- | ------------------------------ |
| `TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md` | KEEP | Clear terminology definitions. |

## docs/determinism/

| File                         | Rec  | Remarks                                  |
| ---------------------------- | ---- | ---------------------------------------- |
| `DETERMINISM_CLAIMS_v0.1.md` | KEEP | Executive determinism claims. CI-backed. |
| `CLAIM_MAP.yaml`             | KEEP | Machine-readable claims registry.        |
| `sec-claim-map.json`         | KEEP | Security claims (CBOR decoder).          |

## docs/benchmarks/

| File        | Rec  | Remarks                               |
| ----------- | ---- | ------------------------------------- |
| All 4 files | KEEP | Benchmark data and analysis. Current. |

## docs/procedures/

| File                           | Rec     | Remarks                     |
| ------------------------------ | ------- | --------------------------- |
| `PR-SUBMISSION-REVIEW-LOOP.md` | REWRITE | Update for METHOD workflow. |
| `EXTRACT-PR-COMMENTS.md`       | KEEP    | Utility procedure.          |
| `ISSUE-DEPENDENCIES.md`        | KEEP    | Issue dep tracking.         |

## docs/audits/

| File                                    | Rec  | Remarks                                 |
| --------------------------------------- | ---- | --------------------------------------- |
| `echo-report-2026-04-01.md`             | KEEP | Honest grade-C editorial audit. Recent. |
| `echo-docs-editorial-cut-2026-04-01.md` | KEEP | Companion findings.                     |

## docs/book/

| File            | Rec     | Remarks                                                                                  |
| --------------- | ------- | ---------------------------------------------------------------------------------------- |
| All LaTeX files | ARCHIVE | Book structure exists, content incomplete. Move to archive until there's a cycle for it. |

## docs/man/

| File            | Rec  | Remarks                     |
| --------------- | ---- | --------------------------- |
| All 4 man pages | KEEP | CLI documentation. Current. |

## docs/assets/, docs/public/

| Dir                        | Rec  | Remarks      |
| -------------------------- | ---- | ------------ |
| `assets/dags/`             | KEEP | DAG configs. |
| `assets/wvp/`              | KEEP | WVP assets.  |
| `public/assets/collision/` | KEEP | Demo assets. |

## docs/archive/

| Dir      | Rec  | Remarks           |
| -------- | ---- | ----------------- |
| `study/` | KEEP | Already archived. |

## docs/.obsidian/

| Dir | Rec  | Remarks                 |
| --- | ---- | ----------------------- |
| All | KEEP | Vault config. Harmless. |

---

## Summary

| Action  | Count | Notes                                                   |
| ------- | ----- | ------------------------------------------------------- |
| KEEP    | ~55   | Core specs, guides, determinism, benchmarks, procedures |
| REWRITE | ~8    | Entry points, theory, architecture, workflows, course   |
| ARCHIVE | ~30   | ADRs, book, old plans, research artifacts               |
| DELETE  | ~20   | Unimplemented future specs, stale plans, 5x methodology |
| BACKLOG | ~40   | ROADMAP items → METHOD backlog lanes                    |

The docs corpus is roughly 50% signal, 25% historical, and 25% fiction
(specs for things that don't exist). METHOD adoption is the right time to
separate what Echo _is_ from what Echo _might become_.
