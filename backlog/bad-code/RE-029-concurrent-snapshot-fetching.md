<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RE-029 — Enforce det_fixed by Default

Legend: [RE — Runtime Engine]

## Idea

The project currently supports both `det_fixed` (DFix64) and optimistic `F32Scalar` math profiles. While useful for local experimentation, allowing `f32` in core paths introduces a high risk of cross-platform determinism poisoning if an external crate uses standard floats.

Enforce the `det_fixed` feature by default in the main workspace and release profiles. Make `F32Scalar` an explicit, gated opt-in that triggers a compiler warning about "non-consensus path risk."

## Why

1. **Security**: Hardens the system against floating-point drift attacks.
2. **Predictability**: Ensures that all published artifacts satisfy the "0-ULP Inevitability" claim.
3. **Governance**: Aligns the implementation with the binary "Determinism is sacred" tenet.

## Effort

Small — update Cargo.toml default features and workspace-wide build scripts.
