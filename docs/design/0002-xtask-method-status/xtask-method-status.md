<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0002 — xtask method status

_Cycle for adding a `method status` subcommand to the xtask CLI so
the METHOD state of the repo is one command away._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on: nothing (first tooling cycle)

## Why this cycle exists

METHOD was adopted in the previous session. The backlog lanes are
populated, legends are defined, and the first cycle has shipped. But
the only way to see the state of the backlog is `ls`. That works,
but it requires multiple commands and mental arithmetic to answer
"what's next?" and "how loaded is each legend?"

This cycle exists to make `cargo xtask method status` the single
command that answers those questions.

## Human users / jobs / hills

### Primary human users

- the developer (James) deciding what to pull next
- a future contributor reading the repo to understand project state

### Human jobs

1. See how many items are in each backlog lane at a glance.
2. See which cycles are active (started but not retro'd).
3. See which legends carry the most backlog weight.

### Human hill

A human can run one command and answer "what's next, what's active,
and where is the load?" without opening any files.

## Agent users / jobs / hills

### Primary agent users

- Claude, deciding what to recommend pulling
- any future agent operating under METHOD in this repo

### Agent jobs

1. Read structured output to determine backlog state.
2. Identify active cycles to avoid conflicting pulls.
3. Assess legend load to inform priority suggestions.

### Agent hill

An agent can parse the output of `cargo xtask method status` and
programmatically determine which lane has the most items, which
legends are heaviest, and whether any cycles are currently active.

## Human playback

1. James runs `cargo xtask method status` after opening a new session.
2. The output shows:
    - backlog lanes with item counts (e.g., `asap: 15`)
    - one active cycle (`0002-xtask-method-status`)
    - legend load breakdown (e.g., `PLATFORM: 25, KERNEL: 13, ...`)
3. James reads the asap count and the legend load, then decides
   whether to pull a PLATFORM or KERNEL item next.
4. No files were opened. The decision was informed by one command.

## Agent playback

1. Claude runs `cargo xtask method status` at the start of a session.
2. The output contains parseable lines for lane counts, active cycles,
   and legend load.
3. Claude determines that PLATFORM has the highest backlog load and
   asap has 15 items.
4. Claude recommends pulling a PLATFORM item from asap.

## Implementation outline

1. Add a `Method` variant to the `Commands` enum in `xtask/src/main.rs`
   with a `MethodCommand::Status` sub-subcommand.
2. Implement `method_status()` that:
    - finds the repo root (walk up from `env::current_dir()` looking
      for `Cargo.toml`, or use a fixed relative path)
    - reads `docs/method/backlog/{inbox,asap,up-next,cool-ideas,bad-code}/`
      and counts `.md` files per lane
    - reads `docs/design/*/` to find cycle directories
    - reads `docs/method/retro/*/` to find completed cycles
    - diffs design vs retro to identify active cycles
    - parses `<LEGEND>_` prefixes from all backlog filenames and
      counts per legend
3. Print three sections to stdout: lanes, active cycles, legend load.
4. Exit 0 on success.

## Tests to write first

- a test proving `method status` exits 0 when run against a repo
  with populated backlog lanes
- a test proving lane counts are correct for a known fixture
- a test proving active cycle detection works (design dir exists,
  retro dir does not)
- a test proving legend load counts parse prefixes correctly

## Risks / unknowns

- The xtask binary currently lives in a single `main.rs` file. Adding
  another subcommand increases the god-file problem. If this gets
  unwieldy, extracting a `method.rs` module is the natural next step,
  but that's debt, not a blocker.
- Path resolution: xtask runs from the repo root via `.cargo/config.toml`
  alias, so relative paths should work. But if someone runs it from
  a subdirectory, it could break. We should resolve paths relative to
  the workspace root.

## Postures

- **Accessibility:** Plain text output, no decoration required to
  parse meaning. Screen readers can consume it directly.
- **Localization:** Not applicable — CLI output is English-only for
  now. No hardcoded left/right layout assumptions.
- **Agent inspectability:** Output is one item per line, consistent
  formatting. An agent can parse it to determine what to pull next.

## Non-goals

- Other METHOD subcommands (inbox, pull, close, drift). Those remain
  in the backlog.
- Color output or rich formatting. Plain text is the contract.
- Reading file contents. Status only cares about filenames and
  directory structure.
- JSON output mode. If needed later, it's a separate backlog item.
