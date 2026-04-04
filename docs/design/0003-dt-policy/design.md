<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0003 — Lock the dt policy

_Ratify fixed timestep as a project-wide invariant._

Legend: KERNEL

Depends on:

- nothing

## Why this cycle exists

Every open design question in the time/strand/settlement space
implicitly depends on whether tick N on worldline A represents the
same elapsed simulation time as tick N on worldline B. The answer
is yes. It has always been yes in practice. This cycle makes it
explicit and permanent.

Fixed timestep is not a default with an opt-out. It is an invariant.
Every tick is the same simulated duration across all worldlines. This
means tick numbers are directly comparable across worldlines, strands
are forkable and settleable without dt compatibility checks, and
replay is structurally sound without recording per-tick time deltas.

This is a spec-only cycle. No runtime code. The deliverable is an
invariant document and a normative reference from the existing spec
corpus.

## Human users / jobs / hills

### Primary human users

- Engine contributors implementing time-aware systems
- Game designers choosing time models for their simulations
- Debugger developers building fork/compare workflows

### Human jobs

1. Know that tick numbers are always comparable across worldlines.
2. Know that no code path needs to account for variable dt.

### Human hill

A contributor can trust that tick N means the same thing everywhere
in the system, without checking configuration or consulting a
compatibility matrix.

## Agent users / jobs / hills

### Primary agent users

- Agents writing time-aware adapters or session protocol code
- Agents implementing strand/settlement specs downstream

### Agent jobs

1. Assume fixed dt when generating time-dependent code.
2. Never emit variable-dt plumbing.

### Agent hill

An agent can assume tick-comparability across worldlines without
querying per-worldline configuration.

## Human playback

1. The human opens `docs/invariants/FIXED-TIMESTEP.md`.
2. The document states: every tick is the same simulated duration.
   This is an invariant, not a configuration option.
3. The human can answer "are tick numbers comparable across these
   two worldlines?" with "yes, always" without reading anything else.

## Agent playback

1. The agent reads the invariant document.
2. The document contains the normative statement.
3. The agent treats tick-comparability as a global assumption.

## Implementation outline

1. Create `docs/invariants/` directory.
2. Write `docs/invariants/FIXED-TIMESTEP.md` with the invariant
   statement, rationale, and consequences.
3. Cross-reference from SPEC-0004 (worldlines) — add a one-line
   normative reference to the invariant.
4. Cross-reference from `CONTINUUM.md` or `continuum-foundations.md`
   if appropriate.
5. Update the strand-contract and strand-settlement backlog items
   to note that dt compatibility checks are unnecessary (the
   invariant eliminates the concern).

## Tests to write first

- Shell assertion: `docs/invariants/FIXED-TIMESTEP.md` exists.
- Shell assertion: the invariant document contains the word "MUST"
  and the phrase "fixed timestep".
- Shell assertion: SPEC-0004 contains a reference to the invariant.
- Shell assertion: no file in `crates/` references "variable.dt"
  or "variable_dt" or "dt_stream" (negative test — the concept
  does not exist in code).

## Risks / unknowns

- **Risk: future use case needs variable dt.** If a real consumer
  appears, the invariant can be relaxed to a default in a future
  cycle with explicit design work. But we do not design for
  hypothetical requirements. Fixed until proven otherwise.

## Postures

- **Accessibility:** Not applicable — spec-only, no UI.
- **Localization:** Not applicable — internal invariant document.
- **Agent inspectability:** The invariant is a single normative
  statement parseable by grep.

## Non-goals

- Variable-dt as an opt-in stream (eliminated by this invariant).
- Runtime enforcement of the invariant (the invariant is structural —
  there is no variable-dt mechanism to enforce against).
- The strand contract itself (next cycle).
- Settlement semantics (future cycle after strand contract).
