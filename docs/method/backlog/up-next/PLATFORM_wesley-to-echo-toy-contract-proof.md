<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley To Echo Toy Contract Proof

Status: RED.

Depends on:

- [Registry provider wiring and host boundary decision](../asap/PLATFORM_static-contract-registry-and-host-boundary.md)
- [0016 - Wesley To Echo Toy Contract Proof](../../../design/0016-wesley-to-echo-toy-contract-proof/design.md)
- [echo-wesley-gen v2 Update](./PLATFORM_echo-wesley-gen-v2.md)
- [WESLEY Protocol Consumer Cutover](../asap/PLATFORM_WESLEY_protocol-consumer-cutover.md)

## Why now

Before `jedit` becomes the first serious application consumer, Echo and Wesley
need one tiny contract that proves the full authoring and hosting path.

This should be deliberately boring. The value is the path:

```text
GraphQL -> Wesley IR -> echo-wesley-gen Rust -> EINT -> dispatch -> observe
```

This proof should reuse existing pieces: EINT v1, `dispatch_intent(...)`,
`RegistryInfo`, `echo-registry-api::RegistryProvider`, `GeneratedRegistry`, and
the current observation/read-envelope boundary.

## What it should look like

Use a tiny toy contract, such as a counter, with one intent and one observer.

Example domain:

- `Increment`
- `CounterValue`

The exact schema is not important. The proof must exercise generated identity,
op ids, vars encoding, EINT packing, dispatch, registry metadata, and one
read/observation path.

## Current RED

The current RED is documented in
[0016 - Wesley To Echo Toy Contract Proof](../../../design/0016-wesley-to-echo-toy-contract-proof/design.md).

`echo-wesley-gen` already emits op constants, `OPS`, `GeneratedRegistry`, and
`REGISTRY`. It does not yet emit the first-consumer app-level helper that
validates/encodes operation vars, packs EINT v1, and maps a generated query/read
helper to `observe(...)` / `ReadingEnvelope`.

## Acceptance criteria

- Wesley compiles the toy GraphQL contract to Echo-consumable Rust artifacts.
- `echo-wesley-gen` emits op ids, op catalog metadata, and a generated
  `RegistryProvider`.
- The consumer proof uses `pack_intent_v1(...)` with a generated op id and vars
  payload.
- `dispatch_intent(...)` admits one valid toy intent.
- Registry metadata from the installed kernel or app bundle matches the
  generated schema and codec metadata.
- One read path proves how generated query/observer operations relate to
  `observe(...)` and `ReadingEnvelope`.
- Golden ABI vectors are stable.

## Non-goals

- Do not use a text editing contract.
- Do not add dynamic contract loading.
- Do not require browser packaging.
- Do not add Continuum transport.
- Do not create a second registry or intent envelope.
