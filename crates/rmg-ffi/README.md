# rmg-ffi

Thin C ABI bindings for the `rmg-core` deterministic graph rewriting engine.

This crate produces a C-callable library for embedding Echo’s core in other runtimes (C/C++, Lua, etc.). It exposes a minimal, stable surface: engine creation, rule registration by name, apply/commit, and snapshot hash retrieval.

## Platforms and Toolchain

- Rust: 1.68 (pinned via `rust-toolchain.toml`)
- Targets: macOS (aarch64/x86_64), Linux (x86_64). Windows support is planned.

## Building

Build static and shared libraries:

```
cargo build -p rmg-ffi --release
```

Artifacts (platform-dependent):

- `target/release/librmg_ffi.a` (static)
- `target/release/librmg_ffi.dylib` or `librmg_ffi.so` (shared)

## Linking

Example (clang):

```
clang -o demo demo.c -L target/release -lrmg_ffi -Wl,-rpath,@executable_path/../lib
```

Ensure the library search path includes `target/release` (or install path) at runtime.

## API Overview

Headers are generated in a follow-up task; the intended functions mirror `rmg-core`:

- `rmg_engine_new(...) -> rmg_engine*`
- `rmg_engine_free(rmg_engine*)`
- `rmg_engine_register_rule(rmg_engine*, const char* name) -> int` (0 = ok)
- `rmg_engine_begin(rmg_engine*) -> uint64_t`
- `rmg_engine_apply(rmg_engine*, uint64_t tx, const char* rule_name, const rmg_node_id* scope) -> int`
- `rmg_engine_commit(rmg_engine*, uint64_t tx, rmg_snapshot* out) -> int`

Snapshots expose a 32-byte BLAKE3 hash and root id. See `docs/spec-mwmr-concurrency.md` for determinism rules.

## Quick Start (Pseudo‑C)

```c
rmg_engine* eng = rmg_engine_new();
rmg_engine_register_rule(eng, "motion/update");
uint64_t tx = rmg_engine_begin(eng);
rmg_node_id scope = rmg_make_node_id("entity-1");
int applied = rmg_engine_apply(eng, tx, "motion/update", &scope);
rmg_snapshot snap;
rmg_engine_commit(eng, tx, &snap);
```

## Troubleshooting

- Undefined symbols at link: verify `-L` and `-l` flags and that `cargo build --release` produced the library.
- Snapshot hashes differ across runs: confirm identical state and rule registrations; see determinism invariants in `docs/determinism-invariants.md`.

## More Documentation

- Root docs: see repository `README.md` for the architecture and links.
- Engine surface: `crates/rmg-core/src/lib.rs` (re‑exports) and rustdoc.
