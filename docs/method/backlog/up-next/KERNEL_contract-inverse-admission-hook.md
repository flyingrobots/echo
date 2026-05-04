<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Inverse Admission Hook

Status: planned kernel/runtime implementation.

Depends on:

- [Installed Wesley contract host dispatch](../asap/PLATFORM_installed-wesley-contract-host-dispatch.md)
- [Contract reading identity and bounded payloads](./KERNEL_contract-reading-identity-and-bounded-payloads.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

Undo must be witnessed history. A contract knows how to invert its own domain
semantics; Echo should provide a generic admission hook, not a generic blind
`WarpOp` inverse API.

## What it should look like

Given:

- target tick, receipt, or range;
- current target coordinate;
- contract family;
- inverse policy;
- installed contract artifact;

Echo asks the contract to produce one or more inverse intents or a typed
obstruction. Produced inverse intents are admitted normally.

## Acceptance criteria

- Building `hello` from five insert ticks then unapplying C2 appends one inverse
  tick.
- Reading at the new frontier is `helo`.
- Original C2 remains in provenance.
- Provenance length increases by one.
- Inverse receipt links to the target receipt.
- Unmappable spans, missing receipts, unavailable inverse fragments, and
  contract-version mismatches return typed obstructions.
- Sequence unapply applies deterministic ordering or reports partial success
  explicitly.

## Non-goals

- Do not delete or rewrite old ticks.
- Do not expose `WarpOp` replay patches as user-facing undo semantics.
- Do not silently succeed when inverse evidence is missing.
