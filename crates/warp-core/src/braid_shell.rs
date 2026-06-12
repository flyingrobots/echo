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
use crate::revelation::{shell_posture_obstruction, PostureObstruction, RevelationPosture};
use crate::strand::StrandId;
use crate::worldline::WorldlineId;

const SHELL_DOMAIN: &[u8] = b"echo.shell.braid.v1\0";
const MEMBER_DOMAIN: &[u8] = b"echo.braid.member.v1\0";
const WITNESS_DOMAIN: &[u8] = b"echo.braid.witness.v1\0";

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

/// One member entry in a braid shell: compact replay facts, never history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BraidShellMember {
    /// Strand whose claims this member summarizes.
    pub strand_ref: StrandId,
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
    pub posture: RevelationPosture,
}

impl BraidShellMember {
    /// Canonical content digest for this member.
    #[must_use]
    pub fn member_digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(MEMBER_DOMAIN);
        hasher.update(self.strand_ref.as_bytes());
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
            } => {
                hasher.update(&[4]);
                hasher.update(&[*reason_code]);
                hasher.update(witness);
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
    /// Member entries are not in canonical order.
    #[error("braid shell members are not in canonical order")]
    NonCanonicalMemberOrder,
    /// No shell record exists for the requested digest.
    #[error("no braid shell retained for digest {digest:?}")]
    ShellNotFound {
        /// Digest that resolved to nothing.
        digest: Hash,
    },
    /// A collapse lineage parent is missing or not plural.
    #[error("collapse lineage parent {collapsed_from:?} is missing or not plural")]
    InvalidCollapseLineage {
        /// Parent shell digest named by `collapsed_from`.
        collapsed_from: Hash,
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
}

/// Witness digest with a quality bar: zero and empty-input digests refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessDigest(Hash);

impl WitnessDigest {
    /// Wraps a witness digest, refusing shrug values.
    ///
    /// # Errors
    ///
    /// Returns [`BraidShellError::EmptyWitness`] for the all-zero digest and
    /// the digest of empty input.
    pub fn new(hash: Hash) -> Result<Self, BraidShellError> {
        if hash == [0; 32] || hash == crate::blake3_empty() {
            return Err(BraidShellError::EmptyWitness);
        }
        Ok(Self(hash))
    }

    /// Returns the underlying digest.
    #[must_use]
    pub fn as_hash(&self) -> &Hash {
        &self.0
    }
}

/// Retained braid-scale settlement boundary (θ_braid).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BraidShell {
    /// Shell body version.
    pub version: u32,
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
    pub witness_digest: Hash,
    /// Revelation posture of the shell itself.
    pub posture: RevelationPosture,
    /// Canonical content digest of the full shell body.
    pub digest: Hash,
}

impl BraidShell {
    /// Assembles a shell: canonicalizes member order, checks the posture
    /// floor and outcome/member coherence, and seals witness + digest.
    ///
    /// # Errors
    ///
    /// Returns [`BraidShellError`] when the member set is empty, the shell
    /// posture exceeds its least-revealed member, or the outcome arm
    /// disagrees with member verdicts.
    pub fn assemble(
        worldline_id: WorldlineId,
        basis: ProvenanceRef,
        mut members: Vec<BraidShellMember>,
        policy_id: Hash,
        mut outcome: BraidShellOutcome,
        posture: RevelationPosture,
    ) -> Result<Self, BraidShellError> {
        if members.is_empty() {
            return Err(BraidShellError::EmptyMembers);
        }
        members.sort_by_key(BraidShellMember::member_digest);
        if let BraidShellOutcome::Plural { alternative_ids } = &mut outcome {
            // Retained alternatives are a set; canonical order, not transcript
            // order (the member verdict digest binds the ordered transcript).
            alternative_ids.sort_unstable();
        }
        if let Some(obstruction) =
            shell_posture_obstruction(posture, members.iter().map(|member| member.posture))
        {
            return Err(BraidShellError::PostureExceedsMembers(obstruction));
        }
        check_outcome_member_coherence(&outcome, &members)?;

        let witness_digest = compute_witness_digest(
            BRAID_SHELL_VERSION,
            worldline_id,
            &basis,
            &members,
            policy_id,
            &outcome,
            posture,
        );
        let digest = compute_shell_digest(
            BRAID_SHELL_VERSION,
            worldline_id,
            &basis,
            &members,
            policy_id,
            &outcome,
            witness_digest,
            posture,
        );
        Ok(Self {
            version: BRAID_SHELL_VERSION,
            worldline_id,
            basis,
            members,
            policy_id,
            outcome,
            witness_digest,
            posture,
            digest,
        })
    }

    /// Validates the shell as a self-contained retained record.
    ///
    /// # Errors
    ///
    /// Returns [`BraidShellError`] when member order is non-canonical, the
    /// posture floor is violated, outcome and members disagree, or the
    /// stored witness/shell digests do not match the recomputed body.
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
        let mut previous: Option<Hash> = None;
        for member in &self.members {
            let current = member.member_digest();
            if let Some(prior) = previous {
                if prior > current {
                    return Err(BraidShellError::NonCanonicalMemberOrder);
                }
            }
            previous = Some(current);
        }
        if let BraidShellOutcome::Plural { alternative_ids } = &self.outcome {
            if alternative_ids.windows(2).any(|pair| pair[0] > pair[1]) {
                return Err(BraidShellError::NonCanonicalAlternativeOrder);
            }
        }
        if let Some(obstruction) = shell_posture_obstruction(
            self.posture,
            self.members.iter().map(|member| member.posture),
        ) {
            return Err(BraidShellError::PostureExceedsMembers(obstruction));
        }
        check_outcome_member_coherence(&self.outcome, &self.members)?;

        let witness = compute_witness_digest(
            self.version,
            self.worldline_id,
            &self.basis,
            &self.members,
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
        let digest = compute_shell_digest(
            self.version,
            self.worldline_id,
            &self.basis,
            &self.members,
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
        self.members
            .iter()
            .any(|member| member.strand_ref == *strand_id)
    }
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
    members: &[BraidShellMember],
    policy_id: Hash,
    outcome: &BraidShellOutcome,
    posture: RevelationPosture,
) {
    hasher.update(&version.to_le_bytes());
    hasher.update(worldline_id.as_bytes());
    hash_provenance_ref(hasher, basis);
    hasher.update(&(members.len() as u64).to_le_bytes());
    for member in members {
        hasher.update(&member.member_digest());
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
    members: &[BraidShellMember],
    policy_id: Hash,
    outcome: &BraidShellOutcome,
    posture: RevelationPosture,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WITNESS_DOMAIN);
    hash_shell_body(
        &mut hasher,
        version,
        worldline_id,
        basis,
        members,
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
    members: &[BraidShellMember],
    policy_id: Hash,
    outcome: &BraidShellOutcome,
    witness_digest: Hash,
    posture: RevelationPosture,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(SHELL_DOMAIN);
    hash_shell_body(
        &mut hasher,
        version,
        worldline_id,
        basis,
        members,
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
    pub member_verdicts: Vec<(StrandId, MemberVerdict)>,
    /// Settlement policy identity the act ran under.
    pub policy_id: Hash,
    /// Witness digest binding the act.
    pub witness_digest: Hash,
    /// Revelation posture of the shell.
    pub posture: RevelationPosture,
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
    if let BraidShellOutcome::Derived {
        collapsed_from: Some(parent_digest),
        ..
    } = &shell.outcome
    {
        let parent =
            records
                .shell(parent_digest)
                .ok_or(BraidShellError::InvalidCollapseLineage {
                    collapsed_from: *parent_digest,
                })?;
        parent.validate()?;
        if !matches!(parent.outcome, BraidShellOutcome::Plural { .. }) {
            return Err(BraidShellError::InvalidCollapseLineage {
                collapsed_from: *parent_digest,
            });
        }
    }
    Ok(BraidShellReplay {
        outcome_kind: shell.outcome_kind(),
        member_verdicts: shell
            .members
            .iter()
            .map(|member| (member.strand_ref, member.verdict))
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
        return Err(BraidShellError::InvalidCollapseLineage {
            collapsed_from: plural_shell_digest,
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

    fn boundary_digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(TICK_SHELL_DOMAIN);
        hasher.update(self.worldline_id.as_bytes());
        hasher.update(self.u0_ref.as_bytes());
        hasher.update(&self.input_boundary_hash);
        hasher.update(&self.output_boundary_hash);
        hasher.update(&self.logical_counter.to_le_bytes());
        hasher.update(&(self.payload.entries.len() as u64).to_le_bytes());
        for entry in &self.payload.entries {
            hasher.update(&entry.expected.commit_hash);
        }
        hasher.finalize().into()
    }
}

/// Scan-backed query over retained braid shells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BraidShellQuery {
    /// Match shells judged against this comparison basis.
    pub basis: Option<ProvenanceRef>,
    /// Match shells summarizing this member strand.
    pub member_strand: Option<StrandId>,
    /// Match shells with this outcome arm.
    pub outcome: Option<AdmissionOutcomeKind>,
    /// Match shells with this revelation posture.
    pub posture: Option<RevelationPosture>,
}

impl BraidShell {
    /// Returns whether the shell matches every present query field.
    #[must_use]
    pub fn matches(&self, query: &BraidShellQuery) -> bool {
        query.basis.is_none_or(|basis| self.basis == basis)
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
    use crate::ident::make_warp_id;
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
            strand_ref: make_strand_id(label),
            support_pin_digest: [0x21; 32],
            basis_digest: [0x22; 32],
            frontier_digest: [0x23; 32],
            footprint_digest: [0x24; 32],
            claim_digest: [0x25; 32],
            verdict,
            verdict_digest: [0x26; 32],
            posture: RevelationPosture::AuthorOnly,
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
            RevelationPosture::AuthorOnly,
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
        let expected_verdicts: Vec<(StrandId, MemberVerdict)> = shell
            .members
            .iter()
            .map(|member| (member.strand_ref, member.verdict))
            .collect();
        let records = Records::with([shell]);

        let replay = replay_braid_shell(&digest, &records).unwrap();
        assert_eq!(replay.outcome_kind, AdmissionOutcomeKind::Plural);
        assert_eq!(replay.member_verdicts, expected_verdicts);
        assert_eq!(replay.policy_id, [0x5E; 32]);
        assert_eq!(replay.posture, RevelationPosture::AuthorOnly);
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
            Err(BraidShellError::WitnessMismatch { .. } | BraidShellError::DigestMismatch { .. })
        ));

        let mut posture_tampered = shell.clone();
        posture_tampered.posture = RevelationPosture::Scratch;
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
    fn shell_posture_cannot_exceed_least_revealed_member() {
        let result = BraidShell::assemble(
            wl(1),
            basis_ref(),
            vec![member("member-a", MemberVerdict::Plural)],
            [0x5E; 32],
            BraidShellOutcome::Plural {
                alternative_ids: vec![[0x31; 32]],
            },
            RevelationPosture::Shared,
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
            RevelationPosture::AuthorOnly,
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
            RevelationPosture::AuthorOnly,
        )
        .unwrap();
        let derived_digest = derived.digest;

        let complete = Records::with([plural, derived.clone()]);
        let replay = replay_braid_shell(&derived_digest, &complete).unwrap();
        assert_eq!(replay.outcome_kind, AdmissionOutcomeKind::Derived);

        let missing_parent = Records::with([derived]);
        assert_eq!(
            replay_braid_shell(&derived_digest, &missing_parent),
            Err(BraidShellError::InvalidCollapseLineage {
                collapsed_from: plural_digest,
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
            RevelationPosture::AuthorOnly,
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
            RevelationPosture::AuthorOnly,
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

    #[test]
    fn witness_digest_refuses_shrug_values() {
        assert_eq!(
            WitnessDigest::new([0; 32]),
            Err(BraidShellError::EmptyWitness)
        );
        assert_eq!(
            WitnessDigest::new(crate::blake3_empty()),
            Err(BraidShellError::EmptyWitness)
        );
        assert!(WitnessDigest::new([0x99; 32]).is_ok());
    }

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
            basis: Some(basis_ref()),
            member_strand: Some(make_strand_id("member-a")),
            outcome: Some(AdmissionOutcomeKind::Plural),
            posture: Some(RevelationPosture::AuthorOnly),
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
}
