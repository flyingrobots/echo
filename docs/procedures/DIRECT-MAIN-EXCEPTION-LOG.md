<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Direct Main Exception Log

Status: process scar tissue.
Scope: exceptional direct pushes to `main` that bypass the normal PR workflow.

## Doctrine

Direct pushes to `main` remain forbidden by default.

The normal path is branch, PR, review, green CI, then merge. A direct-main push is
acceptable only when an explicit maintainer instruction authorizes a narrow
exception and the change is low-risk, already locally validated, and provenance is
recorded immediately afterward.

This log does not create a standing fast path. It records exceptions so the repo
keeps its process history honest.

## 2026-05-14 — Echo graph model docs checkpoint

### Why `main` was bypassed

The maintainer explicitly authorized pushing the docs checkpoint directly after
confirming the two local commits were wanted on `main`.

The change was docs-only and captured a design checkpoint for Echo's built-in
graph data model before additional optic, authority, and transaction work
continued.

### Commits pushed

```text
a3d6632 docs(core): define built-in Echo graph data model
512a33d docs(core): expand Echo graph model coverage
```

### Files changed

```text
CHANGELOG.md
docs/design/built-in-echo-graph-data-model.md
```

No Rust source, tests, workflows, scripts, generated artifacts, or runtime code
were touched by the direct push.

### Validation run

The pre-push hook classified the change as docs-only and ran the markdown
formatting gate:

```text
[verify-local] docs-only change set
[verify-local] prettier --check (2 markdown files)
Checking formatting...
All matched files use Prettier code style!
[verify-local] completed in 1s (fresh)
```

GitHub accepted the push through an admin bypass and reported the rule violation
that was bypassed:

```text
remote: Bypassed rule violations for refs/heads/main:
remote:
remote: - Changes must be made through a pull request.
```

### Future rule

Prefer PRs unless an emergency or docs-only fast path is explicitly authorized by
a maintainer.

Even for docs-only direct pushes:

- state the exception before pushing;
- run the relevant docs validation;
- record exact commits and changed files;
- confirm no source/runtime code was touched;
- add a process note afterward if the exception affected repository governance.
