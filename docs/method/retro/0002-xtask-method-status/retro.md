<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retrospective — Cycle 0002: xtask method status

- **Outcome:** Hill met.
- **Cycle type:** Feature.

## What shipped

- `crates/method/` — standalone library crate, zero Echo dependencies
    - `MethodWorkspace::discover()` validates METHOD directory structure
    - `StatusReport::build()` scans lanes, parses legend prefixes,
      diffs design vs retro for active cycles
    - `StatusReport` derives `Serialize` — JSON is the agent-first
      surface
    - 10 tests (7 integration, 3 unit)
- `cargo xtask method status` — human-readable output
- `cargo xtask method status --json` — agent-structured output

## Playback

### Agent

1. `--json` exits 0 and returns valid JSON? **Yes.**
2. Lane counts correct? **Yes.** asap: 18, up-next: 12, etc.
3. Active cycle detected? **Yes.** `0002-xtask-method-status`.
4. Legend load across all lanes? **Yes.** PLATFORM: 34, KERNEL: 19.

### Human

1. Answer "what's next?" from one command? **Yes.**
2. Compact enough to scan? **Yes.** ~15 lines.

## What went right

- The design doc was reviewed before implementation. Human caught
  three issues: standalone crate (not bolted onto xtask), agent-first
  JSON output (not "add --json later"), and design doc template
  quality (compared against bijou/warp-ttd standards).
- The METHOD loop was followed: pull → design → STOP → review →
  RED → GREEN → playback → close.
- The crate is cleanly extractable — no Echo imports.

## What went wrong

- RED and GREEN had to land in the same commit because clippy denies
  `todo!()` stubs. The tests were written first (discipline preserved)
  but the RED commit was impossible under the lint policy.

## Drift check

The design doc said "plain text is the contract" for the default
output. That's what shipped. The `--json` flag was added as the
agent surface per METHOD's "agent-first" principle — a design doc
revision that happened during review.

No undocumented drift.

## New backlog items

- `PLATFORM_method-status-legend-progress` — show completed cycle
  count per legend alongside backlog count (progress ratio).

## Cool ideas

None.
