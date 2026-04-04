<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro — 0003 dt-policy

## What shipped

The FIXED-TIMESTEP invariant: `docs/invariants/FIXED-TIMESTEP.md`.

Seven normative rulings lock dt as fixed per worldline. No committed
tick carries its own dt. Wall-clock time never enters semantic
history. Cross-worldline operations require identical `tick_quantum`.

WL-003 added to SPEC-0004 as a cross-reference.

Test script: `scripts/tests/fixed_timestep_invariant_test.sh`
(14 assertions, all passing).

## Playback witness

### Human playback

| #   | Question                                             | Answer                                                | Witness                                  |
| --- | ---------------------------------------------------- | ----------------------------------------------------- | ---------------------------------------- |
| 1   | Does the invariant document exist?                   | Yes                                                   | test 1 PASS                              |
| 2   | Does it state seven normative rulings?               | Yes                                                   | tests 3.1–3.7 all PASS                   |
| 3   | "Are tick numbers comparable across two worldlines?" | Yes, if tick_quantum is identical (R7)                | grep for "identical.\*tick_quantum" PASS |
| 4   | "Can my adapter use wall-clock time?"                | Yes, for telemetry — canonical decision required (R6) | grep for "canonical decision" PASS       |
| 5   | "What does catch-up mean?"                           | 0/1/N fixed ticks, not one larger tick                | Consequences section present             |

### Agent playback

| #   | Question                                       | Answer                                 | Witness                                                   |
| --- | ---------------------------------------------- | -------------------------------------- | --------------------------------------------------------- |
| 1   | Can the agent read the invariant?              | Yes, file exists and is grep-parseable | test 1 PASS                                               |
| 2   | Does ProvenanceEntry have a per-tick dt field? | No                                     | negative tests PASS (no variable_dt/dt_stream in crates/) |
| 3   | Is tick_quantum a genesis parameter?           | Yes, stated in R1                      | grep for "immutable.\*tick_quantum" PASS                  |

Full test output in `witness/test-output.txt`.

## Drift check

- **Scope drift:** None. The original design proposed variable-dt as
  an opt-in escape hatch. This was eliminated during design review —
  the human directed a project-wide invariant instead. Simpler,
  sharper, better.
- **Spec drift:** `warp_geom::Tick` doc comment already said "fixed
  `dt` per branch." The invariant formalizes existing practice.

## New debt

- `tick_quantum` is declared as a worldline-genesis parameter but no
  runtime type or field exists yet. When worldline creation is
  formalized, this parameter needs a home.
- `docs/invariants/` now exists with one file. The inbox item
  `KERNEL_invariants-as-docs` tracks extracting the other invariants
  (No Global State, Two-Plane Law, etc.) into this directory.

## Cool ideas

- The invariant test pattern (shell assertions grepping for normative
  phrases in spec documents) could be generalized. A
  `scripts/check_invariants.sh` that runs all `*_invariant_test.sh`
  scripts would catch spec weakening across the board.
- `tick_quantum` could be surfaced through the TTD adapter as a
  worldline metadata field, so the debugger can display "this
  worldline runs at 60 ticks/sec" alongside the timeline.
