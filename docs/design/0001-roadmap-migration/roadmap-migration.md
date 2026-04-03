<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Cycle 0001: Migrate ROADMAP to METHOD backlog

- **Sponsor human:** James
- **Sponsor agent:** Claude
- **Hill:** The old ROADMAP structure is gone and every item lives in
  exactly one METHOD backlog lane.

## Playback questions

1. Does `ls docs/ROADMAP/` fail (directory deleted)?
2. Does `ls docs/ROADMAP.md` fail (file deleted)?
3. Does every former ROADMAP item exist in exactly one METHOD lane?
4. Are legend prefixes applied where appropriate?
5. Can `ls docs/method/backlog/*/` show the full picture of what's
   next, what's soon, and what's a cool idea?

## Mapping

From `DOCS_AUDIT.md`:

- `lock-the-hashes/`, `developer-cli/`, `proof-core/` → `asap/`
- `time-semantics-lock/`, `first-light/` → `up-next/`
- `time-travel/`, `splash-guy/`, `tumble-tower/`, `deep-storage/`,
  `proof-time-convergence/` → `cool-ideas/`
- `backlog/` → `inbox/` (triage individually)

## Postures

- **Accessibility:** Not applicable — filesystem reorganization only.
- **Localization:** Not applicable.
- **Agent inspectability:** After migration, `ls` queries against
  backlog lanes answer "what's next" without needing git history or
  conversation context.

## Non-goals

- Rewriting the content of migrated items. They move as-is with a
  legend prefix added to the filename.
- Building xtask tooling (that's a separate backlog item).
- Deciding priority within a lane. Items land in the lane; ordering
  within the lane is judgment at pull time.
