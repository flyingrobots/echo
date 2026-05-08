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

All TTL, deadline, retry, and expiry semantics MUST be expressed in
HistoryTime: ticks, epochs, causal coordinates, or admitted timer
events. Wall-clock durations MUST NOT appear in semantic state as
hidden expiry authority. Timers are expressed as tick counts, epoch
counts, or admitted timer-event history.

Timers are not mutable host-local handles. A timer request is an
Intent. Submitting the Intent does not start the semantic timer. The
scheduler/admission path MUST choose an explicit admission outcome, and
only an admitted timer-start tick arms the semantic timer. A timer
firing, expiry, or cancellation is also an Intent. It becomes semantic
only if admitted against an explicit causal basis that names the
admitted start/request receipt.

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

## Timer and deadline doctrine

Timer causality is admitted, not observed implicitly.

The safe timer pattern is:

1. An adapter or contract submits a timer Intent.
2. Echo validates capability, causal basis, and admission law.
3. The scheduler/admission path returns a typed outcome such as
   `Admitted`, `Staged`, `Plural`, `Conflict`, or `Obstructed`.
4. Only an admitted `timer.start` tick arms the semantic timer.
5. A later adapter observation MAY cause submission of `timer.fire`,
   `timer.expire`, or `timer.cancel`.
6. Only an admitted fire/expire/cancel tick changes semantic history.
7. Replay consumes the admitted ticks and receipts. Replay MUST NOT
   recalculate elapsed wall-clock time.

HostTime may cause an adapter to propose timer Intents. HostTime is
not itself the semantic decision. The admitted tick and its receipt are
the artifact of record.

### Worked example

```text
C100: coordinate before timer request
I0:   Intent(timer.start,
             timer_id = A,
             requested_delay_hint = 5s,
             base = C100)
O0:   AdmissionOutcome::Admitted(tick = T100, receipt = R100)

Host wall clock later wakes the adapter.

I1:   Intent(timer.fire,
             timer_id = A,
             start_receipt = R100,
             observed_host_delay_hint = 5.02s,
             base = C149)
O1:   AdmissionOutcome::Admitted(tick = T150, receipt = R150)
```

The semantic facts are `T100` and `T150`. The wall-clock delay may
explain why the adapter proposed `I1`, but replay, rewind, fork, and
read identity consume only the admitted timer ticks and receipts.

If `I0` is `Staged`, `Conflict`, or `Obstructed`, timer `A` is not
armed. If `I1` cites a missing, stale, conflicting, or unadmitted
start receipt, Echo MUST reject, stage, preserve plurality, or return a
typed obstruction. It MUST NOT silently mutate the latest frontier.

A paused observer view does not advance its observed HistoryTime
coordinate. A tick-based TTL in that view does not expire merely
because host wall-clock time passed. A live writer may continue to
admit new ticks on its own frontier, but any reading must name the
coordinate whose timer state it observes.

### TTL and deadline touch points

| Touch point                     | Semantic rule                                                                                                                                       |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Session keep-alive              | Transport pings are HostTime telemetry. Semantic liveness requires an admitted heartbeat, timeout, or obstruction event.                            |
| Admission budgets               | Semantic budgets are ticks, operations, bytes, fuel, or explicit admission posture. Host-time execution limits are tooling guards, not state facts. |
| Retry policies                  | Retry eligibility is a tick/epoch coordinate or an admitted retry Intent. A wall-clock sleep may only trigger proposal of that Intent.              |
| Wormhole/checkpoint retention   | Semantic validity is tied to retained receipts, ranges, and witness basis. HostTime retention may evict cache bytes only if reads fail closed.      |
| Cached and retained readings    | Cache age is operational. A retained reading remains valid only for its read identity, coordinate, witness basis, versions, aperture, and posture.  |
| Adapter-driven real-time timers | Wall-clock wakeups may propose timer fire/expire Intents. Only admitted fire/expire ticks affect replayable semantics.                              |

### Violation checklist

Treat any of the following as a violation until proven otherwise:

- Replay, reducer, query, admission, or scheduler code calls `now()`,
  `Instant::now`, `SystemTime::now`, sleep, browser timestamps, or
  host frame time to decide semantic expiry.
- A committed artifact stores `expires_at_ms`, `deadline_unix_ms`,
  wall-clock duration, or similar HostTime fields as causal authority.
- A timer fire/expiry path assumes submission success instead of
  checking for an admitted start receipt and admitted fire receipt.
- A cache or retention TTL returns stale bytes as a live reading
  without naming the original read identity and coordinate.
- Missing retained history is treated as successful expiry or successful
  replay instead of returning a typed obstruction.

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
- [Continuum foundations](../architecture/continuum-foundations.md) — archived
  bridge note for older Continuum framing
- `warp_geom::Tick` — code-level precedent
