<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro: Cycle 0001 — Migrate ROADMAP to METHOD backlog

- **Outcome:** Hill met. Loop not followed.
- **Cycle type:** Design (deliverable is filesystem reorganization, not code).

## What happened

The ROADMAP migration was the first cycle under METHOD in this repo.
53 items were moved from `docs/ROADMAP/` into METHOD backlog lanes
with legend prefixes. The old ROADMAP structure was deleted. All five
playback questions answered yes.

## What went wrong

We skipped almost the entire METHOD loop:

1. **Design doc written after work started.** The design doc was
   written mid-execution, not reviewed before the agent began moving
   files. It served as post-hoc documentation, not as a steering
   artifact.
2. **No RED phase.** No tests were written. For a design cycle with
   filesystem-only changes, shell assertions (`! test -d docs/ROADMAP`)
   would have been appropriate.
3. **No formal playback.** The `ls` verification was ad hoc. No
   committed witness artifact.
4. **No PR or review.** Work was committed directly to main.
5. **Human did not review design doc before execution.** The agent
   charged ahead after "pull it" without pausing for design approval.

## Why it happened

Momentum. The migration was mechanical and the mapping was already
defined in `DOCS_AUDIT.md`. The agent treated "pull it" as "do it"
instead of "begin the design phase." First-cycle energy overrode
first-cycle discipline.

## What we learned

- The agent must pause after writing the design doc and wait for
  human review before proceeding to RED. "Pull" means "start
  designing," not "start building."
- Even design cycles benefit from tests. A shell script asserting
  filesystem state is a valid test for a filesystem reorganization.
- The first cycle sets the precedent. Skipping the loop on cycle
  0001 normalizes skipping it forever.

## Drift check

No undocumented drift. The migration mapping matched `DOCS_AUDIT.md`
exactly. Legend prefix assignments were reasonable.

## New debt

None.

## Cool ideas

None.
