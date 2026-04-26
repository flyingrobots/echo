<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method pull

Status: active and not implemented. `cargo xtask method --help` exposes only
`status`; `xtask/src/main.rs` has only `MethodCommand::Status`. `crates/method`
already exposes `MethodWorkspace::design_root()`, so the missing work is command
behavior, naming, and safe file movement.

Implement `cargo xtask method pull <item>` — promote a backlog item
into the next numbered cycle.

## Acceptance

- Moves the backlog file to `docs/design/<next-cycle>/`.
- Auto-numbers the cycle directory (e.g., `0001-<name>/`).
- Strips the legend prefix from the design doc filename.
- Prints the cycle number and path.
