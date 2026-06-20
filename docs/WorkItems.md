<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Work Items

Last audited: 2026-06-20.

This is an inventory, not a replacement for the repo's planning system. When
there is disagreement, prefer the specific backlog card, design packet, issue,
or executable test over this summary.

Sources checked during this audit:

- [`docs/BEARING.md`](BEARING.md)
- [`docs/method/backlog/`](method/backlog/)
- [`backlog/`](../backlog/)
- open GitHub issues in `flyingrobots/echo`

## Summary

| Source                 | Open count/status | Notes                                                                   |
| ---------------------- | ----------------- | ----------------------------------------------------------------------- |
| `docs/method/backlog/` | legacy marker     | Contains only `.gitkeep`; live backlog moved to GitHub Issues.          |
| `backlog/bad-code/`    | 5                 | Older RE-series debt cards still present.                               |
| `backlog/cool-ideas/`  | 3                 | Older CI-series idea cards still present.                               |
| GitHub open issues     | live              | Run `gh issue list --repo flyingrobots/echo --state open --limit 1000`. |

## Current Execution Gravity

The active direction remains:

```text
prove Echo with jedit as a real external app
without moving app nouns into Echo
and without giving application code tick, WAL, or trusted runtime authority
```

Current active signposts:

- Echo's release feature bar:
  [`docs/design/v0.1.0-release-plan.md`](design/v0.1.0-release-plan.md)
- sequencing and prioritization filter:
  [`docs/design/work-item-sequencing-and-prioritization.md`](design/work-item-sequencing-and-prioritization.md)
- jedit external release gate:
  [`docs/design/v0.1.0-jedit-release-gate.md`](design/v0.1.0-jedit-release-gate.md)
- next ten jedit/Echo release-gate slices:
  [`docs/design/v0.1.0-jedit-next-ten-slices.md`](design/v0.1.0-jedit-next-ten-slices.md)
- causal WAL doctrine:
  [`docs/design/causal-wal-end-to-end.md`](design/causal-wal-end-to-end.md)

Progress bars from the current work stream:

```text
[##########] Echo/jedit retained-evidence release-gate batch [10/10 slices]
[##########] PR checkpoint batch [10/10 slices before next PR]
[##########] Echo WAL truth boundary and runtime ACK plumbing [95/95 slices]
```

Current batch status: complete; open paired Echo and jedit PRs before starting
the next implementation batch.

## Known Cross-Repo And Storage Doctrine Gaps

This inventory is Echo-local unless a row explicitly names another repository.
The following mission-critical gaps were not fully represented by the current
backlog lanes when this audit started:

- `[Echo][jedit]` WSC causal-history persistence, export, and recovery.
- `[Echo]` WAL/WSC storage relationship and recovery authority.
- `[warp-ttd][Echo]` WAL-backed causal commit evidence read model.
- `[Echo]` JS/WASM/browser client release surface.
- `[Echo][Wesley]` package publish, generated package compatibility, and
  versioning.
- `[Echo][Graft][Think]` post-jedit application portability checklist.
- `[Echo]` retained evidence posture versus durable recovery evidence.

The Echo-owned follow-up cards are now GitHub Issues in the release lane:

- [#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521)
- [#522 WSC Causal-History Storage](https://github.com/flyingrobots/echo/issues/522)
- [#519 Retained Evidence Durability Boundary](https://github.com/flyingrobots/echo/issues/519)
- [#516 JS/WASM/Browser Client Release Surface](https://github.com/flyingrobots/echo/issues/516)
- [#523 Package Publish And Versioning](https://github.com/flyingrobots/echo/issues/523)

The WAL/WSC/durability goalpost roadmap is
[`docs/design/wal-wsc-durability-roadmap.md`](design/wal-wsc-durability-roadmap.md).

## Legacy Method Backlog

The per-lane filesystem cards formerly under `docs/method/backlog/` have moved
to GitHub Issues. The directory remains only as a `.gitkeep` marker for legacy
workspace-discovery compatibility.

Use live issue queries instead of restoring stale local links:

- ASAP: `gh issue list --repo flyingrobots/echo --state open --label lane:asap`
- Release: `gh issue list --repo flyingrobots/echo --state open --label lane:release`
- Up next:
  `gh issue list --repo flyingrobots/echo --state open --label lane:up-next`
- Inbox: `gh issue list --repo flyingrobots/echo --state open --label lane:inbox`
- Bad code:
  `gh issue list --repo flyingrobots/echo --state open --label lane:bad-code`
- Cool ideas:
  `gh issue list --repo flyingrobots/echo --state open --label lane:cool-ideas`

## v0.1.0 Lane

The `v0.1.0` lane has moved to GitHub Issues. Active release-lane issues
include:

- [#506 Release-Grade Quickstart](https://github.com/flyingrobots/echo/issues/506)
- [#507 Contract-Aware Receipts And Readings](https://github.com/flyingrobots/echo/issues/507)
- [#508 Contract Obstruction Taxonomy](https://github.com/flyingrobots/echo/issues/508)
- [#509 Contract Reading Identity And Bounded Payloads](https://github.com/flyingrobots/echo/issues/509)
- [#510 Witnessed Intent Submission Persistence](https://github.com/flyingrobots/echo/issues/510)
- [#511 App-Safe Client Surface](https://github.com/flyingrobots/echo/issues/511)
- [#512 Contract Artifact Retention In echo-cas](https://github.com/flyingrobots/echo/issues/512)
- [#513 Contract Retention And Semantic Lookup Seams](https://github.com/flyingrobots/echo/issues/513)
- [#514 External Contract Proof Fixture](https://github.com/flyingrobots/echo/issues/514)
- [#515 jedit Real Echo Release Gate](https://github.com/flyingrobots/echo/issues/515)
- [#516 JS/WASM/Browser Client Release Surface](https://github.com/flyingrobots/echo/issues/516)
- [#517 Product-Facing Intent Outcome API](https://github.com/flyingrobots/echo/issues/517)
- [#518 Reference Trusted Runtime Host Loop](https://github.com/flyingrobots/echo/issues/518)
- [#519 Retained Evidence Durability Boundary](https://github.com/flyingrobots/echo/issues/519)
- [#520 Versioned Contract And API Compatibility](https://github.com/flyingrobots/echo/issues/520)
- [#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521)
- [#522 WSC Causal-History Storage](https://github.com/flyingrobots/echo/issues/522)
- [#523 Package Publish And Versioning](https://github.com/flyingrobots/echo/issues/523)
- [#524 v0.1.0 Release Candidate](https://github.com/flyingrobots/echo/issues/524)
- [#525 Authority Boundary Audit](https://github.com/flyingrobots/echo/issues/525)
- [#526 v0.1.0 Replay And DIND Proof](https://github.com/flyingrobots/echo/issues/526)

## Local Legacy Cards

These older local cards still exist outside `docs/method/backlog/`. Treat them
as legacy debt/idea records unless a live GitHub issue says otherwise.

### Bad Code

- [RE-028 — Merkle-Tree Memoization in Snapshot Accumulator](../backlog/bad-code/RE-028-snapshot-accumulator-memoization.md)
- [RE-029 — Enforce det_fixed by Default](../backlog/bad-code/RE-029-concurrent-snapshot-fetching.md)
- [RE-030 — Converge QueryView Reads onto Optics](../backlog/bad-code/RE-030-queryview-optic-convergence.md)
- [RE-031 Capability Grant Validation Admission Integration](../backlog/bad-code/RE-031-capability-grant-validation-admission-integration.md)
- [RE-032: Publish Durable Scheduler Fault Evidence](../backlog/bad-code/RE-032-durable-scheduler-fault-evidence.md)

### Cool Ideas

- [CI-001 — Causal "Multiverse" Puzzle Engine](../backlog/cool-ideas/CI-001-causal-puzzle-engine.md)
- [CI-002 — Deterministic Rule Profiling (Flamegraphs)](../backlog/cool-ideas/CI-002-deterministic-flamegraphs.md)
- [CI-003 — Append-only Braid Membership](../backlog/cool-ideas/CI-003-append-only-braid-membership.md)

## Live GitHub Queries

GitHub Issues are the live tracker. Use these commands instead of this file for
current counts and labels:

```bash
gh issue list --repo flyingrobots/echo --state open --limit 1000
gh issue list --repo flyingrobots/echo --state open --label lane:release --limit 1000
gh issue list --repo flyingrobots/echo --state open --label legend:platform --limit 1000
gh issue list --repo flyingrobots/echo --state open --search "WAL OR WSC OR durability"
```

Historical audit notes, including previous closure decisions such as `#281` and
`#285`, live in GitHub issue history and merged PR evidence rather than in this
inventory.

## Notes For Future Audits

- Do not close open issues just because they are old. Several old issues are
  mirrored in `docs/method/task-matrix.md`, backlog cards, and docs audits.
- The GitHub issue tracker contains both execution work and intentional idea
  parking lots. Keep the issue open when the repo still carries a matching
  backlog card or task-DAG node.
- Favor moving stale issue text into filesystem backlog cards before closing
  the issue, unless the work is already clearly completed by merged code/docs.
