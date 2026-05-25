<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Work Item Sequencing And Prioritization

Status: active sequencing guide.

Last updated: 2026-05-25.

This guide adopts the work-item audit in [`docs/WorkItems.md`](../WorkItems.md)
as an inventory snapshot and turns it into an execution filter. It is not a
mandate to complete every historical backlog item before release. Work moves up
only when it shortens the path to a real `jedit`-on-Echo proof or hardens
evidence, replay, retention, authority, compatibility, or release
reproducibility around that proof.

The current gravity remains:

```text
prove Echo with jedit as a real external app
without moving app nouns into Echo
and without giving application code tick, WAL, or trusted runtime authority
```

The immediate release-proof plan is
[`docs/design/v0.1.0-jedit-next-ten-slices.md`](v0.1.0-jedit-next-ten-slices.md).
The broader release feature bar is
[`docs/design/v0.1.0-release-plan.md`](v0.1.0-release-plan.md).
The active inventory snapshot is [`docs/WorkItems.md`](../WorkItems.md).

## Strategy

Use five execution bands:

| Band | Meaning                                       | Planning intent                                                                              |
| ---: | --------------------------------------------- | -------------------------------------------------------------------------------------------- |
|    0 | In-flight release-proof slices                | Finish the narrow proof path before broadening scope.                                        |
|    1 | Hard blockers for credible external-app proof | Pull in only what unlocks jedit, evidence, retention, app-safe API, or authority boundaries. |
|    2 | Release-candidate hardening and audit closure | Convert working paths into repeatable proof, docs, CI, and compatibility gates.              |
|    3 | Near-follow-on platform work                  | Start after the release-proof path stabilizes.                                               |
|    4 | Inbox, debt, demos, and ideas                 | Preserve visibility without stealing execution energy.                                       |

Priority rules, in order:

1. Prefer work that shortens the path to a real `jedit`-on-Echo proof.
2. Prefer work that makes evidence, retention, replay, and contract boundaries
   observable and testable.
3. Prefer work that tightens authority boundaries and app-safe APIs before
   adding runtime surface area.
4. Prefer work that turns design claims into executable release gates,
   fixtures, and proof artifacts.
5. Defer broad platform ambition, migration work, browser work, demo courses,
   and idea-lot issues until the external-app proof is credible.

Before executing any listed item, verify whether the backing card is still
current. If code has already landed, convert the item into closure, doc update,
or release-gate verification work instead of reimplementing it.

## Storage Doctrine Gap

The audit surfaced one release-bar storage gap that should stay visible while
the `jedit` proof proceeds:

```text
WAL bytes are the durable commit authority.
WARP graph facts track WAL segment evidence.
WSC serializes graph facts and may bundle or reference WAL bytes.
```

The WARP graph may contain WAL evidence nodes, but those nodes are projected
facts, not recovery bootstrap authority. Echo recovery must be able to start
from a configured WAL root or storage manifest, validate committed segments,
and rebuild the graph/read-model/index facts. A graph-projected WAL filepath is
a storage locator, not causal identity. Causal identity comes from writer
epoch, LSN range, segment digest, commit digest chain, and validated commit
anchors.

The doctrine is tracked by:

- [`PLATFORM_wal-wsc-storage-relationship.md`](../method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md)
- [`PLATFORM_wsc-causal-history-storage.md`](../method/backlog/v0.1.0/PLATFORM_wsc-causal-history-storage.md)
- [`PLATFORM_retained-evidence-durability-boundary.md`](../method/backlog/v0.1.0/PLATFORM_retained-evidence-durability-boundary.md)

## First Chunk

The first chunk is the active release-proof package:

```text
[###-------] Echo/jedit retained-evidence release-gate batch [3/10 slices]
[###-------] PR checkpoint batch [3/5 slices before recommended PR]
```

Finish the package in this order:

| Seq | Band | Package                                               | Source                        | Outcome                                                                                                                          |
| --: | ---- | ----------------------------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
|   1 | 0    | Slice 4: `jedit` adapter consumes Echo retained refs  | Next-ten plan                 | `jedit` reads retained evidence from the adapter-projected Echo envelope instead of inventing reading refs in the witness layer. |
|   2 | 0    | Slice 5: `jedit` replay witness shell                 | Next-ten plan                 | Minimal replay witness shell proves the external-app evidence path can be rerun and inspected.                                   |
|   3 | 0    | Slice 6 plus product-facing outcome API               | Next-ten plan and v0.1.0 lane | Echo exposes a stable app-facing intent outcome shape without exposing scheduler or tick authority.                              |
|   4 | 0    | Slice 7: generated structural-history request path    | Next-ten plan                 | `jedit` moves from retained-ref plumbing toward generated, product-owned request construction.                                   |
|   5 | 0    | Retention, WAL/WSC, and semantic lookup proof package | v0.1.0 lane                   | Release-grade retained-evidence posture and lookup seams exist, with durable claims only where WAL/WSC-backed evidence exists.   |

Open a PR after Slice 5 unless Slice 4 or Slice 5 exposes an API boundary that
should land independently.

## Release-Proof Sequence

This sequence is sorted by recommended execution, not by current folder or
issue number.

| Seq | Band | Work package                                                   | Source                                                                                                                                                                                                                                                                                                                                                                                                  | Why this spot                                                                                                                |
| --: | ---- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
|   1 | 0    | Slice 4: `jedit` adapter consumes Echo retained refs           | Next-ten plan                                                                                                                                                                                                                                                                                                                                                                                           | Unlocks the retained-evidence integration path already in progress.                                                          |
|   2 | 0    | Slice 5: `jedit` replay witness shell                          | Next-ten plan                                                                                                                                                                                                                                                                                                                                                                                           | Establishes replay/witness mechanics needed for proof, debugging, and release credibility.                                   |
|   3 | 0    | Slice 6 and product-facing intent outcome API                  | Next-ten plan; [`PLATFORM_product-facing-intent-outcome-api.md`](../method/backlog/v0.1.0/PLATFORM_product-facing-intent-outcome-api.md)                                                                                                                                                                                                                                                                | Tightens the app-facing contract before more integration complexity lands.                                                   |
|   4 | 0    | Slice 7: `jedit` generated structural-history request path     | Next-ten plan                                                                                                                                                                                                                                                                                                                                                                                           | Advances the external-app proof from retained refs toward product-owned generated request construction.                      |
|   5 | 0    | Contract retention, WAL/WSC, and semantic lookup proof package | [`PLATFORM_contract-artifact-retention-in-echo-cas.md`](../method/backlog/v0.1.0/PLATFORM_contract-artifact-retention-in-echo-cas.md); [`PLATFORM_contract-retention-and-semantic-lookup-seams.md`](../method/backlog/v0.1.0/PLATFORM_contract-retention-and-semantic-lookup-seams.md); [`PLATFORM_wal-wsc-storage-relationship.md`](../method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md) | Makes retained evidence discoverable without overclaiming durability or confusing WAL storage locators with causal identity. |
|   6 | 0    | Slice 8: generated package install path                        | Next-ten plan                                                                                                                                                                                                                                                                                                                                                                                           | Proves generated-path realism and reduces toy-demo risk.                                                                     |
|   7 | 0    | External proof fixture and app-safe client surface             | [`PLATFORM_external-contract-proof-fixture.md`](../method/backlog/v0.1.0/PLATFORM_external-contract-proof-fixture.md); [`PLATFORM_app-safe-client-surface.md`](../method/backlog/v0.1.0/PLATFORM_app-safe-client-surface.md)                                                                                                                                                                            | Keeps the external app on bounded APIs instead of privileged internals.                                                      |
|   8 | 0    | Slice 9: `jedit` real mutation and query round trip            | Next-ten plan                                                                                                                                                                                                                                                                                                                                                                                           | Decisive real-app interaction proof.                                                                                         |
|   9 | 1    | Contract-aware receipts/readings and bounded reading identity  | [`KERNEL_contract-aware-receipts-and-readings.md`](../method/backlog/v0.1.0/KERNEL_contract-aware-receipts-and-readings.md); [`KERNEL_contract-reading-identity-and-bounded-payloads.md`](../method/backlog/v0.1.0/KERNEL_contract-reading-identity-and-bounded-payloads.md)                                                                                                                            | Strengthens what the app can prove it wrote and read.                                                                        |
|  10 | 1    | Thin release-grade quickstart draft                            | [`DOCS_release-grade-quickstart.md`](../method/backlog/v0.1.0/DOCS_release-grade-quickstart.md)                                                                                                                                                                                                                                                                                                         | Quickstarts expose integration drift early; final polish can wait for RC.                                                    |
|  11 | 1    | Witnessed submission persistence closure                       | [`KERNEL_witnessed-intent-submission-persistence.md`](../method/backlog/v0.1.0/KERNEL_witnessed-intent-submission-persistence.md)                                                                                                                                                                                                                                                                       | Reconcile older persistence wording with landed WAL-backed ACK work before claiming release durability.                      |
|  12 | 1    | Reference trusted runtime host loop closure                    | [`PLATFORM_reference-trusted-runtime-host-loop.md`](../method/backlog/v0.1.0/PLATFORM_reference-trusted-runtime-host-loop.md)                                                                                                                                                                                                                                                                           | Confirms trusted host behavior without handing scheduler authority to application code.                                      |
|  13 | 1    | Contract obstruction taxonomy                                  | [`KERNEL_contract-obstruction-taxonomy.md`](../method/backlog/v0.1.0/KERNEL_contract-obstruction-taxonomy.md)                                                                                                                                                                                                                                                                                           | Makes non-happy-path release evidence legible.                                                                               |
|  14 | 1    | Slice 10: non-happy path and release-gate report               | Next-ten plan                                                                                                                                                                                                                                                                                                                                                                                           | Closes the active ten-slice release-proof batch.                                                                             |
|  15 | 1    | `jedit` real Echo release gate                                 | [`PLATFORM_jedit-real-echo-release-gate.md`](../method/backlog/v0.1.0/PLATFORM_jedit-real-echo-release-gate.md)                                                                                                                                                                                                                                                                                         | Converts the proof from an ad hoc demo into a durable gate.                                                                  |
|  16 | 1    | Versioned contract and API compatibility                       | [`PLATFORM_versioned-contract-api-compatibility.md`](../method/backlog/v0.1.0/PLATFORM_versioned-contract-api-compatibility.md)                                                                                                                                                                                                                                                                         | Lowers generated-contract drift before release-candidate pressure.                                                           |
|  17 | 1    | CI det-policy hardening                                        | [`PLATFORM_ci-det-policy-hardening.md`](../method/backlog/asap/PLATFORM_ci-det-policy-hardening.md)                                                                                                                                                                                                                                                                                                     | Finishes current CI-hardening follow-ups after issue `#285` closure.                                                         |
|  18 | 1    | Decoder negative-test map                                      | [`PLATFORM_decoder-negative-test-map.md`](../method/backlog/asap/PLATFORM_decoder-negative-test-map.md)                                                                                                                                                                                                                                                                                                 | Completes security evidence around malformed input controls.                                                                 |
|  19 | 2    | Authority boundary audit                                       | [`SECURITY_authority-boundary-audit.md`](../method/backlog/v0.1.0/SECURITY_authority-boundary-audit.md)                                                                                                                                                                                                                                                                                                 | Must run after concrete app-safe surfaces exist, before RC.                                                                  |
|  20 | 2    | Replay/DIND proof and release-candidate readiness              | [`TEST_v0.1.0-replay-dind-proof.md`](../method/backlog/v0.1.0/TEST_v0.1.0-replay-dind-proof.md); [`RELEASE_v0.1.0-release-candidate.md`](../method/backlog/v0.1.0/RELEASE_v0.1.0-release-candidate.md)                                                                                                                                                                                                  | Consolidates determinism and replay claims into a repeatable release proof.                                                  |

## Post-Proof Expansion

After the release-proof sequence stabilizes, pull from these areas in order:

1. Admission outcome family and bounded admission policy.
2. Installed/compiled contract hosting roadmap and doctrine cleanup.
3. TTD protocol schema reconciliation and rollback playbooks.
4. Continuum admission/proof-family cutovers.
5. Echo CLI/MCP agent surfaces.
6. Browser/WASM surface expansion.
7. Demo/course and visualization ideas.

## Guardrails

- Do not treat this sequence as a reason to reimplement already-landed code.
- Do not overclaim durable retention where only missing-retention posture
  exists.
- Do not let `jedit` product nouns enter Echo core.
- Do not expose tick, WAL append, package install, scheduler, or trusted
  recovery authority to application code.
- Do not close old GitHub issues merely because they are old; close them only
  when the corresponding card, code path, or design claim is demonstrably
  complete.
- Do not make WARP graph WAL nodes the recovery bootstrap. They are projected
  evidence facts over WAL-backed history, not the authority that makes the WAL
  recoverable.

## Operating Cadence

- Sprint A: finish slices 4-6, including product-facing outcome API and the
  retained-evidence handoff.
- Sprint B: finish slices 7-9, including proof fixture and app-safe evidence
  seams.
- Sprint C: finish slice 10, compatibility, authority audit, replay proof, and
  quickstart hardening.
- Sprint D: cut release-candidate evidence, then selectively promote ASAP
  hosting and schema work.

Every sprint should end with executable proof: tests, release-gate output,
golden artifacts, or a documented audit. Design claims without witnesses do not
count as closure.
