<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WASM Trusted Runtime Host Control Boundary

Status: local WASM authority boundary implemented.

This packet records the low-level WASM host-control boundary needed by the
`v0.1.0` browser and JavaScript release work. It is generic Echo runtime
plumbing. It is not an application API and does not import application nouns
into `warp-core` or `warp-wasm`.

## Claim

The raw WASM package has two distinct authority surfaces:

```text
application-facing surface
-> dispatch_intent(...)
-> observe(...)
-> scheduler_status(...)

trusted host/runtime-owner surface
-> dispatch_control_intent_trusted(...)
```

Application dispatch still rejects reserved scheduler control envelopes. The
trusted host export accepts packed `ControlIntentV1` envelopes only through a
kernel installed as `TrustedKernelControlPort`.

Those control envelopes represent trusted runtime-control history. They are not
application/domain intents, and recording one does not itself create a tick or
`TickReceipt`.

## Why This Exists

The local Rust contract-host path already separates application submit/observe
authority from trusted runtime tick authority. Browser and JavaScript release
work needs the same split at the raw WASM package boundary.

Without this split, an external application witness has only two bad options:

- send scheduler `start` / `until_idle` control through app-facing intent
  dispatch, which Echo correctly rejects; or
- expose raw runtime owner power directly to application code.

The correct shape is a trusted host adapter that owns the raw control export and
hands application code only the app-safe surface.

## Implemented Surface

`warp-wasm` now provides:

- `install_kernel(Box<dyn KernelPort>)` for app-only installed kernels;
- `install_trusted_kernel(Box<dyn TrustedKernelControlPort>)` for trusted host
  kernels;
- `dispatch_intent(...)` / `dispatch_intent_cbor(...)`, which reject
  `CONTROL_INTENT_V1_OP_ID`;
- `dispatch_control_intent_trusted(...)` /
  `dispatch_control_intent_trusted_cbor(...)`, which decode packed
  `ControlIntentV1` and require a trusted installed kernel.

The raw bundler package exports `dispatch_control_intent_trusted(...)` because
host adapters need it. High-level application packages and facades must not
re-export that function to untrusted application code.

## Authority Boundary

```text
application code
-> app-safe package facade
-> dispatch_intent / observe / scheduler_status

trusted host adapter
-> raw WASM package
-> dispatch_control_intent_trusted
-> trusted runtime-owned scheduler control
```

The trusted export does not make application dispatch synchronous. It only gives
the host/runtime owner a legal way to run Echo's scheduler lifecycle after
application ingress has been submitted.

## Evidence

- `public_dispatch_rejects_packed_control_intent_start`
- `public_dispatch_rejects_hand_built_reserved_control_op_id`
- `public_optic_dispatch_rejects_hand_built_reserved_control_op_id`
- `trusted_control_dispatches_packed_control_intent_start`
- `trusted_control_is_unavailable_for_app_only_installed_kernel`
- `trusted_control_rejects_non_control_intent_envelope`
- `scripts/tests/warp_wasm_package_exports_test.sh`

## Non-Goals

- No application-controlled ticks.
- No synchronous execution from app dispatch.
- No product-specific operation names.
- No high-level browser facade in this slice.
- No production sandbox claim.
- No wall-clock cadence semantics.
