<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0003 — Lock the dt policy

_Ratify fixed timestep as the default and define variable-dt as an
opt-in admitted stream with explicit braidability constraints._

Legend: KERNEL

Depends on:

- nothing

## Why this cycle exists

The dt policy is the single decision that gates all downstream time
work: strands, settlement, time travel, and the debugger's
fork-compare workflow. Every open design question in the TT0–TT3
sequence implicitly depends on whether tick N on worldline A represents
the same elapsed simulation time as tick N on worldline B.

The backlog already leans heavily toward fixed timestep. The TT1 item
practically writes the ruling. But the decision has never been formally
ratified, so every downstream spec either assumes it informally or
hedges around it. This cycle locks it.

This is a spec-only cycle. No runtime code. The deliverable is a
normative section in `docs/spec/SPEC-0010-dt-policy.md` and a
classification of dt implications for strand and settlement work.

## Human users / jobs / hills

### Primary human users

- Engine contributors implementing time-aware systems
- Game designers choosing time models for their simulations
- Debugger developers building fork/compare workflows

### Human jobs

1. Know whether tick numbers are comparable across worldlines.
2. Know when variable-dt is available and what constraints it carries.
3. Know the exact braidability rule for variable-dt lanes.

### Human hill

A contributor can read one document and know whether their
time-dependent code will replay correctly across worldlines, without
digging through scattered backlog items or chat transcripts.

## Agent users / jobs / hills

### Primary agent users

- Agents writing time-aware adapters or session protocol code
- Agents implementing strand/settlement specs downstream

### Agent jobs

1. Determine which dt model applies to a given worldline.
2. Determine whether two worldlines are settlement-eligible based on
   their dt configuration.

### Agent hill

An agent can read the spec and programmatically determine whether two
worldlines have compatible dt configurations for comparison or
settlement.

## Human playback

1. The human opens `docs/spec/SPEC-0010-dt-policy.md`.
2. Section 1 states: fixed timestep is the default. Every tick is the
   same simulated duration.
3. Section 2 states: variable-dt is opt-in, admitted as a stream fact
   with its own `StreamAdmissionDecision` entry.
4. Section 3 states the braidability rule: variable-dt lanes are
   replayable and comparable, but not braidable or settleable unless
   their dt streams are identical.
5. The human can answer "can these two worldlines be settled?" without
   reading any other document.

## Agent playback

1. The agent reads the spec.
2. The spec contains a normative decision table: `{ dt_mode, replay,
compare, braid, settle }`.
3. The agent determines eligibility by matching worldline dt
   configurations against the table.

## Implementation outline

1. Create `docs/spec/SPEC-0010-dt-policy.md`.
2. Write Section 1: Fixed Timestep Default — every tick is a fixed
   simulated duration; the duration is a worldline-level configuration
   parameter, not a per-tick value.
3. Write Section 2: Variable-dt Opt-in — variable-dt is an admitted
   stream with its own `StreamAdmissionDecision`; the dt value is
   recorded in provenance; dt values feed into the admission digest
   chain.
4. Write Section 3: Braidability Rule — variable-dt lanes can be
   replayed and compared, but are not braidable or settleable unless
   their dt streams are identical (same elapsed simulation time per
   tick). This is because composing different elapsed time behind the
   same tick numbers is incoherent.
5. Write Section 4: Implications — downstream consequences for strand
   contract, settlement spec, and TTD fork/compare workflow. This
   section is informative, not normative.
6. Cross-reference from the HistoryTime/HostTime classification work
   in the TT0 backlog (T-7-1-1).

## Tests to write first

- Shell assertion: `SPEC-0010-dt-policy.md` exists and contains the
  normative keyword "MUST" in all three ruling sections.
- Shell assertion: the decision table contains rows for both
  `fixed` and `variable` dt modes.
- Shell assertion: the braidability rule explicitly states that
  variable-dt lanes are not settleable unless dt streams are identical.
- Shell assertion: the spec cross-references SPEC-0004 (worldlines)
  and SPEC-0005 (provenance payload).

## Risks / unknowns

- **Risk: variable-dt opt-in may be premature.** We have no consumer
  yet. Mitigation: the spec declares it opt-in and defers runtime
  implementation. If no consumer appears, the variable-dt section
  remains a reserved extension point.
- **Unknown: exact fixed timestep duration.** The spec should declare
  that the duration is a worldline parameter, not hardcode a value.
  Specific durations are a runtime configuration concern.

## Postures

- **Accessibility:** Not applicable — spec-only, no UI.
- **Localization:** Not applicable — internal spec document.
- **Agent inspectability:** The decision table is structured data
  that an agent can parse. The spec uses normative language (MUST,
  SHOULD, MAY) per RFC 2119 convention.

## Non-goals

- Runtime enforcement of the dt policy (future cycle).
- Changes to `echo-session-proto` wire format.
- Implementation of variable-dt `StreamAdmissionDecision` (deferred
  until a consumer exists).
- The strand contract itself (next cycle: 0004).
- Settlement semantics (future cycle after strand contract).
