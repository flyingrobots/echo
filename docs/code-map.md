<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Code Map

> Quick index from concepts → code, with the most relevant specs.

## Crates

- warp-core — deterministic graph rewriting engine (Rust)
    - Public API aggregator: `crates/warp-core/src/lib.rs`
    - Identifiers & hashing: `crates/warp-core/src/ident.rs`
    - Node/edge records: `crates/warp-core/src/record.rs`
    - In-memory graph store: `crates/warp-core/src/graph.rs`
    - Rules and patterns: `crates/warp-core/src/rule.rs`
    - Transactions: `crates/warp-core/src/tx.rs`
    - Deterministic scheduler: `crates/warp-core/src/scheduler.rs`
    - Snapshots + hashing: `crates/warp-core/src/snapshot.rs`
    - Payload codecs (demo): `crates/warp-core/src/payload.rs`
    - Engine implementation: `crates/warp-core/src/engine_impl.rs`
    - Playback & view sessions: `crates/warp-core/src/playback.rs` (PlaybackCursor, ViewSession, TruthSink, TruthFrame)
    - Worldlines & temporal graphs: `crates/warp-core/src/worldline.rs` (WorldlineId, HashTriplet, apply_warp_op_to_store)
    - Provenance tracking: `crates/warp-core/src/provenance_store.rs` (ProvenanceStore trait, LocalProvenanceStore)
    - Retention policies: `crates/warp-core/src/retention.rs` (RetentionPolicy enum)
    - Materialization V2 codec: `crates/warp-core/src/materialization/frame_v2.rs` (V2Packet encoder/decoder)
    - Demo rule: `crates/warp-core/src/demo/motion.rs`
    - Deterministic math: `crates/warp-core/src/math/*`
    - Tests (integration): `crates/warp-core/tests/*`

- warp-ffi — C ABI for host integrations
    - `crates/warp-ffi/src/lib.rs`

- warp-wasm — wasm-bindgen bindings
    - `crates/warp-wasm/src/lib.rs`

- warp-cli — CLI scaffolding
    - `crates/warp-cli/src/main.rs`

## Specs → Code

- WARP core model — docs/spec-warp-core.md → `ident.rs`, `record.rs`, `graph.rs`, `rule.rs`, `engine_impl.rs`, `snapshot.rs`, `scheduler.rs`
- Scheduler — docs/spec-scheduler.md → `scheduler.rs`, `engine_impl.rs`
- ECS storage (future) — docs/spec-ecs-storage.md → new `ecs/*` modules (TBD)
- Serialization — docs/spec-serialization-protocol.md → `snapshot.rs` (hashing), future codecs
- Deterministic math — docs/SPEC_DETERMINISTIC_MATH.md, docs/math-validation-plan.md → `math/*`
- Temporal bridge — docs/spec-temporal-bridge.md → future modules (TBD)
- Worldlines & playback (SPEC-0004) — docs/spec/SPEC-0004-worldlines-playback-truthbus.md → `playback.rs`, `worldline.rs`, `provenance_store.rs`, `retention.rs`, `materialization/frame_v2.rs`

## Test Coverage

- Reducer emission: `crates/warp-core/tests/reducer_emission_tests.rs` (T11-T13 reducer tests)
- View session & playback: `crates/warp-core/tests/view_session_tests.rs` (Playback + T16 tests)
- Playback outputs: `crates/warp-core/tests/outputs_playback_tests.rs` (SPEC-0004 test IDs T1, T4, T5, T6, T7, T8)
- Checkpoint & fork: `crates/warp-core/tests/checkpoint_fork_tests.rs` (T17-T18 checkpoint/fork tests)
- Playback cursor: `crates/warp-core/tests/playback_cursor_tests.rs` (Cursor seek tests)

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

- Start with `README.md` and `docs/meta/docs-index.md`.
- For engine flow, read `engine_impl.rs` (apply → schedule → commit → snapshot).
- For demo behavior, see `demo/motion.rs` and tests under `crates/warp-core/tests/*`.
