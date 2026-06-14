<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0028 — Strand Typestates, Blinded References, Proof Envelopes, and Evolving Braids

_Close the remaining gaps in the warp-core specs for AION Paper VIII / Continuum: enforce causal posture guarantees at the type level with Strand typestates, blind member identities in braid shells for unlinkable verification, wrap ZK/Verkle proofs in explicit verification envelopes, and implement the event log folder for evolving braids._

Legend: `PLATFORM`

Status: **approved (James review, 2026-06-14) — RED next**

> Statically preventing a non-Shared strand from entering settlement is not a runtime validation; it is a compilation invariance. Combined with ZK-honest claims and blinded references, we make the braid a zero-knowledge boundary. — review verdict

## Doctrine

AIΩN Paper VIII (Continuum):

- **Prop 5.1 (Typestate Partitioning)** — Causal posture transitions (e.g. `Scratch` → `AuthorOnly` → `Shared`) form a one-way lattice. Executions or operations requesting global settlement must statically prove they act on a `Shared` posture, guaranteeing no un-revalidated local context leaks.
- **§3.4 (Zero-Knowledge Braid Boundaries)** — To maintain participant privacy and prevent linkability across independent braids, membership reference identities in public braid shells must be sealable. Verifiers should check the validity of a braid's members using blinded domain-separated commitments.
- **§6.2 (Verkle/ZK Envelopes)** — Any braid shell claiming validity under zero-knowledge or Verkle space constraints must encapsulate its validation claims within an explicit `ProofEnvelope` validating an `ObserverHonestyClaim`.

## Current state (verified @14c89ef6)

All four key gaps from the Echo codebase gap analysis have been fully implemented, tested, and integrated:

1. **Strand Typestates (`revelation.rs`, `strand.rs`):**
    - Parameterized `Strand<P: CausalPostureState = DynamicPosture>` to statically guarantee posture constraints at compile time.
    - Built infallible `into_dynamic(self)` and fallible `try_into_shared(self)` conversions.
    - Gated `plan` and `settle` methods statically on `Strand<Shared>`, ensuring non-Shared strands cannot be planned or settled.
2. **Blinded Member References (`braid_shell.rs`):**
    - Refactored `BraidShellMember` to store a `BraidMemberRef` instead of a plain `StrandId`.
    - `BraidMemberRef` supports `Revealed(StrandId)` and `Sealed(Hash)` variants.
    - Sealed variants commit to the `StrandId` using a domain-separated `blake3` commitment: `BLAKE3("braid-member-seal:" || strand_id)`.
3. **ZK/Verkle Proof Envelopes (`proof.rs`, `braid_shell.rs`):**
    - Defined `ProofKind` (ZK, Verkle, Merkle, Custom), `ProofEnvelope`, and `ObserverHonestyClaim`.
    - Added `BraidShell::assemble_with_proof` to attach envelopes and enforce validation checks.
4. **Evolving Braid Logs (`braid.rs`):**
    - Created `BraidEvent` representing state transition logs (`Created`, `MemberWoven`, `SettlementFinalized`).
    - Implemented event folding logic in the `Braid` state struct with strict duplicate and out-of-order event checks.

---

## Technical Specifications

### 1. Causal Posture Typestates

We define the typestate traits and marker structs to represent the four causal posture states:

```rust
pub trait CausalPostureState: private::Sealed {}

pub struct Shared;
pub struct AuthorOnly;
pub struct Scratch;
pub struct DynamicPosture;

impl CausalPostureState for Shared {}
impl CausalPostureState for AuthorOnly {}
impl CausalPostureState for Scratch {}
impl CausalPostureState for DynamicPosture {}
```

The `Strand` struct is parameterized with `P: CausalPostureState`, defaulting to `DynamicPosture` to maintain backwards compatibility inside the `StrandRegistry` (which uses `BTreeMap<StrandId, Strand<DynamicPosture>>`):

```rust
pub struct Strand<P: CausalPostureState = DynamicPosture> {
    pub strand_id: StrandId,
    pub fork_basis_ref: ForkBasisRef,
    pub child_worldline_id: WorldlineId,
    pub writer_heads: Vec<WriterHeadKey>,
    pub support_pins: Vec<SupportPin>,
    pub retention_posture: RetentionPosture,
    pub _marker: std::marker::PhantomData<P>,
}
```

Static gating on `SettlementService` guarantees that only `Shared` strands can enter planning or settlement:

```rust
impl Strand<Shared> {
    pub fn plan(&self, ...) -> Result<SettlementPlan, SettlementError> {
        SettlementService::plan_with_policy_internal(..., self, ...)
    }

    pub fn settle(&self, ...) -> Result<SettlementResult, SettlementError> {
        SettlementService::settle_with_policy_internal(..., self, ...)
    }
}
```

---

### 2. Blinded Member References

Member references inside the public `BraidShell` can be sealed to protect user/strand identity:

```rust
pub enum BraidMemberRef {
    Revealed(StrandId),
    Sealed(Hash),
}

impl BraidMemberRef {
    pub fn seal(strand_id: StrandId) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"braid-member-seal:");
        hasher.update(strand_id.as_bytes());
        Self::Sealed(Hash::from_bytes(hasher.finalize().into()))
    }

    pub fn matches(&self, strand_id: StrandId) -> bool {
        match self {
            Self::Revealed(r) => *r == strand_id,
            Self::Sealed(h) => {
                let expected = Self::seal(strand_id);
                match expected {
                    Self::Sealed(expected_hash) => *h == expected_hash,
                    Self::Revealed(_) => unreachable!(),
                }
            }
        }
    }
}
```

---

### 3. ZK/Verkle Proof Envelopes

A `ProofEnvelope` contains the observer honesty claim and the cryptographic proof:

```rust
pub enum ProofKind {
    ZeroKnowledge,
    Verkle,
    Merkle,
    Custom(String),
}

pub struct ProofEnvelope {
    pub kind: ProofKind,
    pub honesty_claim: ObserverHonestyClaim,
    pub proof_bytes: Vec<u8>,
}

pub struct ObserverHonestyClaim {
    pub observer_id: ActorId,
    pub braid_id: Hash,
    pub state_root: Hash,
}
```

Validation occurs during shell assembly:

```rust
impl BraidShell {
    pub fn assemble_with_proof(
        mut self,
        proof: ProofEnvelope,
    ) -> Result<Self, SettlementError> {
        if proof.honesty_claim.braid_id != self.coordinate.as_hash() {
            return Err(SettlementError::ProofValidation("braid_id mismatch"));
        }
        self.proof_envelope = Some(proof);
        Ok(self)
    }
}
```

---

### 4. Evolving Braid Logs

Evolving braids transition through discrete events folded sequentially:

```rust
pub enum BraidEvent {
    Created {
        braid_id: Hash,
        policy: PolicyId,
    },
    MemberWoven {
        member: BraidMemberRef,
        frontier: Hash,
    },
    SettlementFinalized {
        outcome_digest: Hash,
    },
}

pub struct Braid {
    pub braid_id: Hash,
    pub policy: PolicyId,
    pub members: Vec<BraidMemberRef>,
    pub status: BraidStatus,
    pub version: u64,
}
```

Folding a log checks for invariants such as duplicate membership, out-of-order events, and correct starting events.
