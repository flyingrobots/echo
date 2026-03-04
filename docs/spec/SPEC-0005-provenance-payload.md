<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005: Provenance Payload

**Status:** Draft
**Authors:** James Ross
**Prerequisite:** SPEC-0004 (Worldlines, Playback, TruthBus)
**Blocks:** PP-2 (Implementation), Time Travel Debugging

---

## 1. Purpose

This specification translates the provenance formalism from Paper III (AION
Foundations) into concrete Echo types. It defines the data structures needed
to answer "show me why" queries — tracing any observed state back through the
causal chain of tick patches that produced it.

### Scope

- **In scope:** Type definitions, wire format, composition rules, bridge to
  existing APIs, attestation envelope structure.
- **Out of scope:** Implementation (PP-2+), storage tiers (echo-cas), network
  transport, consensus protocols.

---

## 2. Glossary Mapping — Paper III → Echo

| Paper III Symbol               | Paper III Name           | Echo Type                                           | Location                            | Status                       |
| ------------------------------ | ------------------------ | --------------------------------------------------- | ----------------------------------- | ---------------------------- |
| `μ_i`                          | TickPatch                | `WorldlineTickPatchV1`                              | `warp-core/src/worldline.rs`        | **Exists**                   |
| `P = (μ₀, …, μₙ₋₁)`            | ProvenancePayload        | `ProvenancePayload`                                 | —                                   | **New**                      |
| `(U₀, P)`                      | BoundaryEncoding         | `(WarpId, ProvenancePayload)` via `ProvenanceStore` | `warp-core/src/provenance_store.rs` | **Partial**                  |
| `BTR`                          | BoundaryTransitionRecord | `BoundaryTransitionRecord`                          | —                                   | **New**                      |
| `H(μ)`                         | TickPatchDigest          | `WorldlineTickPatchV1::patch_digest`                | `worldline.rs`                      | **Exists**                   |
| `(h_state, h_patch, h_commit)` | HashTriplet              | `HashTriplet`                                       | `worldline.rs`                      | **Exists**                   |
| `ρ`                            | Trace / Receipt          | `TickReceipt`                                       | `warp-core/src/receipt.rs`          | **Exists** (needs extension) |
| `In(μ)`                        | Input slots              | `WorldlineTickPatchV1::in_slots: Vec<SlotId>`       | `worldline.rs`                      | **Exists**                   |
| `Out(μ)`                       | Output slots             | `WorldlineTickPatchV1::out_slots: Vec<SlotId>`      | `worldline.rs`                      | **Exists**                   |
| `𝕡`                            | Provenance graph         | `ProvenanceGraph`                                   | —                                   | **New** (algorithm)          |
| `D(v)`                         | Derivation graph         | `DerivationGraph`                                   | —                                   | **New** (algorithm)          |
| `W`                            | Worldline                | `WorldlineId`                                       | `worldline.rs`                      | **Exists**                   |
| `U₀`                           | Initial state ref        | `WarpId` (via `ProvenanceStore::u0()`)              | `provenance_store.rs`               | **Exists**                   |
| `κ`                            | Policy ID                | `WorldlineTickHeaderV1::policy_id: u32`             | `worldline.rs`                      | **Exists**                   |
| `t`                            | Global tick              | `WorldlineTickHeaderV1::global_tick: u64`           | `worldline.rs`                      | **Exists**                   |
| `α(v)`                         | AtomWrite                | `AtomWrite`                                         | `worldline.rs`                      | **Exists**                   |
| `checkpoint(t)`                | State checkpoint         | `CheckpointRef`                                     | `provenance_store.rs`               | **Exists**                   |

---

## 3. Inventory — Existing vs. New

### 3.1 Existing Types (no changes required)

| Type                      | Role in PP-1                                                                                             |
| ------------------------- | -------------------------------------------------------------------------------------------------------- |
| `WorldlineTickPatchV1`    | The atomic unit of provenance — one tick's delta for one warp. Contains ops, slot I/O, and patch digest. |
| `WorldlineTickHeaderV1`   | Shared tick metadata: global_tick, policy_id, rule_pack_id, plan/decision/rewrites digests.              |
| `HashTriplet`             | Three-way commitment `(state_root, patch_digest, commit_hash)` for verification.                         |
| `WorldlineId`             | Identifies a worldline (history branch).                                                                 |
| `AtomWrite`               | Causal arrow: records which rule mutated which atom at which tick, with old/new values.                  |
| `ProvenanceStore` (trait) | History access: retrieve patches, expected hashes, outputs, checkpoints per worldline.                   |
| `LocalProvenanceStore`    | In-memory `BTreeMap`-backed implementation of `ProvenanceStore`.                                         |
| `CheckpointRef`           | Fast-seek anchor: `(tick, state_hash)`.                                                                  |
| `TickReceipt`             | Candidate outcomes: applied vs. rejected, with blocking causality via `blocked_by`.                      |
| `TickReceiptEntry`        | Per-candidate record: `(rule_id, scope_hash, scope, disposition)`.                                       |
| `SlotId`                  | Abstract resource identifier: `Node`, `Edge`, `Attachment`, or `Port`.                                   |
| `WarpOp`                  | Canonical delta operation (8 variants: upsert/delete node/edge, set attachment, portal, instance).       |
| `OutputFrameSet`          | Ordered channel outputs: `Vec<(ChannelId, Vec<u8>)>`.                                                    |
| `CursorReceipt`           | Provenance envelope for truth delivery: `(session, cursor, worldline, warp, tick, commit_hash)`.         |
| `TruthFrame`              | Authoritative value with provenance: `(CursorReceipt, channel, value, value_hash)`.                      |

### 3.2 New Types (defined in this spec)

| Type                                | Role in PP-1                                                                          | Section |
| ----------------------------------- | ------------------------------------------------------------------------------------- | ------- |
| `ProvenancePayload`                 | Ordered sequence of tick patches — the "proof" that transforms U₀ into current state. | §4.1    |
| `BoundaryTransitionRecord`          | Tamper-evident envelope binding input hash, output hash, payload, and policy.         | §4.2    |
| `ProvenanceNode` / `ProvenanceEdge` | Graph nodes/edges for the provenance graph `𝕡`.                                       | §4.3    |
| `DerivationGraph`                   | Backward causal cone algorithm specification.                                         | §4.4    |

### 3.3 Extensions to Existing Types

| Type                   | Extension                                                                                                                          | Rationale                                                                                                            |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `TickReceipt`          | Add `blocking_poset: Vec<Vec<u32>>` (already exists as `blocked_by`). Extend `TickReceiptRejection` with richer rejection reasons. | Paper III trace `ρ` requires detailed rejection causality.                                                           |
| `TickReceiptRejection` | Add: `GuardFailure`, `PreconditionViolation`, `ResourceContention`.                                                                | Current `FootprintConflict` is the only rejection reason; richer reasons enable "show me why this rule didn't fire". |

---

## 4. New Type Definitions

### 4.1 ProvenancePayload

The provenance payload is an ordered sequence of tick patches that, applied
sequentially to an initial state `U₀`, deterministically reproduce the current
state.

```rust
/// Ordered sequence of tick patches forming a provenance proof.
///
/// Invariant: patches[i].header.global_tick == i (zero-indexed from
/// the worldline's registration tick, contiguous, no gaps).
///
/// Paper III: P = (μ₀, μ₁, …, μₙ₋₁)
pub struct ProvenancePayload {
    /// The worldline this payload belongs to.
    pub worldline_id: WorldlineId,
    /// Initial state reference (MVP: WarpId).
    pub u0: WarpId,
    /// Ordered tick patches. Must be contiguous and zero-gap.
    pub patches: Vec<WorldlineTickPatchV1>,
    /// Corresponding hash triplets for each tick (verification anchors).
    pub expected: Vec<HashTriplet>,
}
```

**Monoid structure (composition):**

```text
compose(P₁, P₂) = ProvenancePayload {
    worldline_id: P₁.worldline_id,  // must match P₂
    u0: P₁.u0,
    patches: P₁.patches ++ P₂.patches,
    expected: P₁.expected ++ P₂.expected,
}
```

- Identity: empty payload `(worldline_id, u0, [], [])`.
- Associativity: concatenation is associative.
- Precondition: `P₁.worldline_id == P₂.worldline_id` and
  last tick of `P₁` + 1 == first tick of `P₂` (contiguity).

**Construction from `LocalProvenanceStore`:**

```rust
impl ProvenancePayload {
    pub fn from_store(
        store: &impl ProvenanceStore,
        worldline_id: WorldlineId,
        tick_range: Range<u64>,
    ) -> Result<Self, HistoryError> {
        let u0 = store.u0(worldline_id)?;
        let mut patches = Vec::new();
        let mut expected = Vec::new();
        for tick in tick_range {
            patches.push(store.patch(worldline_id, tick)?);
            expected.push(store.expected(worldline_id, tick)?);
        }
        Ok(Self { worldline_id, u0, patches, expected })
    }
}
```

### 4.2 BoundaryTransitionRecord (BTR)

A tamper-evident envelope that commits to:

- The state before (`h_in` — state root at tick start)
- The state after (`h_out` — state root at tick end)
- The full provenance payload
- The policy under which the transition was evaluated

```rust
/// Tamper-evident record of a state transition boundary.
///
/// Paper III: BTR = (h_in, h_out, U₀, P, t, κ)
///
/// The BTR is the unit of trust for replay verification: given h_in,
/// a verifier can replay P and confirm h_out matches.
pub struct BoundaryTransitionRecord {
    /// State root hash before the transition.
    pub h_in: Hash,
    /// State root hash after the transition.
    pub h_out: Hash,
    /// Initial state reference.
    pub u0: WarpId,
    /// The provenance payload (ordered patches).
    pub payload: ProvenancePayload,
    /// Global tick at transition boundary.
    pub tick: u64,
    /// Policy ID governing the transition.
    pub policy_id: u32,
    /// Commit hash binding all fields.
    pub commit_hash: Hash,
}
```

**Verification algorithm:**

```text
verify_btr(btr, initial_store):
    1. store ← clone(initial_store)
    2. assert canonical_state_hash(store) == btr.h_in
    3. for each patch in btr.payload.patches:
        a. patch.apply_to_store(&mut store)
        b. assert canonical_state_hash(store) == btr.payload.expected[i].state_root
    4. assert canonical_state_hash(store) == btr.h_out
    5. recompute commit_hash from (h_out, parents, patch_digest, policy_id)
    6. assert recomputed == btr.commit_hash
```

### 4.3 Provenance Graph Nodes and Edges

The provenance graph `𝕡` connects tick patches through their slot I/O:
if `Out(μ_i)` ∩ `In(μ_j)` ≠ ∅, there is a causal edge from `μ_i` to `μ_j`.

```rust
/// A node in the provenance graph.
///
/// Each node represents one tick patch in one worldline.
pub struct ProvenanceNode {
    pub worldline_id: WorldlineId,
    pub tick: u64,
    pub patch_digest: Hash,
    pub in_slots: Vec<SlotId>,
    pub out_slots: Vec<SlotId>,
}

/// A directed edge in the provenance graph.
///
/// Represents a causal dependency: the source tick produced slots
/// that the target tick consumed.
pub struct ProvenanceEdge {
    /// Source tick (producer).
    pub from: (WorldlineId, u64),
    /// Target tick (consumer).
    pub to: (WorldlineId, u64),
    /// The slots that connect them (Out(from) ∩ In(to)).
    pub shared_slots: Vec<SlotId>,
}
```

**Construction algorithm:**

```text
build_provenance_graph(store, worldline_id, tick_range):
    nodes ← []
    edges ← []
    for tick in tick_range:
        patch ← store.patch(worldline_id, tick)
        node ← ProvenanceNode {
            worldline_id, tick,
            patch_digest: patch.patch_digest,
            in_slots: patch.in_slots,
            out_slots: patch.out_slots,
        }
        nodes.push(node)

        // Find causal predecessors.
        for prev_tick in (0..tick).rev():
            prev_patch ← store.patch(worldline_id, prev_tick)
            shared ← intersect(prev_patch.out_slots, patch.in_slots)
            if !shared.is_empty():
                edges.push(ProvenanceEdge {
                    from: (worldline_id, prev_tick),
                    to: (worldline_id, tick),
                    shared_slots: shared,
                })

    return (nodes, edges)
```

**Optimization note:** In practice, maintain a slot→tick index to avoid the
O(n²) backward scan. The naive algorithm is shown for specification clarity.

### 4.4 Derivation Graph — Backward Causal Cone

The derivation graph `D(v)` for a slot `v` at tick `t` is the backward
transitive closure of the provenance graph, restricted to patches that
contributed (directly or transitively) to the value of `v`.

```rust
/// Backward causal cone for a specific slot at a specific tick.
///
/// Paper III: D(v) = transitive closure of 𝕡 backward from v.
pub struct DerivationGraph {
    /// The query: which slot's provenance are we tracing?
    pub query_slot: SlotId,
    /// The tick at which the query is evaluated.
    pub query_tick: u64,
    /// Provenance nodes in the backward cone (topologically sorted).
    pub nodes: Vec<ProvenanceNode>,
    /// Causal edges within the cone.
    pub edges: Vec<ProvenanceEdge>,
}
```

**Algorithm:**

```text
derive(store, worldline_id, slot, tick):
    frontier ← { (worldline_id, tick) }
    visited ← {}
    result_nodes ← []
    result_edges ← []

    while frontier is not empty:
        (wl, t) ← frontier.pop()
        if (wl, t) in visited: continue
        visited.insert((wl, t))

        patch ← store.patch(wl, t)
        if slot not in patch.out_slots and (wl, t) != (worldline_id, tick):
            continue  // This tick didn't produce anything we care about.

        node ← ProvenanceNode from patch
        result_nodes.push(node)

        // Trace backward through in_slots.
        for in_slot in patch.in_slots:
            for prev_tick in (0..t).rev():
                prev_patch ← store.patch(wl, prev_tick)
                if in_slot in prev_patch.out_slots:
                    result_edges.push(ProvenanceEdge {
                        from: (wl, prev_tick),
                        to: (wl, t),
                        shared_slots: [in_slot],
                    })
                    frontier.insert((wl, prev_tick))
                    break  // Found the most recent producer.

    return DerivationGraph {
        query_slot: slot,
        query_tick: tick,
        nodes: topological_sort(result_nodes),
        edges: result_edges,
    }
```

---

## 5. Wire Format

### 5.1 Encoding Rules

All provenance types use canonical CBOR encoding, consistent with warp-core's
`ciborium` conventions:

- **Integer encoding:** Minimal-length CBOR integers.
- **Map keys:** Sorted lexicographically (canonical CBOR).
- **Byte strings:** Raw `[u8; 32]` for hashes (no hex encoding on wire).
- **Arrays:** CBOR definite-length arrays.

### 5.2 Domain Separation Tags

Each type gets a unique domain separator for hash computation, consistent
with `warp_core::domain`:

| Type                            | Domain Tag                     | Bytes |
| ------------------------------- | ------------------------------ | ----- |
| `ProvenancePayload` digest      | `echo:provenance_payload:v1\0` | 27    |
| `BoundaryTransitionRecord` hash | `echo:btr:v1\0`                | 12    |
| `ProvenanceEdge` identifier     | `echo:provenance_edge:v1\0`    | 24    |

These tags MUST be added to `crates/warp-core/src/domain.rs` during
implementation (PP-2).

### 5.3 ProvenancePayload Digest

```text
provenance_payload_digest = BLAKE3(
    "echo:provenance_payload:v1\0"
    worldline_id: [u8; 32]
    u0: [u8; 32]
    num_patches: u64 (LE)
    for each patch:
        patch_digest: [u8; 32]
)
```

### 5.4 BTR Commit Hash

```text
btr_hash = BLAKE3(
    "echo:btr:v1\0"
    h_in: [u8; 32]
    h_out: [u8; 32]
    u0: [u8; 32]
    payload_digest: [u8; 32]
    tick: u64 (LE)
    policy_id: u32 (LE)
)
```

---

## 6. Worked Examples

### 6.1 Three-Tick Accumulator (Paper III Appendix A)

**Setup:** A single worldline with an accumulator node. Each tick increments
the accumulator by 1.

```text
Worldline W, U₀ = warp_id("acc")
  Tick 0: acc = 0 → acc = 1   (μ₀)
  Tick 1: acc = 1 → acc = 2   (μ₁)
  Tick 2: acc = 2 → acc = 3   (μ₂)
```

**ProvenancePayload:**

```text
P = {
  worldline_id: W,
  u0: warp_id("acc"),
  patches: [μ₀, μ₁, μ₂],
  expected: [
    HashTriplet { state_root: H(acc=1), patch_digest: H(μ₀), commit_hash: C₀ },
    HashTriplet { state_root: H(acc=2), patch_digest: H(μ₁), commit_hash: C₁ },
    HashTriplet { state_root: H(acc=3), patch_digest: H(μ₂), commit_hash: C₂ },
  ],
}
```

**BTR for tick 0→2:**

```text
BTR = {
  h_in: H(acc=0),      // state root at tick 0 start
  h_out: H(acc=3),     // state root at tick 2 end
  u0: warp_id("acc"),
  payload: P,
  tick: 2,
  policy_id: 0,
  commit_hash: BLAKE3("echo:btr:v1\0" || h_in || h_out || u0 || H(P) || 2u64 || 0u32),
}
```

**Provenance graph:**

```text
μ₀ → μ₁ → μ₂
(each tick's out_slots contain the accumulator node; each subsequent
 tick's in_slots consume it)
```

**Derivation of acc at tick 2:**

```text
D(acc) = { μ₀, μ₁, μ₂ }   // Full causal cone — every tick contributed.
```

### 6.2 Branching Fork with Shared Prefix

**Setup:** Two worldlines diverge at tick 3 from a common prefix.

```text
Worldline W₁:
  Tick 0-2: shared prefix (μ₀, μ₁, μ₂)
  Tick 3: branch A operation (μ₃ₐ)

Worldline W₂ (forked from W₁ at tick 2):
  Tick 0-2: inherited from W₁
  Tick 3: branch B operation (μ₃ᵦ)
```

**ProvenancePayloads:**

```text
P₁ = { worldline_id: W₁, u0, patches: [μ₀, μ₁, μ₂, μ₃ₐ], ... }
P₂ = { worldline_id: W₂, u0, patches: [μ₀, μ₁, μ₂, μ₃ᵦ], ... }
```

**Key property:** `P₁.patches[0..3] == P₂.patches[0..3]` (shared prefix).
The provenance graphs diverge at tick 3.

**Fork creation via `LocalProvenanceStore::fork()`:**

```rust
store.fork(
    source: W₁,
    fork_tick: 2,     // Fork after tick 2
    new_id: W₂,
)
```

This copies patches 0..2 from W₁ to W₂, then W₂ independently appends μ₃ᵦ.

---

## 7. Bridge to Existing APIs

### 7.1 LocalProvenanceStore::append() → ProvenancePayload

`append()` already stores per-tick patches, expected hash triplets, and
outputs. A `ProvenancePayload` is constructed by reading back a contiguous
range of ticks:

```rust
let payload = ProvenancePayload::from_store(
    &store,
    worldline_id,
    0..store.len(worldline_id)?,
)?;
```

No changes to `LocalProvenanceStore` are required for basic payload
construction.

### 7.2 ProvenancePayload → PlaybackCursor

The `PlaybackCursor` already supports seeking via `seek_to()`, which
internally replays patches from `ProvenanceStore`. A `ProvenancePayload` can
feed a cursor by wrapping it in a `ProvenanceStore` adapter:

```rust
impl ProvenanceStore for ProvenancePayload {
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError> { ... }
    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError> { ... }
    fn patch(&self, w: WorldlineId, tick: u64) -> Result<WorldlineTickPatchV1, HistoryError> { ... }
    fn expected(&self, w: WorldlineId, tick: u64) -> Result<HashTriplet, HistoryError> { ... }
    // outputs, checkpoint_before: delegate or return unavailable
}
```

This allows a `PlaybackCursor` to replay directly from a portable provenance
payload without a full `LocalProvenanceStore`.

### 7.3 TickReceipt Extensions

Current `TickReceiptRejection` has a single variant: `FootprintConflict`.
For "show me why" queries, richer rejection reasons are needed:

```rust
pub enum TickReceiptRejection {
    FootprintConflict,           // Existing
    GuardFailure,                // New: rule's guard predicate returned false
    PreconditionViolation,       // New: required state missing
    ResourceContention,          // New: write-write conflict on shared resource
}
```

**Migration path:** These are additive enum variants. Existing code matching
on `FootprintConflict` is unaffected. Wire format uses CBOR enum tags;
new variants get new tags (backward-compatible for decoders that ignore
unknown tags, forward-compatible for encoders).

### 7.4 Hash Commitment Compatibility

All new hash computations use BLAKE3 with domain separation, consistent with:

- `compute_state_root_for_warp_store()` — domain `echo:state_root:v1\0`
- `compute_commit_hash_v2()` — domain `echo:commit_id:v2\0`
- `compute_tick_commit_hash_v2()` — domain `tick_commit:v2`

New domain tags (§5.2) follow the same `echo:<type>:v1\0` convention.

**No existing hash commitments are changed.** All new types layer on top of
existing hashes without modifying them.

---

## 8. Attestation Envelope (PP Envelope)

The attestation envelope wraps a `BoundaryTransitionRecord` with
external claims and signatures. This is the publishable unit of provenance.

### 8.1 Structure

```rust
/// Provenance attestation envelope.
///
/// Wraps a BTR with external claims and cryptographic signatures.
/// This is the publishable, transferable unit of provenance.
pub struct ProvenanceEnvelope {
    /// Header: version, timestamp, envelope ID.
    pub header: EnvelopeHeader,
    /// The runtime provenance (BTR).
    pub btr: BoundaryTransitionRecord,
    /// External claims about the provenance.
    pub claims: Vec<ProvenanceClaim>,
    /// Cryptographic signatures over (header || btr_hash || claims_digest).
    pub signatures: Vec<EnvelopeSignature>,
}

pub struct EnvelopeHeader {
    pub version: u16,
    pub envelope_id: Hash,
    pub created_at: u64,  // Unix timestamp (seconds)
}
```

### 8.2 Claim Types

```rust
pub enum ProvenanceClaim {
    /// Identifies the build system that produced the simulation binary.
    BuiltBy {
        builder_id: String,
        build_hash: Hash,
    },
    /// References a parent BTR that this one was derived from.
    DerivedFrom {
        parent_btr_hash: Hash,
        relationship: DerivationRelationship,
    },
    /// Cryptographic identity of the signer.
    SignedBy {
        signer_id: String,
        public_key: Vec<u8>,
    },
    /// Human review attestation.
    ReviewedBy {
        reviewer_id: String,
        review_hash: Hash,
    },
}

pub enum DerivationRelationship {
    Fork,       // Branched from parent worldline
    Merge,      // Merged multiple worldlines
    Extension,  // Appended ticks to parent
}
```

### 8.3 SLSA Alignment

The `ProvenanceEnvelope` maps to SLSA v1.0 concepts:

| SLSA Concept       | Echo Mapping                   |
| ------------------ | ------------------------------ |
| Build provenance   | `BuiltBy` claim                |
| Source provenance  | `DerivedFrom` claim chain      |
| Verification       | BTR replay verification (§4.2) |
| Attestation bundle | `ProvenanceEnvelope`           |

Full SLSA compliance requires additional fields (builder identity URI,
build configuration digest) that are deferred to implementation.

### 8.4 BTR vs. Envelope

- **BTR** is _runtime provenance_: it records what happened during simulation
  execution. It is produced automatically by the engine.
- **Envelope** is _attestation provenance_: it wraps a BTR with external
  claims about who built it, who reviewed it, and what it was derived from.
  It is produced by tooling and humans.

---

## 9. Deviation Notes — Echo vs. Paper III

| Area                  | Paper III          | Echo                                                | Rationale                                                                                                                                                   |
| --------------------- | ------------------ | --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hash function         | Unspecified        | BLAKE3                                              | Performance; keyed mode for future MAC support.                                                                                                             |
| Patch encoding        | Abstract `μ`       | `WorldlineTickPatchV1` with concrete `Vec<WarpOp>`  | Echo's typed graph ops are the canonical encoding.                                                                                                          |
| Initial state         | Abstract `U₀`      | `WarpId` (MVP)                                      | Sufficient for single-warp worldlines. Multi-warp U₀ requires `WarpState` snapshot (future).                                                                |
| Slot model            | Abstract resources | `SlotId` enum: `Node`, `Edge`, `Attachment`, `Port` | Four concrete slot types cover Echo's graph model.                                                                                                          |
| Checkpoint            | Not in Paper III   | `CheckpointRef { tick, state_hash }`                | Pragmatic optimization for fast seeking in long worldlines.                                                                                                 |
| Receipt               | Abstract trace `ρ` | `TickReceipt` with `TickReceiptEntry` entries       | Concrete candidate outcomes with blocking causality.                                                                                                        |
| Attestation           | Not in Paper III   | `ProvenanceEnvelope` with SLSA alignment            | Extension for real-world trust chains.                                                                                                                      |
| Cross-worldline edges | Implicit           | Not yet implemented                                 | Provenance graph currently operates within a single worldline. Cross-worldline provenance edges require multi-worldline `ProvenanceStore` queries (future). |

---

## 10. Open Questions

1. **Multi-warp U₀:** When a worldline spans multiple warp instances, `WarpId`
   is insufficient as the initial state reference. Should `U₀` become a
   `Vec<(WarpId, Hash)>` (one state root per warp)?

2. **Provenance graph persistence:** Should the provenance graph be computed
   on-demand from `ProvenanceStore`, or materialized and stored? On-demand is
   simpler but O(n²) worst case; materialized requires storage management.

3. **Cross-worldline provenance:** When a fork creates a new worldline, the
   provenance graph should have edges from the source worldline to the fork.
   The current `ProvenanceEdge` type supports this via
   `(WorldlineId, tick)` tuples, but the construction algorithm (§4.3) only
   considers a single worldline. Multi-worldline traversal is deferred.

4. **Envelope signature scheme:** Which signature algorithm? Ed25519 is the
   pragmatic default, but the envelope should be algorithm-agnostic (include
   an algorithm identifier field).

---

## 11. Implementation Roadmap

| Phase | Deliverable                                                           | Depends On |
| ----- | --------------------------------------------------------------------- | ---------- |
| PP-2  | `ProvenancePayload` type + `from_store()` constructor + unit tests    | This spec  |
| PP-3  | `BoundaryTransitionRecord` type + verification algorithm              | PP-2       |
| PP-4  | `ProvenanceGraph` construction + `DerivationGraph` backward cone      | PP-3       |
| PP-5  | `TickReceiptRejection` extensions (additive)                          | PP-2       |
| PP-6  | `ProvenanceEnvelope` + claim types + signature verification           | PP-3       |
| PP-7  | Wire format (CBOR) + golden vector tests                              | PP-2, PP-3 |
| PP-8  | `ProvenancePayload` as `ProvenanceStore` adapter for `PlaybackCursor` | PP-2       |
