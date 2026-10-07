<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Study Feedback Roadmap

## Executive summary

This plan addresses four defects confirmed in Echo source. The audit uses commit `a93e9d82e89455ed1fa0b63447c88de544b9da26`.

The state-root API overstates its hash boundary. The operation runner requires an unrelated Git checkout and changes its input directory. Its error hides typed Action outcomes. Recovery repeats replay prefixes and retains each basis state.

These repairs make evidence boundaries clear, make the supplied runner usable, and remove repeated recovery work. They do not complete every proposed Echo feature. The separate Keep sequence below supplies the requested CAS integration plan.

The user requested this checked-in plan. That request overrides the normal policy that live plans exist only in GitHub. GitHub issues and PRs remain the status authority.

The S01 repair landed in [PR #762](https://github.com/flyingrobots/echo/pull/762) at main commit `da929ca6093977e909af20ef918e2431ad9b338c`. GitHub records its final status.

## Source and verification boundary

Source: `FEEDBACK-echo.md`, SHA-256 `a831edf49300065e982f166dae8fd2a801f533c0e88fd8afb3e52108725ccdfa`.

Reader source: `library/causal-computing/2026-10-07-synapse-backend-experiments/originals/FEEDBACK-echo.md`.

The auditor read all 13 feedback sections and the affected Echo code. The original benchmark sources, WAL stores, and raw results are absent. Reported measurements remain unverified. Source inspection confirms mechanisms; it does not reproduce elapsed times.

## Claim decisions

| Item | Decision | Evidence and reason |
| --- | --- | --- |
| 1 | Confirmed documentation defect; S01 | `snapshot.rs:81-158` hashes reachable nodes. `echo_operation.rs:4651-4683` creates a node and attachment without an edge. `worldline_state.rs:249` incorrectly says full-state. The Merkle specification deliberately defines reachable state. Preserve that law. The patch digest binds detached writes. |
| 2 | Confirmed repeated work and retention; S04 | `trusted_runtime_host.rs:4196-4217` caches each basis state. `provenance_store.rs:1053-1077` restores a base and replays its prefix. Validation requests each distinct basis. However, `tick_history` contains snapshots, receipts, and patches, not full graph stores. The proposed snapshot explanation is incorrect. |
| 3 | Confirmed durability mechanism; no demonstrated latency defect | `submit_intent_with_runtime_wal_ack_inner` commits acceptance before return. `causal_wal.rs:5895,7860` syncs the commit file. Tick batching does not batch submission commits. The API already defines this durable ACK contract. The 20 ms figure is unverified. A batch API requires a separate accepted contract. |
| 4 | Unverified measurement; no stated size limit | WAL records retain submission, package, receipt, patch, and decision evidence. The report gives no permitted byte limit. Do not weaken retained evidence to meet an invented ratio. |
| 5 | Confirmed runner defect; S02 | `xtask/src/main.rs:454-461` discovers and enters a Git root before command parsing. The runner consumes supplied artifact paths. |
| 6 | Confirmed runner diagnostic defect; S03 | `run_edict_operation.rs:379-385` discards the outcome. `echo_operation.rs:4760-4765` retains a deterministic ResultProjectionInvalid kind. Expose that kind. New retained reason codes require a separate schema contract. |
| 7 | Confirmed declared codec boundary | `parse_input` checks shape and replacement bytes. README states that generic ingress does not validate codec-owned input schemas. The compiler cannot prove arbitrary caller JSON. Type enforcement belongs in the generated or authored adapter. No generic-core defect is established. |
| 8 | Confirmed bootstrap requirement | `build_host` reconstructs the initial lane. Replay validates the registered initial boundary. WAL docs describe bootstrap sources. A standalone open API is a feature proposal. Do not infer a broken recovery promise. |
| 9 | Broad claim rejected; narrower limitation confirmed | `TrustedRuntimeApp::observe` calls ObservationService. `ObservationAt::Tick` selects past coordinates and returns a reading envelope. `QueryBytes` optics remain unsupported. Bounded Edict setup hashes the frontier, as its current contract states. |
| 10 | Confirmed declared feature gap, already tracked | `WorldlineRuntime::fork_strand` exists. The trusted host has no durable fork crossing. WAL docs declare this limit. Existing issue #605 owns WAL-backed fork/drop work; do not create a duplicate or silently adopt that broader feature. |
| 11 | Confirmed host authority split | Cargo features distinguish internal rule authoring and trusted runtime ownership. Application handles cannot own scheduler authority. The suggested convenience crate is a product proposal. Its unavailable harness does not establish the claimed import count. |
| 12 | Pins confirmed; defect rejected | `hello-echo/producers.lock.json` pins exact producers. Its README requires those revisions and explains how pins advance. A refusal on another main revision enforces reproducibility. No automatic latest-main compatibility promise exists. |
| 13 | Source-reported praise | Existing runner tests cover these mechanisms. The reported 22,100 Actions and timings remain unverified without raw results. |

Code paths are relative to this repository, except the explicitly named hello-echo consumer. All conclusions describe the audited revision.

## Execution sequence

- [x] [S01: Describe the reachable-state boundary of WorldlineState::state_root](tasks/S01.md) — [issue #754](https://github.com/flyingrobots/echo/issues/754)
- [x] [S02: Run xtask run-edict-operation without a Git checkout or directory change](tasks/S02.md) — [issue #755](https://github.com/flyingrobots/echo/issues/755)
- [x] [S03: Report the typed Action obstruction when the operation runner cannot commit](tasks/S03.md) — [issue #756](https://github.com/flyingrobots/echo/issues/756)
- [ ] [S04: Avoid repeated prefix replay and unbounded state retention during Action WAL recovery](tasks/S04.md) — [issue #757](https://github.com/flyingrobots/echo/issues/757)

Execute S01, S02, S03, then S04. Each issue produces one independently mergeable PR. This order puts smaller contract and runner repairs before recovery work.

The feedback-repair graph has four vertices and zero required edges. All four tasks form one antichain at the audited revision. The chosen sequence serializes the shared worker; it does not imply a code prerequisite.

Each task owns its stated defect. S01 owns hash-boundary text. S02 owns command directory behavior. S03 owns runner outcome errors. S04 owns recovery work and retention. No task owns a second task's requirements.

## Keep CAS integration sequence

Use `~/git/keep` as the source project. The local Keep checkout is `001ae2a`; its refreshed `origin/main` is `3165890e9291cfb5fe10e81a9d7cd151f3e59464`. Inspect and pin the actual integration revision. Preserve the existing paused checkout.

[Issue #722](https://github.com/flyingrobots/echo/issues/722) is the integration container. Its six milestones describe the same work as the six task documents below. Do not count the container as another executable PR.

The [physical-content boundary](docs/architecture/echo-keep-physical-content-boundary.md) owns the accepted architecture. Keep supplies physical bytes and receipts. Echo retains content identity, WSC identity, causal history, semantic meaning, and authority. Keep receipts cannot replace Echo observations.

Echo selects Rust 1.96.0 for its toolchain and warp-core. The workspace default and echo-cas still declare Rust 1.90.0. Keep requires Rust 1.96. The older Rust 1.90 statement in #722 is stale. A matching version number does not prove dependency, platform, or durability compatibility.

Keep main exports `DurableStore` and fenced snapshots. Its crate documentation still states that production durable ingestion, garbage collection, and compaction are unimplemented. Thus durable reads and durable publication have different readiness.

- [ ] [K01: Prove the Echo and Keep content identity bridge](tasks/K01.md) — [issue #759](https://github.com/flyingrobots/echo/issues/759)
- [ ] [K02: Add the Echo physical-content port and existing CAS adapters](tasks/K02.md) — [issue #760](https://github.com/flyingrobots/echo/issues/760)
- [ ] [K03: Add an experimental Keep ReferenceStore adapter](tasks/K03.md) — [issue #761](https://github.com/flyingrobots/echo/issues/761)
- [ ] [K04: Add a pinned-generation durable Keep read adapter](tasks/K04.md)
- [ ] [K05: Prove durable Echo and Keep publication reconciliation](tasks/K05.md)
- [ ] [K06: Prepare the migration and production adoption decision](tasks/K06.md)

Execute K01 and K02 as independent slices, then K03. Stop this run after the experimental adapter passes its conformance and review gates.

K04–K06 are conditional follow-on work. K04 requires guarded storage that actually passes Keep's Linux ext4 admission. K05 is blocked on external Keep durable ingestion, operation lookup, and non-expiring retention anchors. K06 also requires crash, migration, rollback, and adoption evidence. Completing K03 does not unblock these requirements.

Proposed edges: K01 → K03; K02 → K03; K03 → K04; K04 → K05; K05 → K06. K01 supplies the identity law. K02 supplies the port. K03 supplies backend conformance. K04 supplies pinned-view receipt validation. K05 supplies durable reconciliation.

No feedback repair requires Keep integration. No Keep integration task requires a feedback repair merely because both touch storage. The shared Docker worker requires serial execution, not a graph edge.

The initial proposed graph has ten task vertices and five internal edges. S01, S02, and S03 are complete. The remaining graph has seven task vertices and five internal edges; its first layer is `{S04,K01,K02}`. Its layers are `{S01,S02,S03,S04,K01,K02}`, `{K03}`, `{K04}`, `{K05}`, `{K06}`. External prerequisite nodes are `keep_durable_ingestion`, `durable_operation_lookup`, `non_expiring_retention_anchor`, and `admitted_guarded_storage`. Each has an unresolved edge into K05. Admitted guarded storage also gates K04. These nodes are capability requirements, not invented tracker issues. The ten-task, five-edge count covers internal task edges only. GitHub records #759 and #760 as blockers of #761 under container #722. Future K04–K06 edges remain proposals until their executable issues exist. Reconcile these tracker edges before execution.

The user authorized K01–K03 for this run: implement the experimental adapter first. K04–K06 remain conditional follow-on work. The initial target is experimental integration. Production adoption requires a separately reviewable accepted decision after the evidence gates pass. Keep must not become the default through an incidental dependency change.

## Solution constraints from independent critique

S03 reports bounded outcome categories. It distinguishes a missing outcome from an obstruction and avoids raw record dumps.

S04 indexes exact obligations before replay. It sorts verification references within each worldline and keeps retained protocol order authoritative. Its two-sweep plan supports stale and cross-worldline bases. Replay counters bound patch applications; they do not prove linear elapsed time.

K01 verifies two distinct identity laws on the same source and reconstructed bytes. Equal source bytes do not make Echo and Keep digest values equal. K02 preserves existing CAS defaults and package compatibility.

The independent critique confirms the S01 contract repair and experimental K01–K03 boundary. Its Reader source is receipt `7e05db3c-49d0-45df-b81b-2f1a6a25a6f7`. Reconciliation details remain on PR #758; this plan contains the resulting requirements.

## Verification and merge rules

Use the existing `echo-read-runtime` worker and stable `/lease-target` cache. Use `/Users/Shared/git-locks/workstation.git` with `host/heavy-work` and `host/docker/echo-read-runtime/`. The monitor also inspects `echo-provider-builder`; reserve its key during validation.

Keep the 20 GiB build, 4 GiB data, and 128 MiB log limits. Keep the 50 GiB host and VM free-space floors. Use copied source, four CPUs, 6 GiB RAM, bounded logs, process deadlines, and the fail-closed guard. Do not use host test fallbacks.

Record before/after evidence for S01. Record executable RED/GREEN for behavior repairs. Use deterministic replay-work counters for S04. Do not claim a timing result from source inspection.

Each PR must pass its relevant tests, linters, hosted required checks, and current-head Code Lawyer and agy reviews. Preserve issue, PR, and merge-commit linkage. Do not amend, rebase, or force-push. The user has authorized ordinary pushes and merges.

STE prose follows the installed ASD-STE100 guide. Full dictionary conformance is unverified.
