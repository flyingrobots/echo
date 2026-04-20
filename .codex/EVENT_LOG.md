<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

2026-04-16T13:40:12Z | context | Stopped using codex-think; using local .codex event log instead. User asked what replaceRangeAsTick in Echo is doing.
2026-04-16T13:44:15Z | correction | User correctly objected to jedit-specific binders in warp-core production API. Investigating usages before removing them from public/runtime surface.
2026-04-16T13:48:19Z | correction | Removed jedit-specific binder DTOs/functions from warp-core public surface; left them test-only; ignored .codex in echo. Commit 7aaafdb.
2026-04-16T14:47:00Z | progress | Recorded Echo host-surface blocker for jedit hot-text runtime integration.

- 2026-04-16: Locked in generic Observer API / ObserverPlan doctrine for Echo, added design note 0013, and reframed the jedit optic handoff backlog around compiled observer plans.
  2026-04-18T13:40:00Z | branch-cleanup | Deleted approved remote perf baseline branches `chore/perf-baseline-*` from echo.
  2026-04-18T13:47:30Z | ci | Disabled the perf baseline auto-update workflow and documented that `perf-baseline.json` updates are now manual; merged via PR #317 to origin/main.
