<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Spec-000 Rewrite (Living Spec)

Leptos + Trunk WASM scaffold for Spec-000: “Everything Is a Rewrite.” This page will embed the actual Echo/JITOS kernel in the browser to demonstrate rewrite-driven state.

## Dev

```
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
make spec-000-dev   # from repo root
```

Serves at http://127.0.0.1:8080 with hot reload.

## Build

```
make spec-000-build  # outputs dist/
```

## Next steps
- Wire kernel bindings (wasm-bindgen feature)
- Render WARP graph + rewrite log
- Add completion badge win condition
