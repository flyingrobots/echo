<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Developer CLI | **Priority:** P0

# CLI Scaffold (#47)

Subcommand structure and ergonomic defaults for the current clap-based
`echo-cli`.

Status: active backlog item. The clap subcommand shell is already implemented;
only config file support and shell completions remain in this backlog card.

## T-6-1-2: Config file support and shell completions

**User Story:** As a developer, I want to set default CLI options in a config file and generate shell completions so that the CLI is ergonomic for daily use.

**Requirements:**

- R1: Support `~/.config/echo/config.toml` with fields matching global flags (`format`, `snapshot_dir`, `verbose`).
- R2: CLI flags override config file values. Config file is optional (missing = use defaults).
- R3: `echo-cli completions <shell>` subcommand generates completions for bash/zsh/fish (via `clap_complete`).
- R4: Add `clap_complete` and `toml` dependencies.

**Acceptance Criteria:**

- [ ] AC1: With `config.toml` setting `format = "json"`, `echo-cli verify` outputs JSON.
- [ ] AC2: With `config.toml` setting `format = "json"` and CLI flag `--format text`, output is text (flag wins).
- [ ] AC3: `echo-cli completions bash` outputs valid bash completion script.
- [ ] AC4: Missing config file does not produce an error or warning.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Config file loading, flag override logic, shell completion generation.
**Out of Scope:** Project-level config (`.echo.toml` in repo root). Config file creation wizard.

**Test Plan:**

- **Goldens:** Bash completion script golden file (regenerated on clap struct changes).
- **Failures:** Malformed TOML prints error with line number. Unknown config keys are warned but not fatal.
- **Edges:** Config file with only some fields set. Empty config file. Config dir does not exist.
- **Fuzz/Stress:** N/A.

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~100 LoC
