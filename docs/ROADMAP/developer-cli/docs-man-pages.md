<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Developer CLI](README.md) | **Priority:** P0

# Docs/man pages (#51)

CLI documentation: man pages, usage examples, and integration with the docs site.

## T-6-5-1: Man page generation and README examples

**User Story:** As a developer, I want `man echo-cli` to work and the README to have copy-pasteable examples so that CLI usage is discoverable.

**Requirements:**

- R1: Add `clap_mangen` dependency and an xtask command (`cargo xtask man`) that generates man pages to `docs/man/`.
- R2: Generate man pages for the top-level command and each subcommand (verify, bench, inspect, completions).
- R3: Add a "CLI Usage" section to the repository README with examples for each subcommand.
- R4: CI step verifies man pages are up-to-date (regenerate and diff; fail if stale).

**Acceptance Criteria:**

- [ ] AC1: `man docs/man/echo-cli.1` renders correctly in a terminal.
- [ ] AC2: `man docs/man/echo-cli-verify.1` shows verify-specific options and examples.
- [ ] AC3: CI fails if someone changes clap args without regenerating man pages.
- [ ] AC4: README examples are copy-pasteable and exit 0 when run against a valid fixture.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

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
