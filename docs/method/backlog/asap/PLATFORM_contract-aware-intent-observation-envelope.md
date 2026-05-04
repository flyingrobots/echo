<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract-Aware Intent And Observation Envelope

Status: active planned design and RED candidate.

Depends on:

- [Wesley compiled contract hosting doctrine](./PLATFORM_wesley-compiled-contract-hosting-doctrine.md)
- [Reading envelope family boundary](../up-next/PLATFORM_reading-envelope-family-boundary.md)
- [Observer plans and reading artifacts](./PLATFORM_observer-plan-reading-artifacts.md)

## Why now

Echo already exposes generic WASM calls such as `dispatch_intent(...)` and
`observe(...)`. To host Wesley-compiled contracts, those calls need envelopes
that name contract family, schema identity, intent or observer kind, basis,
payload, and posture without hard-coding application semantics.

## What it should look like

Add focused RED tests for generic contract envelopes.

Candidate intent envelope fields:

- contract family
- schema hash
- intent kind
- basis or frontier
- payload bytes or payload ref
- caller context
- rights posture
- budget posture

Candidate observation envelope fields:

- contract family
- schema hash
- observer kind
- basis or frontier
- aperture payload
- rights posture
- budget posture

## Acceptance criteria

- Unknown contract family is rejected deterministically.
- Unsupported schema hash is rejected deterministically.
- Malformed payload is rejected before contract execution.
- Valid fake envelope reaches a registered fake handler.
- Responses preserve stable contract and schema identity.
- No text, editor, Graft, or `jedit` noun enters the Echo core ABI.

## Non-goals

- Do not build the registry beyond the minimum fake needed for RED tests.
- Do not implement Wesley generation.
- Do not add dynamic loading.
- Do not add application payload types.
- Do not change `jedit`.
