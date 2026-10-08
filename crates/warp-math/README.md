<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# warp-math

`warp-math` contains Echo's deterministic math primitives: scalar wrappers,
fixed-point conversion helpers, vectors, matrices, quaternions, deterministic
trig, and timeline-friendly pseudo-random numbers.

Signed Q32.32 arithmetic and conversion algorithms come from the exact
`bunny-num` 0.6.0 dependency shared with Edict. New canonical callers use
`fixed_q32_32::FixedQ32_32` checked methods. The `det_fixed` `DFix64` adapter and
existing float conversion helpers retain their saturating compatibility policy;
motion payload wire bytes stay unchanged. The legacy WASM codec's truncating
helper and Echo's f32 trig path remain explicit compatibility boundaries.
See the [normative policy](../../docs/determinism/SPEC_DETERMINISTIC_MATH.md).

This crate is intentionally small. Code that only needs math should depend on
`warp-math` directly instead of pulling in `warp-core`. `warp-core` re-exports
the same surface at `warp_core::math::*` for compatibility with existing engine
callers.
