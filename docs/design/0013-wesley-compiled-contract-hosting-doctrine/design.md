<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0013 - Wesley Compiled Contract Hosting Doctrine

_Define Echo as a generic host for Wesley-compiled contract families, not as an
application-specific runtime API._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [0006 - Echo Continuum alignment](../0006-echo-continuum-alignment/design.md)
- [0011 - Optic and observer runtime doctrine](../0011-optic-observer-runtime-doctrine/design.md)
- [Echo Continuum Runtime And CAS Readings](../continuum-runtime-and-cas-readings.md)
- [Echo Contract Hosting Roadmap](../../method/backlog/asap/PLATFORM_echo-contract-hosting-roadmap.md)
- [Wesley Compiled Contract Hosting Doctrine](../../method/backlog/asap/PLATFORM_wesley-compiled-contract-hosting-doctrine.md)

## Why this cycle exists

Echo is moving toward observer-relative readings, witnessed suffix admission,
contract-generated artifacts, and Continuum-compatible causal exchange. The
next architectural risk is letting the first serious consumer drag
application-specific APIs into Echo core.

That would be the wrong cut.

Echo should not grow privileged APIs for text editing, code intelligence,
debugging, games, simulations, or any other consuming domain. Echo should host
Wesley-compiled contract families through generic intent and observation
surfaces.

The corrected model is:

```text
GraphQL contract -> Wesley generated Rust -> Echo contract host
```

The contract authors domain meaning. Wesley compiles that meaning into Rust,
ABI codecs, schema identity, and generated dispatch or observation surfaces.
Echo hosts the resulting family as a deterministic witnessed causal substrate.

## Human users / jobs / hills

### Primary human users

- Echo maintainers defining the runtime substrate boundary.
- Wesley maintainers compiling authored contracts into Echo-consumable Rust.
- Application authors using Echo through generated contract clients.
- Reviewers preventing consumer-specific behavior from entering Echo core.

### Human jobs

1. Decide whether a proposed API belongs in Echo substrate, Wesley-generated
   contract output, or an application adapter.
2. Sequence the next implementation cards without re-arguing whether Echo owns
   text editing, Graft projections, or other app-specific domains.
3. Review future changes for contract identity, schema identity, basis,
   receipt, and reading honesty.

### Human hill

A human can classify a proposed Echo change as substrate, generated contract,
or application behavior without relying on chat context or folklore.

## Agent users / jobs / hills

### Primary agent users

- Agents implementing the contract-aware intent envelope.
- Agents implementing the static contract registry.
- Agents wiring Wesley-generated toy contracts into Echo.
- Agents reviewing future jedit or Graft integration work.

### Agent jobs

1. Read one design packet and determine whether a change violates the
   substrate/application boundary.
2. Generate RED tests for contract envelopes and registry behavior without
   inventing application-specific DTOs.
3. Keep optic doctrine attached to generic contract hosting rather than
   hand-written domain APIs.

### Agent hill

An agent can inspect a contract-hosting patch and programmatically determine
whether Echo core gained consumer-specific nouns that should instead be
Wesley-generated or application-owned.

## Doctrine

Echo is a deterministic witnessed causal substrate.

Wesley is the contract compiler.

GraphQL is the authored contract language for application/runtime families.

Applications own domain behavior and product semantics.

Echo owns:

- deterministic scheduling
- basis and frontier handling
- admission outcome algebra
- witnessed transition receipts
- observer-relative reading envelopes
- witness and retained artifact references
- `echo-cas` retention policy
- strand, braid, import, and suffix admission substrate
- generic ABI entrypoints such as `dispatch_intent(...)` and `observe(...)`

Contracts own:

- domain nouns
- domain payload types
- intent kinds
- observer or read kinds
- domain validation
- domain transition law
- domain emission law
- domain-specific reading payloads

Applications own:

- product workflows
- UI and interaction policy
- adapters around generated clients
- application-specific persistence and save/open behavior where applicable
- decisions about which contract operations to expose to users

## Bright-Line Rule

Echo core must not add consumer-specific APIs.

Examples of APIs that do not belong in Echo core:

- `ReplaceRange`
- `BufferWorldline`
- `SaveCheckpoint`
- `RenameSymbol`
- `DeadSymbols`
- `GraftProjection`
- `JeditBuffer`

Those names may appear in generated contract payloads, generated clients,
application adapters, tests for generated families, or documentation about
consumers. They should not become substrate-owned Echo runtime nouns.

The Echo-owned shape is generic:

```text
dispatch contract intent
observe contract reading
retain contract artifact
admit contract suffix
settle contract strand
```

## Contract Hosting Stack

The intended stack is:

```text
Application GraphQL contract
  -> Wesley IR
  -> Wesley generated Rust DTOs, codecs, schema identity, and handlers
  -> Echo static contract registry
  -> Echo contract-aware intent and observation envelopes
  -> Echo deterministic admission, receipt, reading, and retention substrate
  -> Application-generated client or adapter
```

The first implementation should use static in-process registration. Dynamic
loading, network contract distribution, and runtime installation are future
problems.

## Required Nouns

Future design and implementation cards should converge on these noun families.
Exact Rust names may differ, but the boundaries should stay visible.

### Contract Identity

- `ContractFamilyId`
- `ContractSchemaHash`
- `ContractVersion`
- `ContractCodecId`
- `GeneratedBundleIdentity`

These identify what generated family Echo is hosting and which schema/law the
payloads claim.

### Intent Dispatch

- `ContractIntentEnvelope`
- `IntentKind`
- `IntentPayload`
- `IntentBasis`
- `IntentAdmission`
- `ContractDispatchContext`
- `ContractDispatchResult`

These define the generic write-side path. Echo validates envelope shape,
resolves the registered family, calls generated contract law, and wraps the
outcome in Echo admission/receipt vocabulary.

### Observation

- `ContractObservationEnvelope`
- `ObserverKind`
- `ObservationBasis`
- `ObservationAperture`
- `ContractObservationContext`
- `ContractObservationResult`
- `ContractReading`

These define the generic read-side path. Echo resolves the generated observer
or read family and emits an Echo `ReadingEnvelope` around the
contract-specific payload.

### Witness And Retention

- `ContractReceipt`
- `ContractWitnessRef`
- `ContractReadingRef`
- `RetainedContractArtifact`
- `ContractArtifactCoordinate`

These define how contract execution and observation become durable,
inspectable, cacheable, and future-exportable without confusing cached
readings with substrate truth.

### Registry

- `ContractRegistry`
- `ContractDescriptor`
- `ContractHandler`

These define how generated contract families become available to Echo. The
first cut should be small, static, and deterministic.

## Optic Mapping

Wesley contracts and Echo optics are not competing models.

Contracts author domain optics. Echo hosts the generic optic lifecycle.

The shared runtime pattern remains:

```text
slice -> lower -> witness -> retain
```

For a contract intent:

```text
slice:
  resolve contract identity, schema identity, basis, and needed causal evidence

lower:
  run generated contract transition law under Echo admission constraints

witness:
  produce admission outcome, receipt identity, payload hash, and witness refs

retain:
  store receipt, payload, witness refs, and optional reading hints in echo-cas
```

For a contract observation:

```text
slice:
  resolve contract identity, schema identity, observer kind, basis, aperture,
  rights, and budget

lower:
  run generated observer or emission law

witness:
  produce reading identity, witness refs, residual posture, and payload hash

retain:
  optionally cache the reading artifact by honest contract/optic coordinate
```

For a contract strand or counterfactual:

```text
slice:
  contract basis, parent movement, and local divergence

lower:
  revalidate or compare claims under Echo strand and admission law

witness:
  report admitted, staged, plural, conflict, or obstruction evidence

retain:
  keep the strand shell, settlement artifact, and reading artifacts as needed
```

## Built-In Observers Versus Contract Observers

Echo may keep built-in substrate/debug observers for runtime inspection.

Examples:

- scheduler status
- commit boundary head
- commit boundary snapshot
- recorded truth channels
- receipt or provenance inspection
- neighborhood and settlement publication

Those observers are Echo-owned because they inspect Echo substrate.

Application observers should come from authored contracts and Wesley-generated
families. Echo should host them, not hand-author them in core.

## ABI Direction

Echo's public ABI should keep the generic shape:

```text
dispatch_intent(bytes) -> bytes
observe(bytes) -> bytes
scheduler_status() -> bytes
```

The bytes should increasingly be contract-aware envelopes, not ad hoc payloads.

The envelope should name:

- contract family
- schema hash
- intent or observer kind
- basis or frontier
- payload or aperture
- rights posture
- budget posture

The response should name:

- admission or observation posture
- receipt or reading identity
- witness refs
- retained artifact refs where available
- contract and schema identity

## Error And Outcome Posture

Echo should keep failure categories separate:

- malformed ABI payload
- unknown contract family
- unsupported schema hash
- unknown intent or observer kind
- malformed contract payload
- unresolved basis
- budget obstruction
- rights obstruction
- contract-level conflict
- contract-level plurality
- staged local admission
- admitted transition

These should not collapse into string status fields or generic errors.

## Human playback

1. A human reviews a proposed `ReplaceRange` API in Echo core.
2. This design identifies it as application contract behavior, not substrate.
3. The human redirects it to a `jedit` GraphQL contract compiled by Wesley.
4. Echo only receives the generated contract family through generic envelopes.

## Agent playback

1. An agent reads this packet and the contract-hosting backlog cards.
2. The agent creates RED tests for contract-aware envelopes.
3. The tests mention contract family, schema hash, intent kind, basis, and
   payload.
4. The tests do not introduce text, Graft, or `jedit` domain nouns into Echo
   core.

## Implementation outline

1. Implement contract-aware intent and observation envelope RED tests.
2. Add a minimal static contract registry with fake handlers.
3. Prove one Wesley-to-Echo toy contract path.
4. Extend receipts and readings to carry contract/schema identity.
5. Retain contract artifacts in `echo-cas` with honest semantic coordinates.
6. Let `jedit` define the first serious text editing contract outside Echo
   core.
7. Let Graft consume Echo/jedit frontiers and readings, not Echo internals.
8. Add generic contract strands and counterfactuals after the basic contract
   path is real.

## Tests to write first

- RED: unknown contract family is rejected deterministically.
- RED: unsupported schema hash is rejected deterministically.
- RED: malformed contract payload fails before generated handler execution.
- RED: valid fake contract intent reaches a registered fake handler.
- RED: valid fake contract observation returns a reading envelope that names
  contract family, schema hash, observer kind, and basis.
- RED: no consumer-specific domain types are required to exercise the contract
  envelope path.

## Risks / unknowns

- **Wesley IR shape is still moving.** Keep the first Echo registry static and
  fake-handler-friendly so it can be tested before full generated bundles are
  stable.
- **Contract handler shape could become a broad runtime facade.** Keep the
  boundary limited to dispatch and observe. Do not let it absorb scheduler,
  CAS, transport, or app runtime behavior.
- **ABI compatibility pressure could preserve old payload forms too long.**
  Treat this as a major boundary clarification. Compatibility shims must not
  become alternate truth paths.
- **Applications may want convenient domain clients.** Generate those clients
  or keep them in application adapters. Do not move them into Echo core.

## Postures

- **Accessibility:** This packet is architecture and backlog guidance only.
  It should remain understandable as a plain Markdown read with explicit
  boundary examples.
- **Localization:** Contract and runtime noun choices should remain stable and
  technical. Domain-specific user-facing terms belong to applications.
- **Agent inspectability:** The packet intentionally names forbidden
  consumer-specific Echo core nouns and the allowed generic substrate nouns so
  agents can check future patches mechanically.

## Non-goals

- Do not change production code in this design cycle.
- Do not update Wesley implementation in this design cycle.
- Do not define the `jedit` text contract here.
- Do not define Graft structural projection contracts here.
- Do not implement dynamic contract loading.
- Do not implement Continuum transport.
- Do not introduce IPA or proof-carrying execution.
- Do not add consumer-specific APIs to Echo core.
