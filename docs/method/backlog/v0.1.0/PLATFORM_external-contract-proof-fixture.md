<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# External Contract Proof Fixture

Status: v0.1.0 release blocker.

Depends on:

- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [Contract reading identity and bounded payloads](./KERNEL_contract-reading-identity-and-bounded-payloads.md)
- [Contract retention and semantic lookup seams](./PLATFORM_contract-retention-and-semantic-lookup-seams.md)
- [Product-facing intent outcome API](./PLATFORM_product-facing-intent-outcome-api.md)
- external `jedit` contract/runtime work, if `jedit` supplies the serious
  consumer shape

## Why now

The generic contract-host path needs one serious external consumer-shaped
proof before Echo can claim `v0.1.0` buildability. `jedit` is the preferred
shape because it pressures bounded readings, retained evidence, conflicts, and
replay without letting Echo core import text-editor nouns.

## What it should look like

Use an application-owned Wesley contract fixture that proves this path:

```text
external contract
-> Wesley generated artifacts
-> Echo package install
-> generated intent submission
-> scheduler-owned execution
-> generated QueryView/Query reading
-> retained evidence
-> local replay proof
```

The fixture may use text-like operations and readings, but the nouns remain in
the external contract and generated payloads.

## Acceptance criteria

- The fixture includes at least one mutation.
- The fixture includes at least one `QueryView`/`Query` reading.
- The mutation and query use non-trivial vars.
- The reading evidence includes bounded basis, aperture, and budget identity.
- Receipt and reading evidence can be retained and inspected.
- At least one conflict, rejection, obstruction, or residual path is exercised.
- Local replay reproduces the fixture outcome.
- Echo core contains no `jedit`, text, rope, buffer, editor, or Graft APIs
  outside generated fixture payloads.
- The fixture may declare retained tick/receipt obligations, but application
  code does not create ticks or `TickReceipt` values.

## Non-goals

- Do not build the `jedit` product UI.
- Do not author the `jedit` product contract inside Echo.
- Do not make the fixture a privileged Echo ontology.
- Do not add a special `jedit` ABI.
- Do not implement Graft automation in Echo core.
