<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0014 - EINT, Registry, And Observation Boundary Inventory

_Inventory the existing Echo intent, registry, and observation substrate before
adding Wesley-generated contract hosting behavior._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [0013 - Wesley Compiled Contract Hosting Doctrine](../0013-wesley-compiled-contract-hosting-doctrine/design.md)

## Why this packet exists

Echo already has enough generic substrate to accept app-authored intent bytes
and expose observation and registry metadata. The immediate risk is not missing
machinery. The immediate risk is planning a second machinery path because the
existing one was not named precisely.

This packet records the current boundary so the next Wesley-to-Echo consumer
proof starts from repo truth:

```text
Wesley-generated app code
  -> validate generated op and vars
  -> pack EINT v1 bytes
  -> dispatch_intent(...)
  -> observe(...) / app-level read helper
```

The packet is intentionally conservative. It does not claim that this is the
final host architecture for every future runtime. It defines the narrow first
path that avoids duplicate envelopes, duplicate registries, and accidental
app-specific APIs in Echo core.

## Existing boundary inventory

### EINT v1

`echo-wasm-abi` already defines EINT v1:

```text
"EINT" || op_id:u32le || vars_len:u32le || vars
```

Important properties:

- `pack_intent_v1(op_id, vars)` packs application intents.
- `unpack_intent_v1(bytes)` parses application-blind intent bytes.
- `CONTROL_INTENT_V1_OP_ID` is reserved for privileged control intents.
- application intents cannot use the reserved control op id.

This gives Echo a generic write ingress without knowing application nouns.

### WASM dispatch

`warp-wasm` already exports:

```text
dispatch_intent(intent_bytes)
```

The installed kernel parses EINT, classifies the reserved control op id, and
otherwise ingests application EINT bytes as canonical intent material.

### Kernel port

`KernelPort::dispatch_intent(...)` already defines the app-agnostic byte
boundary. Its docs explicitly say the boundary makes no assumptions about
installed schema, rules, or domain.

This is important. A first jedit or toy contract proof should not require Echo
core to gain `ReplaceRange`, `CounterIncrement`, or other application nouns.

### Registry metadata

`echo-wasm-abi` already has `RegistryInfo` with:

- `codec_id`
- `registry_version`
- `schema_sha256_hex`
- `abi_version`

`warp-wasm` already exports:

- `get_registry_info`
- `get_codec_id`
- `get_registry_version`
- `get_schema_sha256_hex`

That is enough for a first handshake that says which generated app bundle and
codec a host believes it is serving.

### Registry provider

`echo-registry-api` already defines `RegistryProvider`.

It is explicitly application-supplied and generated from GraphQL / Wesley IR.
It can expose:

- registry metadata
- operation lookup by id
- deterministic operation catalog iteration
- enum descriptors
- object descriptors

`echo-wesley-gen` already emits generated op ids, `OPS`, `op_by_id`,
`GeneratedRegistry`, and static `REGISTRY`.

The open question is therefore wiring, not ontology.

### Observation

`KernelPort::observe(...)` already exists as the canonical world-state read
entrypoint. It accepts `ObservationRequest` and returns `ObservationArtifact`
with `ReadingEnvelope` metadata.

Generated query or read operations should be mapped onto this existing read
surface where possible. They should not create a parallel `query_contract`
ABI until a concrete consumer proof shows that `observe(...)` cannot honestly
carry the reading.

### echo-cas

`echo-cas` already stores opaque blobs by content hash. CAS hashes are
content-only. Semantic identity, domain separation, reading scope, and contract
meaning belong in typed refs and coordinates above CAS.

That means contract artifact retention should add typed semantic keys and
retention refs above CAS, not change the CAS hash law.

## Decision 1: Do not add a second intent envelope

EINT v1 remains the contract path for the first consumer proof.

The first toy contract and first jedit-shaped consumer should pack generated
operation ids and encoded vars into EINT v1. They should not introduce a new
`ContractIntentEnvelope`.

EINT v2 is deferred until one of these is concretely required:

- multiple generated registries installed in one host at the same time;
- registry scope must be encoded per intent rather than negotiated at
  handshake time;
- a consumer needs domain-separated intent bytes that EINT v1 cannot name
  without ambiguity.

## Decision 2: First validation is app-level generated validation

For the first consumer proof, generated application code should validate op ids
and vars before calling `dispatch_intent(...)`.

That means:

- generated code consults generated `REGISTRY`;
- generated code packs canonical EINT bytes;
- Echo ingests application EINT bytes opaquely;
- Echo still validates EINT shape and reserved control op usage;
- Echo does not validate domain payload shape in core.

This decision preserves the current app-agnostic `KernelPort` boundary and
matches the product direction that jedit owns text behavior in app-level code.

Host-side registry validation remains a valid later option, but it should be a
separate implementation decision with RED tests proving what must be rejected
at the Echo boundary.

## Decision 3: RegistryInfo is enough for first handshake

For the first consumer proof, `RegistryInfo` is enough to prove that the host
and generated app bundle agree on:

- ABI version;
- codec id;
- registry version;
- schema hash.

The full op catalog may still be useful for browser UI, devtools, diagnostics,
or generated-client self-checks, but it is not required as a new Echo ABI export
before the toy proof.

If browser-side jedit later needs runtime operation discovery, add that as an
explicit registry-catalog export card. Do not smuggle it into dispatch.

## Decision 4: Generated query/read ops need a narrow bridge

The first missing bridge is not write ingress. It is generated read semantics.

Wesley-generated `QUERY` operations need one of these shapes:

1. application-level generated helper constructs an `ObservationRequest`, calls
   `observe(...)`, and decodes the returned reading payload;
2. generated query op remains app-local until Echo has a typed observer plan for
   that family;
3. later host-side registry wiring maps generated `QUERY` op ids to built-in or
   app-installed observer plans.

For the first consumer proof, prefer option 1 unless a RED proves that the
current `ObservationRequest` shape cannot name the required reading.

## First implementation implication

The next implementation card should not be "build contract envelopes."

The next implementation card should be the toy proof:

```text
GraphQL contract
  -> Wesley IR
  -> echo-wesley-gen generated Rust
  -> generated RegistryProvider
  -> pack_intent_v1(generated_op_id, encoded_vars)
  -> dispatch_intent(...)
  -> observe(...) for one read path
```

Before production changes, the RED should prove only the missing consumer
bridge. It should not fail because Echo lacks a generic intent envelope or
registry metadata.

## Rejected designs

### ContractIntentEnvelope v1

Rejected for the first consumer. EINT v1 already exists.

### ContractRegistry trait

Rejected for the first consumer. `echo-registry-api::RegistryProvider` already
exists and `echo-wesley-gen` already emits a provider.

### jedit-specific ABI

Rejected. jedit behavior belongs in the app contract and generated client or
adapter, not Echo core.

### Echo-side domain validation by default

Rejected for the first consumer. Echo should keep app payload validation outside
core unless a host-boundary RED proves that Echo must reject a malformed
generated payload before ingress.

## Open questions

- Should host-side registry validation become mandatory once Echo supports
  installed generated contract families?
- Does `RegistryInfo.registry_version` need to stay a string in
  `echo-wasm-abi` while `echo-registry-api::RegistryInfo.registry_version` is
  `u32`, or should a future compatibility card align the two surfaces?
- Should `warp-wasm` expose the full generated operation catalog for browser
  handshakes and devtools?
- Which `ObservationRequest` fields are sufficient for the first generated
  query/read helper?
- Should a future EINT v2 include registry scope, family id, or schema hash, or
  is handshake-scoped registry identity enough?

## Acceptance impact

This packet completes the inventory card and narrows the next two backlog
items:

- registry provider wiring should now focus on host-side validation and catalog
  export decisions;
- the toy proof should reuse EINT v1 and `RegistryProvider` rather than invent
  envelopes or registries.
