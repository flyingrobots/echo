<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0007: BOAW Storage, Execution, Merge, and Privacy

> **Naming note:** "BOAW" is a retired codename. The live implementation is
> under `warp_core::parallel` and adjacent runtime modules.

- **Status:** Partially superseded
- **Date:** 2026-01-17
- **Superseded in part by:** [ADR 0020](0020-retained-reading-storage-and-proof-boundary.md), which governs WSC/CAS ontology and proof authority

## Context

The spike-era store combined mutable tables, in-place execution, and post-hoc
diffing. That shape made parallel execution, deterministic convergence,
immutable snapshots, and privacy-safe provenance difficult to enforce.

The runtime needed a mechanical boundary between a read-only basis, candidate
execution, deterministic admission, emitted operations, and the next committed
reading. It also needed merge and privacy rules that did not silently turn
opaque bytes or secrets into durable causal facts.

## Decision

### Immutable basis and deltas

Execution reads an immutable basis. Candidate work emits tick-local operations
instead of mutating shared state. Deletes are logical unlinks from the next
reading; physical reclamation is an independent retention policy.

Thread-local deltas are merged in a canonical order. The merged patch and the
resulting committed reading have deterministic identities. Structural sharing
is a storage optimization and must not change the logical result.

WSC is a deterministic physical representation for retained WARP-shaped
material. A segment-addressed CAS may provide exact byte identity and physical
sharing. Under ADR 0020, neither WSC nor CAS is causal history, admission or
recovery authority, semantic reading identity, or proof authority.

### Footprints and admission

Generated work declares complete read and write footprints for nodes, edges,
attachments, boundary ports, and any index or bucket targets. Two candidates
are independent only when neither writes or deletes material read, written, or
anchored by the other.

The scheduler uses a fixed logical shard topology and canonical candidate
ordering. Hardware worker count may change execution throughput but must not
change admission, merge order, patch bytes, or committed identity.

Footprint enforcement is a correctness oracle for generated rule packs. The
runtime may execute admitted independent work concurrently, but each worker
emits only into its own delta and cannot mutate the shared basis.

### Commit and merge

The commit boundary canonically combines admitted deltas, materializes the next
reading, and binds the patch and reading identities. Multi-parent collapse is a
typed reconciliation of claims and structured values, not an instruction to
invent a merge for arbitrary binary blobs.

Presence and value conflicts remain distinct. Unsupported or interfering
claims become explicit conflict artifacts unless a declared deterministic type
policy provides a lawful join. Parent and candidate order cannot leak into a
commutative result.

### Privacy

Durable causal evidence records typed commitments, proofs, opaque references,
and policy identities rather than sensitive plaintext. Guessable secrets must
not be committed as an unsalted hash. Diagnostics may reveal more only under an
explicit type and disclosure policy; it is not an escape from provenance law.

Proofs are evidence for named claims. A valid proof does not grant admission,
execution, reveal, or recovery authority. Conflicting valid claims remain an
explicit semantic conflict unless a named law resolves them.

## Rejected Alternatives

- Mutate a shared store during candidate execution and reconstruct a diff later.
- Derive logical shard count or canonical order from available CPUs.
- Treat write/write disjointness as sufficient while ignoring reads, deletes,
  anchors, indexes, or boundary ports.
- Merge arbitrary opaque bytes without a typed deterministic policy.
- Put sensitive raw values into append-only provenance and rely on later
  deletion.
- Treat a WSC file, CAS hash, commitment, or proof as causal authority.

## Consequences

- Parallelism is permitted only behind deterministic admission and isolated
  delta emission.
- The runtime needs canonical operation ordering, footprint validation, and a
  deterministic materialization boundary.
- Mergeable application values require registered type law; opaque values
  conflict or use an explicitly declared deterministic policy.
- Storage and proof implementations remain subordinate to causal and semantic
  identities.
- Privacy posture is part of the durable evidence contract, not a logging mode.

## Evidence Anchors

- `crates/warp-core/src/parallel/`
- `crates/warp-core/src/tick_delta.rs`
- `crates/warp-core/src/tick_patch.rs`
- `crates/warp-core/src/footprint.rs`
- `crates/warp-core/src/wsc/`
- `crates/warp-core/src/proof.rs`
- `docs/adr/0020-retained-reading-storage-and-proof-boundary.md`

## Historical Note

The original record included a migration plan, proposed test suite, large Rust
scaffolds, sequencing checklist, and completion status. Those non-authoritative
process artifacts remain in Git history; they are not part of this durable
decision.
