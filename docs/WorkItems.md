<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Work Tracking Boundary

GitHub Issues are the live work tracker. The Echo 1.0 Convergence Project is
the live release control surface:

<https://github.com/users/flyingrobots/projects/14>

This file is not an inventory, roadmap, sprint plan, status report, or issue
ledger. It exists to keep repository readers from treating historical local
documents as current planning truth.

## Canonical Trackers

- Echo 1.0 Release Bar:
  [#584](https://github.com/flyingrobots/echo/issues/584)
- Echo 1.0 milestone:
  <https://github.com/flyingrobots/echo/milestone/31>
- GitHub-native roadmap migration:
  [#587](https://github.com/flyingrobots/echo/issues/587)
- Local Method backlog retirement:
  [#528](https://github.com/flyingrobots/echo/issues/528)
- WAL/WSC storage relationship:
  [#521](https://github.com/flyingrobots/echo/issues/521)
- WAL/WSC durability doctrine:
  [`docs/design/wal-wsc-durability-roadmap.md`](design/wal-wsc-durability-roadmap.md)
- Echo 1.0 release contract:
  [`docs/releases/echo-1.0-contract.md`](releases/echo-1.0-contract.md)

## Local Backlog Boundary

`docs/method/backlog/` contains only `.gitkeep`; live backlog moved to GitHub
Issues.

Older files under `backlog/bad-code/` and `backlog/cool-ideas/` are legacy
debt and idea records. Treat them as source material until a GitHub issue or
closed PR supersedes them.

Do not restore local lane inventories, open-count tables, progress bars,
current-batch status, or per-issue roadmap checklists in this file.

## Query The Live State

Use GitHub for current state:

```bash
gh issue list --repo flyingrobots/echo --state open --limit 1000
gh issue list --repo flyingrobots/echo --state open --label release:echo-1.0 --limit 1000
gh issue list --repo flyingrobots/echo --state open --milestone "Echo 1.0" --limit 1000
gh project item-list 14 --owner flyingrobots --limit 1000
```

Close issues only when their executable exit criteria have passed and the
evidence is linked. A merged PR without the required proof is not sufficient
release evidence.
