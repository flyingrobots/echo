<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Contract Hosting Roadmap

Status: active sequencing card.

Echo should become a generic host for Wesley-compiled contract families. It
must not grow application-specific APIs for text editing, code intelligence,
debugging, or any other consumer domain.

## Doctrine

Echo is the deterministic witnessed causal substrate.

Wesley authors and compiles contract families from GraphQL into generated Rust,
ABI codecs, schema identity, and contract dispatch/read surfaces.

Applications such as `jedit` own their domain contracts and product behavior.
Echo hosts those contracts through generic intent and observation envelopes.

## Sequence

1. [Wesley compiled contract hosting doctrine](./PLATFORM_wesley-compiled-contract-hosting-doctrine.md)
    - Design packet:
      [0013 - Wesley Compiled Contract Hosting Doctrine](../../../design/0013-wesley-compiled-contract-hosting-doctrine/design.md)
2. [Contract-aware intent and observation envelope](./PLATFORM_contract-aware-intent-observation-envelope.md)
3. [Static contract registry and host boundary](./PLATFORM_static-contract-registry-and-host-boundary.md)
4. [Wesley to Echo toy contract proof](../up-next/PLATFORM_wesley-to-echo-toy-contract-proof.md)
5. [Contract-aware receipts and readings](../up-next/KERNEL_contract-aware-receipts-and-readings.md)
6. [Contract artifact retention in echo-cas](../up-next/PLATFORM_contract-artifact-retention-in-echo-cas.md)
7. [jedit text contract MVP](../up-next/PLATFORM_jedit-text-contract-mvp.md)
8. [Graft live frontier structural readings](../up-next/PLATFORM_graft-live-frontier-structural-readings.md)
9. [Contract strands and counterfactuals](../up-next/KERNEL_contract-strands-and-counterfactuals.md)
10. [Continuum contract artifact interchange](../cool-ideas/PLATFORM_continuum-contract-artifact-interchange.md)

## Non-goals

- Do not add `ReplaceRange`, `BufferWorldline`, or text-editing types to Echo
  core unless they are generated application contract payloads.
- Do not add a special `jedit` ABI.
- Do not let Graft mutate Echo state directly.
- Do not build dynamic plugin loading before static contract hosting works.
- Do not start IPA, proof systems, or network Continuum protocol work in this
  cluster.

## Done looks like

- Each item in the sequence has a narrow card with dependencies, acceptance
  criteria, and non-goals.
- Future agents can pick the next card without re-arguing whether Echo owns
  application-specific APIs.
