<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0008: Worldline Runtime Model

- **Status:** Accepted
- **Date:** 2026-03-09
- **Qualified by:** [ADR 0010](ADR-0010-observational-seek-and-administrative-rewind.md) and [ADR 0018](0018-sessions-causal-posture-and-authority.md)

## Context

Echo records witnessed causal history per worldline. Playback, scheduling,
observation, forking, and recovery need one account of which coordinates exist,
which actors may advance them, and which operations merely read recorded
history. A global mutable timeline or debugger-owned rewind would make those
boundaries implicit and would let one consumer rewrite unrelated history.

## Decision

### Worldlines and heads

A worldline is a core runtime identity with its own admitted causal history and
frontier. It is not a UI session, materialized graph, or debugger construct.

A writer head may propose advancement of its bound worldline through the
trusted runtime's deterministic admission and commit path. A reader head may
seek and replay already admitted history but cannot advance or reposition the
worldline frontier. Seeking beyond the frontier clamps or obstructs according
to the observation contract; it never synthesizes future history.

Multiple heads may name one worldline. Head-local observation or playback does
not mutate other heads or worldlines. Administrative rewind is a separate,
explicit maintenance authority governed by ADR 0010, not the default playback
operation.

### Scheduling

The scheduler considers runnable writer heads in a canonical order derived from
stable worldline and head identities. Paused, obstructed, or unauthorized heads
do not advance.

Writers targeting the same worldline may execute concurrently only when their
admitted work is footprint-independent. Interfering work has a deterministic
serial order. Host worker count, timing, and map iteration cannot affect
admission, commit order, receipt bytes, or frontier identity.

Reader heads do not participate in commit scheduling. Their reads lower through
the explicit observation and optic boundaries and retain the coordinate and
evidence posture of the history they inspect.

### Admission and authority

External consumers propose explicit-basis intents. The trusted runtime admits,
stages, pluralizes, conflicts, or obstructs those proposals under named law.
Product adapters and tools cannot mutate a worldline through a private state
setter or a UI-local timeline.

Internal engine mutation helpers may exist for trusted implementation and test
construction, but they do not define an external application authority.
Duplicate suppression is scoped to the resolved causal target; equal payload
bytes alone do not make two differently based proposals identical.

### History and replay

Admitted causal history is append-only at the worldline boundary. Replay reads
recorded commits, patches, receipts, witnesses, and retained boundary material;
it does not rerun a live scheduler to invent missing history.

Forking shares a witnessed prefix and creates an independently advancing
suffix. Per-worldline tick is an append coordinate. Any cross-worldline clock is
correlation metadata and cannot impersonate a worldline-local causal position.

## Required Invariants

- A worldline frontier advances monotonically through admitted commits.
- A writer head advances only its bound worldline and only under scheduler
  authority.
- A reader head never mutates a frontier or synthesizes future history.
- Head-local seek and observation never rewind unrelated worldlines.
- Runnable-writer order and interference resolution are deterministic.
- Equivalent admitted inputs over the same basis produce equal receipts and
  committed identities.
- Scheduler order is independent of wall clock, worker count, and host map
  iteration.
- Fork identity binds the witnessed prefix from which the new suffix descends.

## Rejected Alternatives

- One global mutable timeline shared by every worldline and consumer.
- Debugger- or UI-owned rewind that mutates runtime history.
- Reader playback implemented by running unrecorded scheduler steps.
- Host timing or CPU count as a canonical scheduling input.
- Treat a session, materialized view, or equal payload as a causal coordinate.

## Consequences

- Worldline and head identities are explicit in scheduling, observation,
  playback, receipts, and recovery.
- Tooling remains a consumer of runtime authority rather than a privileged
  mutator.
- Concurrent writer execution requires honest footprints and deterministic
  interference handling.
- Observational seek and administrative maintenance remain separate APIs and
  evidence postures.
- Cross-worldline exchange must use witnessed causal transport rather than
  shared mutable state.

## Evidence Anchors

- `crates/warp-core/src/head.rs`
- `crates/warp-core/src/coordinator.rs`
- `crates/warp-core/src/worldline.rs`
- `crates/warp-core/src/worldline_state.rs`
- `crates/warp-core/src/provenance_store.rs`
- `crates/warp-core/src/observation.rs`
- `docs/adr/ADR-0010-observational-seek-and-administrative-rewind.md`
- `docs/adr/0018-sessions-causal-posture-and-authority.md`

## Historical Note

The original record included a normative implementation order, dated file and
status inventory, application examples, test-plan table, critical-path claims,
and document-amendment procedure. Those process artifacts remain in Git history
and are not architectural authority.
