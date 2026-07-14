<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# warp-wasm

wasm-bindgen bindings for Echo’s WASM ABI and registry API, targeting tooling and web environments.

See the repository root `README.md` for the full overview.

## What this crate does

- Wraps `echo-wasm-abi` and `echo-registry-api` in `wasm-bindgen` bindings so
  Echo’s deterministic wire protocol can be used from JavaScript/TypeScript in
  web-based tools and playgrounds.
- Exposes the current intent-shaped and optic-shaped surface (`ABI_VERSION` 12
  in `echo-wasm-abi`). `observe_optic(...)` carries the product read shape and
  returns a bounded reading or typed obstruction, but the current engine does
  not verify its caller-supplied capability/law identifiers and obstructs
  generated `QueryBytes` apertures. Installed-contract queries therefore still
  use lower-level raw observation. `dispatch_intent(...)` and
  `dispatch_optic_intent(...)` provide untrusted ingress/proposal paths, while
  `scheduler_status()` exposes read-only scheduler metadata. Neighborhood and
  settlement exports also remain lower-level ABI surfaces.
- The engine-backed boundary uses logical clocks only:
  `WorldlineTick` is per-worldline append identity and `GlobalTick` is runtime
  cycle correlation metadata. No wall-clock time enters Echo internals.
- Public application dispatch rejects privileged scheduler control envelopes.
  Trusted runtime control is kept on a separate host/runtime-owner export,
  `dispatch_control_intent_trusted(...)`, and a separate Rust
  `TrustedKernelControlPort` installation path. Browser adapters must not hand
  the raw kernel/control surface to untrusted application code.
- Intended to power future browser-based visualizers and inspectors built on
  top of the same core engine as native tools.

## Package boundary

Echo owns the WASM package build ritual for downstream consumers. Build the
bundler package with:

```sh
# from repository root
scripts/build-warp-wasm-package.sh
```

The command refreshes `crates/warp-wasm/pkg` with:

```sh
# equivalent underlying invocation from crates/warp-wasm/
wasm-pack build --target bundler --out-dir pkg --out-name rmg_wasm -- --features engine
```

The package export smoke test imports `crates/warp-wasm/pkg/rmg_wasm.js` and
asserts the byte ABI export surface:

```sh
# from repository root
scripts/tests/warp_wasm_package_exports_test.sh
```

Consumer-facing app code should not depend on Echo's internal default
worldline. Durable app integration should route through optic-owned basis
resolution; raw worldline coordinates remain substrate/debug evidence, not a
product contract.

## Documentation

- Echo runtime model: `docs/architecture/outline.md`.
- Current WASM ABI contract: `docs/spec/SPEC-0009-wasm-abi.md`.
- JS/CBOR encoding rules: `docs/spec/js-cbor-mapping.md`.
