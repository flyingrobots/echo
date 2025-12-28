<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# warp-wasm

wasm-bindgen bindings for the `warp-core` engine for tooling and web environments.

See the repository root `README.md` for the full overview.

## What this crate does

- Wraps `warp-core` in `wasm-bindgen` bindings so Echo’s deterministic engine
  can be used from JavaScript/TypeScript in web-based tools and playgrounds.
- Intended to power future browser-based visualizers and inspectors built on
  top of the same core engine as native tools.

## Documentation

- High-level engine and tool architecture:
  - Core + Math booklets in `docs/book/echo/`,
  - Tool booklet (`booklet-05-tools.tex`) for inspector/editor patterns.
- Web/wasm integration specifics will be documented alongside the first
  browser-based tool that consumes this crate.
