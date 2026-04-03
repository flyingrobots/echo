<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo — Agent Instructions

## METHOD

This repo follows [METHOD](docs/method/README.md). All work flows
through the backlog and cycle loop. The agent must follow the loop
honestly — no skipping phases, no post-hoc design docs.

### The loop (agent responsibilities)

**Phase 1 — Pull and design** (when the human says "pull \<item\>"):

1. Move the backlog file to `docs/design/<next-cycle>/`.
2. Write the design doc (hill, playback questions, postures, non-goals).
3. **STOP. Present the design doc to the human for review.** Do not
   proceed until the human approves the design.

**Phase 2 — RED / GREEN** (when the human approves the design):

1. **RED** — write failing tests. Even for design cycles, write shell
   or script assertions that verify the expected outcome. Playback
   questions become test cases.
2. Run the tests. Confirm they fail.
3. **GREEN** — do the work. Make the tests pass.
4. Run the tests. Confirm they pass.

**Phase 3 — Playback** (when the work is done):

1. Run through every playback question. Record the witness (test
   output, ls output, whatever proves the answer).
2. Present the playback to the human. Both sponsors must say yes.

**Phase 4 — Close** (when both sponsors agree):

1. Write the retro in `docs/method/retro/<cycle>/`. Include drift
   check, new debt, cool ideas.
2. Commit the cycle packet.

### Backlog operations

- `ls docs/method/backlog/asap/` — what to pull next.
- `ls docs/method/backlog/*/` — full backlog view.
- `ls docs/design/` — active cycles.
- `ls docs/method/retro/` — completed cycles.
- `ls docs/method/graveyard/` — rejected ideas.

### Legends

| Code       | Domain                                                              |
| ---------- | ------------------------------------------------------------------- |
| `KERNEL`   | Core simulation: WARP graph, scheduling, commit, parallel execution |
| `MATH`     | Deterministic math: IEEE 754, trig oracle, collision, geometry      |
| `PLATFORM` | Tooling: WASM, xtask, CI, benchmarks, CAS, Wesley                   |
| `DOCS`     | Documentation: guides, specs, living docs, course material          |

### Cycle numbering

Cycles are numbered sequentially: `0001`, `0002`, etc. The directory
name is `<number>-<slug>/` (e.g., `0001-roadmap-migration/`).

## Build and test

```bash
cargo test                  # full test suite
cargo clippy --all-targets  # lint (zero warnings policy)
cargo fmt -- --check        # format check
cargo xtask                 # developer CLI (subcommands vary)
```

## Determinism

Echo is a deterministic simulation engine. All floating-point
operations must be canonicalized per `docs/SPEC_DETERMINISTIC_MATH.md`.
No global state, no `rand`, no system time, no unordered containers
in deterministic paths. CI enforces this via `scripts/ban-globals.sh`
and `scripts/ban-nondeterminism.sh`.
