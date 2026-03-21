// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Entry-oriented provenance store and BTR helpers for SPEC-0004.
//!
//! Phase 4 replaces the old aligned side arrays with a single entry model that
//! is structurally ready for DAG parents, richer event kinds, and deterministic
//! export packaging.
//!
//! # Key Types
//!
//! - [`ProvenanceStore`]: Trait defining the authoritative provenance access API.
//! - [`LocalProvenanceStore`]: In-memory entry-backed implementation.
//! - [`ProvenanceService`]: Standalone multi-worldline provenance subsystem.
//! - [`ProvenanceEntry`]: Single source of truth for one recorded provenance step.
//! - [`BoundaryTransitionRecord`]: Deterministic contiguous provenance segment.
//! - [`HistoryError`]: Error type for history access failures.
//!
//! # `U0Ref` = `WarpId`
//!
//! Per SPEC-0004 correction #1, the `U0Ref` (initial state reference) for MVP is
//! simply the `WarpId`. The engine's `initial_state` for a warp serves as the U0
//! starting point for replay.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::clock::{GlobalTick, WorldlineTick};
use crate::head::WriterHeadKey;
use crate::ident::{Hash, NodeKey, WarpId};
use crate::materialization::FinalizedChannel;
use crate::receipt::TickReceipt;
use crate::snapshot::{compute_commit_hash_v2, compute_state_root_for_warp_state, Snapshot};
use crate::tick_patch::{TickCommitStatus, WarpTickPatchV1};
use crate::tx::TxId;
use crate::worldline_state::WorldlineState;

use super::worldline::{
    ApplyError, AtomWrite, AtomWriteSet, HashTriplet, OutputFrameSet, WorldlineId,
    WorldlineTickPatchV1,
};

/// Errors that can occur when accessing worldline history.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HistoryError {
    /// The requested tick is not available in the store.
    #[error("history unavailable for tick {tick}")]
    HistoryUnavailable {
        /// The tick that was requested but not found.
        tick: WorldlineTick,
    },

    /// The requested worldline does not exist.
    #[error("worldline not found: {0:?}")]
    WorldlineNotFound(WorldlineId),

    /// The target worldline already exists (e.g., during a fork).
    #[error("worldline already exists: {0:?}")]
    WorldlineAlreadyExists(WorldlineId),

    /// The provided tick does not match the expected next tick (append-only invariant).
    #[error("tick gap: expected tick {expected}, got {got}")]
    TickGap {
        /// The tick that was expected (current history length).
        expected: WorldlineTick,
        /// The tick that was provided.
        got: WorldlineTick,
    },

    /// The entry worldline does not match the destination worldline.
    #[error("entry worldline mismatch: expected {expected:?}, got {got:?}")]
    EntryWorldlineMismatch {
        /// The registered worldline that was being appended.
        expected: WorldlineId,
        /// The worldline encoded in the entry.
        got: WorldlineId,
    },

    /// A local commit entry must carry the committing writer head.
    #[error("local commit missing head attribution for tick {tick}")]
    LocalCommitMissingHeadKey {
        /// The entry tick.
        tick: WorldlineTick,
    },

    /// A local commit entry must carry a replay patch.
    #[error("local commit missing patch for tick {tick}")]
    LocalCommitMissingPatch {
        /// The entry tick.
        tick: WorldlineTick,
    },

    /// The local commit head must belong to the same worldline as the entry.
    #[error("local commit head/worldline mismatch: entry {entry_worldline:?}, head {head_key:?}")]
    HeadWorldlineMismatch {
        /// Worldline encoded in the entry.
        entry_worldline: WorldlineId,
        /// Head key carried by the entry.
        head_key: WriterHeadKey,
    },

    /// `append_local_commit(...)` only admits local-commit provenance entries.
    #[error("append_local_commit rejected non-local event kind {got:?} at tick {tick}")]
    InvalidLocalCommitEventKind {
        /// The entry tick.
        tick: WorldlineTick,
        /// The unexpected event kind.
        got: ProvenanceEventKind,
    },

    /// Parent references must already be stored in canonical commit-hash order.
    #[error("parent refs must be in canonical commit-hash order at tick {tick}")]
    NonCanonicalParents {
        /// The entry tick whose parent refs were non-canonical.
        tick: WorldlineTick,
    },

    /// A parent ref must resolve to an already-recorded provenance entry.
    #[error("missing parent ref {parent:?} for tick {tick}")]
    MissingParentRef {
        /// The entry tick carrying the invalid parent ref.
        tick: WorldlineTick,
        /// The missing parent ref.
        parent: ProvenanceRef,
    },

    /// A parent ref must match the stored commit hash at its referenced coordinate.
    #[error("parent ref commit hash mismatch at tick {tick}: parent {parent:?}, stored {stored_commit_hash:?}")]
    ParentCommitHashMismatch {
        /// The entry tick carrying the invalid parent ref.
        tick: WorldlineTick,
        /// The provided parent ref.
        parent: ProvenanceRef,
        /// The stored commit hash at the referenced coordinate.
        stored_commit_hash: Hash,
    },

    /// A checkpoint carried a state for the wrong root warp.
    #[error("checkpoint root warp mismatch for worldline {worldline_id:?}: expected {expected:?}, got {actual:?}")]
    CheckpointRootWarpMismatch {
        /// Worldline being checkpointed.
        worldline_id: WorldlineId,
        /// Registered root warp for the worldline.
        expected: WarpId,
        /// Root warp encoded by the checkpoint state.
        actual: WarpId,
    },

    /// A checkpoint replay base did not match the registered initial boundary.
    #[error("checkpoint initial boundary hash mismatch for worldline {worldline_id:?}")]
    CheckpointInitialBoundaryHashMismatch {
        /// Worldline being checkpointed.
        worldline_id: WorldlineId,
        /// Registered deterministic initial boundary hash.
        expected: Hash,
        /// Hash computed from the checkpoint's replay base.
        actual: Hash,
    },

    /// A checkpoint's materialized state root was inconsistent or disagreed with provenance.
    #[error("checkpoint state root mismatch at tick {tick}")]
    CheckpointStateRootMismatch {
        /// Checkpoint coordinate whose state root mismatched.
        tick: WorldlineTick,
        /// Expected committed state root.
        expected: Hash,
        /// Observed checkpoint state root.
        actual: Hash,
    },
}

/// Errors that can occur when constructing or validating a BTR.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BtrError {
    /// Wrapped history lookup failure.
    #[error(transparent)]
    History(#[from] HistoryError),

    /// A BTR must carry at least one provenance entry.
    #[error("BTR payload cannot be empty")]
    EmptyPayload,

    /// The record worldline must match the payload worldline.
    #[error("BTR worldline mismatch: expected {expected:?}, got {got:?}")]
    WorldlineMismatch {
        /// Worldline claimed by the record.
        expected: WorldlineId,
        /// Worldline found in payload content.
        got: WorldlineId,
    },

    /// A payload entry belonged to a different worldline.
    #[error("BTR payload mixed worldlines: expected {expected:?}, got {got:?}")]
    MixedWorldline {
        /// The payload worldline that all entries must match.
        expected: WorldlineId,
        /// The mismatching entry worldline.
        got: WorldlineId,
    },

    /// Payload ticks must form one contiguous run.
    #[error("BTR payload is not contiguous: expected tick {expected}, got {got}")]
    NonContiguousTicks {
        /// The next expected tick.
        expected: WorldlineTick,
        /// The observed tick.
        got: WorldlineTick,
    },

    /// Payload tick arithmetic overflowed `u64`.
    #[error("BTR payload tick arithmetic overflowed")]
    TickOverflow,

    /// The record worldline was not registered in the provenance service.
    #[error("BTR references unknown worldline: {0:?}")]
    UnknownWorldline(WorldlineId),

    /// The record `u0_ref` does not match the registered worldline.
    #[error("BTR u0_ref mismatch: expected {expected:?}, got {got:?}")]
    U0RefMismatch {
        /// Registered value.
        expected: WarpId,
        /// Value carried by the BTR.
        got: WarpId,
    },

    /// The input boundary hash does not match the worldline prefix before the payload.
    #[error("BTR input boundary hash mismatch")]
    InputBoundaryHashMismatch {
        /// Expected deterministic input boundary.
        expected: Hash,
        /// Value carried by the BTR.
        got: Hash,
    },

    /// The output boundary hash does not match the payload tip.
    #[error("BTR output boundary hash mismatch")]
    OutputBoundaryHashMismatch {
        /// Expected deterministic output boundary.
        expected: Hash,
        /// Value carried by the BTR.
        got: Hash,
    },

    /// A payload entry diverges from the authoritative stored history.
    #[error("BTR payload entry mismatch at tick {tick}")]
    EntryMismatch {
        /// The mismatching entry tick.
        tick: WorldlineTick,
    },
}

/// Errors that can occur while reconstructing full worldline state from provenance.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReplayError {
    /// Wrapped history lookup failure.
    #[error(transparent)]
    History(#[from] HistoryError),

    /// Wrapped full-state patch replay failure.
    #[error("replay apply error at tick {tick}: {source}")]
    Apply {
        /// Tick whose patch application failed.
        tick: WorldlineTick,
        /// Underlying worldline-state apply failure.
        #[source]
        source: ApplyError,
    },

    /// A stored checkpoint had materialized state that did not match its metadata hash.
    #[error("checkpoint state root mismatch at tick {tick}")]
    CheckpointStateRootMismatch {
        /// Checkpoint coordinate whose stored state was inconsistent.
        tick: WorldlineTick,
        /// Stored committed state root.
        expected: Hash,
        /// Recomputed checkpoint state root.
        actual: Hash,
    },

    /// The replay base does not belong to the requested worldline root warp.
    #[error("replay base root warp mismatch: expected {expected:?}, got {actual:?}")]
    ReplayBaseWarpMismatch {
        /// Registered root warp for the worldline.
        expected: WarpId,
        /// Root warp found in the supplied replay base.
        actual: WarpId,
    },

    /// The replay base initial state does not match the registered provenance boundary.
    #[error("replay base initial boundary hash mismatch")]
    InitialBoundaryHashMismatch {
        /// Registered deterministic initial boundary hash.
        expected: Hash,
        /// Hash computed from the supplied replay base.
        actual: Hash,
    },

    /// A replayable provenance entry was missing its stored patch.
    #[error("replay entry missing patch at tick {tick}")]
    MissingPatch {
        /// Tick whose patch was required but absent.
        tick: WorldlineTick,
    },

    /// Replay metadata could not derive a receipt transaction id from the tick.
    #[error("replay tick arithmetic overflowed while deriving metadata for tick {tick}")]
    TickOverflow {
        /// Tick whose metadata overflowed.
        tick: WorldlineTick,
    },

    /// Patch digest commitments disagreed during replay reconstruction.
    #[error("replay patch digest mismatch at tick {tick}")]
    PatchDigestMismatch {
        /// Tick whose digest mismatched.
        tick: WorldlineTick,
        /// Authoritative stored digest.
        expected: Hash,
        /// Reconstructed digest.
        actual: Hash,
    },

    /// Replayed state root did not match the stored provenance commitment.
    #[error("replay state root mismatch at tick {tick}")]
    StateRootMismatch {
        /// Tick whose state root mismatched.
        tick: WorldlineTick,
        /// Stored committed state root.
        expected: Hash,
        /// Recomputed state root after replay.
        actual: Hash,
    },

    /// Replayed commit hash did not match the stored provenance commitment.
    #[error("replay commit hash mismatch at tick {tick}")]
    CommitHashMismatch {
        /// Tick whose commit hash mismatched.
        tick: WorldlineTick,
        /// Stored committed hash.
        expected: Hash,
        /// Recomputed commit hash after replay.
        actual: Hash,
    },
}

/// Reference to a checkpoint within the provenance store.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CheckpointRef {
    /// Cursor coordinate where this checkpoint was taken.
    ///
    /// `0` denotes the U0 replay base and `N` denotes the state after applying
    /// patches `0..N-1`; this is not a provenance entry index.
    pub worldline_tick: WorldlineTick,
    /// Hash of the state at this checkpoint.
    pub state_hash: Hash,
}

/// Materialized replay checkpoint stored alongside provenance metadata.
#[derive(Clone, Debug)]
pub struct ReplayCheckpoint {
    /// Metadata for the stored checkpoint coordinate.
    pub checkpoint: CheckpointRef,
    /// Full worldline state materialized at `checkpoint.worldline_tick`.
    pub state: WorldlineState,
}

impl ReplayCheckpoint {
    /// Builds a deterministic replay checkpoint from a full worldline state.
    #[must_use]
    pub fn from_state(state: &WorldlineState) -> Self {
        Self {
            checkpoint: CheckpointRef {
                worldline_tick: state.current_tick(),
                state_hash: state.state_root(),
            },
            state: state.replay_checkpoint_clone(),
        }
    }
}

/// Reference to a parent provenance commit.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProvenanceRef {
    /// Parent worldline.
    pub worldline_id: WorldlineId,
    /// Parent tick identity.
    pub worldline_tick: WorldlineTick,
    /// Parent commit hash.
    pub commit_hash: Hash,
}

/// Event kind recorded by a provenance entry.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProvenanceEventKind {
    /// A local writer-head commit produced by the live runtime.
    LocalCommit,
    /// Placeholder for a future cross-worldline message delivery.
    CrossWorldlineMessage {
        /// Source worldline.
        source_worldline: WorldlineId,
        /// Source tick.
        source_worldline_tick: WorldlineTick,
        /// Stable message id.
        message_id: Hash,
    },
    /// Placeholder for a future merge/import event.
    MergeImport {
        /// Source worldline.
        source_worldline: WorldlineId,
        /// Source tick.
        source_worldline_tick: WorldlineTick,
        /// Stable imported op id.
        op_id: Hash,
    },
    /// Placeholder for a future conflict artifact.
    ConflictArtifact {
        /// Stable conflict artifact id.
        artifact_id: Hash,
    },
}

/// Single authoritative provenance record for one worldline step.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProvenanceEntry {
    /// Worldline that owns this entry.
    pub worldline_id: WorldlineId,
    /// Append identity within the worldline (0-based).
    pub worldline_tick: WorldlineTick,
    /// Correlation metadata from the runtime SuperTick.
    pub commit_global_tick: GlobalTick,
    /// Writer head that produced this entry, when applicable.
    pub head_key: Option<WriterHeadKey>,
    /// Explicit parent refs in canonical stored order.
    pub parents: Vec<ProvenanceRef>,
    /// Recorded event kind.
    pub event_kind: ProvenanceEventKind,
    /// Recorded state/patch/commit commitments.
    pub expected: HashTriplet,
    /// Replay patch for this entry, when applicable.
    pub patch: Option<WorldlineTickPatchV1>,
    /// Recorded materialization outputs.
    pub outputs: OutputFrameSet,
    /// Recorded atom-write provenance.
    pub atom_writes: AtomWriteSet,
}

impl ProvenanceEntry {
    /// Returns the commit reference for this entry.
    #[must_use]
    pub fn as_ref(&self) -> ProvenanceRef {
        ProvenanceRef {
            worldline_id: self.worldline_id,
            worldline_tick: self.worldline_tick,
            commit_hash: self.expected.commit_hash,
        }
    }

    /// Constructs a local commit provenance entry.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn local_commit(
        worldline_id: WorldlineId,
        worldline_tick: WorldlineTick,
        commit_global_tick: GlobalTick,
        head_key: WriterHeadKey,
        parents: Vec<ProvenanceRef>,
        expected: HashTriplet,
        patch: WorldlineTickPatchV1,
        outputs: OutputFrameSet,
        atom_writes: AtomWriteSet,
    ) -> Self {
        Self {
            worldline_id,
            worldline_tick,
            commit_global_tick,
            head_key: Some(head_key),
            parents,
            event_kind: ProvenanceEventKind::LocalCommit,
            expected,
            patch: Some(patch),
            outputs,
            atom_writes,
        }
    }
}

/// Single-worldline contiguous provenance payload.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BtrPayload {
    /// Worldline represented by this payload.
    pub worldline_id: WorldlineId,
    /// First worldline tick included in `entries`.
    pub start_worldline_tick: WorldlineTick,
    /// Contiguous entries in append order.
    pub entries: Vec<ProvenanceEntry>,
}

impl BtrPayload {
    /// Returns the exclusive end tick for the payload.
    ///
    /// # Errors
    ///
    /// Returns [`BtrError::TickOverflow`] if the payload length cannot be added
    /// to `start_tick` without overflowing `u64`.
    pub fn end_tick_exclusive(&self) -> Result<WorldlineTick, BtrError> {
        self.start_worldline_tick
            .checked_add(self.entries.len() as u64)
            .ok_or(BtrError::TickOverflow)
    }

    /// Validates structural payload invariants.
    ///
    /// # Errors
    ///
    /// Returns [`BtrError`] if the payload is empty, mixes worldlines, or is
    /// not contiguous by `worldline_tick`.
    pub fn validate(&self) -> Result<(), BtrError> {
        let Some(first) = self.entries.first() else {
            return Err(BtrError::EmptyPayload);
        };
        if first.worldline_id != self.worldline_id {
            return Err(BtrError::MixedWorldline {
                expected: self.worldline_id,
                got: first.worldline_id,
            });
        }
        if first.worldline_tick != self.start_worldline_tick {
            return Err(BtrError::NonContiguousTicks {
                expected: self.start_worldline_tick,
                got: first.worldline_tick,
            });
        }

        let mut expected_tick = self.start_worldline_tick;
        for entry in &self.entries {
            if entry.worldline_id != self.worldline_id {
                return Err(BtrError::MixedWorldline {
                    expected: self.worldline_id,
                    got: entry.worldline_id,
                });
            }
            if entry.worldline_tick != expected_tick {
                return Err(BtrError::NonContiguousTicks {
                    expected: expected_tick,
                    got: entry.worldline_tick,
                });
            }
            expected_tick = expected_tick
                .checked_increment()
                .ok_or(BtrError::TickOverflow)?;
        }

        Ok(())
    }
}

/// Boundary Transition Record (BTR) for a contiguous provenance segment.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoundaryTransitionRecord {
    /// Worldline carried by this record.
    pub worldline_id: WorldlineId,
    /// Initial worldline state handle.
    pub u0_ref: WarpId,
    /// State boundary hash before the payload begins.
    pub input_boundary_hash: Hash,
    /// State boundary hash after the payload ends.
    pub output_boundary_hash: Hash,
    /// Contiguous payload entries.
    pub payload: BtrPayload,
    /// Deterministic monotone counter. No wall-clock semantics.
    pub logical_counter: u64,
    /// Opaque auth payload reserved for later phases.
    pub auth_tag: Vec<u8>,
}

impl BoundaryTransitionRecord {
    /// Validates self-contained BTR invariants.
    ///
    /// # Errors
    ///
    /// Returns [`BtrError`] if the payload is malformed, worldline ids disagree,
    /// or the output boundary does not match the payload tip.
    pub fn validate(&self) -> Result<(), BtrError> {
        self.payload.validate()?;
        if self.payload.worldline_id != self.worldline_id {
            return Err(BtrError::WorldlineMismatch {
                expected: self.worldline_id,
                got: self.payload.worldline_id,
            });
        }
        let Some(last) = self.payload.entries.last() else {
            return Err(BtrError::EmptyPayload);
        };
        if self.output_boundary_hash != last.expected.state_root {
            return Err(BtrError::OutputBoundaryHashMismatch {
                expected: last.expected.state_root,
                got: self.output_boundary_hash,
            });
        }
        Ok(())
    }
}

/// Trait for accessing worldline provenance data.
pub trait ProvenanceStore: Send + Sync {
    /// Returns the U0 reference (initial state handle) for a worldline.
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError>;

    /// Returns the registered deterministic initial boundary hash for a worldline.
    fn initial_boundary_hash(&self, w: WorldlineId) -> Result<Hash, HistoryError>;

    /// Returns the number of recorded ticks for a worldline.
    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError>;

    /// Returns the entry for a specific tick.
    fn entry(&self, w: WorldlineId, tick: WorldlineTick) -> Result<ProvenanceEntry, HistoryError>;

    /// Returns the stored parent refs for a specific tick.
    fn parents(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Result<Vec<ProvenanceRef>, HistoryError>;

    /// Appends a local commit entry.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] if the worldline does not exist, the tick is not
    /// append-only, or the entry violates local-commit invariants.
    fn append_local_commit(&mut self, entry: ProvenanceEntry) -> Result<(), HistoryError>;

    /// Returns the nearest checkpoint before a given tick, if any.
    fn checkpoint_before(&self, w: WorldlineId, tick: WorldlineTick) -> Option<CheckpointRef>;

    /// Returns the nearest materialized checkpoint before a given tick, if any.
    fn checkpoint_state_before(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Option<ReplayCheckpoint>;

    /// Returns whether the worldline has any recorded history.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    fn is_empty(&self, w: WorldlineId) -> Result<bool, HistoryError> {
        Ok(self.len(w)? == 0)
    }
}

pub(crate) fn finalized_channels(outputs: &OutputFrameSet) -> Vec<FinalizedChannel> {
    outputs
        .iter()
        .map(|(channel, data)| FinalizedChannel {
            channel: *channel,
            data: data.clone(),
        })
        .collect()
}

pub(crate) fn replay_artifacts_for_entry(
    root: NodeKey,
    entry: &ProvenanceEntry,
    patch: &crate::worldline::WorldlineTickPatchV1,
) -> Result<(Snapshot, TickReceipt, WarpTickPatchV1), ReplayError> {
    let tick = entry.worldline_tick;
    if entry.expected.patch_digest != patch.patch_digest {
        return Err(ReplayError::PatchDigestMismatch {
            tick,
            expected: entry.expected.patch_digest,
            actual: patch.patch_digest,
        });
    }

    let replay_patch = WarpTickPatchV1::new(
        patch.policy_id(),
        patch.rule_pack_id(),
        TickCommitStatus::Committed,
        patch.in_slots.clone(),
        patch.out_slots.clone(),
        patch.ops.clone(),
    );
    let replay_patch_digest = replay_patch.digest();
    if replay_patch_digest != patch.patch_digest {
        return Err(ReplayError::PatchDigestMismatch {
            tick,
            expected: patch.patch_digest,
            actual: replay_patch_digest,
        });
    }

    let parents = entry
        .parents
        .iter()
        .map(|parent| parent.commit_hash)
        .collect::<Vec<_>>();
    let tx = tick
        .as_u64()
        .checked_add(1)
        .map(TxId::from_raw)
        .ok_or(ReplayError::TickOverflow { tick })?;
    let snapshot = Snapshot {
        root,
        hash: entry.expected.commit_hash,
        state_root: entry.expected.state_root,
        parents,
        plan_digest: patch.header.plan_digest,
        decision_digest: patch.header.decision_digest,
        rewrites_digest: patch.header.rewrites_digest,
        patch_digest: entry.expected.patch_digest,
        policy_id: patch.policy_id(),
        tx,
    };
    let receipt = TickReceipt::new(snapshot.tx, Vec::new(), Vec::new());
    Ok((snapshot, receipt, replay_patch))
}

fn expected_state_root_at_materialized_tick<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    tick: WorldlineTick,
) -> Result<Hash, ReplayError> {
    if tick == WorldlineTick::ZERO {
        return provenance
            .initial_boundary_hash(worldline_id)
            .map_err(ReplayError::from);
    }

    let commit_tick = tick
        .checked_sub(1)
        .ok_or(ReplayError::TickOverflow { tick })?;
    Ok(provenance
        .entry(worldline_id, commit_tick)?
        .expected
        .state_root)
}

pub(crate) fn hydrate_replay_metadata<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    replayed: &mut WorldlineState,
    target_tick: WorldlineTick,
) -> Result<(), ReplayError> {
    let target_len = usize::try_from(target_tick.as_u64())
        .map_err(|_| ReplayError::TickOverflow { tick: target_tick })?;
    if replayed.tick_history.len() > target_len {
        replayed.tick_history.truncate(target_len);
    }

    let root = *replayed.root();
    let mut last_entry = None;
    for raw_tick in replayed.tick_history.len() as u64..target_tick.as_u64() {
        let entry = provenance.entry(worldline_id, WorldlineTick::from_raw(raw_tick))?;
        let patch = entry.patch.as_ref().ok_or(ReplayError::MissingPatch {
            tick: entry.worldline_tick,
        })?;
        let (snapshot, receipt, replay_patch) = replay_artifacts_for_entry(root, &entry, patch)?;
        replayed
            .tick_history
            .push((snapshot, receipt, replay_patch));
        last_entry = Some(entry);
    }

    finalize_replay_metadata(replayed, target_tick, last_entry.as_ref());

    Ok(())
}

fn finalize_replay_metadata(
    replayed: &mut WorldlineState,
    target_tick: WorldlineTick,
    last_entry: Option<&ProvenanceEntry>,
) {
    if target_tick == WorldlineTick::ZERO {
        replayed.last_snapshot = None;
        replayed.last_materialization.clear();
        replayed.last_materialization_errors.clear();
        replayed.tx_counter = 0;
        return;
    }

    debug_assert_eq!(replayed.tick_history.len() as u64, target_tick.as_u64());
    replayed.last_snapshot = replayed
        .tick_history
        .last()
        .map(|(snapshot, _, _)| snapshot.clone());
    if let Some(entry) = last_entry {
        replayed.last_materialization = finalized_channels(&entry.outputs);
    }
    replayed.last_materialization_errors.clear();
    replayed.tx_counter = target_tick.as_u64();
}

fn clear_replay_metadata(replayed: &mut WorldlineState) {
    replayed.last_snapshot = None;
    replayed.tick_history.clear();
    replayed.last_materialization.clear();
    replayed.last_materialization_errors.clear();
    replayed.tx_counter = 0;
    replayed.committed_ingress.clear();
}

pub(crate) fn validate_replay_base<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    base_state: &WorldlineState,
) -> Result<(), ReplayError> {
    let expected_root_warp = provenance.u0(worldline_id)?;
    if base_state.root().warp_id != expected_root_warp {
        return Err(ReplayError::ReplayBaseWarpMismatch {
            expected: expected_root_warp,
            actual: base_state.root().warp_id,
        });
    }

    let expected_initial_boundary = provenance.initial_boundary_hash(worldline_id)?;
    let actual_initial_boundary =
        compute_state_root_for_warp_state(base_state.initial_state(), base_state.root());
    if actual_initial_boundary != expected_initial_boundary {
        return Err(ReplayError::InitialBoundaryHashMismatch {
            expected: expected_initial_boundary,
            actual: actual_initial_boundary,
        });
    }

    Ok(())
}

pub(crate) fn restore_replay_base<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    base_state: &WorldlineState,
    target_tick: WorldlineTick,
) -> Result<(WorldlineState, WorldlineTick), ReplayError> {
    let checkpoint_lookup_tick = target_tick.checked_increment().unwrap_or(target_tick);
    if let Some(checkpoint) =
        provenance.checkpoint_state_before(worldline_id, checkpoint_lookup_tick)
    {
        let expected_state_root = expected_state_root_at_materialized_tick(
            provenance,
            worldline_id,
            checkpoint.checkpoint.worldline_tick,
        )?;
        if checkpoint.checkpoint.state_hash != expected_state_root {
            return Err(ReplayError::CheckpointStateRootMismatch {
                tick: checkpoint.checkpoint.worldline_tick,
                expected: expected_state_root,
                actual: checkpoint.checkpoint.state_hash,
            });
        }

        let actual_state_root = checkpoint.state.state_root();
        if actual_state_root != expected_state_root {
            return Err(ReplayError::CheckpointStateRootMismatch {
                tick: checkpoint.checkpoint.worldline_tick,
                expected: expected_state_root,
                actual: actual_state_root,
            });
        }

        let checkpoint_tick = checkpoint.checkpoint.worldline_tick;
        let mut replayed = checkpoint.state;
        clear_replay_metadata(&mut replayed);
        hydrate_replay_metadata(provenance, worldline_id, &mut replayed, checkpoint_tick)?;
        return Ok((replayed, checkpoint_tick));
    }

    let mut replayed = base_state.clone();
    replayed.warp_state = replayed.initial_state.clone();
    clear_replay_metadata(&mut replayed);
    Ok((replayed, WorldlineTick::ZERO))
}

pub(crate) fn advance_replay_state<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    replayed: &mut WorldlineState,
    start_tick: WorldlineTick,
    target_tick: WorldlineTick,
) -> Result<(), ReplayError> {
    if start_tick == target_tick {
        return Ok(());
    }

    let root = *replayed.root();
    let mut last_entry = None;
    for raw_tick in start_tick.as_u64()..target_tick.as_u64() {
        let tick = WorldlineTick::from_raw(raw_tick);
        let entry = provenance.entry(worldline_id, tick)?;
        let patch = entry
            .patch
            .as_ref()
            .ok_or(ReplayError::MissingPatch { tick })?;

        patch
            .apply_to_worldline_state(replayed)
            .map_err(|source| ReplayError::Apply { tick, source })?;

        let actual_state_root =
            compute_state_root_for_warp_state(replayed.warp_state(), replayed.root());
        if actual_state_root != entry.expected.state_root {
            return Err(ReplayError::StateRootMismatch {
                tick,
                expected: entry.expected.state_root,
                actual: actual_state_root,
            });
        }

        let parent_hashes = entry
            .parents
            .iter()
            .map(|parent| parent.commit_hash)
            .collect::<Vec<_>>();
        let actual_commit_hash = compute_commit_hash_v2(
            &actual_state_root,
            &parent_hashes,
            &entry.expected.patch_digest,
            patch.policy_id(),
        );
        if actual_commit_hash != entry.expected.commit_hash {
            return Err(ReplayError::CommitHashMismatch {
                tick,
                expected: entry.expected.commit_hash,
                actual: actual_commit_hash,
            });
        }

        let (snapshot, receipt, replay_patch) = replay_artifacts_for_entry(root, &entry, patch)?;
        replayed
            .tick_history
            .push((snapshot, receipt, replay_patch));
        last_entry = Some(entry);
    }

    finalize_replay_metadata(replayed, target_tick, last_entry.as_ref());
    Ok(())
}

pub(crate) fn replay_worldline_state_at_from_provenance<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    base_state: &WorldlineState,
    target_tick: WorldlineTick,
) -> Result<WorldlineState, ReplayError> {
    let history_len = WorldlineTick::from_raw(provenance.len(worldline_id)?);
    if target_tick > history_len {
        return Err(ReplayError::History(HistoryError::HistoryUnavailable {
            tick: target_tick,
        }));
    }

    validate_replay_base(provenance, worldline_id, base_state)?;
    let (mut replayed, start_tick) =
        restore_replay_base(provenance, worldline_id, base_state, target_tick)?;
    advance_replay_state(
        provenance,
        worldline_id,
        &mut replayed,
        start_tick,
        target_tick,
    )?;
    Ok(replayed)
}

#[derive(Debug, Clone)]
struct WorldlineHistory {
    u0_ref: WarpId,
    initial_boundary_hash: Hash,
    entries: Vec<ProvenanceEntry>,
    checkpoints: Vec<ReplayCheckpoint>,
}

fn expected_state_root_for_checkpoint(
    history: &WorldlineHistory,
    tick: WorldlineTick,
) -> Result<Hash, HistoryError> {
    if tick == WorldlineTick::ZERO {
        return Ok(history.initial_boundary_hash);
    }

    let commit_index = usize::try_from(
        tick.checked_sub(1)
            .ok_or(HistoryError::HistoryUnavailable { tick })?
            .as_u64(),
    )
    .map_err(|_| HistoryError::HistoryUnavailable { tick })?;

    history
        .entries
        .get(commit_index)
        .map(|entry| entry.expected.state_root)
        .ok_or(HistoryError::HistoryUnavailable { tick })
}

fn validate_checkpoint_for_history(
    worldline_id: WorldlineId,
    history: &WorldlineHistory,
    checkpoint: &ReplayCheckpoint,
) -> Result<(), HistoryError> {
    let checkpoint_tick = checkpoint.checkpoint.worldline_tick;
    if checkpoint_tick.as_u64() > history.entries.len() as u64 {
        return Err(HistoryError::HistoryUnavailable {
            tick: checkpoint_tick,
        });
    }

    let root = *checkpoint.state.root();
    if root.warp_id != history.u0_ref {
        return Err(HistoryError::CheckpointRootWarpMismatch {
            worldline_id,
            expected: history.u0_ref,
            actual: root.warp_id,
        });
    }

    let actual_initial_boundary = compute_state_root_for_warp_state(
        checkpoint.state.initial_state(),
        checkpoint.state.root(),
    );
    if actual_initial_boundary != history.initial_boundary_hash {
        return Err(HistoryError::CheckpointInitialBoundaryHashMismatch {
            worldline_id,
            expected: history.initial_boundary_hash,
            actual: actual_initial_boundary,
        });
    }

    let actual_state_root = checkpoint.state.state_root();
    if actual_state_root != checkpoint.checkpoint.state_hash {
        return Err(HistoryError::CheckpointStateRootMismatch {
            tick: checkpoint_tick,
            expected: checkpoint.checkpoint.state_hash,
            actual: actual_state_root,
        });
    }

    let expected_state_root = expected_state_root_for_checkpoint(history, checkpoint_tick)?;
    if actual_state_root != expected_state_root {
        return Err(HistoryError::CheckpointStateRootMismatch {
            tick: checkpoint_tick,
            expected: expected_state_root,
            actual: actual_state_root,
        });
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProvenanceWorldlineCheckpoint {
    entry_len: usize,
    checkpoint_len: usize,
}

/// Lightweight rollback marker for touched provenance worldlines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceCheckpoint {
    worldlines: BTreeMap<WorldlineId, ProvenanceWorldlineCheckpoint>,
}

/// In-memory provenance store backed by `Vec`s.
#[derive(Debug, Clone, Default)]
pub struct LocalProvenanceStore {
    worldlines: BTreeMap<WorldlineId, WorldlineHistory>,
}

impl LocalProvenanceStore {
    /// Creates a new empty provenance store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new worldline with its U0 reference.
    ///
    /// This convenience helper uses a deterministic zero digest as the initial
    /// boundary. [`ProvenanceService`] should prefer
    /// [`Self::register_worldline_with_boundary`] so BTR construction can use the
    /// real genesis boundary hash.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineAlreadyExists`] if the worldline is already
    /// registered with a different `u0_ref`.
    pub fn register_worldline(
        &mut self,
        id: WorldlineId,
        u0_ref: WarpId,
    ) -> Result<(), HistoryError> {
        self.register_worldline_with_boundary(id, u0_ref, crate::constants::digest_len0_u64())
    }

    /// Registers a new worldline with its U0 reference and initial boundary hash.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineAlreadyExists`] if the worldline is already
    /// registered with a different configuration.
    pub fn register_worldline_with_boundary(
        &mut self,
        id: WorldlineId,
        u0_ref: WarpId,
        initial_boundary_hash: Hash,
    ) -> Result<(), HistoryError> {
        use std::collections::btree_map::Entry;

        match self.worldlines.entry(id) {
            Entry::Occupied(existing) => {
                let existing = existing.get();
                if existing.u0_ref != u0_ref
                    || existing.initial_boundary_hash != initial_boundary_hash
                {
                    return Err(HistoryError::WorldlineAlreadyExists(id));
                }
                Ok(())
            }
            Entry::Vacant(vacant) => {
                vacant.insert(WorldlineHistory {
                    u0_ref,
                    initial_boundary_hash,
                    entries: Vec::new(),
                    checkpoints: Vec::new(),
                });
                Ok(())
            }
        }
    }

    fn history(&self, w: WorldlineId) -> Result<&WorldlineHistory, HistoryError> {
        self.worldlines
            .get(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))
    }

    fn history_mut(&mut self, w: WorldlineId) -> Result<&mut WorldlineHistory, HistoryError> {
        self.worldlines
            .get_mut(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))
    }

    fn validate_local_commit_entry(
        worldlines: &BTreeMap<WorldlineId, WorldlineHistory>,
        worldline_id: WorldlineId,
        expected_tick: WorldlineTick,
        entry: &ProvenanceEntry,
    ) -> Result<(), HistoryError> {
        if entry.worldline_id != worldline_id {
            return Err(HistoryError::EntryWorldlineMismatch {
                expected: worldline_id,
                got: entry.worldline_id,
            });
        }
        if entry.worldline_tick != expected_tick {
            return Err(HistoryError::TickGap {
                expected: expected_tick,
                got: entry.worldline_tick,
            });
        }
        let Some(head_key) = entry.head_key else {
            return Err(HistoryError::LocalCommitMissingHeadKey {
                tick: entry.worldline_tick,
            });
        };
        if head_key.worldline_id != entry.worldline_id {
            return Err(HistoryError::HeadWorldlineMismatch {
                entry_worldline: entry.worldline_id,
                head_key,
            });
        }
        if entry.patch.is_none() {
            return Err(HistoryError::LocalCommitMissingPatch {
                tick: entry.worldline_tick,
            });
        }
        if !matches!(entry.event_kind, ProvenanceEventKind::LocalCommit) {
            return Err(HistoryError::InvalidLocalCommitEventKind {
                tick: entry.worldline_tick,
                got: entry.event_kind.clone(),
            });
        }
        if !entry
            .parents
            .windows(2)
            .all(|pair| pair[0].commit_hash < pair[1].commit_hash)
        {
            return Err(HistoryError::NonCanonicalParents {
                tick: entry.worldline_tick,
            });
        }
        for parent in &entry.parents {
            let parent_index = usize::try_from(parent.worldline_tick.as_u64()).map_err(|_| {
                HistoryError::MissingParentRef {
                    tick: entry.worldline_tick,
                    parent: *parent,
                }
            })?;
            let stored = worldlines
                .get(&parent.worldline_id)
                .and_then(|history| history.entries.get(parent_index))
                .ok_or(HistoryError::MissingParentRef {
                    tick: entry.worldline_tick,
                    parent: *parent,
                })?;
            if stored.expected.commit_hash != parent.commit_hash {
                return Err(HistoryError::ParentCommitHashMismatch {
                    tick: entry.worldline_tick,
                    parent: *parent,
                    stored_commit_hash: stored.expected.commit_hash,
                });
            }
        }
        Ok(())
    }

    /// Returns the atom writes for a specific tick.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] if the worldline or tick is unavailable.
    pub fn atom_writes(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Result<AtomWriteSet, HistoryError> {
        Ok(self.entry(w, tick)?.atom_writes)
    }

    /// Returns the atom write history for a specific atom by walking its causal cone.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    pub fn atom_history(
        &self,
        w: WorldlineId,
        atom: &crate::ident::NodeKey,
    ) -> Result<Vec<AtomWrite>, HistoryError> {
        let history = self.history(w)?;
        let attachment_slot = crate::tick_patch::SlotId::Attachment(
            crate::attachment::AttachmentKey::node_alpha(*atom),
        );
        let node_slot = crate::tick_patch::SlotId::Node(*atom);

        let mut writes_rev: Vec<AtomWrite> = Vec::new();

        for entry in history.entries.iter().rev() {
            let Some(patch) = entry.patch.as_ref() else {
                continue;
            };

            let touched = patch
                .out_slots
                .iter()
                .any(|slot| *slot == attachment_slot || *slot == node_slot);
            if !touched {
                continue;
            }

            for aw in entry.atom_writes.iter().rev() {
                if &aw.atom == atom {
                    let is_creation = aw.is_create();
                    writes_rev.push(aw.clone());
                    if is_creation {
                        writes_rev.reverse();
                        return Ok(writes_rev);
                    }
                }
            }
        }

        writes_rev.reverse();
        Ok(writes_rev)
    }

    /// Records a checkpoint for a worldline.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn add_checkpoint(
        &mut self,
        w: WorldlineId,
        checkpoint: ReplayCheckpoint,
    ) -> Result<(), HistoryError> {
        {
            let history = self.history(w)?;
            validate_checkpoint_for_history(w, history, &checkpoint)?;
        }
        let history = self.history_mut(w)?;
        match history
            .checkpoints
            .binary_search_by_key(&checkpoint.checkpoint.worldline_tick, |c| {
                c.checkpoint.worldline_tick
            }) {
            Ok(index) => history.checkpoints[index] = checkpoint,
            Err(pos) => history.checkpoints.insert(pos, checkpoint),
        }
        Ok(())
    }

    /// Creates a checkpoint from the given full worldline state.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn checkpoint(
        &mut self,
        w: WorldlineId,
        state: &WorldlineState,
    ) -> Result<CheckpointRef, HistoryError> {
        let checkpoint = ReplayCheckpoint::from_state(state);
        let checkpoint_ref = checkpoint.checkpoint;
        self.add_checkpoint(w, checkpoint)?;
        Ok(checkpoint_ref)
    }

    /// Creates a new worldline that is a prefix-copy of the source up to `fork_tick`.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] if the source is missing, the target exists, or
    /// `fork_tick` is out of range.
    pub fn fork(
        &mut self,
        source: WorldlineId,
        fork_tick: WorldlineTick,
        new_id: WorldlineId,
    ) -> Result<(), HistoryError> {
        if self.worldlines.contains_key(&new_id) {
            return Err(HistoryError::WorldlineAlreadyExists(new_id));
        }

        let source_history = self.history(source)?;
        let source_len = source_history.entries.len();
        if fork_tick.as_u64() >= source_len as u64 {
            return Err(HistoryError::HistoryUnavailable { tick: fork_tick });
        }

        let end_idx = usize::try_from(
            fork_tick
                .checked_increment()
                .ok_or(HistoryError::HistoryUnavailable { tick: fork_tick })?
                .as_u64(),
        )
        .map_err(|_| HistoryError::HistoryUnavailable { tick: fork_tick })?;
        let checkpoint_max_tick = fork_tick.checked_increment().unwrap_or(WorldlineTick::MAX);
        let new_history = WorldlineHistory {
            u0_ref: source_history.u0_ref,
            initial_boundary_hash: source_history.initial_boundary_hash,
            entries: source_history.entries[..end_idx]
                .iter()
                .cloned()
                .map(|entry| Self::rewrite_entry_for_fork(entry, source, new_id))
                .collect(),
            checkpoints: source_history
                .checkpoints
                .iter()
                .filter(|c| c.checkpoint.worldline_tick <= checkpoint_max_tick)
                .cloned()
                .collect(),
        };
        self.worldlines.insert(new_id, new_history);
        Ok(())
    }

    fn rewrite_entry_for_fork(
        mut entry: ProvenanceEntry,
        source: WorldlineId,
        new_id: WorldlineId,
    ) -> ProvenanceEntry {
        entry.worldline_id = new_id;
        if let Some(head_key) = entry.head_key.as_mut() {
            if head_key.worldline_id == source {
                head_key.worldline_id = new_id;
            }
        }
        for parent in &mut entry.parents {
            if parent.worldline_id == source {
                parent.worldline_id = new_id;
            }
        }
        entry
    }

    /// Returns the initial boundary hash registered for this worldline.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn initial_boundary_hash(&self, w: WorldlineId) -> Result<Hash, HistoryError> {
        Ok(self.history(w)?.initial_boundary_hash)
    }

    /// Returns the tip ref for a worldline, if any.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn tip_ref(&self, w: WorldlineId) -> Result<Option<ProvenanceRef>, HistoryError> {
        Ok(self.history(w)?.entries.last().map(ProvenanceEntry::as_ref))
    }

    fn checkpoint_for<I>(&self, worldline_ids: I) -> Result<ProvenanceCheckpoint, HistoryError>
    where
        I: IntoIterator<Item = WorldlineId>,
    {
        let mut worldlines = BTreeMap::new();
        for worldline_id in worldline_ids {
            let history = self.history(worldline_id)?;
            worldlines.insert(
                worldline_id,
                ProvenanceWorldlineCheckpoint {
                    entry_len: history.entries.len(),
                    checkpoint_len: history.checkpoints.len(),
                },
            );
        }
        Ok(ProvenanceCheckpoint { worldlines })
    }

    fn restore(&mut self, checkpoint: &ProvenanceCheckpoint) {
        for (worldline_id, marker) in &checkpoint.worldlines {
            if let Some(history) = self.worldlines.get_mut(worldline_id) {
                history.entries.truncate(marker.entry_len);
                history.checkpoints.truncate(marker.checkpoint_len);
            } else {
                debug_assert!(false, "provenance checkpoint referenced unknown worldline");
            }
        }
    }
}

impl ProvenanceStore for LocalProvenanceStore {
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError> {
        Ok(self.history(w)?.u0_ref)
    }

    fn initial_boundary_hash(&self, w: WorldlineId) -> Result<Hash, HistoryError> {
        self.initial_boundary_hash(w)
    }

    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError> {
        Ok(self.history(w)?.entries.len() as u64)
    }

    fn entry(&self, w: WorldlineId, tick: WorldlineTick) -> Result<ProvenanceEntry, HistoryError> {
        let index = usize::try_from(tick.as_u64())
            .map_err(|_| HistoryError::HistoryUnavailable { tick })?;
        self.history(w)?
            .entries
            .get(index)
            .cloned()
            .ok_or(HistoryError::HistoryUnavailable { tick })
    }

    fn parents(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Result<Vec<ProvenanceRef>, HistoryError> {
        Ok(self.entry(w, tick)?.parents)
    }

    fn append_local_commit(&mut self, entry: ProvenanceEntry) -> Result<(), HistoryError> {
        let expected_tick =
            WorldlineTick::from_raw(self.history(entry.worldline_id)?.entries.len() as u64);
        Self::validate_local_commit_entry(
            &self.worldlines,
            entry.worldline_id,
            expected_tick,
            &entry,
        )?;
        let history = self.history_mut(entry.worldline_id)?;
        history.entries.push(entry);
        Ok(())
    }

    fn checkpoint_before(&self, w: WorldlineId, tick: WorldlineTick) -> Option<CheckpointRef> {
        let history = self.worldlines.get(&w)?;
        let pos = history
            .checkpoints
            .binary_search_by_key(&tick, |c| c.checkpoint.worldline_tick)
            .unwrap_or_else(|e| e);
        if pos == 0 {
            None
        } else {
            Some(history.checkpoints[pos - 1].checkpoint)
        }
    }

    fn checkpoint_state_before(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Option<ReplayCheckpoint> {
        let history = self.worldlines.get(&w)?;
        let pos = history
            .checkpoints
            .binary_search_by_key(&tick, |c| c.checkpoint.worldline_tick)
            .unwrap_or_else(|e| e);
        if pos == 0 {
            None
        } else {
            Some(history.checkpoints[pos - 1].clone())
        }
    }
}

/// Standalone multi-worldline provenance subsystem.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceService {
    store: LocalProvenanceStore,
}

impl ProvenanceService {
    /// Creates an empty provenance service.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a worldline using its deterministic replay base.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineAlreadyExists`] if the worldline is already
    /// registered with different U0 or boundary metadata.
    pub fn register_worldline(
        &mut self,
        worldline_id: WorldlineId,
        state: &WorldlineState,
    ) -> Result<(), HistoryError> {
        let initial_boundary_hash =
            compute_state_root_for_warp_state(state.initial_state(), state.root());
        self.store.register_worldline_with_boundary(
            worldline_id,
            state.root().warp_id,
            initial_boundary_hash,
        )
    }

    /// Records a checkpoint for a worldline.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn add_checkpoint(
        &mut self,
        worldline_id: WorldlineId,
        checkpoint: ReplayCheckpoint,
    ) -> Result<(), HistoryError> {
        self.store.add_checkpoint(worldline_id, checkpoint)
    }

    /// Creates a checkpoint from the provided full worldline state.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn checkpoint(
        &mut self,
        worldline_id: WorldlineId,
        state: &WorldlineState,
    ) -> Result<CheckpointRef, HistoryError> {
        self.store.checkpoint(worldline_id, state)
    }

    /// Returns the largest checkpoint with `checkpoint.worldline_tick < tick`, if any.
    #[must_use]
    pub fn checkpoint_before(
        &self,
        worldline_id: WorldlineId,
        tick: WorldlineTick,
    ) -> Option<CheckpointRef> {
        self.store.checkpoint_before(worldline_id, tick)
    }

    /// Returns the largest materialized checkpoint with `checkpoint.worldline_tick < tick`, if any.
    #[must_use]
    pub fn checkpoint_state_before(
        &self,
        worldline_id: WorldlineId,
        tick: WorldlineTick,
    ) -> Option<ReplayCheckpoint> {
        self.store.checkpoint_state_before(worldline_id, tick)
    }

    /// Creates a new worldline that is a prefix-copy of the source up to `fork_tick`.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] if the source is missing, the target exists, or
    /// `fork_tick` is out of range.
    pub fn fork(
        &mut self,
        source: WorldlineId,
        fork_tick: WorldlineTick,
        new_id: WorldlineId,
    ) -> Result<(), HistoryError> {
        self.store.fork(source, fork_tick, new_id)
    }

    /// Returns the deterministic tip ref for a worldline, if any.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline isn't registered.
    pub fn tip_ref(
        &self,
        worldline_id: WorldlineId,
    ) -> Result<Option<ProvenanceRef>, HistoryError> {
        self.store.tip_ref(worldline_id)
    }

    /// Creates a lightweight rollback checkpoint for touched worldlines.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if any listed worldline has
    /// not been registered.
    pub fn checkpoint_for<I>(&self, worldline_ids: I) -> Result<ProvenanceCheckpoint, HistoryError>
    where
        I: IntoIterator<Item = WorldlineId>,
    {
        self.store.checkpoint_for(worldline_ids)
    }

    /// Restores touched worldlines to a previously captured rollback checkpoint.
    pub fn restore(&mut self, checkpoint: &ProvenanceCheckpoint) {
        self.store.restore(checkpoint);
    }

    /// Reconstructs full [`WorldlineState`] for a worldline up to `target_tick`.
    ///
    /// `target_tick` is a cursor coordinate: tick 0 is the replay base with no
    /// patches applied; tick N is the state after applying patches `0..N-1`.
    /// Replay restores the nearest stored checkpoint when available, hydrates
    /// snapshot/materialization metadata from authoritative provenance, then
    /// replays only the remaining patch suffix.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError`] if the supplied replay base does not match the
    /// registered worldline boundary, if any entry lacks a patch, or if
    /// reconstructed hashes differ from stored provenance commitments.
    pub fn replay_worldline_state_at(
        &self,
        worldline_id: WorldlineId,
        base_state: &WorldlineState,
        target_tick: WorldlineTick,
    ) -> Result<WorldlineState, ReplayError> {
        replay_worldline_state_at_from_provenance(
            &self.store,
            worldline_id,
            base_state,
            target_tick,
        )
    }

    /// Reconstructs full [`WorldlineState`] for a worldline from stored provenance entries.
    ///
    /// The supplied `base_state` provides the deterministic replay base for the
    /// worldline and is reset to its preserved `initial_state()` before patches
    /// are replayed. The returned state includes reconstructed `tick_history`
    /// snapshots so historical snapshot/fork paths can operate on portal and
    /// instance-bearing history without live execution.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError`] if the supplied replay base does not match the
    /// registered worldline boundary, if any entry lacks a patch, or if
    /// reconstructed hashes differ from stored provenance commitments.
    pub fn replay_worldline_state(
        &self,
        worldline_id: WorldlineId,
        base_state: &WorldlineState,
    ) -> Result<WorldlineState, ReplayError> {
        let history_len = WorldlineTick::from_raw(self.store.len(worldline_id)?);
        self.replay_worldline_state_at(worldline_id, base_state, history_len)
    }

    /// Builds a contiguous BTR from the registered provenance history.
    ///
    /// # Errors
    ///
    /// Returns [`BtrError`] if the selected range is malformed or the worldline
    /// is unknown.
    pub fn build_btr(
        &self,
        worldline_id: WorldlineId,
        start_tick: WorldlineTick,
        end_tick_exclusive: WorldlineTick,
        logical_counter: u64,
        auth_tag: Vec<u8>,
    ) -> Result<BoundaryTransitionRecord, BtrError> {
        let history_len = WorldlineTick::from_raw(self.store.len(worldline_id)?);
        if start_tick >= end_tick_exclusive || end_tick_exclusive > history_len {
            return Err(BtrError::EmptyPayload);
        }

        let entries = (start_tick.as_u64()..end_tick_exclusive.as_u64())
            .map(WorldlineTick::from_raw)
            .map(|tick| self.store.entry(worldline_id, tick))
            .collect::<Result<Vec<_>, _>>()?;
        let payload = BtrPayload {
            worldline_id,
            start_worldline_tick: start_tick,
            entries,
        };
        payload.validate()?;

        let input_boundary_hash = if start_tick == WorldlineTick::ZERO {
            self.store.initial_boundary_hash(worldline_id)?
        } else {
            self.store
                .entry(
                    worldline_id,
                    start_tick.checked_sub(1).ok_or(BtrError::TickOverflow)?,
                )?
                .expected
                .state_root
        };
        let output_boundary_hash = payload
            .entries
            .last()
            .ok_or(BtrError::EmptyPayload)?
            .expected
            .state_root;
        let record = BoundaryTransitionRecord {
            worldline_id,
            u0_ref: self.store.u0(worldline_id)?,
            input_boundary_hash,
            output_boundary_hash,
            payload,
            logical_counter,
            auth_tag,
        };
        self.validate_btr(&record)?;
        Ok(record)
    }

    /// Validates a BTR against the registered provenance history.
    ///
    /// # Errors
    ///
    /// Returns [`BtrError`] if the record is structurally invalid or does not
    /// match the registered worldline history.
    pub fn validate_btr(&self, record: &BoundaryTransitionRecord) -> Result<(), BtrError> {
        record.validate()?;

        let history = self
            .store
            .worldlines
            .get(&record.worldline_id)
            .ok_or(BtrError::UnknownWorldline(record.worldline_id))?;
        if record.u0_ref != history.u0_ref {
            return Err(BtrError::U0RefMismatch {
                expected: history.u0_ref,
                got: record.u0_ref,
            });
        }

        let expected_input = if record.payload.start_worldline_tick == WorldlineTick::ZERO {
            history.initial_boundary_hash
        } else {
            self.store
                .entry(
                    record.worldline_id,
                    record
                        .payload
                        .start_worldline_tick
                        .checked_sub(1)
                        .ok_or(BtrError::TickOverflow)?,
                )?
                .expected
                .state_root
        };
        if record.input_boundary_hash != expected_input {
            return Err(BtrError::InputBoundaryHashMismatch {
                expected: expected_input,
                got: record.input_boundary_hash,
            });
        }

        let expected_output = record
            .payload
            .entries
            .last()
            .ok_or(BtrError::EmptyPayload)?
            .expected
            .state_root;
        if record.output_boundary_hash != expected_output {
            return Err(BtrError::OutputBoundaryHashMismatch {
                expected: expected_output,
                got: record.output_boundary_hash,
            });
        }

        for entry in &record.payload.entries {
            let stored = self
                .store
                .entry(record.worldline_id, entry.worldline_tick)?;
            if &stored != entry {
                return Err(BtrError::EntryMismatch {
                    tick: entry.worldline_tick,
                });
            }
        }

        Ok(())
    }
}

impl ProvenanceStore for ProvenanceService {
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError> {
        self.store.u0(w)
    }

    fn initial_boundary_hash(&self, w: WorldlineId) -> Result<Hash, HistoryError> {
        self.store.initial_boundary_hash(w)
    }

    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError> {
        self.store.len(w)
    }

    fn entry(&self, w: WorldlineId, tick: WorldlineTick) -> Result<ProvenanceEntry, HistoryError> {
        self.store.entry(w, tick)
    }

    fn parents(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Result<Vec<ProvenanceRef>, HistoryError> {
        self.store.parents(w, tick)
    }

    fn append_local_commit(&mut self, entry: ProvenanceEntry) -> Result<(), HistoryError> {
        self.store.append_local_commit(entry)
    }

    fn checkpoint_before(&self, w: WorldlineId, tick: WorldlineTick) -> Option<CheckpointRef> {
        self.store.checkpoint_before(w, tick)
    }

    fn checkpoint_state_before(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Option<ReplayCheckpoint> {
        self.store.checkpoint_state_before(w, tick)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::cast_possible_truncation)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::redundant_clone)]
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::attachment::AttachmentKey;
    use crate::graph::GraphStore;
    use crate::head::{make_head_id, WriterHeadKey};
    use crate::ident::{make_node_id, make_type_id, make_warp_id, NodeKey, WarpId};
    use crate::materialization::make_channel_id;
    use crate::record::NodeRecord;
    use crate::tick_patch::{PortalInit, SlotId, TickCommitStatus, WarpOp, WarpTickPatchV1};
    use crate::worldline::{AtomWrite, WorldlineTickHeaderV1};

    fn test_worldline_id() -> WorldlineId {
        WorldlineId([1u8; 32])
    }

    fn test_warp_id() -> WarpId {
        WarpId([2u8; 32])
    }

    fn test_head_key() -> WriterHeadKey {
        WriterHeadKey {
            worldline_id: test_worldline_id(),
            head_id: make_head_id("default"),
        }
    }

    fn wt(raw: u64) -> WorldlineTick {
        WorldlineTick::from_raw(raw)
    }

    fn gt(raw: u64) -> GlobalTick {
        GlobalTick::from_raw(raw)
    }

    fn test_patch(tick: u64) -> WorldlineTickPatchV1 {
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick: gt(tick),
                policy_id: 0,
                rule_pack_id: [0u8; 32],
                plan_digest: [0u8; 32],
                decision_digest: [0u8; 32],
                rewrites_digest: [0u8; 32],
            },
            warp_id: test_warp_id(),
            ops: vec![],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [tick as u8; 32],
        }
    }

    fn test_initial_state(warp_id: WarpId) -> WorldlineState {
        let root = make_node_id("root");
        let mut root_store = GraphStore::new(warp_id);
        root_store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("RootType"),
            },
        );
        WorldlineState::from_root_store(root_store, root).expect("test initial state")
    }

    fn test_triplet(tick: u64) -> HashTriplet {
        HashTriplet {
            state_root: [tick as u8; 32],
            patch_digest: [(tick + 1) as u8; 32],
            commit_hash: [(tick + 2) as u8; 32],
        }
    }

    fn test_node_key() -> NodeKey {
        NodeKey {
            warp_id: test_warp_id(),
            local_id: make_node_id("test-atom"),
        }
    }

    fn test_rule_id() -> [u8; 32] {
        [42u8; 32]
    }

    fn test_entry(tick: u64) -> ProvenanceEntry {
        ProvenanceEntry::local_commit(
            test_worldline_id(),
            wt(tick),
            gt(tick),
            test_head_key(),
            if tick == 0 {
                Vec::new()
            } else {
                vec![ProvenanceRef {
                    worldline_id: test_worldline_id(),
                    worldline_tick: wt(tick - 1),
                    commit_hash: test_triplet(tick - 1).commit_hash,
                }]
            },
            test_triplet(tick),
            test_patch(tick),
            vec![],
            Vec::new(),
        )
    }

    fn append_test_entry(
        store: &mut LocalProvenanceStore,
        worldline_id: WorldlineId,
        patch: WorldlineTickPatchV1,
        expected: HashTriplet,
        outputs: OutputFrameSet,
        atom_writes: AtomWriteSet,
    ) {
        let commit_global_tick = patch.commit_global_tick();
        let worldline_tick = wt(commit_global_tick.as_u64());
        let parents = store
            .tip_ref(worldline_id)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        let entry = ProvenanceEntry::local_commit(
            worldline_id,
            worldline_tick,
            commit_global_tick,
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("fixture"),
            },
            parents,
            expected,
            patch,
            outputs,
            atom_writes,
        );
        store.append_local_commit(entry).unwrap();
    }

    fn test_patch_with_atom_slots(tick: u64, atoms: &[NodeKey]) -> WorldlineTickPatchV1 {
        let mut patch = test_patch(tick);
        for atom in atoms {
            patch
                .out_slots
                .push(SlotId::Attachment(AttachmentKey::node_alpha(*atom)));
        }
        patch
    }

    fn test_patch_with_atom_mutation(tick: u64, atoms: &[NodeKey]) -> WorldlineTickPatchV1 {
        let mut patch = test_patch(tick);
        for atom in atoms {
            let slot = SlotId::Attachment(AttachmentKey::node_alpha(*atom));
            patch.in_slots.push(slot);
            patch.out_slots.push(slot);
        }
        patch
    }

    fn test_patch_with_node_slots(tick: u64, atoms: &[NodeKey]) -> WorldlineTickPatchV1 {
        let mut patch = test_patch(tick);
        for atom in atoms {
            patch.out_slots.push(SlotId::Node(*atom));
        }
        patch
    }

    fn make_replay_patch(
        commit_global_tick: GlobalTick,
        warp_id: WarpId,
        ops: Vec<WarpOp>,
        in_slots: Vec<SlotId>,
        out_slots: Vec<SlotId>,
    ) -> WorldlineTickPatchV1 {
        let policy_id = crate::POLICY_ID_NO_POLICY_V0;
        let rule_pack_id = [0xabu8; 32];
        let replay_patch = WarpTickPatchV1::new(
            policy_id,
            rule_pack_id,
            TickCommitStatus::Committed,
            in_slots.clone(),
            out_slots.clone(),
            ops.clone(),
        );
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick,
                policy_id,
                rule_pack_id,
                plan_digest: [commit_global_tick.as_u64() as u8; 32],
                decision_digest: [commit_global_tick.as_u64() as u8 + 1; 32],
                rewrites_digest: [commit_global_tick.as_u64() as u8 + 2; 32],
            },
            warp_id,
            ops,
            in_slots,
            out_slots,
            patch_digest: replay_patch.digest(),
        }
    }

    fn replay_entry_from_patch(
        worldline_id: WorldlineId,
        worldline_tick: WorldlineTick,
        head_key: WriterHeadKey,
        parents: Vec<ProvenanceRef>,
        prior_state: &WorldlineState,
        patch: WorldlineTickPatchV1,
    ) -> (ProvenanceEntry, WorldlineState) {
        let mut next_state = prior_state.clone();
        patch
            .apply_to_worldline_state(&mut next_state)
            .expect("fixture patch should apply");
        let state_root =
            compute_state_root_for_warp_state(next_state.warp_state(), next_state.root());
        let parent_hashes = parents
            .iter()
            .map(|parent| parent.commit_hash)
            .collect::<Vec<_>>();
        let commit_hash = compute_commit_hash_v2(
            &state_root,
            &parent_hashes,
            &patch.patch_digest,
            patch.header.policy_id,
        );
        (
            ProvenanceEntry::local_commit(
                worldline_id,
                worldline_tick,
                patch.commit_global_tick(),
                head_key,
                parents,
                HashTriplet {
                    state_root,
                    patch_digest: patch.patch_digest,
                    commit_hash,
                },
                patch,
                Vec::new(),
                Vec::new(),
            ),
            next_state,
        )
    }

    #[test]
    fn worldline_not_found() {
        let store = LocalProvenanceStore::new();
        let result = store.u0(test_worldline_id());
        assert!(matches!(result, Err(HistoryError::WorldlineNotFound(_))));
    }

    #[test]
    fn register_and_query_u0() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let warp = test_warp_id();

        store.register_worldline(w, warp).unwrap();

        assert_eq!(store.u0(w).unwrap(), warp);
        assert_eq!(store.len(w).unwrap(), 0);
        assert!(store.is_empty(w).unwrap());
    }

    #[test]
    fn append_and_query_entry_api() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        let entry = test_entry(0);
        store.append_local_commit(entry.clone()).unwrap();

        assert_eq!(store.len(w).unwrap(), 1);
        assert_eq!(store.entry(w, wt(0)).unwrap(), entry);
        assert!(store.parents(w, wt(0)).unwrap().is_empty());
    }

    #[test]
    fn entry_round_trips_patch_expected_outputs() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let outputs = vec![(make_channel_id("test:ok"), b"ok".to_vec())];

        store.register_worldline(w, test_warp_id()).unwrap();
        append_test_entry(
            &mut store,
            w,
            test_patch(0),
            test_triplet(0),
            outputs.clone(),
            Vec::new(),
        );

        let entry = store.entry(w, wt(0)).unwrap();
        assert_eq!(entry.patch.unwrap().commit_global_tick(), gt(0));
        assert_eq!(entry.expected, test_triplet(0));
        assert_eq!(entry.outputs, outputs);
    }

    #[test]
    fn history_unavailable_for_missing_tick() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();
        store.append_local_commit(test_entry(0)).unwrap();

        let result = store.entry(w, wt(1));
        assert!(matches!(
            result,
            Err(HistoryError::HistoryUnavailable { tick }) if tick == wt(1)
        ));
    }

    #[test]
    fn append_tick_gap_returns_error() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        store.append_local_commit(test_entry(0)).unwrap();
        let result = store.append_local_commit(test_entry(2));
        assert!(matches!(
            result,
            Err(HistoryError::TickGap {
                expected,
                got
            }) if expected == wt(1) && got == wt(2)
        ));
    }

    #[test]
    fn append_local_commit_rejects_missing_head_key() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        let mut entry = test_entry(0);
        entry.head_key = None;
        let result = store.append_local_commit(entry);
        assert!(matches!(
            result,
            Err(HistoryError::LocalCommitMissingHeadKey { tick }) if tick == wt(0)
        ));
    }

    #[test]
    fn append_local_commit_rejects_non_local_event_kind() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        let mut entry = test_entry(0);
        entry.event_kind = ProvenanceEventKind::ConflictArtifact {
            artifact_id: [9u8; 32],
        };
        let result = store.append_local_commit(entry);
        assert!(matches!(
            result,
            Err(HistoryError::InvalidLocalCommitEventKind {
                tick,
                got: ProvenanceEventKind::ConflictArtifact { .. }
            }) if tick == wt(0)
        ));
    }

    #[test]
    fn append_local_commit_rejects_missing_parent_ref() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        let mut entry = test_entry(0);
        let missing_parent = ProvenanceRef {
            worldline_id: w,
            worldline_tick: wt(99),
            commit_hash: [7u8; 32],
        };
        entry.parents = vec![missing_parent];

        let result = store.append_local_commit(entry);
        assert!(matches!(
            result,
            Err(HistoryError::MissingParentRef { tick, parent })
                if tick == wt(0) && parent == missing_parent
        ));
    }

    #[test]
    fn append_local_commit_rejects_parent_commit_hash_mismatch() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();
        store.append_local_commit(test_entry(0)).unwrap();

        let mut entry = test_entry(1);
        let bad_parent = ProvenanceRef {
            worldline_id: w,
            worldline_tick: wt(0),
            commit_hash: [8u8; 32],
        };
        entry.parents = vec![bad_parent];

        let result = store.append_local_commit(entry);
        assert!(matches!(
            result,
            Err(HistoryError::ParentCommitHashMismatch {
                tick,
                parent,
                stored_commit_hash
            }) if tick == wt(1)
                && parent == bad_parent
                && stored_commit_hash == test_triplet(0).commit_hash
        ));
    }

    #[test]
    fn checkpoint_before() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let initial_state = test_initial_state(test_warp_id());
        let root = *initial_state.root();
        let initial_boundary =
            compute_state_root_for_warp_state(initial_state.initial_state(), initial_state.root());
        store
            .register_worldline_with_boundary(w, root.warp_id, initial_boundary)
            .unwrap();

        store
            .add_checkpoint(
                w,
                ReplayCheckpoint {
                    checkpoint: CheckpointRef {
                        worldline_tick: wt(0),
                        state_hash: initial_state.state_root(),
                    },
                    state: initial_state.replay_checkpoint_clone(),
                },
            )
            .unwrap();

        let mut parents = Vec::new();
        let mut current_state = initial_state.clone();
        let mut state_at_five = None;
        for tick in 0..10 {
            let patch = make_replay_patch(
                gt(tick + 1),
                root.warp_id,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            let (entry, next_state) = replay_entry_from_patch(
                w,
                wt(tick),
                test_head_key(),
                parents,
                &current_state,
                patch,
            );
            parents = vec![entry.as_ref()];
            current_state = next_state;
            store.append_local_commit(entry).unwrap();
            if tick == 4 {
                state_at_five = Some(current_state.clone());
            }
        }

        store
            .add_checkpoint(
                w,
                ReplayCheckpoint {
                    checkpoint: CheckpointRef {
                        worldline_tick: wt(5),
                        state_hash: state_at_five
                            .as_ref()
                            .expect("tick-5 state should be captured")
                            .state_root(),
                    },
                    state: state_at_five
                        .expect("tick-5 state should be captured")
                        .replay_checkpoint_clone(),
                },
            )
            .unwrap();
        store
            .add_checkpoint(
                w,
                ReplayCheckpoint {
                    checkpoint: CheckpointRef {
                        worldline_tick: wt(10),
                        state_hash: current_state.state_root(),
                    },
                    state: current_state.replay_checkpoint_clone(),
                },
            )
            .unwrap();

        assert!(store.checkpoint_before(w, wt(0)).is_none());
        assert_eq!(
            store.checkpoint_before(w, wt(1)).unwrap().worldline_tick,
            wt(0)
        );
        assert_eq!(
            store.checkpoint_before(w, wt(7)).unwrap().worldline_tick,
            wt(5)
        );
        assert_eq!(
            store.checkpoint_before(w, wt(15)).unwrap().worldline_tick,
            wt(10)
        );
        assert_eq!(
            store.checkpoint_before(w, wt(10)).unwrap().worldline_tick,
            wt(5)
        );
    }

    #[test]
    fn checkpoint_convenience_records_and_is_visible() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let initial_state = test_initial_state(test_warp_id());
        let root = *initial_state.root();
        let initial_boundary =
            compute_state_root_for_warp_state(initial_state.initial_state(), initial_state.root());
        store
            .register_worldline_with_boundary(w, root.warp_id, initial_boundary)
            .unwrap();
        let mut parents = Vec::new();
        let mut current_state = initial_state.clone();
        for tick in 0..5 {
            let patch = make_replay_patch(
                gt(tick + 1),
                root.warp_id,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            let (entry, next_state) = replay_entry_from_patch(
                w,
                wt(tick),
                test_head_key(),
                parents,
                &current_state,
                patch,
            );
            parents = vec![entry.as_ref()];
            current_state = next_state;
            store.append_local_commit(entry).unwrap();
        }
        let state =
            replay_worldline_state_at_from_provenance(&store, w, &initial_state, wt(5)).unwrap();
        let cp = store.checkpoint(w, &state).unwrap();
        let found = store.checkpoint_before(w, wt(6));
        assert_eq!(found.unwrap().worldline_tick, wt(5));
        assert_eq!(found.unwrap().state_hash, cp.state_hash);
    }

    #[test]
    fn add_checkpoint_rejects_wrong_root_warp() {
        let mut store = LocalProvenanceStore::new();
        let worldline_id = test_worldline_id();
        store
            .register_worldline(worldline_id, test_warp_id())
            .unwrap();

        let wrong_root_warp = make_warp_id("checkpoint-wrong-root");
        let wrong_root_node = make_node_id("checkpoint-wrong-root-node");
        let mut graph = GraphStore::new(wrong_root_warp);
        graph.insert_node(
            wrong_root_node,
            NodeRecord {
                ty: make_type_id("CheckpointWrongRoot"),
            },
        );
        let wrong_state = WorldlineState::from_root_store(graph, wrong_root_node)
            .expect("wrong-root state should still be constructible");

        let result = store.add_checkpoint(worldline_id, ReplayCheckpoint::from_state(&wrong_state));
        assert!(
            matches!(
                result,
                Err(HistoryError::CheckpointRootWarpMismatch {
                    worldline_id: got_worldline,
                    expected,
                    actual,
                }) if got_worldline == worldline_id
                    && expected == test_warp_id()
                    && actual == wrong_root_warp
            ),
            "expected checkpoint root warp mismatch, got {result:?}"
        );
    }

    #[test]
    fn fork_copies_entry_prefix_and_checkpoints() {
        let mut store = LocalProvenanceStore::new();
        let source = test_worldline_id();
        let target = WorldlineId([99u8; 32]);
        let initial_state = test_initial_state(test_warp_id());
        let root = *initial_state.root();
        let initial_boundary =
            compute_state_root_for_warp_state(initial_state.initial_state(), initial_state.root());

        store
            .register_worldline_with_boundary(source, root.warp_id, initial_boundary)
            .unwrap();
        let (entry0, state1) = replay_entry_from_patch(
            source,
            wt(0),
            test_head_key(),
            Vec::new(),
            &initial_state,
            make_replay_patch(gt(1), root.warp_id, Vec::new(), Vec::new(), Vec::new()),
        );
        let entry0_expected = entry0.expected;
        let entry0_ref = entry0.as_ref();
        store.append_local_commit(entry0).unwrap();
        let (entry1, _state2) = replay_entry_from_patch(
            source,
            wt(1),
            test_head_key(),
            vec![entry0_ref],
            &state1,
            make_replay_patch(gt(2), root.warp_id, Vec::new(), Vec::new(), Vec::new()),
        );
        store.append_local_commit(entry1).unwrap();
        let checkpoint_state =
            replay_worldline_state_at_from_provenance(&store, source, &initial_state, wt(1))
                .unwrap();
        store.checkpoint(source, &checkpoint_state).unwrap();

        store.fork(source, wt(0), target).unwrap();
        assert_eq!(store.len(target).unwrap(), 1);
        let forked_entry = store.entry(target, wt(0)).unwrap();
        assert_eq!(forked_entry.worldline_id, target);
        assert_eq!(forked_entry.head_key.unwrap().worldline_id, target);
        assert_eq!(forked_entry.expected, entry0_expected);
        let checkpoint = store
            .checkpoint_before(target, wt(2))
            .expect("fork should retain the tip checkpoint");
        assert_eq!(checkpoint.worldline_tick, wt(1));
    }

    #[test]
    fn fork_rewrites_same_worldline_parent_refs_to_target_worldline() {
        let mut store = LocalProvenanceStore::new();
        let source = test_worldline_id();
        let target = WorldlineId([99u8; 32]);
        let warp = test_warp_id();

        store.register_worldline(source, warp).unwrap();
        store.append_local_commit(test_entry(0)).unwrap();
        store.append_local_commit(test_entry(1)).unwrap();

        store.fork(source, wt(1), target).unwrap();

        let forked_entry = store.entry(target, wt(1)).unwrap();
        assert_eq!(
            forked_entry.parents,
            vec![ProvenanceRef {
                worldline_id: target,
                worldline_tick: wt(0),
                commit_hash: test_triplet(0).commit_hash,
            }]
        );
    }

    #[test]
    fn append_with_writes_stores_atom_writes() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        let atom_write = AtomWrite::new(test_node_key(), test_rule_id(), 0, None, vec![1, 2, 3]);

        append_test_entry(
            &mut store,
            w,
            test_patch_with_atom_slots(0, &[test_node_key()]),
            test_triplet(0),
            vec![],
            vec![atom_write.clone()],
        );

        let writes = store.atom_writes(w, wt(0)).unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0], atom_write);
    }

    #[test]
    fn atom_history_walks_causal_cone() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        let atom_key = test_node_key();
        let write0 = AtomWrite::new(atom_key, test_rule_id(), 0, None, vec![1]);
        let write1 = AtomWrite::new(atom_key, test_rule_id(), 1, Some(vec![1]), vec![2]);
        let write2 = AtomWrite::new(atom_key, test_rule_id(), 2, Some(vec![2]), vec![3]);

        append_test_entry(
            &mut store,
            w,
            test_patch_with_atom_slots(0, &[atom_key]),
            test_triplet(0),
            vec![],
            vec![write0.clone()],
        );
        append_test_entry(
            &mut store,
            w,
            test_patch_with_atom_mutation(1, &[atom_key]),
            test_triplet(1),
            vec![],
            vec![write1.clone()],
        );
        append_test_entry(
            &mut store,
            w,
            test_patch_with_atom_mutation(2, &[atom_key]),
            test_triplet(2),
            vec![],
            vec![write2.clone()],
        );

        let history = store.atom_history(w, &atom_key).unwrap();
        assert_eq!(history, vec![write0, write1, write2]);
    }

    #[test]
    fn atom_history_via_node_slot() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        let atom = test_node_key();
        let create_write = AtomWrite::new(atom, test_rule_id(), 0, None, vec![1]);
        let mutate_write = AtomWrite::new(atom, test_rule_id(), 1, Some(vec![1]), vec![2]);

        append_test_entry(
            &mut store,
            w,
            test_patch_with_node_slots(0, &[atom]),
            test_triplet(0),
            vec![],
            vec![create_write.clone()],
        );
        append_test_entry(
            &mut store,
            w,
            test_patch_with_node_slots(1, &[atom]),
            test_triplet(1),
            vec![],
            vec![mutate_write.clone()],
        );

        let history = store.atom_history(w, &atom).unwrap();
        assert_eq!(history, vec![create_write, mutate_write]);
    }

    #[test]
    fn build_btr_round_trips_contiguous_segment() {
        let mut service = ProvenanceService::new();
        let w = test_worldline_id();
        let state = WorldlineState::empty();
        service.register_worldline(w, &state).unwrap();
        service.append_local_commit(test_entry(0)).unwrap();
        service.append_local_commit(test_entry(1)).unwrap();

        let btr = service
            .build_btr(w, wt(0), wt(2), 7, b"auth".to_vec())
            .unwrap();
        assert_eq!(btr.logical_counter, 7);
        assert_eq!(btr.payload.start_worldline_tick, wt(0));
        assert_eq!(btr.payload.entries.len(), 2);
        service.validate_btr(&btr).unwrap();
    }

    #[test]
    fn build_btr_preserves_parent_refs_and_head_attribution() {
        let mut service = ProvenanceService::new();
        let w = test_worldline_id();
        let state = WorldlineState::empty();
        service.register_worldline(w, &state).unwrap();

        let first = test_entry(0);
        let first_ref = first.as_ref();
        let second = ProvenanceEntry::local_commit(
            w,
            wt(1),
            gt(1),
            test_head_key(),
            vec![first_ref],
            test_triplet(1),
            test_patch(1),
            vec![(make_channel_id("test:ok"), b"ok".to_vec())],
            Vec::new(),
        );

        service.append_local_commit(first).unwrap();
        service.append_local_commit(second.clone()).unwrap();

        let btr = service
            .build_btr(w, wt(0), wt(2), 9, b"auth".to_vec())
            .unwrap();
        assert_eq!(btr.logical_counter, 9);
        assert_eq!(btr.payload.entries[1].head_key, Some(test_head_key()));
        assert_eq!(btr.payload.entries[1].parents, vec![first_ref]);
        assert_eq!(btr.payload.entries[1], second);
    }

    #[test]
    fn service_fork_copies_entry_prefix_and_checkpoints() {
        let mut service = ProvenanceService::new();
        let source = test_worldline_id();
        let target = WorldlineId([99u8; 32]);
        let state = WorldlineState::empty();
        let root = *state.root();

        service.register_worldline(source, &state).unwrap();
        let (entry0, state1) = replay_entry_from_patch(
            source,
            wt(0),
            test_head_key(),
            Vec::new(),
            &state,
            make_replay_patch(gt(1), root.warp_id, Vec::new(), Vec::new(), Vec::new()),
        );
        let entry0_expected = entry0.expected;
        let entry0_ref = entry0.as_ref();
        service.append_local_commit(entry0).unwrap();
        let (entry1, _state2) = replay_entry_from_patch(
            source,
            wt(1),
            test_head_key(),
            vec![entry0_ref],
            &state1,
            make_replay_patch(gt(2), root.warp_id, Vec::new(), Vec::new(), Vec::new()),
        );
        service.append_local_commit(entry1).unwrap();
        let checkpoint_state = service
            .replay_worldline_state_at(source, &state, wt(1))
            .unwrap();
        service.checkpoint(source, &checkpoint_state).unwrap();

        service.fork(source, wt(0), target).unwrap();

        assert_eq!(service.len(target).unwrap(), 1);
        let forked_entry = service.entry(target, wt(0)).unwrap();
        assert_eq!(forked_entry.worldline_id, target);
        assert_eq!(forked_entry.head_key.unwrap().worldline_id, target);
        assert_eq!(forked_entry.expected, entry0_expected);
        let checkpoint = service
            .checkpoint_before(target, wt(2))
            .expect("fork should retain the tip checkpoint");
        assert_eq!(checkpoint.worldline_tick, wt(1));
        service
            .build_btr(target, wt(0), wt(1), 7, b"auth".to_vec())
            .unwrap();
    }

    #[test]
    fn btr_validation_rejects_non_contiguous_ticks() {
        let payload = BtrPayload {
            worldline_id: test_worldline_id(),
            start_worldline_tick: wt(0),
            entries: vec![test_entry(0), test_entry(2)],
        };
        assert!(matches!(
            payload.validate(),
            Err(BtrError::NonContiguousTicks {
                expected,
                got
            }) if expected == wt(1) && got == wt(2)
        ));
    }

    #[test]
    fn btr_validation_rejects_tick_overflow() {
        let head_key = WriterHeadKey {
            worldline_id: test_worldline_id(),
            head_id: make_head_id("overflow"),
        };
        let payload = BtrPayload {
            worldline_id: test_worldline_id(),
            start_worldline_tick: wt(u64::MAX),
            entries: vec![ProvenanceEntry::local_commit(
                test_worldline_id(),
                wt(u64::MAX),
                gt(0),
                head_key,
                Vec::new(),
                test_triplet(0),
                test_patch(0),
                Vec::new(),
                Vec::new(),
            )],
        };
        assert!(matches!(
            payload.end_tick_exclusive(),
            Err(BtrError::TickOverflow)
        ));
        assert!(matches!(payload.validate(), Err(BtrError::TickOverflow)));
    }

    #[test]
    fn validate_btr_rejects_bad_input_boundary_hash() {
        let mut service = ProvenanceService::new();
        let w = test_worldline_id();
        let state = WorldlineState::empty();
        service.register_worldline(w, &state).unwrap();
        service.append_local_commit(test_entry(0)).unwrap();

        let mut btr = service
            .build_btr(w, wt(0), wt(1), 3, b"auth".to_vec())
            .unwrap();
        btr.input_boundary_hash = [9u8; 32];

        assert!(matches!(
            service.validate_btr(&btr),
            Err(BtrError::InputBoundaryHashMismatch { .. })
        ));
    }

    #[test]
    fn validate_btr_rejects_bad_output_boundary_hash() {
        let mut service = ProvenanceService::new();
        let w = test_worldline_id();
        let state = WorldlineState::empty();
        service.register_worldline(w, &state).unwrap();
        service.append_local_commit(test_entry(0)).unwrap();

        let mut btr = service
            .build_btr(w, wt(0), wt(1), 3, b"auth".to_vec())
            .unwrap();
        btr.output_boundary_hash = [7u8; 32];

        assert!(matches!(
            service.validate_btr(&btr),
            Err(BtrError::OutputBoundaryHashMismatch { .. })
        ));
    }

    #[test]
    fn validate_btr_rejects_payload_entry_mismatch() {
        let mut service = ProvenanceService::new();
        let w = test_worldline_id();
        let state = WorldlineState::empty();
        service.register_worldline(w, &state).unwrap();
        service.append_local_commit(test_entry(0)).unwrap();

        let mut btr = service
            .build_btr(w, wt(0), wt(1), 3, b"auth".to_vec())
            .unwrap();
        btr.payload.entries[0].head_key = Some(WriterHeadKey {
            worldline_id: w,
            head_id: make_head_id("mismatch"),
        });

        assert!(matches!(
            service.validate_btr(&btr),
            Err(BtrError::EntryMismatch { tick }) if tick == wt(0)
        ));
    }

    #[test]
    fn replay_worldline_state_rebuilds_portal_and_instance_history() {
        let mut service = ProvenanceService::new();
        let worldline_id = test_worldline_id();
        let head_key = test_head_key();
        let base_state = WorldlineState::empty();
        let root = *base_state.root();
        let child_warp = make_warp_id("replay-child");
        let child_root = make_node_id("replay-child-root");
        let portal_key = AttachmentKey::node_alpha(root);

        service
            .register_worldline(worldline_id, &base_state)
            .unwrap();

        let open_patch = make_replay_patch(
            gt(1),
            root.warp_id,
            vec![WarpOp::OpenPortal {
                key: portal_key,
                child_warp,
                child_root,
                init: PortalInit::Empty {
                    root_record: crate::record::NodeRecord {
                        ty: make_type_id("ChildRootTy"),
                    },
                },
            }],
            Vec::new(),
            vec![SlotId::Attachment(portal_key)],
        );
        let (entry0, after_open) = replay_entry_from_patch(
            worldline_id,
            wt(0),
            head_key,
            Vec::new(),
            &base_state,
            open_patch,
        );
        let entry0_ref = entry0.as_ref();
        service.append_local_commit(entry0.clone()).unwrap();

        let close_patch = make_replay_patch(
            gt(2),
            root.warp_id,
            vec![
                WarpOp::DeleteWarpInstance {
                    warp_id: child_warp,
                },
                WarpOp::SetAttachment {
                    key: portal_key,
                    value: None,
                },
            ],
            Vec::new(),
            vec![SlotId::Attachment(portal_key)],
        );
        let (entry1, _final_state) = replay_entry_from_patch(
            worldline_id,
            wt(1),
            head_key,
            vec![entry0_ref],
            &after_open,
            close_patch,
        );
        service.append_local_commit(entry1.clone()).unwrap();

        let replayed = service
            .replay_worldline_state(worldline_id, &base_state)
            .expect("replay should succeed");

        assert_eq!(replayed.current_tick(), wt(2));
        assert_eq!(replayed.tick_history().len(), 2);
        assert_eq!(
            replayed.last_snapshot().map(|snapshot| snapshot.hash),
            Some(entry1.expected.commit_hash)
        );
        assert_eq!(
            replayed.tick_history()[0].0.state_root,
            entry0.expected.state_root
        );
        assert!(replayed.warp_state().instance(&child_warp).is_none());
        assert!(replayed.warp_state().store(&child_warp).is_none());

        let root_store = replayed
            .warp_state()
            .store(&root.warp_id)
            .expect("root store missing");
        assert!(root_store.node_attachment(&root.local_id).is_none());

        let engine = crate::Engine::new(root_store.clone(), replayed.root().local_id);
        let snapshot0 = engine
            .snapshot_at_state(&replayed, 0)
            .expect("stored snapshot should exist");
        assert_eq!(snapshot0.hash, entry0.expected.commit_hash);
        assert_eq!(snapshot0.state_root, entry0.expected.state_root);
    }

    #[test]
    fn replay_worldline_state_at_hydrates_exact_checkpoint_metadata() {
        let mut service = ProvenanceService::new();
        let worldline_id = test_worldline_id();
        let head_key = test_head_key();
        let base_state = WorldlineState::empty();
        let root = *base_state.root();

        service
            .register_worldline(worldline_id, &base_state)
            .unwrap();

        let patch = make_replay_patch(
            gt(1),
            root.warp_id,
            vec![WarpOp::UpsertNode {
                node: root,
                record: crate::record::NodeRecord {
                    ty: make_type_id("ReplayCheckpointRoot"),
                },
            }],
            Vec::new(),
            Vec::new(),
        );
        let (entry0, mut checkpoint_state) = replay_entry_from_patch(
            worldline_id,
            wt(0),
            head_key,
            Vec::new(),
            &base_state,
            patch,
        );
        let (snapshot, receipt, replay_patch) = replay_artifacts_for_entry(
            root,
            &entry0,
            entry0
                .patch
                .as_ref()
                .expect("fixture entry should carry a replay patch"),
        )
        .expect("fixture replay artifacts");
        let mut poisoned_snapshot = snapshot;
        poisoned_snapshot.hash = [0xAA; 32];
        checkpoint_state.last_snapshot = Some(poisoned_snapshot.clone());
        checkpoint_state.tick_history = vec![(poisoned_snapshot, receipt, replay_patch)];
        service.append_local_commit(entry0.clone()).unwrap();
        service
            .add_checkpoint(
                worldline_id,
                ReplayCheckpoint {
                    checkpoint: CheckpointRef {
                        worldline_tick: wt(1),
                        state_hash: entry0.expected.state_root,
                    },
                    state: checkpoint_state,
                },
            )
            .unwrap();

        let replayed = service
            .replay_worldline_state_at(worldline_id, &base_state, wt(1))
            .expect("checkpoint replay should succeed");

        assert_eq!(replayed.current_tick(), wt(1));
        assert_eq!(replayed.tick_history().len(), 1);
        assert_eq!(
            replayed.last_snapshot().map(|snapshot| snapshot.hash),
            Some(entry0.expected.commit_hash)
        );
        assert_eq!(
            replayed.tick_history()[0].0.hash,
            entry0.expected.commit_hash,
            "checkpoint-carried tick history must be rebuilt from provenance"
        );

        let engine = crate::Engine::new(
            replayed
                .store(&root.warp_id)
                .expect("root store missing")
                .clone(),
            root.local_id,
        );
        let snapshot0 = engine
            .snapshot_at_state(&replayed, 0)
            .expect("stored snapshot should exist");
        assert_eq!(snapshot0.hash, entry0.expected.commit_hash);
        assert_eq!(snapshot0.state_root, entry0.expected.state_root);
    }

    #[test]
    fn replay_worldline_state_at_restores_materialization_from_recorded_outputs() {
        let mut service = ProvenanceService::new();
        let worldline_id = test_worldline_id();
        let head_key = test_head_key();
        let base_state = WorldlineState::empty();
        let root = *base_state.root();
        let output_channel = make_channel_id("replay:checkpoint-output");
        let output_bytes = vec![0xAB, 0xCD, 0xEF];

        service
            .register_worldline(worldline_id, &base_state)
            .unwrap();

        let patch = make_replay_patch(
            gt(1),
            root.warp_id,
            vec![WarpOp::UpsertNode {
                node: root,
                record: crate::record::NodeRecord {
                    ty: make_type_id("ReplayOutputRoot"),
                },
            }],
            Vec::new(),
            Vec::new(),
        );

        let mut next_state = base_state.clone();
        patch
            .apply_to_worldline_state(&mut next_state)
            .expect("fixture patch should apply");
        let state_root =
            compute_state_root_for_warp_state(next_state.warp_state(), next_state.root());
        let commit_hash = compute_commit_hash_v2(
            &state_root,
            &[],
            &patch.patch_digest,
            patch.header.policy_id,
        );
        let entry = ProvenanceEntry::local_commit(
            worldline_id,
            wt(0),
            patch.commit_global_tick(),
            head_key,
            Vec::new(),
            HashTriplet {
                state_root,
                patch_digest: patch.patch_digest,
                commit_hash,
            },
            patch,
            vec![(output_channel, output_bytes.clone())],
            Vec::new(),
        );
        service.append_local_commit(entry.clone()).unwrap();
        service
            .add_checkpoint(
                worldline_id,
                ReplayCheckpoint {
                    checkpoint: CheckpointRef {
                        worldline_tick: wt(1),
                        state_hash: entry.expected.state_root,
                    },
                    state: next_state,
                },
            )
            .unwrap();

        let replayed = service
            .replay_worldline_state_at(worldline_id, &base_state, wt(1))
            .expect("checkpoint replay should succeed");

        assert_eq!(replayed.last_materialization().len(), 1);
        assert_eq!(replayed.last_materialization()[0].channel, output_channel);
        assert_eq!(replayed.last_materialization()[0].data, output_bytes);
        assert!(replayed.last_materialization_errors().is_empty());
    }

    #[test]
    fn add_checkpoint_rejects_checkpoint_that_disagrees_with_provenance() {
        let mut service = ProvenanceService::new();
        let worldline_id = test_worldline_id();
        let head_key = test_head_key();
        let base_state = WorldlineState::empty();
        let root = *base_state.root();

        service
            .register_worldline(worldline_id, &base_state)
            .unwrap();

        let patch = make_replay_patch(
            gt(1),
            root.warp_id,
            vec![WarpOp::UpsertNode {
                node: root,
                record: crate::record::NodeRecord {
                    ty: make_type_id("ReplayCheckpointMismatch"),
                },
            }],
            Vec::new(),
            Vec::new(),
        );
        let (entry0, _checkpoint_state) = replay_entry_from_patch(
            worldline_id,
            wt(0),
            head_key,
            Vec::new(),
            &base_state,
            patch,
        );
        service.append_local_commit(entry0).unwrap();
        assert!(
            matches!(
                service.add_checkpoint(
                    worldline_id,
                    ReplayCheckpoint {
                        checkpoint: CheckpointRef {
                            worldline_tick: wt(1),
                            state_hash: base_state.state_root(),
                        },
                        state: base_state.replay_checkpoint_clone(),
                    },
                ),
                Err(HistoryError::CheckpointStateRootMismatch { tick, .. }) if tick == wt(1)
            ),
            "expected authoritative checkpoint write rejection"
        );
    }

    #[test]
    fn replay_worldline_state_rejects_wrong_initial_boundary() {
        let mut service = ProvenanceService::new();
        let worldline_id = test_worldline_id();
        let base_state = WorldlineState::empty();

        service
            .register_worldline(worldline_id, &base_state)
            .unwrap();

        let mut wrong_base = WorldlineState::empty();
        let wrong_root_warp = wrong_base.root().warp_id;
        let wrong_root_node = wrong_base.root().local_id;
        wrong_base
            .initial_state
            .store_mut(&wrong_root_warp)
            .expect("root store missing")
            .insert_node(
                wrong_root_node,
                crate::record::NodeRecord {
                    ty: make_type_id("WrongBase"),
                },
            );

        let result = service.replay_worldline_state(worldline_id, &wrong_base);
        assert!(matches!(
            result,
            Err(ReplayError::InitialBoundaryHashMismatch { .. })
        ));
    }

    #[test]
    fn btr_validation_rejects_mixed_worldlines() {
        let entry = ProvenanceEntry::local_commit(
            WorldlineId([2u8; 32]),
            wt(0),
            gt(0),
            WriterHeadKey {
                worldline_id: WorldlineId([2u8; 32]),
                head_id: make_head_id("b"),
            },
            Vec::new(),
            test_triplet(0),
            test_patch(0),
            Vec::new(),
            Vec::new(),
        );
        let payload = BtrPayload {
            worldline_id: test_worldline_id(),
            start_worldline_tick: wt(0),
            entries: vec![entry],
        };
        assert!(matches!(
            payload.validate(),
            Err(BtrError::MixedWorldline { .. })
        ));
    }
}
