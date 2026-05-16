<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RE-031 Capability Grant Validation Admission Integration

Lane: bad-code.
Status: follow-up.

## Problem

`CapabilityGrantIntentGate` can now publish obstruction facts when recorded
grant material fails narrow identity coverage against a registered optic
artifact. That is correct refusal-first substrate work, but it is not a full
authority model.

The next authority slice must not let identity coverage drift into accepted
authority by implication.

## Required follow-up

- Define accepted grant material separately from submitted grant intent
  material.
- Keep `CapabilityGrantValidationObstructed` as refusal evidence, not an
  admission receipt.
- Add a real admission boundary before any successful `AdmissionTicket` exists.
- Add deterministic expiry semantics instead of opaque caller-supplied expiry
  posture.
- Decide where successful grant validation, admission tickets, and law witness
  bundles live.

## Non-goals

- no Continuum schema freeze;
- no quorum model;
- no scheduler integration;
- no execution path.
