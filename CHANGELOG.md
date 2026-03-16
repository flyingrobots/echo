<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Changelog

## Unreleased

### feat(tooling): split local verification into parallel lanes

- **Changed** the local full verifier now runs as curated parallel lanes with
  isolated `CARGO_TARGET_DIR`s for clippy, tests, rustdoc, and guard checks,
  which cuts local wall-clock time by avoiding one giant serialized cargo
  invocation.
- **Changed** staged and reduced local Rust checks now use a narrower fast-path
  target surface, keeping the heaviest all-target clippy drag in CI instead of
  every local iteration loop.
- **Changed** full local verification is now scope-aware: tooling-only full
  changes stay tooling-local, while critical Rust changes run local smoke lanes
  and defer exhaustive proof to CI.
- **Changed** local `warp-core` smoke selection is now file-family aware:
  default source edits stay on `--lib`, runtime/inbox files pull `inbox`,
  playback files pull playback-smoke tests, and PRNG edits pull the golden
  regression.
- **Changed** local `warp-wasm` and `echo-wasm-abi` smoke selection is now
  file-family aware too: `warp-wasm/src/lib.rs` stays on plain lib smoke,
  `warp_kernel.rs` pulls the engine-enabled lane, canonical ABI work pulls only
  canonical/floating-point vectors, and non-Rust crate docs no longer wake Rust
  lanes at all.
- **Added** `make verify-ultra-fast` as the shortest local edit-loop lane:
  changed Rust crates get `cargo check`, critical runtime surfaces still pull
  targeted smoke tests, tooling-only changes stay on a syntax/smoke path, and
  clippy/rustdoc/guard scans stay on heavier local paths and CI.
- **Added** `make verify-full-sequential` as an explicit fallback when the lane
  runner itself needs debugging.

### feat(warp-core): close Phase 4 and pivot reads to observe

- **Added** ADR-0011 documenting the explicit observation contract with
  worldline, coordinate, frame, and projection semantics.
- **Changed** Phase 4 provenance/BTR work is now the documented substrate
  baseline: provenance is entry-based, parent refs are stored explicitly, and
  the standalone `ProvenanceService` owns authoritative worldline history.
- **Added** `ObservationService::observe(...)` as the canonical internal read
  path with explicit worldline, coordinate, frame, and projection semantics.
- **Added** deterministic observation artifacts and error mapping:
  `INVALID_WORLDLINE`, `INVALID_TICK`, `UNSUPPORTED_FRAME_PROJECTION`,
  `UNSUPPORTED_QUERY`, and `OBSERVATION_UNAVAILABLE`.
- **Changed** `WarpKernel` and the WASM ABI now expose `observe(...)`, while
  `get_head`, `snapshot_at`, and `drain_view_ops` are thin one-phase adapters
  over the observation contract. `execute_query(...)` currently lowers through
  observation semantics and returns deterministic `UNSUPPORTED_QUERY` until full
  query support is implemented.
- **Changed** `drain_view_ops()` is now legacy adapter/debug behavior only: it
  reads recorded truth through `observe(...)` and tracks only adapter-local
  drain state instead of mutating runtime-owned materialization state.
- **Changed** `ttd-browser` migrated to the entry-based provenance API after
  the Phase 4 hard cut removed the old provenance convenience methods.

### fix(warp-core): close final Phase 3 PR review threads

- **Fixed** `Engine::commit_with_state()` now restores both the engine-owned
  runtime metadata and the borrowed `WorldlineState` even if rule execution
  unwinds, and duplicate admitted ingress is deduplicated by `ingress_id`
  before command enqueue.
- **Fixed** the canonical pre-commit hook now routes staged crate verification
  through `scripts/verify-local.sh pre-commit`, which uses index-scoped changed
  files plus an index-tree stamp instead of branch-`HEAD` reuse.
- **Clarified** cumulative `unpause(PlaybackMode::Paused)` notes now describe
  the shipped deterministic all-build failure instead of mixing final behavior
  with the earlier debug-only guard.

### fix(tooling): reduce duplicate local and feature-branch verification

- **Changed** `scripts/hooks/pre-commit` and `scripts/hooks/pre-push` now
  delegate to the canonical `.githooks/` implementations instead of enforcing a
  stale parallel local policy.
- **Added** `scripts/verify-local.sh` plus `make verify-fast`,
  `make verify-pr`, and `make verify-full` so local verification can scale with
  the change set and reuse a same-`HEAD` success stamp.
- **Changed** the canonical pre-push hook now classifies docs-only, reduced,
  and critical verification paths, escalating to a determinism/tooling-focused
  local gate only for determinism-critical, CI, hook, and build-system changes.
- **Fixed** manual `make verify-full` runs and the canonical pre-push full gate
  now share the same success stamp, so an explicit clean full pass suppresses
  the identical hook rerun for the same `HEAD`.
- **Changed** the curated local full test lane now runs library and integration
  targets only for the small non-core confidence crates, cutting doc-test-only
  churn while the script reports total elapsed time on completion or failure.
- **Changed** the main CI workflow no longer runs on `push` for `feat/**`
  branches, leaving `pull_request` as the authoritative branch-validation lane
  while `main` retains push-time protection.
- **Changed** the CI `Tests` gate now fans in from parallel `workspace sans
warp-core` and `warp-core` shards, preserving the required `Tests` status
  while cutting PR wall-clock time spent waiting on one serialized workspace job.
- **Changed** the `warp-core` CI shard now uses `cargo nextest` for the main
  test inventory and keeps `cargo test --doc` as a separate step so the heavy
  crate runs faster without dropping its doctest coverage.

### fix(warp-core): resolve final Phase 3 review invariants

- **Fixed** `Engine` now caches canonical `cmd/*` rule order at registration
  time instead of rebuilding and sorting that list for every admitted ingress
  envelope.
- **Fixed** `WorldlineRegistry::register(...)` now preserves the restored
  frontier tick implied by `WorldlineState.tick_history` instead of rewinding
  restored worldlines to tick 0.
- **Fixed** `WorldlineState` root validation is now fallible and explicit:
  callers must supply or derive the unique root instance with a backing store,
  and the old fabricated fallback root is gone.
- **Fixed** `WarpKernel::with_engine(...)` now returns a typed
  `KernelInitError` for non-fresh or invalid caller-supplied engine state
  instead of panicking through the WASM host boundary.
- **Clarified** ADR-0008 and the Phase 3 implementation plan now describe
  duplicate suppression as per-resolved-head, use full `head_key` values for
  per-head APIs, and keep `WorldlineRuntime` pseudocode encapsulated.

### fix(warp-core): resolve late Phase 3 PR follow-ups

- **Fixed** `WorldlineRuntime` no longer exposes raw public registries that can
  desynchronize the default-writer / named-inbox route tables; named inbox
  lookup is now allocation-free on the live ingress path.
- **Fixed** `SchedulerCoordinator::super_tick()` now preflights
  `global_tick`/`frontier_tick` overflow before draining inboxes or mutating
  worldline state.
- **Fixed** runtime ingress event materialization is now folded back into the
  recorded tick patch boundary, so replaying `initial_state + tick_history`
  matches the committed post-state.
- **Fixed** `WarpKernel::with_engine(...)` now rejects non-fresh engines
  instead of silently dropping runtime history that it cannot preserve.

### fix(warp-core): close remaining Phase 3 PR review threads

- **Fixed** duplicate worldline registration now surfaces as a typed
  `RuntimeError::DuplicateWorldline` at the runtime boundary instead of being
  silently ignored at the call site.
- **Fixed** golden-vector and proptest determinism harnesses now pin
  `EngineBuilder` to a single worker so hashes do not inherit ambient
  `ECHO_WORKERS` or host core-count entropy.
- **Fixed** GV-004 now pins both engines to the expected `state_root`,
  `patch_digest`, and `commit_hash` artifacts rather than checking only one run
  against constants and the second run for self-consistency.
- **Clarified** hook/docs governance: `.githooks/` installed via `make hooks`
  is canonical, `scripts/hooks/` are legacy shims, ADR-0008 now states seek is
  observational-only, and the ADR exceptions ledger no longer uses a sentinel
  pseudo-entry.

### fix(warp-core): harden Phase 3 runtime review follow-ups

- **Fixed** `HeadId` is now opaque with internal range bounds, so public callers
  cannot fabricate arbitrary head identities while `heads_for_worldline()` still
  keeps its `BTreeMap` range-query fast path.
- **Fixed** `WriterHead` now derives pause state from `mode`, and
  `unpause(PlaybackMode::Paused)` now fails deterministically in all builds
  instead of only under `debug_assert!`.
- **Fixed** `PlaybackHeadRegistry` and `WorldlineRegistry` no longer expose raw
  public mutable access to stored heads/frontiers; runtime code uses targeted
  internal inbox/frontier mutation instead.
- **Fixed** `IngressEnvelope` fields are now private and `HeadInbox::ingest()`
  enforces the canonical content hash in release builds too, closing the
  debug-only invariant hole.
- **Fixed** `SchedulerCoordinator::peek_order()` now derives runnable order from
  the head registry instead of trusting cached state, and tick counters now fail
  deterministically on overflow.
- **Fixed** INV-002 now asserts exact head-key equality against the canonical
  expected order, not just length plus pairwise zip checks.
- **Fixed** the ADR implementation plan now shows private-field pseudocode for
  worldline frontiers and the stronger verification matrix, including the
  rustdoc warnings gate (`RUSTDOCFLAGS="-D warnings" cargo doc ... --no-deps`).

### fix(warp-core): address CodeRabbit round-3 PR feedback

- **Fixed** `WriterHead.key` is now private with a `key()` getter, preventing
  mutation via `PlaybackHeadRegistry::get_mut()` which would break the BTreeMap
  key invariant.
- **Fixed** INV-002 proptest now verifies exact key identity (sorted+deduped
  input vs output), catching bugs where rebuild substitutes one key for another.
- **Fixed** plan doc pseudocode updated to reflect private fields with getters
  (`WriterHead`, `WorldlineFrontier`) and correct constructor name
  (`IngressEnvelope::local_intent`).

### fix(warp-core): address CodeRabbit round-2 PR feedback

- **Fixed** `WriterHead.mode` is now private with a `mode()` getter, preventing
  the `mode`/`paused` pair from diverging via direct field assignment.
- **Fixed** `SchedulerCoordinator::super_tick()` now uses canonical runnable
  order derived from the head registry via `peek_order()` instead of trusting
  stale runnable-cache state.
- **Fixed** `HeadInbox::set_policy()` now revalidates pending envelopes against
  the new policy, evicting any that no longer pass.
- **Fixed** `HeadInbox::admit()` now uses `mem::take` + `into_values()` instead
  of `clone()` + `clear()` for zero-copy admission in `AcceptAll`/`KindFilter`.
- **Fixed** `HeadInbox::ingest()` added envelope hash invariant checks; later
  hardening enforces the canonical `ingress_id`/payload-hash match in release
  builds as well.
- **Fixed** `WorldlineState.warp_state` is now `pub(crate)` with a `warp_state()`
  getter, and `WorldlineFrontier` fields are `pub(crate)` with public getters.
- **Fixed** INV-002 proptest now verifies set preservation (length check) in
  addition to canonical ordering.
- **Fixed** removed `redundant_clone` clippy suppression from `head.rs` and
  `coordinator.rs` test modules.
- **Fixed** ADR exceptions ledger sentinel row no longer mimics an active entry.
- **Fixed** verification matrix in implementation plan now matches hook-enforced
  gate (`--workspace --all-targets -D missing_docs`).

### fix(warp-core): self-review fixes for Phases 0–3

- **Fixed** `HeadInbox::ingest()` now rejects non-matching envelopes at ingest
  time under `KindFilter` policy, preventing unbounded memory growth.
- **Fixed** GV-003 golden vector now covers all 6 fork entries (ticks 0..=5),
  closing a gap where the fork-tick itself was never verified.
- **Added** INV-002 proptest for canonical head ordering (shuffled insertion
  always produces canonical `(worldline_id, head_id)` order).
- **Added** duplicate-tick detection to INV-001 (append at existing tick fails).
- **Fixed** `heads_for_worldline()` now uses BTreeMap range queries (O(log n + k)
  instead of O(n) full scan).
- **Fixed** `unpause()` initially added a debug-only guard for `Paused`; later
  hardening made the failure deterministic in all build configurations.
- **Fixed** pre-commit hook now passes `--workspace` to clippy.
- **Improved** documentation: multi-writer frontier semantics, `global_tick`
  behavior on empty SuperTicks, `compute_ingress_id` length-prefix safety,
  `InboxAddress` as human-readable alias.

### feat(warp-core): Phase 3 deterministic ingress and per-head inboxes

- **Added** `IntentKind` — stable, content-addressed intent kind identifier
  using domain-separated BLAKE3 (`"intent-kind:" || label`).
- **Added** `IngressEnvelope` — unified, content-addressed ingress model
  with deterministic routing and idempotent deduplication.
- **Added** `IngressTarget` — routing discriminant: `DefaultWriter`,
  `InboxAddress`, or `ExactHead` (control/debug only).
- **Added** `IngressPayload` — payload enum starting with `LocalIntent`,
  extensible for cross-worldline messages (Phase 10) and imports (Phase 11).
- **Added** `HeadInbox` — per-head inbox with `BTreeMap`-keyed pending
  envelopes for deterministic admission order.
- **Added** `InboxPolicy` — admission control: `AcceptAll`, `KindFilter`,
  or `Budgeted { max_per_tick }`.

### feat(warp-core): Phase 2 SchedulerCoordinator for ADR-0008

- **Added** `SchedulerCoordinator` — serial canonical scheduling loop that
  iterates runnable writer heads in `(worldline_id, head_id)` order and
  advances each worldline's frontier tick.
- **Added** `WorldlineRuntime` — top-level runtime struct bundling worldline
  registry, head registry, runnable set, and global tick.
- **Added** `StepRecord` — output record documenting which heads were stepped
  and in what order during a SuperTick.

### feat(warp-core): Phase 1 runtime primitives for ADR-0008

- **Added** `HeadId`, `WriterHeadKey`, `WriterHead` — first-class head types
  for worldline-aware scheduling. Heads are control objects (identity, mode,
  paused state), not private mutable stores.
- **Added** `PlaybackHeadRegistry` — `BTreeMap`-backed registry providing
  canonical `(worldline_id, head_id)` iteration order.
- **Added** `RunnableWriterSet` — ordered live index of non-paused writer heads.
- **Added** `WorldlineState` — broad wrapper around `WarpState` preventing API
  calcification around `GraphStore`.
- **Added** `WorldlineFrontier` — the single mutable frontier state per
  worldline, owning `WorldlineState` and `frontier_tick`.
- **Added** `WorldlineRegistry` — `BTreeMap`-backed registry of worldline
  frontiers with deterministic iteration.
- **Added** `make_head_id()` — domain-separated BLAKE3 identifier factory
  (`"head:" || label`).

### test(warp-core): Phase 0 invariant harness for ADR-0008/0009

- **Added** golden vector suite (`golden_vectors_phase0.rs`) pinning commit
  determinism, provenance replay integrity, fork reproducibility, and
  idempotent ingress hashes before the worldline runtime refactor.
- **Added** invariant test suite (`invariant_property_tests.rs`) enforcing
  monotonic worldline ticks, idempotent ingress, cross-worldline isolation,
  commit determinism, and provenance immutability; INV-001/002/003/005 use
  `proptest`, while INV-004/006 are fixed regression tests.
- **Added** ADR exceptions ledger (`docs/adr/adr-exceptions.md`) — operational
  from Phase 0 onward, every intentional model violation must be logged with
  owner and expiry.
- **Added** ADR-0010: Observational Seek, Explicit Snapshots, and
  Administrative Rewind — companion ADR clarifying the seek/rewind split
  under the one-frontier-state-per-worldline design.
- **Added** implementation plan for ADR-0008 and ADR-0009
  (`docs/plans/adr-0008-and-0009.md`) — 14-phase roadmap with verification
  matrix and exit criteria.
- **Added** git hooks (`scripts/hooks/pre-commit`, `scripts/hooks/pre-push`)
  for lint and test gating.

### docs(adr): ADR-0009 Inter-Worldline Communication

- **Added** ADR-0009: Inter-Worldline Communication, Frontier Transport, and
  Conflict Policy — formalizes message-passing-only communication between
  worldlines, frontier-relative patches, suffix transport as the replication
  primitive, four-dimensional footprint interference, explicit conflict
  surfacing over silent LWW, and the state-vs-history convergence separation.

### docs(adr): ADR-0008 Worldline Runtime Model

- **Added** ADR-0008: Worldline Runtime Model — formalizes writer/reader heads,
  SuperTick scheduling contract, three-domain boundaries (Echo Core, App, Janus),
  per-head seek/jump semantics, and the 8-step normative refactor plan.

### feat(warp-core): Wire up TTD domain logic from ttd-spec branch

- **Exported** `compute_tick_commit_hash_v2`, `compute_op_emission_index_digest`,
  and `OpEmissionEntry` from `warp-core` public API (previously `dead_code`).
- **Wired** `LocalProvenanceStore::append_with_writes()` to actually store
  atom writes instead of discarding them.
- **Added** `LocalProvenanceStore::atom_writes(w, tick)` — query atom writes
  for a specific tick (TTD "Show Me Why" provenance).
- **Added** `LocalProvenanceStore::atom_history(w, atom)` — causal cone walk
  using `out_slots` (Paper III `Out(μ)`) to filter ticks that wrote to
  the atom, with early termination at creation. O(history) scan, no reverse index.
- **Fixed** `LocalProvenanceStore::fork()` to copy `atom_writes` alongside
  patches, expected hashes, and outputs.
- **Added** 12 tests covering atom write storage, queries, filtering, fork,
  causal-cone walk, skip behavior, early termination, within-tick ordering,
  and `SlotId::Node(atom)` provenance path.

### test(echo-dind-harness): Golden-vector coverage for TTD digest surface

- **Added** `digest_golden_vectors.rs` — DIND-level golden-hash tests that
  exercise `compute_emissions_digest`, `compute_op_emission_index_digest`,
  and `compute_tick_commit_hash_v2` through warp-core's crate-root re-exports.
- **Pinned** 3 golden vectors: individual emission/op-emission-index digests
  plus a full hash chain (emissions → op-index → tick-commit). Any wire format
  drift in the public digest surface is now caught outside module-local tests.

### fix(warp-core): Preserve within-tick write order in atom_history

- **Fixed** `atom_history()` within-tick write ordering: the backward tick
  walk collected per-tick writes in forward order, so the final `reverse()`
  flipped within-tick execution sequence. Iterate `tick_writes.iter().rev()`
  so the global reverse restores original order.
- **Fixed** creation truncation: if a single tick had `[create, mutate]` for
  the same atom, forward iteration hit `is_create()` first and returned early,
  losing the subsequent mutation.
- **Updated** `fork()` rustdoc to mention `atom_writes` in the copied fields.
- **Documented** `append_with_writes()` invariant: atom writes must reference
  atoms declared in `patch.out_slots` for `atom_history()` visibility.

### fix: Ban-globals regex for macro patterns

- **Fixed** `scripts/ban-globals.sh`: the `\bthread_local!\b` and
  `\blazy_static!\b` patterns never matched because `!` is not a word
  character in ripgrep regex, making the trailing `\b` impossible. Use
  escaped `\!` without trailing `\b`.
- **Added** `.ban-globals-allowlist` to exempt `warp-wasm/src/lib.rs`
  (WASM boundary legitimately needs module-scoped `thread_local` +
  `install_kernel`).

### fix(wasm): Address PR review feedback

- **Fixed** `init()` now returns real 32-byte `state_root` and
  `commit_id` hashes from the freshly constructed kernel instead of
  empty vecs.
- **Fixed** `WarpKernel::with_engine()` auto-registers `sys/ack_pending`
  if absent, silently ignoring duplicates. Prevents `ENGINE_ERROR` on
  first dispatched intent when callers forget to register it.
- **Removed** unnecessary `#[allow(dead_code)]` on public
  `WarpKernel::with_engine()` method.
- **Removed** redundant explicit type annotation on `get_registry_info()`.
- **Fixed** broken `RegistryInfo` rustdoc link after import removal.

### fix: Resolve pre-existing clippy warnings across workspace

- **Fixed** `warp-core`: cfg-gate footprint enforcement internals (`FootprintGuard`,
  `OpTargets`, `op_write_targets`, etc.) so they compile out cleanly under
  `unsafe_graph` without dead-code warnings.
- **Fixed** `warp-core`: add `#[allow(unused_mut)]` on cfg-conditional mutation
  in `engine_impl.rs`.
- **Fixed** `echo-wasm-bindings`: restructure `TtdController` WASM bindings to
  use per-method `wasm_bindgen` with `JsValue`/`JsError` wrappers instead of
  blanket `wasm_bindgen` on the impl block (fixes trait bound errors under
  `--all-features`).
- **Fixed** `echo-scene-codec`: add clippy allow attributes to test module for
  `expect_used`, `unwrap_used`, and `float_cmp`.
- **Fixed** `warp-core` tests: move enforcement-only imports into cfg-gated
  `mod enforcement` in `parallel_footprints.rs`.

### fix(wasm): Validate intent envelopes, enforce envelope construction, add trait defaults

- **Fixed** `dispatch_intent` now validates the EINT envelope before passing
  bytes to the engine, returning `INVALID_INTENT` (code 2) for malformed
  envelopes instead of forwarding garbage.
- **Added** `Display` impl for `EnvelopeError` (no_std compatible).
- **Changed** `OkEnvelope` and `ErrEnvelope` fields to private with `::new()`
  constructors, enforcing correct `ok` field values at compile time.
- **Added** default implementations for `KernelPort::execute_query` and
  `KernelPort::render_snapshot` returning `NOT_SUPPORTED`, making future
  trait evolution non-breaking.
- **Updated** SPEC-0009 error code 2 description and versioning notes.

### feat(wasm): Ship reusable app-kernel WASM boundary with real exports (ECO-001)

- **Added** `KernelPort` trait to `echo-wasm-abi` — app-agnostic byte-level
  boundary contract for WASM host adapters. Includes ABI response DTOs
  (`DispatchResponse`, `StepResponse`, `HeadInfo`, `DrainResponse`,
  `RegistryInfo`), error codes, and CBOR wire envelope types.
- **Added** `WarpKernel` to `warp-wasm` (behind `engine` feature) — wraps
  `warp-core::Engine` implementing `KernelPort`. Registers `sys/ack_pending`
  system rule, provides deterministic tick execution.
- **Replaced** all placeholder WASM exports with real implementations:
  `dispatch_intent`, `step`, `drain_view_ops`, `get_head`, `snapshot_at`,
  `get_registry_info`, and handshake metadata getters now return live data.
- **Added** `init()` export for kernel initialization; calling exports before
  init returns structured error (no panics).
- **Added** CBOR success/error envelope protocol (`{ ok: true/false, ... }`)
  for all `Uint8Array` returns.
- **Added** `install_kernel()` public API for app-agnostic kernel injection.
- **Added** SPEC-0009 documenting ABI v1 contract, wire encoding, error
  codes, versioning strategy, and migration notes.
- **Added** 14 conformance tests covering dispatch, step, drain, snapshot,
  determinism, error paths, and handshake metadata.
- `execute_query` and `render_snapshot` honestly report `NOT_SUPPORTED`
  (error code 5) until the engine query dispatcher lands.

### Refactor: Retire BOAW/JITOS/Continuum codenames

- **Renamed** `warp_core::boaw` module to `warp_core::parallel` — all import
  paths, re-exports, and doc comments updated.
- **Renamed** 14 `boaw_*` integration test files to `parallel_*`, updated test
  harness types (`BoawScenario` → `ParallelScenario`, `BoawTestHarness` →
  `ParallelTestHarness`, etc.) and string literals.
- **Renamed** `boaw_baseline` benchmark to `parallel_baseline`, updated
  `warp-benches/Cargo.toml` target.
- **Annotated** 5 ADR files with deprecation notice (filenames preserved as
  historical records).
- **Updated** book/LaTeX sections, specs, guides, and source comments to
  replace BOAW references with `parallel`.
- **Replaced** "Echo/JITOS" → "Echo" in `echo-wasm-bindings`, `echo-wasm-abi`,
  and `spec-000-rewrite` crate metadata and READMEs.
- **Replaced** "JITOS Engineering Standard" → "Echo Engineering Standard" in
  `METHODOLOGY.md`.
- **Replaced** "Echo / Continuum" → "Echo" in ADR-0007.

### Chore: Archive ~90 stale docs, restructure docs-index

- **Archived** 6 entire directories to `docs/archive/`: `notes/`, `plans/`,
  `tasks/`, `rfc/`, `memorials/`, `jitos/` (session artifacts, completed work).
- **Archived** `docs/study/` (51 files: LaTeX papers, build artifacts, tour
  materials) to `docs/archive/study/`.
- **Archived** 4 completed DIND mission docs from `docs/determinism/` and the
  superseded `ECHO_ROADMAP.md` from `docs/ROADMAP/`.
- **Archived** 18 stale loose docs from `docs/` root: `AGENTS.md`,
  `ISSUES_MATRIX.md`, `code-map.md`, `phase1-plan.md`,
  `roadmap-mwmr-mini-epic.md`, `branch-merge-playbook.md`,
  `testing-and-replay-plan.md`, `runtime-diagnostics-plan.md`,
  `telemetry-graph-replay.md`, `warp-demo-roadmap.md`,
  `warp-runtime-architecture.md`, `capability-ownership-matrix.md`,
  `ROLLBACK_TTD.md`, `aion-papers-bridge.md`, `rust-rhai-ts-division.md`,
  `hash-graph.md`, `two-lane-abi.md`, `diagrams.md`.
- **Archived** dead redirect `guide/collision-tour.md` (both targets missing).
- **Rewrote** `docs/meta/docs-index.md`: curated golden-path index with clear
  separation of implemented specs, vision specs (unimplemented), ADRs, and
  archive. Removed broken links and stale entries.

### Docs: Fix violations found during docs sweep

- **Fixed** `CONTRIBUTING.md`: Rust version 1.71.1 → 1.90.0, `AGENTS.md` path
  to `docs/AGENTS.md`, `reference/typescript/` → `packages/` and `apps/`,
  commit message guidance aligned with conventional commits.
- **Fixed** `.devcontainer/post-create.sh`: reads toolchain version from
  `rust-toolchain.toml` instead of hardcoding 1.71.1, removed stale
  `rmg-core` crate reference.
- **Fixed** `warp-wasm/README.md`: corrected dependency claim from `warp-core`
  to `echo-wasm-abi` + `echo-registry-api`.
- **Fixed** `echo-session-proto/README.md`: removed broken `docs/tex/` paths,
  pointed to `docs/js-cbor-mapping.md` and book sections instead.
- **Fixed** `ttd-browser/README.md`: removed broken `docs/plans/ttd-app.md`
  reference and nonexistent `ttd-controller` crate mention.
- **Fixed** `NOTICE`: copyright year 2025 → 2025–2026, SPDX identifier aligned
  to `LicenseRef-MIND-UCAL-1.0`.
- **Fixed** ROADMAP priority mismatch: 5 milestone READMEs aligned from P2 → P3
  to match the parent index. Proof Core status downgraded from "Verified" to
  "In Progress" (Docs Polish feature still incomplete).
- **Fixed** `guide/cargo-features.md`: removed nonexistent `spec-000-rewrite`
  crate section.
- **Fixed** `guide/course/glossary.md`: corrected `ViolationKind` variant
  `AdjacencyViolation` → `OpWarpUnknown` with full variant names.
- **Fixed** `BENCHMARK_GUIDE.md`: updated "CI Integration (Future)" section to
  reflect existing G3 perf gate.
- **Fixed** `echo-session-client` and `echo-session-service` Cargo.toml
  descriptions: removed stale "(skeleton)" qualifier.

### Fix: Task list guard CI failure

- **Fixed** `scripts/check_task_lists.sh`: accept file arguments for testability;
  fall back to built-in `FILES` array when none are given.
- **Fixed** `scripts/tests/check_task_lists_test.sh`: updated tests to pass file
  arguments to the checker and match current output messages. Tests were broken
  after the `FILES` array was emptied when task lists were archived.

### Chore: Clean up root directory

- **Removed** stale root-level ADR duplicates (`ADR-0003` through `ADR-0006`);
  canonical copies already exist in `docs/adr/`.
- **Removed** completed one-shot plan `MERGE_TTD_BRANCH_PLAN.md`.
- **Moved** determinism docs (`DETERMINISM-AUDIT.md`, `DIND-MISSION*.md`) to
  `docs/determinism/`.
- **Moved** task trackers (`TASKS.md`, `TASKS-DAG.md`, `WASM-TASKS.md`) to
  `docs/tasks/`.
- **Moved** `ECHO_ROADMAP.md` to `docs/ROADMAP/`, `COMING_SOON.md` to
  `docs/plans/`, `AGENTS.md` to `docs/`.
- **Deleted** untracked junk files (`paper-7eee.log`, `dind-report.json`,
  `.DS_Store`).
- **Archived** 14 superseded/completed docs: redirect stubs removed,
  canonical content already in `docs/archive/`. Updated cross-references
  in `docs-index.md`, `code-map.md`, `DETERMINISTIC_MATH.md`, and
  `warp-geom/README.md`.
- **Added** `docs/archive/README.md` defining archive policy.

### CI: G3 perf regression gate (#280)

- **CI:** G3 perf regression gate now compares criterion benchmark output
  against a git-tracked `perf-baseline.json` and fails if any benchmark
  regresses beyond 15% (configurable via `--threshold`). Structured
  `perf-report.json` artifact uploaded alongside raw `perf.log`.
- **CI:** New `perf-baseline-update.yml` workflow auto-generates baseline
  update PRs on main pushes that touch Rust sources.
- **Scripts:** Added `check_perf_regression.cjs` (gate comparison) and
  `generate_perf_baseline.cjs` (baseline generation from bencher output).

### Docs: Allowlist governance (#287)

- **Policy:** Added "Determinism Allowlist Governance" section to
  `docs/RELEASE_POLICY.md` documenting acceptable exemption criteria,
  approval requirements, and audit cadence for `.ban-nondeterminism-allowlist`.
- **Scripts:** Added cross-reference from `ban-nondeterminism.sh` header to
  the governance policy.

### Docs Polish (#41)

- **License:** Renamed SPDX identifier `MIND-UCAL-1.0` →
  `LicenseRef-MIND-UCAL-1.0` across 328 files to comply with SPDX
  Appendix IV (custom identifiers must use `LicenseRef-` prefix).
  Updated `ensure_spdx.sh` tooling and pre-commit hook accordingly.
- **Fix:** Fixed radix sort scope pair index inversion in `scheduler.rs`
  `bucket16()`. LSD passes were processing scope bytes MSB-first instead of
  LSB-first, causing the radix-sort path (n > 1024) to produce a different
  ordering than the comparison-sort path (n ≤ 1024). Added 3 property tests:
  `proptest_drain_matches_btreemap_reference` (fuzzes both sort paths),
  `proptest_insertion_order_independence`, and `threshold_boundary_determinism`.
- **Spec:** Replaced "Theorem A" in `spec-mwmr-concurrency.md` with the
  formal name from Paper II: "Skeleton-plane Tick Confluence theorem (§6,
  Thm. 6.1)".
- **Spec:** Changed `<i>Alea iacta est</i>` to semantic HTML in
  `memorials/2026-01-18-phase4-rubicon.md` (foreign phrase italics).
- **Spec:** Resolved 4 CRITICAL CodeRabbit items: normative frame ordering
  rule in `spec-editor-and-inspector.md` (stable sort by `(tick, frameType)`,
  UTF-8 lexicographic, insertion-order tie-break); added `getNode()` to
  `BridgeContext` in `spec-temporal-bridge.md` with `NodeId` disambiguation
  note (timeline hash vs WARP graph `u64`); defined `world:config` capability
  in `spec-capabilities-and-security.md` and removed "not yet defined" warning
  from `spec-runtime-config.md`; verified `SweepProxy` rename in
  `spec-knots-in-time.md`. Also changed `producer` return type from `object`
  to `unknown` in `spec-editor-and-inspector.md`.
- **Spec:** Rewrote `spec-branch-tree.md` to resolve all 10 CodeRabbit
  review items. Key changes: formal `ReadKey`/`WriteKey`/`QualifiedKey`
  type definitions with ECS-layer layering rationale; `MergeStrategyId`
  as extensible namespaced string registry; extracted `TimelineNodeCore`
  hashable subset and replaced broken hash formula; unified `parents[]`
  replacing `parentId + mergeParents?`; renamed entropy heuristic to
  "branch strain" (distinguished from Aion Boltzmann entropy); defined
  `WorldView`, `GCPolicy`, three explicit GC modes with transitive pin
  semantics, domain-separated seed derivation, layered causal-edge
  semantics, `CapabilityAssertion` with forward reference to capabilities
  spec, and `StabilityObserver` lifecycle.

- **Polish:** Resolved remaining 13 Tier 2 CodeRabbit items (all 66 complete):
  session token format (HMAC-SHA256) and filter semantics in
  `spec-editor-and-inspector.md`; signing canonicalization subsection with
  8-field byte layout in `spec-warp-confluence.md`; breaking-change criteria
  and deprecation timeline in `spec-world-api.md`; `BlockManifest` section
  encoding in `spec-serialization-protocol.md`; radix sort internals
  documentation in `scheduler-optimization-followups.md`; enum style
  unification in SPEC-0002; `cargo metadata` provenance command in
  `cargo-features.md`; expanded serde acceptance criteria in
  `issue-canonical-f32.md`.
- **Polish:** Resolved 12 Tier 1 CodeRabbit items: `remain disjoint` →
  `are non-conflicting` in SPEC-0003, tightened subnormal definition in
  `DETERMINISTIC_MATH.md`, added Aion inline definition in
  `branch-merge-playbook.md`, converted numbered narrative to bullets in
  `spec-time-streams-and-wormholes.md`, verified 3 already-correct items
  (serialization link, admission MUST language, musings blank line),
  dismissed 3 prettier-enforced formatting items.
- **Spec:** Resolved all 3 TODO comments in `spec-scheduler.md`:
  bidirectional dependency resolution rules, cleaner `registerSystem`
  pseudo-code, and a formal resource conflict detection model aligned with
  warp-core's `Footprint`. Replaced `ComponentSignature` with `SystemFootprint`
  (reads/writes/exclusiveTags).
- **Review:** Addressed 69 CodeRabbit review comments across 37 files:
    - **xtask:** Cross-platform `command_exists`, annotated UTF-8 errors, warned
      on non-UTF-8 path drops, simplified `has_extension`, fixed doc comment.
    - **Specs:** Hardened 15 spec docs — added error handling for branch-tree
      commit conflicts, defined equality predicates, bounded parent counts in
      merkle-commit, specified canonical field ordering, fixed broken cross-refs
      and link styles, added validation rules and error codes to runtime-config,
      unified `BranchId`/`KairosBranchId`, clarified signing payloads.
    - **Notes/Archive:** Corrected O(n log n) cost attribution in scheduler
      notes, fixed stale code references and branch names, expanded commit
      hashes, added provenance blocks.
    - **Docs:** Fixed ADR-0004 placeholder, normalized titles, corrected
      dependency direction in ISSUES_MATRIX, fixed emphasis style, consolidated
      repetitive bullets, added cargo-features provenance note, fixed heading
      levels in warp-math-claims, fixed workflow artifacts in mat-bus-finish RFC.
- **Archive:** Moved 6 superseded docs to `docs/archive/` with redirect stubs
  (`spec-deterministic-math.md`, `spec-geom-collision.md`,
  `notes/scheduler-radix-optimization.md`, `notes/xtask-wizard.md`,
  `plans/cross-warp-parallelism.md`, `plans/BOAW-tech-debt.md`).
- **Consolidate:** Added "Docs Map" callouts to `SPEC_DETERMINISTIC_MATH.md`
  and `DETERMINISTIC_MATH.md` linking all 5 docs in the deterministic math
  cluster. Updated `scheduler.md` Quick Map with status labels.
- **Fix:** Repaired 13 broken cross-references (`docs/specs/` -> `docs/spec/`,
  `memorial.md` -> `memorials/...`, `streams-inspector-frame.md` ->
  `streams-inspector.md`, `docs/spec/SPEC-0004...` prefix, archived file
  image paths, nonexistent README link).
- **New:** `cargo xtask lint-dead-refs` — scans `docs/` for broken markdown
  cross-references. Handles relative paths, VitePress root-relative links,
  and `docs/public/` asset resolution. Use `--all` to also check non-markdown
  file references (images, HTML).
- **New:** `cargo xtask markdown-fix` — auto-fixes common markdown lint
  violations: SPDX header repair, prettier formatting, and markdownlint
  `--fix`. Supports `--no-prettier` and `--no-lint` flags.
- **New:** `cargo xtask docs-lint` — combined pipeline that runs
  `markdown-fix` followed by `lint-dead-refs`. Single command for full docs
  hygiene.
- **New:** Configuration reference (`docs/guide/configuration-reference.md`)
  covering engine parameters, protocol constants, and environment variables.
- **New:** Cargo feature flags reference (`docs/guide/cargo-features.md`)
  covering all 19 features across 11 crates.
- **Fix:** `cargo xtask lint-dead-refs` now uses `pulldown-cmark` for link
  extraction (handles title text, balanced parens, angle-bracket URLs) and
  separates scan scope from VitePress docs root. Includes 10 unit tests.
- **Fix:** `det_fixed` correctly documented as a behavioral switch in
  `cargo-features.md`; `worker_count` default now shows `NUM_SHARDS` cap.
- **Fix:** All file collection in xtask now uses `git ls-files` instead of
  filesystem walks (skips build artifacts like `.vitepress/dist/`).
- **Update:** Archival stubs enriched with date, reason, and PR metadata.
  Draft spec (`spec-scheduler.md`) marked with `[!CAUTION]` disclaimer and
  TODO markers for unspecified sections.
- **Update:** Math code fences converted from `text` to `math` with proper
  LaTeX markup across `THEORY.md`, `SPEC-0001`, and scheduler notes.
- **Fix:** Archived `cross-warp-parallelism.md` annotated with implementation
  traceability and `WorkUnit` struct deviation (no `shard_id` in actual code).
- **Fix:** Archived `BOAW-tech-debt.md` checklists annotated as frozen with
  tracking-moved callout pointing to `TECH-DEBT-BOAW.md`.
- **Fix:** Archived `scheduler-radix-optimization.md` canonical order clarified
  and code-reference staleness disclaimer made visible.
- **New:** `.coderabbit.yaml` — excludes `docs/archive/**` from CodeRabbit
  reviews (frozen historical records generate low-value feedback).
- **Update:** README determinism claims link, reference docs section,
  docs-index entries, docs-audit log.

### Added — Proof Core (P1 Milestone)

- **Determinism Claims v0.1:** New `docs/determinism/DETERMINISM_CLAIMS_v0.1.md`
  documenting five determinism claims (DET-001 through DET-005) covering
  static inspection, float parity, parallel execution, trig oracle golden
  vectors, and torture-rerun reproducibility.
- **Trig Golden Vectors (DET-004):** New test
  `crates/warp-core/tests/trig_golden_vectors.rs` with a 2048-sample golden
  binary (`testdata/trig_golden_2048.bin`) that locks down `dfix64` sin/cos/tan
  outputs across platforms. Runs on Linux and macOS in CI.
- **Torture Rerun Script (DET-005):** `scripts/torture-100-reruns.sh` — turnkey
  repro script that runs 100 sequential simulations and asserts identical hashes.
- **CI Trig Oracle Gate:** Added trig golden vector tests to
  `.github/workflows/det-gates.yml` for both Linux and macOS runners, with log
  artifacts uploaded alongside existing determinism artifacts.
- **CLAIM_MAP.yaml:** Added DET-004 and DET-005 entries with required evidence
  pointers and owner roles.
- **Evidence Generator:** Wired DET-004 and DET-005 into
  `scripts/generate_evidence.cjs` so the evidence policy cross-check passes.
- **Ban-Nondeterminism Allowlist:** Added `trig_golden_vectors.rs` to
  `.ban-nondeterminism-allowlist` (test-only `std::fs` for reading golden
  vector binaries).

### Changed — Roadmap

- Updated `docs/ROADMAP/proof-core/README.md`: checked off P1 exit criteria,
  marked milestone as "In Progress".
- Resequenced roadmap phases: P0 verified, P1→P2→P3 ordering clarified.

### Fixed — Code Review (PR #291)

- **Bench Recursive Scanning:** `collect_criterion_results` now walks
  directories recursively, correctly finding grouped (`benchmark_group`) and
  parameterised (`BenchmarkId`) benchmarks that Criterion stores in nested
  directories (e.g. `group/bench/new/estimates.json`).
- **Bench Regex Filter:** Post-filter now uses `regex::Regex` to match
  Criterion's own regex semantics instead of substring `contains`. Filters
  with anchors or metacharacters (e.g. `^hotpath$`) now work correctly.

### Fixed — Self-Review (PP-1 Branch)

- **Stale `warp-ffi` References:** Removed deleted crate from git hooks
  (`pre-push-parallel`, `pre-push-sequential`), `warp-core/README.md`, and
  `AGENTS.md`. Only historical references in CHANGELOG and TASKS-DAG remain.
- **Broken Spec Paths:** Fixed `docs/specs/` → `docs/spec/` in two acceptance
  criteria in `docs/ROADMAP/backlog/security.md`.
- **`emit()` Error Propagation:** Changed `output::emit()` to return
  `Result<()>` instead of silently printing to stderr on serialization failure.
  All call sites (`bench.rs`, `verify.rs`, `inspect.rs`) now propagate with `?`.
- **SPEC-0005 Clarity:** Bound loop index variable in BTR verification algorithm
  (H-3); documented missing-producer behavior in `derive()` (H-4); clarified
  multi-producer vs. most-recent-producer semantics between
  `build_provenance_graph()` and `derive()` (M-4); added
  `canonical_state_hash()` cross-reference (M-5); specified composition error
  semantics (M-6); added set semantics for `Out(μ)`/`In(μ)` (M-8); expanded
  `ProvenanceNode` constructor in pseudocode (L-10); documented empty derivation
  graph semantics (L-11); formalized identity composition (L-12); defined
  `H(P)` notation in example (L-13); added Paper III citation (L-14).
- **`format_duration()` Infinity:** Added `is_infinite()` check alongside
  `is_nan()` so `f64::INFINITY` returns "N/A" instead of formatting as seconds.
- **Safe `edge_ix` Cast:** Replaced `as usize` with `usize::try_from()` in
  `inspect.rs` tree builder to guard against truncation on 32-bit targets.
- **Bench Test Ordering:** Added positional assertion ensuring `--` precedes
  the filter pattern in `build_bench_command`.
- **Bench Empty Warning:** Added stderr warning when no benchmark results found.
- **WSC Loader Warnings:** Warning messages now include entity IDs (first 4
  bytes hex) for easier debugging.
- **Inspect Docstring:** Changed "Prints" to "Displays" in module docstring.
- **`TREE_MAX_DEPTH` Doc:** Added doc comment explaining the depth limit's
  purpose.
- **Fragile `len() - 1`:** Changed `i == node.children.len() - 1` to
  `i + 1 == node.children.len()` to avoid underflow on empty children (though
  the loop guards against this, the pattern is safer).

### Fixed — Developer CLI (`echo-cli`)

- **Bench Filter:** `echo-cli bench --filter <pattern>` now passes the filter
  as a Criterion regex (`-- <pattern>`) instead of a `--bench` cargo target
  selector. Previous behavior would look for a bench _target_ named after the
  pattern rather than filtering benchmarks by regex.
- **Verify Expected Hash:** `--expected` now correctly reports "unchecked" for
  warps 1+ instead of silently claiming "pass". Emits a stderr warning when
  `--expected` is used with multi-warp snapshots. Text and JSON output now
  use consistent lowercase status values.
- **Unused Dependency:** Removed `colored = "2"` from `warp-cli` (declared but
  never imported).
- **Output Hardening:** `emit()` no longer panics on JSON serialization failure;
  falls back to stderr. Bench exit status now reports Unix signal numbers
  instead of a misleading `-1`.
- **Error Handling:** `collect_criterion_results` now logs a warning on
  unparseable `estimates.json` instead of silently skipping. `format_duration`
  returns "N/A" for NaN/negative values. `att_row_to_value` warns on missing
  blob data instead of silent fallback.
- **Dead Code:** Replaced blanket `#![allow(dead_code)]` on `lib.rs` with
  targeted `#[allow(dead_code)]` on the `output` module only.
- **Man Page Headers:** Subcommand man pages now use prefixed names
  (`echo-cli-bench`, `echo-cli-verify`, `echo-cli-inspect`) in `.TH` headers
  instead of bare subcommand names.
- **Visibility:** Narrowed all non-API structs and functions from `pub` to
  `pub(crate)` in bench, verify, inspect, and wsc_loader modules. Only
  `cli.rs` types remain `pub` (required by xtask man page generation).
- **cargo-deny:** Fixed wildcard dependency error for `warp-cli` in
  `xtask/Cargo.toml` by adding explicit `version = "0.1.0"` alongside
  the path override.
- **Man Page Cleanup:** `cargo xtask man-pages` now removes stale
  `echo-cli*.1` files before regeneration so the output directory is an
  exact snapshot.

### Fixed — Code Review (PR #289, Round 2)

- **Inspect Tree Warp Identity:** Multi-warp snapshots now label each tree
  section with its warp index (`Tree (warp 0):`, `Tree (warp 1):`) instead of
  flattening all trees into a single unlabeled `Tree:` section.
- **WSC Loader Attachment Checks:** Replaced `debug_assert!` with runtime
  warnings for attachment multiplicity violations. Previously, release builds
  silently dropped extra attachments; now emits a warning to stderr.
- **Test Naming:** Renamed `tampered_wsc_fails` to `tampered_wsc_does_not_panic`
  to accurately reflect the test's behavior (no assertion, just no-panic guard).
- **Test Coverage:** Added `roundtrip_with_edge_attachments` and
  `roundtrip_with_descend_attachment` tests to `wsc_loader.rs`, covering
  previously untested code paths.
- **SPEC-0005 `global_tick` Invariant:** Reworded from `patches[i].global_tick == i`
  to correctly state contiguity relative to the payload's start tick, since
  payloads can begin at any absolute tick via `from_store(store, wl, 5..10)`.
- **SPEC-0005 BTR Verification:** Fixed step 5 of the verification algorithm
  to reference the actual hash formula from §5.4 instead of a nonexistent
  `parents` field.
- **SPEC-0005 Derivation Algorithm:** Fixed backward-cone traversal that dropped
  transitive dependencies. The original filter checked the root query slot at
  every hop; now accepts all frontier nodes unconditionally (they are already
  known-causal) and traces all `in_slots` backward.
- **Stale `warp-ffi` References:** Removed dead `warp-ffi` entry from
  `det-policy.yaml`, C ABI text from `phase1-plan.md`, and stale CLI names
  from `rust-rhai-ts-division.md`.

### Fixed — Docs & CI

- **TASKS-DAG Spec Path:** `SPEC-PROVENANCE-PAYLOAD.md` →
  `SPEC-0005-provenance-payload.md` in sub-task title and AC1 (two
  occurrences). Same stale path fixed in ROADMAP backlog `security.md`.
- **SPEC-0005 Byte Counts:** Domain separation tag sizes corrected:
  `echo:provenance_payload:v1\0` = 27 bytes (was 28),
  `echo:provenance_edge:v1\0` = 24 bytes (was 25).
- **Project Tour:** Updated `warp-cli` description from "Placeholder CLI home"
  to list actual subcommands (verify, bench, inspect).
- **CI Formatting:** Removed stray blank line between warp-geom and warp-wasm
  rustdoc steps in `ci.yml`.

### Added — Developer CLI (`echo-cli`)

- **CLI Scaffold (`warp-cli`):** Replaced placeholder with full `clap` 4 derive
  subcommand dispatch. Three subcommands: `verify`, `bench`, `inspect`. Global
  `--format text|json` flag for machine-readable output.
- **Verify Subcommand:** `echo-cli verify <snapshot.wsc>` loads a WSC snapshot,
  validates structural integrity via `validate_wsc`, reconstructs the in-memory
  `GraphStore` from columnar data, and computes the state root hash. Optional
  `--expected <hex>` flag compares against a known hash.
- **WSC Loader:** New `wsc_loader` module bridges WSC columnar format to
  `GraphStore` — the inverse of `warp_core::wsc::build_one_warp_input`.
  Reconstructs nodes, edges, and attachments from `WarpView`.
- **Bench Subcommand:** `echo-cli bench [--filter <pattern>]` shells out to
  `cargo bench -p warp-benches`, parses Criterion JSON from
  `target/criterion/*/new/estimates.json`, and renders an ASCII table via
  `comfy-table`. Supports `--format json` for CI integration.
- **Inspect Subcommand:** `echo-cli inspect <snapshot.wsc> [--tree]` displays
  WSC metadata (tick, schema hash, warp count), graph statistics (node/edge
  counts, type breakdown, connected components via BFS), and optional ASCII
  tree rendering depth-limited to 5 levels.
- **Man Pages:** Added `clap_mangen`-based man page generation to `xtask`.
  `cargo xtask man-pages` generates `docs/man/echo-cli.1`,
  `echo-cli-verify.1`, `echo-cli-bench.1`, `echo-cli-inspect.1`.

### Removed

- **`warp-ffi` crate deleted:** The C ABI integration path (`crates/warp-ffi`)
  has been removed. The C ABI approach was abandoned in favor of Rust plugin
  extension via `RewriteRule` trait registration and Rhai scripting. See
  TASKS-DAG.md #26 (Graveyard). This is a **BREAKING CHANGE** for any
  downstream code that depended on the C FFI surface.

### Added — Provenance Payload Spec (PP-1)

- **SPEC-0005:** Published `docs/spec/SPEC-0005-provenance-payload.md` mapping
  Paper III (AION Foundations) formalism to concrete Echo types. Defines four
  new types (`ProvenancePayload`, `BoundaryTransitionRecord`, `ProvenanceNode`,
  `DerivationGraph`), wire format with CBOR encoding and domain separation tags,
  two worked examples (3-tick accumulator, branching fork), bridge to existing
  `ProvenanceStore`/`PlaybackCursor` APIs, and attestation envelope with SLSA
  alignment.

### Fixed (CI)

- **Evidence Derivation:** Replaced artifact-directory-presence check for `DET-001` with
  structured parsing and validation of `static-inspection.json`; `FAILED` static inspections
  now correctly yield `UNVERIFIED` evidence instead of relying solely on artifact existence.
  Adds `source_file`, `source_status`, and optional `error` fields to DET-001 evidence.
- **Evidence Script Hardening:** Added TypeError guard on `generateEvidence` input,
  try/catch with `process.exit(1)` in CLI mode, truncated log interpolations to
  200 chars in `checkStaticInspection`, harmonized parameter naming, and tightened
  JSDoc return types to `'VERIFIED'|'UNVERIFIED'` union.

### Fixed (Docs)

- **Docs Build:** Rewrote `ADR-0007-impl.md` from a 3185-line raw conversation
  transcript into a proper 13-section ADR document. The Vue template compiler
  was crashing on bare Rust generics (`BTreeMap<NodeId, NodeRecord>`, etc.)
  outside fenced code blocks. The new document preserves all architectural
  knowledge as a structured implementation companion to ADR-0007.
- **Stale Hash Domain:** Updated three stale `DIND_STATE_HASH_V2` references in
  `graph.rs` doc comment and `ADR-0007-impl.md` §2.1/§7 to match the actual
  domain prefix `echo:state_root:v1` defined in `domain.rs`. Renamed the
  adjacent `V2 Changes` subsection to `Layout Notes` to remove versioning
  ambiguity.
- **Module Count:** Fixed off-by-one module count in `ADR-0007-impl.md` metadata
  and §1 prose (36 → 37) and added qualifier noting tables cover key modules only.
- **Stale Design Doc:** Deleted `docs/WARP-GRAPH.md` (1,219-line chat transcript
  fully superseded by `ADR-0007-impl.md` and `crates/warp-core/src/wsc/`).
  Extracted all-zero-key caveat into ADR §9.6 and `save_wsc()` convenience
  wrapper gap into `TASKS-DAG.md` backlog before removal.

## [0.1.3] — 2026-02-21

### Fixed (Sprint S1)

- **CI Security:** Hardened `det-gates` workflow against script injection by using
  environment variables for all `github.*` interpolations (branch refs, SHA,
  run ID, event name).
- **WASM Reproducibility:** Implemented bit-exact reproducibility checks (G4)
  for `ttd-browser` WASM using hash comparison of clean isolated rebuilds.
- **Static Inspection:** Added automated CI guard for `DET-001` covering all 14
  DET_CRITICAL crate paths (expanded from `echo-wasm-abi` only). Report now
  conditional on check outcome (PASSED/FAILED).
- **Evidence Validation:** Made artifact presence checks in `validate-evidence`
  conditional on classification tier; added `det-macos-artifacts` check;
  `run_reduced` and `DET_NONCRITICAL` paths no longer hard-fail.
- **Policy Classification:** Promoted `warp-benches` from DET_NONCRITICAL to
  DET_IMPORTANT so benchmark crate changes trigger reduced gates.
- **Benchmark Correctness:** Replaced `let _ =` with `.unwrap()` on all
  `bus.emit()` calls; migrated `iter_with_setup` to `iter_batched`.
- **CBOR Robustness:** Expanded negative security tests for `ProjectionKind`
  and `LabelAnchor` enum tags and optimized `MAX_OPS` boundary check.
- **Evidence Integrity:** Enhanced `generate_evidence.cjs` and `validate_claims.cjs`
  with stricter semantic validation (SHAs, run IDs) and artifact existence checks.
- **Script Quality:** Replaced `process.exit(1)` with `throw` in
  `classify_changes.cjs`; removed dead import; exported functions for testing.
- **Governance:** Moved `sec-claim-map.json` to `docs/determinism/`, formalized
  gate states in `RELEASE_POLICY.md`, tightened claim statements in
  `CLAIM_MAP.yaml`.
- **CI Permissions:** Added `permissions: contents: read` to `det-gates.yml`
  for least-privilege workflow execution.
- **CI Robustness:** Made ripgrep install idempotent; gated `validate-evidence`
  on `classify-changes` success; invoked CJS scripts via `node` for
  cross-platform portability.
- **Evidence Validation:** Relaxed `commit_sha` check to accept `local` sentinel
  for local development; exported `generateEvidence` and `validateClaims`
  functions for unit testing (#286).
- **Claims Precision:** Sharpened `PRF-001` statement to reference specific
  Criterion benchmark rather than generic threshold language.
- **Backlog:** Added five `TASKS-DAG.md` items: BLD-001 claim gap, macOS parity
  claim, CI concurrency controls, expanded script test coverage, and
  `det-policy.yaml` path simplification.
- **Evidence Completeness:** Added `REPRO-001` claim for G4 build reproducibility
  to `CLAIM_MAP.yaml` and wired into `generate_evidence.cjs`.
- **Script Hardening:** Added `Array.isArray` guard for `required_gates` in
  `validate_det_policy.cjs`; used explicit null/undefined check in
  `validate_claims.cjs` instead of falsy coercion.
- **Test Robustness:** Encoded all 5 CBOR fields in `reject_invalid_version`
  to prevent false passes from decoder field-read reordering.
- **Docs:** Added G3 staging-optional rationale in `RELEASE_POLICY.md`;
  merge-commit revert guidance and evidence packet filing in `ROLLBACK_TTD.md`;
  documented `tests/**`/`e2e/**` classification rationale in `det-policy.yaml`.
- **Gate Coverage:** Made G3 (perf-regression) run for all non-`run_none` paths,
  not just `run_full`. Ensures PRF-001 claim fires for DET_IMPORTANT changes
  (e.g., `warp-benches`). Moved `perf-artifacts` presence check to always-required.
- **Classification Precision:** Carved `tests/dind*` and `testdata/dind/**` out
  of the DET_NONCRITICAL `docs` catch-all into a dedicated `dind-tests-root`
  entry at DET_IMPORTANT, preventing gate evasion for DIND test modifications.
- **Policy Simplification:** Replaced 20+ explicit docs paths with `**` catch-all
  in `det-policy.yaml`; max-class semantics ensure higher-priority patterns win.
- **CI Concurrency:** Added `concurrency` block to `det-gates.yml` to cancel
  superseded runs on the same branch.
- **CI Robustness:** Added push-event empty changelist guard (defaults to full
  run).
- **Dynamic DETERMINISM_PATHS:** Replaced hardcoded crate list in
  `static-inspection` with `yq`/`jq` extraction from `det-policy.yaml`
  DET_CRITICAL entries, eliminating manual sync.
- **Evidence Sync Guardrails:** Added CI cross-check step validating claim IDs
  in `evidence.json` match `CLAIM_MAP.yaml` exactly; added `sec-claim-map.json`
  test ID existence verification against source.
- **macOS Parity Claim:** Added `DET-003` to `CLAIM_MAP.yaml` and
  `generate_evidence.cjs` for macOS-specific determinism verification.
- **Claims Precision:** Fixed REPRO-001 evidence type from `static_inspection`
  to `hash_comparison`; flattened verbose `required_evidence` syntax.
- **Test Assertions:** Strengthened `reject_invalid_enum_tags` test to assert
  specific error messages instead of bare `is_err()` checks.
- **CI Timeouts:** Added `timeout-minutes` to all `det-gates.yml` jobs to
  prevent hung jobs from burning the 6-hour GitHub default.
- **Classification Optimization:** Added early-exit in `classify_changes.cjs`
  when `maxClass` reaches `DET_CRITICAL` (guarded by `require_full_classification`).
- **Build Repro Fix:** Restored `rustup target add wasm32-unknown-unknown`
  in `build-repro` — required because `rust-toolchain.toml` pins a specific
  Rust version that overrides the `dtolnay/rust-toolchain` action's target.

## [0.1.2] — 2026-02-14

### Added — TTD Hardening Sprint S1 (Gates & Evidence)

- **Path-Aware CI Gates:** Implemented `det-policy.yaml` and `classify_changes.cjs`
  to classify workspace crates (DET_CRITICAL/IMPORTANT/NONCRITICAL) and drive
  selective CI gate triggering (G1-G4).
- **Hardening Gates (G1-G4):**
    - **G1 (Determinism):** Integrated float parity tests and the DIND (Deterministic
      Ironclad Nightmare Drills) suite on both Linux and macOS.
    - **G2 (Security):** Added negative security tests for the CBOR decoder
      (MAX_OPS, invalid versions/enums, truncated payloads).
    - **G3 (Performance):** Created `materialization_hotpath` Criterion benchmark
      in `warp-benches` to track materialization overhead.
    - **G4 (Build):** Added WASM build reproducibility checks verifying bit-exact
      artifacts across clean rebuilds.
- **Evidence Integrity:** Added `generate_evidence.cjs` and `validate_claims.cjs`
  to ensure all `VERIFIED` claims are backed by immutable CI artifacts (run IDs,
  commit SHAs).
- **Static Inspection:** Integrated `DET-001` automated static inspection into CI
  to verify zero-HashMap usage in deterministic guest paths.
- **Governance:** Published `RELEASE_POLICY.md` (staging/prod blockers) and
  `ROLLBACK_TTD.md` (commit-ordered rollback sequences).
- **Security Claim Mapping:** Exported `sec-claim-map.json` mapping decoder
  controls to explicit negative test cases.

### Added — Deterministic Scene Data (TTD)

- **Scene Rendering Port (`echo-scene-port`):** Defined the core data model for
  deterministic scene updates, including nodes, edges, labels, and camera state.
- **Scene Codec (`echo-scene-codec`):** Implemented a high-performance `minicbor`
  codec for `SceneDelta` serialization with strict validation.
- **Float Parity Proof:** Integrated a cross-language verification suite
  ensuring `canonicalize_f32` produces bit-identical results between Rust and
  JavaScript.
- **Scene Integrity Drills:** Added stress tests for atomic state mutations and
  robustness against truncated CBOR payloads.

### Added — TTD Protocol & Core Hardening

- **TTD Wire Protocols (v2):** Implemented high-integrity codecs for intents and
  receipts.
    - Added `EINT v2` (Intent Envelope) with Little-Endian fixed headers and
      BLAKE3 payload checksums.
    - Added `TTDR v2` (Tick Receipt Record) supporting full provenance
      commitments including state roots and emission digests.
    - Integrated "Header Integrity Drills" and a decoder fuzzer to ensure
      protocol robustness.
- **Provenance & Merkle Hardening:** Expanded core state model to support
  deterministic "Show Me Why" features.
    - Added `AtomWrite` records to track causal arrows from rules to state
      changes.
    - Implemented `compute_tick_commit_hash_v2`, binding schema identity,
      worldline history, and materialized emissions into a single Merkle root.
    - Added `TruthSink::clear_session` to ensure isolated and leak-free memory
      management for TTD sessions.

### Added — Determinism & Verification

- **DIND Phase 5 (The Shuffle):** Added robustness against insertion order and
  HashMap iteration leaks.
    - Implemented `echo-dind converge` command to verify that shuffles of
      commutative operations (e.g. disjoint `put_kv`) yield identical final state
      hashes.
    - Added randomized scenario generator (`scripts/bootstrap_randomized_order.mjs`)
      producing semantically equivalent transcripts via different orderings.
    - Added regression tests for Invariant A (Self-Consistency) and Invariant B
      (Convergence) in CI; see [issue #22](https://github.com/flyingrobots/echo/issues/22).
- **Domain-Separated Hash Contexts:** Added unique domain-separation prefixes
  to all core commitment hashes to prevent cross-context structural collisions.
    - `state_root` (graph hash), `patch_digest` (tick patch), and `commit_id` (Merkle
      root) now use distinct BLAKE3 domain tags (e.g. `echo:state_root:v1\0`).
    - `RenderGraph::compute_hash` (`echo-graph`) now uses its own domain tag,
      ensuring renderable snapshots cannot collide with engine state roots.
    - Added `warp_core::domain` module containing public prefix constants.
    - Integrated cross-domain collision tests into CI.
- **Benchmarks CI Integration:** The `warp-benches` package is now integrated
  into the CI compilation gate (`cargo check --benches`).

### Changed — Roadmap & Governance

- **Roadmap Refactor ("Sharpened" structure):** Migrated the flat roadmap into
  a 2-level hierarchy based on features and milestones.
    - Established a **WIP Cap policy**: maximum 2 active milestones and 3 active
      feature files per milestone to prevent context thrashing.
    - Added binary **Exit Criteria** to all milestone READMEs to ensure clear,
      objective completion signals.
    - Renamed milestones for clarity (e.g. `lock-the-hashes`, `first-light`,
      `proof-core`).
    - Audited and updated license headers (SPDX) and formatting (Prettier/MD028)
      across roadmap documents.

### Changed — Gateway Resilience (`echo-session-ws-gateway`)

- **Typed `HubConnectError` enum** replaces the opaque `HubConnectError(String)`.
  Four variants (`Timeout`, `Connect`, `Handshake`, `Subscribe`) carry structured
  context, and a `should_retry()` predicate is wired into the ninelives retry
  policy so future non-transient variants can short-circuit retries.
- **Hub observer task exits are surfaced** — the fire-and-forget
  `tokio::spawn` is wrapped in a watcher task that logs unexpected exits and
  panics at `warn!`/`error!` level, preventing silent observer disappearance.
- **`connect_failures` restored to per-attempt semantics** — the metric now
  increments on every failed connection attempt (1:1 with `connect_attempts`),
  not once per exhausted retry burst. This preserves dashboard/alerting accuracy
  during prolonged hub outages.

- **Hub observer reconnect** now uses `ninelives` retry policy with exponential
  backoff (250 ms → 3 s) and full jitter, replacing hand-rolled backoff state.
  Retries are grouped into bursts of 10 attempts; on exhaustion a 10 s cooldown
  separates bursts. This prevents synchronized retry storms across gateway
  instances and improves recovery behavior during prolonged hub outages.
- Connection setup (connect + handshake + subscribe) extracted into
  `hub_observer_try_connect`, separating connection logic from retry
  orchestration.
- Entire connection attempt (connect + handshake + subscribe) is now wrapped in
  a single 5 s timeout, preventing a stalled peer from hanging the retry loop.
- Retry policy construction uses graceful error handling instead of `.expect()`,
  so a misconfiguration disables the observer with a log rather than panicking
  inside a fire-and-forget `tokio::spawn`.
- Added 1 s cooldown after the read loop exits to prevent tight reconnect loops
  when the hub accepts connections but immediately closes them.

### Fixed

- **Security:** upgraded `bytes` 1.11.0 → 1.11.1 to fix RUSTSEC-2026-0007
  (integer overflow in `BytesMut::reserve`).
