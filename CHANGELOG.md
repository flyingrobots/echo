<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Changelog

## Unreleased

### Docs Polish (#41)

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
- **New:** Configuration reference (`docs/guide/configuration-reference.md`)
  covering engine parameters, protocol constants, and environment variables.
- **New:** Cargo feature flags reference (`docs/guide/cargo-features.md`)
  covering all 19 features across 11 crates.
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
