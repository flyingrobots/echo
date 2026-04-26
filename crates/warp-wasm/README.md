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
  (`ABI_VERSION` 5 in `echo-wasm-abi`): `observe(...)` is the only public
  world-state read export, `scheduler_status()` is the read-only scheduler
  metadata export, and `dispatch_intent(...)` is the write/control ingress.
  The current ABI also publishes strand settlement comparison, planning,
  execution entrypoints, and read-side basis posture on observation artifacts.
- The engine-backed boundary uses logical clocks only:
  `WorldlineTick` is per-worldline append identity and `GlobalTick` is runtime
  cycle correlation metadata. No wall-clock time enters Echo internals.
- Public scheduler control is expressed as privileged control intents packed
  into the same EINT envelope format as domain intents. The canonical control
  surface includes `Start`, `Stop`, and `SetHeadEligibility`.
- Intended to power future browser-based visualizers and inspectors built on
  top of the same core engine as native tools.

## Documentation

- High-level engine and tool architecture:
    - Core + Math booklets in `docs/book/echo/`,
    - Tool booklet (`booklet-05-tools.tex`) for inspector/editor patterns.
- Web/wasm integration specifics will be documented alongside the first
  browser-based tool that consumes this crate.
