<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# warp-ffi

Thin C ABI bindings for Echo’s deterministic engine (`warp-core`).

This crate produces a C-callable library for embedding Echo’s core in other runtimes (C/C++, host modules alongside Rhai, etc.).

Today, the exposed surface is intentionally small and focused on the **motion rewrite spike** (a concrete, deterministic end-to-end example). As the engine hardens, this crate can grow toward a broader “register rules by name, apply/commit, snapshot” ABI.

## Platforms and Toolchain

- Rust toolchain is pinned by the repository `rust-toolchain.toml`.
- MSRV policy is tracked by CI (when enabled) and the root docs.
- Targets: macOS (aarch64/x86_64), Linux (x86_64). Windows support is planned.

## Building

Build static and shared libraries:

```
cargo build -p warp-ffi --release
```

Artifacts (platform-dependent):

- `target/release/libwarp_ffi.a` (static)
- `target/release/libwarp_ffi.dylib` or `libwarp_ffi.so` (shared)

## Linking

Example (clang):

```
clang -o demo demo.c -L target/release -lwarp_ffi -Wl,-rpath,@executable_path/../lib
```

Ensure the library search path includes `target/release` (or install path) at runtime.

## API Overview

Headers are generated in a follow-up task. The currently-exported ABI is motion-demo focused:

- `warp_engine_new() -> warp_engine*`
- `warp_engine_free(warp_engine*)`
- `warp_engine_spawn_motion_entity(warp_engine*, const char* label, ... , warp_node_id* out)`
- `warp_engine_begin(warp_engine*) -> warp_tx_id`
- `warp_engine_apply_motion(warp_engine*, warp_tx_id, const warp_node_id*) -> int` (`0`/`1` as bool)
- `warp_engine_commit(warp_engine*, warp_tx_id, warp_snapshot* out) -> int` (`0`/`1` as bool)
- `warp_engine_read_motion(warp_engine*, const warp_node_id*, float* out_pos3, float* out_vel3) -> int`

Snapshots currently expose a 32-byte BLAKE3 hash. See `docs/spec-mwmr-concurrency.md` for determinism rules.

## Quick Start (Pseudo‑C)

```c
warp_engine* eng = warp_engine_new();
warp_node_id entity;
warp_engine_spawn_motion_entity(eng, "entity-1", /* pos */ 0,0,0, /* vel */ 0,0,0, &entity);
warp_tx_id tx = warp_engine_begin(eng);
warp_engine_apply_motion(eng, tx, &entity);
warp_snapshot snap;
warp_engine_commit(eng, tx, &snap);
warp_engine_free(eng);
```

## Troubleshooting

- Undefined symbols at link: verify `-L` and `-l` flags and that `cargo build --release` produced the library.
- Snapshot hashes differ across runs: confirm identical state and rule registrations; see determinism invariants in `docs/determinism-invariants.md`.

## More Documentation

- Root docs: see repository `README.md` for the architecture and links.
- Engine surface: `crates/warp-core/src/lib.rs` (re‑exports) and rustdoc.
- Engine design details: Core booklet (`docs/book/echo/booklet-02-core.tex`)
  and ECS/scheduler specs in `docs/` (`spec-ecs-storage.md`,
  `spec-scheduler.md`, etc.).
