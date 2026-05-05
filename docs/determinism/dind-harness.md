<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# DIND Harness

The DIND harness is Echo's deterministic verification runner. It replays
canonical intent transcripts and asserts that state hashes are identical across
runs, platforms, and build profiles.

Location:

- `crates/echo-dind-harness`
- `crates/echo-dind-tests` (stable test app used by the harness)
- `testdata/dind` (scenarios + goldens)

## Quickstart

```sh
# Via xtask (recommended)
cargo xtask dind run

# Valid subcommands: run, record, torture, converge

# Or directly via cargo
cargo run -p echo-dind-harness -- help
```

Direct harness examples:

```sh
cargo run -p echo-dind-harness -- run testdata/dind/000_smoke_theme.eintlog \
  --golden testdata/dind/000_smoke_theme.hashes.json
cargo run -p echo-dind-harness -- torture testdata/dind/000_smoke_theme.eintlog --runs 20
cargo run -p echo-dind-harness -- converge \
  testdata/dind/051_randomized_convergent_seed0001.eintlog \
  testdata/dind/051_randomized_convergent_seed0002.eintlog
```

Cross-platform DIND runs weekly in CI via `.github/workflows/dind-cross-platform.yml` (Windows, macOS, and Linux matrix).

## Determinism Guardrails

Echo ships guard scripts to enforce determinism in core crates:

- `scripts/ban-globals.sh`
- `scripts/ban-nondeterminism.sh`
- `scripts/ban-unordered-abi.sh`

### FootprintGuard Enforcement Tests

Echo validates footprint enforcement alongside DIND via the **slice theorem
proof** test suite (`crates/warp-core/tests/slice_theorem_proof.rs`).
These tests execute the same workload under varying worker counts
(1, 2, 4, 8, 16, 32) and verify that `patch_digest`, `state_root`, and
`commit_hash` remain identical, proving that the footprint declarations
are both correct and complete.

The FootprintGuard is active in debug builds unless the `unsafe_graph` feature
is enabled, meaning undeclared reads or writes surface as a
`FootprintViolation` before convergence checks can hide the issue.

### Snapshot/Restore Fuzz Gate

`warp-core` also carries a snapshot/restore fuzz gate for replay-state
serialization determinism:

```sh
cargo test -p warp-core --test snapshot_restore_fuzz
```

The gate builds a deterministic 500-tick worldline, snapshots materialized state
at 50 deterministic pseudo-random coordinates, restores the snapshot from
canonical WSC bytes, replays the remaining suffix from recorded provenance, and
compares the restored `state_root` with the uninterrupted run. The report names
the snapshot tick, restore tick, comparison tick, and expected/actual hashes for
each iteration.

The current applicable snapshot format is canonical WSC v1. If Echo adds a
separate debug snapshot encoding later, that format should be added to the same
matrix rather than becoming an un-gated restore path.

The corruption test flips one stored WSC byte. Passing behavior is either a
closed restore/validation failure or an explicit suffix-replay hash mismatch;
silent success is not acceptable.

## Convergence scope (Invariant B)

For commutative scenarios, `MANIFEST.json` can specify a `converge_scope`
node label (e.g., `sim/state`). The `converge` command compares the
projected hash of the subgraph reachable from that node, while still
printing full hashes for visibility.

### Converge scope semantics (short spec)

**What scopes exist today (DIND test app):**

- `sim/state` — the authoritative state root for the test app (includes theme/nav/route + kv).
- `sim/state/kv` (not currently used) — a narrower root for KV-only projections.

**What is included in the projected hash:**

- All nodes reachable by following **outbound edges** from the scope root.
- All edges where both endpoints are reachable.
- All node and edge attachments for the included nodes/edges.

**What is excluded:**

- Anything not reachable from the scope root (e.g., `sim/inbox`, event history, sequence sidecars).
- Inbound edges from outside the scope.

**What “commutative” means here:**

- The operations are order-independent with respect to the **projected subgraph**.
- Either they touch disjoint footprints or they are semantically commutative
  (e.g., set union on disjoint keys).

**When you must NOT use projection:**

- When event history is semantically meaningful (auditing, causality, timelines).
- When last-write-wins behavior or ordered effects are part of the contract.
- When differences in inbox/order should be observable by the consumer.

### CLI override (debug only)

`converge` accepts an override for ad-hoc debugging:

```sh
cargo run -p echo-dind-harness -- converge --scope sim/state --i-know-what-im-doing <scenarios...>
```

This bypasses `MANIFEST.json` and emits a warning. Do not use it for canonical
test results.
