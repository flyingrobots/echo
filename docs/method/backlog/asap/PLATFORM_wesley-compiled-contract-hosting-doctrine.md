<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Compiled Contract Hosting Doctrine

Status: active planned design.

Depends on:

- [Echo Continuum Runtime And CAS Readings](../../../design/continuum-runtime-and-cas-readings.md)
- [0011 - Optic and observer runtime doctrine](../../../design/0011-optic-observer-runtime-doctrine/design.md)

Design packet:

- [0013 - Wesley Compiled Contract Hosting Doctrine](../../../design/0013-wesley-compiled-contract-hosting-doctrine/design.md)

## Why now

Echo is moving toward observer-relative readings, witnessed suffix admission,
and Continuum-compatible artifacts. The next architectural risk is accidentally
turning Echo into an application runtime with special APIs for the first serious
consumer.

The corrected model is:

```text
GraphQL contract -> Wesley generated Rust -> EINT / observe -> Echo substrate
```

Echo must host generated contract families generically, but the existing repo
already has major pieces of that path: EINT v1, `dispatch_intent(...)`,
`observe(...)`, `RegistryInfo`, `echo-registry-api::RegistryProvider`, and
`echo-wesley-gen` generated registry output. Domain behavior belongs to the
authored contract and consuming application.

## What it should look like

Write a design packet that defines Echo as a Wesley-compiled contract host.

The packet should name at least:

- EINT v1
- `RegistryInfo`
- `RegistryProvider`
- generated `REGISTRY`
- schema hash
- codec id
- registry version
- `IntentKind`
- `IntentPayload`
- `IntentBasis`
- `IntentAdmission`
- `ContractReceipt`
- `ContractWitnessRef`
- `ObservationRequest`
- `ReadingEnvelope`
- `ContractReading`

It should also map those nouns to the existing optic loop:

```text
slice -> lower -> witness -> retain
```

## Done looks like

- A design doc explains that contracts author domain optics while Echo hosts
  the generic optic lifecycle.
- The doc explicitly says Echo must not add text-editor APIs, Graft APIs, or
  consumer-specific ABI methods.
- The doc identifies the first implementation cards as existing-boundary
  inventory and registry-provider wiring decisions, not duplicate envelope or
  registry construction.
- The doc distinguishes built-in substrate/debug observers from
  contract-defined application observers.

## Non-goals

- Do not change production code.
- Do not update Wesley in this card.
- Do not define the `jedit` text contract here.
- Do not design dynamic plugin loading.
- Do not implement Continuum transport.
