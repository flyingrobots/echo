// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Explicit observation contract for worldline reads.
//!
//! Phase 5 makes observation the single canonical internal read path. Every
//! meaningful read names:
//!
//! - a worldline,
//! - a coordinate,
//! - a semantic frame,
//! - and a projection.
//!
//! Observation is strictly read-only. It never advances runtime state, drains
//! inboxes, rewrites provenance, or mutates compatibility mirrors.

use blake3::Hasher;
use echo_wasm_abi::kernel_port as abi;
use thiserror::Error;

use crate::clock::{GlobalTick, WorldlineTick};
use crate::coordinator::WorldlineRuntime;
use crate::engine_impl::Engine;
use crate::ident::Hash;
use crate::materialization::ChannelId;
use crate::provenance_store::{ProvenanceService, ProvenanceStore};
use crate::snapshot::Snapshot;
use crate::worldline::WorldlineId;

const OBSERVATION_VERSION: u32 = 2;
const OBSERVATION_ARTIFACT_DOMAIN: &[u8] = b"echo:observation-artifact:v2\0";

/// Coordinate selector for an observation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationCoordinate {
    /// Worldline to observe.
    pub worldline_id: WorldlineId,
    /// Requested coordinate within the worldline.
    pub at: ObservationAt,
}

/// Requested position within a worldline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationAt {
    /// Observe the current worldline frontier.
    Frontier,
    /// Observe a specific committed historical tick.
    Tick(WorldlineTick),
}

impl ObservationAt {
    fn to_abi(self) -> abi::ObservationAt {
        match self {
            Self::Frontier => abi::ObservationAt::Frontier,
            Self::Tick(worldline_tick) => abi::ObservationAt::Tick {
                worldline_tick: abi::WorldlineTick(worldline_tick.as_u64()),
            },
        }
    }
}

/// Semantic frame declared by an observation request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationFrame {
    /// Read commit-boundary state metadata.
    CommitBoundary,
    /// Read recorded truth from provenance outputs.
    RecordedTruth,
    /// Read query-shaped projections.
    QueryView,
}

impl ObservationFrame {
    fn to_abi(self) -> abi::ObservationFrame {
        match self {
            Self::CommitBoundary => abi::ObservationFrame::CommitBoundary,
            Self::RecordedTruth => abi::ObservationFrame::RecordedTruth,
            Self::QueryView => abi::ObservationFrame::QueryView,
        }
    }
}

/// Coarse projection kind used by the validity matrix and deterministic errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationProjectionKind {
    /// Head metadata projection.
    Head,
    /// Snapshot metadata projection.
    Snapshot,
    /// Recorded truth channels projection.
    TruthChannels,
    /// Query payload projection.
    Query,
}

impl ObservationProjectionKind {
    /// Converts a validated internal projection into the ABI form.
    ///
    /// This helper is only valid when `self` and `projection` are matching
    /// variants. Reaching the fallback arm is a programmer error in the
    /// observation service, not a recoverable runtime condition.
    fn to_abi(self, projection: &ObservationProjection) -> abi::ObservationProjection {
        match (self, projection) {
            (Self::Head, ObservationProjection::Head) => abi::ObservationProjection::Head,
            (Self::Snapshot, ObservationProjection::Snapshot) => {
                abi::ObservationProjection::Snapshot
            }
            (Self::TruthChannels, ObservationProjection::TruthChannels { channels }) => {
                abi::ObservationProjection::TruthChannels {
                    channels: channels.as_ref().map(|ids| {
                        ids.iter()
                            .map(|channel| channel.0.to_vec())
                            .collect::<Vec<_>>()
                    }),
                }
            }
            (
                Self::Query,
                ObservationProjection::Query {
                    query_id,
                    vars_bytes,
                },
            ) => abi::ObservationProjection::Query {
                query_id: *query_id,
                vars_bytes: vars_bytes.clone(),
            },
            _ => {
                debug_assert!(
                    false,
                    "ObservationProjectionKind::to_abi requires matching kind/projection variants"
                );
                unreachable!(
                    "ObservationProjectionKind::to_abi requires matching kind/projection variants"
                )
            }
        }
    }
}

/// Requested projection within a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationProjection {
    /// Head metadata projection.
    Head,
    /// Snapshot metadata projection.
    Snapshot,
    /// Recorded truth channels projection.
    TruthChannels {
        /// Optional channel filter. `None` means all recorded channels.
        channels: Option<Vec<ChannelId>>,
    },
    /// Query payload placeholder.
    Query {
        /// Stable query identifier.
        query_id: u32,
        /// Canonical vars payload bytes.
        vars_bytes: Vec<u8>,
    },
}

impl ObservationProjection {
    /// Returns the coarse projection kind used for validation and error reporting.
    #[must_use]
    pub fn kind(&self) -> ObservationProjectionKind {
        match self {
            Self::Head => ObservationProjectionKind::Head,
            Self::Snapshot => ObservationProjectionKind::Snapshot,
            Self::TruthChannels { .. } => ObservationProjectionKind::TruthChannels,
            Self::Query { .. } => ObservationProjectionKind::Query,
        }
    }

    fn to_abi(&self) -> abi::ObservationProjection {
        self.kind().to_abi(self)
    }
}

/// Canonical observation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationRequest {
    /// Worldline coordinate being observed.
    pub coordinate: ObservationCoordinate,
    /// Declared semantic frame.
    pub frame: ObservationFrame,
    /// Requested projection within that frame.
    pub projection: ObservationProjection,
}

/// Fully resolved coordinate returned with every observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedObservationCoordinate {
    /// Observation contract version.
    pub observation_version: u32,
    /// Worldline that was actually observed.
    pub worldline_id: WorldlineId,
    /// Original coordinate selector from the request.
    pub requested_at: ObservationAt,
    /// Concrete resolved tick.
    pub resolved_worldline_tick: WorldlineTick,
    /// Commit-cycle stamp for the resolved commit, if any.
    pub commit_global_tick: Option<GlobalTick>,
    /// Observation freshness watermark after resolving this artifact.
    pub observed_after_global_tick: Option<GlobalTick>,
    /// Canonical state root at the resolved coordinate.
    pub state_root: Hash,
    /// Canonical commit hash at the resolved coordinate.
    pub commit_hash: Hash,
}

impl ResolvedObservationCoordinate {
    fn to_abi(&self) -> abi::ResolvedObservationCoordinate {
        abi::ResolvedObservationCoordinate {
            observation_version: self.observation_version,
            worldline_id: abi::WorldlineId::from_bytes(self.worldline_id.0),
            requested_at: self.requested_at.to_abi(),
            resolved_worldline_tick: abi::WorldlineTick(self.resolved_worldline_tick.as_u64()),
            commit_global_tick: self
                .commit_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            observed_after_global_tick: self
                .observed_after_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            state_root: self.state_root.to_vec(),
            commit_hash: self.commit_hash.to_vec(),
        }
    }
}

/// Minimal frontier/head observation payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadObservation {
    /// Observed tick.
    pub worldline_tick: WorldlineTick,
    /// Commit-cycle stamp for the observed commit, if any.
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical state root at that tick.
    pub state_root: Hash,
    /// Canonical commit hash at that tick.
    pub commit_hash: Hash,
}

impl HeadObservation {
    fn to_abi(&self) -> abi::HeadObservation {
        abi::HeadObservation {
            worldline_tick: abi::WorldlineTick(self.worldline_tick.as_u64()),
            commit_global_tick: self
                .commit_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            state_root: self.state_root.to_vec(),
            commit_id: self.commit_hash.to_vec(),
        }
    }
}

/// Minimal historical snapshot observation payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldlineSnapshot {
    /// Observed historical tick.
    pub worldline_tick: WorldlineTick,
    /// Commit-cycle stamp for the observed commit, if any.
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical state root at that tick.
    pub state_root: Hash,
    /// Canonical commit hash at that tick.
    pub commit_hash: Hash,
}

impl WorldlineSnapshot {
    fn to_abi(&self) -> abi::SnapshotObservation {
        abi::SnapshotObservation {
            worldline_tick: abi::WorldlineTick(self.worldline_tick.as_u64()),
            commit_global_tick: self
                .commit_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            state_root: self.state_root.to_vec(),
            commit_id: self.commit_hash.to_vec(),
        }
    }
}

/// Observation payload variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationPayload {
    /// Head metadata.
    Head(HeadObservation),
    /// Historical snapshot metadata.
    Snapshot(WorldlineSnapshot),
    /// Recorded truth payloads in channel-id order.
    TruthChannels(Vec<(ChannelId, Vec<u8>)>),
    /// Query result bytes.
    QueryBytes(Vec<u8>),
}

impl ObservationPayload {
    fn to_abi(&self) -> abi::ObservationPayload {
        match self {
            Self::Head(head) => abi::ObservationPayload::Head {
                head: head.to_abi(),
            },
            Self::Snapshot(snapshot) => abi::ObservationPayload::Snapshot {
                snapshot: snapshot.to_abi(),
            },
            Self::TruthChannels(channels) => abi::ObservationPayload::TruthChannels {
                channels: channels
                    .iter()
                    .map(|(channel, data)| abi::ChannelData {
                        channel_id: channel.0.to_vec(),
                        data: data.clone(),
                    })
                    .collect(),
            },
            Self::QueryBytes(data) => abi::ObservationPayload::QueryBytes { data: data.clone() },
        }
    }
}

/// Full observation artifact with deterministic identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationArtifact {
    /// Resolved coordinate metadata.
    pub resolved: ResolvedObservationCoordinate,
    /// Declared semantic frame.
    pub frame: ObservationFrame,
    /// Declared projection.
    pub projection: ObservationProjection,
    /// Deterministic artifact hash.
    pub artifact_hash: Hash,
    /// Observation payload.
    pub payload: ObservationPayload,
}

impl ObservationArtifact {
    /// Converts this artifact into the shared ABI DTO shape.
    #[must_use]
    pub fn to_abi(&self) -> abi::ObservationArtifact {
        abi::ObservationArtifact {
            resolved: self.resolved.to_abi(),
            frame: self.frame.to_abi(),
            projection: self.projection.to_abi(),
            artifact_hash: self.artifact_hash.to_vec(),
            payload: self.payload.to_abi(),
        }
    }
}

/// Deterministic observation failures.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ObservationError {
    /// The requested worldline is not registered.
    #[error("invalid worldline: {0:?}")]
    InvalidWorldline(WorldlineId),
    /// The requested historical tick is not available.
    #[error("invalid tick {tick} for worldline {worldline_id:?}")]
    InvalidTick {
        /// Worldline that was targeted.
        worldline_id: WorldlineId,
        /// Requested tick.
        tick: WorldlineTick,
    },
    /// The frame/projection pairing is not valid in v1.
    #[error("unsupported frame/projection pairing: {frame:?} + {projection:?}")]
    UnsupportedFrameProjection {
        /// Declared frame.
        frame: ObservationFrame,
        /// Requested projection kind.
        projection: ObservationProjectionKind,
    },
    /// Query observation is not implemented yet.
    #[error("query observation is not supported in phase 5")]
    UnsupportedQuery,
    /// The requested observation cannot be produced at this coordinate.
    #[error("observation unavailable for worldline {worldline_id:?} at {at:?}")]
    ObservationUnavailable {
        /// Worldline that was targeted.
        worldline_id: WorldlineId,
        /// Requested coordinate.
        at: ObservationAt,
    },
    /// Canonical artifact encoding failed.
    #[error("observation artifact encoding failed: {0}")]
    CodecFailure(String),
}

/// Immutable observation service.
pub struct ObservationService;

impl ObservationService {
    /// Observe a worldline at an explicit coordinate and frame.
    ///
    /// The runtime, provenance store, and engine are borrowed immutably. This
    /// method never mutates live frontier state or recorded history.
    ///
    /// # Errors
    ///
    /// Returns [`ObservationError`] for invalid worldlines/ticks, unsupported
    /// frame/projection pairings, unsupported query requests, or unavailable
    /// recorded truth.
    pub fn observe(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        engine: &Engine,
        request: ObservationRequest,
    ) -> Result<ObservationArtifact, ObservationError> {
        let worldline_id = request.coordinate.worldline_id;
        if runtime.worldlines().get(&worldline_id).is_none() {
            return Err(ObservationError::InvalidWorldline(worldline_id));
        }
        Self::validate_frame_projection(request.frame, &request.projection)?;
        if matches!(request.frame, ObservationFrame::QueryView) {
            return Err(ObservationError::UnsupportedQuery);
        }

        let resolved = Self::resolve_coordinate(runtime, provenance, engine, &request)?;
        let payload = match (&request.frame, &request.projection) {
            (ObservationFrame::CommitBoundary, ObservationProjection::Head) => {
                ObservationPayload::Head(HeadObservation {
                    worldline_tick: resolved.resolved_worldline_tick,
                    commit_global_tick: resolved.commit_global_tick,
                    state_root: resolved.state_root,
                    commit_hash: resolved.commit_hash,
                })
            }
            (ObservationFrame::CommitBoundary, ObservationProjection::Snapshot) => {
                ObservationPayload::Snapshot(WorldlineSnapshot {
                    worldline_tick: resolved.resolved_worldline_tick,
                    commit_global_tick: resolved.commit_global_tick,
                    state_root: resolved.state_root,
                    commit_hash: resolved.commit_hash,
                })
            }
            (
                ObservationFrame::RecordedTruth,
                ObservationProjection::TruthChannels { channels },
            ) => {
                let entry = provenance
                    .entry(worldline_id, resolved.resolved_worldline_tick)
                    .map_err(|_| ObservationError::ObservationUnavailable {
                        worldline_id,
                        at: request.coordinate.at,
                    })?;
                let outputs = match channels {
                    Some(filter) => entry
                        .outputs
                        .into_iter()
                        .filter(|(channel, _)| filter.contains(channel))
                        .collect(),
                    None => entry.outputs,
                };
                ObservationPayload::TruthChannels(outputs)
            }
            (ObservationFrame::QueryView, ObservationProjection::Query { .. }) => {
                return Err(ObservationError::UnsupportedQuery);
            }
            _ => unreachable!("validity matrix must reject unsupported combinations"),
        };

        let artifact_hash =
            Self::compute_artifact_hash(&resolved, request.frame, &request.projection, &payload)?;
        Ok(ObservationArtifact {
            resolved,
            frame: request.frame,
            projection: request.projection,
            artifact_hash,
            payload,
        })
    }

    fn validate_frame_projection(
        frame: ObservationFrame,
        projection: &ObservationProjection,
    ) -> Result<(), ObservationError> {
        let projection_kind = projection.kind();
        let valid = matches!(
            (frame, projection_kind),
            (
                ObservationFrame::CommitBoundary,
                ObservationProjectionKind::Head | ObservationProjectionKind::Snapshot
            ) | (
                ObservationFrame::RecordedTruth,
                ObservationProjectionKind::TruthChannels
            ) | (
                ObservationFrame::QueryView,
                ObservationProjectionKind::Query
            )
        );
        if valid {
            Ok(())
        } else {
            Err(ObservationError::UnsupportedFrameProjection {
                frame,
                projection: projection_kind,
            })
        }
    }

    fn resolve_coordinate(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        engine: &Engine,
        request: &ObservationRequest,
    ) -> Result<ResolvedObservationCoordinate, ObservationError> {
        let worldline_id = request.coordinate.worldline_id;
        let frontier = runtime
            .worldlines()
            .get(&worldline_id)
            .ok_or(ObservationError::InvalidWorldline(worldline_id))?;

        match (request.frame, request.coordinate.at) {
            (ObservationFrame::CommitBoundary, ObservationAt::Frontier) => {
                let snapshot = frontier
                    .state()
                    .last_snapshot()
                    .cloned()
                    .unwrap_or_else(|| engine.snapshot_for_state(frontier.state()));
                let commit_global_tick = frontier
                    .frontier_tick()
                    .checked_sub(1)
                    .map(|committed_tick| {
                        provenance
                            .entry(worldline_id, committed_tick)
                            .map(|entry| entry.commit_global_tick)
                            .map_err(|_| ObservationError::ObservationUnavailable {
                                worldline_id,
                                at: request.coordinate.at,
                            })
                    })
                    .transpose()?;
                Ok(Self::resolved_commit_boundary(
                    worldline_id,
                    request.coordinate.at,
                    frontier.frontier_tick(),
                    commit_global_tick,
                    runtime.global_tick(),
                    &snapshot,
                ))
            }
            (ObservationFrame::CommitBoundary, ObservationAt::Tick(tick)) => {
                let entry = provenance
                    .entry(worldline_id, tick)
                    .map_err(|_| ObservationError::InvalidTick { worldline_id, tick })?;
                Ok(ResolvedObservationCoordinate {
                    observation_version: OBSERVATION_VERSION,
                    worldline_id,
                    requested_at: request.coordinate.at,
                    resolved_worldline_tick: tick,
                    commit_global_tick: Some(entry.commit_global_tick),
                    observed_after_global_tick: current_cycle_tick(runtime),
                    state_root: entry.expected.state_root,
                    commit_hash: entry.expected.commit_hash,
                })
            }
            (ObservationFrame::RecordedTruth, ObservationAt::Frontier) => {
                let Some(resolved_worldline_tick) = frontier.frontier_tick().checked_sub(1) else {
                    return Err(ObservationError::ObservationUnavailable {
                        worldline_id,
                        at: request.coordinate.at,
                    });
                };
                let entry = provenance
                    .entry(worldline_id, resolved_worldline_tick)
                    .map_err(|_| ObservationError::ObservationUnavailable {
                        worldline_id,
                        at: request.coordinate.at,
                    })?;
                Ok(ResolvedObservationCoordinate {
                    observation_version: OBSERVATION_VERSION,
                    worldline_id,
                    requested_at: request.coordinate.at,
                    resolved_worldline_tick,
                    commit_global_tick: Some(entry.commit_global_tick),
                    observed_after_global_tick: current_cycle_tick(runtime),
                    state_root: entry.expected.state_root,
                    commit_hash: entry.expected.commit_hash,
                })
            }
            (ObservationFrame::RecordedTruth, ObservationAt::Tick(tick)) => {
                let entry = provenance
                    .entry(worldline_id, tick)
                    .map_err(|_| ObservationError::InvalidTick { worldline_id, tick })?;
                Ok(ResolvedObservationCoordinate {
                    observation_version: OBSERVATION_VERSION,
                    worldline_id,
                    requested_at: request.coordinate.at,
                    resolved_worldline_tick: tick,
                    commit_global_tick: Some(entry.commit_global_tick),
                    observed_after_global_tick: current_cycle_tick(runtime),
                    state_root: entry.expected.state_root,
                    commit_hash: entry.expected.commit_hash,
                })
            }
            (ObservationFrame::QueryView, _) => Err(ObservationError::UnsupportedQuery),
        }
    }

    fn resolved_commit_boundary(
        worldline_id: WorldlineId,
        requested_at: ObservationAt,
        resolved_worldline_tick: WorldlineTick,
        commit_global_tick: Option<GlobalTick>,
        latest_cycle_global_tick: GlobalTick,
        snapshot: &Snapshot,
    ) -> ResolvedObservationCoordinate {
        ResolvedObservationCoordinate {
            observation_version: OBSERVATION_VERSION,
            worldline_id,
            requested_at,
            resolved_worldline_tick,
            commit_global_tick,
            observed_after_global_tick: option_cycle_tick(latest_cycle_global_tick),
            state_root: snapshot.state_root,
            commit_hash: snapshot.hash,
        }
    }

    fn compute_artifact_hash(
        resolved: &ResolvedObservationCoordinate,
        frame: ObservationFrame,
        projection: &ObservationProjection,
        payload: &ObservationPayload,
    ) -> Result<Hash, ObservationError> {
        let input = abi::ObservationHashInput {
            resolved: resolved.to_abi(),
            frame: frame.to_abi(),
            projection: projection.to_abi(),
            payload: payload.to_abi(),
        };
        let bytes = echo_wasm_abi::encode_cbor(&input)
            .map_err(|err| ObservationError::CodecFailure(err.to_string()))?;
        let mut hasher = Hasher::new();
        hasher.update(OBSERVATION_ARTIFACT_DOMAIN);
        hasher.update(&bytes);
        Ok(hasher.finalize().into())
    }
}

fn option_cycle_tick(tick: GlobalTick) -> Option<GlobalTick> {
    (tick != GlobalTick::ZERO).then_some(tick)
}

fn current_cycle_tick(runtime: &WorldlineRuntime) -> Option<GlobalTick> {
    option_cycle_tick(runtime.global_tick())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::coordinator::WorldlineRuntime;
    use crate::head::{make_head_id, WriterHead, WriterHeadKey};
    use crate::head_inbox::{make_intent_kind, InboxPolicy, IngressEnvelope, IngressTarget};
    use crate::ident::{make_node_id, make_type_id, WarpId};
    use crate::materialization::make_channel_id;
    use crate::receipt::TickReceipt;
    use crate::record::NodeRecord;
    use crate::tick_patch::{TickCommitStatus, WarpTickPatchV1};
    use crate::worldline::{HashTriplet, WorldlineTickHeaderV1, WorldlineTickPatchV1};
    use crate::{
        EngineBuilder, GraphStore, PlaybackMode, ProvenanceEntry, SchedulerCoordinator,
        WorldlineState,
    };

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
    }

    fn wt(raw: u64) -> WorldlineTick {
        WorldlineTick::from_raw(raw)
    }

    fn gt(raw: u64) -> GlobalTick {
        GlobalTick::from_raw(raw)
    }

    fn empty_runtime_fixture() -> (Engine, WorldlineRuntime, ProvenanceService, WorldlineId) {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let engine = EngineBuilder::new(store, root).workers(1).build();
        let default_worldline = WorldlineId(engine.root_key().warp_id.0);
        let mut runtime = WorldlineRuntime::new();
        let default_state = WorldlineState::try_from(engine.state().clone()).unwrap();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(default_worldline, &default_state)
            .unwrap();
        runtime
            .register_worldline(default_worldline, default_state)
            .unwrap();
        runtime
            .register_writer_head(WriterHead::with_routing(
                WriterHeadKey {
                    worldline_id: default_worldline,
                    head_id: make_head_id("default"),
                },
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .unwrap();
        (engine, runtime, provenance, default_worldline)
    }

    fn one_commit_fixture() -> (Engine, WorldlineRuntime, ProvenanceService, WorldlineId) {
        let (mut engine, mut runtime, mut provenance, worldline_id) = empty_runtime_fixture();
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("echo.intent/test"),
                b"hello".to_vec(),
            ))
            .unwrap();
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        (engine, runtime, provenance, worldline_id)
    }

    fn recorded_truth_fixture() -> (Engine, WorldlineRuntime, ProvenanceService, WorldlineId) {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );
        let engine = EngineBuilder::new(store, root).workers(1).build();
        let worldline_id = wl(7);
        let mut state = WorldlineState::empty();
        let snapshot = engine.snapshot_for_state(&state);
        state.tick_history.push((
            snapshot.clone(),
            TickReceipt::new(snapshot.tx, Vec::new(), Vec::new()),
            WarpTickPatchV1::new(
                crate::POLICY_ID_NO_POLICY_V0,
                crate::blake3_empty(),
                TickCommitStatus::Committed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ));
        state.last_snapshot = Some(snapshot.clone());
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
        let channel = make_channel_id("test:truth");
        provenance
            .append_local_commit(ProvenanceEntry::local_commit(
                worldline_id,
                wt(0),
                gt(1),
                WriterHeadKey {
                    worldline_id,
                    head_id: make_head_id("default"),
                },
                Vec::new(),
                HashTriplet {
                    state_root: snapshot.state_root,
                    patch_digest: snapshot.patch_digest,
                    commit_hash: snapshot.hash,
                },
                WorldlineTickPatchV1 {
                    header: WorldlineTickHeaderV1 {
                        commit_global_tick: gt(1),
                        policy_id: crate::POLICY_ID_NO_POLICY_V0,
                        rule_pack_id: crate::blake3_empty(),
                        plan_digest: snapshot.plan_digest,
                        decision_digest: snapshot.decision_digest,
                        rewrites_digest: snapshot.rewrites_digest,
                    },
                    warp_id: WarpId(root.0),
                    ops: Vec::new(),
                    in_slots: Vec::new(),
                    out_slots: Vec::new(),
                    patch_digest: snapshot.patch_digest,
                },
                vec![(channel, b"truth".to_vec())],
                Vec::new(),
            ))
            .unwrap();
        (engine, runtime, provenance, worldline_id)
    }

    #[test]
    fn validity_matrix_accepts_only_centralized_pairs() {
        let truth = ObservationProjection::TruthChannels { channels: None };
        let query = ObservationProjection::Query {
            query_id: 7,
            vars_bytes: Vec::new(),
        };

        assert!(ObservationService::validate_frame_projection(
            ObservationFrame::CommitBoundary,
            &ObservationProjection::Head,
        )
        .is_ok());
        assert!(ObservationService::validate_frame_projection(
            ObservationFrame::CommitBoundary,
            &ObservationProjection::Snapshot,
        )
        .is_ok());
        assert!(ObservationService::validate_frame_projection(
            ObservationFrame::RecordedTruth,
            &truth,
        )
        .is_ok());
        assert!(
            ObservationService::validate_frame_projection(ObservationFrame::QueryView, &query,)
                .is_ok()
        );

        assert!(matches!(
            ObservationService::validate_frame_projection(
                ObservationFrame::RecordedTruth,
                &ObservationProjection::Head,
            ),
            Err(ObservationError::UnsupportedFrameProjection { .. })
        ));
        assert!(matches!(
            ObservationService::validate_frame_projection(ObservationFrame::CommitBoundary, &truth,),
            Err(ObservationError::UnsupportedFrameProjection { .. })
        ));
    }

    #[test]
    fn frontier_head_matches_live_frontier_snapshot() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest {
                coordinate: ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Frontier,
                },
                frame: ObservationFrame::CommitBoundary,
                projection: ObservationProjection::Head,
            },
        )
        .unwrap();

        let frontier = runtime.worldlines().get(&worldline_id).unwrap();
        let snapshot = frontier
            .state()
            .last_snapshot()
            .cloned()
            .unwrap_or_else(|| engine.snapshot_for_state(frontier.state()));
        assert_eq!(
            artifact.resolved.resolved_worldline_tick,
            frontier.frontier_tick()
        );
        assert_eq!(artifact.resolved.commit_global_tick, Some(gt(1)));
        assert_eq!(artifact.resolved.observed_after_global_tick, Some(gt(1)));
        assert_eq!(artifact.resolved.state_root, snapshot.state_root);
        assert_eq!(artifact.resolved.commit_hash, snapshot.hash);
    }

    #[test]
    fn recorded_truth_frontier_without_commits_is_unavailable() {
        let (engine, runtime, provenance, worldline_id) = empty_runtime_fixture();
        let err = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest {
                coordinate: ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Frontier,
                },
                frame: ObservationFrame::RecordedTruth,
                projection: ObservationProjection::TruthChannels { channels: None },
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            ObservationError::ObservationUnavailable {
                worldline_id,
                at: ObservationAt::Frontier,
            }
        );
    }

    #[test]
    fn recorded_truth_reads_recorded_outputs_only() {
        let (engine, runtime, provenance, worldline_id) = recorded_truth_fixture();
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest {
                coordinate: ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Frontier,
                },
                frame: ObservationFrame::RecordedTruth,
                projection: ObservationProjection::TruthChannels { channels: None },
            },
        )
        .unwrap();
        let channels = if let ObservationPayload::TruthChannels(channels) = artifact.payload {
            channels
        } else {
            Vec::new()
        };
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].1, b"truth".to_vec());
    }

    #[test]
    fn identical_requests_produce_stable_artifact_hashes() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let request = ObservationRequest {
            coordinate: ObservationCoordinate {
                worldline_id,
                at: ObservationAt::Frontier,
            },
            frame: ObservationFrame::CommitBoundary,
            projection: ObservationProjection::Head,
        };
        let first =
            ObservationService::observe(&runtime, &provenance, &engine, request.clone()).unwrap();
        let second = ObservationService::observe(&runtime, &provenance, &engine, request).unwrap();
        assert_eq!(first.artifact_hash, second.artifact_hash);
        assert_eq!(first.to_abi(), second.to_abi());
    }

    #[test]
    fn observation_is_zero_write_for_runtime_and_provenance() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let runtime_before = runtime.clone();
        let provenance_before = provenance.clone();

        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest {
                coordinate: ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Tick(wt(0)),
                },
                frame: ObservationFrame::CommitBoundary,
                projection: ObservationProjection::Snapshot,
            },
        )
        .unwrap();

        assert_eq!(artifact.resolved.resolved_worldline_tick, wt(0));
        assert_eq!(artifact.resolved.commit_global_tick, Some(gt(1)));
        assert_eq!(artifact.resolved.observed_after_global_tick, Some(gt(1)));
        assert_eq!(
            provenance
                .entry(worldline_id, wt(0))
                .unwrap()
                .commit_global_tick,
            gt(1)
        );
        let frontier_after = runtime.worldlines().get(&worldline_id).unwrap();
        let frontier_before = runtime_before.worldlines().get(&worldline_id).unwrap();
        assert_eq!(runtime.global_tick(), runtime_before.global_tick());
        assert_eq!(
            frontier_after.frontier_tick(),
            frontier_before.frontier_tick()
        );
        assert_eq!(
            frontier_after.state().current_tick(),
            frontier_before.state().current_tick()
        );
        assert_eq!(
            frontier_after
                .state()
                .last_snapshot()
                .map(|snapshot| snapshot.hash),
            frontier_before
                .state()
                .last_snapshot()
                .map(|snapshot| snapshot.hash)
        );
        assert_eq!(
            provenance.len(worldline_id).unwrap(),
            provenance_before.len(worldline_id).unwrap()
        );
        assert_eq!(
            provenance.entry(worldline_id, wt(0)).unwrap(),
            provenance_before.entry(worldline_id, wt(0)).unwrap()
        );
    }
}
