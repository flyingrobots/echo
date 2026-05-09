<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Runtime Model

_Echo is a deterministic WARP runtime for witnessed causal history and bounded observation._

Core doctrine: [there is no graph](there-is-no-graph.md). Graph-like structure
is an observer-relative holographic reading over witnessed causal history, not a
canonical substrate-owned object.

Echo itself is a WARP optic for real-time deterministic simulation. It admits,
observes, retains, and reveals witnessed causal history through its local
runtime law; it is not an implementation of a hidden global graph.

## What Echo owns

Echo owns the hot runtime path:

- carrier state in `warp-core`
- deterministic rewrite settlement
- replayable tick patches
- Merkle commitments over state and patch boundaries
- worldline/provenance retention
- observation artifacts for public reads
- WASM and session protocols that expose readings over the carrier

Echo does not own every possible platform noun around WARP. This repo's live docs should describe the implemented Echo runtime first, then name future or external surfaces only when there is a concrete boundary.

## Core nouns

Carrier state: the WARP state held by `warp-core`.

WARP optic: a bounded, capability-scoped, law-named operation over causal
history. It may admit a transition, observe a projection, slice a hologram, or
retain/reveal an artifact.

Hologram: the witnessed output of a WARP optic. A hologram carries enough basis,
law, aperture, evidence, identity, and posture to recreate the claimed object
up to the equivalence relation declared by the optic law.

Witness: the retained evidence that a transition or reading came from a specific state, policy, patch, or coordinate.

Shell: a retained boundary artifact such as a tick patch, provenance payload, or boundary record.

Reading: an observer-relative artifact emitted from a coordinate, frame, and projection. Readings are public API values; they are not raw carrier state.

Worldline: the retained ordered history used for replay, slices, and coordinate-based observation.

## Runtime flow

1. External callers submit canonical intents.
2. Inbox sequencing derives content identity and canonical pending order.
3. Rules propose candidate rewrites with explicit footprints.
4. The scheduler admits a deterministic independent subset.
5. The engine applies admitted rewrites and emits a snapshot, receipt, and tick patch.
6. Worldline/provenance stores retain patches and expected hashes.
7. Observation services resolve coordinates and emit readings.

## Current implementation anchors

- [warp-core](../spec/warp-core.md)
- [Canonical Inbox Sequencing](../spec/canonical-inbox-sequencing.md)
- [WARP Rewrite Scheduler](../spec/scheduler-warp-core.md)
- [WARP Tick Patch](../spec/warp-tick-patch.md)
- [Merkle Commit](../spec/merkle-commit.md)
- [Worldlines, Playback, and Observation](../spec/SPEC-0004-worldlines-playback-truthbus.md)
- [WASM ABI Contract](../spec/SPEC-0009-wasm-abi.md)
- [There Is No Graph](there-is-no-graph.md)
- [Continuum Transport](continuum-transport.md)

## Design posture

Describe Echo from code-backed surfaces outward: kernel before platform, witnessed history before graph-shaped view, replayable shell before explanation, observation artifact before UI state, and current ABI before historical ABI numbers.
