<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Legacy Hook Shims

The canonical repository hooks live in [`.githooks/`](../../.githooks)
and should be installed with `make hooks`, which configures `core.hooksPath`
to point to that repository-relative directory.

The scripts in this directory are compatibility shims for manual invocation or
older local workflows. They now delegate directly to the canonical hook
implementations in [`.githooks/`](../../.githooks) so a repo configured with
`core.hooksPath=scripts/hooks` does not drift from the documented policy.

Authoritative behavior lives in `.githooks/pre-commit` and
`.githooks/pre-push`. For explicit local runs outside git hooks, prefer the
`make verify-fast`, `make verify-pr`, and `make verify-full` entry points.
