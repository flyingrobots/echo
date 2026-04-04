<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Strand settlement and conflict artifacts

Depends on: strand contract (KERNEL_strand-contract)

Replaces: KERNEL_stream-merge-semantics (Ref: #245)

Define deterministic settlement/import semantics for strands. This is
not "merge two worldlines" — it is compare → plan → import → conflict
artifact, modeled after git-warp's compare/transfer-plan split.

## Core principle

Merge the causes, not the published truth.

TruthFrames are authoritative outputs for a specific cursor/tick/channel.
Playback must reproduce them byte-for-byte. They are replay artifacts,
not merge inputs. Settlement operates on the underlying operations and
channel emissions, governed by channel policy.

## Channel policy as eligibility gate

- **StrictSingle**: Disagreement between strands becomes a
  `ConflictArtifact`. There is no automatic resolution.
- **Reduce(op)**: Eligible for settlement only through an explicit
  reducer function. The reducer must be deterministic.
- **Log**: Union of emissions only when the channel explicitly opts
  into multi-event history. Default is conflict.

## Provenance event kind mapping

The three placeholder variants in `ProvenanceEventKind` map to the
settlement protocol:

1. **`CrossWorldlineMessage`** — pre-settlement coordination between
   strands (before import begins).
2. **`MergeImport`** — accepted imports from one strand into another
   (the settlement itself).
3. **`ConflictArtifact`** — unresolved residue that could not be
   deterministically settled.

## Settlement protocol (v1)

1. **Compare**: Diff the strand's suffix against the base worldline's
   state at the strand's `base_ref` tick. Produce a structured delta.
2. **Plan**: For each channel emission in the delta, consult the
   channel's policy to determine eligibility. Produce a settlement
   plan listing accepted imports and conflict artifacts.
3. **Import**: Apply accepted imports as `MergeImport` provenance
   entries on the target worldline. Each import carries the source
   strand coordinate.
4. **Record conflicts**: Emit `ConflictArtifact` entries for
   unresolvable disagreements. These are first-class provenance, not
   silent drops.

## What this does NOT do

- Automatic conflict resolution (v1 surfaces conflicts, does not
  resolve them).
- Blind patch interleave or last-writer-wins.
- Settlement of non-ephemeral strands (v1 strands are session-scoped).
- Universal merge oracle.

## Relationship to git-warp

git-warp separates `braidStrand()` (geometry: pinning support overlays)
from `planStrandTransfer()` (history: deterministic settlement runway).
Echo follows the same separation. Braid is defined in the strand
contract. Settlement is defined here.

## Relationship to warp-ttd

warp-ttd Cycle D wants create/write/tick/compare/drop and explicitly
excludes automatic conflict resolution. Echo should hit that bar first.
Settlement surfaces through the adapter as structured conflict reports,
not silent merges.
