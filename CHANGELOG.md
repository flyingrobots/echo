<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Changelog

## Unreleased

### Added - SPEC-0004: Worldlines & Playback

- **`worldline.rs`**: Worldline types for history tracking
    - `WorldlineId(Hash)`: Opaque worldline identifier (derived from initial state hash in production; tests use fixed bytes)
    - `HashTriplet`: state_root + patch_digest + commit_hash per tick
    - `WorldlineTickPatchV1`: Per-warp projection of global tick operations
    - `apply_warp_op_to_store()`: Apply WarpOp to GraphStore with explicit variant coverage

- **`playback.rs`**: Playback cursor and session types
    - `PlaybackCursor`: Materialized viewpoint into worldline history
    - `PlaybackMode`: Paused, Play, StepForward, StepBack, Seek state machine
    - `ViewSession`: Client subscription binding with channel filtering
    - `TruthSink`: Minimal BTreeMap-based frame collector
    - `CursorReceipt`, `TruthFrame`: Cursor-addressed authoritative values

- **`provenance_store.rs`**: Provenance store trait (hexagonal port)
    - `ProvenanceStore` trait: Seam for history access (patches, expected hashes, outputs)
    - `LocalProvenanceStore`: In-memory Vec-backed implementation
    - `add_checkpoint()`: Record checkpoint for fast seek during replay
    - `fork()`: Prefix-copy worldline up to fork_tick

- **`retention.rs`**: Retention policy for worldline history
    - `RetentionPolicy` enum: KeepAll, CheckpointEvery, KeepRecent, ArchiveToWormhole

- **`materialization/frame_v2.rs`**: MBUS v2 wire format with cursor stamps
    - `V2Packet`: Cursor-stamped truth frame packets
    - `encode_v2_packet()`, `decode_v2_packet()`: Roundtrip encoding
    - Inline unit tests T19-T22 (SPEC-0004 test IDs): `mbus_v2_roundtrip_single_packet`, `mbus_v1_rejects_v2`, `mbus_v2_rejects_v1`, `mbus_v2_multi_packet_roundtrip`

#### Tests - SPEC-0004

- **All SPEC-0004 tests passing** (see test files for complete list; SPEC-0004 test IDs, not Rust function names)
- **`crates/warp-core/tests/reducer_emission_tests.rs`**: Reducer integration tests (T11-T13)
- **`crates/warp-core/tests/view_session_tests.rs`**: Worker count invariance tests (T16)
- **Hexagonal testing**: Playback contract tested using ProvenanceStore fakes (T1, T7)
- **Total warp-core tests**: all passing (run `cargo test -p warp-core -- --list 2>/dev/null | tail -1` for current count)

### Added - Cross-Warp Parallelism (Phase 6B+)

- **`WorkUnit` struct** (`boaw/exec.rs`): Work unit carrying `warp_id` + items for one shard
- **`build_work_units()`** (`boaw/exec.rs`): Partitions items by warp then by shard into work units
- **`execute_work_queue()`** (`boaw/exec.rs`): Global work queue with atomic unit claiming
    - Single spawn site (no nested threading)
    - Workers claim `(warp, shard)` units via `AtomicUsize`
    - Views resolved per-unit, dropped before claiming next unit
    - Fixed worker pool sized to `available_parallelism()`

### Changed - Cross-Warp Parallelism

- **Engine execution** (`engine_impl.rs`): Replaced serial per-warp for-loop with global work queue
    - Previous: `for (warp_id, rewrites) in by_warp { execute_parallel_sharded(...) }`
    - Now: `execute_work_queue(&units, workers, |warp_id| state.store(warp_id))`
    - Multi-warp ticks now parallelize across all `(warp, shard)` units simultaneously

### Changed - API

- **`WarpOpKey` now public** (`tick_patch.rs`): Export `WarpOpKey` from `warp_core` public API
- **`WarpOp::sort_key()` now public**: Changed from `pub(crate)` to `pub` to enable external determinism verification
- **`compute_commit_hash_v2` now public** (`snapshot.rs`): Promoted from `pub(crate)` to `pub` and re-exported from `warp_core`; enables external Merkle chain verification

### Removed - Tier 0 Cleanup

- **Stride fallback** (`boaw/exec.rs`): Deleted `execute_parallel_stride()` and `parallel-stride-fallback` feature
    - Phase 6A stride execution superseded by Phase 6B sharded execution
    - Removed feature gate, env var check, and ASCII warning banner
    - Deleted `sharded_equals_stride` and `sharded_equals_stride_permuted` tests (no longer needed post-transition)
- **Deprecated `emit_view_op_delta()`** (`rules.rs`): Deleted non-deterministic function that used `delta.len()` sequencing

### Fixed - Review Feedback

- **P0: Off-by-one in `publish_truth`** (`playback.rs`): Query `prov_tick = cursor.tick - 1` (0-based index of last applied patch) instead of `cursor.tick`; added early-return guard for `cursor.tick == 0`
- **P0: Wrong package in bench docs** (`docs/notes/boaw-perf-baseline.md`): Corrected `warp-core` → `warp-benches`
- **P1: Merkle chain verification** (`playback.rs`): `seek_to` now verifies `patch_digest`, recomputes `commit_hash` via `compute_commit_hash_v2`, and tracks parent chain per tick; added `SeekError::PatchDigestMismatch` and `SeekError::CommitHashMismatch` variants
- **P1: Dead variant removal** (`playback.rs`): Removed `SeekThen::RestorePrevious` (broken semantics; treated identically to `Pause`)
- **P1: OOM prevention** (`materialization/frame_v2.rs`): Bound `entry_count` by remaining payload size in `decode_v2_packet` to prevent malicious allocation
- **P1: Fork guard** (`provenance_store.rs`): Added `WorldlineAlreadyExists` error variant; `fork()` rejects duplicate worldline IDs
- **P1: Dangling edge validation** (`worldline.rs`): `UpsertEdge` now verifies `from`/`to` nodes exist in store before applying
- **P1: Silent skip → Result** (`boaw/exec.rs`): `execute_work_queue` returns `Result<Vec<TickDelta>, WarpId>` instead of panicking on missing store; caller maps to `EngineError::InternalCorruption`
- **P2: Tilde-pin bytes dep** (`crates/warp-benches/Cargo.toml`): `bytes = "~1.11"` for minor-version stability
- **P2: Markdownlint MD060** (`.markdownlint.json`): Removed global MD060 disable (all tables are well-formed; no false positives to suppress)
- **P2: Test hardening** (`tests/`): Real `compute_commit_hash_v2` in all test worldline setups, u8 truncation guards (`num_ticks <= 127`), updated playback tests to match corrected `publish_truth` indexing
- **Trivial: Phase 6B benchmark** (`boaw_baseline.rs`): Added `bench_work_queue` exercising full `build_work_units → execute_work_queue` pipeline across multi-warp setups
- **Trivial: Perf baseline stats** (`docs/notes/boaw-perf-baseline.md`): Expanded statistical context note with sample size, CI methodology, and Criterion report location

### Fixed - PR #257 Review

- **pre-commit hook**: Preserve stderr in prettier checks (changed `>/dev/null 2>&1` to `>/dev/null`) so errors are visible
- **`boaw_end_to_end.rs`**: Added `state_root` and `patch_digest` checks to `boaw_small_scenario_serial_parallel_equivalence` for full hash parity
- **`boaw_merge_tripwire.rs`**: Replaced brittle Debug string assertion with direct `WarpOpKey` comparison in `merge_conflict_reports_correct_key`
- **`boaw_stress_multiwarp.rs`**: Corrected misleading "10k rewrites" description to "5k rewrites" (test only executed one warp)

### Documentation - PR #257 Review

- **OpenPortal placeholder tests** (`boaw_openportal_rules.rs`): Added `TODO(T7.1)` tracking comments to 4 ignored tests for discoverability

### Documentation - Agent Context System

- **2-tier context system** (`AGENTS.md`): Documented Redis + knowledge graph handoff protocol
    - Session start: Bootstrap from `echo:agent:handoff` stream + `search_nodes()`
    - During work: Continuous updates to Redis stream after significant actions
    - Session end: Mandatory handoff entry with branch, status, next steps
    - Entity naming conventions: `<Feature>_Architecture`, `<Feature>_Phase<N>`, `<Feature>_BugFix`

### Fixed - Documentation

- **rustdoc links**: Changed intra-doc links to private `default_worker_count()` to plain code spans
- **determinism allowlist**: Added `engine_impl.rs` for `ECHO_WORKERS` env var (configuration, not runtime non-determinism)

### Added - Phase 6B (ADR-0007)

#### Virtual Shards

- **`boaw/shard.rs`**: Virtual shard partitioning for cache locality
    - `NUM_SHARDS = 256`: Protocol constant, frozen once shipped
    - `shard_of(scope: &NodeId) -> usize`: Byte-stable routing (see ADR-0007 § 7.1)
    - `partition_into_shards(items) -> Vec<VirtualShard>`: Distributes ExecItems by scope locality
    - 5 hardcoded test vectors to prevent routing regression

- **`execute_parallel_sharded()`** (`boaw/exec.rs`): Shard-based parallel execution
    - Workers claim shards via `AtomicUsize` (lockless work-stealing)
    - Items in same shard processed together for cache locality
    - Worker count capped at `min(workers, NUM_SHARDS)` to prevent over-threading

- **5 new Phase 6B tests** (`tests/boaw_parallel_exec.rs`):
    - `sharded_equals_stride`: Key correctness proof for 6A → 6B transition
    - `sharded_equals_stride_permuted`: Permutation invariance with sharded execution
    - `worker_count_capped_at_num_shards`: Verifies cap at 256 workers
    - `sharded_distribution_is_deterministic`: Shard routing stability
    - `default_parallel_uses_sharded`: Default path verification

#### Engine Integration

- **Engine parallel execution** (`engine_impl.rs`): `apply_reserved_rewrites()` now uses
  `execute_parallel_sharded()` for per-warp parallel execution
    - Rewrites grouped by `warp_id`, parallelized within each warp
    - Worker count configurable via `ECHO_WORKERS` env var or `EngineBuilder::workers(n)`
    - Defaults to `available_parallelism().min(NUM_SHARDS)` (capped at 256)

- **Warp-scoped footprints** (`echo-dind-tests/rules.rs`): Footprint helpers now use
  `NodeSet`/`EdgeSet` with explicit warp scoping via `insert_with_warp()`

- **`WarpOpKey` warp distinction test** (`tick_patch.rs`): Verifies ops targeting same
  local node but different warps have distinct sort keys

### Architecture - Phase 6B

- **Merge invariant**: Canonical merge by `(WarpOpKey, OpOrigin)` enforces determinism regardless of execution order
- **Engine integration**: `apply_reserved_rewrites()` uses `execute_parallel_sharded()` with per-warp parallelism

### Fixed - Phase 6B

- **DIND determinism**: Regenerated all golden hash files with parallel execution
- **View op ID collisions** (`echo-dind-tests/rules.rs`): Fixed `emit_view_op_delta()` using worker-local `delta.len()` which varied based on shard claim order; created `emit_view_op_delta_scoped()` that derives op ID from intent scope (NodeId) for deterministic sequencing
- Benchmarks: removed unseeded randomness from `scheduler_adversarial` by generating inputs via deterministic `warp_core::math::Prng` (no `rand::thread_rng()`).
- `demo_rules::port_executor` now skips emitting `SetAttachment` when the canonical motion payload bytes do not change after update.
- Docs: fixed `spec-warp-core.md` Stage B1 executor example to match the Phase 5 BOAW `ExecuteFn` signature and `TickDelta::push` API.

---

### Added - Phase 6A: Parallel Execution "FREE MONEY" (ADR-0007)

- **`boaw` module** (`boaw/mod.rs`, `boaw/exec.rs`, `boaw/merge.rs`): Parallel execution with canonical merge
    - `execute_serial()`: Baseline serial execution for determinism comparison
    - `execute_parallel()`: Lockless stride-partitioned parallel execution across N workers
    - `merge_deltas()`: Canonical merge sorting by `(WarpOpKey, OpOrigin)` with conflict detection
    - `ExecItem`: Execution unit bundling executor, scope, and origin metadata
    - `MergeConflict`: Error type for footprint violations (conflicts = bugs in Phase 6A)

- **`OpOrigin` enhanced** (`tick_delta.rs`):
    - Added `op_ix: u32` field for per-rewrite sequential ordering
    - Added `PartialOrd`, `Ord`, `Hash` derives for canonical sorting
    - `ScopedDelta` now auto-increments `op_ix` on each `emit()` call

- **`TickDelta::into_parts_unsorted()`**: Returns `(Vec<WarpOp>, Vec<OpOrigin>)` for merge

- **`WarpOp` derives** (`tick_patch.rs`): Added `PartialEq`, `Eq` for merge deduplication

- **7 new determinism tests** (`tests/boaw_parallel_exec.rs`):
    - `parallel_equals_serial_basic`: Serial and parallel produce identical merged results
    - `worker_count_invariance`: Identical output across worker counts [1, 2, 4, 8, 16, 32]
    - `permutation_invariance_under_parallelism`: Shuffled input × varied workers = same output
    - `merge_dedupes_identical_ops`: Merge correctly deduplicates identical ops
    - `empty_execution_produces_empty_result`: Edge case coverage
    - `single_item_execution`: Edge case coverage
    - `large_workload_worker_count_invariance`: 100 items × all worker counts

### Architecture - Phase 6A

- **Determinism is a merge decision, not an execution constraint**: Workers execute in arbitrary order; canonical sort at merge enforces determinism
- **Worker-count invariance proven**: Same `patch_digest`, `state_root`, `commit_hash` regardless of worker count
- **Conflicts are bugs**: Phase 6A treats merge conflicts as footprint model violations (explode loudly)
- **No worker ID in output**: `OpOrigin` contains only `(intent_id, rule_id, match_ix, op_ix)` — no thread/worker information

---

### Added - Phase 5: Read-Only Execution (ADR-0007)

- **`GraphView<'a>`** (`graph_view.rs`): Read-only wrapper for `GraphStore`
    - Provides immutable access to nodes, edges, and state during execution
    - Enforces the emit-only contract for executors
    - Engine holds immutable store reference during tick execution

### Changed - Phase 5

- `ExecuteFn` signature now takes `&GraphView` instead of `&mut GraphStore`
- `MatchFn` signature updated to use `&GraphView` for rule matching
- `FootprintFn` signature updated to use `&GraphView` for footprint computation
- Executors are now emit-only: no direct `GraphStore` mutations during execution
- State updates applied via `WarpTickPatchV1::apply_to_state()` after execution completes

### Architecture - Phase 5

- Execution phase is now purely read-only with respect to graph state
- All mutations flow through the delta/patch system
- All BOAW determinism tests pass with read-only execution model

---

### Added - Phase 4: SnapshotAccumulator (ADR-0007)

- **`SnapshotAccumulator`** (`snapshot_accum.rs`): Columnar accumulator that builds WSC directly from `base + ops` without reconstructing GraphStore
    - `from_warp_state()`: Captures immutable base state
    - `apply_ops()`: Processes all 8 `WarpOp` variants
    - `compute_state_root()`: Computes state hash directly from accumulator tables
    - `build()`: Produces WSC bytes and state_root

### Changed - Phase 4

- `apply_reserved_rewrites()` now returns `Vec<WarpOp>` (the finalized delta ops)
- Added validation under `delta_validate` feature: asserts accumulator's `state_root` matches legacy computation
- `TickDelta::finalize()` uses `sort_by_key` for cleaner sorting

### Fixed - Phase 4

- `assert_delta_matches_diff()` / `validate_delta_matches_diff()` now compare full `WarpOp` values (payload included), not just `sort_key()`, so executor-emitted ops that target the same key but carry the wrong data are caught under `delta_validate`.

### Architecture - Phase 4

- Delta ops are now the source of truth for state changes
- `SnapshotAccumulator` validates that `base + ops → state_root` matches legacy path
- Paves the way for Phase 5: read-only execution with accumulator as primary output

### Added - Pre-Phase 4 Foundations

- Added real `EngineHarness` implementation for BOAW compliance tests (ADR-0007)
- Added `BoawSnapshot` struct and `boaw/touch` test rule
- 8 BOAW determinism tests now pass with real engine hashes
- Added `TickDelta` module for collecting ops during execution (ADR-0007 Phase 3)
- Changed `ExecuteFn` signature to accept `&mut TickDelta` parameter
- Added `assert_delta_matches_diff()` validation helper with `delta_validate` feature

## 2026-01-17 — MaterializationBus Phase 3 Complete

- Completed MaterializationBus Phase 3 implementation:
    - FinalizeReport pattern: `finalize()` never fails, returns `{channels, errors}`
    - Prevents silent data loss when one channel has StrictSingle conflict
    - 7 new SPEC Police tests for conflict preservation
- Added new modules to `warp-core/src/materialization`:
    - `emission_port.rs` — Port abstraction for emission routing
    - `reduce_op.rs` — Reduction operation definitions
    - `scoped_emitter.rs` — Scoped emission context management
- Added CI workflows:
    - `determinism.yml` — PR-gated determinism tests
    - `dind-cross-platform.yml` — Weekly cross-platform determinism proof (Linux x64/ARM64, Windows, macOS)
- Added tooling:
    - `cargo xtask dind` command with `run`, `record`, `torture`, and `converge` subcommands
- DIND mission 100% complete.

- Added `codec` module to `echo-wasm-abi`:
    - Deterministic binary codec (`Reader`/`Writer`) for length-prefixed LE scalars
    - Q32.32 fixed-point helpers (`fx_from_i64`, `fx_from_f32`, `vec3_fx_from_*`)
    - Overflow-safe conversions with saturation for out-of-range inputs
    - `Encode`/`Decode` traits for composable serialization
- Added `fixed` module to `warp-core`:
    - `Fx32` scalar type for Q32.32 fixed-point arithmetic
    - `Vec3Fx` 3D vector type with fixed-point components
    - Overflow-safe constructors with range validation
- Added WSC (Write-Streaming Columnar) snapshot format to `warp-core`:
    - Deterministic serialization of WARP graph state with zero-copy mmap deserialization
    - 8-byte aligned columnar layout for SIMD-friendly access
    - New modules: `wsc::{build, read, types, validate, view, write}`
    - Uses `bytemuck` for safe Pod/Zeroable transmutation (no `unsafe` code)
- Upgraded canonical state hash from V1 (u32 counts) to V2 (u64 counts) for future-proofing.
- Changed generated file convention from `generated/*.rs` to `*.generated.rs`.
- Updated pre-push hook to exclude `*.generated.rs` files from `missing_docs` lint.
- Added `#[repr(transparent)]` to ID newtypes (`NodeId`, `EdgeId`, `TypeId`, `WarpId`).
- Added `as_bytes()` method to `EdgeId` and `TypeId` for consistent byte access.
- Added `crates/echo-dind-harness` to the Echo workspace (moved from flyingrobots.dev).
- Added `crates/echo-dind-tests` as the stable DIND test app (no dependency on flyingrobots.dev).
- Moved DIND scenarios and generator scripts into `testdata/dind` and `scripts/`.
- Added convergence scopes in DIND manifest; `converge` now compares projected hashes.
- Documented convergence scope semantics and added a guarded `--scope` override.
- Wired determinism guard scripts and DIND PR suite into CI.
- Added spec for canonical inbox sequencing and deterministic scheduler tie-breaks.
- Added determinism guard scripts: `scripts/ban-globals.sh`, `scripts/ban-nondeterminism.sh`, and `scripts/ban-unordered-abi.sh`.
- Added `ECHO_ROADMAP.md` to capture phased plans aligned with recent ADRs.
- Removed legacy wasm encode helpers from `warp-wasm` (TS encoders are the protocol source of truth).
