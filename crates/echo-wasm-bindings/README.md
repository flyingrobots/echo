<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# echo-wasm-bindings

Minimal WASM bindings shim for Echo/JITOS living specs. Provides `DemoKernel` with add/set/connect/delete and rewrite history, using `echo-wasm-abi` DTOs. Exports wasm-bindgen-friendly methods when built with `--features wasm`.

## Dev

- Native tests: `cargo test -p echo-wasm-bindings`
- WASM build (example): `wasm-pack build --target web -F wasm` or via `trunk` when wired into spec pages.

## Exposed API

- `DemoKernel::new() -> DemoKernel`
- `DemoKernel::add_node(id: String)` (no-op if id exists)
- `DemoKernel::set_field(target: String, field: String, value: Value)`
- `DemoKernel::connect(from: String, to: String)` (no-op if either node is missing)
- `DemoKernel::delete_node(target: String)` (no-op if node is missing)
- `DemoKernel::graph() -> Rmg` / `DemoKernel::history() -> Vec<Rewrite>` (native clones)
- `DemoKernel::graph_json() -> String` / `DemoKernel::history_json() -> String` (JSON strings; native + WASM)
- `serializeGraph() -> String` / `serializeHistory() -> String` (WASM JS names; JSON strings)
