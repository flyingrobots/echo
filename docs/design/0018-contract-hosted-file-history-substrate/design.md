<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0018 - Contract-Hosted File History Substrate

_Make Echo capable of hosting a Wesley-compiled application contract that models
file history, while keeping Echo core generic._

Legend: [PLATFORM](../../method/legends/PLATFORM.md),
[KERNEL](../../method/legends/KERNEL.md)

Source request:
[request.md](./request.md)

Depends on:

- [0013 - Wesley Compiled Contract Hosting Doctrine](../0013-wesley-compiled-contract-hosting-doctrine/design.md)
- [0014 - EINT, Registry, And Observation Boundary Inventory](../0014-eint-registry-observation-boundary-inventory/design.md)
- [0015 - Registry Provider Host Boundary Decision](../0015-registry-provider-host-boundary-decision/design.md)
- [0016 - Wesley To Echo Toy Contract Proof](../0016-wesley-to-echo-toy-contract-proof/design.md)
- [0017 - Authenticated Wesley Intent Admission Posture](../0017-authenticated-wesley-intent-admission-posture/design.md)
- [Contract-hosted file history substrate roadmap](../../method/backlog/asap/PLATFORM_contract-hosted-file-history-substrate.md)

## Status

Proposed.

## Hill

Echo should host a Wesley-compiled contract family that models a file as Echo
history without becoming a text editor runtime.

The file is not "base text plus local editor patches" as canonical truth. The
file is a materialized reading at an Echo coordinate:

```text
worldline | strand | braid projection
  -> observation/read law
  -> ReadingEnvelope + payload bytes
```

All mutations must enter Echo as intents. Reads must go through observations.
Undo or "unapply" must append witnessed inverse history. It must never delete,
rewrite, or hide old ticks.

`jedit` is the proof fixture and first serious consumer. It is not an Echo core
ontology.

## Doctrine

Echo remains a generic deterministic witnessed causal substrate.

Echo core must not grow privileged APIs or types for:

- text editing;
- `jedit`;
- Graft;
- editors;
- ropes;
- file buffers.

Application contracts may define those nouns. Wesley may compile them. Echo may
host the compiled artifact through generic contract-hosting, ingress,
observation, provenance, strand, braid, inverse, and retention surfaces.

The hard boundary is:

```text
application noun -> generated contract artifact
generic Echo substrate -> installed artifact host
```

Echo may store generated adapters as trait objects or static registries, but the
kernel-facing boundary must stay app-agnostic.

## Current Repo Truth

The useful current path is already present:

```text
EINT v1 = "EINT" || op_id:u32le || vars_len:u32le || vars
KernelPort::dispatch_intent(bytes)
warp-wasm dispatch_intent(bytes) and observe(bytes)
IngressEnvelope::local_intent
SchedulerCoordinator::super_tick
Engine cmd/* rule dispatch
echo-wesley-gen EINT mutation helpers
echo-wesley-gen ObservationRequest query helpers
echo-registry-api::RegistryProvider
ObservationRequest { frame: QueryView, projection: Query { ... } }
Provenance, replay patches, playback seek/checkpoints
Session-scoped strands, support pins, NeighborhoodSite, SettlementService
echo-cas content-addressed storage
```

The missing pieces are the installed contract host, QueryView dispatch,
contract-aware readings, intent-only external runtime operations, first-class
generic braids, contract-defined inverse admission, and retention/streaming
rules that keep large files out of full materialization.

## Installed Contract Host

Echo needs a generic installed contract host that can bind generated Wesley
artifacts to runtime ingress and observation.

Candidate shape:

```text
Application GraphQL
  -> Wesley IR
  -> generated DTOs/codecs/op ids/registry/handlers
  -> installed Echo contract host
  -> dispatch_intent(EINT bytes)
  -> ingress/scheduling/provenance
  -> mutation handler transition law
  -> query handler read law
  -> ReadingEnvelope + payload bytes
```

Initial implementation should reuse EINT v1 and
`echo-registry-api::RegistryProvider` unless a RED test proves those surfaces
cannot carry the required identity.

The installed host must:

- reject unsupported op ids when validation is enabled;
- decode vars through the registered generated codec;
- resolve footprint authority from the verified artifact, not from caller JSON;
- run mutation handlers inside admission, witness, and provenance;
- record enough artifact identity for receipts and readings;
- return typed obstructions instead of fake success.

## QueryView Observer Bridge

`echo-wesley-gen` can build `ObservationRequest` values for QueryView queries,
but `ObservationService` currently rejects QueryView as unsupported. The bridge
should dispatch QueryView/Query to an installed contract observer when the
contract host can handle the query op.

Query results should return:

```text
ObservationPayload::QueryBytes(bytes)
```

The `ReadingEnvelope` for contract observations must name enough evidence for a
reader to know what was observed:

- observed coordinate;
- contract family, artifact, and schema identity;
- query op id;
- vars digest;
- observer/read law version when available;
- witness refs;
- budget posture;
- rights posture;
- residual, plurality, conflict, or obstruction posture.

Unsupported query ops must return typed obstruction/error. They must not return
an empty success payload.

Contract observers must support bounded readings. A text-window query must be
able to read a visible aperture without materializing the full file.

## Intent-Only External Mutation

Existing internal services may remain implementation details, but external
mutation surfaces for contract families, strands, braids, settlement, and
inverse operations need intent paths.

The external shape should be:

```text
EINT bytes
  -> IngressEnvelope
  -> scheduler/admission
  -> generated or generic handler
  -> witnessed tick/provenance
```

Direct settlement, strand creation, support pinning, provenance fork, braid
member append, braid settlement, and inverse operations can keep direct internal
service calls, but the jedit-style proof path must not need those calls as its
external API.

## Generic Braids

`jedit` wants a file modeled as a worldline plus an ordered braid of strands
over that worldline. Echo should add a generic braid substrate, not a text type.

The simple sequential braid law is:

```text
baseline = worldline at B0
S0 forks from baseline
projection = baseline + S0

S1 forks from current projection
projection = (baseline + S0) + S1

S2 forks from current projection
projection = ((baseline + S0) + S1) + S2
```

A `Braid` should name:

- braid id;
- baseline worldline/ref;
- ordered member refs;
- current projection ref/digest;
- contract family/schema identity when contract-backed;
- basis/revalidation posture.

Braid projection must be observable. It must be able to return complete,
residual, plural, obstructed, or conflict posture. Braid member append and
settlement/collapse/admission must be intents.

Support pins remain geometry. They must not be flattened into settlement
imports.

## Contract Inverse Admission

Undo-as-history is contract-defined.

The law:

```text
unapply(target tick)
  -> ask installed contract for inverse intent(s)
  -> admit inverse intent(s) as normal history
  -> retain receipts linking inverse ticks to target receipts
```

The original target tick remains in provenance. The inverse appends new
history. If the original causal span no longer maps cleanly to the current
frontier, the contract returns a typed obstruction or conflict.

Echo should not expose a generic blind inverse of `WarpOp` as the application
undo model. WarpOp patches are replay artifacts. Domain inverse law belongs to
the contract family.

## Retention, CAS, And Streaming

Large files must not require full text materialization.

CAS remains content-only. Semantic references above CAS must carry contract,
schema, type, layout, codec, and hash identity.

The contract-hosting path needs:

- payloads that can refer to retained text/blob fragments by CAS ref;
- query observers that can read apertures or byte ranges under a budget;
- a streaming/blob reader seam if `BlobStore::get Arc<[u8]>` is insufficient;
- cached full-text readings treated as cache only, never canonical truth;
- inverse-fragment retention policy;
- explicit obstruction when a fine-grained inverse cannot be proven because
  history has been compressed, garbage-collected, or moved cold.

Wormholes are future history/provenance compression artifacts. They are not rope
chunks, portals, or editor state zoom.

## jedit Proof Fixture

The jedit contract should be an application-owned fixture and example. It
should define text nouns, rope-like layout, blob references, text windows,
braids, tick receipts, checkpoints, inverse policies, and obstruction reasons in
GraphQL. Echo should host the generated artifact without importing those nouns
into core.

The fixture should prove:

- generated mutation EINT reaches an installed contract handler;
- QueryView reaches a contract observer and returns QueryBytes;
- bounded text windows do not materialize the full file;
- create buffer, replace range, create braid, append braid edit, and unapply all
  use `dispatch_intent`;
- unapply appends inverse history and links receipts;
- unsupported or stale bases obstruct honestly;
- CAS retention controls inverse availability.

## RED/GREEN Sequence

1. RED: installed contract mutation handler behind `dispatch_intent`.
2. GREEN: minimal toy generated contract mutates via Echo scheduling and
   provenance.
3. RED: QueryView remains unsupported for generated query.
4. GREEN: contract observer registry and QueryBytes reading.
5. RED: bounded reading identity and residual posture.
6. GREEN: bounded contract observer support.
7. RED: intent-only external strand, braid, and settlement mutation.
8. GREEN: generic intent wrappers and tests.
9. RED: jedit fixture `unapplyTick` appends inverse tick.
10. GREEN: contract inverse hook with jedit example fixture.
11. RED/GREEN: CAS retention for inverse fragments and bounded text blobs.
12. Design/RED: wormhole/checkpoint retention policy preserving inverse
    semantics.

## Non-Goals

- Do not add jedit text types to Echo core.
- Do not add a special jedit ABI.
- Do not invent a second intent envelope before proving EINT v1 cannot work.
- Do not implement full production crypto before the admission posture RED.
- Do not trust caller-supplied footprint JSON for scheduling independence.
- Do not implement Graft automation in Echo core.
- Do not make cached materialized text canonical truth.
- Do not redefine wormholes as rope chunks, portals, or state zoom.

## Open Questions

- Should installed contract handlers live in `warp-core`, an Echo application
  crate, or a new contract-hosting crate that `warp-core` can depend on without
  importing application nouns?
- Does EINT v1 need an outer admission certificate before the first installed
  contract proof, or can the certificate remain host-side metadata for this
  slice?
- Which existing receipt structures should carry inverse-to-target links?
- How much of generic braid projection belongs in the current Strand and
  SettlementService model versus a new braid registry?
- What is the first streaming seam that preserves current `echo-cas`
  content-only policy?
