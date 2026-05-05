<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Developer CLI | **Priority:** P0

# CLI Scaffold (#47)

Subcommand structure and ergonomic defaults for the current clap-based
`echo-cli`.

Status: T-6-1-1 complete; T-6-1-2 remains planned. `crates/warp-cli/src/cli.rs`
and `main.rs` provide the clap subcommand shell for `verify`, `bench`, and
`inspect`, the global `--format` flag, and the `echo-cli` binary name.
`--verbose` and `--snapshot-dir` are not part of the current global surface:
current subcommands take explicit paths, and verbosity/config defaults belong
to T-6-1-2 if they are still wanted.

## T-6-1-1: clap subcommand structure and global flags

Status: complete.

Implementation status: complete. `clap`, the subcommand enum, binary name,
no-subcommand error path, unknown-subcommand error path, global `--format`
parsing, invalid-format rejection, and the checked-in top-level help golden are
implemented and tested.

Completion evidence:

- Added `crates/warp-cli/tests/golden/echo-cli-help.txt`.
- Added an integration test that locks `echo-cli --help` to the golden output.
- Added parser coverage for invalid `--format` values.
- Revalidated `--verbose` and `--snapshot-dir` as non-goals for T-6-1-1 because
  the current command surface has explicit command paths and no shared
  verbosity behavior.

**User Story:** As a developer, I want a well-structured CLI with `echo verify|bench|inspect` subcommands so that I can interact with Echo from the terminal.

**Requirements:**

- R1: Add `clap = { version = "4", features = ["derive"] }` dependency to warp-cli.
- R2: Define top-level `Cli` struct with `#[command(subcommand)]` and variants: `Verify`, `Bench`, `Inspect`.
- R3: Global flags: `--format [text|json]` is implemented; `--verbose` and `--snapshot-dir <path>` are deferred unless T-6-1-2 proves a current global use.
- R4: Running `echo` with no subcommand prints help. Unknown subcommands print error + help.
- R5: Binary name is `echo-cli` (avoid collision with `/bin/echo`).

**Acceptance Criteria:**

- [x] AC1: `echo-cli --help` prints usage with all three subcommands listed.
- [x] AC2: `echo-cli verify --help` prints verify-specific options.
- [x] AC3: `echo-cli --format json verify` parses the global flag correctly before the subcommand.
- [x] AC4: `echo-cli unknown` exits with code 2.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** clap setup, subcommand enum, global flags, help text, binary name.
**Out of Scope:** Subcommand implementations (separate tasks). Config file parsing. Shell completions.

**Test Plan:**

- **Goldens:** `echo-cli --help` output checked in as a golden text file.
- **Failures:** Missing required subcommand-specific args. Invalid format value.
- **Edges:** `--verbose --verbose` (count-based verbosity). `--snapshot-dir` with spaces in path.
- **Fuzz/Stress:** N/A (argument parsing only).

**Blocked By:** none
**Blocking:** T-6-2-1, T-6-3-1, T-6-4-1

**Est. Hours:** 3h
**Expected Complexity:** ~100 LoC

---

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

**Blocked By:** residual global-flag decision from T-6-1-1
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~100 LoC
