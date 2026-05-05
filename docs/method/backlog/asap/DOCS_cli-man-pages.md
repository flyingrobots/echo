<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Developer CLI | **Priority:** P0

# Docs/man pages (#51)

CLI documentation: man pages, usage examples, and integration with the docs site.

Status: complete. `clap_mangen`, `cargo xtask man-pages`, checked-in
`docs/man/echo-cli*.1` pages, root README examples, and
`cargo xtask man-pages --check` are implemented. CI now verifies that committed
man pages stay fresh against the current clap surface.

## T-6-5-1: Man page generation and README examples

Status: complete.

**User Story:** As a developer, I want `man echo-cli` to work and the README to have copy-pasteable examples so that CLI usage is discoverable.

**Requirements:**

- R1: Use the existing `clap_mangen` dependency and `cargo xtask man-pages`
  command to generate man pages to `docs/man/`.
- R2: Generate man pages for the top-level command and each current subcommand
  (`verify`, `bench`, `inspect`).
- R3: Add a "CLI Usage" section to the repository README with examples for each subcommand.
- R4: CI step verifies man pages are up-to-date (regenerate and diff; fail if stale).

**Acceptance Criteria:**

- [x] AC1: `man docs/man/echo-cli.1` renders correctly in a terminal.
- [x] AC2: `man docs/man/echo-cli-verify.1` shows verify-specific options and examples.
- [x] AC3: CI fails if someone changes clap args without regenerating man pages.
- [x] AC4: README examples are copy-pasteable and exit 0 when run against a valid fixture.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** Man page generation, xtask integration, README section, CI freshness check.
**Out of Scope:** mdbook integration. Online docs site deployment. Localization.

**Test Plan:**

- **Goldens:** Generated man pages checked in; CI diffs against regenerated output.
- **Failures:** Stale man pages (CI gate).
- **Edges:** Subcommand with no specific options (man page should still be generated with inherited global flags).
- **Fuzz/Stress:** N/A.

**Blocked By:** T-6-1-1, T-6-2-1, T-6-3-1, T-6-4-1
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~100 LoC (xtask) + generated man pages
