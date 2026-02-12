<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Roadmap Index

> Scope: Echo + Wesley + git-mind planning and sequencing.
> Format: ROADMAP index -> milestone README -> feature file (tasks inline).
> Last updated: 2026-02-12

This is the map-of-content (MoC) index for roadmap navigation. Detailed specs live in `docs/ROADMAP/`.

## Execution Policy

- Priority order and dependency order are tracked separately.
- WIP cap: maximum 2 active milestones at once.
- WIP cap: maximum 3 active feature files per active milestone.
- Any work outside WIP caps is queued, not in-progress.

## Dependency DAG

```mermaid
flowchart TD
  A["P0 Lock the Hashes"] --> C["P1 First Light"]
  A --> D["P1 Proof Core"]
  B["P0 Developer CLI"] --> C
  E["P1 Time Semantics Lock"] --> F["P2 Time Travel"]
  D --> G["P2 Proof+Time Convergence"]
  F --> G
  C --> H["P2 Splash Guy"]
  C --> I["P2 Tumble Tower"]
  C --> J["P2 Deep Storage"]
```

## Priority / Status

| Pri | Milestone                                                          | Features | Est. Hours          | Status      | Blocked By              |
| --- | ------------------------------------------------------------------ | -------- | ------------------- | ----------- | ----------------------- |
| P0  | [Lock the Hashes](ROADMAP/lock-the-hashes/README.md)               | 2        | ~20h                | In Progress | —                       |
| P0  | [Developer CLI](ROADMAP/developer-cli/README.md)                   | 5        | ~30h                | Not Started | Lock the Hashes         |
| P1  | [First Light](ROADMAP/first-light/README.md)                       | 9        | ~90h                | Not Started | —                       |
| P1  | [Proof Core](ROADMAP/proof-core/README.md)                         | 3        | ~18h                | Planned     | Lock the Hashes         |
| P1  | [Time Semantics Lock](ROADMAP/time-semantics-lock/README.md)       | 1        | ~6h                 | Planned     | —                       |
| P2  | [Time Travel](ROADMAP/time-travel/README.md)                       | 3        | ~56h                | Planned     | Time Semantics Lock     |
| P2  | [Proof+Time Convergence](ROADMAP/proof-time-convergence/README.md) | 1        | ~10h                | Planned     | Proof Core, Time Travel |
| P2  | [Splash Guy](ROADMAP/splash-guy/README.md)                         | 1        | TBD (skeleton ~28h) | Planned     | First Light             |
| P2  | [Tumble Tower](ROADMAP/tumble-tower/README.md)                     | 1        | TBD (skeleton ~45h) | Planned     | First Light             |
| P2  | [Deep Storage](ROADMAP/deep-storage/README.md)                     | 4        | ~45h                | Planned     | First Light             |
| —   | [Backlog](ROADMAP/backlog/README.md)                               | 13       | ~156h               | Unscheduled | —                       |

## Milestone Directories

- `docs/ROADMAP/lock-the-hashes/`
- `docs/ROADMAP/developer-cli/`
- `docs/ROADMAP/first-light/`
- `docs/ROADMAP/proof-core/`
- `docs/ROADMAP/time-semantics-lock/`
- `docs/ROADMAP/time-travel/`
- `docs/ROADMAP/proof-time-convergence/`
- `docs/ROADMAP/splash-guy/`
- `docs/ROADMAP/tumble-tower/`
- `docs/ROADMAP/deep-storage/`
- `docs/ROADMAP/backlog/`

## Cross-Project Notes

- Wesley work is grouped into First Light because it is upstream of the website demo deliverable.
- git-mind NEXUS is moved to Backlog because it is independent of Echo's critical path.
- Proof work is split into Proof Core (P1) and Proof+Time Convergence (P2) to avoid false blocking.

## Issue Matrix

Issue coverage is maintained in `docs/ROADMAP/ISSUE-INDEX.md`.
