<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Compiled Contract Hosting Doctrine

Status: design packet accepted; implementation advanced through contract-host
mutation helpers, query observer helpers, and the installed contract package
registry boundary. Current blocker is normal installed mutation dispatch through
witnessed ingress and scheduler-owned ticks.

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

## Current checkpoint

The design packet defines Echo as a Wesley-compiled contract host. Since this
card was created, Echo has also landed:

- EINT v1 application ingress through `dispatch_intent(...)`;
- generated application request helpers;
- scheduler-owned mutation host helper seams;
- `echo-wesley-gen --contract-host` mutation handler-rule helpers;
- core QueryView/Query observer routing;
- `echo-wesley-gen --contract-host` query observer helper constructors.

Echo now has one installed package/registry boundary that binds:

- EINT v1
- `RegistryInfo`
- `RegistryProvider`
- generated `REGISTRY`
- schema hash
- codec id
- registry version
- supported operation ids
- mutation handlers
- query observers
- authored observer plan identities
- contract package/version identity

before handlers or observers install into `Engine`. Direct
`native_rule_bootstrap` registration remains an internal fixture and
transitional engine-test path; it does not provide registry/package identity
guarantees.

The current WARP/Echo noun map lives in
`docs/design/warp-optic-implementation-map.md`. The current optic admission
checkpoint lives in `docs/design/optic-admission-ladder-checkpoint.md`.

The old optic loop remains useful doctrine:

```text
slice -> lower -> witness -> retain
```

## Done looks like

- The accepted design doc remains linked from this card.
- The current next implementation card is installed contract mutation dispatch
  through the normal witnessed intent and scheduler-owned tick path, not more
  Wesley generation or another doctrine packet.
- Echo still must not add text-editor APIs, Graft APIs, or consumer-specific
  ABI methods.
- Built-in substrate/debug observers remain distinct from contract-defined
  application observers.

## Non-goals

- Do not change production code.
- Do not update Wesley in this card.
- Do not define the `jedit` text contract here.
- Do not design dynamic plugin loading.
- Do not implement Continuum transport.
