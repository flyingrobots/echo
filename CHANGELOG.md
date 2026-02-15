<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Changelog

## Unreleased

## [0.1.3] — 2026-02-15

### Fixed (Sprint S1)

- **CI Security:** Hardened `det-gates` workflow against script injection by using
  environment variables for branch references.
- **WASM Reproducibility:** Implemented bit-exact reproducibility checks (G4)
  for `echo-wasm-abi` using hash comparison of clean rebuilds.
- **Static Inspection:** Added automated CI guard for `DET-001` verifying zero
  `HashMap` usage in deterministic guest paths.
- **Benchmark Methodology:** Optimized materialization benchmarks to measure
  pure emitter throughput by removing allocation overhead from hot loops.
- **CBOR Robustness:** Expanded negative security tests for `ProjectionKind`
  and `LabelAnchor` enum tags and optimized `MAX_OPS` boundary check.
- **Evidence Integrity:** Enhanced `generate_evidence.cjs` and `validate_claims.cjs`
  with stricter semantic validation (SHAs, run IDs) and artifact existence checks.
- **Script Quality:** Improved error handling, docstring coverage, and modularity
  across all hardening scripts.
- **Governance Alignment:** Moved `sec-claim-map.json` to `docs/determinism/`
  and formalized `INFERRED`/`UNVERIFIED` states in `RELEASE_POLICY.md`.

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

### Fixed (Legacy)

- **Security:** upgraded `bytes` 1.11.0 → 1.11.1 to fix RUSTSEC-2026-0007
  (integer overflow in `BytesMut::reserve`).
