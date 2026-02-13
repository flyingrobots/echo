<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Changelog

## Unreleased

### Added — Determinism & Verification

- **DIND Phase 5 (The Shuffle):** Added robustness against insertion order and
  HashMap iteration leaks.
    - Implemented `echo-dind converge` command to verify that shuffles of
      commutative operations (e.g. disjoint `put_kv`) yield identical final state
      hashes.
    - Added randomized scenario generator (`scripts/bootstrap_randomized_order.mjs`)
      producing semantically equivalent transcripts via different orderings.
    - Added regression tests for Invariant A (Self-Consistency) and Invariant B
      (Convergence) in CI.
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
      across all 66 roadmap documents.

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
