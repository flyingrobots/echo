<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract-Aware Receipts And Readings

Status: planned kernel hardening.

Depends on:

- [Wesley to Echo toy contract proof](./PLATFORM_wesley-to-echo-toy-contract-proof.md)
- [Reading envelope family boundary](./PLATFORM_reading-envelope-family-boundary.md)

## Why now

A generated contract path is not enough unless receipts and readings are honest
about the contract family, schema, basis, payload, and witness material they
commit to.

Echo should not return global-sounding state hashes for contract-local reads.
Contract observations should emit reading identities and envelopes whose scope
is explicit.

## What it should look like

Extend receipt and reading identity inputs so contract execution can name:

- contract family
- schema hash
- intent or observer kind
- basis or frontier
- payload hash
- generated handler or law version
- witness refs
- admission or residual posture

## Acceptance criteria

- The same intent over the same basis produces the same receipt identity.
- Changing schema hash changes receipt identity.
- Changing payload changes receipt identity.
- Contract observation reading identity includes contract family, schema,
  observer kind, basis, and aperture.
- Unsupported or stale basis returns typed obstruction rather than a fake
  reading.
- Existing built-in observation tests remain green.

## Non-goals

- Do not implement cryptographic proofs.
- Do not introduce IPA or commitment math.
- Do not require full Continuum export/import.
- Do not add consumer-specific receipt fields to Echo core.
