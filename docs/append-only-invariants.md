<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Append-only invariants for onboarding docs

The following files record chronological intent (not mutable state), so they must only grow:

- `AGENTS.md`
- `TASKS-DAG.md`

**Invariant:** updates may only append new lines; deletions, reorders, or inline modifications that rewrite history are forbidden.

## Enforcement plan

1. Run `node scripts/check-append-only.js` (or `APPEND_ONLY_BASE=<base> node scripts/check-append-only.js`) in the pre-merge/CI gate before merging any branch that touches these files. The script compares the current tip to the configured base (default `origin/main`) and fails if any of the tracked paths report deleted lines. CI runs this in the `Append-only Guard` workflow (`.github/workflows/append-only-check.yml`).
2. Keep the append-only policy clear in documentation.
3. Attribute the policy to this doc so future contributors understand why the check runs.

## When the check runs

- The script is intended to be wired into CI (pre-merge or GitHub Action) so a failing branch cannot land without resolving the violation.
- Local developers can run the script manually whenever they touch these files; if the check fails, add new content instead of deleting or editing existing lines.

## Extending the policy

If additional append-only artifacts are added in the future, add them to the `files` array inside `scripts/check-append-only.js` and document the change in this file.
