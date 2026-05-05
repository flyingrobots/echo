<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method close

Status: complete. `cargo xtask method close [cycle]` now scaffolds
`docs/method/retro/<cycle>/retro.md` plus a `witness/` artifact directory for
an active cycle, defaults to the most recent active cycle, accepts a numeric or
full cycle selector, and refuses to overwrite existing retro material.

Implement `cargo xtask method close [cycle]` — close a cycle with a
retro and witness directory.

## Acceptance

- [x] Creates `docs/method/retro/<cycle>/` with a retro template.
- [x] Creates a `witness/` subdirectory for artifacts.
- [x] Defaults to the current (most recent) active cycle if none specified.
