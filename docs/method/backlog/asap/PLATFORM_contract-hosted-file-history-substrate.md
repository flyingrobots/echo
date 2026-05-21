<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract-Hosted File History Substrate

Status: active sequencing card.

Design packet:
[0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

Source request:
[request.md](../../../design/0018-contract-hosted-file-history-substrate/request.md)

## Why now

PR #326 established the first Echo/Wesley contract-hosting roadmap. The next
body of work makes that path real enough for a jedit-like contract without
turning Echo into a text editor substrate.

The core doctrine remains strict: Echo hosts generated application contracts
through generic deterministic surfaces. `jedit` supplies the proof fixture and
consumer shape.

## Sequence

1. [Installed Wesley contract host dispatch](./PLATFORM_installed-wesley-contract-host-dispatch.md)
    - Status: local package dispatch proof complete.
2. [Contract QueryView observer bridge](./PLATFORM_contract-queryview-observer-bridge.md)
    - Status: core bridge, generated helpers, package registry boundary, and
      local dispatch proof complete.
3. [Contract-aware receipts and readings](../up-next/KERNEL_contract-aware-receipts-and-readings.md)
4. [Contract reading identity and bounded payloads](../up-next/KERNEL_contract-reading-identity-and-bounded-payloads.md)
5. [Contract artifact retention in echo-cas](../up-next/PLATFORM_contract-artifact-retention-in-echo-cas.md)
6. [Contract retention and streaming seams](../up-next/PLATFORM_contract-retention-and-streaming-seams.md)
7. [jedit contract proof fixture](../up-next/PLATFORM_jedit-contract-proof-fixture.md)
8. [Intent-only contract runtime mutations](../up-next/KERNEL_intent-only-contract-runtime-mutations.md)
9. [Generic contract braid substrate](../up-next/KERNEL_generic-contract-braid-substrate.md)
10. [Contract inverse admission hook](../up-next/KERNEL_contract-inverse-admission-hook.md)

## Acceptance criteria

- The source request is archived in docs.
- The design packet captures doctrine, non-goals, missing substrate, and
  execution order.
- Each implementation slice has a narrow backlog card with RED/GREEN acceptance
  criteria.
- The sequence preserves the rule that application nouns stay out of Echo core.

## Non-goals

- Do not implement runtime code in this sequencing card.
- Do not add text, rope, editor, Graft, or jedit APIs to Echo core.
- Do not replace the existing Echo contract-hosting roadmap; extend it.
