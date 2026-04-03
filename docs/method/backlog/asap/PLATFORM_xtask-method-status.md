<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method status

Implement `cargo xtask method status` — summarize backlog lanes, active
cycles, and legend load.

This is the most important METHOD command. It answers "what is
everyone working on?" by reading the filesystem.

## Acceptance

- Counts files per backlog lane.
- Lists active cycles (directories in `docs/design/` with no
  corresponding retro).
- Shows legend load (count of backlog items per legend prefix).
