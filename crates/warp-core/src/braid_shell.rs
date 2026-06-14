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
use crate::provenance_store::ProvenanceRef;
use crate::revelation::{
    shell_posture_obstruction, AuthorityDomainRef, CausalPosture, PostureObstruction, WitnessDigest,
};
use crate::strand::StrandId;
use crate::worldline::WorldlineId;

const SHELL_DOMAIN: &[u8] = b"echo.shell.braid.v1\0";
const MEMBER_DOMAIN: &[u8] = b"echo.braid.member.v1\0";
const WITNESS_DOMAIN: &[u8] = b"echo.braid.witness.v1\0";
const COORDINATE_DOMAIN: &[u8] = b"echo.braid.coordinate.v1\0";

/// Current braid shell body version.
pub const BRAID_SHELL_VERSION: u32 = 1;

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

/// Reference to a braid member, supporting both revealed and cryptographically sealed references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BraidMemberRef {
    /// Publicly revealed strand identity.
    Revealed(StrandId),
    /// Cryptographically sealed/blinded member reference.
    Sealed {
        /// Salted or randomized commitment digest of the member's identity.
        blinded_commitment: Hash,
        /// Causal authority domain controlling the private history.
        authority: AuthorityDomainRef,
    },
}

impl BraidMemberRef {
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
    /// The proof verification failed.
    #[error("proof verification failed: {reason}")]
    ProofVerificationFailed {
        /// Reason for verification failure.
        reason: String,
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
    /// Optional proof envelope verifying the correctness of this settlement.
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
    /// [`BraidShellError::DuplicateAlternativeId`]), an empty witness on a
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

    /// Assembles a shell with a cryptographic proof envelope: validates member
    /// order, checks posture floor and coherence, verifies the proof envelope
    /// (if present) against the derived witness, and seals the shell.
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
            if let Err(err) = p.verify(witness_digest) {
                return Err(BraidShellError::ProofVerificationFailed { reason: err });
            }
        }

        let digest = compute_shell_digest(
            BRAID_SHELL_VERSION,
            worldline_id,
            &basis,
            &member_digests,
            policy_id,
            &outcome,
            witness_digest,
            posture,
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
    /// non-canonical or duplicate plural alternatives, an empty
    /// collapse/obstruction witness, a posture floor violation, an
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
            if let Err(err) = p.verify(self.witness_digest) {
                return Err(BraidShellError::ProofVerificationFailed { reason: err });
            }
        }

        let digest = compute_shell_digest(
            self.version,
            self.worldline_id,
            &self.basis,
            &member_digests,
            self.policy_id,
            &self.outcome,
            self.witness_digest,
            self.posture,
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

    /// Returns whether the shell summarizes the given member strand.
    #[must_use]
    pub fn has_member_strand(&self, strand_id: &StrandId) -> bool {
        self.members.iter().any(|member| match member.member_ref {
            BraidMemberRef::Revealed(id) => id == *strand_id,
            BraidMemberRef::Sealed { .. } => false,
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

/// One strand may appear at most once among shell members.
fn check_unique_member_strands(members: &[BraidShellMember]) -> Result<(), BraidShellError> {
    for (index, member) in members.iter().enumerate() {
        if members[..index]
            .iter()
            .any(|earlier| match (earlier.member_ref, member.member_ref) {
                (BraidMemberRef::Revealed(e_id), BraidMemberRef::Revealed(m_id)) => e_id == m_id,
                (
                    BraidMemberRef::Sealed {
                        blinded_commitment: e_c,
                        ..
                    },
                    BraidMemberRef::Sealed {
                        blinded_commitment: m_c,
                        ..
                    },
                ) => e_c == m_c,
                _ => false,
            })
        {
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
    /// Settlement policy identity the act ran under.
    pub policy_id: Hash,
    /// Witness digest binding the act.
    pub witness_digest: Hash,
    /// Revelation posture of the shell.
    pub posture: CausalPosture,
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
    Ok(BraidShellReplay {
        outcome_kind: shell.outcome_kind(),
        member_verdicts: shell
            .members
            .iter()
            .map(|member| (member.member_ref, member.verdict))
            .collect(),
        policy_id: shell.policy_id,
        witness_digest: shell.witness_digest,
        posture: shell.posture,
    })
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

/// Scan-backed query over retained braid shells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BraidShellQuery {
    /// Match the shell at this braid coordinate.
    pub coordinate: Option<BraidCoordinate>,
    /// Match shells judged against this comparison basis.
    pub basis: Option<ProvenanceRef>,
    /// Match shells summarizing this member strand.
    pub member_strand: Option<StrandId>,
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
                .member_strand
                .as_ref()
                .is_none_or(|strand| self.has_member_strand(strand))
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
        assert_eq!(replay.posture, CausalPosture::AuthorOnly);
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
            member_strand: Some(make_strand_id("member-a")),
            outcome: Some(AdmissionOutcomeKind::Plural),
            posture: Some(CausalPosture::AuthorOnly),
        }));
        assert!(!shell.matches(&BraidShellQuery {
            outcome: Some(AdmissionOutcomeKind::Conflict),
            ..BraidShellQuery::default()
        }));
        assert!(!shell.matches(&BraidShellQuery {
            member_strand: Some(make_strand_id("nobody")),
            ..BraidShellQuery::default()
        }));
    }

    #[test]
    fn assemble_with_proof_validates_envelope() {
        use crate::proof::{ProofEnvelope, ProofKind};

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
        let expected_witness = temp_shell.witness_digest;

        // Valid proof: matches the witness_digest and has non-empty bytes
        let valid_proof = ProofEnvelope {
            kind: ProofKind::ZkSnark,
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
        shell_with_valid_proof.validate().unwrap();

        // Invalid proof: mismatched public inputs hash
        let invalid_proof_mismatch = ProofEnvelope {
            kind: ProofKind::ZkSnark,
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
            Err(BraidShellError::ProofVerificationFailed { .. })
        ));

        // Invalid proof: empty proof bytes
        let invalid_proof_empty = ProofEnvelope {
            kind: ProofKind::ZkSnark,
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
            Err(BraidShellError::ProofVerificationFailed { .. })
        ));
    }
}
