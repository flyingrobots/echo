// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Admission-side nouns for Echo's bounded-site law.
//!
//! A [`BoundedSite`] is the admission-side formalisation of focal closure. It
//! is intentionally derived from existing runtime truth rather than introducing
//! a second geometry system. In the first cut, the site is derived directly
//! from a rewrite's declared [`Footprint`](crate::footprint::Footprint):
//!
//! - `claim_footprint` preserves the full declared read/write claim
//! - `affected_region` captures the local write wake
//! - `reintegration_boundary` preserves the boundary ports implicated by the claim
//!
//! Later revisions may enrich the affected region and reintegration boundary
//! without changing the single-tick law that all admission still happens under
//! `super_tick()`.

use crate::footprint::{AttachmentSet, EdgeSet, Footprint, NodeSet, PortSet};
use crate::ident::Hash;

/// Engine-defined policy identity for one admission act.
///
/// This is intentionally narrow in the first cut: it names the deterministic
/// law selected by the host/runtime without permitting host-authored execution
/// code to redefine admission semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionPolicyRef {
    /// Stable policy identifier committed into downstream shell artifacts.
    pub policy_id: Hash,
}

/// Locally affected region derived from a claim footprint.
///
/// In the first cut, this is the direct write wake of the claim. Future
/// revisions may extend it with richer derived closure without changing the
/// primary `BoundedSite` contract.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AffectedRegion {
    /// Nodes locally affected by the claim.
    pub nodes: NodeSet,
    /// Edges locally affected by the claim.
    pub edges: EdgeSet,
    /// Attachments locally affected by the claim.
    pub attachments: AttachmentSet,
}

impl AffectedRegion {
    /// Derives the first-cut affected region from a claim footprint.
    #[must_use]
    pub fn from_footprint(footprint: &Footprint) -> Self {
        Self {
            nodes: footprint.n_write.clone(),
            edges: footprint.e_write.clone(),
            attachments: footprint.a_write.clone(),
        }
    }
}

/// Boundary surface that later comparison or settlement must respect.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReintegrationBoundary {
    /// Input ports implicated by the claim.
    pub inputs: PortSet,
    /// Output ports implicated by the claim.
    pub outputs: PortSet,
}

impl ReintegrationBoundary {
    /// Derives the first-cut reintegration boundary from a claim footprint.
    #[must_use]
    pub fn from_footprint(footprint: &Footprint) -> Self {
        Self {
            inputs: footprint.b_in.clone(),
            outputs: footprint.b_out.clone(),
        }
    }
}

/// Admission-side focal closure for one local claim site.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BoundedSite {
    /// Full declared read/write claim for the site.
    pub claim_footprint: Footprint,
    /// Local write wake used for first-cut admission reasoning.
    pub affected_region: AffectedRegion,
    /// Boundary surface implicated by the claim.
    pub reintegration_boundary: ReintegrationBoundary,
}

impl BoundedSite {
    /// Builds a bounded site from the current footprint model.
    #[must_use]
    pub fn from_footprint(footprint: &Footprint) -> Self {
        Self {
            claim_footprint: footprint.clone(),
            affected_region: AffectedRegion::from_footprint(footprint),
            reintegration_boundary: ReintegrationBoundary::from_footprint(footprint),
        }
    }
}

impl From<&Footprint> for BoundedSite {
    fn from(value: &Footprint) -> Self {
        Self::from_footprint(value)
    }
}

impl From<Footprint> for BoundedSite {
    fn from(value: Footprint) -> Self {
        Self::from_footprint(&value)
    }
}

/// First-class lawful plurality over one bounded site.
///
/// Echo does not require every subsystem to emit this exact struct today, but
/// the runtime needs one shared noun family for lawful plurality rather than
/// flattening it into ad hoc arrays or generic failure residue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluralArtifact<R, P> {
    /// Site over which plurality remained lawful.
    pub site: BoundedSite,
    /// Participants whose claims coexist at that site.
    pub participants: Vec<P>,
    /// Claims that remained irreducibly plural.
    pub claims: Vec<R>,
}

impl<R, P> PluralArtifact<R, P> {
    /// Constructs a plural artifact with parallel participant and claim lists.
    #[must_use]
    pub fn new(site: BoundedSite, participants: Vec<P>, claims: Vec<R>) -> Self {
        assert_eq!(
            participants.len(),
            claims.len(),
            "plural participants must stay parallel to claims"
        );
        Self {
            site,
            participants,
            claims,
        }
    }
}

/// Shared lawful outcome family for admission acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionOutcomeKind {
    /// A single derived result was admitted.
    Derived,
    /// Multiple claims remained lawfully plural over one bounded site.
    Plural,
    /// The act produced explicit conflict residue.
    Conflict,
    /// The act was obstructed before a derived or plural result could be admitted.
    Obstruction,
}

/// Shared lawful outcome family for admission acts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionOutcome<R, P, C, O> {
    /// A single derived result was admitted.
    Derived(R),
    /// Multiple claims remained lawfully plural over the bounded site.
    Plural(Box<PluralArtifact<R, P>>),
    /// The act produced explicit conflict residue.
    Conflict(C),
    /// The act was obstructed before a derived or plural result could be admitted.
    Obstruction(O),
}

impl<R, P, C, O> AdmissionOutcome<R, P, C, O> {
    /// Returns the top-level lawful outcome kind.
    #[must_use]
    pub fn kind(&self) -> AdmissionOutcomeKind {
        match self {
            Self::Derived(_) => AdmissionOutcomeKind::Derived,
            Self::Plural(_) => AdmissionOutcomeKind::Plural,
            Self::Conflict(_) => AdmissionOutcomeKind::Conflict,
            Self::Obstruction(_) => AdmissionOutcomeKind::Obstruction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::AttachmentKey;
    use crate::footprint::pack_port_key;
    use crate::ident::{make_edge_id, make_node_id, make_warp_id, EdgeKey, NodeKey};

    #[test]
    fn bounded_site_derives_from_footprint_without_inventing_new_geometry() {
        let warp_id = make_warp_id("bounded-site-warp");
        let read_node = make_node_id("read-node");
        let write_node = make_node_id("write-node");
        let read_edge = make_edge_id("read-edge");
        let write_edge = make_edge_id("write-edge");

        let mut footprint = Footprint::default();
        footprint.n_read.insert_with_warp(warp_id, read_node);
        footprint.n_write.insert_with_warp(warp_id, write_node);
        footprint.e_read.insert_with_warp(warp_id, read_edge);
        footprint.e_write.insert_with_warp(warp_id, write_edge);
        footprint.a_read.insert(AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: read_node,
        }));
        footprint.a_write.insert(AttachmentKey::edge_beta(EdgeKey {
            warp_id,
            local_id: write_edge,
        }));
        footprint
            .b_in
            .insert(warp_id, pack_port_key(&read_node, 0, true));
        footprint
            .b_out
            .insert(warp_id, pack_port_key(&write_node, 0, false));
        footprint.factor_mask = 0b1010;

        let site = BoundedSite::from_footprint(&footprint);

        assert_eq!(site.claim_footprint, footprint);
        assert_eq!(site.affected_region.nodes, footprint.n_write);
        assert_eq!(site.affected_region.edges, footprint.e_write);
        assert_eq!(site.affected_region.attachments, footprint.a_write);
        assert_eq!(site.reintegration_boundary.inputs, footprint.b_in);
        assert_eq!(site.reintegration_boundary.outputs, footprint.b_out);
    }

    #[test]
    fn plural_artifact_requires_parallel_participants_and_claims() {
        let site = BoundedSite::default();
        let artifact = PluralArtifact::new(site, vec!["lane-a", "lane-b"], vec![1, 2]);

        assert_eq!(artifact.participants, vec!["lane-a", "lane-b"]);
        assert_eq!(artifact.claims, vec![1, 2]);
    }

    #[test]
    fn admission_outcome_reports_top_level_kind() {
        let derived: AdmissionOutcome<u8, &str, &str, &str> = AdmissionOutcome::Derived(7);
        let plural: AdmissionOutcome<u8, &str, &str, &str> =
            AdmissionOutcome::Plural(Box::new(PluralArtifact::new(
                BoundedSite::default(),
                vec!["lane-a", "lane-b"],
                vec![1u8, 2u8],
            )));
        let conflict: AdmissionOutcome<u8, &str, &str, &str> =
            AdmissionOutcome::Conflict("conflict");
        let obstruction: AdmissionOutcome<u8, &str, &str, &str> =
            AdmissionOutcome::Obstruction("blocked");

        assert_eq!(derived.kind(), AdmissionOutcomeKind::Derived);
        assert_eq!(plural.kind(), AdmissionOutcomeKind::Plural);
        assert_eq!(conflict.kind(), AdmissionOutcomeKind::Conflict);
        assert_eq!(obstruction.kind(), AdmissionOutcomeKind::Obstruction);
    }
}
