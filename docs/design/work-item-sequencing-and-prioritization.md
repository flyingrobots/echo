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

Before executing any listed item, verify whether the backing GitHub Issue is
still current. If code has already landed, convert the item into closure, doc
update, or release-gate verification work instead of reimplementing it.

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

- [#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521)
- [#522 WSC Causal-History Storage](https://github.com/flyingrobots/echo/issues/522)
- [#519 Retained Evidence Durability Boundary](https://github.com/flyingrobots/echo/issues/519)

`#521` is the active release-doctrine checkpoint before the next implementation
batch. Its narrow purpose is to keep durable claims honest:

- WAL bytes are the durable commit authority;
- WARP graph WAL nodes are projected evidence facts;
- WSC carries or references that evidence;
- recovery bootstraps from WAL root or storage manifest material, not from
  pre-existing graph facts.

## Current Chunk

The first ten-slice jedit/Echo release-gate batch is complete. The current
chunk is a release-doctrine reconciliation before more release-surface work
lands:

```text
[##########] Echo/jedit retained-evidence release-gate batch [10/10 slices]
[#---------] WAL/WSC release-doctrine checkpoint [1/3 slices]
```

Finish the checkpoint in this order:

| Seq | Band | Package                                 | Source                                                  | Outcome                                                                                                                                       |
| --: | ---- | --------------------------------------- | ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
|   1 | 0    | WAL/WSC doctrine links and lint witness | [#521](https://github.com/flyingrobots/echo/issues/521) | BEARING, WorkItems, sequencing, and WAL design agree on WAL authority, graph projection, WSC modes, storage locators, and bootstrap recovery. |
|   2 | 0    | WSC causal-history export evidence plan | [#522](https://github.com/flyingrobots/echo/issues/522) | Export posture distinguishes ref-only, self-contained, and CAS-addressed WSC without making raw paths causal identity.                        |
|   3 | 0    | Retained evidence durability boundary   | [#519](https://github.com/flyingrobots/echo/issues/519) | Durable retention claims are tied to WAL/checkpoint/CAS evidence; missing material stays typed obstruction.                                   |

The first checkpoint is docs and witness only. It should not modify runtime WAL
behavior; it makes the already-landed doctrine mechanically visible before the
next durability implementation slice.

## Release-Proof Sequence

This sequence is sorted by recommended execution, not by current folder or
issue number.

| Seq | Band | Work package                                                   | Source                                                                                                                                                                                                                                                                                      | Why this spot                                                                                                                |
| --: | ---- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
|   1 | 0    | Slice 4: `jedit` adapter consumes Echo retained refs           | Next-ten plan                                                                                                                                                                                                                                                                               | Unlocks the retained-evidence integration path already in progress.                                                          |
|   2 | 0    | Slice 5: `jedit` replay witness shell                          | Next-ten plan                                                                                                                                                                                                                                                                               | Establishes replay/witness mechanics needed for proof, debugging, and release credibility.                                   |
|   3 | 0    | Slice 6 and product-facing intent outcome API                  | Next-ten plan; [#517 Product-Facing Intent Outcome API](https://github.com/flyingrobots/echo/issues/517)                                                                                                                                                                                    | Tightens the app-facing contract before more integration complexity lands.                                                   |
|   4 | 0    | Slice 7: `jedit` generated structural-history request path     | Next-ten plan                                                                                                                                                                                                                                                                               | Advances the external-app proof from retained refs toward product-owned generated request construction.                      |
|   5 | 0    | Contract retention, WAL/WSC, and semantic lookup proof package | [#512 Contract Artifact Retention In echo-cas](https://github.com/flyingrobots/echo/issues/512); [#513 Contract Retention And Semantic Lookup Seams](https://github.com/flyingrobots/echo/issues/513); [#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521) | Makes retained evidence discoverable without overclaiming durability or confusing WAL storage locators with causal identity. |
|   6 | 0    | Slice 8: generated package install path                        | Next-ten plan                                                                                                                                                                                                                                                                               | Proves generated-path realism and reduces toy-demo risk.                                                                     |
|   7 | 0    | External proof fixture and app-safe client surface             | [#514 External Contract Proof Fixture](https://github.com/flyingrobots/echo/issues/514); [#511 App-Safe Client Surface](https://github.com/flyingrobots/echo/issues/511)                                                                                                                    | Keeps the external app on bounded APIs instead of privileged internals.                                                      |
|   8 | 0    | Slice 9: `jedit` real mutation and query round trip            | Next-ten plan                                                                                                                                                                                                                                                                               | Decisive real-app interaction proof.                                                                                         |
|   9 | 1    | Contract-aware receipts/readings and bounded reading identity  | [#507 Contract-Aware Receipts And Readings](https://github.com/flyingrobots/echo/issues/507); [#509 Contract Reading Identity And Bounded Payloads](https://github.com/flyingrobots/echo/issues/509)                                                                                        | Strengthens what the app can prove it wrote and read.                                                                        |
|  10 | 1    | Thin release-grade quickstart draft                            | [#506 Release-Grade Quickstart](https://github.com/flyingrobots/echo/issues/506)                                                                                                                                                                                                            | Quickstarts expose integration drift early; final polish can wait for RC.                                                    |
|  11 | 1    | Witnessed submission persistence closure                       | [#510 Witnessed Intent Submission Persistence](https://github.com/flyingrobots/echo/issues/510)                                                                                                                                                                                             | Reconcile older persistence wording with landed WAL-backed ACK work before claiming release durability.                      |
|  12 | 1    | Reference trusted runtime host loop closure                    | [#518 Reference Trusted Runtime Host Loop](https://github.com/flyingrobots/echo/issues/518)                                                                                                                                                                                                 | Confirms trusted host behavior without handing scheduler authority to application code.                                      |
|  13 | 1    | Contract obstruction taxonomy                                  | [#508 Contract Obstruction Taxonomy](https://github.com/flyingrobots/echo/issues/508)                                                                                                                                                                                                       | Makes non-happy-path release evidence legible.                                                                               |
|  14 | 1    | Slice 10: non-happy path and release-gate report               | Next-ten plan                                                                                                                                                                                                                                                                               | Closes the active ten-slice release-proof batch.                                                                             |
|  15 | 1    | `jedit` real Echo release gate                                 | [#515 jedit Real Echo Release Gate](https://github.com/flyingrobots/echo/issues/515)                                                                                                                                                                                                        | Converts the proof from an ad hoc demo into a durable gate.                                                                  |
|  16 | 1    | Versioned contract and API compatibility                       | [#520 Versioned Contract And API Compatibility](https://github.com/flyingrobots/echo/issues/520)                                                                                                                                                                                            | Lowers generated-contract drift before release-candidate pressure.                                                           |
|  17 | 1    | CI det-policy hardening                                        | [#394 CI det-policy hardening](https://github.com/flyingrobots/echo/issues/394)                                                                                                                                                                                                             | Finishes current CI-hardening follow-ups after issue `#285` closure.                                                         |
|  18 | 1    | Decoder negative-test map                                      | [#398 Explicit negative test mapping for decoder controls](https://github.com/flyingrobots/echo/issues/398)                                                                                                                                                                                 | Completes security evidence around malformed input controls.                                                                 |
|  19 | 2    | Authority boundary audit                                       | [#525 Authority Boundary Audit](https://github.com/flyingrobots/echo/issues/525)                                                                                                                                                                                                            | Must run after concrete app-safe surfaces exist, before RC.                                                                  |
|  20 | 2    | Replay/DIND proof and release-candidate readiness              | [#526 v0.1.0 Replay And DIND Proof](https://github.com/flyingrobots/echo/issues/526); [#524 v0.1.0 Release Candidate](https://github.com/flyingrobots/echo/issues/524)                                                                                                                      | Consolidates determinism and replay claims into a repeatable release proof.                                                  |

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
