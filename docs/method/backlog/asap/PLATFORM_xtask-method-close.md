<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method close

Status: active and not implemented. `cargo xtask method --help` exposes only
`status`; `xtask/src/main.rs` has only `MethodCommand::Status`. Keep this as
the execution handle until cycle-closing can be done through xtask instead of
manual filesystem edits.

Implement `cargo xtask method close [cycle]` — close a cycle with a
retro and witness directory.

## Acceptance

- Creates `docs/method/retro/<cycle>/` with a retro template.
- Creates a `witness/` subdirectory for artifacts.
- Defaults to the current (most recent) active cycle if none specified.
