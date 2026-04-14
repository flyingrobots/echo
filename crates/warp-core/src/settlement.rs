// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Base-worldline strand settlement for Echo v1.

use blake3::Hasher;
use echo_wasm_abi::kernel_port as abi;
use thiserror::Error;

use crate::clock::{GlobalTick, WorldlineTick};
use crate::coordinator::{RuntimeError, WorldlineRuntime};
use crate::ident::Hash;
use crate::materialization::ChannelId;
use crate::provenance_store::{
    finalized_channels, replay_artifacts_for_entry, HistoryError, ProvenanceEntry,
    ProvenanceEventKind, ProvenanceRef, ProvenanceService, ProvenanceStore,
};
use crate::snapshot::{compute_commit_hash_v2, compute_state_root_for_warp_state};
use crate::strand::{StrandId, StrandRegistry};
use crate::tick_patch::{TickCommitStatus, WarpTickPatchV1};
use crate::worldline::{
    ApplyError, HashTriplet, WorldlineId, WorldlineTickHeaderV1, WorldlineTickPatchV1,
};

const CONFLICT_ARTIFACT_DOMAIN: &[u8] = b"echo:settlement-conflict-artifact:v1\0";

/// Deterministic reasons a source settlement step could not be imported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictReason {
    /// The source step depends on channel policy detail that v1 cannot import.
    ChannelPolicyConflict,
    /// The source step is not replayable under the current base-worldline law.
    UnsupportedImport,
    /// The base worldline advanced away from the strand's recorded fork coordinate.
    BaseDivergence,
    /// The source and target lanes disagree on time-quantum assumptions.
    QuantumMismatch,
}

impl ConflictReason {
    const fn code(self) -> u8 {
        match self {
            Self::ChannelPolicyConflict => 1,
            Self::UnsupportedImport => 2,
            Self::BaseDivergence => 3,
            Self::QuantumMismatch => 4,
        }
    }

    fn to_abi(self) -> abi::ConflictReason {
        match self {
            Self::ChannelPolicyConflict => abi::ConflictReason::ChannelPolicyConflict,
            Self::UnsupportedImport => abi::ConflictReason::UnsupportedImport,
            Self::BaseDivergence => abi::ConflictReason::BaseDivergence,
            Self::QuantumMismatch => abi::ConflictReason::QuantumMismatch,
        }
    }
}

/// Compare surface for one strand suffix relative to its recorded base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementDelta {
    /// Strand being compared.
    pub strand_id: StrandId,
    /// Recorded fork basis for the strand.
    pub fork_basis_ref: crate::strand::ForkBasisRef,
    /// Child worldline carrying speculative suffix history.
    pub source_lane_id: WorldlineId,
    /// First suffix tick eligible for settlement consideration.
    pub source_suffix_start_tick: WorldlineTick,
    /// Last suffix tick currently present on the source worldline.
    pub source_suffix_end_tick: WorldlineTick,
    /// Authoritative source provenance refs in settlement order.
    pub source_entries: Vec<ProvenanceRef>,
}

impl SettlementDelta {
    /// Converts the delta into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::SettlementDelta {
        abi::SettlementDelta {
            strand_id: abi::StrandId::from_bytes(*self.strand_id.as_bytes()),
            base_ref: fork_basis_ref_to_abi(self.fork_basis_ref),
            source_worldline_id: abi::WorldlineId::from_bytes(*self.source_lane_id.as_bytes()),
            source_suffix_start_tick: abi::WorldlineTick(self.source_suffix_start_tick.as_u64()),
            source_suffix_end_tick: abi::WorldlineTick(self.source_suffix_end_tick.as_u64()),
            source_entries: self
                .source_entries
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
        }
    }
}

/// Deterministic settlement evaluation for one strand against its base worldline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementPlan {
    /// Strand being settled.
    pub strand_id: StrandId,
    /// Base worldline receiving settlement output.
    pub target_worldline: WorldlineId,
    /// Provenance coordinate the strand claims as its base.
    pub target_base_ref: ProvenanceRef,
    /// Ordered import or conflict decisions for the suffix.
    pub decisions: Vec<SettlementDecision>,
}

impl SettlementPlan {
    /// Converts the plan into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::SettlementPlan {
        abi::SettlementPlan {
            strand_id: abi::StrandId::from_bytes(*self.strand_id.as_bytes()),
            target_worldline: abi::WorldlineId::from_bytes(*self.target_worldline.as_bytes()),
            target_base_ref: provenance_ref_to_abi(self.target_base_ref),
            decisions: self
                .decisions
                .iter()
                .map(SettlementDecision::to_abi)
                .collect(),
        }
    }
}

/// One deterministic settlement decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettlementDecision {
    /// Source history that can be imported into the base worldline.
    ImportCandidate(ImportCandidate),
    /// Source history that must remain explicit residue.
    ConflictArtifact(ConflictArtifactDraft),
}

impl SettlementDecision {
    fn to_abi(&self) -> abi::SettlementDecision {
        match self {
            Self::ImportCandidate(candidate) => abi::SettlementDecision::ImportCandidate {
                candidate: candidate.to_abi(),
            },
            Self::ConflictArtifact(artifact) => abi::SettlementDecision::ConflictArtifact {
                artifact: artifact.to_abi(),
            },
        }
    }
}

/// One accepted unit of source provenance eligible for import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportCandidate {
    /// Source provenance coordinate being imported.
    pub source_ref: ProvenanceRef,
    /// Source writer head when the imported entry was a local commit.
    pub source_head_key: Option<crate::head::WriterHeadKey>,
    /// Stable imported operation identifier.
    pub imported_op_id: Hash,
}

impl ImportCandidate {
    fn to_abi(&self) -> abi::ImportCandidate {
        abi::ImportCandidate {
            source_ref: provenance_ref_to_abi(self.source_ref),
            source_head_key: self.source_head_key.map(writer_head_key_to_abi),
            imported_op_id: self.imported_op_id.to_vec(),
        }
    }
}

/// One unresolved settlement residue draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictArtifactDraft {
    /// Stable artifact identifier for this residue record.
    pub artifact_id: Hash,
    /// Source provenance coordinate that could not be imported.
    pub source_ref: ProvenanceRef,
    /// Channels implicated by the unresolved source entry.
    pub channel_ids: Vec<ChannelId>,
    /// Deterministic reason the source entry was rejected.
    pub reason: ConflictReason,
}

impl ConflictArtifactDraft {
    fn to_abi(&self) -> abi::ConflictArtifactDraft {
        abi::ConflictArtifactDraft {
            artifact_id: self.artifact_id.to_vec(),
            source_ref: provenance_ref_to_abi(self.source_ref),
            channel_ids: self
                .channel_ids
                .iter()
                .map(|channel_id| channel_id.0.to_vec())
                .collect(),
            reason: self.reason.to_abi(),
        }
    }
}

/// Runtime result of executing one settlement plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementResult {
    /// Deterministic plan that was executed.
    pub plan: SettlementPlan,
    /// Target-worldline refs appended as `MergeImport`.
    pub appended_imports: Vec<ProvenanceRef>,
    /// Target-worldline refs appended as `ConflictArtifact`.
    pub appended_conflicts: Vec<ProvenanceRef>,
}

impl SettlementResult {
    /// Converts the result into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::SettlementResult {
        abi::SettlementResult {
            plan: self.plan.to_abi(),
            appended_imports: self
                .appended_imports
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
            appended_conflicts: self
                .appended_conflicts
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
        }
    }
}

fn fork_basis_ref_to_abi(fork_basis_ref: crate::strand::ForkBasisRef) -> abi::BaseRef {
    abi::BaseRef {
        source_worldline_id: abi::WorldlineId::from_bytes(
            *fork_basis_ref.source_lane_id.as_bytes(),
        ),
        fork_tick: abi::WorldlineTick(fork_basis_ref.fork_tick.as_u64()),
        commit_hash: fork_basis_ref.commit_hash.to_vec(),
        boundary_hash: fork_basis_ref.boundary_hash.to_vec(),
        provenance_ref: provenance_ref_to_abi(fork_basis_ref.provenance_ref),
    }
}

fn provenance_ref_to_abi(reference: ProvenanceRef) -> abi::ProvenanceRef {
    abi::ProvenanceRef {
        worldline_id: abi::WorldlineId::from_bytes(*reference.worldline_id.as_bytes()),
        worldline_tick: abi::WorldlineTick(reference.worldline_tick.as_u64()),
        commit_hash: reference.commit_hash.to_vec(),
    }
}

fn writer_head_key_to_abi(key: crate::head::WriterHeadKey) -> abi::WriterHeadKey {
    abi::WriterHeadKey {
        worldline_id: abi::WorldlineId::from_bytes(*key.worldline_id.as_bytes()),
        head_id: abi::HeadId::from_bytes(*key.head_id.as_bytes()),
    }
}

/// Errors surfaced while comparing, planning, or executing settlement.
#[derive(Debug, Error)]
pub enum SettlementError {
    /// The requested strand is not live in the registry.
    #[error("strand not found: {0:?}")]
    StrandNotFound(StrandId),
    /// The strand fork coordinate cannot advance to a suffix start tick.
    #[error("fork tick overflow for strand {0:?}")]
    ForkTickOverflow(StrandId),
    /// Runtime frontier state and provenance history disagree for a worldline.
    #[error("runtime/provenance drift for worldline {worldline_id:?}: frontier {frontier_tick}, provenance {provenance_len}")]
    RuntimeProvenanceDrift {
        /// Drifted worldline.
        worldline_id: WorldlineId,
        /// Frontier tick currently held in runtime memory.
        frontier_tick: WorldlineTick,
        /// Provenance history length for the same worldline.
        provenance_len: WorldlineTick,
    },
    /// A source entry claimed to be importable but carried no replay patch.
    #[error("source entry missing patch: {source_ref:?}")]
    SourceEntryMissingPatch {
        /// Source provenance coordinate missing its replay patch.
        source_ref: ProvenanceRef,
    },
    /// Applying the imported patch produced a different state than the source entry committed.
    #[error("import state root mismatch for source {source_ref:?}: expected {expected:?}, got {actual:?}")]
    ImportedStateRootMismatch {
        /// Source provenance coordinate being imported.
        source_ref: ProvenanceRef,
        /// Boxed expected state root keeps the error variant small.
        expected: Box<Hash>,
        /// Boxed actual state root keeps the error variant small.
        actual: Box<Hash>,
    },
    /// Applying a settlement patch to the target frontier failed.
    #[error("settlement apply failed on {worldline_id:?} at tick {tick}: {source}")]
    Apply {
        /// Target worldline receiving settlement output.
        worldline_id: WorldlineId,
        /// Target tick being appended.
        tick: WorldlineTick,
        /// Underlying patch apply failure.
        #[source]
        source: ApplyError,
    },
    /// Wrapped runtime error.
    #[error(transparent)]
    Runtime(#[from] Box<RuntimeError>),
    /// Wrapped provenance error.
    #[error(transparent)]
    History(#[from] HistoryError),
    /// Wrapped replay-artifact reconstruction error.
    #[error(transparent)]
    Replay(#[from] crate::provenance_store::ReplayError),
}

/// Deterministic source-basis settlement service.
pub struct SettlementService;

impl From<RuntimeError> for SettlementError {
    fn from(source: RuntimeError) -> Self {
        Self::Runtime(Box::new(source))
    }
}

struct RecordedEntryDraft {
    event_kind: ProvenanceEventKind,
    patch: WorldlineTickPatchV1,
    expected_state_root: Hash,
    outputs: crate::worldline::OutputFrameSet,
    atom_writes: crate::worldline::AtomWriteSet,
    source_ref: Option<ProvenanceRef>,
}

impl SettlementService {
    /// Compares the strand suffix against its recorded base coordinate.
    pub fn compare(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        strand_id: StrandId,
    ) -> Result<SettlementDelta, SettlementError> {
        let strand = strand(runtime.strands(), strand_id)?;
        ensure_frontier_matches_provenance(
            runtime,
            provenance,
            strand.fork_basis_ref.source_lane_id,
        )?;
        let source_len =
            ensure_frontier_matches_provenance(runtime, provenance, strand.child_worldline_id)?;
        let source_suffix_start_tick = strand
            .fork_basis_ref
            .fork_tick
            .checked_increment()
            .ok_or(SettlementError::ForkTickOverflow(strand_id))?;
        let source_entries = (source_suffix_start_tick.as_u64()..source_len.as_u64())
            .map(WorldlineTick::from_raw)
            .map(|tick| {
                provenance
                    .entry(strand.child_worldline_id, tick)
                    .map(|entry| entry.as_ref())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let source_suffix_end_tick = source_entries
            .last()
            .map_or(strand.fork_basis_ref.fork_tick, |entry| {
                entry.worldline_tick
            });
        Ok(SettlementDelta {
            strand_id,
            fork_basis_ref: strand.fork_basis_ref,
            source_lane_id: strand.child_worldline_id,
            source_suffix_start_tick,
            source_suffix_end_tick,
            source_entries,
        })
    }

    /// Produces a deterministic import/conflict plan for the strand suffix.
    pub fn plan(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        strand_id: StrandId,
    ) -> Result<SettlementPlan, SettlementError> {
        let strand = strand(runtime.strands(), strand_id)?;
        let delta = Self::compare(runtime, provenance, strand_id)?;
        let target_worldline = strand.fork_basis_ref.source_lane_id;
        let target_frontier_tick =
            ensure_frontier_matches_provenance(runtime, provenance, target_worldline)?;
        let expected_target_tick = strand
            .fork_basis_ref
            .fork_tick
            .checked_increment()
            .ok_or(SettlementError::ForkTickOverflow(strand_id))?;
        let target_tip = provenance.tip_ref(target_worldline)?;

        let mut decisions = Vec::new();
        let mut simulated = runtime
            .worldlines()
            .get(&target_worldline)
            .ok_or(RuntimeError::UnknownWorldline(target_worldline))?
            .state()
            .clone();
        let mut blocked_reason = None;

        for source_ref in &delta.source_entries {
            let source_entry = provenance.entry(delta.source_lane_id, source_ref.worldline_tick)?;
            let reason = blocked_reason.or_else(|| {
                if target_frontier_tick != expected_target_tick
                    || target_tip != Some(strand.fork_basis_ref.provenance_ref)
                {
                    Some(ConflictReason::BaseDivergence)
                } else if !matches!(source_entry.event_kind, ProvenanceEventKind::LocalCommit) {
                    Some(ConflictReason::UnsupportedImport)
                } else {
                    None
                }
            });

            if let Some(reason) = reason {
                blocked_reason = Some(reason);
                decisions.push(SettlementDecision::ConflictArtifact(conflict_draft(
                    target_worldline,
                    &source_entry,
                    reason,
                )));
                continue;
            }

            let Some(patch) = source_entry.patch.as_ref() else {
                blocked_reason = Some(ConflictReason::UnsupportedImport);
                decisions.push(SettlementDecision::ConflictArtifact(conflict_draft(
                    target_worldline,
                    &source_entry,
                    ConflictReason::UnsupportedImport,
                )));
                continue;
            };

            if patch.apply_to_worldline_state(&mut simulated).is_err() {
                blocked_reason = Some(ConflictReason::UnsupportedImport);
                decisions.push(SettlementDecision::ConflictArtifact(conflict_draft(
                    target_worldline,
                    &source_entry,
                    ConflictReason::UnsupportedImport,
                )));
                continue;
            }

            let actual_state_root =
                compute_state_root_for_warp_state(simulated.warp_state(), simulated.root());
            if actual_state_root != source_entry.expected.state_root {
                blocked_reason = Some(ConflictReason::UnsupportedImport);
                decisions.push(SettlementDecision::ConflictArtifact(conflict_draft(
                    target_worldline,
                    &source_entry,
                    ConflictReason::UnsupportedImport,
                )));
                continue;
            }

            decisions.push(SettlementDecision::ImportCandidate(ImportCandidate {
                source_ref: source_entry.as_ref(),
                source_head_key: source_entry.head_key,
                imported_op_id: source_entry.expected.commit_hash,
            }));
        }

        Ok(SettlementPlan {
            strand_id,
            target_worldline,
            target_base_ref: strand.fork_basis_ref.provenance_ref,
            decisions,
        })
    }

    /// Executes the deterministic base-worldline settlement plan.
    pub fn settle(
        runtime: &mut WorldlineRuntime,
        provenance: &mut ProvenanceService,
        strand_id: StrandId,
    ) -> Result<SettlementResult, SettlementError> {
        let plan = Self::plan(runtime, provenance, strand_id)?;
        if plan.decisions.is_empty() {
            return Ok(SettlementResult {
                plan,
                appended_imports: Vec::new(),
                appended_conflicts: Vec::new(),
            });
        }

        let runtime_before = runtime.clone();
        let provenance_before = provenance.checkpoint_for([plan.target_worldline])?;
        let outcome = (|| {
            let mut appended_imports = Vec::new();
            let mut appended_conflicts = Vec::new();

            for decision in &plan.decisions {
                let commit_global_tick = runtime.advance_global_tick()?;
                let appended_ref = match decision {
                    SettlementDecision::ImportCandidate(candidate) => append_import_candidate(
                        runtime,
                        provenance,
                        plan.target_worldline,
                        candidate,
                        commit_global_tick,
                    )?,
                    SettlementDecision::ConflictArtifact(draft) => append_conflict_artifact(
                        runtime,
                        provenance,
                        plan.target_worldline,
                        draft,
                        commit_global_tick,
                    )?,
                };
                match decision {
                    SettlementDecision::ImportCandidate(_) => appended_imports.push(appended_ref),
                    SettlementDecision::ConflictArtifact(_) => {
                        appended_conflicts.push(appended_ref);
                    }
                }
            }

            Ok(SettlementResult {
                plan,
                appended_imports,
                appended_conflicts,
            })
        })();

        if outcome.is_err() {
            *runtime = runtime_before;
            provenance.restore(&provenance_before);
        }
        outcome
    }
}

fn strand(
    registry: &StrandRegistry,
    strand_id: StrandId,
) -> Result<&crate::strand::Strand, SettlementError> {
    registry
        .get(&strand_id)
        .ok_or(SettlementError::StrandNotFound(strand_id))
}

fn ensure_frontier_matches_provenance(
    runtime: &WorldlineRuntime,
    provenance: &ProvenanceService,
    worldline_id: WorldlineId,
) -> Result<WorldlineTick, SettlementError> {
    let frontier_tick = runtime
        .worldlines()
        .get(&worldline_id)
        .ok_or(RuntimeError::UnknownWorldline(worldline_id))?
        .frontier_tick();
    let provenance_len = WorldlineTick::from_raw(provenance.len(worldline_id)?);
    if frontier_tick != provenance_len {
        return Err(SettlementError::RuntimeProvenanceDrift {
            worldline_id,
            frontier_tick,
            provenance_len,
        });
    }
    Ok(frontier_tick)
}

fn append_import_candidate(
    runtime: &mut WorldlineRuntime,
    provenance: &mut ProvenanceService,
    target_worldline: WorldlineId,
    candidate: &ImportCandidate,
    commit_global_tick: GlobalTick,
) -> Result<ProvenanceRef, SettlementError> {
    let source_entry = provenance.entry(
        candidate.source_ref.worldline_id,
        candidate.source_ref.worldline_tick,
    )?;
    let source_patch =
        source_entry
            .patch
            .as_ref()
            .ok_or(SettlementError::SourceEntryMissingPatch {
                source_ref: candidate.source_ref,
            })?;
    let imported_patch = patch_with_commit_global_tick(source_patch, commit_global_tick);
    append_recorded_entry(
        runtime,
        provenance,
        target_worldline,
        RecordedEntryDraft {
            event_kind: ProvenanceEventKind::MergeImport {
                source_worldline: candidate.source_ref.worldline_id,
                source_worldline_tick: candidate.source_ref.worldline_tick,
                op_id: candidate.imported_op_id,
            },
            patch: imported_patch,
            expected_state_root: source_entry.expected.state_root,
            outputs: source_entry.outputs.clone(),
            atom_writes: source_entry.atom_writes.clone(),
            source_ref: Some(candidate.source_ref),
        },
    )
}

fn append_conflict_artifact(
    runtime: &mut WorldlineRuntime,
    provenance: &mut ProvenanceService,
    target_worldline: WorldlineId,
    draft: &ConflictArtifactDraft,
    commit_global_tick: GlobalTick,
) -> Result<ProvenanceRef, SettlementError> {
    let warp_id = runtime
        .worldlines()
        .get(&target_worldline)
        .ok_or(RuntimeError::UnknownWorldline(target_worldline))?
        .state()
        .root()
        .warp_id;
    let no_op_patch = empty_worldline_patch(warp_id, commit_global_tick);
    let expected_state_root = runtime
        .worldlines()
        .get(&target_worldline)
        .ok_or(RuntimeError::UnknownWorldline(target_worldline))?
        .state()
        .state_root();
    append_recorded_entry(
        runtime,
        provenance,
        target_worldline,
        RecordedEntryDraft {
            event_kind: ProvenanceEventKind::ConflictArtifact {
                artifact_id: draft.artifact_id,
            },
            patch: no_op_patch,
            expected_state_root,
            outputs: Vec::new(),
            atom_writes: Vec::new(),
            source_ref: None,
        },
    )
}

fn append_recorded_entry(
    runtime: &mut WorldlineRuntime,
    provenance: &mut ProvenanceService,
    target_worldline: WorldlineId,
    draft: RecordedEntryDraft,
) -> Result<ProvenanceRef, SettlementError> {
    let parents = provenance
        .tip_ref(target_worldline)?
        .into_iter()
        .collect::<Vec<_>>();
    let parent_hashes = parents
        .iter()
        .map(|parent| parent.commit_hash)
        .collect::<Vec<_>>();
    let frontier = runtime.frontier_mut(&target_worldline)?;
    let worldline_tick = frontier.frontier_tick();
    let root = *frontier.state().root();

    if let Err(source) = draft.patch.apply_to_worldline_state(frontier.state_mut()) {
        return Err(SettlementError::Apply {
            worldline_id: target_worldline,
            tick: worldline_tick,
            source,
        });
    }

    let actual_state_root = frontier.state().state_root();
    if actual_state_root != draft.expected_state_root {
        return Err(SettlementError::ImportedStateRootMismatch {
            source_ref: draft.source_ref.unwrap_or(ProvenanceRef {
                worldline_id: target_worldline,
                worldline_tick,
                commit_hash: [0; 32],
            }),
            expected: Box::new(draft.expected_state_root),
            actual: Box::new(actual_state_root),
        });
    }

    let patch_digest = draft.patch.patch_digest;
    let commit_hash = compute_commit_hash_v2(
        &actual_state_root,
        &parent_hashes,
        &patch_digest,
        draft.patch.policy_id(),
    );
    let entry = ProvenanceEntry::recorded_event(
        target_worldline,
        worldline_tick,
        draft.patch.commit_global_tick(),
        parents,
        draft.event_kind,
        HashTriplet {
            state_root: actual_state_root,
            patch_digest,
            commit_hash,
        },
        draft.patch,
        draft.outputs,
        draft.atom_writes,
    );
    provenance.append_recorded_event(entry.clone())?;
    let patch_ref =
        entry
            .patch
            .as_ref()
            .ok_or_else(|| SettlementError::SourceEntryMissingPatch {
                source_ref: draft.source_ref.unwrap_or(entry.as_ref()),
            })?;
    let (snapshot, receipt, replay_patch) = replay_artifacts_for_entry(root, &entry, patch_ref)?;
    frontier.state_mut().record_replayed_tick(
        snapshot,
        receipt,
        replay_patch,
        finalized_channels(&entry.outputs),
    );
    frontier
        .advance_tick()
        .ok_or(RuntimeError::FrontierTickOverflow(target_worldline))?;
    Ok(entry.as_ref())
}

fn conflict_draft(
    target_worldline: WorldlineId,
    source_entry: &ProvenanceEntry,
    reason: ConflictReason,
) -> ConflictArtifactDraft {
    ConflictArtifactDraft {
        artifact_id: compute_conflict_artifact_id(target_worldline, source_entry.as_ref(), reason),
        source_ref: source_entry.as_ref(),
        channel_ids: source_entry
            .outputs
            .iter()
            .map(|(channel, _)| *channel)
            .collect(),
        reason,
    }
}

fn compute_conflict_artifact_id(
    target_worldline: WorldlineId,
    source_ref: ProvenanceRef,
    reason: ConflictReason,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(CONFLICT_ARTIFACT_DOMAIN);
    hasher.update(target_worldline.as_bytes());
    hasher.update(source_ref.worldline_id.as_bytes());
    hasher.update(&source_ref.worldline_tick.as_u64().to_le_bytes());
    hasher.update(&source_ref.commit_hash);
    hasher.update(&[reason.code()]);
    hasher.finalize().into()
}

fn patch_with_commit_global_tick(
    source_patch: &WorldlineTickPatchV1,
    commit_global_tick: GlobalTick,
) -> WorldlineTickPatchV1 {
    WorldlineTickPatchV1 {
        header: WorldlineTickHeaderV1 {
            commit_global_tick,
            policy_id: source_patch.policy_id(),
            rule_pack_id: source_patch.rule_pack_id(),
            plan_digest: source_patch.header.plan_digest,
            decision_digest: source_patch.header.decision_digest,
            rewrites_digest: source_patch.header.rewrites_digest,
        },
        warp_id: source_patch.warp_id,
        ops: source_patch.ops.clone(),
        in_slots: source_patch.in_slots.clone(),
        out_slots: source_patch.out_slots.clone(),
        patch_digest: source_patch.patch_digest,
    }
}

fn empty_worldline_patch(
    warp_id: crate::ident::WarpId,
    commit_global_tick: GlobalTick,
) -> WorldlineTickPatchV1 {
    let no_op_patch = WarpTickPatchV1::new(
        crate::POLICY_ID_NO_POLICY_V0,
        crate::blake3_empty(),
        TickCommitStatus::Committed,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    WorldlineTickPatchV1 {
        header: WorldlineTickHeaderV1 {
            commit_global_tick,
            policy_id: no_op_patch.policy_id(),
            rule_pack_id: no_op_patch.rule_pack_id(),
            plan_digest: crate::blake3_empty(),
            decision_digest: crate::blake3_empty(),
            rewrites_digest: crate::blake3_empty(),
        },
        warp_id,
        ops: no_op_patch.ops().to_vec(),
        in_slots: no_op_patch.in_slots().to_vec(),
        out_slots: no_op_patch.out_slots().to_vec(),
        patch_digest: no_op_patch.digest(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use crate::head::{make_head_id, WriterHead, WriterHeadKey};
    use crate::head_inbox::InboxPolicy;
    use crate::ident::{make_node_id, make_type_id};
    use crate::playback::PlaybackMode;
    use crate::record::NodeRecord;
    use crate::strand::{ForkBasisRef, Strand};
    use crate::tick_patch::{SlotId, WarpOp};
    use crate::{GraphStore, WorldlineState};

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    fn wt(raw: u64) -> WorldlineTick {
        WorldlineTick::from_raw(raw)
    }

    fn gt(raw: u64) -> GlobalTick {
        GlobalTick::from_raw(raw)
    }

    fn register_head(
        runtime: &mut WorldlineRuntime,
        worldline_id: WorldlineId,
        label: &str,
    ) -> WriterHeadKey {
        let key = WriterHeadKey {
            worldline_id,
            head_id: make_head_id(label),
        };
        runtime
            .register_writer_head(WriterHead::with_routing(
                key,
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .unwrap();
        key
    }

    fn append_local_patch(
        state: &mut WorldlineState,
        provenance: &mut ProvenanceService,
        worldline_id: WorldlineId,
        head_key: WriterHeadKey,
        commit_global_tick: GlobalTick,
        patch: WorldlineTickPatchV1,
    ) -> ProvenanceEntry {
        let worldline_tick = state.current_tick();
        let parents = provenance
            .tip_ref(worldline_id)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        patch.apply_to_worldline_state(state).unwrap();
        let state_root = state.state_root();
        let parent_hashes = parents
            .iter()
            .map(|parent| parent.commit_hash)
            .collect::<Vec<_>>();
        let commit_hash = compute_commit_hash_v2(
            &state_root,
            &parent_hashes,
            &patch.patch_digest,
            patch.policy_id(),
        );
        let entry = ProvenanceEntry::local_commit(
            worldline_id,
            worldline_tick,
            commit_global_tick,
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
        );
        provenance.append_local_commit(entry.clone()).unwrap();
        let patch_ref = entry.patch.as_ref().unwrap();
        let (snapshot, receipt, replay_patch) =
            replay_artifacts_for_entry(*state.root(), &entry, patch_ref).unwrap();
        state.record_replayed_tick(snapshot, receipt, replay_patch, Vec::new());
        entry
    }

    fn node_upsert_patch(
        state: &WorldlineState,
        label: &str,
        commit_global_tick: GlobalTick,
    ) -> WorldlineTickPatchV1 {
        let root = *state.root();
        let node = crate::ident::NodeKey {
            warp_id: root.warp_id,
            local_id: make_node_id(label),
        };
        let replay_patch = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            crate::blake3_empty(),
            TickCommitStatus::Committed,
            Vec::new(),
            vec![SlotId::Node(node)],
            vec![WarpOp::UpsertNode {
                node,
                record: NodeRecord {
                    ty: make_type_id("settlement-node"),
                },
            }],
        );
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick,
                policy_id: replay_patch.policy_id(),
                rule_pack_id: replay_patch.rule_pack_id(),
                plan_digest: crate::blake3_empty(),
                decision_digest: crate::blake3_empty(),
                rewrites_digest: crate::blake3_empty(),
            },
            warp_id: root.warp_id,
            ops: replay_patch.ops().to_vec(),
            in_slots: replay_patch.in_slots().to_vec(),
            out_slots: replay_patch.out_slots().to_vec(),
            patch_digest: replay_patch.digest(),
        }
    }

    fn setup_runtime_with_strand(
        base_diverged: bool,
    ) -> (
        WorldlineRuntime,
        ProvenanceService,
        StrandId,
        WorldlineId,
        WorldlineId,
    ) {
        let base_worldline = wl(1);
        let child_worldline = wl(2);
        let mut base_store = GraphStore::new(crate::ident::make_warp_id("settlement-root"));
        let root_node = make_node_id("root");
        base_store.insert_node(
            root_node,
            NodeRecord {
                ty: make_type_id("root"),
            },
        );
        let mut base_state = WorldlineState::from_root_store(base_store, root_node).unwrap();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(base_worldline, &base_state)
            .unwrap();

        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state.clone())
            .unwrap();
        let base_head = register_head(&mut runtime, base_worldline, "base-head");
        let base_patch = node_upsert_patch(&base_state, "base-node", gt(1));
        let base_entry = append_local_patch(
            &mut base_state,
            &mut provenance,
            base_worldline,
            base_head,
            gt(1),
            base_patch,
        );
        runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state.clone())
            .unwrap();
        register_head(&mut runtime, base_worldline, "base-head");

        provenance
            .fork(base_worldline, wt(0), child_worldline)
            .unwrap();
        let mut child_state = provenance
            .replay_worldline_state(base_worldline, &base_state)
            .unwrap();
        runtime
            .register_worldline(child_worldline, child_state.clone())
            .unwrap();
        let child_head = register_head(&mut runtime, child_worldline, "child-head");
        let strand_id = crate::strand::make_strand_id("test-strand");
        let strand = Strand {
            strand_id,
            fork_basis_ref: ForkBasisRef {
                source_lane_id: base_worldline,
                fork_tick: wt(0),
                commit_hash: base_entry.expected.commit_hash,
                boundary_hash: base_entry.expected.state_root,
                provenance_ref: base_entry.as_ref(),
            },
            child_worldline_id: child_worldline,
            writer_heads: vec![child_head],
            support_pins: Vec::new(),
        };
        runtime.register_strand(strand).unwrap();

        if base_diverged {
            let diverged_head = WriterHeadKey {
                worldline_id: base_worldline,
                head_id: make_head_id("base-head"),
            };
            let diverged_patch = node_upsert_patch(&base_state, "base-diverged", gt(2));
            let entry = append_local_patch(
                &mut base_state,
                &mut provenance,
                base_worldline,
                diverged_head,
                gt(2),
                diverged_patch,
            );
            let _ = entry;
            runtime = WorldlineRuntime::new();
            runtime
                .register_worldline(base_worldline, base_state.clone())
                .unwrap();
            register_head(&mut runtime, base_worldline, "base-head");
            runtime
                .register_worldline(child_worldline, child_state.clone())
                .unwrap();
            register_head(&mut runtime, child_worldline, "child-head");
            runtime
                .register_strand(Strand {
                    strand_id,
                    fork_basis_ref: ForkBasisRef {
                        source_lane_id: base_worldline,
                        fork_tick: wt(0),
                        commit_hash: base_entry.expected.commit_hash,
                        boundary_hash: base_entry.expected.state_root,
                        provenance_ref: base_entry.as_ref(),
                    },
                    child_worldline_id: child_worldline,
                    writer_heads: vec![child_head],
                    support_pins: Vec::new(),
                })
                .unwrap();
        }

        let child_patch = node_upsert_patch(&child_state, "child-node", gt(3));
        append_local_patch(
            &mut child_state,
            &mut provenance,
            child_worldline,
            child_head,
            gt(3),
            child_patch,
        );
        runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state)
            .unwrap();
        register_head(&mut runtime, base_worldline, "base-head");
        runtime
            .register_worldline(child_worldline, child_state)
            .unwrap();
        register_head(&mut runtime, child_worldline, "child-head");
        runtime
            .register_strand(Strand {
                strand_id,
                fork_basis_ref: ForkBasisRef {
                    source_lane_id: base_worldline,
                    fork_tick: wt(0),
                    commit_hash: base_entry.expected.commit_hash,
                    boundary_hash: base_entry.expected.state_root,
                    provenance_ref: base_entry.as_ref(),
                },
                child_worldline_id: child_worldline,
                writer_heads: vec![child_head],
                support_pins: Vec::new(),
            })
            .unwrap();
        (
            runtime,
            provenance,
            strand_id,
            base_worldline,
            child_worldline,
        )
    }

    #[test]
    fn settlement_imports_child_suffix_into_base_worldline() {
        let (mut runtime, mut provenance, strand_id, base_worldline, _) =
            setup_runtime_with_strand(false);

        let plan = SettlementService::plan(&runtime, &provenance, strand_id).unwrap();
        assert_eq!(plan.decisions.len(), 1);
        assert!(matches!(
            plan.decisions[0],
            SettlementDecision::ImportCandidate(_)
        ));

        let result = SettlementService::settle(&mut runtime, &mut provenance, strand_id).unwrap();
        assert_eq!(result.appended_imports.len(), 1);
        assert!(result.appended_conflicts.is_empty());

        let imported = provenance.entry(base_worldline, wt(1)).unwrap();
        assert!(matches!(
            imported.event_kind,
            ProvenanceEventKind::MergeImport { .. }
        ));
        assert_eq!(
            runtime
                .worldlines()
                .get(&base_worldline)
                .unwrap()
                .frontier_tick(),
            wt(2)
        );
        let root_warp = runtime
            .worldlines()
            .get(&base_worldline)
            .unwrap()
            .state()
            .root()
            .warp_id;
        let node_id = make_node_id("child-node");
        assert!(runtime
            .worldlines()
            .get(&base_worldline)
            .unwrap()
            .state()
            .store(&root_warp)
            .unwrap()
            .node(&node_id)
            .is_some());
    }

    #[test]
    fn settlement_records_conflict_artifact_when_base_diverged() {
        let (mut runtime, mut provenance, strand_id, base_worldline, _) =
            setup_runtime_with_strand(true);
        let state_root_before = runtime
            .worldlines()
            .get(&base_worldline)
            .unwrap()
            .state()
            .state_root();

        let plan = SettlementService::plan(&runtime, &provenance, strand_id).unwrap();
        assert_eq!(plan.decisions.len(), 1);
        assert!(matches!(
            &plan.decisions[0],
            SettlementDecision::ConflictArtifact(ConflictArtifactDraft {
                reason: ConflictReason::BaseDivergence,
                ..
            })
        ));

        let result = SettlementService::settle(&mut runtime, &mut provenance, strand_id).unwrap();
        assert!(result.appended_imports.is_empty());
        assert_eq!(result.appended_conflicts.len(), 1);

        let conflict = provenance.entry(base_worldline, wt(2)).unwrap();
        assert!(matches!(
            conflict.event_kind,
            ProvenanceEventKind::ConflictArtifact { .. }
        ));
        assert_eq!(conflict.expected.state_root, state_root_before);
        assert_eq!(
            runtime
                .worldlines()
                .get(&base_worldline)
                .unwrap()
                .frontier_tick(),
            wt(3)
        );
    }
}
