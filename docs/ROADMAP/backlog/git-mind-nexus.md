<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# git-mind NEXUS

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled
> **Formerly:** MS-3 (demoted — independent of Echo critical path)

Cross-repo federation, schema validation, and data exchange for git-mind knowledge graphs. Enables git-mind instances to sync, validate structural constraints, and exchange graph fragments via a portable format.

---

## F3.1: Remote Sync

Cross-repo federation via git-warp's CRDT capabilities. Enable git-mind instances to sync knowledge graphs across repositories so that distributed teams can collaborate on a shared knowledge base.

### T-3-1-1: git-warp remote transport for knowledge graph sync

**User Story:** As a git-mind user, I want to sync my knowledge graph with a remote git-mind instance so that distributed collaborators see the same nodes and edges.

**Requirements:**

- R1: Implement a `sync` command that connects to a remote git repo (via git-warp's remote protocol).
- R2: Merge remote knowledge graph changes using git-warp's CRDT merge semantics (LWW for node properties, union for edges).
- R3: Handle divergent histories: concurrent edits to the same node property resolve via LWW timestamp.
- R4: Report sync summary: nodes added/updated/conflicted, edges added/removed.
- R5: Support `--dry-run` flag that shows what would change without applying.

**Acceptance Criteria:**

- [ ] AC1: Two git-mind instances can sync a 100-node graph bidirectionally and converge to identical state.
- [ ] AC2: Concurrent edits to the same node property resolve deterministically via LWW.
- [ ] AC3: `--dry-run` output matches actual sync results.
- [ ] AC4: Sync works over local filesystem remotes (git-warp local transport).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: git-mind. `sync.js` module. Integration with git-warp remote transport.
**Out of Scope:** HTTP/SSH transport (uses git-warp's native transport). Conflict resolution UI. Selective sync (all-or-nothing in v1).

**Test Plan:**

- **Goldens:** Snapshot of merged graph state after a known divergent edit scenario.
- **Failures:** Unreachable remote, corrupted remote state, incompatible schema versions.
- **Edges:** Sync with empty remote, sync with identical state (no-op), sync after local delete.
- **Fuzz/Stress:** Stress test: 10 concurrent syncs against the same remote (1000 nodes each).

**Blocked By:** none
**Blocking:** T-3-3-1

**Est. Hours:** 6h
**Expected Complexity:** ~400 LoC

---

### T-3-1-2: Sync conflict reporting and resolution policy

**User Story:** As a git-mind user, I want clear reporting of sync conflicts and configurable resolution policies so that I understand what changed and can control merge behavior.

**Requirements:**

- R1: Produce a structured conflict report (JSON) listing every property conflict with local value, remote value, and resolved value.
- R2: Support resolution policies: `lww` (last-writer-wins, default), `local-wins`, `remote-wins`.
- R3: Policy is configurable per-sync via CLI flag (`--policy lww|local|remote`).
- R4: Conflict report is written to stdout (or a file via `--report-file`).

**Acceptance Criteria:**

- [ ] AC1: Conflicting edits produce a report listing both values and the resolution.
- [ ] AC2: `--policy local-wins` keeps local values for all conflicts.
- [ ] AC3: `--policy remote-wins` keeps remote values for all conflicts.
- [ ] AC4: Report JSON is parseable and includes node IDs, property names, and timestamps.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: git-mind. Conflict report generation, policy selection logic, CLI flags.
**Out of Scope:** Interactive conflict resolution UI. Per-property policy configuration. Three-way merge.

**Test Plan:**

- **Goldens:** Snapshot of conflict report JSON for a known 3-conflict scenario under each policy.
- **Failures:** Invalid policy name, report file path not writable.
- **Edges:** Zero conflicts (clean merge), all-conflict scenario, single-property conflict.
- **Fuzz/Stress:** N/A.

**Blocked By:** T-3-1-1
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~250 LoC

---

## F3.2: Schema Validators

Validation improvements for the knowledge graph schema. Enforce structural constraints on nodes and edges to catch invalid graph mutations early.

### T-3-2-1: Node and edge schema constraint validators

**User Story:** As a git-mind user, I want structural validation on my knowledge graph so that invalid nodes and edges are rejected before they corrupt the graph.

**Requirements:**

- R1: Define a schema constraint format (JSON) specifying required properties, property types, and allowed edge types per node kind.
- R2: Validate nodes against constraints on insert and update operations.
- R3: Validate edges against constraints: source/destination node kinds must be in the allowed set.
- R4: Validation errors include the constraint that was violated, the node/edge ID, and the offending value.
- R5: Schema constraints are stored in the git-mind repo (`.git-mind/schema.json`).

**Acceptance Criteria:**

- [ ] AC1: Inserting a node missing a required property produces a validation error.
- [ ] AC2: Creating an edge between disallowed node kinds produces a validation error.
- [ ] AC3: Updating a node property to an invalid type produces a validation error.
- [ ] AC4: Valid operations pass validation silently.
- [ ] AC5: Schema constraints file is loaded at startup and cached.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: git-mind. `schema/validator.js` module. Schema constraint format. Integration with node/edge CRUD operations.
**Out of Scope:** Schema migration between constraint versions. GraphQL schema generation from constraints. Async validation.

**Test Plan:**

- **Goldens:** Snapshot of validation error messages for 5+ violation scenarios.
- **Failures:** Missing schema file (graceful degradation or error), malformed schema JSON, circular kind references.
- **Edges:** Empty schema (all operations pass), maximally restrictive schema (most operations fail), schema with no edge constraints.
- **Fuzz/Stress:** Property test: random valid nodes always pass validation; random mutations to valid nodes produce either valid nodes or structured errors.

**Blocked By:** none
**Blocking:** T-3-3-1

**Est. Hours:** 6h
**Expected Complexity:** ~350 LoC

---

### T-3-2-2: Schema constraint CLI and introspection

**User Story:** As a git-mind user, I want CLI commands to inspect and manage schema constraints so that I can understand and evolve my graph's structural rules.

**Requirements:**

- R1: `git-mind schema show` prints the current schema constraints in a human-readable format.
- R2: `git-mind schema validate` runs a full graph validation against current constraints and reports all violations.
- R3: `git-mind schema init` creates a starter schema file with common defaults.
- R4: Validation summary includes counts: nodes checked, edges checked, violations found.

**Acceptance Criteria:**

- [ ] AC1: `schema show` outputs formatted constraint definitions.
- [ ] AC2: `schema validate` on a clean graph reports zero violations.
- [ ] AC3: `schema validate` on a graph with known violations reports all of them.
- [ ] AC4: `schema init` creates `.git-mind/schema.json` with a valid starter schema.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: git-mind. CLI commands (`show`, `validate`, `init`). Integration with T-3-2-1 validator.
**Out of Scope:** Schema diff between versions. Automated constraint inference from existing data.

**Test Plan:**

- **Goldens:** Snapshot of `schema show` output for a known schema. Snapshot of `schema validate` output with known violations.
- **Failures:** Running `schema show` with no schema file, `schema init` when file already exists.
- **Edges:** `schema validate` on empty graph, `schema validate` with empty schema.
- **Fuzz/Stress:** N/A.

**Blocked By:** T-3-2-1
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~200 LoC

---

## F3.3: Export/Import Interop

Federation data exchange format. Define and implement a portable format for exporting and importing knowledge graph fragments between git-mind instances, enabling workflows beyond full sync (e.g., selective sharing, archival, migration).

### T-3-3-1: Export format specification and implementation

**User Story:** As a git-mind user, I want to export a subgraph to a portable file so that I can share knowledge fragments with collaborators who may not have direct sync access.

**Requirements:**

- R1: Define a JSON-based export format (`git-mind-export/v1`) containing nodes, edges, schema constraints, and provenance metadata.
- R2: `git-mind export` command writes a subgraph (selected by node kind, label pattern, or full graph) to a `.gme` file.
- R3: Export includes content hashes for integrity verification on import.
- R4: Export format is versioned (`format_version` field) for forward compatibility.
- R5: Provenance metadata includes source repo URL, export timestamp, and exporting user.

**Acceptance Criteria:**

- [ ] AC1: `git-mind export --all` produces a `.gme` file containing all nodes and edges.
- [ ] AC2: `git-mind export --kind Person` produces a file containing only Person nodes and their incident edges.
- [ ] AC3: Export file includes `format_version: "git-mind-export/v1"` and content hashes.
- [ ] AC4: Export file is valid JSON and human-readable.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: git-mind. Export format spec (inline docs), `export.js` module, CLI `export` command.
**Out of Scope:** Import (T-3-3-2). Binary/compressed export format. Streaming export for large graphs.

**Test Plan:**

- **Goldens:** Snapshot of export file for a known 10-node graph (full export and filtered export).
- **Failures:** Export with invalid filter (unknown kind), export on empty graph.
- **Edges:** Single-node export (no edges), export with orphan edges (edges to non-exported nodes are excluded).
- **Fuzz/Stress:** Export a 10,000-node graph and verify file is valid JSON under 60 seconds.

**Blocked By:** T-3-1-1 (export format should align with sync wire format), T-3-2-1 (export includes schema constraints)
**Blocking:** T-3-3-2

**Est. Hours:** 6h
**Expected Complexity:** ~350 LoC

---

### T-3-3-2: Import with conflict detection and merge

**User Story:** As a git-mind user, I want to import a `.gme` file into my knowledge graph with conflict detection so that shared fragments merge cleanly into my existing data.

**Requirements:**

- R1: `git-mind import` reads a `.gme` file and merges its contents into the local graph.
- R2: Verify content hashes on import; reject corrupted files.
- R3: Detect conflicts: nodes with the same ID but different properties are flagged.
- R4: Support `--policy` flag (same as sync: `lww`, `local-wins`, `remote-wins`) for conflict resolution.
- R5: Validate imported nodes/edges against local schema constraints (T-3-2-1).
- R6: Report import summary: nodes added/updated/skipped, edges added/skipped, conflicts resolved.

**Acceptance Criteria:**

- [ ] AC1: Importing a `.gme` file into an empty graph creates all nodes and edges.
- [ ] AC2: Importing into a graph with overlapping nodes detects and resolves conflicts per policy.
- [ ] AC3: A corrupted `.gme` file (modified after export) is rejected with a hash mismatch error.
- [ ] AC4: Imported nodes that violate schema constraints are reported and skipped (not silently accepted).
- [ ] AC5: Import summary counts match actual changes.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: git-mind. `import.js` module, CLI `import` command. Integration with schema validator and conflict resolution.
**Out of Scope:** Streaming import. Format version migration (v1 only). Undo/rollback of import.

**Test Plan:**

- **Goldens:** Snapshot of graph state after importing a known `.gme` file into a known base graph, under each conflict policy.
- **Failures:** Corrupted file, incompatible format version, schema violations in imported data.
- **Edges:** Import into empty graph, import with zero conflicts, import with all-conflict scenario, import of a file exported from the same graph (idempotency).
- **Fuzz/Stress:** Import a 10,000-node `.gme` file and verify graph integrity under 60 seconds.

**Blocked By:** T-3-3-1
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~400 LoC
