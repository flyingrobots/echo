<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley To Echo Toy Contract Proof

Status: planned cross-repo proof.

Depends on:

- [Static contract registry and host boundary](../asap/PLATFORM_static-contract-registry-and-host-boundary.md)
- [echo-wesley-gen v2 Update](./PLATFORM_echo-wesley-gen-v2.md)
- [WESLEY Protocol Consumer Cutover](../asap/PLATFORM_WESLEY_protocol-consumer-cutover.md)

## Why now

Before `jedit` becomes the first serious application consumer, Echo and Wesley
need one tiny contract that proves the full authoring and hosting path.

This should be deliberately boring. The value is the path:

```text
GraphQL -> Wesley IR -> generated Rust -> Echo registry -> dispatch -> observe
```

## What it should look like

Use a tiny toy contract, such as a counter, with one intent and one observer.

Example domain:

- `Increment`
- `CounterValue`

The exact schema is not important. The proof must exercise generated identity,
codecs, dispatch, receipt creation, and observation.

## Acceptance criteria

- Wesley compiles the toy GraphQL contract to Echo-consumable Rust artifacts.
- Echo statically registers the generated contract.
- `dispatch_intent(...)` admits one valid toy intent.
- `observe(...)` returns one contract-defined reading payload.
- The receipt names contract family, schema hash, intent kind, basis, and
  payload hash.
- The reading envelope names contract family, schema hash, observer kind, and
  basis.
- Golden ABI vectors are stable.

## Non-goals

- Do not use a text editing contract.
- Do not add dynamic contract loading.
- Do not require browser packaging.
- Do not add Continuum transport.
