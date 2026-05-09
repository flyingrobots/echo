<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0004 - Worldlines, Playback, and Observation

_Define worldlines as retained witnessed history and playback as an observer viewpoint over that history._

Legend: PLATFORM / KERNEL

Depends on:

- [WARP Tick Patch](warp-tick-patch.md)
- [Merkle Commit](merkle-commit.md)
- [Provenance Payload](SPEC-0005-provenance-payload.md)
- [WASM ABI Contract](SPEC-0009-wasm-abi.md)
- [FIXED-TIMESTEP](../invariants/FIXED-TIMESTEP.md)

## Why this packet exists

Earlier drafts centered TruthBus as the public read story. The current Echo language is sharper: the runtime retains witnessed history, and public reads are observer-relative observation artifacts. Playback cursors and session helpers are implementation surfaces that materialize a viewpoint; they are not the semantic center.

## Human users / jobs / hills

Human users need time travel to feel like a debugger, not like a second simulation.

The hill: seeking to a prior tick applies recorded patches, verifies expected hashes, and emits the same observer reading that was recorded for that tick.

## Agent users / jobs / hills

Agent users need reproducible coordinates for "show me this state at that frontier."

The hill: an agent can name a worldline coordinate, request an observation frame, and receive an artifact with resolved coordinate metadata and a reading envelope.

## Decision 1: Worldline is the retained history boundary

A worldline retains enough material to reconstruct state and recorded readings: initial state or checkpoint reference, ordered tick patches, expected hash triplets, and recorded output frames where applicable.

The worldline is not itself the observer. It is the carrier that makes replay, audit, slicing, and observation possible.

## Decision 2: PlaybackCursor is a viewpoint

A playback cursor materializes a worldline at a coordinate without mutating the writer head unless it is explicitly acting as the writer. Seeking replays recorded patches and verifies expected hashes. It does not re-run rules.

Playback coordinates follow the [FIXED-TIMESTEP](../invariants/FIXED-TIMESTEP.md)
invariant: ticks are HistoryTime coordinates, and HostTime cannot affect replay
or coordinate identity except through an admitted canonical decision record.
Timer starts, fires, expiries, and cancellations follow the same law: an Intent
is only a proposal, and only an admitted tick plus receipt becomes replayable
timer history.

## Decision 3: Observation is the public read contract

Public reads are expressed through observation artifacts: coordinate resolution, reading-envelope metadata, declared frame, declared projection, artifact hash, and payload. Observation is a reading emitted from an observer basis, not raw access to the causal carrier.

The reading envelope is part of the contract, not decoration: it carries the
observer plan, optional hosted observer instance, native basis, witness refs,
parent/basis posture, budget posture, rights posture, and residual posture that
bound the emitted reading.

## Decision 4: Session output is replace-only

Client-facing frames are authoritative values for a coordinate and channel. A client replaces rendered state from the received reading; it does not infer rollback, replay, or hidden diffs.

Older TruthBus naming may still appear in compatibility code, but the documented concept is observation over retained history.

## Decision 5: Retention cannot break verification

Retention policies may keep all patches, checkpoint periodically, or archive older history through a future transport layer. They must not destroy the ability to verify a retained coordinate.

## Consequences

Worldlines carry witnessed history. Playback cursors are observer viewpoints. Observation artifacts are the public read surface. Recorded outputs are retained readings that make playback byte-identical where the runtime promises byte-identical output.
