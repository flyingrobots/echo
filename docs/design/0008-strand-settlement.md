<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0008 — Strand settlement and conflict artifacts

_Define Echo's first deterministic settlement runway for strands:
compare -> plan -> import -> conflict artifact._

Legend: KERNEL

Depends on:

- [0004 — Strand contract](./0004-strand-contract.md)
- [0007 — Braid geometry and neighborhood publication](./0007-braid-geometry-and-neighborhood-publication.md)
- [0006 — Echo Continuum alignment](./0006-echo-continuum-alignment.md)

## Why this cycle exists

After `0007`, Echo has an honest story for:

- speculative lanes
- fork-basis provenance
- braid geometry
- local plurality publication

What it still does not have is a lawful way to bring speculative lane history
back into canonical history.

That is the settlement problem.

Settlement is not:

- "merge two worldlines" as if they were symmetrical peers
- "copy the published truth frames back in"
- "last writer wins"
- "compose braid geometry until it becomes history"

Settlement is a separate history law:

- compare the strand's suffix against its fork-basis coordinate
- plan what may be imported under deterministic policy
- import accepted cause-level history
- record unresolved residue as first-class conflict artifacts

This is the missing leg between plural speculative work and real parity with
`git-warp`'s compare/transfer-plan workflow.

## Design decision

Echo should define v1 strand settlement as a **canonical-source settlement
runway**:

1. compare one strand's suffix against the worldline and coordinate recorded in
   `ForkBasisRef`
2. produce a deterministic settlement plan
3. append accepted imports to the canonical target worldline as `MergeImport`
   provenance entries
4. append unresolved residue as `ConflictArtifact` provenance entries

v1 does **not** generalize to arbitrary target lanes yet, and it does
not yet support settlement when the strand's source basis is itself a
speculative lane.

`0009` governs the runtime-control ontology and the generalized
`ForkBasisRef` naming. This packet narrows only the settlement runway.

That is a deliberate narrowing, not a theoretical claim. It gives Echo one
honest settlement path without pretending every cross-worldline import problem
is solved at once.

## Human users / jobs / hills

### Primary human users

- engine contributors implementing strand compare/import behavior
- debugger contributors who need conflict artifacts instead of silent drops
- advanced users exploring speculative lanes and then deciding what can come
  back into canonical history

### Human jobs

1. Compare a strand against its fork-basis coordinate and see the resulting delta.
2. Produce a deterministic import plan instead of ad hoc merge behavior.
3. See explicit conflict artifacts for what could not be imported.

### Human hill

A human can run settlement on a strand and get one stable answer:
what imports cleanly, what conflicts, and why.

## Agent users / jobs / hills

### Primary agent users

- agents implementing or auditing settlement/runtime publication
- agents building Continuum/Wesley proof slices over shared debugger nouns

### Agent jobs

1. Compute a settlement plan deterministically from runtime truth.
2. Distinguish accepted imports from recorded conflicts.
3. Map settlement outputs into reintegration detail and receipt shell without
   inventing new folklore.

### Agent hill

An agent can observe one strand, compute the same settlement runway every time,
and publish accepted imports versus conflict residue through explicit runtime
artifacts.

## Core principle

**Merge the causes, not the published truth.**

`TruthFrame` and session playback surfaces are replay artifacts. They are
authoritative for observation and delivery, but they are not merge inputs.

Settlement operates on recorded causal/runtime truth such as:

- `ProvenanceEntry`
- replay patches
- atom writes
- recorded channel outputs
- channel policies

It does not settle by replaying UI frames back into history.

## Scope of v1 settlement

### Allowed source

- one live strand
- `fork_basis_ref.source_lane_id` must name a canonical worldline in v1
- source history is the child worldline suffix strictly after
  `fork_basis_ref.fork_tick`

### Allowed target

- only the canonical worldline named by `fork_basis_ref.source_lane_id`
- only against the exact fork-basis coordinate recorded in `fork_basis_ref`

### Explicit exclusions

v1 does not define:

- settlement into arbitrary non-target worldlines
- settlement between two sibling strands
- settlement for strands whose `fork_basis_ref.source_lane_id` is
  itself speculative
- automatic conflict resolution
- support-pin history import
- synthetic merged worldlines

Support pins remain geometry only. They inform reads and neighborhood
publication, but they are not additional import sources in v1 settlement.

## Settlement objects

The exact Rust type names can still change, but the runtime publication ladder
should look like this.

### 1. `SettlementDelta`

The compare result for one strand relative to its fork-basis coordinate.

Minimum contents:

```text
SettlementDelta {
    strand_id:                StrandId,
    fork_basis_ref:           ForkBasisRef,
    source_child_worldline_id: WorldlineId,
    source_suffix_start_tick: WorldlineTick,
    source_suffix_end_tick:   WorldlineTick,
    source_entries:           Vec<ProvenanceRef>,
}
```

This is intentionally narrow. It identifies the source settlement window and
the authoritative recorded entries that the planner will inspect.

### 2. `SettlementPlan`

The deterministic result of evaluating the delta under channel and import law.

Minimum contents:

```text
SettlementPlan {
    strand_id:          StrandId,
    target_worldline:   WorldlineId,
    target_basis_ref:   ProvenanceRef,
    decisions:          Vec<SettlementDecision>,
}
```

Where each `SettlementDecision` is one of:

- `ImportCandidate`
- `ConflictArtifactDraft`

### 3. `ImportCandidate`

One accepted unit of source history eligible to become target provenance.

Minimum contents:

```text
ImportCandidate {
    source_ref:         ProvenanceRef,
    source_head_key:    Option<WriterHeadKey>,
    imported_op_id:     Hash,
}
```

The imported unit is source provenance, not playback shell.

### 4. `ConflictArtifactDraft`

One unresolved unit of settlement residue.

Minimum contents:

```text
ConflictArtifactDraft {
    artifact_id:        Hash,
    source_ref:         ProvenanceRef,
    channel_ids:        Vec<ChannelId>,
    reason:             ConflictReason,
}
```

`ConflictReason` should begin narrow and deterministic:

- `ChannelPolicyConflict`
- `UnsupportedImport`
- `BasisDivergence`
- `QuantumMismatch`

The exact reason set can grow later, but v1 must not collapse every failure
into one anonymous "could not merge" blob.

## Compare phase

The compare phase answers:

- what exact suffix exists on the strand after the fork point
- what the planner will evaluate

Compare walks the strand child worldline after `fork_basis_ref.fork_tick` and
collects authoritative `ProvenanceRef`s / entries in append order.

Compare does **not** decide eligibility. It only defines the candidate runway.

## Plan phase

The plan phase evaluates each source entry under deterministic import law.

### Planning inputs

- source `ProvenanceEntry`
- source replay patch / atom writes / outputs
- target fork-basis coordinate
- target worldline policy state
- channel policy for all affected channels

### Planning rules

1. **Quantum agreement is mandatory.**
   Cross-worldline settlement requires identical `tick_quantum`.
2. **Support pins are not import sources.**
   Only the strand's own post-fork suffix is planned.
3. **Channel policy gates import eligibility.**
4. **Unsettled residue becomes explicit conflict artifacts.**

### Channel policy law

Echo already has real channel policies:

- `StrictSingle`
- `Reduce(op)`
- `Log`

Settlement should treat them as import eligibility law:

- `StrictSingle`
    - if the source contribution disagrees with the target's admissible single
      value, plan a conflict artifact
- `Reduce(op)`
    - import is eligible only through the deterministic reducer
- `Log`
    - import is eligible only when the channel explicitly opts into historical
      multiplicity; otherwise the planner must still be allowed to surface a
      conflict instead of smearing events together

The planner's job is not to hide disagreement. It is to classify it
deterministically.

## Import phase

The import phase appends accepted imports to the canonical target worldline as
`ProvenanceEventKind::MergeImport` entries.

Those entries should:

- point back to the imported source coordinate
- preserve deterministic parentage on the target worldline
- carry imported patch/causal truth as replayable provenance
- remain distinguishable from ordinary local commits

v1 does not need a universal import algebra. It does need one honest import
recording path that can be replayed and inspected.

## Conflict artifact phase

The conflict phase appends unresolved residue as
`ProvenanceEventKind::ConflictArtifact` entries on the target worldline.

The provenance event kind is already present in repo truth. This cycle gives it
real semantics:

- it is not a warning log
- it is not shell-only metadata
- it is first-class recorded history saying "this source residue existed and
  could not be deterministically settled under current law"

The artifact payload may still be represented through auxiliary shell data or a
future richer record, but the provenance entry itself must exist as kernel
truth.

## Provenance event semantics

This cycle sharpens three existing placeholders in
`ProvenanceEventKind`:

1. `CrossWorldlineMessage`
    - remains pre-settlement coordination / future runway
    - not required for v1 settlement execution
2. `MergeImport`
    - becomes the authoritative target-worldline record for accepted imports
3. `ConflictArtifact`
    - becomes the authoritative target-worldline record for unresolved
      settlement residue

That is enough to move these event kinds out of placeholder limbo without
forcing all future cross-worldline behavior into this one cycle.

## Relationship to braid geometry

`0007` defined support pins and local plural site publication.

This cycle depends on that geometry, but does not redefine it.

The law is:

- braid geometry answers "which lanes define the local site?"
- settlement answers "which part of one strand's history can lawfully become
  target history?"

If those two concerns are collapsed, Echo will either overfit settlement into
the observation model or quietly treat support overlays as if they were imports.

## Relationship to Continuum and `warp-ttd`

Settlement must eventually feed:

- reintegration detail
- receipt shell
- conflict inspection in `warp-ttd`

But `warp-ttd` should consume the published settlement nouns, not invent them.

The important top-level debugger truth is:

- accepted import
- explicit conflict artifact
- stable source/target coordinate

That gives Continuum tools one honest cross-host story instead of "Echo
settlement looks nothing like `git-warp` transfer planning."

## What this cycle does not do

- implement automatic conflict resolution
- define arbitrary target selection beyond the canonical target worldline
- define a full conflict artifact schema family
- settle support-pin participants as if they were source history
- replace reintegration detail with settlement shell

## Immediate implementation consequences

1. Echo needs a native settlement compare surface over strand suffixes.
2. Echo needs a deterministic planner that reads channel policies as import law.
3. Echo must give `MergeImport` and `ConflictArtifact` real runtime semantics.
4. Echo should publish settlement outputs so adapters can surface them without
   re-deriving them from raw provenance.
5. Later Wesley/Continuum proof slices should target these settlement outputs
   as shared observer/debugger nouns, with Echo-local shell allowed around
   them.

## Open questions

1. Should `ImportCandidate` operate at provenance-entry granularity only, or
   should v1 plan at finer op/channel granularity inside an entry?
2. Does `ConflictArtifactDraft` need a first-class payload type in kernel truth
   immediately, or can v1 begin with provenance entry plus shell data?
3. When Echo later permits durable strands, does settlement remain
   canonical-source-only, or become target-parameterized?

## Decision

Echo should add one honest, deterministic settlement runway now:

- compare one strand's suffix to its fork-basis coordinate
- plan imports under channel policy
- record accepted imports as `MergeImport`
- record unresolved residue as `ConflictArtifact`

That is enough to move Echo from speculative-lane experimentation toward real
conceptual parity with `git-warp`, without pretending it already owns a
universal merge oracle.
