<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Merkle Commit

_Define the state and commit hashes that let Echo retain a witnessed history without retaining observer interpretations as canonical state._

Depends on:

- [WARP Tick Patch](warp-tick-patch.md)
- [SPEC-0001 - Attachment Plane v0 Typed Atoms](SPEC-0001-attachment-plane-v0-atoms.md)
- [SPEC-0002 - Descended Attachments v1](SPEC-0002-descended-attachments-v1.md)

## Purpose

Echo needs stable hash commitments for replay, slicing, and audit. The hash boundary must commit to the witnessed carrier and replayable delta, while leaving observer-relative readings and diagnostic narration outside the commit unless a later version explicitly adds them.

Hash mismatches must have a concrete explanation: when a state root or commit id
changes, a reviewer can trace the change to reachable state, patch bytes,
parents, policy identity, or, for commit hash v3, the tick receipt digest.

Independent verification must be deterministic. A verifier can rebuild state
from retained patches and compare the same `state_root` and commit id without
access to the original process.

## Decision 1: `state_root` commits to reachable WARP state

The state root is BLAKE3 over canonical encoding of reachable WARP state from a root `NodeKey`. Reachability follows outbound skeleton edges and descended attachment portals from reachable node/edge slots.

Both canonical snapshots and incremental snapshot accumulation prepend the existing `echo:state_root:v1\0` domain separator before the root binding. Incremental accumulation must produce the same state root as canonical snapshot hashing for the same reachable state. Previously emitted accumulator roots that omitted this prefix do not satisfy that law; this correction does not migrate retained artifacts.

This is the `LegacyReachableV1` commitment profile, which every worldline uses by default. The experimental, off-by-default `application_root_profile` feature lets a worldline be registered with `ApplicationRadixV0`, whose `state_root` is the domain-tagged application root instead; see [Experimental commitment profile](../topics/ApplicationRoot.md#experimental-commitment-profile). Commit hash recipes are the same under either profile.

## Decision 2: `commit_id` v2 commits to replay, not narration

Commit header v2 commits to version, parents, state root, patch digest, and policy id. `patch_digest` is the digest of the replayable tick patch.

## Decision 3: Diagnostic digests are retained but not committed by v2

`plan_digest`, `decision_digest`, and `rewrites_digest` are deterministic diagnostics. They are useful witnesses, but commit hash v2 does not include them.

## Decision 3a: `commit_id` v3 binds a commit-bound tick receipt

Commit header v3 commits to the domain tag `echo:commit_id:v3`, version 3,
parents, state root, patch digest, the tick receipt digest (`decision_digest`),
and policy id (`compute_commit_hash_v3`). It exists so that a tick's attribution
section is part of its commit identity: commit hash → receipt digest →
attribution (#873).

The tick's retained receipt selects the version (`compute_commit_hash_for_receipt`):

| Receipt                                              | Receipt digest version | Retained record | Commit hash |
| ---------------------------------------------------- | ---------------------- | --------------- | ----------- |
| No attribution section (legacy)                      | 2                      | `EPRV0001`      | v2          |
| Attribution, retained before commit binding existed  | 3                      | `EPRV0002`      | v2          |
| Attribution, commit-bound (every new scheduler tick) | 4                      | `EPRV0003`      | v3          |
| No receipt (engine-direct commit, recorded event)    | —                      | —               | v2          |

The selection reads an explicit format marker, never the mere presence of an
attribution section, so records written before commit binding keep their v2
identities and replay unchanged. Replacing a commit-bound receipt's attribution
changes its digest and so its commit hash; stripping the binding selects v2,
which cannot reproduce the recorded v3 hash. Retained-commit validation and
provenance replay both recompute the selected version.

For commit-bound ticks, Decision 3 no longer applies to `decision_digest`: v3
commits to it. `plan_digest` and `rewrites_digest` remain diagnostics.

## Decision 4: Empty list digests are length-prefixed

For length-prefixed list digests, `EMPTY_LEN_DIGEST = blake3(0u64.to_le_bytes())`. It is not `blake3(b"")`.

## Decision 5: Readings stay outside the commit unless promoted

Observation artifacts, view frames, and session packets are readings over the carrier. They may have their own hashes, but they are not part of `commit_id` v2 or v3.
