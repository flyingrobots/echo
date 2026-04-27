<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Merkle Commit

_Define the state and commit hashes that let Echo retain a witnessed history without retaining observer interpretations as canonical state._

Legend: KERNEL

Depends on:

- [WARP Tick Patch](warp-tick-patch.md)
- [SPEC-0001 - Attachment Plane v0 Typed Atoms](SPEC-0001-attachment-plane-v0-atoms.md)
- [SPEC-0002 - Descended Attachments v1](SPEC-0002-descended-attachments-v1.md)

## Why this packet exists

Echo needs stable hash commitments for replay, slicing, and audit. The hash boundary must commit to the witnessed carrier and replayable delta, while leaving observer-relative readings and diagnostic narration outside the commit unless a later version explicitly adds them.

## Human users / jobs / hills

Human users need hash mismatches to mean something concrete.

The hill: when a state root or commit id changes, a reviewer can trace the change to reachable state, patch bytes, parents, or policy id.

## Agent users / jobs / hills

Agent users need deterministic equality tests.

The hill: an agent can rebuild a state from retained patches and compare the same `state_root` and commit id without access to the original process.

## Decision 1: `state_root` commits to reachable WARP state

The state root is BLAKE3 over canonical encoding of reachable WARP state from a root `NodeKey`. Reachability follows outbound skeleton edges and descended attachment portals from reachable node/edge slots.

## Decision 2: `commit_id` v2 commits to replay, not narration

Commit header v2 commits to version, parents, state root, patch digest, and policy id. `patch_digest` is the digest of the replayable tick patch.

## Decision 3: Diagnostic digests are retained but not committed by v2

`plan_digest`, `decision_digest`, and `rewrites_digest` are deterministic diagnostics. They are useful witnesses, but commit hash v2 does not include them.

## Decision 4: Empty list digests are length-prefixed

For length-prefixed list digests, `EMPTY_LEN_DIGEST = blake3(0u64.to_le_bytes())`. It is not `blake3(b"")`.

## Decision 5: Readings stay outside the commit unless promoted

Observation artifacts, view frames, and session packets are readings over the carrier. They may have their own hashes, but they are not part of `commit_id` v2.
