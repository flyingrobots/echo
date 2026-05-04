<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Authenticated Wesley Intent Admission Posture

Status: proposed security hardening.

Depends on:

- [Wesley to Echo toy contract proof](./PLATFORM_wesley-to-echo-toy-contract-proof.md)
- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [0017 - Authenticated Wesley Intent Admission Posture](../../../design/0017-authenticated-wesley-intent-admission-posture/design.md)

## Why now

The toy contract proof showed that Wesley-generated helpers can produce
canonical EINT bytes and observation requests. That is a useful bridge, but it
is not a production admission boundary.

A Wesley intent should not become tick-admissible merely because its EINT bytes
parse. It must be tied to a verified contract artifact, a trustworthy footprint
authority, and an authenticated session or capability accepted by local policy.

## What it should look like

Start RED-only.

Prove the current gap before implementation:

- `dispatch_intent(...)` admits well-formed EINT without a contract artifact id.
- `dispatch_intent(...)` admits well-formed EINT without session or capability
  proof.
- Current ingress evidence cannot name artifact trust posture.
- Current ingress evidence cannot name authenticated admission posture.
- Current ingress evidence cannot bind replay protection.
- Current reading rights posture cannot express policy-authorized observer
  sessions beyond `KernelPublic`.

Then introduce the smallest posture model that can represent:

- registered Wesley contract artifact identity;
- artifact trust posture;
- authenticated intent submission identity;
- policy/session admission result;
- no caller-supplied footprint trust.

## Acceptance criteria

- A design or RED test names the current unauthenticated EINT admission gap.
- The chosen model separates artifact trust from intent/session trust.
- Artifact trust posture can represent local-dev, generated-test, CI, and later
  BLADE-certified ramps without treating them as equivalent.
- Intent admission identity binds op id, canonical vars bytes, target
  coordinate, artifact id, session or policy id, and replay evidence.
- The docs state that caller-supplied footprint claims are not trusted for tick
  scheduling or independence decisions.
- The docs state that Holmes, WATSON, Moriarty, and BLADE are later
  certification providers, not dependencies of this first slice.
- No production crypto is implemented before the posture and RED boundary are
  reviewed.

## Non-goals

- Do not implement WebAuthn, passkeys, TOTP, or transport encryption yet.
- Do not implement Holmes, WATSON, Moriarty, or BLADE.
- Do not add app-specific nouns to Echo core.
- Do not import application Rust types into Echo core.
- Do not replace EINT v1 without a specific RED.
- Do not make `dispatch_intent(...)` read ambient host auth state.
- Do not allow runtime-submitted footprint JSON to become a footprint authority.
