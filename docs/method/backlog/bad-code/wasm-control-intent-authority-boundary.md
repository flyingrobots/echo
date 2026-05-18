<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WASM control intent authority boundary is too implicit

Status: active follow-up. The immediate bug was fixed by making public
application dispatch reject `CONTROL_INTENT_V1_OP_ID` before the kernel can
run scheduler control. `WarpKernel::dispatch_intent(...)` also rejects the
reserved control op id directly, while trusted runtime control now uses a
separate `TrustedKernelControlPort` Rust host/runtime-owner path.

The remaining concern is product packaging and host architecture: browser
adapters must not hand untrusted application code a raw privileged runtime
owner. A worker or host adapter should own Echo's scheduler lifecycle and expose
only application-safe intent ingress plus observation APIs.

Fix direction:

- Keep generated Wesley helpers unable to produce scheduler control envelopes.
- Add browser/package integration tests proving untrusted application adapters
  cannot reach trusted runtime control.
- Document the worker/runtime-owner shape before publishing a high-level
  browser application package.

Acceptance criteria:

- Browser package APIs expose application-safe dispatch and observation only.
- Host/runtime control can start Echo deterministically without exposing the raw
  control path to application code.
- README, WASM ABI docs, and package docs describe the authority split
  consistently.
