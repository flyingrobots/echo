<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# MATH

Deterministic math and geometry.

## What it covers

IEEE 754 canonicalization, deterministic trig oracle, scalar types,
collision detection, broad phase, sweep-and-prune.

## What success looks like

Float operations produce identical results on every platform. NaN
payloads are canonicalized. Subnormals are flushed. Signed zero is
normalized. Geometry primitives are proven correct by property tests.

## How you know

- `SPEC_DETERMINISTIC_MATH.md` policy is enforced by CI.
- Golden vectors for trig and physics lock cross-platform output.
- Property tests (proptest) cover edge cases.
- `ban-nondeterminism.sh` catches prohibited APIs.
