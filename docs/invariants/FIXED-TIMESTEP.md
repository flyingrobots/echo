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
influence simulation semantics directly. HostTime MUST NOT directly
affect admission, commit identity, read identity, replay outcome, or
causal ordering. HostTime MAY influence semantics only through a
recorded canonical decision — an adapter emits a deterministic
decision record before the simulation consumes the result. The
admitted decision record is the artifact of record, not the
wall-clock value that motivated it.

### R7 — Cross-worldline operations require identical tick_quantum

Cross-worldline compare and settlement MUST require identical
`tick_quantum` between the participating worldlines. Operations
between worldlines with different quanta MUST be rejected in v1.
Equal tick numbers represent equal elapsed simulation time only
when the quanta match.

## Time field classification

Echo distinguishes deterministic causal time from host-observed time.

- **HistoryTime** names deterministic causal coordinates: ticks,
  worldline append positions, runtime scheduler cycle coordinates,
  receipt ticks, and tick-denominated deadlines.
- **HostTime** names wall-clock, monotonic host clocks, browser
  timestamps, adapter-local timestamps, pacing durations, logging
  timestamps, and UI telemetry.

This classification is about semantic authority. A HistoryTime field
may still be diagnostic metadata rather than a commit hash input; a
HostTime field may still be useful telemetry. The boundary is that
HostTime is never consumed as deterministic history unless it first
becomes an admitted canonical decision record.

| Surface / field                                                        | Class       | Rationale                                                                  |
| ---------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------- |
| `WorldlineTick`                                                        | HistoryTime | Per-worldline logical append coordinate; explicitly not wall-clock time.   |
| `GlobalTick`                                                           | HistoryTime | Runtime-cycle logical correlation coordinate; no wall-clock semantics.     |
| `SchedulerStatus.latestCycleGlobalTick`                                | HistoryTime | Reports the latest runtime scheduler cycle coordinate.                     |
| `SchedulerStatus.latestCommitGlobalTick`                               | HistoryTime | Reports the scheduler cycle coordinate that produced the latest commit.    |
| `SchedulerStatus.lastQuiescentGlobalTick`                              | HistoryTime | Reports the scheduler cycle coordinate at quiescence.                      |
| `TtdrHeader.tick`                                                      | HistoryTime | Tick receipt coordinate for witnessed deterministic verification.          |
| TTD protocol `tick` / `fromTick` / `toTick` / `targetTick` fields      | HistoryTime | Cursor, seek, violation, snapshot, and truth-frame coordinates.            |
| TTD protocol `initialTick` / `finalTick` fields                        | HistoryTime | Cursor lifecycle tick coordinates, not host timestamps.                    |
| TTD protocol `deadlineTick` fields                                     | HistoryTime | Deadlines are tick-denominated semantic time.                              |
| Legacy `OpEnvelope.ts`                                                 | HostTime    | Monotonic per-host transport timestamp; must not order causal history.     |
| Generated TTD protocol `timestamp` / `Timestamp` fields                | HostTime    | Milliseconds-since-epoch event telemetry; not replay/admission authority.  |
| Hook, CI, and verification timing fields such as `elapsed_ms` or dates | HostTime    | Tooling telemetry and audit logs; outside the deterministic history plane. |

## Static enforcement

The current static guard is `scripts/ban-nondeterminism.sh`. It scans
determinism-critical crate paths and bans wall-clock and pacing APIs
including `std::time::SystemTime`, `SystemTime::now`,
`std::time::Instant`, `Instant::now`, `std::thread::sleep`, and
async runtime sleep calls. The release allowlist rules in
`docs/determinism/RELEASE_POLICY.md` require every exemption to prove
that the nondeterministic API cannot reach the deterministic engine
loop.

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
