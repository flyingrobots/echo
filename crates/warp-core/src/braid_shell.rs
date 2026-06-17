// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! θ_braid — retained braid-scale settlement boundary shells.
//!
//! A braid shell is not a note that plurality happened. It is the retained
//! holographic boundary that makes a braid-scope settlement outcome
//! replayable **without reopening member strand histories** (AIΩN Paper VII
//! Prop 3.5; design packet 0026). The shell binds basis + members + member
//! verdicts + policy + outcome + witness with domain-separated digests, and
//! [`replay_braid_shell`] reproduces the outcome from shell records alone —
//! its signature offers no path to strand histories, so rematerialization is
//! a type error, not a temptation.
//!
//! Hierarchy (E1a doctrine): `PluralAlternative` is per-entry residue;
//! θ_braid is the plural settlement boundary; the `BraidShell` is the
//! replayable outcome. Shells are append-only: a later collapse emits a new
//! `Derived` shell referencing its plural parent through `collapsed_from`;
//! it never rewrites the plural shell.

use blake3::Hasher;

use crate::admission::AdmissionOutcomeKind;
use crate::ident::Hash;
use crate::plurality_law::{PluralityLawReading, PluralityLawReadingError, PluralityLawRef};
use crate::provenance_store::ProvenanceRef;
use crate::revelation::{
    shell_posture_obstruction, AuthorityDomainRef, CausalPosture, PostureObstruction, WitnessDigest,
};
use crate::sealed_membership::DisclosureBudget;
use crate::strand::StrandId;
use crate::witness::WitnessReceipt;
use crate::worldline::WorldlineId;

const SHELL_DOMAIN: &[u8] = b"echo.shell.braid.v1\0";
const MEMBER_DOMAIN: &[u8] = b"echo.braid.member.v1\0";
const WITNESS_DOMAIN: &[u8] = b"echo.braid.witness.v1\0";
const COORDINATE_DOMAIN: &[u8] = b"echo.braid.coordinate.v1\0";
const SEALED_MEMBER_DOMAIN: &[u8] = b"echo.braid.member.sealed.v1\0";

/// Current braid shell body version.
///
/// Version 2 binds the optional proof-envelope digest marker into shell
/// identity. Version 1 proofless shells used the pre-proof digest body.
pub const BRAID_SHELL_VERSION: u32 = 2;

/// Compact settlement verdict for one braid member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberVerdict {
    /// Every claim from this member was imported.
    Derived,
    /// At least one claim from this member remained lawfully plural.
    Plural,
    /// At least one claim from this member became conflict residue and none
    /// remained plural.
    Conflict,
    /// The member's claims were obstructed before judgment completed.
    Obstructed,
}

impl MemberVerdict {
    const fn tag(self) -> u8 {
        match self {
            Self::Derived => 1,
            Self::Plural => 2,
            Self::Conflict => 3,
            Self::Obstructed => 4,
        }
    }
}

/// Reference to a braid member, supporting both revealed and sealed references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BraidMemberRef {
    /// Publicly revealed strand identity.
    Revealed(StrandId),
    /// Sealed member reference.
    Sealed {
        /// Domain-separated commitment digest of the member's identity.
        blinded_commitment: Hash,
        /// Causal authority domain controlling the private history.
        authority: AuthorityDomainRef,
    },
}

impl BraidMemberRef {
    /// Computes the commitment for a sealed reference using caller-supplied
    /// non-public blinding material.
    #[must_use]
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

    /// Returns whether this sealed member reference matches the given strand
    /// ID, child worldline ID, authority, and blinding material.
    #[must_use]
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

    const fn is_sealed(self) -> bool {
        matches!(self, Self::Sealed { .. })
    }

    /// Stable wire tag for canonical serialization.
    #[must_use]
    pub fn canonical_tag(self) -> u8 {
        match self {
            Self::Revealed(_) => 0x01,
            Self::Sealed { .. } => 0x02,
        }
    }

    /// Hash this member reference into the given hasher.
    pub fn hash_into(self, hasher: &mut Hasher) {
        hasher.update(&[self.canonical_tag()]);
        match self {
            Self::Revealed(strand_id) => {
                hasher.update(strand_id.as_bytes());
            }
            Self::Sealed {
                blinded_commitment,
                authority,
            } => {
                hasher.update(&blinded_commitment);
                hasher.update(authority.origin_id.as_bytes());
                hasher.update(authority.domain_id.as_bytes());
            }
        }
    }
}

/// One member entry in a braid shell: compact replay facts, never history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BraidShellMember {
    /// Reference to the member strand, which may be revealed or sealed.
    pub member_ref: BraidMemberRef,
    /// Digest over the member's support-pin set.
    pub support_pin_digest: Hash,
    /// Digest over the member's fork basis facts.
    pub basis_digest: Hash,
    /// Digest over the realized parent frontier the member was judged against.
    pub frontier_digest: Hash,
    /// Digest over the contended footprint slots.
    pub footprint_digest: Hash,
    /// Digest over the member's ordered claim identities.
    pub claim_digest: Hash,
    /// Compact settlement verdict for the member.
    pub verdict: MemberVerdict,
    /// Digest over the member's ordered per-claim decisions.
    pub verdict_digest: Hash,
    /// Revelation posture carried by the member's claims.
    pub posture: CausalPosture,
}

impl BraidShellMember {
    /// Canonical content digest for this member.
    #[must_use]
    pub fn member_digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(MEMBER_DOMAIN);
        self.member_ref.hash_into(&mut hasher);
        hasher.update(&self.support_pin_digest);
        hasher.update(&self.basis_digest);
        hasher.update(&self.frontier_digest);
        hasher.update(&self.footprint_digest);
        hasher.update(&self.claim_digest);
        hasher.update(&[self.verdict.tag()]);
        hasher.update(&self.verdict_digest);
        hasher.update(&[self.posture.canonical_tag()]);
        hasher.finalize().into()
    }
}

/// Braid-shell outcome over the shared lawful algebra.
///
/// Collapse is a witnessed transition, never a fifth arm: a collapse emits a
/// new `Derived` shell whose `collapsed_from` references the plural parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BraidShellOutcome {
    /// A single lawful result was derived.
    Derived {
        /// Target-worldline refs realized by the derivation.
        result_refs: Vec<ProvenanceRef>,
        /// Collapse policy when this derivation collapsed retained plurality.
        collapse_policy: Option<Hash>,
        /// Witness for the explicit collapse act, when applicable.
        collapse_witness: Option<Hash>,
        /// Plural parent shell this derivation collapsed, when applicable.
        collapsed_from: Option<Hash>,
    },
    /// Multiple lawful alternatives remain retained.
    Plural {
        /// Stable plural artifact ids of the retained alternatives.
        alternative_ids: Vec<Hash>,
    },
    /// The settlement produced explicit conflict residue.
    Conflict {
        /// Deterministic per-claim conflict reason codes in claim order.
        reason_codes: Vec<u8>,
    },
    /// The settlement act was obstructed before judgment completed.
    Obstruction {
        /// Deterministic obstruction reason code.
        reason_code: u8,
        /// Witness digest for the obstruction.
        witness: Hash,
        /// Shell whose transition this obstruction refused, when applicable.
        obstructed_from: Option<Hash>,
    },
}

impl BraidShellOutcome {
    /// Maps the shell outcome onto Echo's shared lawful outcome family.
    #[must_use]
    pub fn kind(&self) -> AdmissionOutcomeKind {
        match self {
            Self::Derived { .. } => AdmissionOutcomeKind::Derived,
            Self::Plural { .. } => AdmissionOutcomeKind::Plural,
            Self::Conflict { .. } => AdmissionOutcomeKind::Conflict,
            Self::Obstruction { .. } => AdmissionOutcomeKind::Obstruction,
        }
    }

    fn hash_into(&self, hasher: &mut Hasher) {
        match self {
            Self::Derived {
                result_refs,
                collapse_policy,
                collapse_witness,
                collapsed_from,
            } => {
                hasher.update(&[1]);
                hasher.update(&(result_refs.len() as u64).to_le_bytes());
                for reference in result_refs {
                    hash_provenance_ref(hasher, reference);
                }
                hash_optional_digest(hasher, collapse_policy.as_ref());
                hash_optional_digest(hasher, collapse_witness.as_ref());
                hash_optional_digest(hasher, collapsed_from.as_ref());
            }
            Self::Plural { alternative_ids } => {
                hasher.update(&[2]);
                hasher.update(&(alternative_ids.len() as u64).to_le_bytes());
                for alternative in alternative_ids {
                    hasher.update(alternative);
                }
            }
            Self::Conflict { reason_codes } => {
                hasher.update(&[3]);
                hasher.update(&(reason_codes.len() as u64).to_le_bytes());
                hasher.update(reason_codes);
            }
            Self::Obstruction {
                reason_code,
                witness,
                obstructed_from,
            } => {
                hasher.update(&[4]);
                hasher.update(&[*reason_code]);
                hasher.update(witness);
                hash_optional_digest(hasher, obstructed_from.as_ref());
            }
        }
    }
}

fn hash_provenance_ref(hasher: &mut Hasher, reference: &ProvenanceRef) {
    hasher.update(reference.worldline_id.as_bytes());
    hasher.update(&reference.worldline_tick.as_u64().to_le_bytes());
    hasher.update(&reference.commit_hash);
}

fn hash_optional_digest(hasher: &mut Hasher, digest: Option<&Hash>) {
    match digest {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(value);
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

/// Obstructions raised while assembling, validating, or replaying a shell.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BraidShellError {
    /// A shell must summarize at least one member.
    #[error("braid shell must summarize at least one member")]
    EmptyMembers,
    /// The shell posture exceeds its least-revealed member.
    #[error("braid shell posture exceeds least-revealed member: {0:?}")]
    PostureExceedsMembers(PostureObstruction),
    /// Outcome arm and member verdicts disagree.
    #[error("braid shell outcome {outcome:?} disagrees with member verdicts")]
    OutcomeMemberMismatch {
        /// Outcome arm the shell claims.
        outcome: AdmissionOutcomeKind,
    },
    /// The stored digest does not match the recomputed canonical body.
    #[error("braid shell digest mismatch: stored {stored:?}, recomputed {recomputed:?}")]
    DigestMismatch {
        /// Digest stored on the shell.
        stored: Hash,
        /// Digest recomputed from the body.
        recomputed: Hash,
    },
    /// The stored witness digest does not match the recomputed body witness.
    #[error("braid shell witness mismatch: stored {stored:?}, recomputed {recomputed:?}")]
    WitnessMismatch {
        /// Witness digest stored on the shell.
        stored: Hash,
        /// Witness digest recomputed from the body.
        recomputed: Hash,
    },
    /// The proof shape validation failed.
    #[error("proof shape validation failed: {reason}")]
    ProofShapeValidationFailed {
        /// Reason for shape validation failure.
        reason: crate::proof::ProofError,
    },
    /// Member entries are not in canonical order.
    #[error("braid shell members are not in canonical order")]
    NonCanonicalMemberOrder,
    /// No shell record exists for the requested digest.
    #[error("no braid shell retained for digest {digest:?}")]
    ShellNotFound {
        /// Digest that resolved to nothing.
        digest: Hash,
    },
    /// A shell with this digest is already retained with different content.
    #[error("a divergent braid shell already claims digest {digest:?}")]
    DuplicateDigestDivergentContent {
        /// Digest both shells claim.
        digest: Hash,
    },
    /// The shell claims a body version this build does not speak.
    #[error("unsupported braid shell version {stored} (supported: {supported})")]
    UnsupportedVersion {
        /// Version stored on the shell.
        stored: u32,
        /// Version this build supports.
        supported: u32,
    },
    /// Plural alternatives are not in canonical set order.
    #[error("braid shell plural alternatives are not in canonical order")]
    NonCanonicalAlternativeOrder,
    /// Collapse lineage fields must be all present or all absent.
    #[error("derived shell carries incoherent collapse fields")]
    IncoherentCollapseFields,
    /// A witness digest must never be a 32-byte shrug.
    #[error("empty or null witness digest refused")]
    EmptyWitness,
    /// A policy id must name a non-empty law.
    #[error("empty policy id refused")]
    EmptyPolicyId,
    /// A law reading failed validation.
    #[error("plurality law reading refused: {0}")]
    LawReading(#[from] PluralityLawReadingError),
    /// A lineage parent shell is missing or not plural.
    #[error("lineage parent {parent:?} is missing or not plural")]
    InvalidLineageParent {
        /// Parent shell digest named by a collapse or obstruction lineage.
        parent: Hash,
    },
    /// The stored braid coordinate does not match the recomputed body.
    #[error("braid coordinate mismatch")]
    CoordinateMismatch,
    /// Plural alternatives are a set; duplicates are refused.
    #[error("duplicate plural alternative id {alternative_id:?}")]
    DuplicateAlternativeId {
        /// Alternative id that appeared more than once.
        alternative_id: Hash,
    },
    /// One strand may appear at most once among shell members.
    #[error("duplicate member strand {member_ref:?}")]
    DuplicateMemberStrand {
        /// Member reference that appeared more than once.
        member_ref: BraidMemberRef,
    },
    /// Revealed and sealed member references may not be mixed in one shell.
    #[error("braid shell mixes revealed and sealed member references")]
    MixedMemberReferencePosture,
    /// A retained plural artifact id may never migrate to a different shell.
    #[error("plural artifact {plural_id:?} already bound to shell {existing_shell:?}")]
    PluralArtifactAlreadyBound {
        /// Plural artifact id being bound.
        plural_id: Hash,
        /// Shell digest the id is already bound to.
        existing_shell: Hash,
        /// Shell digest the rebind attempted.
        attempted_shell: Hash,
    },
}

/// Canonical coordinate of a braid: where the shell lives in braid space.
///
/// `hash(basis_ref, canonical_member_digest_list, policy_id)` — the first
/// real consumer of braid-scale addressing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BraidCoordinate(pub Hash);

impl BraidCoordinate {
    fn derive(basis: &ProvenanceRef, member_digests: &[Hash], policy_id: Hash) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(COORDINATE_DOMAIN);
        hash_provenance_ref(&mut hasher, basis);
        hasher.update(&(member_digests.len() as u64).to_le_bytes());
        for member_digest in member_digests {
            hasher.update(member_digest);
        }
        hasher.update(&policy_id);
        Self(hasher.finalize().into())
    }
}

/// Retained braid-scale settlement boundary (θ_braid).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BraidShell {
    /// Shell body version.
    pub version: u32,
    /// Canonical braid coordinate this shell lives at.
    pub coordinate: BraidCoordinate,
    /// Target worldline the settlement judged into.
    pub worldline_id: WorldlineId,
    /// Comparison basis the members were judged against.
    pub basis: ProvenanceRef,
    /// Canonically ordered member entries.
    pub members: Vec<BraidShellMember>,
    /// Settlement policy identity the act ran under.
    pub policy_id: Hash,
    /// Outcome arm over the shared lawful algebra.
    pub outcome: BraidShellOutcome,
    /// Witness digest binding the settlement act.
    ///
    /// E1 scaffolding: this is a domain-separated self-witness — a hash of
    /// the same shell body — folded into `digest`. It guarantees integrity,
    /// not independent attestation (anyone who can compute the shell can
    /// compute it). A real external witness replaces it when settlement
    /// gains witness-bearing authority.
    pub witness_digest: Hash,
    /// Revelation posture of the shell itself.
    pub posture: CausalPosture,
    /// Optional proof-shaped evidence envelope bound into shell identity.
    pub proof: Option<crate::proof::ProofEnvelope>,
    /// Canonical content digest of the full shell body.
    pub digest: Hash,
}

impl BraidShell {
    /// Assembles a shell: canonicalizes member order, checks the posture
    /// floor and outcome/member coherence, and seals witness + digest.
    ///
    /// # Errors
    ///
    /// Returns [`BraidShellError`] for an empty member set
    /// ([`BraidShellError::EmptyMembers`]), a duplicate member strand or
    /// plural alternative ([`BraidShellError::DuplicateMemberStrand`],
    /// [`BraidShellError::DuplicateAlternativeId`]), an empty policy id
    /// ([`BraidShellError::EmptyPolicyId`]), an empty witness on a
    /// collapse/obstruction outcome ([`BraidShellError::EmptyWitness`]),
    /// incoherent collapse fields
    /// ([`BraidShellError::IncoherentCollapseFields`]), a posture exceeding
    /// the least-revealed member
    /// ([`BraidShellError::PostureExceedsMembers`]), or an outcome arm that
    /// disagrees with member verdicts
    /// ([`BraidShellError::OutcomeMemberMismatch`]).
    pub fn assemble(
        worldline_id: WorldlineId,
        basis: ProvenanceRef,
        members: Vec<BraidShellMember>,
        policy_id: Hash,
        outcome: BraidShellOutcome,
        posture: CausalPosture,
    ) -> Result<Self, BraidShellError> {
        Self::assemble_with_proof(
            worldline_id,
            basis,
            members,
            policy_id,
            outcome,
            posture,
            None,
        )
    }

    /// Assembles a shell with a proof-shaped envelope: validates member order,
    /// checks posture floor and coherence, validates the proof envelope shape
    /// (if present) against the derived witness, and seals the shell digest.
    /// Proof cryptographic validity is not verified; only envelope shape and
    /// public-input binding are validated.
    ///
    /// # Errors
    ///
    /// Returns [`BraidShellError`] if any structure constraints are violated or if
    /// the proof envelope validation fails.
    pub fn assemble_with_proof(
        worldline_id: WorldlineId,
        basis: ProvenanceRef,
        mut members: Vec<BraidShellMember>,
        policy_id: Hash,
        mut outcome: BraidShellOutcome,
        posture: CausalPosture,
        proof: Option<crate::proof::ProofEnvelope>,
    ) -> Result<Self, BraidShellError> {
        if members.is_empty() {
            return Err(BraidShellError::EmptyMembers);
        }
        check_policy_id(policy_id)?;
        members.sort_by_cached_key(BraidShellMember::member_digest);
        check_unique_member_strands(&members)?;
        if let BraidShellOutcome::Plural { alternative_ids } = &mut outcome {
            // Retained alternatives are a set; canonical order, not transcript
            // order (the member verdict digest binds the ordered transcript).
            alternative_ids.sort_unstable();
            if let Some(duplicate) = alternative_ids
                .windows(2)
                .find(|pair| pair[0] == pair[1])
                .map(|pair| pair[0])
            {
                return Err(BraidShellError::DuplicateAlternativeId {
                    alternative_id: duplicate,
                });
            }
        }
        check_outcome_law(&outcome)?;
        if let Some(obstruction) =
            shell_posture_obstruction(posture, members.iter().map(|member| member.posture))
        {
            return Err(BraidShellError::PostureExceedsMembers(obstruction));
        }
        check_outcome_member_coherence(&outcome, &members)?;

        // Compute each member digest once; coordinate, witness, and shell
        // digests all consume it.
        let member_digests: Vec<Hash> = members
            .iter()
            .map(BraidShellMember::member_digest)
            .collect();
        let coordinate = BraidCoordinate::derive(&basis, &member_digests, policy_id);
        let witness_digest = compute_witness_digest(
            BRAID_SHELL_VERSION,
            worldline_id,
            &basis,
            &member_digests,
            policy_id,
            &outcome,
            posture,
        );

        if let Some(ref p) = proof {
            if let Err(err) = p.validate_shape(witness_digest) {
                return Err(BraidShellError::ProofShapeValidationFailed { reason: err });
            }
        }
        let proof_digest = proof.as_ref().map(crate::proof::ProofEnvelope::digest);

        let digest = compute_shell_digest(
            BRAID_SHELL_VERSION,
            worldline_id,
            &basis,
            &member_digests,
            policy_id,
            &outcome,
            witness_digest,
            posture,
            proof_digest,
        );
        Ok(Self {
            version: BRAID_SHELL_VERSION,
            coordinate,
            worldline_id,
            basis,
            members,
            policy_id,
            outcome,
            witness_digest,
            posture,
            digest,
            proof,
        })
    }

    /// Validates the shell as a self-contained retained record.
    ///
    /// # Errors
    ///
    /// Returns [`BraidShellError`] for an unsupported version, empty or
    /// non-canonically-ordered members, a duplicate member strand,
    /// non-canonical or duplicate plural alternatives, an empty policy id, an
    /// empty collapse/obstruction witness, a posture floor violation, an
    /// outcome/member disagreement, a coordinate mismatch, or a stored
    /// witness/shell digest that does not match the recomputed body.
    pub fn validate(&self) -> Result<(), BraidShellError> {
        if self.version != BRAID_SHELL_VERSION {
            return Err(BraidShellError::UnsupportedVersion {
                stored: self.version,
                supported: BRAID_SHELL_VERSION,
            });
        }
        if self.members.is_empty() {
            return Err(BraidShellError::EmptyMembers);
        }
        check_policy_id(self.policy_id)?;
        // Compute each member digest once; the order check, coordinate,
        // witness, and shell digests all consume it.
        let member_digests: Vec<Hash> = self
            .members
            .iter()
            .map(BraidShellMember::member_digest)
            .collect();
        if member_digests.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err(BraidShellError::NonCanonicalMemberOrder);
        }
        check_unique_member_strands(&self.members)?;
        if let BraidShellOutcome::Plural { alternative_ids } = &self.outcome {
            if alternative_ids.windows(2).any(|pair| pair[0] > pair[1]) {
                return Err(BraidShellError::NonCanonicalAlternativeOrder);
            }
            if let Some(duplicate) = alternative_ids
                .windows(2)
                .find(|pair| pair[0] == pair[1])
                .map(|pair| pair[0])
            {
                return Err(BraidShellError::DuplicateAlternativeId {
                    alternative_id: duplicate,
                });
            }
        }
        check_outcome_law(&self.outcome)?;
        if let Some(obstruction) = shell_posture_obstruction(
            self.posture,
            self.members.iter().map(|member| member.posture),
        ) {
            return Err(BraidShellError::PostureExceedsMembers(obstruction));
        }
        check_outcome_member_coherence(&self.outcome, &self.members)?;
        if BraidCoordinate::derive(&self.basis, &member_digests, self.policy_id) != self.coordinate
        {
            return Err(BraidShellError::CoordinateMismatch);
        }

        let witness = compute_witness_digest(
            self.version,
            self.worldline_id,
            &self.basis,
            &member_digests,
            self.policy_id,
            &self.outcome,
            self.posture,
        );
        if witness != self.witness_digest {
            return Err(BraidShellError::WitnessMismatch {
                stored: self.witness_digest,
                recomputed: witness,
            });
        }
        if let Some(ref p) = self.proof {
            if let Err(err) = p.validate_shape(self.witness_digest) {
                return Err(BraidShellError::ProofShapeValidationFailed { reason: err });
            }
        }
        let proof_digest = self.proof.as_ref().map(crate::proof::ProofEnvelope::digest);

        let digest = compute_shell_digest(
            self.version,
            self.worldline_id,
            &self.basis,
            &member_digests,
            self.policy_id,
            &self.outcome,
            self.witness_digest,
            self.posture,
            proof_digest,
        );
        if digest != self.digest {
            return Err(BraidShellError::DigestMismatch {
                stored: self.digest,
                recomputed: digest,
            });
        }
        Ok(())
    }

    /// Maps the shell outcome onto Echo's shared lawful outcome family.
    #[must_use]
    pub fn outcome_kind(&self) -> AdmissionOutcomeKind {
        self.outcome.kind()
    }

    /// Returns whether the shell summarizes the given revealed member strand.
    #[must_use]
    pub fn has_revealed_member_strand(&self, strand_id: &StrandId) -> bool {
        self.members.iter().any(|member| match member.member_ref {
            BraidMemberRef::Revealed(id) => id == *strand_id,
            BraidMemberRef::Sealed { .. } => false,
        })
    }

    /// Returns whether the shell summarizes the given member strand, using
    /// non-public blinding material for sealed references.
    #[must_use]
    pub fn has_member_strand_secure(
        &self,
        strand_id: &StrandId,
        child_worldline_id: &WorldlineId,
        authority: &AuthorityDomainRef,
        blinding_secret: &Hash,
    ) -> bool {
        self.members.iter().any(|member| {
            member.member_ref.matches_strand(
                strand_id,
                child_worldline_id,
                authority,
                blinding_secret,
            )
        })
    }
}

/// Witness-bearing outcome fields must clear the [`WitnessDigest`] bar:
/// the newtype bouncer guards every door, not just the constructor.
fn check_outcome_law(outcome: &BraidShellOutcome) -> Result<(), BraidShellError> {
    match outcome {
        BraidShellOutcome::Derived {
            collapse_witness: Some(witness),
            ..
        }
        | BraidShellOutcome::Obstruction { witness, .. } => {
            WitnessDigest::new(*witness).map_err(|_| BraidShellError::EmptyWitness)?;
        }
        _ => {}
    }
    Ok(())
}

fn check_policy_id(policy_id: Hash) -> Result<(), BraidShellError> {
    if policy_id.iter().all(|byte| *byte == 0) {
        return Err(BraidShellError::EmptyPolicyId);
    }
    Ok(())
}

/// One strand may appear at most once among shell members.
fn check_unique_member_strands(members: &[BraidShellMember]) -> Result<(), BraidShellError> {
    if let Some(first) = members.first() {
        let first_is_sealed = first.member_ref.is_sealed();
        if members
            .iter()
            .any(|member| member.member_ref.is_sealed() != first_is_sealed)
        {
            return Err(BraidShellError::MixedMemberReferencePosture);
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for member in members {
        if !seen.insert(member.member_ref) {
            return Err(BraidShellError::DuplicateMemberStrand {
                member_ref: member.member_ref,
            });
        }
    }
    Ok(())
}

fn check_outcome_member_coherence(
    outcome: &BraidShellOutcome,
    members: &[BraidShellMember],
) -> Result<(), BraidShellError> {
    let any_plural = members
        .iter()
        .any(|member| member.verdict == MemberVerdict::Plural);
    let any_conflict = members
        .iter()
        .any(|member| member.verdict == MemberVerdict::Conflict);
    let coherent = match outcome {
        BraidShellOutcome::Derived {
            collapse_policy,
            collapse_witness,
            collapsed_from,
            ..
        } => {
            let lineage_fields = [
                collapse_policy.is_some(),
                collapse_witness.is_some(),
                collapsed_from.is_some(),
            ];
            if lineage_fields.iter().any(|present| *present)
                && lineage_fields.iter().any(|present| !*present)
            {
                return Err(BraidShellError::IncoherentCollapseFields);
            }
            // A collapse-derived shell summarizes the plural members it
            // collapsed; a settlement-derived shell must carry none.
            collapsed_from.is_some() || (!any_plural && !any_conflict)
        }
        BraidShellOutcome::Plural { .. } => any_plural,
        BraidShellOutcome::Conflict { .. } => any_conflict && !any_plural,
        BraidShellOutcome::Obstruction { .. } => true,
    };
    if coherent {
        Ok(())
    } else {
        Err(BraidShellError::OutcomeMemberMismatch {
            outcome: outcome.kind(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn hash_shell_body(
    hasher: &mut Hasher,
    version: u32,
    worldline_id: WorldlineId,
    basis: &ProvenanceRef,
    member_digests: &[Hash],
    policy_id: Hash,
    outcome: &BraidShellOutcome,
    posture: CausalPosture,
) {
    hasher.update(&version.to_le_bytes());
    hasher.update(worldline_id.as_bytes());
    hash_provenance_ref(hasher, basis);
    hasher.update(&(member_digests.len() as u64).to_le_bytes());
    for member_digest in member_digests {
        hasher.update(member_digest);
    }
    hasher.update(&policy_id);
    outcome.hash_into(hasher);
    hasher.update(&[posture.canonical_tag()]);
}

#[allow(clippy::too_many_arguments)]
fn compute_witness_digest(
    version: u32,
    worldline_id: WorldlineId,
    basis: &ProvenanceRef,
    member_digests: &[Hash],
    policy_id: Hash,
    outcome: &BraidShellOutcome,
    posture: CausalPosture,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WITNESS_DOMAIN);
    hash_shell_body(
        &mut hasher,
        version,
        worldline_id,
        basis,
        member_digests,
        policy_id,
        outcome,
        posture,
    );
    hasher.finalize().into()
}

#[allow(clippy::too_many_arguments)]
fn compute_shell_digest(
    version: u32,
    worldline_id: WorldlineId,
    basis: &ProvenanceRef,
    member_digests: &[Hash],
    policy_id: Hash,
    outcome: &BraidShellOutcome,
    witness_digest: Hash,
    posture: CausalPosture,
    proof_digest: Option<Hash>,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(SHELL_DOMAIN);
    hash_shell_body(
        &mut hasher,
        version,
        worldline_id,
        basis,
        member_digests,
        policy_id,
        outcome,
        posture,
    );
    hasher.update(&witness_digest);
    match proof_digest {
        Some(digest) => {
            hasher.update(&[0x01]);
            hasher.update(&digest);
        }
        None => {
            hasher.update(&[0x00]);
        }
    }
    hasher.finalize().into()
}

/// Read access to retained shell records — and nothing else.
///
/// Replay's only window into the world. There is deliberately no method that
/// returns provenance entries, strand registries, or worldline state: a
/// shell that cannot replay through this trait is not a shell.
pub trait BraidShellRecords {
    /// Returns the retained shell with the given digest, if any.
    fn shell(&self, digest: &Hash) -> Option<&BraidShell>;
}

impl BraidShellRecords for std::collections::BTreeMap<Hash, BraidShell> {
    fn shell(&self, digest: &Hash) -> Option<&BraidShell> {
        self.get(digest)
    }
}

/// Outcome reproduced from a retained shell alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BraidShellReplay {
    /// Outcome arm reproduced from the shell.
    pub outcome_kind: AdmissionOutcomeKind,
    /// Member verdicts in canonical member order.
    pub member_verdicts: Vec<(BraidMemberRef, MemberVerdict)>,
    /// Policy identity the act ran under.
    pub policy_id: Hash,
    /// Named law that interpreted retained plurality or collapse.
    pub law_ref: PluralityLawRef,
    /// Witness digest binding the act.
    pub witness_digest: Hash,
    /// Revelation posture of the shell.
    pub posture: CausalPosture,
}

/// Stable proof-binding fact for braid shell audit output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BraidProofBinding {
    /// The audited shell carries no proof envelope.
    Absent,
    /// The audited shell carries a proof envelope that validated against the
    /// shell witness digest.
    Matched {
        /// Proof envelope kind.
        kind: crate::proof::ProofKind,
        /// Canonical proof-envelope digest.
        envelope_digest: Hash,
        /// Public inputs hash carried by the proof envelope.
        public_inputs_hash: Hash,
    },
}

/// Witness posture surfaced by braid shell audit output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BraidWitnessPosture {
    /// Current E1 self-witness: integrity-only local evidence, not independent
    /// attestation.
    SelfWitnessIntegrityOnly {
        /// Witness digest bound into the shell.
        digest: Hash,
    },
}

/// One replay/audit fact for a braid shell member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BraidShellMemberAuditFact {
    /// Reference to the member strand, revealed or sealed.
    pub member_ref: BraidMemberRef,
    /// Settlement verdict reproduced for this member.
    pub verdict: MemberVerdict,
    /// Member-level causal posture.
    pub posture: CausalPosture,
    /// Digest over the member's support-pin set.
    pub support_pin_digest: Hash,
    /// Digest over the member's fork basis facts.
    pub basis_digest: Hash,
    /// Digest over the realized parent frontier judged for this member.
    pub frontier_digest: Hash,
    /// Digest over contended footprint slots.
    pub footprint_digest: Hash,
    /// Digest over ordered claim identities.
    pub claim_digest: Hash,
    /// Digest over ordered per-claim decisions.
    pub verdict_digest: Hash,
    /// Disclosure budget needed to interpret the member reference.
    pub disclosure_budget: DisclosureBudget,
}

/// Replay/audit facts reproduced from a retained braid shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BraidShellAudit {
    /// Canonical shell digest audited.
    pub shell_digest: Hash,
    /// Canonical braid coordinate this shell lives at.
    pub coordinate: BraidCoordinate,
    /// Outcome arm reproduced from the shell.
    pub outcome_kind: AdmissionOutcomeKind,
    /// Policy identity the act ran under.
    pub policy_id: Hash,
    /// Witnessed law reading for this shell audit.
    pub law_reading: PluralityLawReading,
    /// Least-revealed member posture across the audited shell.
    pub posture_floor: CausalPosture,
    /// Revelation posture claimed by the shell itself.
    pub shell_posture: CausalPosture,
    /// Member refs in canonical settlement frontier order.
    pub settlement_frontier: Vec<BraidMemberRef>,
    /// Per-member replay/audit facts in canonical member order.
    pub member_facts: Vec<BraidShellMemberAuditFact>,
    /// Proof binding status for the shell.
    pub proof_binding: BraidProofBinding,
    /// Witness posture for the shell.
    pub witness_posture: BraidWitnessPosture,
    /// Typed witness receipt for the shell audit.
    pub witness_receipt: WitnessReceipt,
}

/// Replays a braid-scope settlement outcome from retained shell records.
///
/// Verifies digest integrity (member digests, witness digest, shell digest)
/// and collapse lineage (a `Derived` shell collapsing plurality must
/// reference a retained `Plural` parent), then reproduces the outcome arm
/// and member verdicts. Member strand histories are unreachable by
/// construction.
///
/// # Errors
///
/// Returns [`BraidShellError`] when the shell is missing, fails
/// self-validation, or names a collapse parent that is absent or not plural.
pub fn replay_braid_shell(
    digest: &Hash,
    records: &dyn BraidShellRecords,
) -> Result<BraidShellReplay, BraidShellError> {
    let shell = validated_shell_for_replay(digest, records)?;
    let law_ref = shell_law_ref(shell)?;
    Ok(BraidShellReplay {
        outcome_kind: shell.outcome_kind(),
        member_verdicts: shell
            .members
            .iter()
            .map(|member| (member.member_ref, member.verdict))
            .collect(),
        policy_id: shell.policy_id,
        law_ref,
        witness_digest: shell.witness_digest,
        posture: shell.posture,
    })
}

/// Returns stable replay/audit facts from a retained braid shell.
///
/// The audit view validates the same retained shell and collapse-lineage
/// constraints as [`replay_braid_shell`]. It exposes support, frontier, proof,
/// posture, member-verdict, and witness facts without reopening member strand
/// histories.
///
/// # Errors
///
/// Returns [`BraidShellError`] when the shell is missing, fails
/// self-validation, or names a collapse parent that is absent or not plural.
pub fn audit_braid_shell(
    digest: &Hash,
    records: &dyn BraidShellRecords,
) -> Result<BraidShellAudit, BraidShellError> {
    let shell = validated_shell_for_replay(digest, records)?;
    let posture_floor = shell
        .members
        .iter()
        .map(|member| member.posture)
        .min()
        .unwrap_or(shell.posture);
    let proof_binding = shell
        .proof
        .as_ref()
        .map_or(BraidProofBinding::Absent, |proof| {
            BraidProofBinding::Matched {
                kind: proof.kind,
                envelope_digest: proof.digest(),
                public_inputs_hash: proof.public_inputs_hash,
            }
        });
    let witness_receipt = WitnessReceipt::self_witness(shell.digest, shell.witness_digest);
    let law_ref = shell_law_ref(shell)?;
    let law_reading = PluralityLawReading::new(
        law_ref,
        shell.digest,
        witness_receipt,
        shell_disclosure_budget(shell),
    )?;

    Ok(BraidShellAudit {
        shell_digest: shell.digest,
        coordinate: shell.coordinate,
        outcome_kind: shell.outcome_kind(),
        policy_id: shell.policy_id,
        law_reading,
        posture_floor,
        shell_posture: shell.posture,
        settlement_frontier: shell
            .members
            .iter()
            .map(|member| member.member_ref)
            .collect(),
        member_facts: shell
            .members
            .iter()
            .map(|member| BraidShellMemberAuditFact {
                member_ref: member.member_ref,
                verdict: member.verdict,
                posture: member.posture,
                support_pin_digest: member.support_pin_digest,
                basis_digest: member.basis_digest,
                frontier_digest: member.frontier_digest,
                footprint_digest: member.footprint_digest,
                claim_digest: member.claim_digest,
                verdict_digest: member.verdict_digest,
                disclosure_budget: member_disclosure_budget(member.member_ref),
            })
            .collect(),
        proof_binding,
        witness_posture: BraidWitnessPosture::SelfWitnessIntegrityOnly {
            digest: shell.witness_digest,
        },
        witness_receipt,
    })
}

const fn member_disclosure_budget(member_ref: BraidMemberRef) -> DisclosureBudget {
    match member_ref {
        BraidMemberRef::Revealed(_) => DisclosureBudget::Public,
        BraidMemberRef::Sealed { .. } => DisclosureBudget::AuthorityScoped,
    }
}

fn shell_disclosure_budget(shell: &BraidShell) -> DisclosureBudget {
    if shell
        .members
        .iter()
        .any(|member| matches!(member.member_ref, BraidMemberRef::Sealed { .. }))
    {
        DisclosureBudget::AuthorityScoped
    } else {
        DisclosureBudget::Public
    }
}

fn shell_law_ref(shell: &BraidShell) -> Result<PluralityLawRef, BraidShellError> {
    match &shell.outcome {
        BraidShellOutcome::Derived {
            collapse_policy: Some(policy_id),
            ..
        } => PluralityLawRef::collapse_policy(*policy_id),
        _ => PluralityLawRef::settlement_policy(shell.policy_id),
    }
    .map_err(|_| BraidShellError::EmptyPolicyId)
}

fn validated_shell_for_replay<'a>(
    digest: &Hash,
    records: &'a dyn BraidShellRecords,
) -> Result<&'a BraidShell, BraidShellError> {
    let shell = records
        .shell(digest)
        .ok_or(BraidShellError::ShellNotFound { digest: *digest })?;
    shell.validate()?;
    if shell.digest != *digest {
        return Err(BraidShellError::DigestMismatch {
            stored: *digest,
            recomputed: shell.digest,
        });
    }
    let lineage_parent = match &shell.outcome {
        BraidShellOutcome::Derived {
            collapsed_from: Some(parent_digest),
            ..
        }
        | BraidShellOutcome::Obstruction {
            obstructed_from: Some(parent_digest),
            ..
        } => Some(*parent_digest),
        _ => None,
    };
    if let Some(parent_digest) = lineage_parent {
        let parent =
            records
                .shell(&parent_digest)
                .ok_or(BraidShellError::InvalidLineageParent {
                    parent: parent_digest,
                })?;
        parent.validate()?;
        if !matches!(parent.outcome, BraidShellOutcome::Plural { .. }) {
            return Err(BraidShellError::InvalidLineageParent {
                parent: parent_digest,
            });
        }
    }
    Ok(shell)
}

/// Deterministic reason code: collapse attempted without a named policy.
pub const COLLAPSE_WITHOUT_POLICY_REASON: u8 = 1;

const COLLAPSE_OBSTRUCTION_WITNESS_DOMAIN: &[u8] = b"echo.braid.collapse.obstruction.v1\0";
const ABSENT_COLLAPSE_POLICY_DOMAIN: &[u8] = b"echo.braid.collapse-policy.absent.v1\0";

/// Named, witnessed law permitting one collapse of retained plurality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollapsePolicy {
    /// Stable collapse policy identity.
    pub policy_id: Hash,
    /// Witness for the explicit collapse act.
    pub witness: WitnessDigest,
}

/// Outcome of one collapse attempt. Both arms are retained law.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollapseResult {
    /// The plurality lawfully collapsed into a new derived shell.
    Derived(BraidShell),
    /// The collapse was refused; the obstruction is itself a retained shell.
    Obstructed(BraidShell),
}

/// Collapses a retained plural shell into a new shell-family record.
///
/// The plural parent is never mutated: a named, witnessed collapse policy
/// produces a new `Derived` shell whose `collapsed_from` references the
/// parent; a missing policy produces a retained `Obstruction` shell. Either
/// way the original plural shell remains byte-identical truth forever.
/// Append-only or bust.
///
/// When `policy` is `None`, `selected_result_refs` is ignored: an obstructed
/// collapse retains no derived result, only the refusal.
///
/// # Errors
///
/// Returns [`BraidShellError`] when the plural shell is missing, fails
/// validation, or is not plural.
pub fn collapse_braid_shell(
    records: &dyn BraidShellRecords,
    plural_shell_digest: Hash,
    selected_result_refs: Vec<ProvenanceRef>,
    policy: Option<CollapsePolicy>,
) -> Result<CollapseResult, BraidShellError> {
    let plural = records
        .shell(&plural_shell_digest)
        .ok_or(BraidShellError::ShellNotFound {
            digest: plural_shell_digest,
        })?;
    plural.validate()?;
    if !matches!(plural.outcome, BraidShellOutcome::Plural { .. }) {
        return Err(BraidShellError::InvalidLineageParent {
            parent: plural_shell_digest,
        });
    }

    if let Some(policy) = policy {
        let derived = BraidShell::assemble(
            plural.worldline_id,
            plural.basis,
            plural.members.clone(),
            policy.policy_id,
            BraidShellOutcome::Derived {
                result_refs: selected_result_refs,
                collapse_policy: Some(policy.policy_id),
                collapse_witness: Some(*policy.witness.as_hash()),
                collapsed_from: Some(plural_shell_digest),
            },
            plural.posture,
        )?;
        return Ok(CollapseResult::Derived(derived));
    }

    let mut witness_hasher = Hasher::new();
    witness_hasher.update(COLLAPSE_OBSTRUCTION_WITNESS_DOMAIN);
    witness_hasher.update(&plural_shell_digest);
    let mut policy_hasher = Hasher::new();
    policy_hasher.update(ABSENT_COLLAPSE_POLICY_DOMAIN);
    let obstructed = BraidShell::assemble(
        plural.worldline_id,
        plural.basis,
        plural.members.clone(),
        policy_hasher.finalize().into(),
        BraidShellOutcome::Obstruction {
            reason_code: COLLAPSE_WITHOUT_POLICY_REASON,
            witness: witness_hasher.finalize().into(),
            obstructed_from: Some(plural_shell_digest),
        },
        plural.posture,
    )?;
    Ok(CollapseResult::Obstructed(obstructed))
}

/// Kind discriminator for the retained boundary shell family.
///
/// θ_tick and θ_braid are siblings in one retained boundary family
/// (AIΩN Paper VII Prop 3.5); θ_import joins later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetainedBoundaryKind {
    /// A contiguous tick-segment boundary record.
    Tick,
    /// A braid-scale settlement boundary shell.
    Braid,
}

/// Shared contract for retained boundary shells of every scale.
pub trait RetainedBoundaryRecord {
    /// Returns which family member this record is.
    fn boundary_kind(&self) -> RetainedBoundaryKind;
    /// Returns the canonical content digest for the record.
    fn boundary_digest(&self) -> Hash;
}

impl RetainedBoundaryRecord for BraidShell {
    fn boundary_kind(&self) -> RetainedBoundaryKind {
        RetainedBoundaryKind::Braid
    }

    fn boundary_digest(&self) -> Hash {
        self.digest
    }
}

const TICK_SHELL_DOMAIN: &[u8] = b"echo.shell.tick.v1\0";

impl RetainedBoundaryRecord for crate::provenance_store::BoundaryTransitionRecord {
    fn boundary_kind(&self) -> RetainedBoundaryKind {
        RetainedBoundaryKind::Tick
    }

    /// Full canonical content digest over the retained BTR body: boundary
    /// facts, payload coordinates, and for every entry its coordinate,
    /// event kind, head key, parents, hash triplet, retained patch body
    /// commitment (header fields explicitly — `patch_digest` does not bind
    /// the header — plus the canonical `patch_digest` which binds ops and
    /// slots), materialization outputs, and atom-write provenance, plus the
    /// auth tag. No retained field named like content is excluded — this is
    /// a record digest, not a vibes checksum.
    fn boundary_digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(TICK_SHELL_DOMAIN);
        hasher.update(self.worldline_id.as_bytes());
        hasher.update(self.u0_ref.as_bytes());
        hasher.update(&self.input_boundary_hash);
        hasher.update(&self.output_boundary_hash);
        hasher.update(&self.logical_counter.to_le_bytes());
        hasher.update(self.payload.worldline_id.as_bytes());
        hasher.update(&self.payload.start_worldline_tick.as_u64().to_le_bytes());
        hasher.update(&(self.payload.entries.len() as u64).to_le_bytes());
        for entry in &self.payload.entries {
            hasher.update(entry.worldline_id.as_bytes());
            hasher.update(&entry.worldline_tick.as_u64().to_le_bytes());
            hasher.update(&entry.commit_global_tick.as_u64().to_le_bytes());
            match &entry.head_key {
                Some(head_key) => {
                    hasher.update(&[1]);
                    hasher.update(head_key.worldline_id.as_bytes());
                    hasher.update(head_key.head_id.as_bytes());
                }
                None => {
                    hasher.update(&[0]);
                }
            }
            crate::coordinator::hash_provenance_event_kind(&mut hasher, &entry.event_kind);
            hasher.update(&(entry.parents.len() as u64).to_le_bytes());
            for parent in &entry.parents {
                hash_provenance_ref(&mut hasher, parent);
            }
            hasher.update(&entry.expected.state_root);
            hasher.update(&entry.expected.patch_digest);
            hasher.update(&entry.expected.commit_hash);
            match &entry.patch {
                Some(patch) => {
                    hasher.update(&[1]);
                    hasher.update(&patch.header.commit_global_tick.as_u64().to_le_bytes());
                    hasher.update(&patch.header.policy_id.to_le_bytes());
                    hasher.update(&patch.header.rule_pack_id);
                    hasher.update(&patch.header.plan_digest);
                    hasher.update(&patch.header.decision_digest);
                    hasher.update(&patch.header.rewrites_digest);
                    hasher.update(patch.warp_id.as_bytes());
                    hasher.update(&patch.patch_digest);
                }
                None => {
                    hasher.update(&[0]);
                }
            }
            hasher.update(&(entry.outputs.len() as u64).to_le_bytes());
            for (channel, data) in &entry.outputs {
                hasher.update(&(channel.0.len() as u64).to_le_bytes());
                hasher.update(channel.0.as_ref());
                hasher.update(&(data.len() as u64).to_le_bytes());
                hasher.update(data);
            }
            hasher.update(&(entry.atom_writes.len() as u64).to_le_bytes());
            for atom_write in &entry.atom_writes {
                hasher.update(atom_write.atom.warp_id.as_bytes());
                hasher.update(atom_write.atom.local_id.as_bytes());
                hasher.update(&atom_write.rule_id);
                hasher.update(&atom_write.tick.to_le_bytes());
                match &atom_write.old_value {
                    Some(old_value) => {
                        hasher.update(&[1]);
                        hasher.update(&(old_value.len() as u64).to_le_bytes());
                        hasher.update(old_value);
                    }
                    None => {
                        hasher.update(&[0]);
                    }
                }
                hasher.update(&(atom_write.new_value.len() as u64).to_le_bytes());
                hasher.update(&atom_write.new_value);
            }
        }
        hasher.update(&(self.auth_tag.len() as u64).to_le_bytes());
        hasher.update(&self.auth_tag);
        hasher.finalize().into()
    }
}

/// Secure member lookup material for sealed braid member references.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BraidShellMemberQuery {
    /// Strand identity being matched.
    pub strand_id: StrandId,
    /// Child worldline identity bound into sealed member commitments.
    pub child_worldline_id: WorldlineId,
    /// Causal authority domain bound into the sealed member reference.
    pub authority: AuthorityDomainRef,
    /// Non-public blinding material used to derive sealed member commitments.
    pub blinding_secret: Hash,
}

impl core::fmt::Debug for BraidShellMemberQuery {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BraidShellMemberQuery")
            .field("strand_id", &self.strand_id)
            .field("child_worldline_id", &self.child_worldline_id)
            .field("authority", &self.authority)
            .field("blinding_secret", &"<redacted>")
            .finish()
    }
}

/// Scan-backed query over retained braid shells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BraidShellQuery {
    /// Match the shell at this braid coordinate.
    pub coordinate: Option<BraidCoordinate>,
    /// Match shells judged against this comparison basis.
    pub basis: Option<ProvenanceRef>,
    /// Match shells summarizing this revealed member strand.
    pub revealed_member_strand: Option<StrandId>,
    /// Match shells summarizing this member using sealed-reference material.
    pub secure_member: Option<BraidShellMemberQuery>,
    /// Match shells with this outcome arm.
    pub outcome: Option<AdmissionOutcomeKind>,
    /// Match shells with this revelation posture.
    pub posture: Option<CausalPosture>,
}

impl BraidShell {
    /// Returns whether the shell matches every present query field.
    #[must_use]
    pub fn matches(&self, query: &BraidShellQuery) -> bool {
        query
            .coordinate
            .is_none_or(|coordinate| self.coordinate == coordinate)
            && query.basis.is_none_or(|basis| self.basis == basis)
            && query
                .revealed_member_strand
                .as_ref()
                .is_none_or(|strand| self.has_revealed_member_strand(strand))
            && query.secure_member.is_none_or(|member| {
                self.has_member_strand_secure(
                    &member.strand_id,
                    &member.child_worldline_id,
                    &member.authority,
                    &member.blinding_secret,
                )
            })
            && query
                .outcome
                .is_none_or(|outcome| self.outcome_kind() == outcome)
            && query.posture.is_none_or(|posture| self.posture == posture)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::clock::WorldlineTick;
    use crate::strand::make_strand_id;
    use std::collections::BTreeMap;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    fn basis_ref() -> ProvenanceRef {
        ProvenanceRef {
            worldline_id: wl(1),
            worldline_tick: WorldlineTick::from_raw(0),
            commit_hash: [0x11; 32],
        }
    }

    fn member(label: &str, verdict: MemberVerdict) -> BraidShellMember {
        BraidShellMember {
            member_ref: BraidMemberRef::Revealed(make_strand_id(label)),
            support_pin_digest: [0x21; 32],
            basis_digest: [0x22; 32],
            frontier_digest: [0x23; 32],
            footprint_digest: [0x24; 32],
            claim_digest: [0x25; 32],
            verdict,
            verdict_digest: [0x26; 32],
            posture: CausalPosture::AuthorOnly,
        }
    }

    fn authority(origin: u8, domain: u8) -> AuthorityDomainRef {
        AuthorityDomainRef::new(
            crate::revelation::OriginId::from_bytes([origin; 32]),
            crate::revelation::AuthorityDomainId::from_bytes([domain; 32]),
        )
    }

    fn sealed_member(
        commitment: Hash,
        authority: AuthorityDomainRef,
        verdict: MemberVerdict,
        claim_byte: u8,
    ) -> BraidShellMember {
        BraidShellMember {
            member_ref: BraidMemberRef::Sealed {
                blinded_commitment: commitment,
                authority,
            },
            support_pin_digest: [0x21; 32],
            basis_digest: [0x22; 32],
            frontier_digest: [0x23; 32],
            footprint_digest: [0x24; 32],
            claim_digest: [claim_byte; 32],
            verdict,
            verdict_digest: [0x26; 32],
            posture: CausalPosture::AuthorOnly,
        }
    }

    fn plural_shell(members: Vec<BraidShellMember>) -> BraidShell {
        BraidShell::assemble(
            wl(1),
            basis_ref(),
            members,
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        )
        .unwrap()
    }

    struct Records(BTreeMap<Hash, BraidShell>);

    impl Records {
        fn with(shells: impl IntoIterator<Item = BraidShell>) -> Self {
            Self(
                shells
                    .into_iter()
                    .map(|shell| (shell.digest, shell))
                    .collect(),
            )
        }
    }

    impl BraidShellRecords for Records {
        fn shell(&self, digest: &Hash) -> Option<&BraidShell> {
            self.0.get(digest)
        }
    }

    #[test]
    fn member_order_permutation_cannot_move_the_shell_digest() {
        let a = member("member-a", MemberVerdict::Plural);
        let b = member("member-b", MemberVerdict::Derived);
        let forward = plural_shell(vec![a.clone(), b.clone()]);
        let reversed = plural_shell(vec![b, a]);

        assert_eq!(forward.digest, reversed.digest);
        assert_eq!(forward, reversed);
    }

    #[test]
    fn replay_reproduces_outcome_and_member_verdicts_from_records_alone() {
        use crate::plurality_law::PluralityLawRef;

        let shell = plural_shell(vec![
            member("member-a", MemberVerdict::Plural),
            member("member-b", MemberVerdict::Derived),
        ]);
        let digest = shell.digest;
        let expected_verdicts: Vec<(BraidMemberRef, MemberVerdict)> = shell
            .members
            .iter()
            .map(|member| (member.member_ref, member.verdict))
            .collect();
        let records = Records::with([shell]);

        let replay = replay_braid_shell(&digest, &records).unwrap();
        assert_eq!(replay.outcome_kind, AdmissionOutcomeKind::Plural);
        assert_eq!(replay.member_verdicts, expected_verdicts);
        assert_eq!(replay.policy_id, [0x5E; 32]);
        assert_eq!(
            Ok(replay.law_ref),
            PluralityLawRef::settlement_policy([0x5E; 32])
        );
        assert_eq!(replay.posture, CausalPosture::AuthorOnly);
    }

    #[test]
    fn replay_audit_reports_member_proof_support_frontier_and_witness_facts() {
        use crate::plurality_law::{PluralityLawEvidencePosture, PluralityLawRef};
        use crate::proof::{ProofEnvelope, ProofKind};
        use crate::sealed_membership::DisclosureBudget;
        use crate::witness::{WitnessAttestation, WitnessCompatibilityRule, WitnessKind};

        let members = vec![member("audit-member", MemberVerdict::Plural)];
        let temp_shell = BraidShell::assemble(
            wl(1),
            basis_ref(),
            members.clone(),
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        )
        .unwrap();
        let proof = ProofEnvelope {
            kind: ProofKind::ReplayTrace,
            proof_bytes: vec![1, 2, 3],
            public_inputs_hash: temp_shell.witness_digest,
        };
        let shell = BraidShell::assemble_with_proof(
            wl(1),
            basis_ref(),
            members,
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
            Some(proof),
        )
        .unwrap();
        let digest = shell.digest;
        let expected_member = shell.members[0].clone();
        let expected_proof_digest = shell.proof.as_ref().unwrap().digest();
        let expected_witness = shell.witness_digest;
        let records = Records::with([shell]);

        let audit = audit_braid_shell(&digest, &records).unwrap();

        assert_eq!(audit.shell_digest, digest);
        assert_eq!(audit.outcome_kind, AdmissionOutcomeKind::Plural);
        assert_eq!(
            Ok(audit.law_reading.law_ref()),
            PluralityLawRef::settlement_policy([0x5E; 32])
        );
        assert_eq!(audit.law_reading.support_digest(), digest);
        assert_eq!(
            audit.law_reading.evidence_posture(),
            PluralityLawEvidencePosture::SelfWitnessIntegrityOnly
        );
        assert_eq!(
            audit.law_reading.disclosure_budget(),
            DisclosureBudget::Public
        );
        assert_eq!(audit.posture_floor, CausalPosture::AuthorOnly);
        assert_eq!(audit.shell_posture, CausalPosture::AuthorOnly);
        assert_eq!(audit.settlement_frontier, vec![expected_member.member_ref]);
        assert_eq!(
            audit.member_facts,
            vec![BraidShellMemberAuditFact {
                member_ref: expected_member.member_ref,
                verdict: expected_member.verdict,
                posture: expected_member.posture,
                support_pin_digest: expected_member.support_pin_digest,
                basis_digest: expected_member.basis_digest,
                frontier_digest: expected_member.frontier_digest,
                footprint_digest: expected_member.footprint_digest,
                claim_digest: expected_member.claim_digest,
                verdict_digest: expected_member.verdict_digest,
                disclosure_budget: DisclosureBudget::Public,
            }]
        );
        assert_eq!(
            audit.proof_binding,
            BraidProofBinding::Matched {
                kind: ProofKind::ReplayTrace,
                envelope_digest: expected_proof_digest,
                public_inputs_hash: expected_witness,
            }
        );
        assert_eq!(
            audit.witness_posture,
            BraidWitnessPosture::SelfWitnessIntegrityOnly {
                digest: expected_witness,
            }
        );
        assert_eq!(audit.witness_receipt.kind(), WitnessKind::SelfWitness);
        assert_eq!(audit.witness_receipt.subject_digest(), digest);
        assert_eq!(audit.witness_receipt.evidence_digest(), expected_witness);
        assert_eq!(
            audit.witness_receipt.compatibility(),
            WitnessCompatibilityRule::E1Scaffold
        );
        assert_eq!(
            audit.witness_receipt.attestation(),
            WitnessAttestation::IntegrityOnly
        );
    }

    #[test]
    fn replay_audit_labels_sealed_member_disclosure_budget() {
        use crate::sealed_membership::DisclosureBudget;

        let shell = plural_shell(vec![sealed_member(
            [0xAA; 32],
            authority(0x01, 0x02),
            MemberVerdict::Plural,
            0x25,
        )]);
        let digest = shell.digest;
        let records = Records::with([shell]);

        let audit = audit_braid_shell(&digest, &records).unwrap();

        assert_eq!(audit.member_facts.len(), 1);
        assert!(matches!(
            audit.member_facts[0].member_ref,
            BraidMemberRef::Sealed { .. }
        ));
        assert_eq!(
            audit.member_facts[0].disclosure_budget,
            DisclosureBudget::AuthorityScoped
        );
        assert_eq!(
            audit.law_reading.disclosure_budget(),
            DisclosureBudget::AuthorityScoped
        );
    }

    #[test]
    fn tampering_with_policy_posture_or_verdict_fails_replay() {
        let shell = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        let digest = shell.digest;

        let mut policy_tampered = shell.clone();
        policy_tampered.policy_id = [0xAA; 32];
        let records = Records::with([policy_tampered]);
        // The tampered shell no longer lives at its claimed digest, and its
        // stored digest no longer matches its body either way.
        assert!(replay_braid_shell(&digest, &records).is_err());

        let mut verdict_tampered = shell.clone();
        verdict_tampered.members[0].verdict_digest = [0xBB; 32];
        assert!(matches!(
            verdict_tampered.validate(),
            Err(BraidShellError::WitnessMismatch { .. }
                | BraidShellError::DigestMismatch { .. }
                | BraidShellError::CoordinateMismatch)
        ));

        let mut posture_tampered = shell.clone();
        posture_tampered.posture = CausalPosture::Scratch;
        assert!(matches!(
            posture_tampered.validate(),
            Err(BraidShellError::WitnessMismatch { .. } | BraidShellError::DigestMismatch { .. })
        ));

        let mut outcome_tampered = shell;
        outcome_tampered.outcome = BraidShellOutcome::Conflict {
            reason_codes: vec![4],
        };
        assert!(outcome_tampered.validate().is_err());
    }

    #[test]
    fn tampering_with_coordinate_fails_validation() {
        let mut shell = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        shell.coordinate = BraidCoordinate([0xCC; 32]);
        assert_eq!(shell.validate(), Err(BraidShellError::CoordinateMismatch));
    }

    #[test]
    fn derived_shell_rejects_empty_collapse_witness() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Derived)],
            [0x5E; 32],
            BraidShellOutcome::Derived {
                result_refs: vec![basis_ref()],
                collapse_policy: Some([0x77; 32]),
                collapse_witness: Some([0; 32]),
                collapsed_from: Some([0x88; 32]),
            },
            CausalPosture::AuthorOnly,
        );
        assert_eq!(result, Err(BraidShellError::EmptyWitness));
    }

    #[test]
    fn obstruction_shell_rejects_empty_witness() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Obstructed)],
            [0x5E; 32],
            BraidShellOutcome::Obstruction {
                reason_code: 1,
                witness: crate::blake3_empty(),
                obstructed_from: None,
            },
            CausalPosture::AuthorOnly,
        );
        assert_eq!(result, Err(BraidShellError::EmptyWitness));
    }

    #[test]
    fn shell_rejects_empty_policy_id() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Plural)],
            [0; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        );

        assert_eq!(result, Err(BraidShellError::EmptyPolicyId));
    }

    #[test]
    fn duplicate_alternative_ids_are_refused() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Plural)],
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32], [0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        );
        assert_eq!(
            result,
            Err(BraidShellError::DuplicateAlternativeId {
                alternative_id: [0x31; 32],
            })
        );
    }

    #[test]
    fn duplicate_member_strands_are_refused() {
        let mut duplicate = member("member-a", MemberVerdict::Plural);
        duplicate.claim_digest = [0x99; 32];
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Plural), duplicate],
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        );
        assert_eq!(
            result,
            Err(BraidShellError::DuplicateMemberStrand {
                member_ref: BraidMemberRef::Revealed(make_strand_id("member-a")),
            })
        );
    }

    #[test]
    fn sealed_members_with_same_commitment_under_different_authorities_are_distinct() {
        let commitment = [0x44; 32];
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![
                sealed_member(
                    commitment,
                    authority(0x10, 0x20),
                    MemberVerdict::Plural,
                    0x25,
                ),
                sealed_member(
                    commitment,
                    authority(0x11, 0x20),
                    MemberVerdict::Plural,
                    0x26,
                ),
            ],
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn mixed_revealed_and_sealed_members_are_refused() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![
                member("member-a", MemberVerdict::Plural),
                sealed_member(
                    [0x44; 32],
                    authority(0x10, 0x20),
                    MemberVerdict::Plural,
                    0x26,
                ),
            ],
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        );

        assert_eq!(result, Err(BraidShellError::MixedMemberReferencePosture));
    }

    #[test]
    fn obstructed_collapse_names_its_plural_parent() {
        let plural = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        let plural_digest = plural.digest;
        let records = Records::with([plural.clone()]);

        let CollapseResult::Obstructed(obstructed) =
            collapse_braid_shell(&records, plural_digest, Vec::new(), None).unwrap()
        else {
            unreachable!("collapse without policy must obstruct");
        };
        let BraidShellOutcome::Obstruction {
            obstructed_from, ..
        } = obstructed.outcome
        else {
            unreachable!("obstructed collapse carries an obstruction outcome");
        };
        assert_eq!(obstructed_from, Some(plural_digest));

        // Replay verifies the named parent is retained and plural.
        let mut store = Records::with([plural]);
        let CollapseResult::Obstructed(obstructed) =
            collapse_braid_shell(&store, plural_digest, Vec::new(), None).unwrap()
        else {
            unreachable!();
        };
        store.0.insert(obstructed.digest, obstructed.clone());
        let replay = replay_braid_shell(&obstructed.digest, &store).unwrap();
        assert_eq!(replay.outcome_kind, AdmissionOutcomeKind::Obstruction);
    }

    #[test]
    fn shells_are_queryable_by_coordinate() {
        let shell = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        assert!(shell.matches(&BraidShellQuery {
            coordinate: Some(shell.coordinate),
            ..BraidShellQuery::default()
        }));
        assert!(!shell.matches(&BraidShellQuery {
            coordinate: Some(BraidCoordinate([0xDD; 32])),
            ..BraidShellQuery::default()
        }));
    }

    #[test]
    fn shell_posture_cannot_exceed_least_revealed_member() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Plural)],
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::Shared,
        );
        assert!(matches!(
            result,
            Err(BraidShellError::PostureExceedsMembers(_))
        ));
    }

    #[test]
    fn outcome_arm_must_agree_with_member_verdicts() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Derived)],
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        );
        assert_eq!(
            result,
            Err(BraidShellError::OutcomeMemberMismatch {
                outcome: AdmissionOutcomeKind::Plural,
            })
        );
    }

    #[test]
    fn collapse_lineage_requires_a_retained_plural_parent() {
        let plural = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        let plural_digest = plural.digest;
        let derived = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Derived)],
            [0x5E; 32],
            BraidShellOutcome::Derived {
                result_refs: vec![basis_ref()],
                collapse_policy: Some([0x77; 32]),
                collapse_witness: Some([0x78; 32]),
                collapsed_from: Some(plural_digest),
            },
            CausalPosture::AuthorOnly,
        )
        .unwrap();
        let derived_digest = derived.digest;

        let complete = Records::with([plural, derived.clone()]);
        let replay = replay_braid_shell(&derived_digest, &complete).unwrap();
        assert_eq!(replay.outcome_kind, AdmissionOutcomeKind::Derived);

        let missing_parent = Records::with([derived]);
        assert_eq!(
            replay_braid_shell(&derived_digest, &missing_parent),
            Err(BraidShellError::InvalidLineageParent {
                parent: plural_digest,
            })
        );
    }

    #[test]
    fn tampering_with_version_fails_validation() {
        let mut shell = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        shell.version = 999;
        assert_eq!(
            shell.validate(),
            Err(BraidShellError::UnsupportedVersion {
                stored: 999,
                supported: BRAID_SHELL_VERSION,
            })
        );
    }

    #[test]
    fn plural_alternatives_are_a_canonical_set() {
        let members = vec![member("member-a", MemberVerdict::Plural)];
        let forward = BraidShell::assemble(
            wl(1),
            basis_ref(),
            members.clone(),
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x32; 32], [0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        )
        .unwrap();
        let reversed = BraidShell::assemble(
            wl(1),
            basis_ref(),
            members,
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32], [0x32; 32]],
            },
            CausalPosture::AuthorOnly,
        )
        .unwrap();
        assert_eq!(forward.digest, reversed.digest);

        let mut tampered = forward;
        tampered.outcome = BraidShellOutcome::Plural {
            alternative_ids: vec![[0x32; 32], [0x31; 32]],
        };
        assert_eq!(
            tampered.validate(),
            Err(BraidShellError::NonCanonicalAlternativeOrder)
        );
    }

    // WitnessDigest shrug-rejection is owned and tested in `revelation`;
    // `check_outcome_law` maps it to `BraidShellError::EmptyWitness`, covered
    // by `derived_shell_rejects_empty_collapse_witness` /
    // `obstruction_shell_rejects_empty_witness` below.

    #[test]
    fn collapse_with_named_policy_derives_without_mutating_the_plural_parent() {
        use crate::plurality_law::PluralityLawFamily;

        let plural = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        let plural_digest = plural.digest;
        let snapshot = plural.clone();
        let records = Records::with([plural]);

        let policy = CollapsePolicy {
            policy_id: [0x77; 32],
            witness: WitnessDigest::new([0x99; 32]).unwrap(),
        };
        let CollapseResult::Derived(derived) =
            collapse_braid_shell(&records, plural_digest, vec![basis_ref()], Some(policy)).unwrap()
        else {
            unreachable!("named collapse policy must derive");
        };

        assert_eq!(
            derived.outcome,
            BraidShellOutcome::Derived {
                result_refs: vec![basis_ref()],
                collapse_policy: Some([0x77; 32]),
                collapse_witness: Some([0x99; 32]),
                collapsed_from: Some(plural_digest),
            }
        );
        assert_eq!(records.shell(&plural_digest), Some(&snapshot));

        let mut store = Records::with([snapshot]);
        store.0.insert(derived.digest, derived.clone());
        let replay = replay_braid_shell(&derived.digest, &store).unwrap();
        assert_eq!(replay.outcome_kind, AdmissionOutcomeKind::Derived);
        assert_eq!(replay.law_ref.family(), PluralityLawFamily::Collapse);
        let audit = audit_braid_shell(&derived.digest, &store).unwrap();
        assert_eq!(
            audit.law_reading.law_ref().family(),
            PluralityLawFamily::Collapse
        );
    }

    #[test]
    fn collapse_without_policy_is_a_retained_obstruction() {
        let plural = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        let plural_digest = plural.digest;
        let records = Records::with([plural]);

        let CollapseResult::Obstructed(obstructed) =
            collapse_braid_shell(&records, plural_digest, Vec::new(), None).unwrap()
        else {
            unreachable!("collapse without policy must obstruct");
        };

        assert_eq!(obstructed.outcome_kind(), AdmissionOutcomeKind::Obstruction);
        let BraidShellOutcome::Obstruction { reason_code, .. } = obstructed.outcome else {
            unreachable!("obstructed collapse carries an obstruction outcome");
        };
        assert_eq!(reason_code, COLLAPSE_WITHOUT_POLICY_REASON);
        obstructed.validate().unwrap();
    }

    #[test]
    fn tick_and_braid_records_are_one_boundary_family() {
        let shell = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);
        assert_eq!(shell.boundary_kind(), RetainedBoundaryKind::Braid);
        assert_eq!(shell.boundary_digest(), shell.digest);
    }

    #[test]
    fn query_matches_by_basis_member_outcome_and_posture() {
        let shell = plural_shell(vec![member("member-a", MemberVerdict::Plural)]);

        assert!(shell.matches(&BraidShellQuery::default()));
        assert!(shell.matches(&BraidShellQuery {
            coordinate: Some(shell.coordinate),
            basis: Some(basis_ref()),
            revealed_member_strand: Some(make_strand_id("member-a")),
            secure_member: None,
            outcome: Some(AdmissionOutcomeKind::Plural),
            posture: Some(CausalPosture::AuthorOnly),
        }));
        assert!(!shell.matches(&BraidShellQuery {
            outcome: Some(AdmissionOutcomeKind::Conflict),
            ..BraidShellQuery::default()
        }));
        assert!(!shell.matches(&BraidShellQuery {
            revealed_member_strand: Some(make_strand_id("nobody")),
            ..BraidShellQuery::default()
        }));
    }

    #[test]
    fn sealed_member_query_requires_secure_material() {
        let strand_id = make_strand_id("sealed-member");
        let child_worldline_id = wl(9);
        let member_authority = authority(0xA1, 0xB1);
        let blinding_secret = [0xA5; 32];
        let member_ref = BraidMemberRef::seal(strand_id, child_worldline_id, blinding_secret);
        let shell = plural_shell(vec![sealed_member(
            member_ref,
            member_authority,
            MemberVerdict::Plural,
            0x27,
        )]);

        assert!(!shell.has_revealed_member_strand(&strand_id));
        assert!(shell.has_member_strand_secure(
            &strand_id,
            &child_worldline_id,
            &member_authority,
            &blinding_secret
        ));
        assert!(!shell.has_member_strand_secure(
            &strand_id,
            &child_worldline_id,
            &authority(0xA1, 0xB2),
            &blinding_secret
        ));
        assert!(!shell.matches(&BraidShellQuery {
            revealed_member_strand: Some(strand_id),
            ..BraidShellQuery::default()
        }));
        assert!(shell.matches(&BraidShellQuery {
            secure_member: Some(BraidShellMemberQuery {
                strand_id,
                child_worldline_id,
                authority: member_authority,
                blinding_secret,
            }),
            ..BraidShellQuery::default()
        }));
        let debug = format!(
            "{:?}",
            BraidShellMemberQuery {
                strand_id,
                child_worldline_id,
                authority: member_authority,
                blinding_secret,
            }
        );
        assert!(debug.contains("blinding_secret: \"<redacted>\""));
        assert!(!debug.contains("blinding_secret: ["));
    }

    #[test]
    fn assemble_with_proof_validates_envelope() {
        use crate::proof::{ProofEnvelope, ProofError, ProofKind};

        let members = vec![member("member-a", MemberVerdict::Plural)];

        // Build it without proof first to retrieve the expected witness digest.
        let temp_shell = BraidShell::assemble(
            wl(1),
            basis_ref(),
            members.clone(),
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        )
        .unwrap();
        assert_eq!(temp_shell.version, BRAID_SHELL_VERSION);
        assert_eq!(BRAID_SHELL_VERSION, 2);
        let expected_witness = temp_shell.witness_digest;

        // Valid replay-trace evidence: matches the witness_digest and has non-empty bytes.
        let valid_proof = ProofEnvelope {
            kind: ProofKind::ReplayTrace,
            proof_bytes: vec![1, 2, 3],
            public_inputs_hash: expected_witness,
        };

        let shell_with_valid_proof = BraidShell::assemble_with_proof(
            wl(1),
            basis_ref(),
            members.clone(),
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
            Some(valid_proof),
        )
        .unwrap();
        assert_eq!(shell_with_valid_proof.version, BRAID_SHELL_VERSION);
        shell_with_valid_proof.validate().unwrap();
        assert_ne!(
            temp_shell.digest, shell_with_valid_proof.digest,
            "proof-bearing shells must have a distinct content identity"
        );

        let mut proof_tampered = shell_with_valid_proof;
        assert!(proof_tampered.proof.is_some());
        if let Some(proof) = proof_tampered.proof.as_mut() {
            proof.proof_bytes.push(4);
        }
        assert!(matches!(
            proof_tampered.validate(),
            Err(BraidShellError::DigestMismatch { .. })
        ));

        // Invalid proof: mismatched public inputs hash
        let invalid_proof_mismatch = ProofEnvelope {
            kind: ProofKind::ReplayTrace,
            proof_bytes: vec![1, 2, 3],
            public_inputs_hash: [0x99; 32],
        };
        let result_mismatch = BraidShell::assemble_with_proof(
            wl(1),
            basis_ref(),
            members.clone(),
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
            Some(invalid_proof_mismatch),
        );
        assert!(matches!(
            result_mismatch,
            Err(BraidShellError::ProofShapeValidationFailed {
                reason: ProofError::PublicInputsMismatch {
                    expected,
                    actual,
                },
            }) if expected == expected_witness && actual == [0x99; 32]
        ));

        // Invalid proof: empty proof bytes
        let invalid_proof_empty = ProofEnvelope {
            kind: ProofKind::ReplayTrace,
            proof_bytes: Vec::new(),
            public_inputs_hash: expected_witness,
        };
        let result_empty = BraidShell::assemble_with_proof(
            wl(1),
            basis_ref(),
            members,
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
            Some(invalid_proof_empty),
        );
        assert!(matches!(
            result_empty,
            Err(BraidShellError::ProofShapeValidationFailed {
                reason: ProofError::EmptyPayload,
            })
        ));
    }

    #[test]
    fn cryptographic_proof_kinds_require_verifier_backend() {
        use crate::proof::{ProofEnvelope, ProofError, ProofKind};

        let members = vec![member("member-a", MemberVerdict::Plural)];
        let temp_shell = BraidShell::assemble(
            wl(1),
            basis_ref(),
            members.clone(),
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            CausalPosture::AuthorOnly,
        )
        .unwrap();

        for kind in [ProofKind::ZkSnark, ProofKind::VectorOpening] {
            let result = BraidShell::assemble_with_proof(
                wl(1),
                basis_ref(),
                members.clone(),
                [0x5E; 32],
                BraidShellOutcome::Plural {
                    alternative_ids: vec![[0x31; 32]],
                },
                CausalPosture::AuthorOnly,
                Some(ProofEnvelope {
                    kind,
                    proof_bytes: vec![1, 2, 3],
                    public_inputs_hash: temp_shell.witness_digest,
                }),
            );
            assert!(matches!(
                result,
                Err(BraidShellError::ProofShapeValidationFailed {
                    reason: ProofError::UnsupportedKind { kind: rejected },
                }) if rejected == kind
            ));
        }
    }

    #[test]
    fn test_secure_sealed_member_matching() {
        let strand_id = make_strand_id("secure-member");
        let child_worldline = WorldlineId::from_bytes([0x88; 32]);
        let member_authority = authority(0x10, 0x20);
        let blinding_secret = [0x44; 32];

        let blinded_commitment = BraidMemberRef::seal(strand_id, child_worldline, blinding_secret);
        let sealed_ref = BraidMemberRef::Sealed {
            blinded_commitment,
            authority: member_authority,
        };

        // Verification matches correctly.
        assert!(sealed_ref.matches_strand(
            &strand_id,
            &child_worldline,
            &member_authority,
            &blinding_secret
        ));

        // Revealed references are not accepted by the sealed secure path.
        assert!(!BraidMemberRef::Revealed(strand_id).matches_strand(
            &strand_id,
            &child_worldline,
            &member_authority,
            &blinding_secret
        ));

        // Mismatched strand_id fails.
        let wrong_strand_id = make_strand_id("wrong-member");
        assert!(!sealed_ref.matches_strand(
            &wrong_strand_id,
            &child_worldline,
            &member_authority,
            &blinding_secret
        ));

        // Mismatched child_worldline fails.
        let wrong_child_worldline = WorldlineId::from_bytes([0x99; 32]);
        assert!(!sealed_ref.matches_strand(
            &strand_id,
            &wrong_child_worldline,
            &member_authority,
            &blinding_secret
        ));

        // Mismatched authority fails.
        assert!(!sealed_ref.matches_strand(
            &strand_id,
            &child_worldline,
            &authority(0x10, 0x21),
            &blinding_secret
        ));

        // Mismatched blinding secret fails.
        assert!(!sealed_ref.matches_strand(
            &strand_id,
            &child_worldline,
            &member_authority,
            &[0x45; 32]
        ));
    }
}
