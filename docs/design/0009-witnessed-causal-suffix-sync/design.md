<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0009 — Witnessed causal suffix export and import

_Define Echo's runtime-side handoff law for simultaneous hot/cold operation
with `git-warp`: export and import witnessed suffix bundles, not state._

Legend: KERNEL

Depends on:

- [0006 — Echo Continuum alignment](../0006-echo-continuum-alignment/design.md)
- [0007 — Braid geometry and neighborhood publication](../0007-braid-geometry-and-neighborhood-publication/design.md)
- [0008 — Strand settlement and conflict artifacts](../0008-strand-settlement/design.md)

## Why this cycle exists

Echo now has honest runtime publication for:

- neighborhood sites
- strand settlement
- conflict artifacts

That is enough to stop talking vaguely about "sync" and define the next actual
runtime boundary.

If Echo and `git-warp` are simultaneous WARP replicas at different runtime
temperatures, Echo must not export:

- latest state snapshot
- latest materialized view
- observer-rendered truth frame

It must export witnessed causal history rooted at a known frontier.

Likewise, importing from `git-warp` must not be an ad hoc side channel. It
must go through the same admission law Echo already claims to own.

## Design decision

Echo should define v1 hot/cold runtime handoff as:

1. export one witnessed causal suffix bundle from a known base frontier
2. import one peer suffix bundle through normal witnessed admission
3. return one explicit import outcome and receipt

Echo must not synchronize state blobs or silently mutate a peer's canonical
branch.

## Core principle

**Synchronize witnessed transitions, not materialized states.**

Echo is a high-performance causal writer and observer host.
From the peer runtime's perspective, it is still just a writer that emits:

- writer identity
- lane identity
- base frontier
- witnessed transition records
- payload references
- receipts and witness material

That is the right level of honesty.

## Scope of v1 suffix sync

### Allowed source

- one lane/worldline suffix rooted at a known frontier
- optionally a strand-derived suffix once the lane target is explicit

### Allowed target

- one receiving runtime that understands the same graph identity and suffix
  family

### Explicit exclusions

v1 does not define:

- whole-graph state mirroring
- last-write-wins policy
- silent direct mutation of a peer branch
- automatic full-duplex sync daemon behavior
- hidden branch collapse under host-time arrival order

## Runtime objects

### 1. `ExportSuffixRequest`

Minimum meaning:

```text
ExportSuffixRequest {
    graph_id:      GraphId,
    lane_id:       LaneId,
    base_frontier: Frontier,
    target_frontier: Option<Frontier>,
}
```

This identifies the suffix Echo is being asked to export.

### 2. `CausalSuffixBundle`

Minimum meaning:

```text
CausalSuffixBundle {
    graph_id:          GraphId,
    source_runtime_id: RuntimeId,
    source_writer_id:  WriterId,
    lane_id:           LaneId,
    base_frontier:     Frontier,
    target_frontier:   Frontier,
    transitions:       Vec<BoundaryTransitionRecord>,
    payload_refs:      Vec<PayloadRef>,
    checkpoints:       Option<Vec<CheckpointRef>>,
    wormholes:         Option<Vec<WormholeRecord>>,
    signatures:        Option<Vec<SignatureEnvelope>>,
    export_witness:    ExportWitness,
}
```

Echo may carry hot-runtime aids such as checkpoints, but those are supplemental
to the transitions themselves.

### 3. `ImportSuffixResult`

Echo should surface the same honest outcome categories it already uses for
admission and settlement-style plurality:

```text
ImportSuffixResult =
    Admitted { frontier, receipt }
  | Staged { lane_id, reason, receipt }
  | Braided { braid_id, cells, receipt }
  | Conflict { artifact, receipt }
  | Obstructed { witness, receipt }
```

## Export law

Export must:

1. verify the requested graph and lane identity
2. normalize the requested base frontier against local runtime truth
3. gather the ordered transition suffix
4. gather referenced payload material
5. include any optional checkpoint or wormhole aids without redefining graph
   truth
6. emit a witness that says what Echo believes it is exporting

Export must not:

- invent transitions that are not already witnessed runtime truth
- emit observer projections as if they were causal transitions
- claim state equivalence instead of causal suffix identity

## Import law

Import must:

1. verify graph identity, lane identity, and bundle integrity
2. normalize the bundle's base frontier against local truth
3. detect already-known transitions and prior imports
4. compute overlap and dependency geometry
5. pass the claim through normal admission policy
6. emit one explicit local receipt for the outcome

Import must not:

- silently overwrite local canonical truth
- mutate a Git branch or peer-owned branch by folklore
- collapse plurality under wall-clock recency

## Loop prevention

Imported history must retain durable provenance such as:

- source runtime identity
- source writer identity
- original transition identity
- import receipt lineage

Otherwise Echo cannot distinguish:

- genuinely new peer history
- its own already-imported suffix arriving again through a colder runtime

That is the minimal anti-loop discipline.

## Runtime API shape

The first honest runtime surface should be narrow:

```text
export_suffix(request) -> CausalSuffixBundle
import_suffix(bundle) -> ImportSuffixResult
```

One supporting read surface is also useful:

```text
frontier_summary(graph_id, lane_id) -> FrontierSummary
```

That gives peers enough information to detect missing suffixes without
pretending to exchange state snapshots.

## First implementation slice

The first proving cut should be one-way:

1. Echo exports one suffix bundle from one known lane frontier.
2. `git-warp` imports it according to its own normal admission law.
3. Re-import proves idempotence instead of novelty.

Only after that should Echo implement reverse import from `git-warp`.

The first cut should prove:

- no state sync folklore
- no direct branch mutation folklore
- no looped re-import
- one inspectable import receipt path

## Done looks like

- Echo can export one lawful causal suffix bundle from hot runtime truth
- Echo can import one lawful peer bundle through its normal admission algebra
- handoff to `git-warp` is honest and inspectable
- runtime temperature remains an execution distinction, not a graph ownership
  distinction
