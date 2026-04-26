<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Handoff

Status: current operational handoff for branch `chore/land-braids`.
Replace this file when it goes stale. Git history is the archive.

## Current Anchor

- Branch: `chore/land-braids`
- HEAD: `413f85c feat: add method inbox capture`
- Remote: pushed and aligned with `origin/chore/land-braids`
- Docs inventory is paused at `d2b38a0 docs: pause docs inventory`

## Ground Rules

- Do not use `codex-think`; the user said it is broken until they say
  otherwise.
- Never amend commits.
- Never rebase unless the user explicitly approves after explanation.
- Never force-push.
- Preserve `pnpm docs:build` as a real gate.
- Do not resume broad docs inventory unless a doc blocks active work, a stale
  doc breaks a gate, or a dedicated docs-cleanup cycle is explicitly scheduled.
- For Rust tests, do not run `cargo test -p <crate> <filter>`. Use `--lib`,
  `--test`, or `cargo xtask test-slice` forms.

## What Just Happened

- Top-level docs cleanup was completed.
- Strict live-docs doctrine was established: docs are current truth; git
  history is the archive.
- Dead-link cleanup restored `pnpm docs:build`.
- `docs/method/backlog/cool-ideas/` was fully audited.
- Full docs inventory was paused intentionally with remaining unaudited docs as
  known debt.
- `cargo xtask method inbox "<idea>"` was implemented.

## Method Inbox Feature

Implemented:

- `method::inbox::filename_from_title`
- `method::inbox::create_inbox_item`
- `cargo xtask method inbox <TITLE>`

Behavior:

- Creates `docs/method/backlog/inbox/<slug>.md`.
- Writes SPDX/copyright header.
- Writes a simple Method-compatible scaffold.
- Prints the created path.
- Refuses to overwrite existing files.

Verification already run:

- `cargo test -p method --test inbox_tests`
- `cargo test -p method --test status_tests`
- `cargo test -p method --lib`
- `cargo clippy -p method --all-targets -- -D warnings`
- `cargo xtask method status --json`
- `cargo xtask method inbox --help`
- `git diff --check`
- `pnpm exec prettier --check docs/method/README.md docs/method/legends/PLATFORM.md`
- `npx markdownlint-cli2 docs/method/README.md docs/method/legends/PLATFORM.md`
- `pnpm docs:build`

## Self-Review Findings To Fix Next

### High: Settlement overlap revalidation is too coarse

`crates/warp-core/src/settlement.rs` classifies overlapped patches as clean
only when the whole state root is unchanged. That can falsely reject a patch
that writes an overlapped slot idempotently while legitimately changing a
disjoint slot.

Fix direction: compare only the overlapped slots before/after replay when
deciding clean vs conflict, and add a regression test with one idempotent
overlapped slot plus one mutable disjoint slot.

### High: Settlement basis evidence is dropped at the ABI boundary

Internal settlement structs now carry `basis_report` and
`overlap_revalidation`, but `to_abi()` and the ABI DTOs do not expose that
evidence to WASM consumers.

Fix direction: add ABI DTO summaries for basis posture and overlap
revalidation, wire settlement `to_abi()` through, update ABI docs, and add
`warp-wasm` tests.

### Medium: VitePress navigation still has missing routes

`docs/.vitepress/config.ts` still links missing pages:

- `/guide/wvp-demo`
- `/guide/collision-tour`
- `/spec-serialization-protocol`
- `/spec-branch-tree`

Fix direction: remove, redirect, or replace those links with current owned
docs, then add a route check that covers VitePress config links.

### Medium: `docs/BEARING.md` contradicts the pause checkpoint

`docs/BEARING.md` still says the next move is five-docs-at-a-time inventory.
The actual state says inventory is paused and feature work resumed.

Fix direction: update `BEARING.md` to reflect the inventory pause and Method
automation lane.

### Medium: `.codex/EVENT_LOG.md` may be local-state residue

The branch contains `.codex/EVENT_LOG.md`. It looks like local coordination
memory, not repo truth.

Fix direction: decide whether this is intentional repo state. If not, delete it
and ignore local Codex event logs while preserving required `.codex`
environment config.

### Low: Inbox slug length is unbounded

`method::inbox::filename_from_title` can generate pathologically long
filenames.

Fix direction: add a deterministic max slug length and tests for long-title
handling plus collision refusal.

## Recommended Next Move

Do not restart docs inventory. Fix the two high-severity runtime/ABI review
findings first. They affect the WARP optics path more directly than additional
docs cleanup.

If choosing a smaller follow-up before runtime work, fix `BEARING.md` and the
VitePress nav links together as one docs-gate hardening commit.
