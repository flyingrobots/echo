# Echo Code Map

> Quick index from concepts → code, with the most relevant specs.

## Crates

- rmg-core — deterministic graph rewriting engine (Rust)
  - Public API aggregator: `crates/rmg-core/src/lib.rs`
  - Identifiers & hashing: `crates/rmg-core/src/ident.rs`
  - Node/edge records: `crates/rmg-core/src/record.rs`
  - In-memory graph store: `crates/rmg-core/src/graph.rs`
  - Rules and patterns: `crates/rmg-core/src/rule.rs`
  - Transactions: `crates/rmg-core/src/tx.rs`
  - Deterministic scheduler: `crates/rmg-core/src/scheduler.rs`
  - Snapshots + hashing: `crates/rmg-core/src/snapshot.rs`
  - Payload codecs (demo): `crates/rmg-core/src/payload.rs`
  - Engine implementation: `crates/rmg-core/src/engine_impl.rs`
  - Demo rule: `crates/rmg-core/src/demo/motion.rs`
  - Deterministic math: `crates/rmg-core/src/math/*`
  - Tests (integration): `crates/rmg-core/tests/*`

- rmg-ffi — C ABI for host integrations
  - `crates/rmg-ffi/src/lib.rs`

- rmg-wasm — wasm-bindgen bindings
  - `crates/rmg-wasm/src/lib.rs`

- rmg-cli — CLI scaffolding
  - `crates/rmg-cli/src/main.rs`

## Specs → Code

- RMG core model — docs/spec-rmg-core.md → `ident.rs`, `record.rs`, `graph.rs`, `rule.rs`, `engine_impl.rs`, `snapshot.rs`, `scheduler.rs`
- Scheduler — docs/spec-scheduler.md → `scheduler.rs`, `engine_impl.rs`
- ECS storage (future) — docs/spec-ecs-storage.md → new `ecs/*` modules (TBD)
- Serialization — docs/spec-serialization-protocol.md → `snapshot.rs` (hashing), future codecs
- Deterministic math — docs/spec-deterministic-math.md → `math/*`
- Temporal bridge/Codex’s Baby — docs/spec-temporal-bridge.md, docs/spec-codex-baby.md → future modules (TBD)

## Conventions

- Column-major matrices, right-handed coordinates, f32 math.
- One concrete concept per file; keep modules < 300 LoC where feasible.
- Tests live in `crates/<name>/tests` and favor small, focused cases.

## Refactor Policy

- 1 file = 1 concrete concept (engine, graph store, identifiers, etc.).
- No 500+ LoC “god files”; split before modules exceed ~300 LoC.
- Keep inline tests in separate files under `crates/<name>/tests`.
- Maintain stable re-exports in `lib.rs` so public API stays coherent.

## Onboarding

- Start with `README.md` and `docs/docs-index.md`.
- For engine flow, read `engine_impl.rs` (apply → schedule → commit → snapshot).
- For demo behavior, see `demo/motion.rs` and tests under `crates/rmg-core/tests/*`.
