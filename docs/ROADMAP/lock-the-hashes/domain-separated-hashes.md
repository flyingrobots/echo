<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Lock the Hashes](README.md) | **Priority:** P0

# Domain-Separated Hash Contexts

Issues: #185, #186

The core commitment hashes (`state_root`, `patch_digest`, `commit_id`) and the `RenderGraph` canonical bytes hash previously used bare `Hasher::new()` without domain-separation prefixes. This means a contrived byte sequence could produce collisions across hash contexts (e.g., a `state_root` colliding with a `commit_id`). These tasks added unique domain-separation tags to each hash context, following the existing pattern established in `crates/warp-core/src/ident.rs` (which already uses `"node:"`, `"type:"`, `"edge:"`, `"warp:"` prefixes).

## T-1-1-1: Domain-separated hash contexts for state_root, patch_digest, and commit_id

**User Story:** As an engine developer, I want each core commitment hash to use a unique domain-separation prefix so that cross-context collision attacks are structurally impossible.

**Requirements:**

- [x] R1: `compute_state_root` in `crates/warp-core/src/snapshot.rs` must prepend a domain tag (e.g., `b"echo:state_root:v1\0"`) before hashing graph content.
- [x] R2: `compute_commit_hash_v2` must prepend a domain tag (e.g., `b"echo:commit_id:v2\0"`) before the version field. The existing `2u16` version tag is retained but now follows the domain prefix.
- [x] R3: The tick patch digest computation (in `crates/warp-core/src/tick_patch.rs`) must prepend a domain tag (e.g., `b"echo:patch_digest:v1\0"`).
- [x] R4: `compute_state_root_for_warp_store` must use the same domain prefix as `compute_state_root` (single-warp variant must produce hashes in the same domain).
- [x] R5: All existing golden vector tests must be updated with new expected hashes reflecting the domain prefix changes.
- [x] R6: Domain prefix constants must be defined in a single location (e.g., a `domain` module in `crates/warp-core/src/`) and documented.

**Acceptance Criteria:**

- [x] AC1: `compute_state_root` output differs from `compute_commit_hash_v2` output even when given identical byte inputs (domain separation proven by test).
- [x] AC2: A cross-domain collision test exists: same raw bytes fed to all three hash contexts produce three distinct digests.
- [x] AC3: All existing CI tests pass with updated golden vectors.
- [x] AC4: Domain prefix constants are `pub` and documented with rationale.
- [x] AC5: `compute_state_root_for_warp_store` produces hashes in the `state_root` domain (regression test).

**Definition of Done:**

- [ ] Code reviewed and merged (PR [#265](https://github.com/flyingrobots/echo/pull/265), merged 2026-02-13T05:45:06Z)
- [ ] Milestone documentation finalized (PR [#266](https://github.com/flyingrobots/echo/pull/266), pending)
- [x] Tests pass (CI green)
- [x] Documentation updated (CHANGELOG.md, README.md)

**Scope:** `crates/warp-core/src/snapshot.rs` (both `compute_state_root` and `compute_commit_hash_v2`), `crates/warp-core/src/tick_patch.rs` (patch digest), domain constant definitions, golden vector updates.
**Out of Scope:** RenderGraph hashing (covered by T-1-1-2). CAS hashing (intentionally domain-free per `echo-cas` design). Legacy `_compute_commit_hash` v1 (retained as-is for migration reference).

**Test Plan:**

- **Goldens:** Update all snapshot hash golden vectors in `crates/warp-core/tests/` (genesis, merge, empty, checkpoint, fork). Each test must assert the exact new hash value.
- **Failures:** Verify that removing or altering a domain prefix causes golden tests to fail (mutation testing).
- **Edges:** Empty graph (zero nodes, zero edges) still produces a valid domain-separated hash. Single-parent vs multi-parent commit hashes differ.
- **Fuzz/Stress:** Property test: for 1000 random graph states, `state_root != commit_id` when patch_digest is set to the state_root value (proving domain separation prevents aliasing).

**Blocked By:** none
**Blocking:** T-1-1-2

**Est. Hours:** 4h
**Expected Complexity:** ~120 LoC (domain constants module ~30, hasher call-site changes ~40, test updates ~50)

---

## T-1-1-2: Domain-separated digest context for RenderGraph canonical bytes

**User Story:** As a tooling developer, I want the RenderGraph canonical hash to use a domain-separation prefix so that a RenderGraph digest cannot collide with a warp-core state_root or commit_id.

**Requirements:**

- [x] R1: `RenderGraph::compute_hash` in `crates/echo-graph/src/lib.rs` must prepend a domain tag (e.g., `b"echo:render_graph:v1\0"`) before hashing the CBOR canonical bytes.
- [x] R2: The domain prefix must be documented alongside the warp-core prefixes (cross-crate consistency).
- [x] R3: Existing tests using `RenderGraph::compute_hash` must be updated with new expected values.
- [x] R4: A cross-domain test must prove that `RenderGraph::compute_hash` output differs from `compute_state_root` output for equivalent graph content.

**Acceptance Criteria:**

- [x] AC1: `RenderGraph::compute_hash` includes domain prefix in its hash computation.
- [x] AC2: Cross-domain collision test passes (RenderGraph hash differs from state_root hash for same logical content).
- [x] AC3: All `echo-graph` tests pass with updated golden values.
- [x] AC4: Domain prefix is documented in `echo-graph` module docs.

**Definition of Done:**

- [ ] Code reviewed and merged (PR [#265](https://github.com/flyingrobots/echo/pull/265), merged 2026-02-13T05:45:06Z)
- [ ] Milestone documentation finalized (PR [#266](https://github.com/flyingrobots/echo/pull/266), pending)
- [x] Tests pass (CI green)
- [x] Documentation updated (CHANGELOG.md, README.md)

**Scope:** `crates/echo-graph/src/lib.rs` (`compute_hash` method), domain prefix constant, golden vector updates in `echo-graph` and downstream crates (`echo-dry-tests`, `echo-dind-tests`, `echo-session-*`).
**Out of Scope:** Changing the CBOR encoding format of `to_canonical_bytes`. Changing the sort order. CAS blob hashing.

**Test Plan:**

- **Goldens:** Snapshot tests for `RenderGraph::compute_hash` with known graph content (empty graph, single node, multi-edge).
- **Failures:** Assert that removing the domain prefix changes the hash (catches accidental regression).
- **Edges:** Empty `RenderGraph` (no nodes, no edges) produces a valid domain-separated hash distinct from the zero-input hash of other domains.
- **Fuzz/Stress:** Property test: random `RenderGraph` instances always produce hashes different from `blake3::hash` of the same canonical bytes (proving domain prefix is present).

**Blocked By:** T-1-1-1 (domain prefix naming convention must be established first)
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~80 LoC (hasher change ~10, domain constant ~10, test updates ~60)
