<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Security/capabilities for fork/rewind/merge

Ref: #246

Status: planned kernel/runtime design.

This is the canonical Echo follow-up for timeline capability law. Echo owns the
capability decision, the typed denial/obstruction posture, and the witnessed
admission result. `warp-ttd` owns debugger session semantics, transport, and UI
surfaces that consume those Echo results.

## Why now

Fork, seek, rewind, merge, settlement, and counterfactual execution are not
ordinary host actions. They expose causal authority over worldlines, strands,
braids, and retained readings. The kernel needs a small, typed capability model
before those operations become public surfaces.

## Required shape

Define capability checks for:

- opening an observer at a coordinate or frontier
- seeking a view to an older coordinate
- creating a strand or fork from a coordinate
- dispatching intents into a forked strand
- admitting, staging, rejecting, or collapsing divergent work
- merging or settling a strand/braid back into another frontier
- revealing retained readings or witness material

The capability model must name:

- actor/cause identity
- session or host authority scope
- subject/focus being acted on
- coordinate/frontier basis
- rights being exercised
- denial and obstruction codes
- receipt or witness evidence emitted on success

## Acceptance criteria

- Capability names and denial/obstruction codes are documented for seek, fork,
  dispatch, merge, settlement, and witness reveal.
- Per-session and per-actor grants can be represented without relying on host
  wall-clock ordering or mutable global state.
- Revocation behavior is explicit: active forks/strands become staged,
  obstructed, or quarantined by typed posture; they are not silently destroyed.
- Provenance sovereignty is stated as a normative rule: a branch or strand
  carries actor/cause evidence, and settlement requires authority over the
  target frontier.
- A future `warp-ttd` adapter can ask Echo what capabilities exist and can show
  typed denials, but does not become the source of kernel truth.

## Non-goals

- Do not design a debugger panel or debugger protocol here.
- Do not add a global mutable state API.
- Do not make rewinds or merges host-time ordered.
- Do not collapse capability failure into boolean success or string status.
