<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Deterministic Math Validation Plan

Status: this document may lag behind the current Rust-first implementation.
Treat it as a checklist of *ideas*, not a CI contract.

If you’re looking for what we actually enforce today, start with:

- Policy (normative): [/SPEC_DETERMINISTIC_MATH](/SPEC_DETERMINISTIC_MATH)
- Claims / budgets: [/warp-math-claims](/warp-math-claims)

Goal: ensure `warp-core`’s deterministic math produces **bit-identical** results across platforms and build configurations, and that we catch regressions (especially in scalar canonicalization and transcendental approximations) in CI.

---

## Scope & Source of Truth

- **In-scope:** `crates/warp-core/src/math/*` and its public surfaces (`F32Scalar`, `DFix64`, `Vec3`, `Mat4`, `Quat`, `Prng`, deterministic trig backend, etc.).
- **Out-of-scope (for now):** JS runtime determinism (Chromium/WebKit/Node) and TypeScript bindings. Those are future layers; the canonical reference implementation is Rust `warp-core`.
- **Policy + invariants:** see `docs/SPEC_DETERMINISTIC_MATH.md` (normative policy) and `docs/DETERMINISTIC_MATH.md` (hazard catalog).

---

## Lanes (What We Validate)

Echo currently has two deterministic-math lanes:

| Lane | Build config | Target behavior |
| ---- | ------------ | --------------- |
| **Float lane** | default | `F32Scalar` + deterministic trig backend |
| **Fixed lane** | `--features det_fixed` | `DFix64` (Q32.32 fixed-point) |

Targets we actively care about (and already exercise in CI):
- Linux glibc (default lane)
- Linux musl (portability lane)
- macOS (spot-check lane)

---

## Validation Principles

**Determinism-first (preferred):**
- Use **exact** equality and bit-level checks whenever we can.
- Treat “epsilon” tests as a last resort, and isolate them behind explicit “budget” thresholds with a stable, deterministic oracle.

---

## What We Test Today (Reality Check)

This plan is considered “up to date” when these concrete checks exist and stay green:

### 1) Scalar canonicalization invariants

`F32Scalar` must enforce:
- `-0.0 → +0.0`
- NaNs canonicalized to the project’s chosen payload
- subnormals flushed to `+0.0`
- reflexive `Eq` (including `NaN == NaN`)

See tests:
- `crates/warp-core/tests/math_scalar_tests.rs`
- `crates/warp-core/tests/determinism_policy_tests.rs`
- `crates/warp-core/tests/nan_exhaustive_tests.rs`

### 2) Deterministic transcendental surface (sin/cos)

We validate two separate things:
- **Bit-level stability** (golden vectors): ensure outputs don’t change across platforms.
- **Approximation error** (budgeted audit): ensure the LUT-backed trig doesn’t drift beyond pinned error budgets.

See tests:
- `crates/warp-core/tests/deterministic_sin_cos_tests.rs`

Note: the “audit” flavor may be `#[ignore]` depending on whether it uses a deterministic oracle; run ignored tests explicitly when present.

### 3) Vector/matrix/quaternion behavior

We validate correctness and invariants for the math types that `warp-core` actually ships today:
- `Vec3` operations (dot/cross/normalize/etc.)
- `Mat4` rotation/multiply/transform behavior
- `Quat` multiplication/normalization/to-mat4 behavior

See tests:
- `crates/warp-core/tests/math_validation.rs`
- `crates/warp-core/tests/math_rotation_tests.rs`
- `crates/warp-core/tests/mat4_mul_tests.rs`

### 4) PRNG determinism

We validate the PRNG is stable and regression-tested with golden sequences:
- `crates/warp-core/tests/math_validation.rs`
- CI also runs a targeted golden regression (see `.github/workflows/ci.yml`).

### 5) Fixed-point lane correctness (`det_fixed`)

`DFix64` is feature-gated; its tests must be run under `--features det_fixed`.

See tests:
- `crates/warp-core/tests/dfix64_tests.rs`

---

## How To Run The Math Validation Locally

Baseline (float lane):

```sh
cargo test -p warp-core
```

Run the “math validation” suite explicitly:

```sh
cargo test -p warp-core --test math_validation
```

Run deterministic trig golden tests explicitly:

```sh
cargo test -p warp-core --test deterministic_sin_cos_tests
```

Run ignored tests (only when you intend to run audits):

```sh
cargo test -p warp-core --test deterministic_sin_cos_tests -- --ignored
```

Fixed-point lane:

```sh
cargo test -p warp-core --features det_fixed
```

MUSL portability lane:

```sh
cargo test -p warp-core --target x86_64-unknown-linux-musl
cargo test -p warp-core --features det_fixed --target x86_64-unknown-linux-musl
```

---

## CI Coverage (Where It Runs)

- See `.github/workflows/ci.yml` for current lanes.
- CI intentionally runs “boring” commands that contributors can reproduce locally.

---

## Guards (Non-test Determinism Enforcement)

In addition to tests, we also enforce “no raw platform trig” via a repo guard script:
- `scripts/check_no_raw_trig.sh`

---

## Future Work (Optional / Not Yet Implemented)

- Cross-runtime determinism tests for JS (Chromium/WebKit) once TS/WASM bindings are in scope.
- A `warp-cli` command to run math diagnostics and report pinned budgets (useful for designers and CI triage).
- Additional scalar backends (e.g., a deterministic `libm`-based float lane, or tighter fixed-point trig).
