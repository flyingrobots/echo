<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method drift

Status: active and not implemented. `cargo xtask method --help` exposes only
`status`; `xtask/src/main.rs` has only `MethodCommand::Status`. Current retros
contain manual "Drift check" sections, so the remaining work is the automated
coverage check.

Implement `cargo xtask method drift [cycle]` — check active cycle
playback questions against committed test descriptions.

## Acceptance

- Parses playback questions from the design doc.
- Searches test files for matching test names or descriptions.
- Reports coverage: which questions have tests, which don't.
- Exit code 1 if any playback question has no matching test.
