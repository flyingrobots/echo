<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method inbox

Implement `cargo xtask method inbox "idea"` — create a backlog file in
`docs/method/backlog/inbox/` from a one-liner.

## Acceptance

- Creates a markdown file with the idea as the heading.
- Filename is slugified from the idea text.
- File is created, not staged or committed (that's the caller's job).
