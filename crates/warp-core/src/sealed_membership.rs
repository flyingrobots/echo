// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Purpose-bound sealed membership presentation vocabulary.

use crate::braid_shell::BraidCoordinate;
use crate::ident::Hash;
use crate::revelation::AuthorityDomainRef;
use crate::witness::WitnessReceipt;

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

/// Purpose-bound sealed membership presentation.
///
/// A presentation proves only a membership claim for a braid coordinate,
/// authority domain, generic purpose, and disclosure budget. It does not reveal
/// global strand identity or source history by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SealedMembershipPresentation {
    /// Braid coordinate the membership claim is scoped to.
    pub braid_coordinate: BraidCoordinate,
    /// Generic capability purpose for the presentation.
    pub purpose: PresentationPurpose,
    /// Authority domain under which the sealed member commitment is meaningful.
    pub authority_domain: AuthorityDomainRef,
    /// Blinded member commitment presented for the purpose.
    pub member_commitment: Hash,
    /// Witness receipt supporting this presentation.
    pub witness_receipt: WitnessReceipt,
    /// Disclosure budget governing the presentation.
    pub disclosure_budget: DisclosureBudget,
}

impl SealedMembershipPresentation {
    /// Creates a purpose-bound sealed membership presentation.
    #[must_use]
    pub const fn new(
        braid_coordinate: BraidCoordinate,
        purpose: PresentationPurpose,
        authority_domain: AuthorityDomainRef,
        member_commitment: Hash,
        witness_receipt: WitnessReceipt,
        disclosure_budget: DisclosureBudget,
    ) -> Self {
        Self {
            braid_coordinate,
            purpose,
            authority_domain,
            member_commitment,
            witness_receipt,
            disclosure_budget,
        }
    }
}
