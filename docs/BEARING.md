<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

This signpost summarizes direction. It does not create commitments or
replace backlog items, design docs, retros, or CLI status.

## Where are we going?

Current priority: adopt METHOD, audit the documentation corpus, and
establish honest bookkeeping for what Echo actually is today.

## What just shipped?

Adaptive parallel selector (PR #313) — the last feature cycle before
METHOD adoption.

## What feels wrong?

- The docs corpus is ~25% fiction (specs for things that don't exist).
  The audit is written; the cleanup is not.
- No cycle has ever run under METHOD in this repo. The first one will
  test whether the process fits a Rust simulation engine.
- 47 unmerged local branches were just pruned, but the worktree at
  `echo-cleanup-docs` is still hanging around.
- The old ROADMAP structure needs to be migrated into METHOD backlog
  lanes. Items are identified in `DOCS_AUDIT.md` but not yet moved.
- xtask tooling for METHOD commands does not exist yet. Manual `ls`
  and `mv` until then.
