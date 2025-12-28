<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# echo-wasm-bindings

Minimal WASM bindings shim for Echo/JITOS living specs. Provides `DemoKernel` with add/set/connect/delete and rewrite history, using `echo-wasm-abi` DTOs. Exports wasm-bindgen-friendly methods when built with `--features wasm`.

## Dev

- Native tests: `cargo test -p echo-wasm-bindings`
- WASM build (example): `wasm-pack build --target web -F wasm` or via `trunk` when wired into spec pages.

## Exposed API

- `DemoKernel::add_node(id)`
- `set_field(target, field, value)`
- `connect(from, to)`
- `delete_node(target)`
- `graph()` / `history()` (native) and `serialize_graph()` / `serialize_history()` (WASM)
