<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Developer CLI | **Priority:** P0

# verify (#48)

Snapshot integrity verification. Reads a WSC snapshot file, recomputes hashes,
and reports mismatches.

Status: complete. `echo-cli verify` validates WSC structure, reconstructs each
warp into a `GraphStore`, recomputes per-warp state roots, supports
`--expected` for warp 0, emits text/JSON reports, exits nonzero on expected-hash
mismatch, and keeps non-TTY text plain while adding colorized pass/fail markers
for terminal output. WSC v1 stores schema/tick/warp graph data but no stored
`state_root`, parent list, or `commit_id`, so commit verification remains out of
scope until a receipt/metadata source exists.

## T-6-2-1: Verify subcommand -- hash recomputation

Status: complete.

Implementation status: complete. `echo-cli verify` reads and validates WSC
snapshots, recomputes per-warp state roots through the same `GraphStore` path as
WSC roundtrip verification, compares warp 0 against `--expected` when supplied,
emits structured JSON, and formats text status plainly for pipes or with
terminal color when stdout is a TTY.

Completion evidence:

- `crates/warp-cli/src/verify.rs` implements WSC validation, state-root
  recomputation, expected-hash mismatch handling, JSON output, and TTY-aware
  text status formatting.
- Tests cover valid snapshots, expected-hash matches and mismatches, JSON mode,
  missing files, empty graphs, tampered WSC input, multi-warp unchecked status,
  plain text without ANSI escapes, and colorized TTY pass/fail formatting.

**User Story:** As a developer, I want to verify snapshot integrity from the CLI so that I can detect corruption or tampering.

**Requirements:**

- R1: `echo-cli verify <snapshot-path>` reads and validates a WSC snapshot file.
- R2: Recompute per-warp `state_root` from the graph data using the same `GraphStore::canonical_state_hash()` path as WSC roundtrip verification.
- R3: If a receipt/snapshot metadata source is added, recompute `commit_id` using `compute_commit_hash_v2` or `compute_tick_commit_hash_v2` with the stored metadata fields.
- R4: Compare the recomputed state root against `--expected` when supplied; compare stored commit metadata only once such metadata is available.
- R5: `--format json` outputs the current structured verify report: file, tick, schema hash, warp count, per-warp state roots, statuses, and overall result.

**Acceptance Criteria:**

- [x] AC1: A valid WSC snapshot passes verification with exit code 0.
- [x] AC2: A snapshot checked with a mismatched `--expected` state root fails with exit code 1 and reports the mismatch.
- [x] AC3: JSON output is valid JSON parseable by `jq`.
- [x] AC4: Text output uses color (green check / red X) when stdout is a TTY, plain text otherwise.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** Hash recomputation, mismatch reporting, text/JSON output, exit codes.
**Out of Scope:** Snapshot loading from network. Batch verification of multiple snapshots. Auto-repair.

**Test Plan:**

- **Goldens:** JSON output for a known-good snapshot. JSON output for an
  expected-hash mismatch.
- **Failures:** Snapshot file not found. Snapshot file is not valid WSC. Snapshot with missing fields.
- **Edges:** Empty graph snapshot (0 nodes). Snapshot with 10,000 nodes (performance: verify completes in <1s).
- **Fuzz/Stress:** Randomly flip bytes in a valid snapshot; verify fails structurally or reports a changed state root without panicking or falsely passing.

**Blocked By:** none (T-6-1-1 is implemented enough for current CLI dispatch)
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~200 LoC
