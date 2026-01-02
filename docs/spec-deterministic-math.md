<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Deterministic Math Module Specification (Phase 0)

Echo’s math module underpins every deterministic system: physics proxies, animation, AI, and branch reconciliation.

**Status (2026-01-02): legacy draft + partial reality.**
- This document started life as a JS/TypeScript-oriented Phase 0 draft.
- The canonical implementation today is Rust `warp-core` (`crates/warp-core/src/math/*`).
- The normative determinism policy is `docs/SPEC_DETERMINISTIC_MATH.md`.
- Validation and CI lanes are tracked in `docs/math-validation-plan.md`.

Treat this spec as a **design sketch for future bindings** (TS/WASM/FFI) and an inventory of desired API shape, not as a statement that the JS implementation exists.

---

## Goals
- Provide deterministic vector/matrix/quaternion operations across platforms (at minimum: Linux/macOS, and eventually WASM/JS bindings).
- Support dual numeric modes via scalar backends:
  - float lane (`F32Scalar`, default)
  - fixed-point lane (`DFix64`, feature-gated today)
- Expose seeded PRNG services suitable for replay and branching.
- Offer allocation-aware APIs (avoid heap churn) for hot loops.
- Surface profiling hooks (NaN guards, range checks) in development builds.

---

## Numeric Modes

### Float32 Mode (default)

- **Rust source of truth:** `F32Scalar` wraps `f32` and enforces canonicalization invariants (NaNs, signed zero, subnormals) at construction and after operations.
- **Transcendentals:** `sin`/`cos` are provided via a deterministic software backend (`warp_core::math::trig`), not platform/libm.
- **Bindings note:** if/when we ship TS/WASM bindings, they must match Rust’s outputs and invariants; “just `Math.fround`” is not sufficient to guarantee cross-engine determinism for transcendentals or NaN payload behavior.

### Fixed-Point Mode (opt-in)

- **Rust source of truth:** `DFix64` is Q32.32 fixed-point stored in `i64` and is currently feature-gated behind `det_fixed` so we can evolve it without destabilizing the default lane.
- **Non-finite mapping:** conversions from float inputs must be deterministic (e.g., NaN → 0, ±∞ saturate) and are covered by tests.
- **Bindings note:** future TS bindings should treat Rust fixtures as canonical; JS `BigInt` fixed-point is a possible implementation strategy, but not a correctness authority.

Mode should be chosen at engine init (or build feature selection), with a clear policy for serialization/hashing so deterministic replay remains stable.

---

## Core Types

### Vec2 / Vec3 / Vec4

```ts
interface Vec2 {
  readonly x: number;
  readonly y: number;
}

type VecLike = Float32Array | number[];
```
- Backed by `Float32Array` of length 2/3/4.
- Methods: `create`, `clone`, `set`, `add`, `sub`, `scale`, `dot`, `length`, `normalize`, `lerp`, `equals`.
- All mutating functions accept `out` parameter for in-place updates to reduce allocations.
- Deterministic clamps: every operation ends with `fround` (float mode) or `fixed` operations.
- Rust parity: `warp_core::math::Vec3` currently implements add/sub/scale/dot/cross/length/normalize; `Vec2`/`Vec4` remain TODO.

### Mat3 / Mat4

- Column-major storage (`Float32Array(9)` / `Float32Array(16)`).
- Methods: `identity`, `fromRotation`, `fromTranslation`, `multiply`, `invert`, `transformVec`.
- Deterministic inversion: use well-defined algorithm with guard against singular matrices (records failure and returns identity or throws based on config).
- Rust parity: `warp_core::math::Mat4` exposes `multiply` and `transform_point`; identity/fromRotation/invert are pending.

### Quat

- Represented as `[x, y, z, w]`.
- Functions: `identity`, `fromAxisAngle`, `multiply`, `slerp`, `normalize`, `toMat4`.
- `slerp` uses deterministic interpolation with clamped range.
- Rust parity: `warp_core::math::Quat` implements identity/fromAxisAngle/multiply/normalize/to_mat4; `slerp` remains TBD.

### Transform

- Struct bundling position (Vec3), rotation (Quat), scale (Vec3).
- Helper for constructing Mat4; ensures consistent order of operations.
- Rust parity: transform helpers are still tracked for Phase 1 (not implemented yet).

### Bounds / AABB

- Useful for physics collision; stores min/max Vec3.
- Provides deterministic union/intersection operations.

---

## PRNG Services

### Engine PRNG

- Based on counter-based generator (e.g., Philox or Xoroshiro128+).
- Implementation in TypeScript with optional WebAssembly acceleration later.
- Interface:
```ts
interface PRNG {
  next(): number;               // returns float in [0,1)
  nextInt(min: number, max: number): number;
  nextFloat(min: number, max: number): number;
  state(): PRNGState;
  jump(): PRNG;                 // independent stream
}
```
- `state` serializable for replay.
- `jump` used for branch forking: clone generator with deterministic offset.
- `seed` derived from combination of world seed + branch ID + optional subsystem tag.
- Rust parity: `warp_core::math::Prng` implements seeding, `next_f32`, and `next_int`; state/jump APIs are follow-up work.

### Deterministic Hashing

- Provide `hash64` function (e.g., SplitMix64) for converting strings/IDs into seeds.
- Ensure stable across platforms; implement in TypeScript to avoid native differences.

### Integration Points

- Scheduler passes `math.prng` on `TickContext`.
- Codex’s Baby `CommandContext` exposes `prng.spawn(scope)` for per-handler streams.
- Timeline branch creation clones PRNG state to maintain deterministic divergence.

---

## Utility Functions

- `clamp(value, min, max)` – deterministic clamp using `Math.min/Math.max` once (avoid multiple rounding).
- `approximatelyEqual(a, b, epsilon)` – uses configured epsilon (float32 ~1e-6).
- `degToRad`, `radToDeg` – using float32 rounding.
- `wrapAngle(angle)` – ensure deterministic wrap [-π, π].
- `bezier`, `catmullRom` – deterministic interpolation functions for animation.

---

## Memory Strategy
- Provide pool of reusable vectors/matrices for temporary calculations (`MathStack`).
- `MathStack` uses deterministic LIFO behavior: `pushVec3()`, `pushMat4()`, `pop()`.
- Guard misuse in dev builds (stack underflow/overflow assertions).

---

## Diagnostics
- Optional `math.enableDeterminismChecks()` toggles NaN/Infinity detection; throws descriptive error with stack trace.
- `math.traceEnabled` allows capturing sequence of operations for debugging (recorded in inspector overlay).
- Stats counters: operations per frame, PRNG usage frequency.

---

## API Surface (draft)
```ts
interface EchoMath {
  mode: "float32" | "fixed32";
  vec2: Vec2Module;
  vec3: Vec3Module;
  vec4: Vec4Module;
  mat3: Mat3Module;
  mat4: Mat4Module;
  quat: QuatModule;
  transform: TransformModule;
  prng: PRNGFactory;
  stack: MathStack;
  constants: {
    epsilon: number;
    tau: number;
  };
  utils: {
    clamp(value: number, min: number, max: number): number;
    approx(a: number, b: number, epsilon?: number): boolean;
    degToRad(deg: number): number;
    radToDeg(rad: number): number;
  };
}
```

`PRNGFactory`:
```ts
interface PRNGFactory {
  create(seed: PRNGSeed): PRNG;
  fromTimeline(fingerprint: TimelineFingerprint, scope?: string): PRNG;
}
```

---

## Determinism Notes
- Avoid `Math.random`; all randomness flows through PRNG.
- `Math.sin/cos` may vary across engines; implement polynomial approximations or wrap to enforce float32 rounding (test across browsers).
- Fixed-point mode may skip trig functions initially; provide lookup tables or polynomial approximations.
- Ensure order of operations consistent; avoid relying on JS evaluation order quirks.

---

## Open Questions
- Should fixed-point mode support quaternions (costly) or restrict to 2D contexts?
- How to expose SIMD acceleration where available without breaking determinism (e.g., WebAssembly fallback).
- Do we allow user-defined math extensions (custom vector sizes) via plugin system?
- Integration with physics adapters: how to synchronize with Box2D/Rapier numeric expectations (float32).

Future work: add unit tests validating cross-environment determinism, micro-benchmarks for operations, and sample usage in the playground.
