# BLAKE3 Audit Checklist

Use this checklist when modifying hashing- or snapshot-related code. The goal is to preserve Echo’s determinism contract while keeping cryptographic usage correct.

- Hash function
  - [ ] Use BLAKE3 (crate `blake3`) only; do not mix algorithms in snapshot/state hashing.
  - [ ] No keyed hashing in snapshot pipeline (we compute public, verifiable IDs).

- Domain separation and inputs
  - [ ] For typed IDs (node/type/edge), use explicit prefixes (`b"node:", b"type:", b"edge:"`).
  - [ ] For rule IDs, construct by hashing the ASCII name with the `b"rule:"` domain prefix:
        `hasher.update(b"rule:"); hasher.update(name.as_bytes()); let id: [u8;32] = hasher.finalize().into();`
        Reference: crates/rmg-core/build.rs (motion rule family id generation).
  - [ ] For commit header digests (commit_id), include exactly these fields in this order:
        1) `version: u16 = 1`
        2) `parents: Vec<Hash>` encoded as length (u64 LE) then each 32‑byte parent hash
        3) `state_root: [u8;32]`
        4) `plan_digest: [u8;32]`
        5) `decision_digest: [u8;32]`
        6) `rewrites_digest: [u8;32]`
        7) `policy_id: u32`
        See docs/spec-merkle-commit.md for the canonical spec.

- Byte order and encoding
  - [ ] All length prefixes are u64 little-endian; IDs are raw 32 bytes.
  - [ ] Deterministic reachability and ordering:
        • Compute the reachable set from the designated root using a deterministic traversal (Echo uses BFS) while tracking a visited set to avoid cycles/duplicates.
        • Include an edge only if both endpoints are in the reachable set.
        • Hash nodes in ascending NodeId order; for each source, hash outgoing edges sorted by ascending EdgeId.
        Note: traversal determines inclusion; ordering is defined by sorted IDs, not traversal order.

- Snapshot invariants
  - [ ] Root ID is included first in the snapshot stream.
  - [ ] Exclude unreachable nodes/edges; filter edges whose destination is unreachable.
  - [ ] For payloads, write length (u64 LE) then exact payload bytes (or 0 if None).

- Header invariants (commit_id)
  - [ ] Version tag is u16 = 1.
  - [ ] Parents: write length (u64 LE) then each 32-byte hash in order.
  - [ ] Append state_root, plan_digest, decision_digest, rewrites_digest, policy_id.

- Tests and fixtures
  - [ ] Add/update golden fixtures for representative graphs when changing encoding.
  - [ ] Update unit tests that assert ordering and reachability behavior.
  - [ ] Record the change and rationale in docs/decision-log.md.

- Tooling
  - [ ] Consider adding a one-off verification script for historical snapshots during migrations.

## Verification and enforcement

- Unit tests (templates)
  - [ ] Golden snapshot fixture: serialize a tiny graph and assert the commit header byte layout (field order, endianness) and hash match expected values.
  - [ ] Reachability tests: assert that unreachable nodes/edges do not affect the hash, and that edges to unreachable targets are excluded (see crates/rmg-core/tests/snapshot_reachability_tests.rs).
  - [ ] Ordering tests: create nodes/edges in shuffled order and assert that the computed hash matches the sorted‑order baseline.

- Assertion helpers (optional)
  - [ ] Add test‑only helpers/macros to validate u64 LE length prefixes, field sequences, and ID sizes at encode time.

- Integration guidance
  - [ ] On format changes, regenerate golden fixtures and add a migration note in docs/decision-log.md. Verify that historical snapshots continue to verify or are migrated with a one‑off script.
