<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0028 — Strand Typestates, Blinded References, Proof Envelopes, and Evolving Braids

_Close the remaining gaps in the warp-core specs for AION Paper VIII / Continuum: carry causal posture through Strand typestates while keeping runtime posture validation authoritative, blind member identities in braid shells for unlinkable verification, wrap ZK/Verkle proofs in explicit verification envelopes, and implement the event log folder for evolving braids._

Legend: `PLATFORM`

Status: **draft / in-review**

> Typestates narrow the public settlement path, but the live registry and runtime posture checks remain the final admission law. Combined with proof-shaped claims and blinded references, the braid shell becomes an auditable privacy boundary. — review verdict

## Doctrine

AIΩN Paper VIII (Continuum):

- **Prop 5.1 (Typestate Partitioning)** — Causal posture transitions (e.g. `Scratch` → `AuthorOnly` → `Shared`) form a one-way lattice. Executions or operations requesting global settlement must carry `Shared` posture evidence and revalidate runtime state so no stale local context leaks.
- **§3.4 (Zero-Knowledge Braid Boundaries)** — To maintain participant privacy and prevent linkability across independent braids, membership reference identities in public braid shells must be sealable. Verifiers should check the validity of a braid's members using blinded domain-separated commitments.
- **§6.2 (Verkle/ZK Envelopes)** — Any braid shell carrying zero-knowledge or Verkle-style evidence must bind that evidence through an explicit `ProofEnvelope`. The current implementation validates replay-trace envelope shape and public-input binding; zero-knowledge and vector-opening proof kinds are reserved until verifier backends exist.

## Current state

All four key gaps from the Echo codebase gap analysis now have current E1 surfaces, tests, and explicit limits:

1. **Strand Typestates (`revelation.rs`, `strand.rs`):**
    - Parameterized `Strand<P: CausalPostureState = DynamicPosture>` so ordinary APIs carry posture intent at the type level while preserving runtime posture as the authoritative admission fact.
    - Built infallible `into_dynamic(self)` and fallible `try_into_shared(self)` conversions.
    - Exposed `Shared`-only `plan` and `settle` conveniences that re-enter the live registry path so stale, forged, or hand-built handles cannot bypass runtime posture and support validation.
2. **Blinded Member References (`braid_shell.rs`):**
    - Refactored `BraidShellMember` to store a `BraidMemberRef` instead of a plain `StrandId`.
    - `BraidMemberRef` supports `Revealed(StrandId)` and `Sealed { blinded_commitment, authority }` variants.
    - Sealed variants commit to the `StrandId`, child worldline, and caller-supplied non-public blinding material using a domain-separated `blake3` commitment.
3. **ZK/Verkle Proof Envelopes (`proof.rs`, `braid_shell.rs`):**
    - Defined `ProofKind` (`ZkSnark`, `ReplayTrace`, `VectorOpening`), `ProofEnvelope`, and `ObserverHonestyClaim`.
    - Added `BraidShell::assemble_with_proof` to attach replay-trace evidence envelopes, validate shape/public-input binding, reject cryptographic proof kinds without verifier backends, and bind the proof envelope digest into shell identity.
4. **Evolving Braid Logs (`braid.rs`):**
    - Created `BraidEvent` representing state transition logs (`BraidCreated`, `MemberWoven`, `SettlementFinalized`, `BraidCollapsed`).
    - Implemented checked incremental application and event folding with lifecycle, duplicate-member, sequence overflow, and collapse-witness checks.

---

## Technical Specifications

### 1. Causal Posture Typestates

We define the typestate traits and marker structs to represent the four causal posture states:

```rust
pub trait CausalPostureState: Clone + std::fmt::Debug + PartialEq + Eq {
    fn causal_posture() -> Option<CausalPosture>;
}

pub struct Shared;
pub struct AuthorOnly;
pub struct Scratch;
pub struct DynamicPosture;

impl CausalPostureState for Shared {
    fn causal_posture() -> Option<CausalPosture> {
        Some(CausalPosture::Shared)
    }
}

impl CausalPostureState for AuthorOnly {
    fn causal_posture() -> Option<CausalPosture> {
        Some(CausalPosture::AuthorOnly)
    }
}

impl CausalPostureState for Scratch {
    fn causal_posture() -> Option<CausalPosture> {
        Some(CausalPosture::Scratch)
    }
}

impl CausalPostureState for DynamicPosture {
    fn causal_posture() -> Option<CausalPosture> {
        None
    }
}
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

The `Shared`-only convenience methods narrow normal callers into the checked settlement path; they are not the sole authority. The live registry and runtime posture checks remain the final admission gate:

```rust
impl Strand<Shared> {
    pub fn plan(&self, ...) -> Result<SettlementPlan, SettlementError> {
        SettlementService::plan(runtime, provenance, self.strand_id)
    }

    pub fn settle(&self, ...) -> Result<SettlementResult, SettlementError> {
        SettlementService::settle(runtime, provenance, self.strand_id)
    }
}
```

---

### 2. Blinded Member References

Member references inside the public `BraidShell` can be sealed to protect user/strand identity:

```rust
pub enum BraidMemberRef {
    Revealed(StrandId),
    Sealed {
        blinded_commitment: Hash,
        authority: AuthorityDomainRef,
    },
}

impl BraidMemberRef {
    pub fn seal(
        strand_id: StrandId,
        child_worldline_id: WorldlineId,
        blinding_secret: Hash,
    ) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(SEALED_MEMBER_DOMAIN);
        hasher.update(&blinding_secret);
        hasher.update(child_worldline_id.as_bytes());
        hasher.update(strand_id.as_bytes());
        hasher.finalize().into()
    }

    pub fn matches_strand(
        &self,
        strand_id: &StrandId,
        child_worldline_id: &WorldlineId,
        authority: &AuthorityDomainRef,
        blinding_secret: &Hash,
    ) -> bool {
        match self {
            Self::Revealed(_) => false,
            Self::Sealed {
                blinded_commitment,
                authority: member_authority,
            } => {
                let expected = Self::seal(*strand_id, *child_worldline_id, *blinding_secret);
                member_authority == authority && *blinded_commitment == expected
            }
        }
    }
}
```

---

### 3. Proof-Shaped Envelopes

A `ProofEnvelope` contains proof-shaped evidence bytes and the public-input
hash they claim to bind. `ObserverHonestyClaim` is a separate assertion type;
`validate_shape` admits replay-trace evidence only and rejects
`ZkSnark`/`VectorOpening` envelopes until real verifier backends exist. It does
not perform cryptographic proof verification; only envelope structure and
public-input hash binding are validated.

```rust
pub enum ProofKind {
    ZkSnark,
    ReplayTrace,
    VectorOpening,
}

pub struct ProofEnvelope {
    pub kind: ProofKind,
    pub proof_bytes: Vec<u8>,
    pub public_inputs_hash: Hash,
}

pub struct ObserverHonestyClaim {
    pub coordinate: BraidCoordinate,
    pub shell_digest: Hash,
    pub observer_domain: AuthorityDomainRef,
}
```

Replay-trace shape validation and proof-envelope digest binding occur during
shell assembly. Proof cryptographic validity is not verified; only envelope
shape and public-input binding are validated:

```rust
impl BraidShell {
    pub fn assemble_with_proof(
        worldline_id: WorldlineId,
        basis: ProvenanceRef,
        mut members: Vec<BraidShellMember>,
        policy_id: Hash,
        mut outcome: BraidShellOutcome,
        posture: CausalPosture,
        proof: Option<crate::proof::ProofEnvelope>,
    ) -> Result<Self, BraidShellError> {
        // ... sorting and validation of members and posture ...
        if let Some(ref p) = proof {
            if let Err(err) = p.validate_shape(witness_digest) {
                return Err(BraidShellError::ProofShapeValidationFailed { reason: err });
            }
        }
        let proof_digest = proof.as_ref().map(crate::proof::ProofEnvelope::digest);
        // ... computes shell digest with proof_digest and returns Self ...
    }
}
```

---

### 4. Evolving Braid Logs

Evolving braids transition through discrete events folded sequentially:

```rust
pub enum BraidStatus {
    Active,
    Finalized,
    Collapsed,
}

pub enum BraidError {
    EmptyLog,
    MissingCreated,
    DuplicateCreated,
    IncoherentSequence {
        expected: u64,
        actual: u64,
    },
    InvalidTransition {
        action: String,
        status: BraidStatus,
    },
    SequenceOverflow {
        sequence_num: u64,
    },
    DuplicateMember {
        member_ref: BraidMemberRef,
    },
    EmptyCollapseWitness,
}

pub enum BraidEvent {
    BraidCreated {
        braid_id: Hash,
        creator_domain: AuthorityDomainRef,
    },
    MemberWoven {
        member_ref: BraidMemberRef,
        sequence_num: u64,
    },
    SettlementFinalized {
        settlement_digest: Hash,
    },
    BraidCollapsed {
        collapse_witness: Hash,
        outcome_digest: Hash,
    },
}

pub struct Braid {
    id: Hash,
    events: Vec<BraidEvent>,
    members: Vec<BraidMemberRef>,
    member_index: BTreeSet<BraidMemberRef>,
    next_sequence_num: u64,
    latest_settlement: Option<Hash>,
    status: BraidStatus,
}
```

Checked `apply` and `fold` preserve these invariants: duplicate creation is rejected, member sequence numbers must match the expected cursor, duplicate members are refused through a deterministic membership index, sequence overflow is explicit, settlement/collapse lifecycle order is enforced, and collapse witnesses must clear the `WitnessDigest` quality bar.
