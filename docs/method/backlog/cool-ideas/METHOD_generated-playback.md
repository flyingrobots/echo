<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generated playback: derive the Phase 3 doc from RED tests + cycle goals

Legend: `METHOD`

## The pain

METHOD Phase 3 asks the agent to run through every playback
question and record the witness. The expected artifact is a
playback document the human reviews and signs off on. In practice:

- Hand-written playback docs go stale the moment a test name
  changes or a witness command moves.
- Authoring playback alongside RED-then-GREEN is a separate
  context switch and routinely gets postponed.
- A stale playback is worse than no playback — it ratifies a state
  the code no longer matches.

The structural answer: the playback doc should be a _generated
artifact_, derived from the same inputs that already exist by the
time Phase 3 starts.

## What the generator takes as input

- The cycle goal lines from `docs/design/<cycle>/design.md`
  (already required).
- The cycle's RED test names — every `#[test]` (Rust) or
  `t.test('...', ...)` (Node) in the test files marked as part of
  the cycle.
- The cycle's acceptance criteria from the design or backlog card
  (already required to be there).
- The witness command set from the cycle's playback questions
  scaffold (a one-line YAML or markdown frontmatter mapping
  question → command).

## What it emits

```text
docs/method/playback/<cycle>/playback.md
```

A markdown document where each playback question is a section, and
each section auto-fills:

- The question (from the scaffold).
- The matching RED test names and their pass/fail at last run.
- The witness command(s) and their last-recorded output.
- A boxed sign-off slot ("Sponsor 1: **_ / Sponsor 2: _**").

The doc is regenerable. If a test renames, the next regen picks up
the new name; if a witness command emits new output, the doc
reflects it. Sign-off slots persist across regens (they live in a
sibling `<cycle>/playback-signoffs.md` keyed by question id, so the
generated doc can rewrite freely without nuking signatures).

## Why this is on-brand

- "Architecture with a receipt" applied to the development process
  itself. Phase 3 is the receipt; generating it from the same
  inputs that produced Phase 2 GREEN closes the loop.
- It eliminates the largest source of "stale doc" in the METHOD
  workflow without removing the human sign-off — the sign-off
  remains the load-bearing artifact; the doc just stops lying
  about what it certifies.
- It makes the "RED test name = playback question shape" coupling
  explicit. RED authors write test names knowing they will
  surface in the playback doc, which improves naming discipline.

## Out of scope here

- Auto-generating the cycle design or RED tests from each other.
  That direction breeds tautology; humans need to author the
  goals.
- Replacing the human sign-off step. Generated docs do not
  ratify; sponsors still do.

## Trigger / acceptance

Resolve this card when:

1. An `xtask method playback <cycle>` command exists and produces
   `docs/method/playback/<cycle>/playback.md` from the inputs
   above.
2. Re-running the command after a test rename produces a doc with
   the new name and preserves prior sign-offs.
3. METHOD Phase 3 documentation references the generator as the
   canonical path (hand-written playback docs become an exception,
   not the default).

## Companion

- `docs/method/backlog/cool-ideas/METHOD_leash-files.md` — same
  spirit: structural records that the machine maintains, with
  prose lanes alongside for the human-readable narrative.
