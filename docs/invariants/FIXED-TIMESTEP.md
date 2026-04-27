<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# FIXED-TIMESTEP

**Status:** Normative | **Legend:** KERNEL | **Cycle:** 0003

## Invariant

Echo uses a fixed timestep. Every worldline has an immutable
`tick_quantum` chosen at genesis, and every committed tick advances
simulation by exactly that quantum. Per-tick variable `dt` does not
exist.

## Rulings

The following rulings are normative. "MUST" and "MUST NOT" follow
RFC 2119 convention.

### R1 — Immutable tick_quantum at genesis

Every worldline MUST have an immutable `tick_quantum` parameter fixed
at worldline creation. The quantum defines the simulated duration of
one tick for that worldline. It MUST NOT change after genesis.

### R2 — One quantum per tick

Each committed tick MUST advance simulation by exactly one
`tick_quantum`. There is no mechanism for a tick to represent a
different duration.

### R3 — dt is not an admitted stream fact

`dt` MUST NOT be an admitted stream fact. No `StreamAdmissionDecision`
carries a per-tick time delta. The simulation does not consume
variable time deltas from any source.

### R4 — dt is never stored per tick

`dt` is never stored per tick in provenance, patches, or any committed
artifact. `ProvenanceEntry`, `WorldlineTickPatchV1`, and all
downstream structures MUST NOT contain a per-tick dt field.

### R5 — Tick-denominated time semantics

All TTL, deadline, retry, and expiry semantics MUST be
tick-denominated. Wall-clock durations MUST NOT appear in semantic
state. Timers are expressed as tick counts or epoch counts.

### R6 — HostTime enters only through canonical decisions

HostTime (wall-clock, frame time, real-time telemetry) MUST NOT
influence simulation semantics directly. HostTime MAY influence
semantics only through a recorded canonical decision — an adapter
emits a deterministic decision record before the simulation consumes
the result. The decision record is the artifact of record, not the
wall-clock value that motivated it.

### R7 — Cross-worldline operations require identical tick_quantum

Cross-worldline compare and settlement MUST require identical
`tick_quantum` between the participating worldlines. Operations
between worldlines with different quanta MUST be rejected in v1.
Equal tick numbers represent equal elapsed simulation time only
when the quanta match.

## Rationale

Echo's hardest open problem is canonical cross-worldline settlement:
one deterministic result, not eventual convergence. If ticks can carry
different durations, equal tick counts stop meaning equal simulated
time. Compare, braid, and settle all become incoherent because you are
composing different elapsed simulation time behind the same tick
numbers.

The code already embodies this invariant. `warp_geom::Tick` documents
"the engine advances in integer ticks with a fixed `dt` per branch."
The scheduler, provenance store, and playback cursor all treat ticks
as uniform integers with no per-tick metadata.

## Consequences

- Tick numbers are directly comparable across worldlines with the
  same `tick_quantum`.
- Strands inherit their parent's `tick_quantum` at fork time. No
  strand can change its quantum.
- Catch-up means running 0, 1, or N fixed ticks in one host frame —
  not "one larger tick." Wormhole checkpoints mark tick boundaries,
  not time boundaries.
- Replay is structurally sound without recording per-tick time deltas.
  The quantum is a worldline parameter, not a per-entry field.
- No variable-dt plumbing needs to exist in the codebase. If a future
  use case demands variable dt, it requires a new design cycle to
  relax this invariant with explicit constraints.

## Cross-references

- [SPEC-0004 — Worldlines, Playback, and Observation](../spec/SPEC-0004-worldlines-playback-truthbus.md)
- [SPEC-0005 — Provenance Payload](../spec/SPEC-0005-provenance-payload.md)
- `CONTINUUM.md` — repo-root hot runtime time model
- `warp_geom::Tick` — code-level precedent
