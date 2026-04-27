<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Theory Map

_A compact map from the latest AIΩN / observer-geometry vocabulary to the Echo runtime._

## Read this as a map

This page is not a replacement for the papers. It names the concepts Echo uses in current docs and points to the runtime surfaces that carry them.

## Primary shift

Echo should not be described as "a graph of graphs" first. The better frame is:

```text
witnessed causal history -> retained shells -> observer-relative readings
```

Graph-shaped objects still matter. They are the carrier shape and many readings are graph-shaped. But the causal claim lives in witnessed history and retained boundary artifacts, not in a diagram alone.

## Runtime vocabulary

Carrier: the WARP state held by `warp-core`.

Worldline: an ordered retained history of patches and expected hashes.

Strand: a constrained lane of worldline-adjacent history with inherited time and settlement rules.

Patch: the replayable delta shell for a tick.

Receipt: the settlement witness explaining accepted and rejected candidates.

Provenance payload: a retained shell for replay and "show me why" queries.

Observation artifact: an observer-relative reading with coordinate, frame, projection, reading-envelope metadata, payload, and artifact hash.

Reading envelope: metadata that makes the observer posture explicit at the ABI boundary.

## WARP optic alignment

The operative runtime shape is:

```text
Lower(frontier, weave) = (Outcome, Witness, Shell)
```

In Echo terms: the frontier is the carrier coordinate, the weave is admitted work under policy, the outcome is the committed transition or rejection, the witness is receipt/hash/coordinate evidence, and the shell is the retained replay artifact.

## Implementation anchors

- [Echo Runtime Model](../architecture/outline.md)
- [warp-core](../spec/warp-core.md)
- [WARP Tick Patch](../spec/warp-tick-patch.md)
- [Merkle Commit](../spec/merkle-commit.md)
- [Worldlines, Playback, and Observation](../spec/SPEC-0004-worldlines-playback-truthbus.md)
- [Provenance Payload and Boundary Records](../spec/SPEC-0005-provenance-payload.md)
- [WASM ABI Contract](../spec/SPEC-0009-wasm-abi.md)

## Documentation rule

Use "graph" when naming carrier structure or a graph-shaped reading. Use "witness", "shell", "worldline", "strand", "observation", and "reading" when describing the causal and epistemic contract.
