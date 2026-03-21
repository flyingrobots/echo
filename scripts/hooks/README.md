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
`make verify-ultra-fast`, `make verify-fast`, `make verify-pr`, and
`make verify-full` entry points. For PR gate visibility before or after a push,
prefer `make pr-status`.

The local full gate now runs as curated parallel lanes with isolated
`CARGO_TARGET_DIR`s, which keeps expensive cargo invocations from serializing on
the same target lock. `make verify-full-sequential` remains available as a
fallback if you need to debug the lane runner itself.

A critical path no longer means “run the same local Rust cargo gauntlet for
every kind of full change.” Tooling-only full changes stay tooling-local, while
critical Rust changes run a local smoke lane and leave the exhaustive all-target
proof to CI.

That local smoke path is also file-family aware for `warp-core`: ordinary source
edits stay on the library test lane, while runtime/inbox, playback, and PRNG
touches pull the specific extra smoke checks they need instead of one fixed
bundle every time.

The same principle now applies to the WASM boundary crates: `warp-wasm`
distinguishes plain lib work from `warp_kernel` engine work, `echo-wasm-abi`
pulls targeted canonical/codec vectors when those surfaces move, and README-only
or other non-Rust crate changes do not wake the Rust smoke lanes.

`make verify-ultra-fast` is now the shortest edit-loop lane. It stays
compile-first: Rust changes get `cargo check` on changed Rust crates plus the
same targeted critical smoke selection used by the full gate, while clippy,
rustdoc, guard scans, and exhaustive local proof stay on the heavier paths and
in CI. Tooling-only changes stay on a syntax/smoke path instead of inheriting
the full hook regression suite.

A successful `make verify-full` run now shares the same success stamp as the
canonical pre-push full gate for the same worktree tree, so commit-only churn
and unchanged unstaged content do not rerun identical full verification
locally. Local timing data now lands
in `.git/verify-local/timing.jsonl`, including run-level and per-lane durations,
which keeps timing artifacts out of the tracked repo while still making lane
cost visible. The staged and reduced local Rust paths are also intentionally
narrower than CI: heavy all-target clippy coverage stays in CI, while local
hooks bias toward faster iteration on the current work surface.
