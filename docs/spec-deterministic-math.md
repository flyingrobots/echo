# Deterministic Math Module Specification (Phase 0)

Echo’s math module underpins every deterministic system: physics proxies, animation, AI, and branch reconciliation. This spec defines the numeric modes, core types, API surface, and PRNG strategy.

---

## Goals
- Provide deterministic vector/matrix/quaternion operations across platforms (browser, Node, native wrappers).
- Support dual numeric modes: float32 clamped and fixed-point (configurable).
- Expose seeded PRNG services that integrate with timeline branching and Codex’s Baby.
- Offer allocation-aware APIs (avoid heap churn) for hot loops.
- Surface profiling hooks (NaN guards, range checks) in development builds.

---

## Numeric Modes

### Float32 Mode (default)
- All operations clamp to IEEE 754 float32 using `Math.fround`.
- Inputs converted to float32 before computation; outputs stored in float32 buffers (`Float32Array`).
- Stable across JS engines as long as `Math.fround` available (polyfill for older runtimes).

### Fixed-Point Mode (opt-in)
- 32.32 fixed-point representation using BigInt internally, surfaced as wrapper `Fixed` type.
- Configured via engine options (`mathMode: "float32" | "fixed32"`).
- Useful for deterministic networking or hardware without stable float operations.
- Bridges through helper functions: `fixed.fromFloat`, `fixed.toFloat`, `fixed.mul`, `fixed.div`.

Mode chosen at engine init; math module provides factory returning mode-specific implementations. The Rust runtime already exposes the float32 primitives in `rmg_core::math`, so FFI/WASM adapters can reuse a single source of truth while TypeScript bindings converge on the same fixtures.

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
- Rust parity: `rmg_core::math::Vec3` currently implements add/sub/scale/dot/cross/length/normalize; `Vec2`/`Vec4` remain TODO.

### Mat3 / Mat4
- Column-major storage (`Float32Array(9)` / `Float32Array(16)`).
- Methods: `identity`, `fromRotation`, `fromTranslation`, `multiply`, `invert`, `transformVec`.
- Deterministic inversion: use well-defined algorithm with guard against singular matrices (records failure and returns identity or throws based on config).
- Rust parity: `rmg_core::math::Mat4` exposes `multiply` and `transform_point`; identity/fromRotation/invert are pending.

### Quat
- Represented as `[x, y, z, w]`.
- Functions: `identity`, `fromAxisAngle`, `multiply`, `slerp`, `normalize`, `toMat4`.
- `slerp` uses deterministic interpolation with clamped range.
- Rust parity: `rmg_core::math::Quat` implements identity/fromAxisAngle/multiply/normalize/to_mat4; `slerp` remains TBD.

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
- Rust parity: `rmg_core::math::Prng` implements seeding, `next_f32`, and `next_int`; state/jump APIs are follow-up work.

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
