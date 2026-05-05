<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics API Design

Source request: [request.md](./request.md)

Depends on:

- [0011 - Optic and observer runtime doctrine](../0011-optic-observer-runtime-doctrine/design.md)
- [0013 - Wesley Compiled Contract Hosting Doctrine](../0013-wesley-compiled-contract-hosting-doctrine/design.md)
- [0014 - EINT, Registry, And Observation Boundary Inventory](../0014-eint-registry-observation-boundary-inventory/design.md)
- [Continuum Runtime And CAS Readings](../continuum-runtime-and-cas-readings.md)
- [Wesley-Compiled Optic Bindings For Echo](./wesley-compiled-optic-bindings.md)
- [Echo Optics API sequencing card](../../method/backlog/asap/PLATFORM_echo-optics-api-design.md)

## Summary

An Echo Optic is the first-class API noun for bounded, capability-scoped,
coordinate-anchored observation and intent dispatch over Echo causal history.

An optic is:

```text
Optic = capability + focus + coordinate + projection law + intent family
```

An optic is not a mutable handle. It is not a file handle, graph handle, object
handle, editor handle, or hidden materialization cursor.

An optic names two lawful things:

1. a bounded way to observe a focused projection at a causal coordinate;
2. a family of intents that may be proposed against that focused projection
   under an explicit causal basis.

The ideal API has one disciplined read path and one disciplined write-side
proposal path:

```text
observe_optic(request) -> reading | obstruction
dispatch_optic_intent(request) -> admission outcome
```

Everything else is support:

```text
open_optic      validates and names an optic descriptor
close_optic     releases session-local optic resources only
retain_reading  stores reading bytes plus their semantic read identity
reveal_reading  retrieves retained reading bytes only when identity matches
```

This design is generic. `jedit` may validate ergonomics as a future consumer,
but it is not the design target and must not create privileged text APIs in
Echo core.

Wesley-compiled output should target this model as generated optic bindings.
Generated bindings may hide byte-level EINT packing from application code, but
they must not hide intent dispatch from Echo. The request crossing into Echo
still names optic id, focus, base coordinate, capability, actor/cause, admission
law, intent family, and proposal payload.

## Core Doctrine

```text
Optic reads.
Intent proposes.
Echo admits.
Receipt witnesses.
```

The prohibitions are part of the API contract:

- no direct setters;
- no global graph API;
- no global mutable state API;
- no file-handle API;
- no hidden full-materialization fallback;
- no latest-writer-wins fallback;
- no stringly status outcome;
- no GraphQL-first runtime substrate;
- no host-bag abstractions such as `RuntimeFacade`, `ObservationManager`,
  `UniversalMaterializer`, or `GraphLikeRuntimeAdapter`.

Optic read truth is observer-relative and witness-backed. Substrate truth remains
the witnessed causal history and admitted receipts.

Optic intent dispatch is not mutation by handle. It is proposal against an
explicit causal basis. Echo may admit, stage, preserve plurality, conflict, or
obstruct. It must not silently mutate the current frontier when the caller named
a stale basis.

Generated code may make this ergonomic:

```rust
text_optic.dispatch_replace_range(port, base_coordinate, vars, actor, cause)
```

but the generated method must build and submit an explicit
`DispatchOpticIntentRequest`. It must not become a setter.

## Optic Model

An optic has five required components.

| Component      | Meaning                                                        |
| -------------- | -------------------------------------------------------------- |
| capability     | what actor/session/policy may reveal or propose                |
| focus          | what worldline, strand, braid, retained reading, or attachment |
| coordinate     | which causal frontier or historical point is being named       |
| projection law | how causal history lowers into a reading                       |
| intent family  | which proposal family is lawful against this focus             |

The focus is not global state. The same worldline may be observed through
different optics with different apertures, projection versions, capabilities,
rights, budgets, and retained-reading policies.

Optics range over:

- worldlines;
- strands;
- braids;
- coordinates/frontiers;
- retained readings;
- cached readings;
- observer apertures;
- witness-backed projections;
- explicit attachment boundaries.

The read path is:

```text
choose aperture
  -> slice causal history
  -> lower under law
  -> witness
  -> retain if needed
  -> emit observer-relative reading
```

The intent path is:

```text
construct intent
  -> validate capability
  -> validate causal basis
  -> apply/admit under contract law
  -> emit tick/admission result
  -> emit receipt/witness
```

## Public API Surface

The smallest useful Rust-facing API surface is:

```rust
pub trait EchoOptics {
    fn open_optic(&mut self, request: OpenOpticRequest)
        -> Result<OpenOpticResult, OpticOpenError>;

    fn close_optic(&mut self, request: CloseOpticRequest)
        -> Result<CloseOpticResult, OpticCloseError>;

    fn observe_optic(&self, request: ObserveOpticRequest)
        -> ObserveOpticResult;

    fn dispatch_optic_intent(&mut self, request: DispatchOpticIntentRequest)
        -> IntentDispatchResult;
}

pub trait EchoReadingRetention {
    fn retain_reading(&mut self, request: RetainReadingRequest)
        -> Result<RetainReadingResult, RetentionError>;

    fn reveal_reading(&self, request: RevealReadingRequest)
        -> Result<RevealReadingResult, RetentionError>;
}
```

The ABI or application adapter may expose camelCase wrappers such as
`openOptic`, `observeOptic`, and `dispatchOpticIntent`. Core Rust should follow
the repository's snake_case style.

The API deliberately separates responsibilities:

```text
Optic observes.
Admission admits.
Retention retains.
Plumber maintains.
Debug explains.
```

Plumber/debug APIs may inspect, repair, prewarm, materialize, or explain, but
they must be named as operational APIs. A public optic read must never call a
plumber/debug fallback and pretend the result is a witnessed bounded reading.

`close_optic` is intentionally weak. It releases session-local descriptor
resources. It does not mutate the subject, invalidate old readings, revoke
history, or close a file-like handle.

## Wesley Compiler Extension

Echo owns the Echo-facing Wesley compiler extension in `crates/echo-wesley-gen`.
That generator should compile Wesley contract operations into typed optic
bindings, not into Echo-core subclasses or app-specific runtime APIs.

The generated output should provide:

- contract family metadata;
- generated DTOs and canonical codecs;
- typed `OpenOpticRequest` builders;
- typed `ObserveOpticRequest` builders for query/read operations;
- typed `DispatchOpticIntentRequest` builders for mutation/proposal operations;
- optional convenience dispatch methods that still require explicit causal
  basis and call `dispatch_optic_intent`.

Current generated helpers remain useful during migration:

```text
*_observation_request(...)
pack_*_intent(...)
```

The preferred Optics surface should add:

```text
*_observe_optic_request(...)
*_dispatch_optic_intent_request(...)
```

The low-level EINT helper is allowed to be internal to the generated binding.
The Echo boundary remains explicit:

```text
EINT bytes are a binding implementation detail.
Intent dispatch is not an optic implementation detail.
```

See [Wesley-Compiled Optic Bindings For Echo](./wesley-compiled-optic-bindings.md)
for the generated API contract.

## Types And Interfaces

These sketches use Rust-style DTOs to show the intended public shape. They are
not committed wire formats.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpticId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionVersion(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReducerVersion(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOptic {
    pub optic_id: OpticId,
    pub focus: OpticFocus,
    pub coordinate: EchoCoordinate,
    pub projection_law: ProjectionLawRef,
    pub reducer_law: Option<ReducerLawRef>,
    pub intent_family: IntentFamilyRef,
    pub capability: OpticCapabilityRef,
}
```

Focus names the lawful subject without exposing a global graph handle:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpticFocus {
    Worldline { worldline_id: WorldlineId },
    Strand { strand_id: StrandId },
    Braid { braid_id: BraidId },
    RetainedReading { key: RetainedReadingKey },
    Attachment { owner: AttachmentOwnerRef, attachment_ref: AttachmentRef },
}
```

Coordinate names the causal basis:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EchoCoordinate {
    Worldline {
        worldline_id: WorldlineId,
        at: CoordinateAt,
    },
    Strand {
        strand_id: StrandId,
        at: CoordinateAt,
        parent_basis: Option<ProvenanceRef>,
    },
    Braid {
        braid_id: BraidId,
        projection_digest: Hash,
        member_count: u64,
    },
    RetainedReading {
        key: RetainedReadingKey,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateAt {
    Frontier,
    Tick(WorldlineTick),
    Provenance(ProvenanceRef),
}
```

Aperture is the bound on observation:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticAperture {
    pub shape: OpticApertureShape,
    pub budget: OpticReadBudget,
    pub attachment_descent: AttachmentDescentPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpticApertureShape {
    Head,
    SnapshotMetadata,
    TruthChannels { channels: Option<Vec<ChannelId>> },
    QueryBytes { query_id: u32, vars_digest: Hash },
    EntityRange { entity_family: EntityFamilyRef, range: ApertureRange },
    AttachmentBoundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticReadBudget {
    pub max_bytes: Option<u64>,
    pub max_nodes: Option<u64>,
    pub max_ticks: Option<u64>,
    pub max_attachments: Option<u64>,
}
```

The open request validates a descriptor. It does not open a mutable object:

```rust
pub struct OpenOpticRequest {
    pub focus: OpticFocus,
    pub coordinate: EchoCoordinate,
    pub projection_law: ProjectionLawRef,
    pub reducer_law: Option<ReducerLawRef>,
    pub intent_family: IntentFamilyRef,
    pub capability: OpticCapability,
    pub cause: OpticCause,
}

pub struct OpenOpticResult {
    pub optic: EchoOptic,
    pub capability_posture: CapabilityPosture,
}

pub struct CloseOpticRequest {
    pub optic_id: OpticId,
    pub cause: OpticCause,
}

pub struct CloseOpticResult {
    pub optic_id: OpticId,
}
```

Observation names optic, aperture, coordinate, and versions:

```rust
pub struct ObserveOpticRequest {
    pub optic_id: OpticId,
    pub focus: OpticFocus,
    pub coordinate: EchoCoordinate,
    pub aperture: OpticAperture,
    pub projection_version: ProjectionVersion,
    pub reducer_version: Option<ReducerVersion>,
    pub capability: OpticCapabilityRef,
}

pub enum ObserveOpticResult {
    Reading(OpticReading),
    Obstructed(OpticObstruction),
}

pub struct OpticReading {
    pub envelope: ReadingEnvelope,
    pub read_identity: ReadIdentity,
    pub payload: ObservationPayload,
    pub retained: Option<RetainedReadingKey>,
}
```

Reading identity names the question, not just the bytes:

```rust
pub struct ReadIdentity {
    pub optic_id: OpticId,
    pub focus_digest: Hash,
    pub coordinate: EchoCoordinate,
    pub aperture_digest: Hash,
    pub projection_version: ProjectionVersion,
    pub reducer_version: Option<ReducerVersion>,
    pub witness_basis: WitnessBasis,
    pub rights_posture: ReadingRightsPosture,
    pub budget_posture: ReadingBudgetPosture,
    pub residual_posture: ReadingResidualPosture,
}

pub struct RetainedReadingKey {
    pub read_identity: ReadIdentity,
    pub content_hash: Hash,
    pub codec_id: CodecId,
    pub byte_len: u64,
}
```

Witness basis must be honest about checkpoint plus live tail:

```rust
pub enum WitnessBasis {
    ResolvedCommit {
        reference: ProvenanceRef,
        state_root: Hash,
        commit_hash: Hash,
    },
    CheckpointPlusTail {
        checkpoint_ref: ProvenanceRef,
        checkpoint_hash: Hash,
        tail_witness_refs: Vec<ProvenanceRef>,
        tail_digest: Hash,
    },
    WitnessSet {
        refs: Vec<ReadingWitnessRef>,
        witness_set_hash: Hash,
    },
    Missing {
        reason: OpticObstructionKind,
    },
}
```

Intent dispatch names the proposal and its causal base:

```rust
pub struct DispatchOpticIntentRequest {
    pub optic_id: OpticId,
    pub base_coordinate: EchoCoordinate,
    pub intent_family: IntentFamilyRef,
    pub focus: OpticFocus,
    pub actor: OpticActor,
    pub cause: OpticCause,
    pub capability: OpticCapabilityRef,
    pub admission_law: AdmissionLawRef,
    pub intent: OpticIntentPayload,
}

pub enum OpticIntentPayload {
    EintV1 { bytes: Vec<u8> },
    ContractOp {
        op_id: u32,
        vars_bytes: Vec<u8>,
        vars_digest: Hash,
    },
}

pub enum IntentDispatchResult {
    Admitted(AdmittedIntent),
    Staged(StagedIntent),
    Plural(PluralIntent),
    Conflict(IntentConflict),
    Obstructed(OpticObstruction),
}
```

Capability is explicit and auditable:

```rust
pub struct OpticCapability {
    pub capability_ref: OpticCapabilityRef,
    pub subject: OpticActor,
    pub allowed_focus: Vec<OpticFocusPattern>,
    pub allowed_apertures: Vec<OpticAperturePattern>,
    pub allowed_intent_families: Vec<IntentFamilyRef>,
    pub max_budget: OpticReadBudget,
    pub rights: OpticRights,
}

pub struct OpticCapabilityRef {
    pub capability_id: Hash,
    pub issuer_ref: Option<ProvenanceRef>,
    pub policy_id: Hash,
}
```

Obstruction is typed:

```rust
pub struct OpticObstruction {
    pub kind: OpticObstructionKind,
    pub optic_id: Option<OpticId>,
    pub focus: Option<OpticFocus>,
    pub coordinate: Option<EchoCoordinate>,
    pub witness_basis: Option<WitnessBasis>,
    pub message: String,
}

pub enum OpticObstructionKind {
    MissingWitness,
    MissingRetainedReading,
    StaleBasis,
    CapabilityDenied,
    BudgetExceeded,
    UnsupportedAperture,
    UnsupportedProjectionLaw,
    UnsupportedIntentFamily,
    AttachmentDescentRequired,
    AttachmentDescentDenied,
    LiveTailRequiresReduction,
    ConflictingFrontier,
    PluralityRequiresExplicitPolicy,
}
```

## Read Semantics

An optic read must be bounded.

Every read request names:

- optic id;
- focus;
- aperture;
- causal coordinate/frontier;
- projection law/version;
- reducer law/version where relevant;
- witness basis;
- read identity;
- residual or obstruction posture;
- bounds/budget posture;
- rights posture.

Read execution must not fall back to full materialization. If the requested
aperture cannot be answered from available evidence, the result is
`OpticObstruction`, not a large scan disguised as success.

`ObservationService::observe(...)` is the current read boundary. The first
Optics API should wrap and sharpen that boundary rather than replace it. Existing
`ObservationRequest`, `ObservationArtifact`, `ReadingEnvelope`, and
`ObservationPayload::QueryBytes` remain the nearest concrete implementation
surface.

## Intent Dispatch Semantics

The optic write-side surface is `dispatch_optic_intent`. The name intentionally
uses dispatch/propose language, not set/update/mutate.

Every dispatch names:

- optic id;
- base coordinate/frontier;
- intent family;
- subject/focus;
- actor/cause;
- capability basis;
- admission law;
- intent payload;
- resulting tick, receipt, or admission posture.

If the base coordinate is stale, Echo must not silently mutate the current
frontier. It may:

- reject;
- obstruct;
- stage;
- preserve plurality;
- require rebase;
- admit under an explicitly named law.

`dispatch_optic_intent` should initially reuse EINT v1 where possible. A future
outer admission certificate may bind capability, contract identity, and causal
basis, but that certificate must remain explicit. It must not become hidden
host-side mutation authority.

Implementation note: the first Rust/ABI slice now exposes
`DispatchOpticIntentRequest`, `OpticIntentPayload::EintV1`, and
`AdmissionLawId`. The shared `KernelPort` default validates focus, base
coordinate, capability, actor/cause, and intent family before routing EINT v1
bytes through the existing `dispatch_intent` path. Because that existing path
ingests into the runtime inbox instead of proving a committed tick/receipt, the
optic result is `IntentDispatchResult::Staged` with an explicit stage ref and
reason, not a fabricated `Admitted` result.

Stale-basis validation is explicit. Core proposals can be checked against a
known current coordinate, and engine-backed dispatch resolves the current
worldline coordinate before staging. If the proposal names an older concrete
base, dispatch returns `OpticObstructionKind::StaleBasis`; it does not enqueue
the EINT or advance current provenance.

Generated-binding readiness is ABI-level, not a generator promise. The current
`echo-wasm-abi` DTO set exposes the optic ids, focus/coordinate/aperture
models, observe and dispatch requests, EINT payload wrapper, typed admission
result, and support refs needed by generated helper code. `echo-wesley-gen`
tests include a hand-written generated-helper-shaped smoke crate so ABI drift
breaks before the generator implementation task.

The first concrete implementation is deliberately narrow. `WorldlineHeadOptic`
is a generic request-builder example over a worldline head, not a mutable handle
and not a universal optic engine. It builds bounded head-read requests,
QueryBytes-shaped requests that currently return typed projection-law
obstructions when no contract observer is installed, and EINT v1 dispatch
requests with explicit base coordinates. Engine-backed `warp-wasm` now exposes
`observe_optic` beside `dispatch_optic_intent`, so the example validates the
same ABI/kernel boundary future generated bindings will use.

## Admission Outcomes

Admission outcomes are not `Ok/Err`.

The API family is:

```rust
pub enum IntentDispatchResult {
    Admitted(AdmittedIntent),
    Staged(StagedIntent),
    Plural(PluralIntent),
    Conflict(IntentConflict),
    Obstructed(OpticObstruction),
}
```

Meanings:

- `Admitted`: Echo accepted the intent into witnessed history and can name the
  resulting tick/admission receipt.
- `Staged`: Echo retained the proposal for explicit later admission, review, or
  rebase. It did not mutate the named frontier.
- `Plural`: Echo preserved lawful plurality instead of forcing a single latest
  result.
- `Conflict`: Echo found incompatible causal claims under the named admission
  law.
- `Obstructed`: Echo cannot lawfully proceed because evidence, rights, basis,
  budget, attachment, or projection law is missing.

No outcome may be represented as boolean success, latest-writer-wins, hidden
host-time ordering, or string status.

## Cached And Retained Readings

`echo-cas` names bytes. A read identity names the question those bytes answer.

A retained reading therefore needs both:

```text
content_hash       -> byte identity
read_identity      -> semantic coordinate and law identity
```

A cached reading is valid only for exactly the identity it names:

- coordinate/frontier;
- witness basis;
- projection version;
- reducer version;
- aperture;
- rights posture;
- budget posture;
- residual posture.

New ticks create new frontiers. They do not mutate old readings. A retained
reading can be reused only when its `ReadIdentity` is exactly valid for the
request or when an explicit witness relation proves containment/equivalence and
the returned identity names that relation.

`retain_reading` and `reveal_reading` belong to retention, not optic mutation.
They must not create or alter substrate truth.

The initial core surface is intentionally small: `RetainedReadingCache` stores
encoded reading bytes under a `RetainedReadingDescriptor`, and
`reveal_reading` succeeds only when the retained key and exact `ReadIdentity`
match. A content hash can index candidate retained readings for diagnostics, but
it is not reveal authority and is not the cache key.

## Live Tail Honesty

An optic read must not return a stale checkpoint hash as if it identified the
live result.

Honest options are:

- reduce the live tail under a bounded witness basis;
- return a read identity that names checkpoint basis plus tail witness set;
- return a slice hash or witness-set hash with explicit meaning;
- fail closed with obstruction or missing-basis posture.

The key law:

```text
checkpoint_hash != live_read_identity
```

unless the read identity also proves there is no live tail or names the exact
tail witness set included in the reading.

The initial `observe_optic` bridge uses the second honest option when a replay
checkpoint exists behind the live frontier: the `ReadIdentity` witness basis is
`CheckpointPlusTail`, with the checkpoint basis, post-checkpoint provenance refs,
and a tail digest. If that tail cannot be enumerated within the requested tick
budget, the read must fail closed instead of identifying the live result with
the checkpoint alone.

## Attachments And Recursive Apertures

Attachments are aperture boundaries.

Default readings should expose attachment refs, attachment posture, or
obstruction. They must not recursively load attachment content.

Recursive descent into attachments requires:

- explicit aperture request;
- explicit capability rights;
- budget sufficient for the descent;
- projection law for the nested subject;
- witness basis for the boundary;
- residual/obstruction posture if descent is refused or incomplete.

This applies equally to content blobs, causal artifacts, retained readings,
receipt/witness refs, foreign suffix shells, and future nested WARP coordinates.

The initial bridge implements the boundary as a typed fail-closed posture:
`AttachmentBoundary` focus with `BoundaryOnly` descent returns
`AttachmentDescentRequired`, carrying the attachment key in the obstruction
focus. `Explicit` descent requires a positive attachment budget and then returns
`AttachmentDescentDenied` until an attachment projection law and capability
checker are installed. No nested WARP or attachment payload is materialized by
default.

## Capability Model

Capabilities limit both reveal and proposal.

Read capability controls:

- focus scope;
- allowed apertures;
- attachment descent;
- max budget;
- rights posture;
- retained-reading reveal.

Intent capability controls:

- allowed intent families;
- base-coordinate policy;
- admission law;
- actor/cause binding;
- whether staging, plurality preservation, conflict artifact creation, or rebase
  request is allowed.

Capabilities must be explicit request fields or resolvable by explicit
capability refs. They must not be ambient host state hidden behind a runtime bag.

## Relationship To Existing Echo Doctrine

This design aligns with existing Echo doctrine:

- **Witnessed causal history as substrate truth:** optics read and propose
  against history; admitted ticks and receipts remain truth.
- **Observer-relative readings:** `ReadingEnvelope`, observer basis, projection
  law, reducer law, residual posture, and rights posture describe the emitted
  reading.
- **Bounded replay/reveal:** aperture and witness basis make bounded reads and
  retained reveal explicit.
- **Suffix admission:** optic dispatch can feed existing admission/evaluator
  paths; it does not invent a sync daemon.
- **Tick receipts as holographic witnesses:** admitted intent outcomes must name
  ticks, receipts, and witness refs.
- **echo-cas as retention, not ontology:** CAS stores bytes; `ReadIdentity`
  stores semantic meaning.
- **Deterministic boundaries:** serde may exist on non-authoritative adapter or
  diagnostic shapes, but it must not be the authority for intents,
  graph-preserved facts, receipts, witness material, read identity, retained
  reading identity, or causal history. Those surfaces enter Echo as
  domain-separated canonical bytes, and values with nondeterministic encodings
  such as floats must be normalized before admission or retention.
- **Echo as peer Continuum runtime:** optics are Echo-local runtime/API law.
  They do not make Echo subordinate to git-warp and do not require a git-warp
  dependency.

This design also sharpens earlier doctrine from `0011`: an optic is the runtime
boundary object, an observer is revelation-side, and reading artifacts are
observer-relative emissions with coordinate and witness posture.

## Test Strategy

Initial RED tests should prove:

- optic reads name causal basis in the returned `ReadIdentity`;
- optic reads are bounded by `OpticAperture` and `OpticReadBudget`;
- missing evidence returns `OpticObstruction`, not full materialization;
- cached readings are keyed by `ReadIdentity`, not only content hash;
- live-tail reads do not reuse stale checkpoint hashes;
- intent dispatch requires explicit base coordinate;
- stale base coordinate does not silently mutate current frontier;
- admission outcomes remain typed as admitted, staged, plural, conflict, or
  obstructed;
- attachment descent is explicit and capability/budget checked;
- plumber/debug APIs cannot satisfy public optic reads as hidden fallbacks.

Use a fake/example optic implementation before broadening:

- one worldline head optic;
- one QueryView-style contract optic;
- one retained reading reveal;
- one attachment-boundary placeholder.

No test should call a global graph mutation or direct setter as the public path.

## Backlog

The executable versions of these tasks are top-level METHOD backlog cards
linked from
[PLATFORM_echo-optics-api-design](../../method/backlog/asap/PLATFORM_echo-optics-api-design.md).
This section records the source task detail for the design packet; it is not
the scheduling surface. New executable work should be added as a visible card
with explicit `Depends on:` links.

### TASK-001: Add Echo Optics doctrine packet

Title: Add Echo Optics doctrine packet.

Goal: Land the controlling design that defines optics as bounded,
capability-scoped, coordinate-anchored read/propose surfaces.

Files likely touched:

- `docs/design/0018-echo-optics-api-design/design.md`
- `docs/design/0018-echo-optics-api-design/request.md`
- `docs/method/backlog/asap/PLATFORM_echo-optics-api-design.md`

Acceptance criteria:

- Design uses the required output sections.
- Design rejects direct mutation, global graph/state APIs, and host-bag
  abstractions.
- Backlog sequence is METHOD-friendly and small-sliced.

Non-goals:

- Do not implement runtime code.
- Do not design jedit as the primary subject.

Test expectations:

- `markdownlint-cli2` and Prettier pass.
- `pnpm docs:build` passes.

### TASK-002: Define core optic nouns and IDs

Title: Define core optic nouns and IDs.

Goal: Add initial Rust DTOs for `EchoOptic`, `OpticId`, `OpticFocus`,
`OpticAperture`, `EchoCoordinate`, `ProjectionVersion`, and `ReducerVersion`.

Files likely touched:

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/lib.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

Acceptance criteria:

- DTOs are deterministic, canonical where ABI-facing, and domain-separated where
  hashed. Serde is not the authoritative encoding for any causal or retained
  identity surface.
- Focus covers worldline, strand, braid, retained reading, and attachment
  boundary without exposing a global graph handle.

Non-goals:

- Do not add a universal optic engine.
- Do not add jedit/editor/file types.

Test expectations:

- Unit tests for stable ID hashing and focus/coordinate encoding.
- ABI round-trip tests for public DTOs.

### TASK-003: Define ReadingEnvelope and ReadIdentity extensions

Title: Define ReadingEnvelope and ReadIdentity.

Goal: Extend current reading metadata with first-class read identity fields
without breaking existing observation behavior.

Files likely touched:

- `crates/warp-core/src/observation.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

Acceptance criteria:

- Read identity names optic id, focus digest, coordinate, aperture digest,
  projection version, reducer version, witness basis, rights, budget, and
  residual posture.
- Existing observations can produce compatible identity for built-in plans.

Non-goals:

- Do not make CAS hash the read identity.
- Do not require full materialization to compute identity.

Test expectations:

- Same read question yields same identity.
- Coordinate, aperture, projection version, or witness basis changes identity.

### TASK-004: Define WitnessBasis and retained reading key

Title: Define WitnessBasis and retained reading key.

Goal: Represent commit, checkpoint-plus-tail, witness-set, and missing-basis
postures for honest retained readings.

Files likely touched:

- `crates/warp-core/src/observation.rs`
- `crates/echo-cas/src/lib.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

Acceptance criteria:

- Retained reading key includes content hash and semantic read identity.
- Checkpoint-plus-tail identity cannot collapse to checkpoint hash alone.

Non-goals:

- Do not build storage GC policy.
- Do not implement proof systems.

Test expectations:

- Retained reading lookup by content hash alone fails.
- Checkpoint-plus-tail and checkpoint-only identities differ.

### TASK-005: Define obstruction and admission result families

Title: Define optic obstruction and admission result families.

Goal: Add typed `OpticObstruction` and `IntentDispatchResult` enums.

Files likely touched:

- `crates/warp-core/src/optic.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

Acceptance criteria:

- Outcomes include Admitted, Staged, Plural, Conflict, and Obstructed.
- Stale basis, missing witness, budget exceeded, capability denied, and
  attachment descent required are distinct obstruction kinds.

Non-goals:

- Do not model outcomes as `Ok/Err`, bool, or string status.
- Do not introduce latest-writer-wins fallback.

Test expectations:

- ABI serialization preserves outcome variants.
- Exhaustive matching tests fail if variants collapse.

### TASK-006: Define open_optic and close_optic request models

Title: Define open optic and close optic request models.

Goal: Add descriptor-validation DTOs for opening and closing session-local optic
resources.

Files likely touched:

- `crates/warp-core/src/optic.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

Acceptance criteria:

- `open_optic` validates focus, coordinate, projection law, intent family, and
  capability.
- `close_optic` releases only session-local descriptor resources.
- Closing an optic does not mutate subject history or invalidate old readings.

Non-goals:

- Do not make optics file handles.
- Do not implement mutable object handles.

Test expectations:

- Opening denied capability returns typed obstruction/error.
- Closing does not change observed frontier or provenance length.

### TASK-007: Define observe_optic model with bounds and aperture

Title: Define observe optic model with bounds and aperture.

Goal: Add the bounded read request/result model and adapt one existing
ObservationService path through it.

Files likely touched:

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/observation.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

Acceptance criteria:

- Observe request includes optic id, focus, coordinate, aperture, projection
  version, reducer version, and capability ref.
- Result returns reading or obstruction.
- No hidden full materialization fallback exists.

Non-goals:

- Do not replace all ObservationService internals in this slice.
- Do not add global graph query API.

Test expectations:

- Bounded head/snapshot optic returns read identity.
- Oversized aperture returns budget obstruction.

### TASK-008: Define dispatch_optic_intent with explicit base coordinate

Title: Define dispatch optic intent with explicit base coordinate.

Goal: Add the write-side proposal DTO and route one existing EINT path through
the optic dispatch model.

Files likely touched:

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/head_inbox.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

Acceptance criteria:

- Request names optic id, base coordinate, intent family, focus, actor/cause,
  capability, admission law, and payload.
- Current EINT v1 payloads can be carried.
- Dispatch outcome is typed.

Non-goals:

- Do not add setters.
- Do not create a second intent envelope without a failing RED.

Test expectations:

- Missing base coordinate is impossible or rejected.
- Accepted intent names resulting tick/receipt/admission posture.

### TASK-009: Add stale-basis obstruction tests

Title: Add stale-basis obstruction tests.

Goal: Prove stale base coordinate does not silently mutate current frontier.

Files likely touched:

- `crates/warp-core/tests/optic_dispatch_tests.rs`
- `crates/warp-core/src/optic.rs`

Acceptance criteria:

- Dispatch against stale base returns Obstructed, Staged, Plural, Conflict, or
  explicitly law-admitted result.
- The default path must not mutate latest frontier silently.

Non-goals:

- Do not implement rebase workflow.
- Do not hide host-time ordering.

Test expectations:

- Provenance length and current head remain unchanged for obstructed stale-base
  dispatch.

### TASK-010: Add cached-reading identity tests

Title: Add cached-reading identity tests.

Goal: Prove retained/cached readings are keyed by read identity, not just
content hash.

Files likely touched:

- `crates/warp-core/tests/optic_retention_tests.rs`
- `crates/echo-cas/src/lib.rs`
- `crates/warp-core/src/observation.rs`

Acceptance criteria:

- Same content bytes under different coordinate or aperture produce distinct
  retained keys.
- Reveal requires matching read identity.

Non-goals:

- Do not build distributed CAS.
- Do not add semantic ontology to CAS hashes.

Test expectations:

- Content-hash-only reveal returns obstruction or lookup miss.
- Matching read identity reveals payload.

### TASK-011: Add live-tail hash honesty tests

Title: Add live-tail hash honesty tests.

Goal: Prevent stale checkpoint hashes from identifying live optic readings.

Files likely touched:

- `crates/warp-core/tests/optic_live_tail_tests.rs`
- `crates/warp-core/src/observation.rs`

Acceptance criteria:

- A live frontier with checkpoint plus tail cannot return checkpoint-only
  identity.
- Result either reduces live tail, names checkpoint-plus-tail witness basis, or
  obstructs.

Non-goals:

- Do not implement production compaction/wormholes.

Test expectations:

- Add tick after checkpoint; live read identity changes and names tail evidence.

### TASK-012: Add attachment boundary/descent placeholder model

Title: Add attachment boundary and descent placeholder model.

Goal: Make attachments explicit aperture boundaries in optic reads.

Files likely touched:

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/attachment.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

Acceptance criteria:

- Default readings expose attachment refs or obstruction posture.
- Recursive descent requires explicit aperture, capability, budget, and law.

Non-goals:

- Do not recursively materialize attachments by default.
- Do not implement nested WARP runtime.

Test expectations:

- Read with no descent returns attachment boundary posture.
- Read with unauthorized descent returns typed obstruction.

### TASK-013: Add narrow fake/example optic implementation

Title: Add narrow fake/example optic implementation.

Goal: Implement one simple optic path to validate ergonomics without broad
runtime abstraction.

Files likely touched:

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/tests/optic_example_tests.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

Acceptance criteria:

- Example optic can read a worldline head or QueryBytes-style payload.
- Example optic can dispatch one EINT intent with explicit base coordinate.
- It uses typed read/admission outcomes.

Non-goals:

- Do not implement a universal optic engine.
- Do not use jedit as the concrete runtime dependency.

Test expectations:

- Read, dispatch, stale-basis, and obstruction tests pass on the example.

### TASK-014: Add adapter notes for future consumers

Title: Add adapter notes for future consumers.

Goal: Document how editors, debuggers, inspectors, replay tools, import/export
flows, retained reading caches, and GraphQL adapters should sit above the core
Optics API.

Files likely touched:

- `docs/architecture/echo-optics-adapter-notes.md`
- `docs/design/0018-echo-optics-api-design/design.md`

Acceptance criteria:

- Notes clearly say GraphQL is an adapter illustration, not the runtime
  substrate.
- Notes reject global state adapters and host-bag abstractions.
- Notes show `jedit` only as an ergonomic example consumer.

Non-goals:

- Do not design product-specific APIs.
- Do not add a sync daemon or git-warp dependency.

Test expectations:

- Docs checks pass.
- Links from design packet and backlog card resolve in docs build.

### TASK-015: Add Echo-owned Wesley optic binding spec

Title: Add Echo-owned Wesley optic binding spec.

Goal: Specify how `echo-wesley-gen` emits typed optic families and bindings
without turning Echo core into application subclasses.

Files likely touched:

- `docs/design/0018-echo-optics-api-design/wesley-compiled-optic-bindings.md`
- `docs/design/0018-echo-optics-api-design/design.md`
- `docs/method/backlog/asap/PLATFORM_echo-optics-api-design.md`

Acceptance criteria:

- Spec says generated output builds `ObserveOpticRequest` and
  `DispatchOpticIntentRequest`.
- Spec says EINT packing may be hidden but intent dispatch remains explicit at
  the Echo boundary.
- Spec rejects generated setters and mutable handles.

Non-goals:

- Do not implement generator changes.
- Do not invent a replacement for EINT v1.

Test expectations:

- Docs checks pass.

### TASK-016: Extend echo-wesley-gen with optic request builders

Title: Extend echo-wesley-gen with optic request builders.

Goal: Generate typed `*_observe_optic_request` and
`*_dispatch_optic_intent_request` helpers alongside existing compatibility
helpers.

Files likely touched:

- `crates/echo-wesley-gen/src/main.rs`
- `crates/echo-wesley-gen/src/ir.rs`
- `crates/echo-wesley-gen/tests/generation.rs`
- `crates/echo-wesley-gen/tests/fixtures/toy-counter/echo-ir-v1.json`

Acceptance criteria:

- Query ops emit typed optic observation request builders.
- Mutation ops emit typed optic dispatch request builders.
- Mutation builders require explicit base coordinate by default.
- Existing EINT and `ObservationRequest` helpers remain available.
- Generated names do not collide with user contract types.

Non-goals:

- Do not remove existing helper surface in this slice.
- Do not add jedit-specific codegen.

Test expectations:

- Generated std smoke crate compiles.
- Generated no-std smoke crate compiles where request builders are no-std-safe.
- Tests assert no generated method uses `set_*` naming.

### TASK-017: Add Echo Optics ABI DTOs required by generated bindings

Title: Add Echo Optics ABI DTOs required by generated bindings.

Goal: Add the minimum ABI DTOs needed for generated optic request builders to
compile against `echo-wasm-abi`.

Files likely touched:

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/echo-wesley-gen/tests/generation.rs`

Acceptance criteria:

- ABI exposes `OpticId`, `OpticFocus`, `EchoCoordinate`, `OpticAperture`,
  `ObserveOpticRequest`, `DispatchOpticIntentRequest`,
  `OpticIntentPayload`, `IntentDispatchResult`, and supporting refs.
- DTOs encode deterministically across the ABI boundary.
- Generated optic helper smoke crate compiles against the ABI.

Non-goals:

- Do not implement full runtime semantics.
- Do not add global graph APIs.

Test expectations:

- ABI encode/decode round-trips.
- Generated consumer crate compiles with generated optic helpers.
