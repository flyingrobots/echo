// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native local-site publication for Echo neighborhood inspection.
//!
//! A [`NeighborhoodSite`] is the kernel-backed publication object for one
//! observed local site. It is intentionally narrower than a full global braid
//! model: it says which lanes participate in the current observed site, not
//! every nearby alternative in the universe.
//!
//! This is a derived publication surface, not the authoritative admission-side
//! site noun. Admission in Echo is judged over a
//! [`BoundedSite`](crate::admission::BoundedSite); neighborhood publication is
//! a later observer-facing projection over nearby lane truth.

use blake3::Hasher;
use echo_wasm_abi::kernel_port as abi;
use thiserror::Error;

use crate::admission::AdmissionOutcomeKind;
use crate::clock::{GlobalTick, WorldlineTick};
use crate::coordinator::WorldlineRuntime;
use crate::engine_impl::Engine;
use crate::ident::Hash;
use crate::observation::{
    ObservationAt, ObservationError, ObservationRequest, ObservationService,
    ResolvedObservationCoordinate,
};
use crate::provenance_store::{ProvenanceService, ProvenanceStore};
use crate::strand::StrandId;
use crate::worldline::WorldlineId;

const NEIGHBORHOOD_SITE_DOMAIN: &[u8] = b"echo:neighborhood-site:v1\0";

/// Stable identity for one published local site.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NeighborhoodSiteId([u8; 32]);

impl NeighborhoodSiteId {
    /// Reconstructs the id from canonical bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the canonical byte representation.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn to_abi(self) -> abi::NeighborhoodSiteId {
        abi::NeighborhoodSiteId::from_bytes(self.0)
    }
}

/// Whether the published site is singleton or plural.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SitePlurality {
    /// Only the primary lane participates.
    Singleton,
    /// The site includes additional base/support participants.
    Braided,
}

impl SitePlurality {
    /// Maps the published site plurality into Echo's shared lawful outcome family.
    #[must_use]
    pub fn admission_outcome_kind(self) -> AdmissionOutcomeKind {
        match self {
            Self::Singleton => AdmissionOutcomeKind::Derived,
            Self::Braided => AdmissionOutcomeKind::Plural,
        }
    }

    fn to_abi(self) -> abi::SitePlurality {
        match self {
            Self::Singleton => abi::SitePlurality::Singleton,
            Self::Braided => abi::SitePlurality::Braided,
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Singleton => 1,
            Self::Braided => 2,
        }
    }
}

/// Role a participant plays in a published local site.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParticipantRole {
    /// The lane being directly observed.
    Primary,
    /// The base coordinate from which the primary strand forked.
    BasisAnchor,
    /// A read-only support-pinned lane.
    Support,
}

impl ParticipantRole {
    fn to_abi(self) -> abi::ParticipantRole {
        match self {
            Self::Primary => abi::ParticipantRole::Primary,
            Self::BasisAnchor => abi::ParticipantRole::BaseAnchor,
            Self::Support => abi::ParticipantRole::Support,
        }
    }

    fn to_core(self) -> NeighborhoodParticipantRole {
        match self {
            Self::Primary => NeighborhoodParticipantRole::Primary,
            Self::BasisAnchor => NeighborhoodParticipantRole::BasisAnchor,
            Self::Support => NeighborhoodParticipantRole::Support,
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Primary => 1,
            Self::BasisAnchor => 2,
            Self::Support => 3,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::BasisAnchor => "basis-anchor",
            Self::Support => "support",
        }
    }
}

/// One lane participating in a published local site.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SiteParticipant {
    /// The participant's worldline.
    pub worldline_id: WorldlineId,
    /// The participant's strand identity, when this participant is a strand.
    pub strand_id: Option<StrandId>,
    /// Participant role within the site.
    pub role: ParticipantRole,
    /// Exact participant tick.
    pub tick: WorldlineTick,
    /// Canonical state hash for the participant at that tick.
    pub state_hash: Hash,
}

impl SiteParticipant {
    fn to_abi(&self) -> abi::SiteParticipant {
        abi::SiteParticipant {
            worldline_id: abi::WorldlineId::from_bytes(*self.worldline_id.as_bytes()),
            strand_id: self
                .strand_id
                .map(|strand_id| abi::StrandId::from_bytes(*strand_id.as_bytes())),
            role: self.role.to_abi(),
            tick: abi::WorldlineTick(self.tick.as_u64()),
            state_hash: self.state_hash.to_vec(),
        }
    }
}

/// Shared observer/debugger plurality for one published neighborhood core.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NeighborhoodPlurality {
    /// Only the primary lane participates.
    Singleton,
    /// Multiple lanes participate in the published local site.
    Plural,
}

impl SitePlurality {
    fn to_core(self) -> NeighborhoodPlurality {
        match self {
            Self::Singleton => NeighborhoodPlurality::Singleton,
            Self::Braided => NeighborhoodPlurality::Plural,
        }
    }
}

impl NeighborhoodPlurality {
    fn to_abi(self) -> abi::NeighborhoodPlurality {
        match self {
            Self::Singleton => abi::NeighborhoodPlurality::Singleton,
            Self::Plural => abi::NeighborhoodPlurality::Plural,
        }
    }
}

/// Shared observer/debugger role for one published neighborhood participant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NeighborhoodParticipantRole {
    /// The directly observed lane.
    Primary,
    /// The fork/source lane anchoring the primary strand.
    BasisAnchor,
    /// A read-only support lane.
    Support,
}

impl NeighborhoodParticipantRole {
    fn to_abi(self) -> abi::NeighborhoodParticipantRole {
        match self {
            Self::Primary => abi::NeighborhoodParticipantRole::Primary,
            Self::BasisAnchor => abi::NeighborhoodParticipantRole::BasisAnchor,
            Self::Support => abi::NeighborhoodParticipantRole::Support,
        }
    }
}

/// Shared observer/debugger participant for one published neighborhood core.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NeighborhoodParticipant {
    /// Stable participant identity within the published site.
    pub participant_id: String,
    /// Stable lane identity for the participant's worldline carrier.
    pub lane_id: String,
    /// Optional strand identity when the participant is a strand-backed lane.
    pub strand_id: Option<String>,
    /// Participant role in the published site.
    pub role: NeighborhoodParticipantRole,
    /// Exact participant frame index.
    pub frame_index: u64,
    /// Canonical state hash for the participant at that frame.
    pub state_hash: String,
}

impl SiteParticipant {
    fn to_core(&self) -> NeighborhoodParticipant {
        NeighborhoodParticipant {
            participant_id: participant_id(self),
            lane_id: worldline_lane_id(self.worldline_id),
            strand_id: self.strand_id.map(strand_id_label),
            role: self.role.to_core(),
            frame_index: self.tick.as_u64(),
            state_hash: hash_label(self.state_hash),
        }
    }
}

impl NeighborhoodParticipant {
    fn to_abi(&self) -> abi::NeighborhoodParticipant {
        abi::NeighborhoodParticipant {
            participant_id: self.participant_id.clone(),
            lane_id: self.lane_id.clone(),
            strand_id: self.strand_id.clone(),
            role: self.role.to_abi(),
            frame_index: self.frame_index,
            state_hash: self.state_hash.clone(),
        }
    }
}

/// Shared observer/debugger projection for one published local site.
///
/// This is the first Echo-side grounding for the shared Continuum
/// `NeighborhoodCore` family shape. It is intentionally narrow and does not
/// carry reintegration detail or receipt shell enrichment.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NeighborhoodCore {
    /// Stable published site identity.
    pub site_id: String,
    /// Anchor lane identity derived from the observed worldline.
    pub anchor_lane_id: String,
    /// Exact resolved anchor frame index.
    pub anchor_frame_index: u64,
    /// Optional anchor head identity. Echo does not yet publish this natively.
    pub anchor_head_id: Option<String>,
    /// Top-level lawful outcome kind for this published site.
    pub outcome_kind: AdmissionOutcomeKind,
    /// Shared singleton-vs-plural truth.
    pub plurality: NeighborhoodPlurality,
    /// Participating lanes for the site.
    pub participants: Vec<NeighborhoodParticipant>,
    /// Narrow human-readable summary for debugger surfaces.
    pub summary: String,
}

impl AdmissionOutcomeKind {
    fn to_core_abi(self) -> abi::AdmissionOutcomeKind {
        match self {
            Self::Derived => abi::AdmissionOutcomeKind::Derived,
            Self::Plural => abi::AdmissionOutcomeKind::Plural,
            Self::Conflict => abi::AdmissionOutcomeKind::Conflict,
            Self::Obstruction => abi::AdmissionOutcomeKind::Obstruction,
        }
    }
}

impl NeighborhoodCore {
    /// Converts the neighborhood core into its shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::NeighborhoodCore {
        abi::NeighborhoodCore {
            site_id: self.site_id.clone(),
            anchor_lane_id: self.anchor_lane_id.clone(),
            anchor_frame_index: self.anchor_frame_index,
            anchor_head_id: self.anchor_head_id.clone(),
            outcome_kind: self.outcome_kind.to_core_abi(),
            plurality: self.plurality.to_abi(),
            participants: self
                .participants
                .iter()
                .map(NeighborhoodParticipant::to_abi)
                .collect(),
            summary: self.summary.clone(),
        }
    }
}

/// Kernel-backed publication object for one observed local site.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NeighborhoodSite {
    /// Stable site identity.
    pub site_id: NeighborhoodSiteId,
    /// Anchor coordinate being published.
    pub anchor: ResolvedObservationCoordinate,
    /// Singleton or plural site truth.
    pub plurality: SitePlurality,
    /// Participating lanes for this site.
    pub participants: Vec<SiteParticipant>,
}

impl NeighborhoodSite {
    /// Returns the top-level lawful outcome kind for the published local site.
    ///
    /// Neighborhood publication remains a derived observer-facing surface, but
    /// it should still use the same shared outcome algebra as the rest of
    /// Echo's admission/publication stack.
    #[must_use]
    pub fn admission_outcome_kind(&self) -> AdmissionOutcomeKind {
        self.plurality.admission_outcome_kind()
    }

    /// Converts the site into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::NeighborhoodSite {
        abi::NeighborhoodSite {
            site_id: self.site_id.to_abi(),
            anchor: self.anchor.to_abi(),
            plurality: self.plurality.to_abi(),
            participants: self
                .participants
                .iter()
                .map(SiteParticipant::to_abi)
                .collect(),
        }
    }

    /// Projects the kernel-local site into the shared neighborhood-core family
    /// shape consumed by observer/debugger surfaces.
    #[must_use]
    pub fn to_core(&self) -> NeighborhoodCore {
        NeighborhoodCore {
            site_id: site_id_label(self.site_id),
            anchor_lane_id: worldline_lane_id(self.anchor.worldline_id),
            anchor_frame_index: self.anchor.resolved_worldline_tick.as_u64(),
            anchor_head_id: None,
            outcome_kind: self.admission_outcome_kind(),
            plurality: self.plurality.to_core(),
            participants: self
                .participants
                .iter()
                .map(SiteParticipant::to_core)
                .collect(),
            summary: neighborhood_summary(self),
        }
    }
}

/// Errors that can occur while publishing a local neighborhood site.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum NeighborhoodError {
    /// Wrapped observation failure while resolving the primary coordinate.
    #[error(transparent)]
    Observation(#[from] ObservationError),

    /// A declared support pin no longer resolves to recorded provenance.
    #[error(
        "support pin on strand {owner:?} could not resolve support strand {support:?} at tick {tick}"
    )]
    InvalidSupportPin {
        /// Strand that owns the invalid support pin.
        owner: StrandId,
        /// Support strand referenced by the pin.
        support: StrandId,
        /// Pinned tick that could not be resolved.
        tick: WorldlineTick,
    },

    /// A declared support pin refers to a missing live strand.
    #[error("support pin target strand not found: owner {owner:?}, support {support:?}")]
    MissingSupportStrand {
        /// Strand that owns the invalid support pin.
        owner: StrandId,
        /// Missing support strand id.
        support: StrandId,
    },
}

/// Read-only publication service for local neighborhood sites.
pub struct NeighborhoodSiteService;

impl NeighborhoodSiteService {
    /// Observe and publish the local site for the given observation request.
    ///
    /// # Errors
    ///
    /// Returns [`NeighborhoodError`] if coordinate resolution fails or if a
    /// declared support pin no longer resolves to live/provenance truth.
    pub fn observe(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        engine: &Engine,
        request: ObservationRequest,
    ) -> Result<NeighborhoodSite, NeighborhoodError> {
        let artifact = ObservationService::observe(runtime, provenance, engine, request)?;
        Self::from_resolved(runtime, provenance, &artifact.resolved)
    }

    /// Build a neighborhood site from an already-resolved observation anchor.
    ///
    /// # Errors
    ///
    /// Returns [`NeighborhoodError`] if a declared support pin no longer
    /// resolves to live/provenance truth.
    pub fn from_resolved(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        resolved: &ResolvedObservationCoordinate,
    ) -> Result<NeighborhoodSite, NeighborhoodError> {
        let primary_strand = runtime
            .strands()
            .find_by_child_worldline(&resolved.worldline_id);

        let mut participants = Vec::new();
        participants.push(SiteParticipant {
            worldline_id: resolved.worldline_id,
            strand_id: primary_strand.map(|strand| strand.strand_id),
            role: ParticipantRole::Primary,
            tick: resolved.resolved_worldline_tick,
            state_hash: resolved.state_root,
        });

        if let Some(strand) = primary_strand {
            participants.push(SiteParticipant {
                worldline_id: strand.fork_basis_ref.source_lane_id,
                strand_id: None,
                role: ParticipantRole::BasisAnchor,
                tick: strand.fork_basis_ref.fork_tick,
                state_hash: strand.fork_basis_ref.boundary_hash,
            });

            for support_pin in &strand.support_pins {
                let Some(_support_strand) = runtime.strands().get(&support_pin.strand_id) else {
                    return Err(NeighborhoodError::MissingSupportStrand {
                        owner: strand.strand_id,
                        support: support_pin.strand_id,
                    });
                };

                let entry = provenance
                    .entry(support_pin.worldline_id, support_pin.pinned_tick)
                    .map_err(|_| NeighborhoodError::InvalidSupportPin {
                        owner: strand.strand_id,
                        support: support_pin.strand_id,
                        tick: support_pin.pinned_tick,
                    })?;
                if entry.expected.state_root != support_pin.state_hash {
                    return Err(NeighborhoodError::InvalidSupportPin {
                        owner: strand.strand_id,
                        support: support_pin.strand_id,
                        tick: support_pin.pinned_tick,
                    });
                }

                participants.push(SiteParticipant {
                    worldline_id: support_pin.worldline_id,
                    strand_id: Some(support_pin.strand_id),
                    role: ParticipantRole::Support,
                    tick: support_pin.pinned_tick,
                    state_hash: support_pin.state_hash,
                });
            }
        }

        let plurality = if participants.len() == 1 {
            SitePlurality::Singleton
        } else {
            SitePlurality::Braided
        };
        let site_id = compute_site_id(resolved, plurality, &participants);
        Ok(NeighborhoodSite {
            site_id,
            anchor: resolved.clone(),
            plurality,
            participants,
        })
    }
}

fn compute_site_id(
    resolved: &ResolvedObservationCoordinate,
    plurality: SitePlurality,
    participants: &[SiteParticipant],
) -> NeighborhoodSiteId {
    let mut hasher = Hasher::new();
    hasher.update(NEIGHBORHOOD_SITE_DOMAIN);
    hasher.update(&resolved.observation_version.to_le_bytes());
    hasher.update(resolved.worldline_id.as_bytes());
    match resolved.requested_at {
        ObservationAt::Frontier => {
            hasher.update(&[0]);
        }
        ObservationAt::Tick(tick) => {
            hasher.update(&[1]);
            hasher.update(&tick.as_u64().to_le_bytes());
        }
    }
    hasher.update(&resolved.resolved_worldline_tick.as_u64().to_le_bytes());
    write_optional_global_tick(&mut hasher, resolved.commit_global_tick);
    write_optional_global_tick(&mut hasher, resolved.observed_after_global_tick);
    hasher.update(&resolved.state_root);
    hasher.update(&resolved.commit_hash);
    hasher.update(&[plurality.code()]);
    hasher.update(&(participants.len() as u64).to_le_bytes());
    for participant in participants {
        hasher.update(participant.worldline_id.as_bytes());
        match participant.strand_id {
            Some(strand_id) => {
                hasher.update(&[1]);
                hasher.update(strand_id.as_bytes());
            }
            None => {
                hasher.update(&[0]);
            }
        }
        hasher.update(&[participant.role.code()]);
        hasher.update(&participant.tick.as_u64().to_le_bytes());
        hasher.update(&participant.state_hash);
    }
    NeighborhoodSiteId(hasher.finalize().into())
}

fn site_id_label(site_id: NeighborhoodSiteId) -> String {
    format!("site:{}", hex::encode(site_id.as_bytes()))
}

fn worldline_lane_id(worldline_id: WorldlineId) -> String {
    format!("wl:{}", hex::encode(worldline_id.as_bytes()))
}

fn strand_id_label(strand_id: StrandId) -> String {
    format!("strand:{}", hex::encode(strand_id.as_bytes()))
}

fn hash_label(hash: Hash) -> String {
    hex::encode(hash)
}

fn participant_id(participant: &SiteParticipant) -> String {
    let carrier = participant.strand_id.map_or_else(
        || worldline_lane_id(participant.worldline_id),
        strand_id_label,
    );
    format!(
        "participant:{}:{}:{}",
        carrier,
        participant.role.label(),
        participant.tick.as_u64()
    )
}

fn neighborhood_summary(site: &NeighborhoodSite) -> String {
    match site.plurality {
        SitePlurality::Singleton => format!(
            "Echo published a singleton local site with {} participant at {}@{}.",
            site.participants.len(),
            worldline_lane_id(site.anchor.worldline_id),
            site.anchor.resolved_worldline_tick.as_u64()
        ),
        SitePlurality::Braided => format!(
            "Echo published a plural local site with {} participants at {}@{}.",
            site.participants.len(),
            worldline_lane_id(site.anchor.worldline_id),
            site.anchor.resolved_worldline_tick.as_u64()
        ),
    }
}

fn write_optional_global_tick(hasher: &mut Hasher, tick: Option<GlobalTick>) {
    match tick {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.as_u64().to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::head::{make_head_id, WriterHead, WriterHeadKey};
    use crate::head_inbox::InboxPolicy;
    use crate::ident::{make_node_id, make_type_id, make_warp_id};
    use crate::provenance_store::{ProvenanceEntry, ProvenanceRef};
    use crate::receipt::TickReceipt;
    use crate::record::NodeRecord;
    use crate::snapshot::Snapshot;
    use crate::strand::{make_strand_id, ForkBasisRef, Strand};
    use crate::tick_patch::{TickCommitStatus, WarpTickPatchV1};
    use crate::worldline::{HashTriplet, WorldlineTickHeaderV1, WorldlineTickPatchV1};
    use crate::{
        blake3_empty, EngineBuilder, GraphStore, PlaybackMode, ProvenanceService, WorldlineState,
    };

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    fn wt(raw: u64) -> WorldlineTick {
        WorldlineTick::from_raw(raw)
    }

    fn gt(raw: u64) -> GlobalTick {
        GlobalTick::from_raw(raw)
    }

    fn committed_state(
        worldline_id: WorldlineId,
        global_tick: GlobalTick,
        label: &str,
    ) -> (WorldlineState, Snapshot, ProvenanceEntry) {
        let warp_id = make_warp_id(&format!("warp-{label}"));
        let root = make_node_id(&format!("root-{label}"));
        let mut store = GraphStore::new(warp_id);
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );
        let engine = EngineBuilder::new(store, root).workers(1).build();
        let mut state = WorldlineState::try_from(engine.state().clone()).unwrap();
        let snapshot = engine.snapshot_for_state(&state);
        state.tick_history.push((
            snapshot.clone(),
            TickReceipt::new(snapshot.tx, Vec::new(), Vec::new()),
            WarpTickPatchV1::new(
                crate::POLICY_ID_NO_POLICY_V0,
                blake3_empty(),
                TickCommitStatus::Committed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ));
        state.last_snapshot = Some(snapshot.clone());

        let entry = ProvenanceEntry::local_commit(
            worldline_id,
            wt(0),
            global_tick,
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id(&format!("head-{label}")),
            },
            Vec::new(),
            HashTriplet {
                state_root: snapshot.state_root,
                patch_digest: snapshot.patch_digest,
                commit_hash: snapshot.hash,
            },
            WorldlineTickPatchV1 {
                header: WorldlineTickHeaderV1 {
                    commit_global_tick: global_tick,
                    policy_id: crate::POLICY_ID_NO_POLICY_V0,
                    rule_pack_id: blake3_empty(),
                    plan_digest: snapshot.plan_digest,
                    decision_digest: snapshot.decision_digest,
                    rewrites_digest: snapshot.rewrites_digest,
                },
                warp_id,
                ops: Vec::new(),
                in_slots: Vec::new(),
                out_slots: Vec::new(),
                patch_digest: snapshot.patch_digest,
            },
            Vec::new(),
            Vec::new(),
        );
        (state, snapshot, entry)
    }

    #[test]
    fn singleton_site_contains_only_primary_participant() {
        let worldline_id = wl(1);
        let (state, snapshot, entry) = committed_state(worldline_id, gt(1), "singleton");

        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(worldline_id, state.clone())
            .unwrap();
        runtime
            .register_writer_head(WriterHead::with_routing(
                WriterHeadKey {
                    worldline_id,
                    head_id: make_head_id("default"),
                },
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .unwrap();
        let mut provenance = ProvenanceService::new();
        provenance.register_worldline(worldline_id, &state).unwrap();
        provenance.append_local_commit(entry).unwrap();

        let site = NeighborhoodSiteService::from_resolved(
            &runtime,
            &provenance,
            &ResolvedObservationCoordinate {
                observation_version: 2,
                worldline_id,
                requested_at: ObservationAt::Tick(wt(0)),
                resolved_worldline_tick: wt(0),
                commit_global_tick: Some(gt(1)),
                observed_after_global_tick: Some(gt(1)),
                state_root: snapshot.state_root,
                commit_hash: snapshot.hash,
            },
        )
        .unwrap();

        assert_eq!(site.plurality, SitePlurality::Singleton);
        assert_eq!(site.admission_outcome_kind(), AdmissionOutcomeKind::Derived);
        assert_eq!(site.participants.len(), 1);
        assert_eq!(site.participants[0].role, ParticipantRole::Primary);
        assert_eq!(site.participants[0].worldline_id, worldline_id);
        assert_eq!(site.participants[0].state_hash, snapshot.state_root);

        let core = site.to_core();
        assert_eq!(
            core.site_id,
            format!("site:{}", hex::encode(site.site_id.as_bytes()))
        );
        assert_eq!(
            core.anchor_lane_id,
            format!("wl:{}", hex::encode(worldline_id.as_bytes()))
        );
        assert_eq!(core.anchor_frame_index, 0);
        assert_eq!(core.anchor_head_id, None);
        assert_eq!(core.outcome_kind, AdmissionOutcomeKind::Derived);
        assert_eq!(core.plurality, NeighborhoodPlurality::Singleton);
        assert_eq!(core.participants.len(), 1);
        assert_eq!(
            core.participants[0].lane_id,
            format!("wl:{}", hex::encode(worldline_id.as_bytes()))
        );
        assert_eq!(core.participants[0].strand_id, None);
        assert_eq!(
            core.participants[0].role,
            NeighborhoodParticipantRole::Primary
        );
        assert_eq!(core.participants[0].frame_index, 0);
        assert_eq!(
            core.participants[0].state_hash,
            hex::encode(snapshot.state_root)
        );
        assert!(core.summary.contains("singleton local site"));
    }

    #[test]
    fn braided_site_publishes_primary_base_and_support_participants() {
        let base_worldline = wl(1);
        let primary_worldline = wl(2);
        let support_worldline = wl(3);

        let (base_state, base_snapshot, base_entry) =
            committed_state(base_worldline, gt(1), "base");
        let (primary_state, primary_snapshot, primary_entry) =
            committed_state(primary_worldline, gt(2), "primary");
        let (support_state, support_snapshot, support_entry) =
            committed_state(support_worldline, gt(3), "support");

        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state.clone())
            .unwrap();
        runtime
            .register_worldline(primary_worldline, primary_state.clone())
            .unwrap();
        runtime
            .register_worldline(support_worldline, support_state.clone())
            .unwrap();

        let primary_head = WriterHeadKey {
            worldline_id: primary_worldline,
            head_id: make_head_id("primary-head"),
        };
        let support_head = WriterHeadKey {
            worldline_id: support_worldline,
            head_id: make_head_id("support-head"),
        };
        runtime
            .register_writer_head(WriterHead::with_routing(
                primary_head,
                PlaybackMode::Paused,
                InboxPolicy::AcceptAll,
                None,
                false,
            ))
            .unwrap();
        runtime
            .register_writer_head(WriterHead::with_routing(
                support_head,
                PlaybackMode::Paused,
                InboxPolicy::AcceptAll,
                None,
                false,
            ))
            .unwrap();

        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(base_worldline, &base_state)
            .unwrap();
        provenance
            .register_worldline(primary_worldline, &primary_state)
            .unwrap();
        provenance
            .register_worldline(support_worldline, &support_state)
            .unwrap();
        provenance.append_local_commit(base_entry).unwrap();
        provenance.append_local_commit(primary_entry).unwrap();
        provenance.append_local_commit(support_entry).unwrap();

        let support_strand_id = make_strand_id("support");
        runtime
            .register_strand(Strand {
                strand_id: support_strand_id,
                fork_basis_ref: ForkBasisRef {
                    source_lane_id: base_worldline,
                    fork_tick: wt(0),
                    commit_hash: base_snapshot.hash,
                    boundary_hash: base_snapshot.state_root,
                    provenance_ref: ProvenanceRef {
                        worldline_id: base_worldline,
                        worldline_tick: wt(0),
                        commit_hash: base_snapshot.hash,
                    },
                },
                child_worldline_id: support_worldline,
                writer_heads: vec![support_head],
                support_pins: Vec::new(),
            })
            .unwrap();

        let primary_strand_id = make_strand_id("primary");
        runtime
            .register_strand(Strand {
                strand_id: primary_strand_id,
                fork_basis_ref: ForkBasisRef {
                    source_lane_id: base_worldline,
                    fork_tick: wt(0),
                    commit_hash: base_snapshot.hash,
                    boundary_hash: base_snapshot.state_root,
                    provenance_ref: ProvenanceRef {
                        worldline_id: base_worldline,
                        worldline_tick: wt(0),
                        commit_hash: base_snapshot.hash,
                    },
                },
                child_worldline_id: primary_worldline,
                writer_heads: vec![primary_head],
                support_pins: Vec::new(),
            })
            .unwrap();
        runtime
            .pin_support(&provenance, primary_strand_id, support_strand_id, wt(0))
            .unwrap();

        let site = NeighborhoodSiteService::from_resolved(
            &runtime,
            &provenance,
            &ResolvedObservationCoordinate {
                observation_version: 2,
                worldline_id: primary_worldline,
                requested_at: ObservationAt::Tick(wt(0)),
                resolved_worldline_tick: wt(0),
                commit_global_tick: Some(gt(2)),
                observed_after_global_tick: Some(gt(3)),
                state_root: primary_snapshot.state_root,
                commit_hash: primary_snapshot.hash,
            },
        )
        .unwrap();

        assert_eq!(site.plurality, SitePlurality::Braided);
        assert_eq!(site.admission_outcome_kind(), AdmissionOutcomeKind::Plural);
        assert_eq!(site.participants.len(), 3);
        assert_eq!(site.participants[0].role, ParticipantRole::Primary);
        assert_eq!(site.participants[0].strand_id, Some(primary_strand_id));
        assert_eq!(site.participants[1].role, ParticipantRole::BasisAnchor);
        assert_eq!(site.participants[1].worldline_id, base_worldline);
        assert_eq!(site.participants[1].state_hash, base_snapshot.state_root);
        assert_eq!(site.participants[2].role, ParticipantRole::Support);
        assert_eq!(site.participants[2].worldline_id, support_worldline);
        assert_eq!(site.participants[2].strand_id, Some(support_strand_id));
        assert_eq!(site.participants[2].state_hash, support_snapshot.state_root);

        let core = site.to_core();
        assert_eq!(
            core.anchor_lane_id,
            format!("wl:{}", hex::encode(primary_worldline.as_bytes()))
        );
        assert_eq!(core.anchor_frame_index, 0);
        assert_eq!(core.outcome_kind, AdmissionOutcomeKind::Plural);
        assert_eq!(core.plurality, NeighborhoodPlurality::Plural);
        assert_eq!(core.participants.len(), 3);
        assert_eq!(
            core.participants[0].role,
            NeighborhoodParticipantRole::Primary
        );
        assert_eq!(
            core.participants[0].lane_id,
            format!("wl:{}", hex::encode(primary_worldline.as_bytes()))
        );
        assert_eq!(
            core.participants[0].strand_id,
            Some(format!(
                "strand:{}",
                hex::encode(primary_strand_id.as_bytes())
            ))
        );
        assert_eq!(
            core.participants[1].role,
            NeighborhoodParticipantRole::BasisAnchor
        );
        assert_eq!(
            core.participants[1].lane_id,
            format!("wl:{}", hex::encode(base_worldline.as_bytes()))
        );
        assert_eq!(core.participants[1].strand_id, None);
        assert_eq!(
            core.participants[2].role,
            NeighborhoodParticipantRole::Support
        );
        assert_eq!(
            core.participants[2].lane_id,
            format!("wl:{}", hex::encode(support_worldline.as_bytes()))
        );
        assert_eq!(
            core.participants[2].strand_id,
            Some(format!(
                "strand:{}",
                hex::encode(support_strand_id.as_bytes())
            ))
        );
        assert!(core.summary.contains("plural local site"));
    }

    #[test]
    fn invalid_support_pin_is_rejected_during_publication() {
        let base_worldline = wl(1);
        let primary_worldline = wl(2);
        let support_worldline = wl(3);
        let (base_state, base_snapshot, base_entry) = committed_state(base_worldline, gt(1), "b");
        let (primary_state, primary_snapshot, primary_entry) =
            committed_state(primary_worldline, gt(2), "p");
        let (support_state, _support_snapshot, support_entry) =
            committed_state(support_worldline, gt(3), "s");

        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state.clone())
            .unwrap();
        runtime
            .register_worldline(primary_worldline, primary_state.clone())
            .unwrap();
        runtime
            .register_worldline(support_worldline, support_state.clone())
            .unwrap();
        let primary_head = WriterHeadKey {
            worldline_id: primary_worldline,
            head_id: make_head_id("primary-head"),
        };
        let support_head = WriterHeadKey {
            worldline_id: support_worldline,
            head_id: make_head_id("support-head"),
        };
        runtime
            .register_writer_head(WriterHead::with_routing(
                primary_head,
                PlaybackMode::Paused,
                InboxPolicy::AcceptAll,
                None,
                false,
            ))
            .unwrap();
        runtime
            .register_writer_head(WriterHead::with_routing(
                support_head,
                PlaybackMode::Paused,
                InboxPolicy::AcceptAll,
                None,
                false,
            ))
            .unwrap();

        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(base_worldline, &base_state)
            .unwrap();
        provenance
            .register_worldline(primary_worldline, &primary_state)
            .unwrap();
        provenance
            .register_worldline(support_worldline, &support_state)
            .unwrap();
        provenance.append_local_commit(base_entry).unwrap();
        provenance.append_local_commit(primary_entry).unwrap();
        provenance.append_local_commit(support_entry).unwrap();

        let support_strand_id = make_strand_id("support");
        runtime
            .register_strand(Strand {
                strand_id: support_strand_id,
                fork_basis_ref: ForkBasisRef {
                    source_lane_id: base_worldline,
                    fork_tick: wt(0),
                    commit_hash: base_snapshot.hash,
                    boundary_hash: base_snapshot.state_root,
                    provenance_ref: ProvenanceRef {
                        worldline_id: base_worldline,
                        worldline_tick: wt(0),
                        commit_hash: base_snapshot.hash,
                    },
                },
                child_worldline_id: support_worldline,
                writer_heads: vec![support_head],
                support_pins: Vec::new(),
            })
            .unwrap();
        let primary_strand_id = make_strand_id("primary");
        runtime
            .register_strand(Strand {
                strand_id: primary_strand_id,
                fork_basis_ref: ForkBasisRef {
                    source_lane_id: base_worldline,
                    fork_tick: wt(0),
                    commit_hash: base_snapshot.hash,
                    boundary_hash: base_snapshot.state_root,
                    provenance_ref: ProvenanceRef {
                        worldline_id: base_worldline,
                        worldline_tick: wt(0),
                        commit_hash: base_snapshot.hash,
                    },
                },
                child_worldline_id: primary_worldline,
                writer_heads: vec![primary_head],
                support_pins: Vec::new(),
            })
            .unwrap();
        runtime
            .pin_support(&provenance, primary_strand_id, support_strand_id, wt(0))
            .unwrap();
        runtime
            .strands_mut_for_tests()
            .get_mut(&primary_strand_id)
            .unwrap()
            .support_pins[0]
            .state_hash = [0xFF; 32];

        let result = NeighborhoodSiteService::from_resolved(
            &runtime,
            &provenance,
            &ResolvedObservationCoordinate {
                observation_version: 2,
                worldline_id: primary_worldline,
                requested_at: ObservationAt::Tick(wt(0)),
                resolved_worldline_tick: wt(0),
                commit_global_tick: Some(gt(2)),
                observed_after_global_tick: Some(gt(3)),
                state_root: primary_snapshot.state_root,
                commit_hash: primary_snapshot.hash,
            },
        );
        assert!(
            matches!(
                result,
                Err(NeighborhoodError::InvalidSupportPin {
                    owner,
                    support,
                    tick,
                }) if owner == primary_strand_id
                    && support == support_strand_id
                    && tick == wt(0)
            ),
            "stale support pin should fail"
        );
    }
}
