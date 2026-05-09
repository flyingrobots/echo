<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0015 - Registry Provider Host Boundary Decision

_Choose the first host boundary for Wesley-generated registries without
changing Echo's app-agnostic EINT ingress._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [0013 - Wesley Compiled Contract Hosting Doctrine](../0013-wesley-compiled-contract-hosting-doctrine/design.md)
- [0014 - EINT, Registry, And Observation Boundary Inventory](../0014-eint-registry-observation-boundary-inventory/design.md)

## Decision

For the first Wesley-to-Echo consumer proof, Echo should use app-level generated
validation before EINT dispatch.

That means the first consumer path is:

```text
generated app client / adapter
  -> generated REGISTRY lookup
  -> generated vars validation and encoding
  -> pack_intent_v1(generated_op_id, vars)
  -> dispatch_intent(...)
```

Echo's current `dispatch_intent(...)` boundary remains app-agnostic and
application-blind. Echo validates EINT shape and reserved control op use. It
does not validate generated operation payload semantics in core for this slice.

## Why this choice

The existing code already establishes two important constraints:

- `KernelPort` is explicitly app-agnostic over canonical intent bytes.
- `echo-registry-api::RegistryProvider` is application-supplied and generated
  from GraphQL / Wesley IR.

Those facts point to a conservative first cut: generated application code owns
domain validation, while Echo owns deterministic ingestion, admission,
observation, receipts, readings, and retention.

This is also the right fit for `jedit`: text editing behavior belongs in the
application contract and generated adapter, not in Echo core.

## Rejected for the first consumer

### Host-side app payload validation

Do not make `warp-wasm` reject unknown generated app op ids or malformed app
vars yet.

That may become correct once Echo supports installed generated contract
families, but it should be introduced by a future RED that proves one of these
needs:

- app-independent host validation before ingestion;
- runtime operation catalog export;
- multiple generated registries installed in one host;
- query op dispatch through an installed observer plan.

### New registry trait

Do not add a second registry abstraction. If host-side validation becomes
necessary, reuse `echo-registry-api::RegistryProvider` or document why it is
insufficient.

### New intent envelope

Do not add a new contract intent envelope for the first consumer. EINT v1
remains the wire shape.

## What Echo should expose now

The first host handshake should use existing metadata:

- `get_registry_info`
- `get_codec_id`
- `get_registry_version`
- `get_schema_sha256_hex`

For generated clients, this is enough to check that the app bundle and host
agree about schema identity, codec identity, registry version, and ABI version.

## What generated code should do now

Generated app code should:

- expose typed operation helpers;
- look up generated operation ids from generated constants or `REGISTRY`;
- validate operation shape before packing EINT;
- encode vars in the generated codec;
- call `pack_intent_v1(...)`;
- call `dispatch_intent(...)`;
- decode dispatch responses and observations into app-level result types.

The generated code may use `RegistryProvider` for self-checking, diagnostics,
or op lookup. It does not need Echo core to consult the provider for the first
consumer proof.

## Query and observation bridge

Generated `QUERY` operations remain the narrow unresolved bridge.

For the first proof, generated read helpers should prefer:

```text
generated read helper
  -> construct ObservationRequest
  -> observe(...)
  -> verify ReadingEnvelope posture
  -> decode app-level reading payload
```

If a generated query cannot be represented through the current
`ObservationRequest`, the next RED should prove that exact missing field or
observer-plan hook. It should not add a broad `query_contract(...)` ABI.

## Future host-side validation card

Host-side validation is deferred, not rejected forever.

When pulled, that future card should answer:

- how a generated `RegistryProvider` is installed into `warp-wasm` or the
  kernel;
- whether host-side validation rejects unknown op ids;
- whether host-side validation can validate vars bytes without importing
  app-specific domain types into Echo core;
- whether generated query ops map to observer plans or remain app-level
  helpers;
- whether registry catalog export is required for browser clients or devtools.

## Acceptance impact

The first implementation should now be the toy contract proof, not registry
plumbing. It should prove:

- generated registry output exists;
- generated app code validates and packs one mutation into EINT v1;
- Echo accepts that EINT through existing dispatch;
- one read helper can use existing observation/read-envelope surfaces or
  produce a narrow RED for the missing observation bridge.

## Non-goals

- No ABI changes.
- No new registry trait.
- No new intent envelope.
- No host-side app payload validation yet.
- No dynamic contract loading.
- No jedit-specific APIs.
- No production code in this design slice.
