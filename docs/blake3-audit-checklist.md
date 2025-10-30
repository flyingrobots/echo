# BLAKE3 Audit Checklist

Use this checklist when modifying hashing- or snapshot-related code. The goal is to preserve Echo’s determinism contract while keeping cryptographic usage correct.

- Hash function
  - [ ] Use BLAKE3 (crate `blake3`) only; do not mix algorithms in snapshot/state hashing.
  - [ ] No keyed hashing in snapshot pipeline (we compute public, verifiable IDs).

- Domain separation and inputs
  - [ ] For typed IDs (node/type/edge), use explicit prefixes (`b"node:", b"type:", b"edge:"`).
  - [ ] For rule IDs, use `b"rule:" ++ name` (see build.rs).
  - [ ] For commit header digests, include exactly the fields defined in docs/spec-merkle-commit.md in the documented order.

- Byte order and encoding
  - [ ] All length prefixes are u64 little-endian; IDs are raw 32 bytes.
  - [ ] Node/edge ordering: nodes by ascending NodeId; edges by ascending EdgeId per source.
  - [ ] Reachability-only traversal: include only nodes and edges reachable from the root.

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
