<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0003 — Lock the dt policy

_Ratify fixed timestep as a history-plane invariant: dt is fixed per
worldline, no committed tick carries its own dt, and wall-clock time
never enters semantic history._

Legend: KERNEL

Depends on:

- nothing

## Why this cycle exists

Echo's hardest open problem is canonical cross-worldline settlement.
The settlement backlog says Echo needs "one deterministic result, not
eventual convergence." If ticks can carry different durations, then
equal tick counts stop meaning equal simulated time, and
compare/braid/settle gets uglier fast. Issue #243 blocks older
time-travel inspector planning, so this is exactly the kind of foundational
invariant worth locking early instead of letting it leak everywhere
later.

The code already leans this way. `warp_geom::Tick` documents "the
engine advances in integer ticks with a fixed `dt` per branch." The
planning docs say fixed timestep is simpler and more deterministic,
while variable dt introduces a new divergence class. The TT1 task
ties the dt decision to catch-up and wormhole behavior. This cycle
slams the door.

The invariant is: dt is fixed per worldline. No committed tick carries
its own dt. Wall-clock time may exist for telemetry, rendering, pacing,
and I/O, but it does not enter semantic history as per-tick dt.

This is a spec-only cycle. No runtime code. The deliverable is an
invariant document and normative cross-references from the existing
spec corpus.

## Normative text

The invariant document will contain these rulings:

1. Every worldline has an immutable `tick_quantum` chosen at genesis.
2. Each committed tick advances simulation by exactly one
   `tick_quantum`.
3. `dt` is not an admitted stream fact and is never stored per tick.
4. Catch-up means running 0, 1, or N fixed ticks in one host frame,
   not "one larger tick."
5. All TTL, deadline, retry, and expiry semantics are
   tick-denominated.
6. HostTime may influence simulation semantics only through a
   recorded canonical decision (per TT0 HistoryTime/HostTime
   classification).
7. Cross-worldline compare and settlement require identical
   `tick_quantum`; otherwise reject in v1.

## Human users / jobs / hills

### Primary human users

- Engine contributors implementing time-aware systems
- Game designers choosing time models for their simulations
- Debugger developers building fork/compare workflows

### Human jobs

1. Know that tick numbers are always comparable across worldlines
   with the same `tick_quantum`.
2. Know that no code path needs to account for per-tick variable dt.
3. Know the boundary: wall-clock time is telemetry, not history.

### Human hill

A contributor can trust that tick N advances simulation by exactly
one `tick_quantum` on every worldline, without checking per-tick
metadata or consulting a compatibility matrix.

## Agent users / jobs / hills

### Primary agent users

- Agents writing time-aware adapters or session protocol code
- Agents implementing strand/settlement specs downstream

### Agent jobs

1. Assume fixed dt when generating time-dependent code.
2. Never emit per-tick dt fields in provenance entries.
3. Gate cross-worldline operations on `tick_quantum` equality.

### Agent hill

An agent can assume tick-comparability across worldlines by comparing
a single genesis parameter (`tick_quantum`), without inspecting
individual ticks.

## Human playback

1. The human opens `docs/invariants/FIXED-TIMESTEP.md`.
2. The document states seven normative rulings (listed above).
3. The human asks: "are tick numbers comparable across these two
   worldlines?" Answer: yes, if their `tick_quantum` is identical
   (which it must be for compare/settle in v1).
4. The human asks: "can my adapter use wall-clock time?" Answer: yes,
   for pacing and telemetry, but it must emit a canonical decision
   record before the simulation consumes the result.
5. The human asks: "what does catch-up mean?" Answer: run N fixed
   ticks, not one big tick.

## Agent playback

1. The agent reads the invariant document.
2. The agent confirms: no per-tick dt field exists in
   `ProvenanceEntry` or `WorldlineTickPatchV1`.
3. The agent confirms: `tick_quantum` is a worldline-genesis
   parameter, not a per-tick value.

## Implementation outline

1. Create `docs/invariants/` directory.
2. Write `docs/invariants/FIXED-TIMESTEP.md` with the seven normative
   rulings, rationale, and consequences.
3. Cross-reference from SPEC-0004 (worldlines) — add a one-line
   normative reference to the invariant.
4. Cross-reference from `CONTINUUM.md` — note that the hot runtime's
   time model is fixed-quantum.
5. Verify that `warp_geom::Tick` doc comment is consistent with the
   invariant (it already says "fixed `dt` per branch").
6. Update the strand-contract and strand-settlement backlog items
   to note that cross-worldline operations require identical
   `tick_quantum`.

## Tests to write first

- Shell assertion: `docs/invariants/FIXED-TIMESTEP.md` exists.
- Shell assertion: the invariant document contains "MUST" and
  "tick_quantum".
- Shell assertion: the invariant document contains all seven rulings
  (grep for key phrases: "immutable tick_quantum", "not an admitted
  stream fact", "never stored per tick", "tick-denominated",
  "canonical decision", "identical tick_quantum").
- Shell assertion: SPEC-0004 contains a reference to the invariant.
- Shell assertion: no file in `crates/` contains "variable.dt" or
  "variable_dt" or "dt_stream" (negative test — the concept does not
  exist in code).

## Risks / unknowns

- **Risk: future use case needs variable dt.** If a real consumer
  appears, the invariant can be relaxed in a future cycle with
  explicit design work. But we do not design for hypothetical
  requirements. Fixed until proven otherwise.
- **Risk: tick_quantum choice is premature.** The invariant declares
  that `tick_quantum` exists and is immutable, not what its value is.
  Actual quantum values are a runtime configuration concern for each
  application.

## Postures

- **Accessibility:** Not applicable — spec-only, no UI.
- **Localization:** Not applicable — internal invariant document.
- **Agent inspectability:** The invariant is a set of normative
  statements parseable by grep.

## Non-goals

- Choosing a specific `tick_quantum` value (application concern).
- Runtime enforcement of the invariant (structural — no variable-dt
  mechanism exists to enforce against).
- HistoryTime/HostTime field classification table (TT0 scope, not
  this cycle — but this cycle establishes the boundary that TT0
  classifies against).
- The strand contract itself (next cycle).
- Settlement semantics (future cycle after strand contract).
