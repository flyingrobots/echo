<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# warp-wasm

wasm-bindgen bindings for Echo’s WASM ABI and registry API, targeting tooling and web environments.

See the repository root `README.md` for the full overview.

## What this crate does

- Wraps `echo-wasm-abi` and `echo-registry-api` in `wasm-bindgen` bindings so
  Echo’s deterministic wire protocol can be used from JavaScript/TypeScript in
  web-based tools and playgrounds.
- Exposes the current observation-first and intent-shaped control surface
  (`ABI_VERSION` 9 in `echo-wasm-abi`): `observe(...)` is the only public
  world-state read export, `scheduler_status()` is the read-only scheduler
  metadata export, and `dispatch_intent(...)` is the write/control ingress.
  The current ABI also publishes strand settlement comparison, planning,
  execution entrypoints, settlement basis evidence, overlap revalidation
  evidence, and read-side basis plus residual posture on observation artifacts.
- The engine-backed boundary uses logical clocks only:
  `WorldlineTick` is per-worldline append identity and `GlobalTick` is runtime
  cycle correlation metadata. No wall-clock time enters Echo internals.
- Public scheduler control is expressed as privileged control intents packed
  into the same EINT envelope format as domain intents. The canonical control
  surface includes `Start`, `Stop`, and `SetHeadEligibility`.
- Intended to power future browser-based visualizers and inspectors built on
  top of the same core engine as native tools.

## Package boundary

Echo owns the WASM package build ritual for downstream consumers. Build the
bundler package with:

```sh
scripts/build-warp-wasm-package.sh
```

The command refreshes `crates/warp-wasm/pkg` with:

```sh
wasm-pack build --target bundler --out-dir pkg --out-name rmg_wasm -- --features engine
```

The package export smoke test imports `crates/warp-wasm/pkg/rmg_wasm.js` and
asserts the byte ABI export surface:

```sh
node --test scripts/tests/warp_wasm_package_exports_test.mjs
```

Consumer-facing app code should not depend on Echo's internal default
worldline. Durable app integration should route through optic-owned basis
resolution; raw worldline coordinates remain substrate/debug evidence, not a
product contract.

## Documentation

- Echo runtime model: `docs/architecture/outline.md`.
- Current WASM ABI contract: `docs/spec/SPEC-0009-wasm-abi.md`.
- JS/CBOR encoding rules: `docs/spec/js-cbor-mapping.md`.
- WARP stream schema for retained browser/session protocol types:
  `docs/spec/warp-view-protocol.md`.
