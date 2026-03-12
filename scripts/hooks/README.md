<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Legacy Hook Shims

The canonical repository hooks live in [`/Users/james/git/echo/.githooks`](/Users/james/git/echo/.githooks)
and should be installed with `make hooks`, which configures `core.hooksPath`
to point there.

The scripts in this directory are intentionally minimal compatibility shims for
manual invocation or older local workflows. They are **not** the authoritative
local CI policy, and they do not replace the broader checks enforced by
`.githooks/pre-commit` and `.githooks/pre-push`.
