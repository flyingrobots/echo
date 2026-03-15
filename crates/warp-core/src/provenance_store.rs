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

#![allow(clippy::cast_possible_truncation)]

use std::collections::BTreeMap;

use thiserror::Error;

use crate::graph::GraphStore;
use crate::head::WriterHeadKey;
use crate::ident::{Hash, WarpId};
use crate::snapshot::{compute_state_root_for_warp_state, compute_state_root_for_warp_store};
use crate::worldline_state::WorldlineState;

use super::worldline::{
    AtomWrite, AtomWriteSet, HashTriplet, OutputFrameSet, WorldlineId, WorldlineTickPatchV1,
};

/// Errors that can occur when accessing worldline history.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HistoryError {
    /// The requested tick is not available in the store.
    #[error("history unavailable for tick {tick}")]
    HistoryUnavailable {
        /// The tick that was requested but not found.
        tick: u64,
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
        expected: u64,
        /// The tick that was provided.
        got: u64,
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
        tick: u64,
    },

    /// A local commit entry must carry a replay patch.
    #[error("local commit missing patch for tick {tick}")]
    LocalCommitMissingPatch {
        /// The entry tick.
        tick: u64,
    },

    /// The local commit head must belong to the same worldline as the entry.
    #[error("local commit head/worldline mismatch: entry {entry_worldline:?}, head {head_key:?}")]
    HeadWorldlineMismatch {
        /// Worldline encoded in the entry.
        entry_worldline: WorldlineId,
        /// Head key carried by the entry.
        head_key: WriterHeadKey,
    },

    /// Parent references must already be stored in canonical commit-hash order.
    #[error("parent refs must be in canonical commit-hash order at tick {tick}")]
    NonCanonicalParents {
        /// The entry tick whose parent refs were non-canonical.
        tick: u64,
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
        expected: u64,
        /// The observed tick.
        got: u64,
    },

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
}

/// Reference to a checkpoint within the provenance store.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CheckpointRef {
    /// Tick number where this checkpoint was taken.
    pub tick: u64,
    /// Hash of the state at this checkpoint.
    pub state_hash: Hash,
}

/// Reference to a parent provenance commit.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProvenanceRef {
    /// Parent worldline.
    pub worldline_id: WorldlineId,
    /// Parent tick identity.
    pub worldline_tick: u64,
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
        source_tick: u64,
        /// Stable message id.
        message_id: Hash,
    },
    /// Placeholder for a future merge/import event.
    MergeImport {
        /// Source worldline.
        source_worldline: WorldlineId,
        /// Source tick.
        source_tick: u64,
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
    pub worldline_tick: u64,
    /// Correlation metadata from the runtime SuperTick.
    pub global_tick: u64,
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
        worldline_tick: u64,
        global_tick: u64,
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
            global_tick,
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
    pub start_tick: u64,
    /// Contiguous entries in append order.
    pub entries: Vec<ProvenanceEntry>,
}

impl BtrPayload {
    /// Returns the exclusive end tick for the payload.
    #[must_use]
    pub fn end_tick_exclusive(&self) -> u64 {
        self.start_tick + self.entries.len() as u64
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
        if first.worldline_tick != self.start_tick {
            return Err(BtrError::NonContiguousTicks {
                expected: self.start_tick,
                got: first.worldline_tick,
            });
        }

        let mut expected_tick = self.start_tick;
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
            expected_tick += 1;
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

    /// Returns the number of recorded ticks for a worldline.
    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError>;

    /// Returns the entry for a specific tick.
    fn entry(&self, w: WorldlineId, tick: u64) -> Result<ProvenanceEntry, HistoryError>;

    /// Returns the stored parent refs for a specific tick.
    fn parents(&self, w: WorldlineId, tick: u64) -> Result<Vec<ProvenanceRef>, HistoryError>;

    /// Appends a local commit entry.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] if the worldline does not exist, the tick is not
    /// append-only, or the entry violates local-commit invariants.
    fn append_local_commit(&mut self, entry: ProvenanceEntry) -> Result<(), HistoryError>;

    /// Returns the nearest checkpoint before a given tick, if any.
    fn checkpoint_before(&self, w: WorldlineId, tick: u64) -> Option<CheckpointRef>;

    /// Returns whether the worldline has any recorded history.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    fn is_empty(&self, w: WorldlineId) -> Result<bool, HistoryError> {
        Ok(self.len(w)? == 0)
    }
}

#[derive(Debug, Clone)]
struct WorldlineHistory {
    u0_ref: WarpId,
    initial_boundary_hash: Hash,
    entries: Vec<ProvenanceEntry>,
    checkpoints: Vec<CheckpointRef>,
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
        worldline_id: WorldlineId,
        expected_tick: u64,
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
        if !entry
            .parents
            .windows(2)
            .all(|pair| pair[0].commit_hash <= pair[1].commit_hash)
        {
            return Err(HistoryError::NonCanonicalParents {
                tick: entry.worldline_tick,
            });
        }
        Ok(())
    }

    /// Returns the atom writes for a specific tick.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] if the worldline or tick is unavailable.
    pub fn atom_writes(&self, w: WorldlineId, tick: u64) -> Result<AtomWriteSet, HistoryError> {
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
        checkpoint: CheckpointRef,
    ) -> Result<(), HistoryError> {
        let history = self.history_mut(w)?;
        match history
            .checkpoints
            .binary_search_by_key(&checkpoint.tick, |c| c.tick)
        {
            Ok(index) => history.checkpoints[index] = checkpoint,
            Err(pos) => history.checkpoints.insert(pos, checkpoint),
        }
        Ok(())
    }

    /// Creates a checkpoint at the given tick by computing the state hash.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn checkpoint(
        &mut self,
        w: WorldlineId,
        tick: u64,
        state: &GraphStore,
    ) -> Result<CheckpointRef, HistoryError> {
        let history = self.history_mut(w)?;
        let state_hash = compute_state_root_for_warp_store(state, history.u0_ref);
        let checkpoint_ref = CheckpointRef { tick, state_hash };
        match history
            .checkpoints
            .binary_search_by_key(&checkpoint_ref.tick, |c| c.tick)
        {
            Ok(index) => history.checkpoints[index] = checkpoint_ref,
            Err(pos) => history.checkpoints.insert(pos, checkpoint_ref),
        }
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
        fork_tick: u64,
        new_id: WorldlineId,
    ) -> Result<(), HistoryError> {
        if self.worldlines.contains_key(&new_id) {
            return Err(HistoryError::WorldlineAlreadyExists(new_id));
        }

        let source_history = self.history(source)?;
        let source_len = source_history.entries.len();
        if fork_tick >= source_len as u64 {
            return Err(HistoryError::HistoryUnavailable { tick: fork_tick });
        }

        let end_idx = (fork_tick + 1) as usize;
        let new_history = WorldlineHistory {
            u0_ref: source_history.u0_ref,
            initial_boundary_hash: source_history.initial_boundary_hash,
            entries: source_history.entries[..end_idx].to_vec(),
            checkpoints: source_history
                .checkpoints
                .iter()
                .filter(|c| c.tick <= fork_tick)
                .copied()
                .collect(),
        };
        self.worldlines.insert(new_id, new_history);
        Ok(())
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
}

impl ProvenanceStore for LocalProvenanceStore {
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError> {
        Ok(self.history(w)?.u0_ref)
    }

    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError> {
        Ok(self.history(w)?.entries.len() as u64)
    }

    fn entry(&self, w: WorldlineId, tick: u64) -> Result<ProvenanceEntry, HistoryError> {
        self.history(w)?
            .entries
            .get(tick as usize)
            .cloned()
            .ok_or(HistoryError::HistoryUnavailable { tick })
    }

    fn parents(&self, w: WorldlineId, tick: u64) -> Result<Vec<ProvenanceRef>, HistoryError> {
        Ok(self.entry(w, tick)?.parents)
    }

    fn append_local_commit(&mut self, entry: ProvenanceEntry) -> Result<(), HistoryError> {
        let history = self.history_mut(entry.worldline_id)?;
        let expected_tick = history.entries.len() as u64;
        Self::validate_local_commit_entry(entry.worldline_id, expected_tick, &entry)?;
        history.entries.push(entry);
        Ok(())
    }

    fn checkpoint_before(&self, w: WorldlineId, tick: u64) -> Option<CheckpointRef> {
        let history = self.worldlines.get(&w)?;
        let pos = history
            .checkpoints
            .binary_search_by_key(&tick, |c| c.tick)
            .unwrap_or_else(|e| e);
        if pos == 0 {
            None
        } else {
            Some(history.checkpoints[pos - 1])
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

    /// Builds a contiguous BTR from the registered provenance history.
    ///
    /// # Errors
    ///
    /// Returns [`BtrError`] if the selected range is malformed or the worldline
    /// is unknown.
    pub fn build_btr(
        &self,
        worldline_id: WorldlineId,
        start_tick: u64,
        end_tick_exclusive: u64,
        logical_counter: u64,
        auth_tag: Vec<u8>,
    ) -> Result<BoundaryTransitionRecord, BtrError> {
        let history_len = self.store.len(worldline_id)?;
        if start_tick >= end_tick_exclusive || end_tick_exclusive > history_len {
            return Err(BtrError::EmptyPayload);
        }

        let entries = (start_tick..end_tick_exclusive)
            .map(|tick| self.store.entry(worldline_id, tick))
            .collect::<Result<Vec<_>, _>>()?;
        let payload = BtrPayload {
            worldline_id,
            start_tick,
            entries,
        };
        payload.validate()?;

        let input_boundary_hash = if start_tick == 0 {
            self.store.initial_boundary_hash(worldline_id)?
        } else {
            self.store
                .entry(worldline_id, start_tick - 1)?
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

        let expected_input = if record.payload.start_tick == 0 {
            history.initial_boundary_hash
        } else {
            self.store
                .entry(record.worldline_id, record.payload.start_tick - 1)?
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
                return Err(BtrError::OutputBoundaryHashMismatch {
                    expected: stored.expected.state_root,
                    got: entry.expected.state_root,
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

    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError> {
        self.store.len(w)
    }

    fn entry(&self, w: WorldlineId, tick: u64) -> Result<ProvenanceEntry, HistoryError> {
        self.store.entry(w, tick)
    }

    fn parents(&self, w: WorldlineId, tick: u64) -> Result<Vec<ProvenanceRef>, HistoryError> {
        self.store.parents(w, tick)
    }

    fn append_local_commit(&mut self, entry: ProvenanceEntry) -> Result<(), HistoryError> {
        self.store.append_local_commit(entry)
    }

    fn checkpoint_before(&self, w: WorldlineId, tick: u64) -> Option<CheckpointRef> {
        self.store.checkpoint_before(w, tick)
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
    use crate::ident::{make_node_id, NodeKey, WarpId};
    use crate::materialization::make_channel_id;
    use crate::tick_patch::SlotId;
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

    fn test_patch(tick: u64) -> WorldlineTickPatchV1 {
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                global_tick: tick,
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
            tick,
            tick,
            test_head_key(),
            if tick == 0 {
                Vec::new()
            } else {
                vec![ProvenanceRef {
                    worldline_id: test_worldline_id(),
                    worldline_tick: tick - 1,
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
        let tick = patch.global_tick();
        let parents = store
            .tip_ref(worldline_id)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        let entry = ProvenanceEntry::local_commit(
            worldline_id,
            tick,
            tick,
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
        assert_eq!(store.entry(w, 0).unwrap(), entry);
        assert!(store.parents(w, 0).unwrap().is_empty());
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

        let entry = store.entry(w, 0).unwrap();
        assert_eq!(entry.patch.unwrap().global_tick(), 0);
        assert_eq!(entry.expected, test_triplet(0));
        assert_eq!(entry.outputs, outputs);
    }

    #[test]
    fn history_unavailable_for_missing_tick() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();
        store.append_local_commit(test_entry(0)).unwrap();

        let result = store.entry(w, 1);
        assert!(matches!(
            result,
            Err(HistoryError::HistoryUnavailable { tick: 1 })
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
                expected: 1,
                got: 2
            })
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
            Err(HistoryError::LocalCommitMissingHeadKey { tick: 0 })
        ));
    }

    #[test]
    fn checkpoint_before() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        store.register_worldline(w, test_warp_id()).unwrap();

        store
            .add_checkpoint(
                w,
                CheckpointRef {
                    tick: 0,
                    state_hash: [0u8; 32],
                },
            )
            .unwrap();
        store
            .add_checkpoint(
                w,
                CheckpointRef {
                    tick: 5,
                    state_hash: [5u8; 32],
                },
            )
            .unwrap();
        store
            .add_checkpoint(
                w,
                CheckpointRef {
                    tick: 10,
                    state_hash: [10u8; 32],
                },
            )
            .unwrap();

        assert!(store.checkpoint_before(w, 0).is_none());
        assert_eq!(store.checkpoint_before(w, 1).unwrap().tick, 0);
        assert_eq!(store.checkpoint_before(w, 7).unwrap().tick, 5);
        assert_eq!(store.checkpoint_before(w, 15).unwrap().tick, 10);
        assert_eq!(store.checkpoint_before(w, 10).unwrap().tick, 5);
    }

    #[test]
    fn checkpoint_convenience_records_and_is_visible() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let warp = test_warp_id();
        store.register_worldline(w, warp).unwrap();

        let graph_store = GraphStore::new(warp);
        let cp = store.checkpoint(w, 5, &graph_store).unwrap();
        let found = store.checkpoint_before(w, 6);
        assert_eq!(found.unwrap().tick, 5);
        assert_eq!(found.unwrap().state_hash, cp.state_hash);
    }

    #[test]
    fn fork_copies_entry_prefix_and_checkpoints() {
        let mut store = LocalProvenanceStore::new();
        let source = test_worldline_id();
        let target = WorldlineId([99u8; 32]);
        let warp = test_warp_id();

        store.register_worldline(source, warp).unwrap();
        store.append_local_commit(test_entry(0)).unwrap();
        store.append_local_commit(test_entry(1)).unwrap();
        store
            .add_checkpoint(
                source,
                CheckpointRef {
                    tick: 1,
                    state_hash: [1u8; 32],
                },
            )
            .unwrap();

        store.fork(source, 0, target).unwrap();
        assert_eq!(store.len(target).unwrap(), 1);
        assert_eq!(store.entry(target, 0).unwrap().expected, test_triplet(0));
        assert!(store.checkpoint_before(target, 1).is_none());
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

        let writes = store.atom_writes(w, 0).unwrap();
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

        let btr = service.build_btr(w, 0, 2, 7, b"auth".to_vec()).unwrap();
        assert_eq!(btr.logical_counter, 7);
        assert_eq!(btr.payload.start_tick, 0);
        assert_eq!(btr.payload.entries.len(), 2);
        service.validate_btr(&btr).unwrap();
    }

    #[test]
    fn btr_validation_rejects_mixed_worldlines() {
        let entry = ProvenanceEntry::local_commit(
            WorldlineId([2u8; 32]),
            0,
            0,
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
            start_tick: 0,
            entries: vec![entry],
        };
        assert!(matches!(
            payload.validate(),
            Err(BtrError::MixedWorldline { .. })
        ));
    }
}
