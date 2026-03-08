<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Cargo Feature Flags

Generated from `Cargo.toml` files as of 2026-03-07. Run
`grep -r '^\[features\]' crates/*/Cargo.toml` to verify.

> **Source of truth:** Crate `Cargo.toml` manifests. This page is a curated
> snapshot — check individual crates for the latest flags.

This document lists all Cargo feature flags across the Echo workspace. For
runtime configuration, see [configuration-reference.md](configuration-reference.md).

## warp-core

The simulation engine. Most flags live here.

| Feature                     | Default | Description                                                                                                                                                     |
| --------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `det_float`                 | no      | CI lane marker: float32-backed scalar backend (`F32Scalar`). Does not change behavior; used to tag test runs.                                                   |
| `det_fixed`                 | no      | CI lane marker: fixed-point Q32.32 backend (`DFix64`). Experimental, test-only.                                                                                 |
| `delta_validate`            | no      | Enables extra validation for delta operations during rewrite execution.                                                                                         |
| `golden_prng`               | no      | Regression check for PRNG sequences. Off by default to avoid freezing algorithm choices. Test-only.                                                             |
| `trig_audit_print`          | no      | Extra diagnostic printing in deterministic trig tests. Test-only.                                                                                               |
| `footprint_enforce_release` | no      | Enforce footprint independence checks in release builds (normally debug-only). Use in CI/staging. Mutually exclusive with `unsafe_graph`.                       |
| `unsafe_graph`              | no      | **DANGER:** Disables footprint enforcement entirely, even in debug. Removes all determinism safety checks. Mutually exclusive with `footprint_enforce_release`. |
| `serde`                     | no      | Serde support for serializable wrappers. **Only use with deterministic CBOR.** JSON is non-deterministic and banned in warp-core.                               |

### Scalar Backend Lanes

The `det_float` feature is a **CI orchestration marker** — it tags a test lane
but does not change runtime behavior. The `det_fixed` feature, however, **is a
behavioral switch**: it selects the fixed-point Q32.32 math backend (`DFix64`),
which genuinely changes runtime arithmetic. Both allow the CI matrix to run
separate test lanes for each scalar backend:

```bash
# Float lane (default behavior)
cargo test -p warp-core --features det_float

# Fixed-point lane (experimental)
cargo test -p warp-core --features det_fixed
```

### Footprint Safety Levels

Footprint enforcement is a determinism safety mechanism that validates rewrite
independence during scheduling. Three modes exist:

| Mode               | When                            | Flag                        |
| ------------------ | ------------------------------- | --------------------------- |
| Enforced (debug)   | Default in debug builds         | (none needed)               |
| Enforced (release) | CI/staging release builds       | `footprint_enforce_release` |
| Disabled           | Proven hot-path exemptions only | `unsafe_graph`              |

Enabling both `footprint_enforce_release` and `unsafe_graph` is a compile error.

## echo-dind-tests

| Feature    | Default | Description                                                        |
| ---------- | ------- | ------------------------------------------------------------------ |
| `dind_ops` | no      | Enables test-only ops (e.g., `put_kv`) for DIND convergence tests. |

## echo-dry-tests

| Feature     | Default | Description                                                 |
| ----------- | ------- | ----------------------------------------------------------- |
| `det_float` | no      | Passthrough to `warp-core/det_float` for CI lane selection. |
| `det_fixed` | no      | Passthrough to `warp-core/det_fixed` for CI lane selection. |

## echo-wasm-abi

| Feature | Default | Description                                                                 |
| ------- | ------- | --------------------------------------------------------------------------- |
| `std`   | **yes** | Standard library support (enables `serde/std`, `ciborium/std`, `half/std`). |
| `alloc` | no      | Alloc-only mode for `no_std` environments.                                  |

## echo-registry-api

| Feature | Default | Description               |
| ------- | ------- | ------------------------- |
| `std`   | **yes** | Standard library support. |

## echo-scene-port

| Feature | Default | Description               |
| ------- | ------- | ------------------------- |
| `std`   | **yes** | Standard library support. |

## echo-scene-codec

| Feature      | Default | Description                                                               |
| ------------ | ------- | ------------------------------------------------------------------------- |
| `std`        | **yes** | Standard library support (enables `echo-scene-port/std`, `minicbor/std`). |
| `test-utils` | no      | Test utilities for codec testing.                                         |

## warp-wasm

| Feature         | Default | Description                                                                  |
| --------------- | ------- | ---------------------------------------------------------------------------- |
| `console-panic` | no      | Routes panic messages to the browser console via `console_error_panic_hook`. |

## ttd-browser

| Feature         | Default | Description                                                                  |
| --------------- | ------- | ---------------------------------------------------------------------------- |
| `console-panic` | no      | Routes panic messages to the browser console via `console_error_panic_hook`. |

## echo-wasm-bindings

| Feature | Default | Description                                              |
| ------- | ------- | -------------------------------------------------------- |
| `wasm`  | no      | Enables `wasm-bindgen` support for WebAssembly bindings. |

## spec-000-rewrite

| Feature | Default | Description                                       |
| ------- | ------- | ------------------------------------------------- |
| `wasm`  | no      | Enables `wasm-bindgen` support for the spec demo. |

## See Also

- [configuration-reference.md](configuration-reference.md) -- runtime configuration
- [../SPEC_DETERMINISTIC_MATH.md](../SPEC_DETERMINISTIC_MATH.md) -- deterministic math policy
- [start-here.md](start-here.md) -- getting started guide
