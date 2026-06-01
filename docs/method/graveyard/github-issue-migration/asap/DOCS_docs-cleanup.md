<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Docs cleanup

Execute the five-at-a-time docs inventory recorded in
`docs/audits/docs-inventory-2026-04-26.md`. The old `docs/DOCS_AUDIT.md`
was deleted because it was stale; git history is the archive.

Current doctrine: `docs/` contains only current, useful, navigable truth.
Git history is the archive.

- Top-level docs are reduced to `BEARING.md`, `index.md`, and `workflows.md`
  (done).
- The strict live-docs doctrine is recorded and applied to prior audit
  decisions (done).
- `pnpm docs:build` passes and is now a real gate (done).
- Continue strict inventory under `docs/method/`.
- Continue directory inventory under owned docs areas after Method is sane.
- Delete stale Method graveyard/history material unless it is active
  operational state.
- Update `procedures/PR-SUBMISSION-REVIEW-LOOP.md` for METHOD if it remains a
  live procedure.

Some of this was done during METHOD adoption. The remainder is tracked
here and in the dated audit ledger.
