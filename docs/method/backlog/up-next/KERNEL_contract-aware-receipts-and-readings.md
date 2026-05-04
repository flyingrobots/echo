<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract-Aware Receipts And Readings

Status: planned kernel hardening.

Depends on:

- [Wesley to Echo toy contract proof](./PLATFORM_wesley-to-echo-toy-contract-proof.md)
- [Reading envelope family boundary](./PLATFORM_reading-envelope-family-boundary.md)

## Why now

Echo already content-addresses EINT intent bytes and already emits
`ReadingEnvelope` metadata for built-in observations. A generated contract path
is not enough unless any additional receipt and reading identity claims are
honest about what they actually commit to.

Echo should not return global-sounding state hashes for contract-local reads.
Contract observations should emit reading identities and envelopes whose scope
is explicit.

## What it should look like

First inventory existing identity sources:

- EINT `intent_id`
- ingress id
- `RegistryInfo`
- generated op id
- payload bytes or payload hash
- `ObservationArtifact::artifact_hash`
- `ReadingEnvelope`

Only extend receipt and reading identity inputs where those existing identities
cannot honestly name the app contract artifact.

Candidate additional identity components:

- schema hash
- intent or observer kind
- basis or frontier
- payload hash
- generated handler or law version
- witness refs
- admission or residual posture

## Acceptance criteria

- The same EINT bytes over the same basis produce the same receipt identity.
- Changing schema hash changes receipt identity.
- Changing payload changes receipt identity.
- Generated observation reading identity includes schema, observer or query op,
  basis, and aperture when those concepts are part of the generated read path.
- Unsupported or stale basis returns typed obstruction rather than a fake
  reading.
- Existing built-in observation tests remain green.

## Non-goals

- Do not implement cryptographic proofs.
- Do not introduce IPA or commitment math.
- Do not require full Continuum export/import.
- Do not add consumer-specific receipt fields to Echo core.
- Do not duplicate `intent_id` or `ReadingEnvelope` if they already provide the
  honest identity needed for the first consumer.
