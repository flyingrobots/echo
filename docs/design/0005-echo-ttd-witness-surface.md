<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0005 — Echo TTD witness surface

_Define how Echo's current runtime objects map to `warp-ttd` neighborhood core, reintegration detail, and receipt shell._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [0004 — Strand contract](./0004-strand-contract.md)
- external `warp-ttd` design packets:
    - `0016-local-neighborhood-browser`
    - `0017-neighborhood-protocol-shapes`

## Why this cycle exists

Echo already has most of the raw runtime truth a `warp-ttd` host adapter
needs:

- stable identity and routing via `WorldlineId`, `WriterHeadKey`,
  ingress targets, playback mode, and scheduler status
- explicit read contracts via `ObservationRequest` and
  `ObservationArtifact`
- committed history via `ProvenanceEntry`
- per-candidate tick outcomes via `TickReceipt`
- recorded truth outputs via `FinalizedChannel`

What Echo does **not** yet have is one honest object for the local site a
debugger wants to inspect. The older backlog note
`PLATFORM_echo-ttd-host-adapter.md` assumed Echo could map directly into
`PlaybackFrame`, `ReceiptSummary`, and `EffectEmissionSummary`. That is too
flat now.

`warp-ttd` has moved to a cleaner model:

- neighborhood core = law-bearing site truth
- reintegration detail = seam anchors, obligations, and evidence
- receipt shell = runtime/provenance explanation around the site

This cycle defines how Echo maps onto that ladder and names the missing
pieces. Without this, the eventual host adapter will either lie by flattening
everything into receipt-shaped sludge or block forever waiting for perfect
runtime objects.

## Human users / jobs / hills

### Primary human users

- runtime contributors implementing the Echo host adapter
- debugger contributors making `warp-ttd` speak honestly across hosts
- future app authors depending on replay-safe debugger semantics

### Human jobs

1. Inspect one Echo coordinate and know what counts as core site truth versus
   runtime explanation.
2. Build a host adapter without guessing which Echo object should populate
   which `warp-ttd` layer.

### Human hill

A human can point at an Echo runtime object and say whether it belongs in
neighborhood core, reintegration detail, or receipt shell without inventing
new folklore.

## Agent users / jobs / hills

### Primary agent users

- agents generating or auditing the Echo host adapter
- agents comparing Echo and `git-warp` debugger surfaces

### Agent jobs

1. Programmatically classify Echo runtime objects into core, reintegration, or
   shell layers.
2. Detect where the adapter must synthesize shape versus where Echo must grow a
   first-class runtime object.

### Agent hill

An agent can inspect Echo's runtime types and determine which `warp-ttd`
neighborhood fields are already grounded in repo truth and which are still
missing.

## Human playback

1. The human reads this cycle while looking at Echo runtime code.
2. The document names concrete source objects for each debugger layer.
3. The human can decide what to synthesize now and what must be promoted into
   runtime truth before the adapter ships.

## Agent playback

1. The agent reads `ObservationArtifact`, `ProvenanceEntry`, `TickReceipt`,
   `FinalizedChannel`, `CursorReceipt`, and `SchedulerStatus`.
2. The document classifies them against the neighborhood ladder.
3. The agent can implement the adapter without flattening core truth into
   receipt shell.

## Design decision

Echo should map its existing runtime surfaces onto the `warp-ttd` neighborhood
ladder as follows:

- `NeighborhoodCoreSummary` is derived primarily from Echo's native
  `NeighborhoodCore` publication, with `ObservationArtifact` retained as
  revelation / shell context rather than the source of local-site truth.
- `ReintegrationDetailSummary` is derived primarily from `TickReceipt`,
  provenance parentage, and materialization conflict structure.
- `ReceiptShellSummary` is derived from `ProvenanceEntry`, finalized channel
  outputs, cursor/session receipts, scheduler metadata, and other explanatory
  runtime context.

Echo now does expose one first-class local-site object:

- `NeighborhoodSite` as Echo-native publication truth
- `NeighborhoodCore` as the shared neighborhood-core family projection

What Echo still does **not** yet expose is a native reintegration-core object
or a full nearby-alternative neighborhood browser. Those remain explicit host
or follow-on runtime gaps and should not be guessed silently.

## TTD observation versus counterfactual creation

This packet needs one explicit separation because `warp-ttd` will
eventually expose both read-side inspection and write-side speculative
forks.

Observation alone does not create new Echo history. A debugger session,
cursor move, or read against `ObservationArtifact` remains revelation
over existing runtime truth.

If a user explicitly asks to continue from an earlier coordinate or
explore a counterfactual, that is a separate act: create a strand with
an exact `fork_basis_ref` and route speculative intents through the
ordinary scheduler.

For Echo v1, the honest posture is:

- debugger-created strands are session-scoped scratch or minimally
  retained speculative lanes
- they are not silently promoted into shared admitted history
- any future durable author-only retention must record creator, tool or
  session origin, exact fork basis, and retention or revelation posture

This keeps the adapter honest about the difference between a reading
surface and a fork surface while still matching the three-tier thinking
room doctrine from Paper VII.

## Runtime mapping

### 1. Neighborhood core

Neighborhood core is the minimum law-bearing site object. In Echo today, the
best grounding is:

- `ObservationArtifact.resolved`
    - stable observed coordinate
    - worldline identity
    - resolved tick
    - commit/global tick stamps
    - commit/state root commitments
- `HeadObservation` / `SnapshotObservation`
    - head and snapshot truth at the chosen coordinate
- `WriterHeadKey`
    - exact head identity when the observed site is head-relative
- strand metadata from `0004`
    - when the observed worldline is a strand, this gives parentage and lane kind

This is enough to ground:

- `siteId` as an adapter-defined stable identifier over observed coordinate plus
  host/lane scope
- `coordinate`
- `primaryWorldlineId`
- `primaryLaneId`
- `headId` when applicable
- `frameIndex` / debugger-local coordinate mapping

This is **not** enough to ground the full neighborhood core shape by itself.

Missing today:

- explicit alternative set near the current site
- richer nearby-neighborhood enumeration beyond the current local site

Echo now closes the first neighborhood-core gap natively through
`NeighborhoodSite` and its shared `NeighborhoodCore` projection:

- explicit participating lane set
- explicit singleton-vs-plural local outcome at one observed site

So the initial Echo adapter no longer needs to invent neighborhood core from
raw `ObservationArtifact` alone. It should consume the native
`NeighborhoodCore` projection first, then add reintegration and shell detail
without redefining the core.

### 2. Reintegration detail

Reintegration detail is the first protocol cash-out of the seam-bearing core.
Echo does not have `R_core` as one object, but several current objects already
carry parts of it.

Best current grounding:

- `TickReceipt.entries`
    - per-candidate applied/rejected outcomes
- `TickReceipt.blocked_by(idx)`
    - explicit blocking causality for rejected candidates
- `TickReceiptRejection`
    - current seam-law failure reason (`FootprintConflict`)
- `ProvenanceEntry.parents`
    - explicit parent lineage at the committed coordinate
- `ProvenanceEntry.head_key`
    - producing writer head for local commits
- `FinalizeReport.errors`
    - deterministic materialization conflict structure when channel finalization
      fails

This means Echo already has partial grounding for:

- seam anchors (`J`) via parent refs, producing head, and observed coordinate
- compatibility obligations (`K`) via receipt dispositions and finalize law
- compatibility evidence (`V`) via blocker indices and deterministic lineage

What is still missing is a first-class **local reintegration object**:

- no single seam-anchor record
- no normalized obligation/evidence surface
- no explicit distinction between reintegration core and explanatory shell

So the adapter should publish reintegration detail as a synthesized summary
layer over receipts/provenance, not pretend Echo already has a native
`R_core` DTO.

### 3. Receipt shell

Receipt shell is where Echo is strongest today.

Best current grounding:

- `ProvenanceEntry`
    - `commit_global_tick`
    - `head_key`
    - `parents`
    - `event_kind`
    - `expected`
    - `patch`
    - `outputs`
    - `atom_writes`
- `FinalizedChannel`
    - recorded truth payloads that survived deterministic materialization
- `CursorReceipt`
    - session/cursor/worldline/tick/commit context
- `TruthFrame`
    - cursor receipt plus channel payload and hash
- `SchedulerStatus`
    - current runtime work/scheduler state

This is explanatory/runtime shell. It is critical for inspection, but it
should not be allowed to redefine neighborhood core or seam detail.

## Adapter doctrine

The Echo host adapter should follow these rules:

1. Do not infer local neighborhood plurality from shell objects.
2. Do not treat `ProvenanceEntry` as if it were already a neighborhood core.
3. Do not treat `TickReceipt` as if it were already a full reintegration
   object.
4. Publish narrow, truthful neighborhood core first.
5. Publish synthesized reintegration detail second.
6. Attach provenance/materialization/runtime context as receipt shell only.

## Immediate implications

### What Echo can support now

- coordinate-grounded site inspection
- head/worldline identity
- replay-safe commit/snapshot observation
- per-tick candidate outcome inspection
- deterministic blocking-causality inspection
- finalized recorded-truth payload inspection

### What Echo cannot support honestly yet

- full participating-lane neighborhood enumeration
- explicit local alternative set
- native reintegration-core DTO

## Implementation outline

1. Reconcile Echo's TTD protocol consumption with canonical `warp-ttd`
   protocol ownership.
2. In the eventual Echo host adapter, consume Echo's native
   `NeighborhoodCore` publication first, with `ObservationArtifact` retained as
   shell/revelation context rather than as the source of core site truth.
3. Synthesize `ReintegrationDetailSummary` from `TickReceipt`,
   `ProvenanceEntry.parents`, and materialization conflict data.
4. Keep `ProvenanceEntry`, `FinalizedChannel`, `CursorReceipt`, `TruthFrame`,
   and `SchedulerStatus` in receipt shell lanes.
5. File or implement follow-on runtime work when the adapter needs explicit
   nearby alternatives or richer reintegration structure.

## Tests to write first

- adapter test proving Echo's native `NeighborhoodCore` publication reaches the
  host boundary without fake alternatives
- adapter test proving `TickReceipt.blocked_by()` becomes reintegration detail,
  not receipt shell-only text
- adapter test proving finalized channels and scheduler status stay in shell and
  do not mutate core outcome

## Risks / unknowns

- Echo may need a first-class local-site runtime object sooner than desired if
  synthesized reintegration detail becomes too lossy.
- Materialization conflict data may belong partly in reintegration detail and
  partly in shell depending on channel law class.
- Strand support may be required before Echo can publish honest participating
  lane sets.

## Postures

- **Accessibility:** not applicable; this cycle defines data boundaries, not a
  rendered interface.
- **Localization:** not applicable; protocol/runtime naming only.
- **Agent inspectability:** explicit goal; this cycle exists to separate
  law-bearing core from explanatory shell for machine consumers.

## Non-goals

- Implement the Echo `warp-ttd` host adapter in this cycle.
- Unify every Echo runtime DTO with `warp-ttd` in one pass.
- Introduce a new local-site runtime object in Echo before the adapter proves
  one is necessary.
