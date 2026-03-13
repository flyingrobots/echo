<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Legacy Hook Shims

The canonical repository hooks live in [`.githooks/`](../../.githooks)
and should be installed with `make hooks`, which configures `core.hooksPath`
to point to that repository-relative directory.

The scripts in this directory are compatibility shims for manual invocation or
older local workflows. `scripts/hooks/pre-commit` and `scripts/hooks/pre-push`
run a limited, independent subset of checks and do **not** delegate to
`.githooks/pre-commit` or `.githooks/pre-push`.

They are **not** the authoritative local CI policy, can drift over time, and
are **not** equivalent to the full enforcement in `.githooks/pre-commit` and
`.githooks/pre-push`.
