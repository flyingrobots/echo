<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method pull

Status: complete. `cargo xtask method pull <item>` promotes a backlog markdown
file into the next numbered `docs/design/<cycle>/` directory, accepts a source
path, file stem, generated METHOD id, or native task id, strips uppercase legend
prefixes from the design filename, and refuses ambiguous or missing selectors.

Implement `cargo xtask method pull <item>` — promote a backlog item
into the next numbered cycle.

## Acceptance

- [x] Moves the backlog file to `docs/design/<next-cycle>/`.
- [x] Auto-numbers the cycle directory (e.g., `0001-<name>/`).
- [x] Strips the legend prefix from the design doc filename.
- [x] Prints the cycle number and path.
