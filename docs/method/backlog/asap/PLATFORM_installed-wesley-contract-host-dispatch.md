<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Installed Wesley Contract Host Dispatch

Status: RED/GREEN implementation slice.

Depends on:

- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)
- [0017 - Authenticated Wesley Intent Admission Posture](../../../design/0017-authenticated-wesley-intent-admission-posture/design.md)
- [Wesley to Echo toy contract proof](../up-next/PLATFORM_wesley-to-echo-toy-contract-proof.md)

## Why now

Echo can accept EINT bytes, but it does not yet route a validated generated
contract operation to an installed contract handler inside the normal witnessed
admission, scheduling, and provenance path.

## RED

Add a failing test with a tiny generated or hand-rolled contract fixture:

- install one mutation op id and generated handler;
- submit generated EINT bytes through `dispatch_intent`;
- prove no direct test-only mutation service is called;
- assert worldline/provenance state changes only after scheduler execution.

## GREEN

Add the minimal generic installed-contract host seam needed to pass the test.

Candidate surface:

- installed contract registry;
- op-id lookup;
- generated vars decode;
- generic mutation handler trait;
- artifact/schema identity attached to receipt or ingress metadata.

## Acceptance criteria

- Unsupported op id obstructs or errors when contract-hosting validation is
  enabled.
- Handler execution is inside Echo admission/witness/provenance.
- Footprint authority is not accepted from caller-supplied JSON.
- Echo core does not import jedit or text-domain Rust types.

## Non-goals

- Do not implement dynamic plugin loading.
- Do not invent an EINT replacement before a RED proves EINT v1 insufficient.
- Do not implement production crypto in this card.
