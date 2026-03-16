<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Legacy Hook Shims

The canonical repository hooks live in [`.githooks/`](../../.githooks)
and should be installed with `make hooks`, which configures `core.hooksPath`
to point to that repository-relative directory.

The scripts in this directory are compatibility shims for manual invocation or
older local workflows. Both [`scripts/hooks/pre-commit`](./pre-commit) and
[`scripts/hooks/pre-push`](./pre-push) now delegate directly to
[`.githooks/`](../../.githooks), so a repo configured with
`core.hooksPath=scripts/hooks` does not drift from the documented policy.

Authoritative behavior lives in `.githooks/pre-commit` and
`.githooks/pre-push`. For explicit local runs outside git hooks, prefer the
`make verify-fast`, `make verify-pr`, and `make verify-full` entry points.

The local full gate now runs as curated parallel lanes with isolated
`CARGO_TARGET_DIR`s, which keeps expensive cargo invocations from serializing on
the same target lock. `make verify-full-sequential` remains available as a
fallback if you need to debug the lane runner itself.

A critical path no longer means “run the same local Rust cargo gauntlet for
every kind of full change.” Tooling-only full changes stay tooling-local, while
critical Rust changes run a local smoke lane and leave the exhaustive all-target
proof to CI.

A successful `make verify-full` run still shares the same success stamp as the
canonical pre-push full gate, so pushing the same `HEAD` does not rerun that
identical full verification locally. The staged and reduced local Rust paths are
also intentionally narrower than CI: heavy all-target clippy coverage stays in
CI, while local hooks bias toward faster iteration on the current work surface.
