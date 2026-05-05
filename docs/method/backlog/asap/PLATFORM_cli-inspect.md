<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Developer CLI | **Priority:** P0

# inspect (#50)

Snapshot summary, graph statistics, and optional terminal visualization.

Status: T-6-4-1 complete; T-6-4-2 remains planned. `echo-cli inspect` loads
and validates WSC files, reports tick/schema hash/warp count plus per-warp IDs,
root node, state root, node/edge counts, type breakdown, connected components,
and optional tree output in text or JSON. WSC v1 does not currently store
`commit_id`, parent list, or `policy_id`, so those fields must not be treated
as implemented metadata unless the WSC format grows them. The remaining active
gap is attachment payload display and `--raw`.

## T-6-4-1: Inspect subcommand -- metadata and graph stats

Status: complete.

Implementation status: complete. `echo-cli inspect` has CLI-level coverage for
metadata, graph statistics, JSON structure, tree rendering, and corrupt WSC
failure behavior against a deterministic WSC fixture.

Completion evidence:

- `crates/warp-cli/src/inspect.rs` computes current WSC metadata, per-warp
  state root, graph counts, type breakdowns, connected component count, and a
  depth-limited root tree.
- `crates/warp-cli/tests/cli_integration.rs` runs the real `echo-cli inspect`
  binary against a generated deterministic snapshot in text/tree and JSON
  modes.
- Corrupt snapshot input exits non-zero with an inspect error instead of a
  panic.

**User Story:** As a developer, I want to inspect a snapshot's metadata and graph structure so that I can debug simulation state without writing code.

**Requirements:**

- R1: `echo-cli inspect <snapshot-path>` prints current WSC metadata: tick count, schema hash, warp count, per-warp ID, root node ID, and computed state root.
- R2: Graph statistics: total nodes, total edges, node types breakdown (count per TypeId), connected components count.
- R3: `--format json` outputs all stats as structured JSON.
- R4: `--tree` flag renders a simple ASCII tree of the graph starting from the root node (depth-limited to 5 levels).

**Acceptance Criteria:**

- [x] AC1: Inspect on a demo snapshot prints all current WSC metadata fields.
- [x] AC2: Node type breakdown sums to total node count.
- [x] AC3: `--tree` output shows root at level 0 with children indented.
- [x] AC4: JSON output includes both metadata and graph stats.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** Metadata display, graph stats computation, ASCII tree rendering, JSON output.
**Out of Scope:** Interactive graph exploration (that is the website demo). Diff between two snapshots.

**Test Plan:**

- **Goldens:** Text and JSON output for a known snapshot fixture.
- **Failures:** Snapshot not found. Corrupt snapshot (graceful error, not panic).
- **Edges:** Snapshot with 0 nodes. Snapshot with disconnected components (tree shows only root's component). Very deep graph with `--tree` (respects depth limit).
- **Fuzz/Stress:** Inspect a 50,000-node snapshot; must complete in <2s.

**Blocked By:** none (T-6-1-1 is implemented enough for current CLI dispatch)
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~250 LoC

---

## T-6-4-2: Inspect -- attachment payload pretty-printing

Status: not implemented. WSC stores attachment rows and blobs, and
`warp-cli` reconstructs them for state-root verification, but inspect does not
yet render attachment payloads.

**User Story:** As a developer, I want inspect to decode and display attachment payloads so that I can see entity data without manual hex decoding.

**Requirements:**

- R1: When the codec registry is available, decode `AtomPayload` bytes using the registered codec and display as formatted fields.
- R2: For motion payloads (v0 and v2 Q32.32), display decoded position/velocity as decimal values.
- R3: For unknown payload types, display hex dump with type_id annotation.
- R4: `--raw` flag disables decoding and shows hex for all payloads.

**Acceptance Criteria:**

- [ ] AC1: Motion payload displays as `position: (x, y, z), velocity: (vx, vy, vz)` with decimal values.
- [ ] AC2: Unknown payload type shows `[type_id: abcd1234...] 0x48656c6c6f...`.
- [ ] AC3: `--raw` flag shows hex for all payloads including known types.
- [ ] AC4: Truncated payloads display a warning and partial hex.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Payload decoding, motion payload formatting, hex fallback, --raw flag.
**Out of Scope:** Interactive payload editing. Custom codec plugin loading.

**Test Plan:**

- **Goldens:** Formatted output for a snapshot containing a motion-rule entity with known Q32.32 values.
- **Failures:** Payload bytes shorter than expected for declared type (warning + hex fallback).
- **Edges:** Empty payload bytes. Payload with all-zero bytes. Maximum-length payload (64KB).
- **Fuzz/Stress:** N/A.

**Blocked By:** none (T-6-4-1 is implemented enough to support payload display work)
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~150 LoC
