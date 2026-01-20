<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Changelog

## Unreleased

### Changed - API

- **`WarpOpKey` now public** (`tick_patch.rs`): Export `WarpOpKey` from `warp_core` public API
- **`WarpOp::sort_key()` now public**: Changed from `pub(crate)` to `pub` to enable external determinism verification

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

- **Stride fallback** (`boaw/exec.rs`): Feature-gated Phase 6A fallback
    - Requires `parallel-stride-fallback` feature + `ECHO_PARALLEL_STRIDE=1` env var
    - Prints loud ASCII warning banner when activated
    - Temporary A/B benchmarking path; will be removed in a future release

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
