<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract-Hosted File History Substrate Request

This file archives the source prompt for the contract-hosted file history
substrate work. The design doc and backlog tasks for this body of work should
use this request as the controlling reference.

````text
OK make a branch for this new body of work. Please also save off this entire prompt to docs/ somewhere, for future reference. The first task in this sequence will be to write a design doc for this request, then create a sequence of backlog tasks to execute it.

Context and doctrine
  ====================

  Echo must remain a generic deterministic witnessed causal substrate. Do not add privileged text-editing, jedit, Graft, editor, or rope APIs to Echo core.

  However, jedit is the first serious consumer and should be used as the concrete example contract family. The point is to make Echo capable of hosting a Wesley-compiled application contract
  that models a file as Echo history.

  The model we need to support is:

  - The Echo graph represents the data model.
  - Echo ticks represent operations applied to that data model over time.
  - The file is not “base text + editor-local patches” as canonical truth.
  - The file is the materialized reading at an Echo coordinate: worldline, strand, or braid projection.
  - All mutations to the file must happen by submitting Intents through Echo.
  - Reads happen through observation/readings, not by mutating or secretly materializing state.
  - Undo is either:
    - preview/seek to an earlier coordinate, read-only, or
    - append a new inverse Intent at the current frontier.
  - “Unapply tick” must never delete or rewrite old history. It appends a compensating/inverse tick whose admission is witnessed.

  Do not let application code bypass this with direct service mutation calls. Existing internal services may remain implementation details, but external mutation surfaces for contract
  families, strands, braids, settlement, and inverse operations must go through Intents.

  Current Echo facts to preserve
  ==============================

  Echo already has:
  - EINT v1: "EINT" || op_id:u32le || vars_len:u32le || vars.
  - KernelPort::dispatch_intent(bytes).
  - warp-wasm dispatch_intent(bytes) and observe(bytes).
  - IngressEnvelope::local_intent and deterministic worldline inbox admission.
  - SchedulerCoordinator::super_tick.
  - Engine materialization of runtime ingress events and cmd/* rule dispatch.
  - echo-wesley-gen generating operation constants, vars structs, EINT mutation helpers, query ObservationRequest helpers, and RegistryProvider.
  - ObservationRequest with QueryView / Query shape, but current ObservationService rejects QueryView as unsupported.
  - Provenance, replay patches, playback seek/checkpoints.
  - Session-scoped strands, support pins, NeighborhoodSite, and SettlementService.
  - echo-cas as content-addressed blob storage.

  Key missing pieces
  ==================

  Implement or design-to-RED-test the generic technology needed for a jedit-like contract:

  1. Installed Wesley contract hosting
  -----------------------------------

  Echo must be able to host a Wesley-generated contract family without adding application nouns to Echo core.

  Needed shape:

  Application GraphQL
    -> Wesley IR
    -> generated Rust DTOs/codecs/op ids/registry/handlers
    -> installed Echo contract host
    -> dispatch_intent(EINT bytes)
    -> Echo ingress/scheduling/provenance
    -> generated mutation handler applies transition law
    -> generated query handler applies read/emission law
    -> ReadingEnvelope + payload bytes

  Requirements:

  - Reuse EINT v1 unless a RED proves it cannot work.
  - Reuse echo-registry-api::RegistryProvider unless a RED proves it cannot work.
  - Echo core must not import jedit domain Rust types.
  - The installed contract host may store trait objects / generated adapters, but the core boundary must remain generic.
  - Host must reject unsupported op ids for an installed contract when contract-hosting validation is enabled.
  - Host must not trust caller-supplied footprints. Footprint authority comes from the verified generated artifact or explicitly trusted authority.
  - Mutation handlers must run inside Echo admission/witness/provenance, not in an unrecorded global side channel.

  2. Contract QueryView observers
  -------------------------------

  echo-wesley-gen can currently build ObservationRequest { frame: QueryView, projection: Query { query_id, vars_bytes } }, but ObservationService rejects QueryView. Implement a generic
  contract observer bridge.

  Requirements:

  - ObservationService::observe must dispatch QueryView/Query to an installed contract observer when available.
  - Query results return ObservationPayload::QueryBytes.
  - ReadingEnvelope must name enough evidence to be honest:
    - observed coordinate,
    - contract family/artifact/schema identity,
    - query op id,
    - vars hash,
    - observer/read law version if available,
    - witness refs,
    - budget posture,
    - rights posture,
    - residual/obstruction posture.
  - Unsupported query op returns typed obstruction/error, not a fake empty reading.
  - Query observers must support bounded readings. A text window read must not require materializing the full file.

  3. Intent-only mutation surfaces for runtime operations
  ------------------------------------------------------

  Some Echo APIs currently mutate directly, such as settle_strand, register_strand, pin_support, unpin_support, and provenance fork. Keep internal services as implementation details, but add
  Intent-level external operation paths.

  Requirements:

  - External mutation of a contract worldline/strand/braid must be EINT -> IngressEnvelope -> scheduler -> handler.
  - Direct settlement mutation must have an Intent equivalent, e.g. settleStrand / settleBraid / admitBraidProjection.
  - Direct strand creation must have an Intent equivalent, e.g. createContractStrand.
  - pin_support/unpin_support must have Intent equivalents if exposed to application flows.
  - Existing direct ABI calls may remain for compatibility/debug temporarily, but tests for the jedit-style path must prove no direct mutation API is required.

  4. First-class generic braids
  -----------------------------

  jedit wants a file to be modeled as a worldline plus an ordered braid of strands over that worldline.

  The desired simple braid law is sequential:

  baseline = canonical file worldline at coordinate B0
  first edit creates S0 forked from baseline
  braid projection = baseline + S0

  next edit creates S1 forked from current braid projection
  braid projection = (baseline + S0) + S1

  next edit creates S2 forked from current braid projection
  braid projection = ((baseline + S0) + S1) + S2

  The braid is always the ordered projection of its members. Each new member forks from the current projection frontier, not from stale baseline, unless explicitly requested.

  Requirements:

  - Add a generic braid substrate, not a jedit type.
  - A Braid must have:
    - braid id,
    - baseline worldline/ref,
    - ordered member refs,
    - current projection ref/digest,
    - contract family/schema identity when contract-backed,
    - basis/revalidation posture.
  - Braid projection must be observable.
  - Braid projection must be able to return plural/obstructed/conflict posture.
  - Braid member append must be an Intent.
  - Braid settlement/collapse/admission must be an Intent.
  - Do not flatten support pins into imports. Keep geometry separate from settlement.

  5. Inverse / unapply tick semantics
  -----------------------------------

  Implement generic substrate support and a jedit example contract for unapplying ticks.

  Important law:

  - “Unapply tick” appends a new inverse operation.
  - It does not delete the old tick.
  - It does not mutate old provenance.
  - It must be admitted through Echo as an Intent.
  - It must be contract-defined because only the contract knows how to invert domain semantics.

  Example:

  C = [add "h", add "e", add "l", add "l", add "o"]
  materialized text = "hello"

  Unapply C2:
  C' = [add "h", add "e", add "l", add "l", add "o", inverse(C2)]
  materialized text = "helo"

  Requirements:

  - Add a generic “contract inverse admission” hook:
    - Given a target tick/receipt or tick range,
    - current target coordinate,
    - contract family,
    - inverse policy,
    - ask the installed contract to produce one or more inverse Intents or an obstruction.
  - The inverse is admitted as normal causal history.
  - The original target tick remains in provenance.
  - The result must retain a receipt linking inverse tick(s) to the original tick/receipt(s).
  - If the original causal span no longer maps cleanly to current frontier, return a typed obstruction or conflict.
  - Do not implement a generic blind inverse of WarpOp as the user-facing undo model. WarpOp patches are replay artifacts; domain inverse law belongs to the contract.

  6. Retention / CAS / streaming
  ------------------------------

  Large files must not require full text materialization.

  Requirements:

  - Contract payloads must be able to refer to retained text/blob fragments by CAS ref.
  - Query observers must support aperture/budgeted reading, e.g. visible lines or byte range.
  - echo-cas may remain content-only, but semantic refs above CAS must include contract/schema/type/layout information.
  - Add or design a streaming/blob reader seam if current BlobStore::get Arc<[u8]> is insufficient.
  - Cached full text readings are allowed as cache only, never canonical truth.
  - Wormholes are future history/provenance compression artifacts. Do not misuse “wormhole” for rope chunks or portals.
  - If implementing wormhole/checkpoint retention, preserve inverse semantics:
    - either keep raw receipts,
    - or keep a rehydratable cold archive,
    - or explicitly obstruct fine-grained unapply inside compressed ranges.

  jedit example contract
  ======================

  Use this as the ideal GraphQL shape for tests/docs/examples. It is fine if exact directive names change. Keep it as an application contract fixture, not Echo core ontology.

  ```graphql
  """
  jedit hot text contract example for Echo/Wesley contract hosting.

  This schema is intentionally application-owned. Echo core must not gain these
  types as privileged substrate nouns. Echo hosts the generated artifact.
  """
  scalar Hash
  scalar Bytes
  scalar UInt64

  enum TextEncoding {
    UTF8
  }

  enum CheckpointKind {
    INITIAL
    MANUAL_SAVE
    AUTO_SAVE
  }

  enum TickKind {
    BUFFER_CREATE
    TEXT_REWRITE
    CHECKPOINT_CREATE
    INVERSE_REWRITE
    BRAID_CREATE
    BRAID_MEMBER_APPEND
    BRAID_SETTLE
  }

  enum AdmissionOutcomeKind {
    ADMITTED
    DUPLICATE
    CONFLICT
    OBSTRUCTED
  }

  enum ReadingResidualPosture {
    COMPLETE
    RESIDUAL
    PLURALITY_PRESERVED
    OBSTRUCTED
    BUDGET_LIMITED
    RIGHTS_LIMITED
  }

  enum BraidMemberRole {
    BASELINE
    EDIT_STRAND
    SUPPORT
    SETTLED_IMPORT
  }

  enum InversePolicy {
    EXACT_SPAN_OR_OBSTRUCT
    MAP_THROUGH_ANCHORS
    ALLOW_CONFLICT_ARTIFACT
  }

  enum UnapplyObstructionReason {
    TARGET_TICK_NOT_FOUND
    RECEIPT_NOT_FOUND
    INVERSE_FRAGMENT_UNAVAILABLE
    CAUSAL_SPAN_UNMAPPABLE
    CURRENT_BASIS_MISMATCH
    CONTRACT_VERSION_MISMATCH
    COMPRESSED_HISTORY_REQUIRES_REHYDRATION
  }

  type BufferWorldline {
    worldlineId: ID!
    bufferKey: String!
    canonicalHeadId: ID!
    projectionPath: String
    createdAtTickId: ID
  }

  type RopeHead {
    headId: ID!
    worldlineId: ID!
    rootNodeId: ID!
    byteLength: UInt64!
    lineCount: UInt64!
    utf16Length: UInt64!
    equivalenceDigest: Hash!
  }

  type TextBlobRef {
    blobId: ID!
    contentHash: Hash!
    byteLength: UInt64!
    encoding: TextEncoding!
  }

  type RopeLeaf {
    leafId: ID!
    blob: TextBlobRef!
    startByte: UInt64!
    endByte: UInt64!
    byteLength: UInt64!
    lineCount: UInt64!
    utf16Length: UInt64!
  }

  type Tick {
    tickId: ID!
    worldlineId: ID!
    kind: TickKind!
    sequenceNumber: UInt64!
    commitRef: String
    author: String
  }

  type TextSpanRef {
    basisHeadId: ID!
    startByte: UInt64!
    endByte: UInt64!
    spanDigest: Hash!
  }

  type TickReceipt {
    receiptId: ID!
    tickId: ID!
    baseHeadId: ID!
    nextHeadId: ID!
    kind: TickKind!
    inputSpan: TextSpanRef
    insertedFragmentDigest: Hash
    deletedFragmentDigest: Hash
    inverseFragmentDigest: Hash
    inverseBlob: TextBlobRef
    footprintDigest: Hash!
    basisProvenanceRef: String!
    summary: String
  }

  type Checkpoint {
    checkpointId: ID!
    worldlineId: ID!
    headId: ID!
    kind: CheckpointKind!
    label: String
    createdByTickId: ID
  }

  type Braid {
    braidId: ID!
    baselineWorldlineId: ID!
    baselineRef: String!
    projectionHeadId: ID!
    projectionDigest: Hash!
    memberCount: UInt64!
  }

  type BraidMember {
    memberId: ID!
    braidId: ID!
    orderIndex: UInt64!
    role: BraidMemberRole!
    strandId: ID
    sourceWorldlineId: ID!
    sourceBaseRef: String!
    sourceTipRef: String!
    projectionAfterDigest: Hash!
  }

  type TextLine {
    lineNumber: UInt64!
    startByte: UInt64!
    endByte: UInt64!
    text: String!
  }

  type TextWindowReading {
    sourceRef: String!
    head: RopeHead!
    startLine: UInt64!
    requestedLineCount: UInt64!
    returnedLineCount: UInt64!
    lines: [TextLine!]!
    residualPosture: ReadingResidualPosture!
    payloadDigest: Hash!
    nextWindowCursor: String
  }

  type CreateBufferWorldlineResult {
    worldline: BufferWorldline!
    head: RopeHead!
    checkpoint: Checkpoint
    receipt: TickReceipt!
  }

  type ReplaceRangeAsTickResult {
    worldline: BufferWorldline!
    nextHead: RopeHead!
    tick: Tick!
    receipt: TickReceipt!
    outcome: AdmissionOutcomeKind!
  }

  type CreateBraidResult {
    braid: Braid!
    baselineMember: BraidMember!
    tick: Tick!
    receipt: TickReceipt!
  }

  type AppendBraidEditResult {
    braid: Braid!
    member: BraidMember!
    nextProjectionHead: RopeHead!
    tick: Tick!
    receipt: TickReceipt!
    outcome: AdmissionOutcomeKind!
  }

  type BraidProjectionReading {
    braid: Braid!
    head: RopeHead!
    residualPosture: ReadingResidualPosture!
    obstruction: UnapplyObstructionReason
  }

  type UnapplyObstruction {
    reason: UnapplyObstructionReason!
    targetTickId: ID
    targetReceiptId: ID
    message: String!
  }

  type UnapplyTickResult {
    worldline: BufferWorldline!
    nextHead: RopeHead
    inverseTick: Tick
    inverseReceipt: TickReceipt
    targetReceiptId: ID!
    obstruction: UnapplyObstruction
    outcome: AdmissionOutcomeKind!
  }

  type UnapplyTickSequenceResult {
    worldline: BufferWorldline!
    nextHead: RopeHead
    inverseTicks: [Tick!]!
    inverseReceipts: [TickReceipt!]!
    obstructions: [UnapplyObstruction!]!
    outcome: AdmissionOutcomeKind!
  }

  input TextBlobInput {
    blobId: ID!
    contentHash: Hash!
    byteLength: UInt64!
    encoding: TextEncoding! = UTF8
  }

  input TextFragmentInput {
    """
    For small tests/dev fixtures only. Large production edits should use blob.
    """
    inlineUtf8: String
    blob: TextBlobInput
    byteLength: UInt64!
    lineCount: UInt64!
    contentHash: Hash!
  }

  input CreateBufferWorldlineInput {
    bufferKey: String!
    projectionPath: String
    initialText: String
    initialBlob: TextBlobInput
    createInitialCheckpoint: Boolean = true
  }

  input EditTargetInput {
    worldlineId: ID!
    braidId: ID
  }

  input ReplaceRangeAsTickInput {
    target: EditTargetInput!
    baseHeadId: ID!
    startByte: UInt64!
    endByte: UInt64!
    insert: TextFragmentInput!
    author: String
  }

  input TextWindowInput {
    worldlineId: ID
    braidId: ID
    headId: ID
    startLine: UInt64!
    lineCount: UInt64!
    maxBytes: UInt64!
  }

  input CreateCheckpointInput {
    worldlineId: ID!
    headId: ID!
    kind: CheckpointKind!
    label: String
  }

  input CreateBraidInput {
    baselineWorldlineId: ID!
    baselineHeadId: ID!
    label: String
  }

  input AppendBraidEditInput {
    braidId: ID!
    basisProjectionHeadId: ID!
    startByte: UInt64!
    endByte: UInt64!
    insert: TextFragmentInput!
    author: String
  }

  input TickSelectorInput {
    tickId: ID
    receiptId: ID
    sequenceNumber: UInt64
  }

  input UnapplyTickInput {
    target: EditTargetInput!
    currentHeadId: ID!
    tick: TickSelectorInput!
    policy: InversePolicy! = EXACT_SPAN_OR_OBSTRUCT
    author: String
  }

  input UnapplyTickSequenceInput {
    target: EditTargetInput!
    currentHeadId: ID!
    ticks: [TickSelectorInput!]!
    policy: InversePolicy! = EXACT_SPAN_OR_OBSTRUCT
    author: String
  }

  type Query {
    """
    Bounded read for editor rendering. Must not require full file materialization.
    """
    textWindow(input: TextWindowInput!): TextWindowReading!
      @wes_op(name: "textWindow", readonly: true)
      @wes_footprint(reads: ["BufferWorldline", "RopeHead", "RopeLeaf", "TextBlob"])

    braidProjection(input: TextWindowInput!): BraidProjectionReading!
      @wes_op(name: "braidProjection", readonly: true)
      @wes_footprint(reads: ["Braid", "BraidMember", "BufferWorldline", "RopeHead"])

    tickReceipt(receiptId: ID!): TickReceipt!
      @wes_op(name: "tickReceipt", readonly: true)
      @wes_footprint(reads: ["TickReceipt"])
  }

  type Mutation {
    createBufferWorldline(input: CreateBufferWorldlineInput!): CreateBufferWorldlineResult!
      @wes_op(name: "createBufferWorldline")
      @wes_footprint(
        creates: ["BufferWorldline", "RopeHead", "TextBlob", "RopeLeaf", "Tick", "TickReceipt", "Checkpoint"]
      )

    replaceRangeAsTick(input: ReplaceRangeAsTickInput!): ReplaceRangeAsTickResult!
      @wes_op(name: "replaceRangeAsTick")
      @wes_footprint(
        reads: ["BufferWorldline", "RopeHead", "RopeLeaf", "TextBlob"]
        writes: ["BufferWorldline"]
        creates: ["TextBlob", "RopeLeaf", "RopeHead", "Tick", "TickReceipt"]
      )

    createCheckpoint(input: CreateCheckpointInput!): Checkpoint!
      @wes_op(name: "createCheckpoint")
      @wes_footprint(
        reads: ["BufferWorldline", "RopeHead"]
        creates: ["Checkpoint", "Tick", "TickReceipt"]
      )

    createBraid(input: CreateBraidInput!): CreateBraidResult!
      @wes_op(name: "createBraid")
      @wes_footprint(
        reads: ["BufferWorldline", "RopeHead"]
        creates: ["Braid", "BraidMember", "Tick", "TickReceipt"]
      )

    """
    Creates a new edit strand/member forked from the current braid projection and
    appends it to the ordered braid. This is the sequential braid edit primitive.
    """
    appendBraidEdit(input: AppendBraidEditInput!): AppendBraidEditResult!
      @wes_op(name: "appendBraidEdit")
      @wes_footprint(
        reads: ["Braid", "BraidMember", "BufferWorldline", "RopeHead", "RopeLeaf", "TextBlob"]
        writes: ["Braid"]
        creates: ["BraidMember", "TextBlob", "RopeLeaf", "RopeHead", "Tick", "TickReceipt"]
      )

    """
    Append an inverse operation for one previous tick/receipt. Does not erase history.
    """
    unapplyTick(input: UnapplyTickInput!): UnapplyTickResult!
      @wes_op(name: "unapplyTick")
      @wes_footprint(
        reads: ["BufferWorldline", "RopeHead", "Tick", "TickReceipt", "TextBlob", "RopeLeaf"]
        writes: ["BufferWorldline"]
        creates: ["RopeHead", "Tick", "TickReceipt", "TextBlob", "RopeLeaf"]
      )

    """
    Append inverse operations for a sequence of previous ticks/receipts.
    """
    unapplyTickSequence(input: UnapplyTickSequenceInput!): UnapplyTickSequenceResult!
      @wes_op(name: "unapplyTickSequence")
      @wes_footprint(
        reads: ["BufferWorldline", "RopeHead", "Tick", "TickReceipt", "TextBlob", "RopeLeaf"]
        writes: ["BufferWorldline"]
        creates: ["RopeHead", "Tick", "TickReceipt", "TextBlob", "RopeLeaf"]
      )
  }

  Illustrated examples

  Example 1: Open a large file without reading all text

  1. Host chunks file contents into CAS or a retained blob store.
  2. Host submits a createBufferWorldline Intent with initialBlob.
  3. UI asks for only visible lines.

  Mutation vars:

  {
    "input": {
      "bufferKey": "file:/repo/src/main.ts",
      "projectionPath": "/repo/src/main.ts",
      "initialBlob": {
        "blobId": "cas:blake3:abc...",
        "contentHash": "abc...",
        "byteLength": 9800000,
        "encoding": "UTF8"
      },
      "createInitialCheckpoint": true
    }
  }

  Observation vars:

  {
    "input": {
      "worldlineId": "wl:main-ts",
      "startLine": 1200,
      "lineCount": 60,
      "maxBytes": 32768
    }
  }

  Expected behavior:

  - Echo returns only the requested text aperture.
  - ReadingEnvelope says whether the reading is complete or residual/budget-limited.
  - No full text String is required in the observer payload.

  Example 2: Typing "hello" and unapplying the third tick

  Initial worldline text is empty.

  Submit five replaceRangeAsTick Intents:

  C0 = insert "h" at [0,0)
  C1 = insert "e" at [1,1)
  C2 = insert "l" at [2,2)
  C3 = insert "l" at [3,3)
  C4 = insert "o" at [4,4)

  Reading at frontier yields:

  hello

  Now submit:

  {
    "input": {
      "target": { "worldlineId": "wl:hello" },
      "currentHeadId": "head:after-C4",
      "tick": { "tickId": "tick:C2" },
      "policy": "EXACT_SPAN_OR_OBSTRUCT",
      "author": "jedit"
    }
  }

  Expected history:

  C0, C1, C2, C3, C4, C5=inverse(C2)

  Expected reading:

  helo

  Required proof:

  - C2 still exists.
  - C5 has a receipt pointing to C2/C2 receipt.
  - Provenance length increased by 1.
  - No historical tick was deleted or rewritten.

  Example 3: Sequential braid edits

  Baseline file coordinate:

  baseline = worldline wl:file at head H0

  Create braid:

  {
    "input": {
      "baselineWorldlineId": "wl:file",
      "baselineHeadId": "H0",
      "label": "agent edit session"
    }
  }

  First braid edit:

  {
    "input": {
      "braidId": "braid:1",
      "basisProjectionHeadId": "H0",
      "startByte": 0,
      "endByte": 0,
      "insert": {
        "inlineUtf8": "h",
        "byteLength": 1,
        "lineCount": 0,
        "contentHash": "hash-h"
      },
      "author": "agent"
    }
  }

  Echo creates member S0. Projection is:

  baseline + S0

  Second braid edit targets current projection:

  {
    "input": {
      "braidId": "braid:1",
      "basisProjectionHeadId": "projection-after-S0",
      "startByte": 1,
      "endByte": 1,
      "insert": {
        "inlineUtf8": "e",
        "byteLength": 1,
        "lineCount": 0,
        "contentHash": "hash-e"
      },
      "author": "agent"
    }
  }

  Echo creates member S1. Projection is:

  (baseline + S0) + S1

  Required proof:

  - Each member has an orderIndex.
  - Each member records its source base/ref and projectionAfterDigest.
  - Observing braidProjection names the braid basis and residual/conflict posture.
  - No member mutates the baseline directly until settlement/admission Intent.

  Example 4: Unapply with obstruction

  If a user tries to unapply a tick whose deleted fragment is no longer retained, return:

  {
    "outcome": "OBSTRUCTED",
    "obstruction": {
      "reason": "INVERSE_FRAGMENT_UNAVAILABLE",
      "targetTickId": "tick:C2",
      "targetReceiptId": "receipt:C2",
      "message": "inverse fragment was not retained in hot/warm/cold storage"
    }
  }

  If a wormhole/compacted range exists and the receipt is cold, return either:

  - a rehydration-required obstruction, or
  - rehydrate from CAS and proceed.

  Acceptance tests

  Add RED tests first. Suggested tests:

  1. Generated mutation EINT reaches installed contract handler

  - Given a tiny generated contract with one mutation,
  - dispatch_intent(EINT) changes worldline state through Echo scheduling/provenance,
  - not through a direct test-only method.

  2. QueryView dispatches to installed contract observer

  - Given a generated query op,
  - observe(QueryView/Query) invokes the installed observer,
  - returns QueryBytes and ReadingEnvelope,
  - no UnsupportedQuery.

  3. Bounded text reading does not materialize full file

  - Use a jedit fixture with large blob-backed text.
  - textWindow for 60 lines returns only those lines.
  - Payload size is bounded.
  - ReadingEnvelope residual/budget posture is honest.

  4. All jedit mutations are Intents

  - createBufferWorldline, replaceRangeAsTick, createBraid, appendBraidEdit, unapplyTick all go through dispatch_intent.
  - No test calls a direct mutation service as the external path.

  5. Unapply appends inverse history

  - Build "hello" from five insert ticks.
  - unapplyTick(C2) appends one inverse tick.
  - Reading becomes "helo".
  - Original C2 remains in provenance.
  - New receipt links to C2 receipt.

  6. Unapply sequence preserves order and obstruction semantics

  - Unapply multiple target ticks.
  - Contract either applies inverse operations in deterministic order or returns typed obstructions.
  - Partial success must be explicitly represented, never hidden.

  7. Direct settlement mutation has Intent equivalent

  - Existing SettlementService may remain internal.
  - Add an EINT/contract operation that performs settlement through scheduler/admission.
  - Provenance records MergeImport or ConflictArtifact as before.

  8. Contract-aware reading identity

  - Same query, same basis, same vars gives same reading identity.
  - Change schema hash, op id, vars, or basis changes identity.
  - Unsupported/stale basis returns obstruction.

  9. CAS retention for inverse fragments

  - TickReceipt inverseBlob / inverseFragmentDigest resolves through retention.
  - If unavailable, unapply returns typed obstruction.
  - GC/compaction does not silently make false successful inverse edits.

  Non-goals

  - Do not add jedit text types to Echo core.
  - Do not add a special jedit ABI.
  - Do not invent a second intent envelope before proving EINT v1 cannot work.
  - Do not implement full production crypto before the admission posture RED.
  - Do not trust caller-supplied footprint JSON for scheduling independence.
  - Do not implement Graft automation in Echo core.
  - Do not make cached materialized text canonical truth.
  - Do not redefine wormholes as rope chunks, portals, or state zoom.

  Recommended implementation order

  1. RED: installed contract mutation handler behind dispatch_intent.
  2. GREEN: minimal toy generated contract mutates via Echo scheduling/provenance.
  3. RED: QueryView currently unsupported for generated query.
  4. GREEN: contract observer registry and QueryBytes reading.
  5. RED: bounded reading identity/residual posture.
  6. GREEN: bounded contract observer support.
  7. RED: intent-only external strand/braid/settlement mutation.
  8. GREEN: generic Intent wrappers and tests.
  9. RED: jedit fixture unapplyTick appends inverse tick.
  10. GREEN: contract inverse hook with jedit example fixture.
  11. RED/GREEN: CAS retention for inverse fragments and bounded text blobs.
  12. Design/RED: wormhole/checkpoint retention policy preserving inverse semantics.


  The key framing in that prompt is intentional: Echo should build the missing generic contract-hosting, observer, braid, inverse, and retention substrate; `jedit` should supply the text
  contract as a proof fixture and consumer.
````
