<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# warp-math

`warp-math` contains Echo's deterministic math primitives: scalar wrappers,
fixed-point conversion helpers, vectors, matrices, quaternions, deterministic
trig, and timeline-friendly pseudo-random numbers.

This crate is intentionally small. Code that only needs math should depend on
`warp-math` directly instead of pulling in `warp-core`. `warp-core` re-exports
the same surface at `warp_core::math::*` for compatibility with existing engine
callers.
