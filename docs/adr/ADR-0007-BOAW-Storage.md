<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# ADR-0007: BOAW Storage + Execution + Merge + Privacy (Atom→WARP End-to-End)

- Status: Accepted
- Date: 2026-01-17
- Project: Echo / Continuum (warp-core)
- Decision: Replace the spike-era monolithic in-place GraphStore model with BOAW: immutable snapshots + COW overlays + lockless parallel execution via per-thread deltas, deterministic commits/hashes, explicit footprint-based independence scheduling, typed merge/collapse, and privacy-safe provenance (mind/diagnostics modes).

---

## 0) Why

The spike GraphStore (monolithic BTreeMaps + in-place mutation + post-hoc diff) was correct for proving rewrite mechanics, but it hard-bakes constraints that fight the engine goals:

- Massive parallel throughput without locks
- Deterministic convergence across platforms
- Zero-copy snapshot IO (WSC)
- Forking and collapse/merge
- Privacy-safe provenance: never write sensitive raw bytes into an append-only ledger

This ADR locks the full end-to-end architecture so new work does not accrete demo tech debt.

---

## 1) Glossary (tight)

- Atom: Typed payload (type_id + bytes) stored as an attachment value. Semantics depend on type policy.
- Attachment: Value associated with a node/edge or a boundary interface. Attachments are the primary data plane.
- WARP / Worldline: A commit-addressed evolving view of state. A worldline is defined by its commit DAG.
- Snapshot (WSC): Immutable, canonical, readable without copying: sorted tables + ranges + blob arena.
- COW: “Delete” means unlink from the worldline view, not physical destruction; physical reclamation is optional GC.
- Footprint: Declared read/write sets (nodes, edges, attachments, boundary ports, and any bucket/index targets) used for independence checks.
- MWMR Scheduling: Multiple writers admitted only when independence is proven; execution is parallel; commit is canonical.
- Collapse/Merge: Deterministic reconciliation of multiple parents into one commit.
- Mind mode: Ledger must be publishable; secrets never enter provenance.
- Diagnostics mode: Richer introspection allowed (still type-governed), meant for trusted debugging contexts.

---

## 2) Decision summary (the “BOAW stack”)

We adopt the following layered architecture:

1. Immutable Base: Snapshots are immutable and read-only (WSC-backed; optionally memory-mapped).
2. TickDelta Overlay: Rewrites do not mutate a shared store. They emit ops into thread-local deltas.
3. Deterministic Admission: Only independent rewrites are admitted per tick (footprints). Ordering is canonical.
4. Lockless Parallel Execute: Workers execute admitted rewrites against a read-only snapshot, emitting ops locally.
5. Deterministic Merge-of-Deltas: Thread-local deltas are merged canonically into a single tick patch.
6. Canonical Commit: Apply patch to build the next snapshot (reachable-only view), compute state root + patch digest + commit hash.
7. Collapse/Merge: Multi-parent merges reconcile claims and structured state, with typed merge rules and conflict artifacts.
8. Privacy: In mind mode, the ledger records only commitments/proofs/opaque refs/policy hashes—never sensitive bytes.

---

## 3) Atom & Attachment policy (typed, enforceable)

Each Atom type MUST declare (via registry metadata):

- Sensitivity: Public | Private | ForbiddenInLedger
- MergeBehavior: Mergeable | LWW | ConflictOnly
- Disclosure: Never | ByConsent | ByWarrant | DiagnosticsOnly
- Canonicalization: how bytes are normalized (endianness, float policy, ordering, etc.)
- Verifier (optional): for ZK proofs or validity checks

Engine enforcement:

- Mind mode: ForbiddenInLedger atoms cannot be written as bytes into tick patches or snapshots. Only indirect forms are allowed (commitment/proof/private_ref).
- Diagnostics mode: richer data MAY be permitted, but still governed by type policy (no “oops logged SSN” allowed).

---

## 4) WARP snapshots (reachable-only)

### 4.1 Reachable-only is the default

A snapshot contains only nodes/edges reachable from the root(s). Unreachable objects are not part of the materialized view.

- Delete in a worldline = Unlink
- The next snapshot omits the node/edge from reachability.
- Physical deletion is NOT part of tick execution.
- Physical reclamation (optional):
- A deterministic GC MAY delete unreachable objects/segments from storage based on pinning/retention policy.
- “Never delete the substrate” is a policy choice: skip GC or pin everything.

### 4.2 Zero-copy WSC is the canonical snapshot IO

WSC remains the target snapshot format:

- Nodes table sorted by NodeId
- Edges table sorted by EdgeId
- Per-node outbound index out_index → ranges into out_edges
- Attachment index tables per node/edge → ranges into attachment rows
- Blob arena referenced by (offset,len)

The reader (WscFile/WarpView) remains valid. The writer/materializer becomes the canonical commit builder.

---

## 5) COW mechanics (overlay reads/writes, unlink deletes, structural sharing, GC policy)

### 5.1 COW is a storage rule, not a vibe

> Rule: During a tick, nothing shared is mutated. Writes produce an overlay delta. The next commit produces a new immutable snapshot that structurally shares with prior snapshots.

So “COW” means:

- Base snapshot is immutable
- Overlay is per-worldline/per-tick
- Commit materializes a new snapshot (sharing unchanged parts)

### 5.2 Worldline-local overlay (the COW write path)

Each worldline has a tick-local overlay (“delta”), produced by parallel execution:

- upsert_nodes: `Vec<(NodeId, NodeRecordRef)>`
- delete_nodes: `Vec<NodeId>` (unlink in view)
- upsert_edges: `Vec<(EdgeId, EdgeRecordRef)>`
- delete_edges: `Vec<EdgeId>`
- set_node_attachment: `Vec<(NodeId, AttachmentKey, ValueRef|None)>`
- set_edge_attachment: `Vec<(EdgeId, AttachmentKey, ValueRef|None)>`
- claim_records: `Vec<ClaimRecord>` (mind-mode safe)

No GraphStore mutation in execute. Ever.

### 5.3 Reads are snapshot + overlay resolution

During execution, reads resolve as:

1. Overlay writes/tombstones win (tick-local truth)
2. Otherwise read from base snapshot

This is purely logical; no copying required. It is an API/lookup rule.

### 5.4 Delete semantics are “Unlink”, always

DeleteNode/DeleteEdge in the tick delta means:

- it is absent in the next snapshot’s reachable-only view
- it may still exist in underlying object storage if referenced by other commits/worldlines

Physical deletion is never part of tick execution. It is GC policy.

### 5.5 Structural sharing strategy (how COW avoids rewriting huge WSCs)

Snapshots are immutable tables; COW requires sharing. We lock in segment-level sharing:

- Snapshot tables are stored as segments (e.g., 1–4MB blocks)
- Each segment is content-addressed (hash of bytes)
- A commit references segment hashes for:
- nodes table segments
- edges table segments
- out_index segments
- out_edges segments
- attachment tables segments
- blob arena segments (or chunked blobs)

Commit builder output = “segment manifest + directory.”
Only changed segments are newly written. Unchanged segments are reused.

WSC can remain the “single-file” format by packing segments on write, but the canonical storage model is segment-addressed. (Packing is a distribution artifact.)

### 5.6 GC policy is pinning, not a second graph

We do not maintain a second mutable “base graph.”

Instead:

- The substrate is an immutable CAS of segments/objects
- “Never delete anything” = pin commits (or disable GC)
- “Free disk” = GC unreachable segments from unpinned commits

Pinned roots define retention; not refcounts.

---

## 6) Footprints & enforcement

### 6.1 Footprint captures all mutation targets

A footprint includes:

- `n_read`, `n_write` (`NodeIds`)
- `e_read`, `e_write` (`EdgeIds`)
- `a_read`, `a_write` (`AttachmentKey`)
- `b_in`, `b_out` (`PortKey` boundary interfaces)
- `factor_mask` coarse prefilter (superset of touched partitions)

**Critical rule:** if something can be mutated, it must be representable in the footprint:

- adjacency buckets (`edges_from[node]`, `edges_to[node]`) are explicit write targets (represented via `AttachmentKey` or dedicated target keys)
- any indexes/caches mutated during commit must be either:

  - (a) derived in commit (preferred), or
  - (b) modeled as explicit targets and shardable

### 6.2 Independence check semantics (snapshot reads)

We use snapshot semantics:

- read-read overlap is allowed
- any write/write or write/read overlap is a conflict
- boundary port conflicts are always conflicts

### 6.3 Footprint enforcement

Executors are not trusted to “stay aligned” with compute_footprint.

We enforce with one of:

- **Plan→Apply fusion:** planning returns `{footprint, apply_closure}`, and apply uses footprint-derived capabilities, OR
- **FootprintGuard:** all mutation emission paths validate the target was claimed (debug-hard now; promotable to hard errors later)

---

## 7) Scheduling/queues (virtual shards)

We explicitly reject "queue per CPU" as the partition key because it is hardware-dependent and cache-hostile.

- We use fixed virtual shards (e.g., 256/1024, power-of-two).
- Route by existing `NodeId`/`EdgeId` bits (no rehash): `shard = lowbits(id) & (SHARDS-1)`.
- Work queues are per shard (or per partition), not per core.
- Workers = hardware threads; workers pull/claim shards dynamically to balance load.

This preserves determinism across machines and improves locality.

### 7.1 Shard Routing Specification (FROZEN - Phase 6B)

```text
NUM_SHARDS = 256
shard = LE_u64(node_id.as_bytes()[0..8]) & (NUM_SHARDS - 1)
```

This formula is **frozen once shipped**. The routing takes the first 8 bytes of the NodeId's 32-byte
BLAKE3 hash, interprets them as a little-endian u64, and masks to the shard count.

- **NUM_SHARDS = 256**: Protocol constant, cannot change without version bump
- **Little-endian**: Explicit and platform-independent
- **First 8 bytes only**: Remaining 24 bytes don't affect routing

Implementation: `crates/warp-core/src/boaw/shard.rs::shard_of()`

---

## 8) Tick pipeline (plan/admit/parallel execute/delta merge/commit)

### 8.1 Canonical ingress

- Ingest intents into an ingress list keyed by canonical `intent_id`.
- Sort deterministically (radix sort permitted if stable and defined).

### 8.2 Plan

- For each candidate intent/rule match:
- compute footprint
- compute `factor_mask`
- produce a `PlannedRewrite` handle (callable) that reads from snapshot only

### 8.3 Admit (deterministic)

- Greedy admit in canonical order by intent_id (and tie-breakers like rule_id, match_ix).
- Reject or defer conflicts deterministically.

### 8.4 Execute (parallel, lockless)

- Workers execute admitted rewrites against read-only snapshot.
- Output is a thread-local TickDelta (append-only ops).

### 8.5 Merge deltas (deterministic)

- Concatenate all thread-local ops.
- Sort canonically by (op_kind, object_key, tie_breaker) where tie-breaker is stable (intent_id / rule_id / match_ix).
- Apply per-type conflict policies (reject/join/LWW/conflict artifact).

### 8.6 Commit

- Build next reachable-only snapshot from previous snapshot + merged ops.
- Compute:
  - `state_root` (canonical hash of materialized reachable state)
  - `patch_digest` (canonical hash of merged ops)
  - `commit_hash = H(parents || state_root || patch_digest || schema_hash || tick || policy_hashes)`

---

## 9) Collapse/merge

### 9.1 Merge is about claims and structured state, not “bytes”

Arbitrary binary does not get magically merged.

We define merge over keys:

- Node keys (`NodeId`)
- Edge keys (`EdgeId`)
- Attachment slots (`AttachmentKey`)
- Claim keys (see Privacy section)

### 9.2 Merge regimes

- Preferred: commutative + associative merges (CRDT-like) for mergeable types.
- Allowed: order-dependent merges only with canonical parent order (sort parents by `commit_hash`, then fold).

### 9.3 Presence vs Value

For each key:

- Presence policy: delete-wins | add-wins | LWW (default: delete-wins for reachability)
- Value policy: type-driven merge
- Mergeable: merge function provided by type registry
- LWW: deterministic winner by canonical ordering key
- ConflictOnly: produce a conflict artifact

### 9.4 Conflict artifacts are first-class, deterministic, safe

When merge cannot resolve:

- Emit a conflict artifact attachment/object containing only:
  - parent commit hashes
  - statement/value hashes
  - type ids
  - policy hashes

No secrets, no raw sensitive bytes.

---

## 10)  Privacy (mind vs diagnostics; claims/proofs/vault)

### 10.1 Ledger stores claims, not secrets

In mind mode, the ledger must be publishable. Therefore:

- No SSNs, credit cards, nude images, private chats, etc.
- Only:
  - commitments
  - ZK proofs (or proof hashes)
  - opaque private refs
  - policy hashes
  - canonical metadata

## 10.2 ClaimRecord (canonical)

We record a deterministic claim record:

- `claim_key` (stable identity of the claim)
- `scheme_id` (ZK / verifier identity)
- `statement_hash` (public statement)
- `commitment` (to secret or ciphertext)
- `proof_bytes` OR `proof_hash` (`policy-controlled`)
- `private_ref` (optional pointer into erasable vault)
- `policy_hash` (redaction/disclosure/retention rules)
- `issuer` (rule/subsystem id), tick, etc.

### 10.3 Commitments must be dictionary-safe

Never commit as `H(secret)` for guessable secrets.

Use:

- secret pepper (not recorded in ledger), e.g. `H(pepper || canonical(secret))`, OR
- commitment to encrypted payload stored in vault

### 10.4 ZK proofs “conflict”

Proofs are evidence; claims are merged.

During collapse for a `claim_key`:

- verify proofs
- if invalid: quarantine (not canonical)
- if multiple valid proofs with same statement: dedupe
- if multiple valid proofs with different statements: claim conflict → conflict artifact unless a declared deterministic policy resolves

---

## 11)  Consequences

Benefits:

- Lockless parallel execution (thread-local deltas)
- Deterministic across platforms (canonical orders, fixed shard topology)
- Zero-copy snapshots (WSC)
- Natural forking and collapse/merge with typed semantics
- Privacy-safe provenance by construction (mind mode publishable)

Costs:

- Requires a snapshot builder / materializer phase (base + ops → next WSC)
- Requires strict type registry (merge + privacy metadata)
- Requires moving away from in-place mutation + diff as the primary execution path

Non-goals (explicit):

- “Merge arbitrary binary bytes into a new binary” is not supported.
- Opaque blobs: `ConflictOnly` or deterministic LWW.
- If merge is needed, define a structured type (e.g., `ArchiveTree`) and canonicalize it; zip/tar bytes are derived artifacts.
- No hardware-dependent determinism (no shard count = `num_cpus`).

---

## 12) Migration plan

1. Introduce `TickDelta` + canonical merge alongside existing `GraphStore` (no semantic change yet).
2. Change executors to emit ops into `TickDelta` (keep `GraphStore` for reads, stop writing it).
3. Implement `SnapshotBuilder` that applies `TickDelta` to produce next snapshot (still can materialize into in-memory tables initially).
4. Wire WSC writer as the canonical snapshot output; `WscFile`/`WarpView` remain the canonical reader.
5. Replace monolithic `GraphStore` with immutable snapshot + overlay model.
6. Add collapse/merge phase with typed registry and conflict artifacts.
7. Add mind/diagnostics enforcement gates; forbid sensitive atoms in mind mode.

---

## 13) Doctrine line

State is an immutable snapshot. Time is a commit DAG. Writes are deltas. Deletes are unlinks. Proofs are claims. Privacy is non-negotiable. Determinism isn’t optional.

---

## 14) Tests

Here’s a test suite spec you can staple to the BOAW ADR so every claim is executable. This is written so you can drop it into `crates/warp-core/tests/` (or `warp-core/src/boaw/tests/`) and start implementing one-by-one.

---

### ADR-00XX Test Suite: BOAW Compliance

Conventions

- All tests must run with --features determinism (or whatever flag you use) and must be platform-stable.
- Every test that compares hashes must do at least 20 random permutations of ingress order (seeded RNG with fixed seeds).
- All “canonical order” tests must validate byte-for-byte output equality.

---

### 1) Snapshot & Hash Determinism (WSC + state_root)

#### T1.1 Snapshot hash is invariant under insertion order

Given: same logical graph built from ops in different order
Expect: identical state_root, identical WSC bytes (or identical segment manifest)

- Build base snapshot from a set of nodes/edges/attachments.
- Shuffle op order 50 times.
- Materialize.
- Assert `state_root` identical across all runs.

#### T1.2 Zero-copy read roundtrip is exact

Given: WSC produced by builder
Expect: WarpView sees exactly the same tables (IDs, types, ranges, blobs)

- Write WSC
- Read via WscFile::from_bytes
- Check node_ix, edge_ix, out_edges_for_node, attachment accessors
- Verify blob slices match original bytes

#### T1.3 Reachable-only semantics

Given: unreachable node/edge exists in object store but not reachable from root
Expect: snapshot excludes it; state_root unchanged if only unreachable changes

- Add unreachable objects to CAS (or builder inputs)
- Ensure snapshot excludes them
- Ensure state_root ignores them

---

### 2) COW Overlay Semantics

#### T2.1 Delete is unlink (view-only)

Given: base snapshot has node X; overlay deletes X
Expect: reads from overlay show X absent; base snapshot still contains X

- Create base snapshot with X
- Apply overlay `DeleteNode(X)`
- Assert view resolver hides X
- Assert base snapshot still resolves X

#### T2.2 Overlay precedence

Given: base has attachment A; overlay sets A to new value
Expect: read yields overlay value; commit includes overlay value

#### T2.3 Structural sharing (segment reuse)

Given: commit changes only one node attachment
Expect: only the affected segments differ; unchanged segments reused

- Build commit C0
- Build commit C1 with one small change
- Compare segment manifests: most hashes identical

---

### 3) Footprints & Independence

#### T3.1 Footprint independence is symmetric

`fp.independent(a,b) == fp.independent(b,a)` for randomized footprints.

#### T3.2 No write/read overlap admitted

Given: two planned rewrites where one writes a node the other reads
Expect: only one admitted

#### T3.3 Bucket/index targets are enforced

Given: two edge deletes with same from but different edge_id
Expect: independence fails when adjacency bucket target is claimed

(This test prevents the “retain() race” forever.)

#### T3.4 FootprintGuard catches drift

Given: executor emits an op not claimed in footprint
Expect: panic in debug (or deterministic error in release mode)

---

### 4) Scheduling & Queues (virtual shards)

#### T4.1 Shard routing is stable across machines

Given: same NodeId/EdgeId
Expect: same shard id (with fixed SHARDS constant)

#### T4.2 Admission does not depend on num_cpus

Given: same ingress set; run scheduler with worker counts {1,2,8,32}
Expect: same admitted set, same patch_digest, same state_root

---

### 5) Parallel Execute: Lockless + Deterministic

#### T5.1 Parallel equals serial (functional equivalence)

Given: identical inputs
Expect: serial execute and parallel execute produce identical merged ops, commit hash, state_root

- Run execute with 1 worker and N workers
- Compare results byte-for-byte

#### T5.2 Permutation invariance under parallelism

Given: shuffled ingress order + varied worker counts
Expect: identical commit hash

This is the “determinism drill sergeant” test for BOAW.

#### T5.3 No shared mutation (lint/test)

If possible: instrument forbidden APIs so calling &mut GraphStore mutators from executor fails compilation or test asserts.

---

### 6) Merge / Collapse

#### T6.1 Parent order invariance for commutative merges

Given: same set of parents in different orders
Expect: identical merge commit hash for mergeable types

#### T6.2 Canonical parent ordering for order-dependent merges

Given: order-dependent merge type
Expect: merge uses parent commit hash sort order and is stable

#### T6.3 Conflict artifact is deterministic

Given: irreconcilable values for same key
Expect: conflict artifact bytes and hash are identical across runs

---

### 7) Privacy: Mind vs Diagnostics

#### T7.1 Mind mode forbids ForbiddenInLedger atoms

Given: attempt to emit attachment bytes of forbidden type
Expect: deterministic rejection (error) OR forced indirection (commitment/proof/private_ref)

##### T7.2 Proof/claim merge: invalid proofs quarantined

Given: two claims, one valid proof, one invalid
Expect: merge selects valid; invalid produces audit artifact or is excluded by policy

##### T7.3 Conflicting valid claims produce conflict artifact

Given: same claim_key, different statement_hash, both verify
Expect: conflict artifact (unless policy resolves)

#### T7.4 Commitment is dictionary-safe

Given: known secret and commitment
Expect: commitment changes when pepper changes; cannot be reproduced without pepper

---

Where to put this in the repo

- `crates/warp-core/tests/boaw_determinism.rs`
- `crates/warp-core/tests/boaw_cow.rs`
- `crates/warp-core/tests/boaw_merge.rs`
- `crates/warp-core/tests/boaw_privacy.rs`

And one “god test”:

- `crates/warp-core/tests/boaw_end_to_end.rs` that runs:

  - ingest permutations
  - multi-worker counts
  - emits WSC
  - reloads WSC zero-copy
  - asserts final commit hash identical

---

## Commit-ready Rust test skeletons

You can drop into `crates/warp-core/tests/` today. They’ll compile with minimal repo coupling, then go red (panic/TODO) until you wire the BOAW APIs.

I’m giving you:

- a tiny deterministic RNG (no rand dependency)
- a shared TestHarness trait you implement once
- 5 test files + tests/common/mod.rs
- a “god test” that hits permutations + worker counts + WSC roundtrip

### Expected workflow

Commit these tests now → they fail loudly → implement BOAW pieces until they pass.

---

## Suggested file layout

```text
crates/warp-core/tests/
  common/
    mod.rs
  boaw_end_to_end.rs
  boaw_determinism.rs
  boaw_cow.rs
  boaw_merge.rs
  boaw_privacy.rs
  boaw_footprints.rs
```

---

## `tests/common/mod.rs`

```rust
// crates/warp-core/tests/common/mod.rs
#![allow(dead_code)]

pub type Hash32 = [u8; 32];

/// Tiny deterministic RNG (xorshift64*) so tests don't need `rand`.
#[derive(Clone)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn gen_range_usize(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            return 0;
        }
        (self.next_u64() as usize) % upper
    }
}

/// Fisher–Yates shuffle (deterministic).
pub fn shuffle<T>(rng: &mut XorShift64, items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let j = rng.gen_range_usize(i + 1);
        items.swap(i, j);
    }
}

/// Useful seed set for determinism drills.
pub const SEEDS: &[u64] = &[
    0x0000_0000_0000_0001,
    0x1234_5678_9ABC_DEF0,
    0xDEAD_BEEF_CAFE_BABE,
    0xFEED_FACE_0123_4567,
    0x0F0F_0F0F_F0F0_F0F0,
];

/// Worker counts to prove "doesn't depend on num_cpus".
pub const WORKER_COUNTS: &[usize] = &[1, 2, 4, 8, 16, 32];

pub fn hex32(h: &Hash32) -> String {
    h.iter().map(|b| format!("{:02x}", b)).collect()
}

/// For comparing hashes with readable diffs.
pub fn assert_hash_eq(a: &Hash32, b: &Hash32, msg: &str) {
    if a != b {
        panic!(
            "{msg}\n  a: {}\n  b: {}",
            hex32(a),
            hex32(b)
        );
    }
}

/// A minimal test façade so tests don't hard-couple to your evolving BOAW API.
/// Implement this once (or provide a real harness builder).
pub trait TestHarness {
    type Snapshot;
    type IngressItem;
    type TickDelta;
    type WscBytes;

    /// Build a base snapshot (reachable-only) from a deterministic scenario.
    fn build_base_snapshot(&self, scenario: Scenario) -> Self::Snapshot;

    /// Generate canonical ingress for a scenario and tick.
    fn make_ingress(&self, scenario: Scenario, tick: u64) -> Vec<Self::IngressItem>;

    /// Execute with 1 worker (serial path), returning (commit_hash, state_root, patch_digest, delta, wsc_bytes?)
    fn execute_serial(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        tick: u64,
    ) -> ExecResult<Self::TickDelta, Self::WscBytes>;

    /// Execute with N workers (parallel path)
    fn execute_parallel(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        tick: u64,
        workers: usize,
    ) -> ExecResult<Self::TickDelta, Self::WscBytes>;

    /// Optional: materialize a WSC snapshot from an exec result (or already included in ExecResult).
    fn wsc_roundtrip_state_root(&self, wsc: &Self::WscBytes) -> Hash32;
}

/// Results you should be able to compare deterministically.
#[derive(Clone)]
pub struct ExecResult<TickDelta, WscBytes> {
    pub commit_hash: Hash32,
    pub state_root: Hash32,
    pub patch_digest: Hash32,
    pub delta: TickDelta,
    pub wsc: Option<WscBytes>,
}

/// Deterministic scenarios so we can scale tests without random blobs.
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    /// Tiny graph with edges/attachments; good for correctness.
    Small,

    /// Lots of independent rewrites; good for throughput/parallel admission.
    ManyIndependent,

    /// High collision rate; ensures admission/rejection is deterministic.
    ManyConflicts,

    /// Deletes/unlinks + attachments; exercises unlink semantics.
    DeletesAndAttachments,

    /// Privacy claims/proofs; mind-mode rules.
    PrivacyClaims,
}

/// Temporary default harness so tests compile immediately.
/// Replace this by constructing your real engine harness.
pub fn harness() -> impl TestHarness {
    PanicHarness
}

struct PanicHarness;

impl TestHarness for PanicHarness {
    type Snapshot = ();
    type IngressItem = ();
    type TickDelta = ();
    type WscBytes = Vec<u8>;

    fn build_base_snapshot(&self, _scenario: Scenario) -> Self::Snapshot {
        ()
    }

    fn make_ingress(&self, _scenario: Scenario, _tick: u64) -> Vec<Self::IngressItem> {
        vec![()]
    }

    fn execute_serial(
        &self,
        _base: &Self::Snapshot,
        _ingress: &[Self::IngressItem],
        _tick: u64,
    ) -> ExecResult<Self::TickDelta, Self::WscBytes> {
        panic!("wire TestHarness::execute_serial to BOAW engine")
    }

    fn execute_parallel(
        &self,
        _base: &Self::Snapshot,
        _ingress: &[Self::IngressItem],
        _tick: u64,
        _workers: usize,
    ) -> ExecResult<Self::TickDelta, Self::WscBytes> {
        panic!("wire TestHarness::execute_parallel to BOAW engine")
    }

    fn wsc_roundtrip_state_root(&self, _wsc: &Self::WscBytes) -> Hash32 {
        panic!("wire TestHarness::wsc_roundtrip_state_root to WSC reader")
    }
}
```

---

## `tests/boaw_end_to_end.rs` (the “god test”)

```rust
// crates/warp-core/tests/boaw_end_to_end.rs
mod common;

use common::*;

#[test]
fn boaw_end_to_end_is_deterministic_across_permutations_and_workers() {
    let h = harness();
    let scenario = Scenario::ManyIndependent;
    let base = h.build_base_snapshot(scenario);

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let tick = 42;

        let mut ingress = h.make_ingress(scenario, tick);
        // Permute ingress to prove canonicalization doesn't care about arrival order.
        for _ in 0..20 {
            shuffle(&mut rng, &mut ingress);

            // Reference run: serial
            let r0 = h.execute_serial(&base, &ingress, tick);

            // Parallel runs: varying worker counts
            for &workers in WORKER_COUNTS {
                let rp = h.execute_parallel(&base, &ingress, tick, workers);

                assert_hash_eq(&r0.state_root, &rp.state_root, "state_root differs across worker counts");
                assert_hash_eq(&r0.patch_digest, &rp.patch_digest, "patch_digest differs across worker counts");
                assert_hash_eq(&r0.commit_hash, &rp.commit_hash, "commit_hash differs across worker counts");

                // If WSC bytes are produced, verify zero-copy roundtrip yields same state_root.
                if let Some(wsc) = &rp.wsc {
                    let root2 = h.wsc_roundtrip_state_root(wsc);
                    assert_hash_eq(&rp.state_root, &root2, "WSC roundtrip state_root mismatch");
                }
            }
        }
    }
}
```

---

## `tests/boaw_determinism.rs` (serial vs parallel + insertion order)

```rust
// crates/warp-core/tests/boaw_determinism.rs
mod common;

use common::*;

#[test]
fn serial_equals_parallel_for_small_scenario() {
    let h = harness();
    let scenario = Scenario::Small;
    let base = h.build_base_snapshot(scenario);
    let tick = 1;

    let ingress = h.make_ingress(scenario, tick);
    let r0 = h.execute_serial(&base, &ingress, tick);
    let rp = h.execute_parallel(&base, &ingress, tick, 8);

    assert_hash_eq(&r0.state_root, &rp.state_root, "state_root differs");
    assert_hash_eq(&r0.patch_digest, &rp.patch_digest, "patch_digest differs");
    assert_hash_eq(&r0.commit_hash, &rp.commit_hash, "commit_hash differs");
}

#[test]
fn admission_and_results_do_not_depend_on_arrival_order() {
    let h = harness();
    let scenario = Scenario::ManyConflicts;
    let base = h.build_base_snapshot(scenario);
    let tick = 7;

    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = h.make_ingress(scenario, tick);

        // Baseline
        let r_base = h.execute_parallel(&base, &ingress, tick, 8);

        for _ in 0..50 {
            shuffle(&mut rng, &mut ingress);
            let r = h.execute_parallel(&base, &ingress, tick, 8);

            assert_hash_eq(&r_base.state_root, &r.state_root, "state_root differs across permutations");
            assert_hash_eq(&r_base.patch_digest, &r.patch_digest, "patch_digest differs across permutations");
            assert_hash_eq(&r_base.commit_hash, &r.commit_hash, "commit_hash differs across permutations");
        }
    }
}
```

---

## `tests/boaw_cow.rs` (unlink + overlay precedence + sharing)

```rust
// crates/warp-core/tests/boaw_cow.rs
mod common;

use common::*;

#[test]
fn delete_is_unlink_not_physical_delete() {
    // This test asserts the ADR contract:
    // - base snapshot remains unchanged
    // - overlay removes visibility in the next snapshot/view
    //
    // Wire this once you have Snapshot + Overlay view resolution.
    let _h = harness();
    todo!("Implement: build base with node X, apply overlay delete, assert view hides X but base still has X");
}

#[test]
fn overlay_wins_over_base_reads() {
    let _h = harness();
    todo!("Implement: base has attachment A; overlay sets A; reads return overlay; commit includes overlay");
}

#[test]
fn structural_sharing_reuses_unchanged_segments() {
    let _h = harness();
    todo!("Implement: commit C0 then C1 with tiny change; verify segment manifest reuses most segments");
}
```

---

## `tests/boaw_footprints.rs` (independence + bucket enforcement + drift guard)

```rust
// crates/warp-core/tests/boaw_footprints.rs
mod common;

use common::*;

#[test]
fn footprint_independence_is_symmetric() {
    // If you already have Footprint in warp-core, wire it here.
    // This test should be pure and fast.
    todo!("Implement: generate random-ish Footprints deterministically; assert fpA.independent(fpB) == fpB.independent(fpA)");
}

#[test]
fn deletes_that_share_adjacency_bucket_must_conflict() {
    // The classic race: delete e1=(A->B) and e2=(A->C) both mutate edges_from[A].
    // Your footprint model must claim the bucket target (e.g., AttachmentKey::EdgesFromBucket(A)).
    todo!("Implement: build two PlannedRewrites; assert admission rejects running both concurrently");
}

#[test]
fn footprint_guard_catches_executor_drift() {
    // Executor emits op to mutate target not claimed in footprint => must fail deterministically.
    todo!("Implement: run executor under FootprintGuard; attempt forbidden write; assert panic/error");
}
```

---

## `tests/boaw_merge.rs` (collapse/merge + conflict artifacts)

```rust
// crates/warp-core/tests/boaw_merge.rs
mod common;

use common::*;

#[test]
fn commutative_merge_is_parent_order_invariant() {
    // For mergeable types (commutative+associative), parent order must not matter.
    todo!("Implement: create two parent commits with commutative attachment merges; merge in both orders; commit_hash equal");
}

#[test]
fn order_dependent_merge_uses_canonical_parent_order() {
    // If you support order-dependent merges, the engine must canonicalize parent ordering.
    todo!("Implement: make an order-dependent merge type; verify parent ordering by commit hash yields stable result");
}

#[test]
fn irreconcilable_conflicts_produce_deterministic_conflict_artifact() {
    todo!("Implement: two parents write different non-mergeable values to same key; merge yields conflict artifact (bytes + hash stable)");
}
```

---

## `tests/boaw_privacy.rs` (mind mode enforcement + claim merging)

```rust
// crates/warp-core/tests/boaw_privacy.rs
mod common;

use common::*;

#[test]
fn mind_mode_forbids_forbidden_in_ledger_atoms() {
    todo!("Implement: in mind mode, attempt to emit ForbiddenInLedger atom bytes; assert deterministic rejection or forced indirection");
}

#[test]
fn invalid_proofs_do_not_win_claim_merge() {
    todo!("Implement: same claim_key, one valid proof, one invalid; merge keeps valid, quarantines invalid deterministically");
}

#[test]
fn conflicting_valid_claims_produce_conflict_artifact() {
    todo!("Implement: same claim_key, two different statement_hash, both verify => conflict artifact unless policy resolves");
}

#[test]
fn commitment_is_dictionary_safe_with_pepper() {
    // Ensure commitment changes if pepper changes, and cannot be reproduced from ledger alone.
    todo!("Implement: commit(secret, pepper1) != commit(secret, pepper2)");
}
```

---

## Next: wiring strategy (fastest path)

Implement TestHarness against your real engine in one place:

- build a base snapshot (maybe from current GraphStore -> WSC builder)
- generate ingress
- call execute_*
- return ExecResult with real hashes
- implement wsc_roundtrip_state_root via WscFile/WarpView + compute_state_root

Once that’s in, the tests stop being “ideas” and start being the drill sergeant you wanted.

---

## Sequencing

- [x] 1) First commit: add the BOAW ADR + tests (red on purpose)

  - [x] Add the ADR (single doc with the COW section folded in).
  - [x] Add the test skeletons exactly like above.
  - [x] Don't touch engine code yet.
  - [x] Goal: establish the contract + the drill sergeant.

- [x] 2) Second commit: wire the TestHarness to whatever you have now

Don't build BOAW yet. Just make the harness call your current pipeline:

- [x] base snapshot builder (even if it's GraphStore → canonical hash)
- [x] execute_serial / execute_parallel can both call serial for now
- [x] return real hashes so the test runner is alive

Goal: tests compile, fail only where you todo!().

- [x] 3) Third commit: flip executors to "emit ops" (TickDelta)

This is the real pivot:

- [x] Change ExecuteFn signature to take:
- [x] `&GraphView` (read-only view of snapshot)
- [x] `&mut TickDelta` (append-only ops)
- [x] Implement minimal TickDelta + canonical merge (sort ops by key).

Goal: parallelism becomes safe because writes are thread-local.

- [x] 4) Fourth commit: SnapshotBuilder v0 (no segment sharing yet)
  - [x] Apply merged ops to produce next snapshot tables.
  - [x] Write WSC bytes.
  - [x] Compute state_root from that snapshot.
  - [x] Make the god test pass for Small + ManyIndependent.

Goal: you now have "immutable snapshot + delta commit" working.

- [x] 5) Read-only execution (Phase 5)
  - [x] Executors receive `GraphView` (read-only) instead of `&mut GraphStore`.
  - [x] No GraphStore mutations during execution — emit ops only.
  - [x] State updated after execution via `apply_to_state()`.
  - [x] Legacy `&mut GraphStore` path removed from executor signature.
  - [x] `#![forbid(unsafe_code)]` required in all crates containing executors.

Goal: execution is pure — reads from snapshot, writes to delta.

- [~] 6) Only then: parallelism + footprints + shards
  - [x] Make admission deterministic. (RadixScheduler, Phase 5)
  - [x] Add worker execution producing per-worker deltas. (`boaw::execute_parallel`)
  - [x] Merge deltas canonically. (`boaw::merge_deltas` — sort by WarpOpKey, OpOrigin)
  - [x] Prove worker-count invariance (the tests). (7 tests in `boaw_parallel_exec.rs`)
  - [ ] Wire into Engine pipeline (Phase 6B)
  - [ ] Virtual shards for locality (Phase 6B)

Goal: "free money" without compromising determinism.
**Status: Phase 6A COMPLETE** (2026-01-18) — parallel exec + canonical merge proven.

---

Then add the "next features"...

- Forking,
- collapse/merge,
- privacy claims,
- SWS,
- etc.

...become clean additions instead of surgery.

---

## The one warning I’ll insist on

> Do not try to build segment-level structural sharing on day one.

First get correct immutable snapshots and deterministic commits. Sharing is an optimization; correctness is the religion.

> If you hit a “where do I start” moment mid-implementation

Paste the current Engine::apply_reserved_rewrites / executor signature and I’ll tell you the smallest surgical change to make it emit TickDelta without detonating the rest of warp-core.
