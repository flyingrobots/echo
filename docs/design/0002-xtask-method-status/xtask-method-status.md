<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0002 — xtask method status

_Cycle for building a `method` library crate and wiring its `status`
command through `cargo xtask`._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on: nothing (first tooling cycle)

## Why this cycle exists

METHOD was adopted in the previous session. The backlog lanes are
populated, legends are defined, and the first cycle has shipped. But
the only way to see the state of the backlog is `ls`. That works,
but it requires multiple commands and mental arithmetic to answer
"what's next?" and "how loaded is each legend?"

This cycle exists to make `cargo xtask method status` the single
command that answers those questions — and to build the underlying
`method` crate as a standalone library that could eventually move
out of this repo.

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

1. Claude runs `cargo xtask method status --json` at session start.
2. The output is a JSON object with `lanes`, `active_cycles`,
   `legend_load`, and `total_items` fields.
3. Claude parses the JSON directly — no regex, no line counting.
4. Claude determines PLATFORM has the highest legend load and asap
   has 15 items. Recommends pulling a PLATFORM item from asap.

## Design decisions

### Standalone `method` crate

The METHOD logic lives in `crates/method/`, not in xtask. This crate
is a library with no binary. It knows how to read a METHOD workspace
from a filesystem path and answer questions about it.

```text
crates/method/
├── Cargo.toml
└── src/
    ├── lib.rs          # pub mod workspace, status
    ├── workspace.rs    # MethodWorkspace: discover and parse a METHOD root
    └── status.rs       # StatusReport: lane counts, active cycles, legend load
```

The crate is organized so it could move to its own repo later (like
`~/git/method/` already has a TS CLI). It has no dependency on Echo,
warp-core, or any other Echo crate. Its only inputs are filesystem
paths.

### xtask integration

xtask depends on `method` and adds a thin `Method` subcommand that
constructs a `MethodWorkspace` from the repo root and calls
`method::status()`. The xtask layer handles CLI parsing and output
formatting. The library returns structured data.

### Workspace discovery

`MethodWorkspace::discover(root: &Path)` looks for:

- `docs/method/backlog/` — the backlog root
- `docs/design/` — the design docs root
- `docs/method/retro/` — the retro root
- `docs/method/legends/` — the legends root

If the backlog directory doesn't exist, the workspace is not a
METHOD repo. Return an error, don't guess.

### StatusReport struct

```rust
#[derive(Serialize)]
pub struct StatusReport {
    pub lanes: BTreeMap<String, usize>,      // lane name -> file count
    pub active_cycles: Vec<ActiveCycle>,      // design with no retro
    pub legend_load: BTreeMap<String, usize>, // legend prefix -> count
    pub total_items: usize,
}

#[derive(Serialize)]
pub struct ActiveCycle {
    pub number: String,    // e.g., "0002"
    pub slug: String,      // e.g., "xtask-method-status"
    pub legend: Option<String>,
}
```

The library returns this struct with `Serialize` derived. The agent
surface (`--json`) is the primary output — it's what gets built and
tested first. The human-readable plain text format is a projection
of the same data, added second. No `--human` flag needed; plain text
is the default, `--json` is the agent flag.

## Implementation outline

1. Create `crates/method/` with `Cargo.toml` (deps: `serde` with
   derive, `serde_json`).
2. Implement `MethodWorkspace::discover(root)` — validate directory
   structure, return workspace handle.
3. Implement `MethodWorkspace::status()` — scan lanes, parse legend
   prefixes, diff design vs retro for active cycles.
4. Add `method` as a dependency to `xtask/Cargo.toml`.
5. Add `Method(MethodArgs)` variant to `Commands` enum with
   `MethodCommand::Status` sub-subcommand.
6. Wire xtask to call `method::MethodWorkspace::discover()` then
   `status()`, format output to stdout.

## Tests to write first

In `crates/method/`:

- test that `discover()` returns error for a path with no backlog dir
- test that `discover()` succeeds for a valid METHOD workspace
- test that `status()` counts lane files correctly for a temp dir
  fixture with known `.md` files
- test that active cycle detection finds design dirs with no
  matching retro dir
- test that legend prefix parsing extracts `KERNEL` from
  `KERNEL_foo.md` and counts correctly
- test that files without a legend prefix are counted under a
  `(none)` or similar bucket
- test that `StatusReport` serializes to valid JSON with expected
  keys (`lanes`, `active_cycles`, `legend_load`, `total_items`)

## Risks / unknowns

- The `method` crate depends on `serde` and `serde_json` from day
  one. Agent surface first means structured output is not optional.
- Workspace path resolution: xtask runs from the repo root via
  `.cargo/config.toml`. The library takes an explicit path, so it
  doesn't care about working directory.
- The crate lives in Echo's workspace for now but is designed to
  extract. It must not import anything from Echo.

## Postures

- **Accessibility:** Plain text output from xtask. No decoration
  required to parse meaning.
- **Localization:** Not applicable — English-only CLI for now.
- **Agent inspectability:** `--json` outputs `StatusReport` as JSON.
  This is the primary surface — built first, tested first. The
  human plain text view is derived from the same struct.

## Non-goals

- Other METHOD subcommands (inbox, pull, close, drift). Those remain
  in the backlog and would be added to the `method` crate later.
- Color output or rich formatting. Plain text is the contract.
- Reading file contents. Status only cares about filenames and
  directory structure.
- Config files or TOML. The filesystem is the database.
