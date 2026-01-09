<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Append-only invariants for onboarding docs

The following files record chronological intent (not mutable state), so they must only grow:

- `AGENTS.md`
- `docs/decision-log.md`
- `TASKS-DAG.md`
- `docs/execution-plan.md`

**Invariant:** updates may only append new lines; deletions, reorders, or inline modifications that rewrite history are forbidden.

## Enforcement plan

1. Run `node scripts/check-append-only.js` (or `APPEND_ONLY_BASE=<base> node scripts/check-append-only.js`) in the pre-merge/CI gate before merging any branch that touches these files. The script compares the current tip to the configured base (default `origin/main`) and fails if any of the tracked paths report deleted lines.
2. Update `docs/decision-log.md` and `docs/execution-plan.md` whenever you add a new entry so the logs stay current and the append-only policy is clear.
3. Document the first failing diff in `docs/decision-log.md` (with the new entry) and attribute the policy to this doc so future contributors understand why the check runs.

## When the check runs

- The script is intended to be wired into CI (pre-merge or GitHub Action) so a failing branch cannot land without resolving the violation.
- Local developers can run the script manually whenever they touch these files; if the check fails, add new content instead of deleting or editing existing lines.

## Extending the policy

If additional append-only artifacts are added in the future, add them to the `files` array inside `scripts/check-append-only.js` and document the change in this file.
