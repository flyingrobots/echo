// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Purpose-bound sealed membership presentation vocabulary.

use blake3::Hasher;
use thiserror::Error;

use crate::braid_shell::BraidCoordinate;
use crate::ident::Hash;
use crate::revelation::AuthorityDomainRef;
use crate::witness::WitnessReceipt;

const PRESENTATION_SUBJECT_DOMAIN: &[u8] = b"echo.sealed-membership.presentation.subject.v1\0";
const PRESENTATION_EVIDENCE_DOMAIN: &[u8] = b"echo.sealed-membership.presentation.evidence.v1\0";

/// Disclosure budget attached to replay and sealed membership facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DisclosureBudget {
    /// Publicly revealed material.
    Public,
    /// Material scoped to an authority domain.
    AuthorityScoped,
    /// Material scoped to a capability presentation.
    CapabilityScoped,
    /// Material visible only to the holder.
    HolderOnly,
    /// Zero-knowledge disclosure budget.
    ZeroKnowledge,
}

/// Generic capability purpose for sealed membership presentations.
///
/// This type is deliberately generic. Application-domain purpose names belong
/// in adapters or authored contracts, not in Echo core.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PresentationPurpose {
    purpose_id: Hash,
}

impl PresentationPurpose {
    /// Creates a generic presentation purpose from a caller-defined digest.
    #[must_use]
    pub const fn new(purpose_id: Hash) -> Self {
        Self { purpose_id }
    }

    /// Returns the purpose digest.
    #[must_use]
    pub const fn purpose_id(self) -> Hash {
        self.purpose_id
    }
}

/// Error raised when a sealed membership presentation is not bound to its witness.
#[derive(Error, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SealedMembershipPresentationError {
    /// The witness receipt subject does not match the presentation claim.
    #[error("sealed membership witness subject mismatch")]
    WitnessSubjectMismatch {
        /// Expected subject digest for the presentation fields.
        expected: Hash,
        /// Actual subject digest carried by the witness receipt.
        actual: Hash,
    },
    /// The witness receipt evidence does not match the presentation claim.
    #[error("sealed membership witness evidence mismatch")]
    WitnessEvidenceMismatch {
        /// Expected evidence digest for the presentation fields.
        expected: Hash,
        /// Actual evidence digest carried by the witness receipt.
        actual: Hash,
    },
}

/// Purpose-bound sealed membership presentation.
///
/// A presentation proves only a membership claim for a braid coordinate,
/// authority domain, generic purpose, and disclosure budget. It does not reveal
/// global strand identity or source history by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SealedMembershipPresentation {
    /// Braid coordinate the membership claim is scoped to.
    braid_coordinate: BraidCoordinate,
    /// Generic capability purpose for the presentation.
    purpose: PresentationPurpose,
    /// Authority domain under which the sealed member commitment is meaningful.
    authority_domain: AuthorityDomainRef,
    /// Blinded member commitment presented for the purpose.
    member_commitment: Hash,
    /// Witness receipt supporting this presentation.
    witness_receipt: WitnessReceipt,
    /// Disclosure budget governing the presentation.
    disclosure_budget: DisclosureBudget,
}

impl SealedMembershipPresentation {
    /// Creates a purpose-bound sealed membership presentation.
    ///
    /// # Errors
    ///
    /// Returns [`SealedMembershipPresentationError`] when the witness receipt
    /// subject or evidence digest does not match the presentation fields.
    pub fn new(
        braid_coordinate: BraidCoordinate,
        purpose: PresentationPurpose,
        authority_domain: AuthorityDomainRef,
        member_commitment: Hash,
        witness_receipt: WitnessReceipt,
        disclosure_budget: DisclosureBudget,
    ) -> Result<Self, SealedMembershipPresentationError> {
        let expected_subject = Self::witness_subject_digest(
            braid_coordinate,
            purpose,
            authority_domain,
            member_commitment,
            disclosure_budget,
        );
        if witness_receipt.subject_digest() != expected_subject {
            return Err(SealedMembershipPresentationError::WitnessSubjectMismatch {
                expected: expected_subject,
                actual: witness_receipt.subject_digest(),
            });
        }
        let expected_evidence = Self::witness_evidence_digest(
            braid_coordinate,
            purpose,
            authority_domain,
            member_commitment,
            disclosure_budget,
        );
        if witness_receipt.evidence_digest() != expected_evidence {
            return Err(SealedMembershipPresentationError::WitnessEvidenceMismatch {
                expected: expected_evidence,
                actual: witness_receipt.evidence_digest(),
            });
        }
        Ok(Self {
            braid_coordinate,
            purpose,
            authority_domain,
            member_commitment,
            witness_receipt,
            disclosure_budget,
        })
    }

    /// Returns the witness subject digest for these presentation fields.
    #[must_use]
    pub fn witness_subject_digest(
        braid_coordinate: BraidCoordinate,
        purpose: PresentationPurpose,
        authority_domain: AuthorityDomainRef,
        member_commitment: Hash,
        disclosure_budget: DisclosureBudget,
    ) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(PRESENTATION_SUBJECT_DOMAIN);
        hash_presentation_fields(
            &mut hasher,
            braid_coordinate,
            purpose,
            authority_domain,
            member_commitment,
            disclosure_budget,
        );
        hasher.finalize().into()
    }

    /// Returns the witness evidence digest for these presentation fields.
    #[must_use]
    pub fn witness_evidence_digest(
        braid_coordinate: BraidCoordinate,
        purpose: PresentationPurpose,
        authority_domain: AuthorityDomainRef,
        member_commitment: Hash,
        disclosure_budget: DisclosureBudget,
    ) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(PRESENTATION_EVIDENCE_DOMAIN);
        hash_presentation_fields(
            &mut hasher,
            braid_coordinate,
            purpose,
            authority_domain,
            member_commitment,
            disclosure_budget,
        );
        hasher.finalize().into()
    }

    /// Returns the braid coordinate the membership claim is scoped to.
    #[must_use]
    pub const fn braid_coordinate(self) -> BraidCoordinate {
        self.braid_coordinate
    }

    /// Returns the generic capability purpose for the presentation.
    #[must_use]
    pub const fn purpose(self) -> PresentationPurpose {
        self.purpose
    }

    /// Returns the authority domain under which the commitment is meaningful.
    #[must_use]
    pub const fn authority_domain(self) -> AuthorityDomainRef {
        self.authority_domain
    }

    /// Returns the blinded member commitment presented for the purpose.
    #[must_use]
    pub const fn member_commitment(self) -> Hash {
        self.member_commitment
    }

    /// Returns the witness receipt supporting this presentation.
    #[must_use]
    pub const fn witness_receipt(self) -> WitnessReceipt {
        self.witness_receipt
    }

    /// Returns the disclosure budget governing the presentation.
    #[must_use]
    pub const fn disclosure_budget(self) -> DisclosureBudget {
        self.disclosure_budget
    }
}

fn hash_presentation_fields(
    hasher: &mut Hasher,
    braid_coordinate: BraidCoordinate,
    purpose: PresentationPurpose,
    authority_domain: AuthorityDomainRef,
    member_commitment: Hash,
    disclosure_budget: DisclosureBudget,
) {
    hasher.update(&braid_coordinate.0);
    hasher.update(&purpose.purpose_id());
    hasher.update(authority_domain.origin_id.as_bytes());
    hasher.update(authority_domain.domain_id.as_bytes());
    hasher.update(&member_commitment);
    hasher.update(&[disclosure_budget_tag(disclosure_budget)]);
}

const fn disclosure_budget_tag(budget: DisclosureBudget) -> u8 {
    match budget {
        DisclosureBudget::Public => 0x01,
        DisclosureBudget::AuthorityScoped => 0x02,
        DisclosureBudget::CapabilityScoped => 0x03,
        DisclosureBudget::HolderOnly => 0x04,
        DisclosureBudget::ZeroKnowledge => 0x05,
    }
}
