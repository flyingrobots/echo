<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Static Contract Registry And Host Boundary

Status: active planned implementation.

Depends on:

- [Contract-aware intent and observation envelope](./PLATFORM_contract-aware-intent-observation-envelope.md)

## Why now

Echo needs a boring runtime boundary where generated contract families can be
registered and resolved. The first implementation should be static and
in-process. Dynamic plugin loading, network distribution, and external contract
installation are later problems.

## What it should look like

Introduce a minimal registry and handler boundary for generated contracts.

Candidate responsibilities:

- register a contract descriptor and handler
- resolve by contract family and schema hash
- route intent kinds to generated dispatch logic
- route observer kinds to generated read logic
- report unsupported contract, schema, intent, or observer deterministically

Candidate descriptor fields:

- family id
- schema hash
- registry version
- codec id
- intent kind catalog
- observer kind catalog
- Wesley generator version or generated bundle identity

## Acceptance criteria

- A statically registered fake contract dispatches through the generic
  envelope path.
- Unknown intent kind fails closed.
- Unknown observer kind fails closed.
- Registry resolution is deterministic.
- Contract handler errors return typed admission or observation posture.
- No dynamic loader or consumer-specific code is introduced.

## Non-goals

- Do not implement WASM dynamic module loading.
- Do not fetch contracts over the network.
- Do not add jedit-specific registration.
- Do not make the handler trait a broad runtime facade.
