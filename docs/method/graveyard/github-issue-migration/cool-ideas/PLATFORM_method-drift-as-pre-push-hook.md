<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Method drift check as pre-push hook

Status: active cool idea. Echo documents `cargo xtask method drift [cycle]` as
planned, but `cargo xtask method --help` exposes only `status` today. The
canonical pre-push hook delegates through `scripts/hooks/pre-push` to
`.githooks/pre-push`; no Method drift gate is wired there yet. The external
`/Users/james/git/method` repo already has drift detection, so the remaining
work is choosing whether Echo calls the external Method CLI or implements the
Rust xtask command first.

Once `cargo xtask method drift` exists, wire it into
`scripts/hooks/pre-push` so playback questions are checked against
test descriptions before code leaves the machine.
