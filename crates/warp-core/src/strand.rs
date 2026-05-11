// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Strand contract for speculative execution lanes.
//!
//! A strand is a named, speculative execution lane derived from a source
//! worldline at a specific tick. It is a relation over a child worldline, not
//! a separate substrate.
//!
//! # Lifecycle
//!
//! A strand either exists in the `StrandRegistry` (live) or does not
//! (dropped). There is no tombstone state. Operational control comes from the
//! ordinary writer-head control plane; strands do not own a private tick path
//! or scheduler.
//!
//! # Invariants
//!
//! See `docs/invariants/STRAND-CONTRACT.md` for the full normative list.
//! Key invariants enforced by this module:
//!
//! - **INV-S1:** `fork_basis_ref` is immutable after creation.
//! - **INV-S2:** Writer heads are created fresh for the child worldline.
//! - **INV-S4:** Strands advance only through ordinary ingress + `super_tick()`.
//! - **INV-S5:** All `fork_basis_ref` fields are verified against provenance.
//! - **INV-S7:** `child_worldline_id != fork_basis_ref.source_lane_id`.
//! - **INV-S8:** Every writer head key belongs to `child_worldline_id`.
//! - **INV-S9:** support pins must be validated, live, and read-only.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::clock::WorldlineTick;
use crate::ident::Hash;
use crate::provenance_store::{ProvenanceRef, ProvenanceService, ProvenanceStore};
use crate::tick_patch::SlotId;
use crate::worldline::WorldlineId;

use crate::head::WriterHeadKey;

/// A 32-byte domain-separated strand identifier.
///
/// Derived from `BLAKE3("strand:" || label)` following the `HeadId`/`NodeId`
/// pattern.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct StrandId([u8; 32]);

impl StrandId {
    /// Construct a `StrandId` from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Produces a stable, domain-separated strand identifier using BLAKE3.
#[must_use]
pub fn make_strand_id(label: &str) -> StrandId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"strand:");
    hasher.update(label.as_bytes());
    StrandId(hasher.finalize().into())
}

/// The exact provenance coordinate the strand was forked from.
///
/// Immutable after creation (INV-S1).
///
/// # Coordinate semantics
///
/// - `fork_tick` is the **last included tick** in the copied prefix.
/// - `commit_hash` is the commit hash **at `fork_tick`**.
/// - `boundary_hash` is the **output boundary hash** at `fork_tick` — the
///   state root after applying the patch at `fork_tick`.
/// - `provenance_ref` carries the same coordinate as a [`ProvenanceRef`].
/// - All fields refer to the **same provenance coordinate**.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ForkBasisRef {
    /// Source lane this strand was forked from in v1.
    pub source_lane_id: WorldlineId,
    /// Last included tick in the copied prefix.
    pub fork_tick: WorldlineTick,
    /// Commit hash at `fork_tick`.
    pub commit_hash: Hash,
    /// Output boundary hash (state root) at `fork_tick`.
    pub boundary_hash: Hash,
    /// Substrate-native coordinate handle.
    pub provenance_ref: ProvenanceRef,
}

/// A read-only reference to another strand's materialized state (braid
/// geometry).
///
/// Support pins are created through validated registry/runtime APIs so the
/// stored `worldline_id`, `pinned_tick`, and `state_hash` remain replayable
/// kernel truth.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SupportPin {
    /// The strand being pinned.
    pub strand_id: StrandId,
    /// The pinned strand's child worldline.
    pub worldline_id: WorldlineId,
    /// Tick at which the support strand is pinned.
    pub pinned_tick: WorldlineTick,
    /// State hash at the pinned tick.
    pub state_hash: Hash,
}

/// Receipt returned when a strand is dropped.
///
/// This is the only record that the strand existed after hard-delete.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DropReceipt {
    /// The dropped strand's identity.
    pub strand_id: StrandId,
    /// The child worldline that was removed.
    pub child_worldline_id: WorldlineId,
    /// The tick the child worldline had reached at drop time.
    pub final_tick: WorldlineTick,
}

/// A strand: a named, ephemeral, speculative execution lane.
///
/// A strand either exists in the `StrandRegistry` (live) or does not
/// (dropped). There is no lifecycle field — operational state is derived
/// from the writer heads.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Strand {
    /// Unique strand identity.
    pub strand_id: StrandId,
    /// Immutable fork coordinate.
    pub fork_basis_ref: ForkBasisRef,
    /// Child worldline created by fork.
    pub child_worldline_id: WorldlineId,
    /// Writer heads for the child worldline (cardinality 1 in v1).
    pub writer_heads: Vec<WriterHeadKey>,
    /// Read-only support pins for braid geometry.
    pub support_pins: Vec<SupportPin>,
}

impl Strand {
    /// Builds a basis-relative report for this strand against current parent history.
    ///
    /// The report is the first live-strand seam: the strand still records an
    /// immutable fork anchor, but realization is evaluated against the current
    /// parent basis. Parent movement outside the strand-owned closed footprint
    /// can flow through; parent writes into that footprint require explicit
    /// revalidation before callers treat the realization as clean.
    ///
    /// # Errors
    ///
    /// Returns [`StrandError`] when provenance is unavailable or the fork tick
    /// cannot be advanced to the suffix start coordinate.
    pub fn live_basis_report<P: ProvenanceStore>(
        &self,
        provenance: &P,
    ) -> Result<StrandBasisReport, StrandError> {
        let suffix_start = self
            .fork_basis_ref
            .fork_tick
            .checked_increment()
            .ok_or(StrandError::ForkTickOverflow(self.strand_id))?;
        let child_len = history_len(provenance, self.child_worldline_id)?;
        let parent_len = history_len(provenance, self.fork_basis_ref.source_lane_id)?;

        if child_len < suffix_start {
            return Err(StrandError::Provenance(format!(
                "child worldline {:?} is shorter than strand suffix start {}",
                self.child_worldline_id, suffix_start
            )));
        }
        if parent_len < suffix_start {
            return Err(StrandError::Provenance(format!(
                "parent worldline {:?} is shorter than strand anchor successor {}",
                self.fork_basis_ref.source_lane_id, suffix_start
            )));
        }

        let owned_divergence = collect_divergence_footprint(
            provenance,
            self.child_worldline_id,
            suffix_start,
            child_len,
        )?;
        let parent_movement = collect_parent_movement(
            provenance,
            self.fork_basis_ref.source_lane_id,
            suffix_start,
            parent_len,
        )?;
        let realized_parent_ref =
            tip_ref_at_len(provenance, self.fork_basis_ref.source_lane_id, parent_len)?
                .unwrap_or(self.fork_basis_ref.provenance_ref);
        let source_suffix_end_tick = last_tick_before(child_len);
        let parent_revalidation = if parent_len == suffix_start {
            StrandRevalidationState::AtAnchor
        } else {
            let overlapping_slots = owned_divergence.overlapping_parent_writes(&parent_movement);
            if overlapping_slots.is_empty() {
                StrandRevalidationState::ParentAdvancedDisjoint {
                    parent_from: self.fork_basis_ref.provenance_ref,
                    parent_to: realized_parent_ref,
                }
            } else {
                StrandRevalidationState::RevalidationRequired {
                    parent_from: self.fork_basis_ref.provenance_ref,
                    parent_to: realized_parent_ref,
                    overlapping_slots,
                }
            }
        };

        Ok(StrandBasisReport {
            strand_id: self.strand_id,
            parent_anchor: self.fork_basis_ref,
            child_worldline_id: self.child_worldline_id,
            source_suffix_start_tick: suffix_start,
            source_suffix_end_tick,
            realized_parent_ref,
            owned_divergence,
            parent_movement,
            parent_revalidation,
        })
    }
}

/// Closed optic footprint owned by a strand's local divergence.
///
/// The write set records slots the child suffix produced. The read set records
/// slots the child suffix depended on. The closed footprint is the union of both:
/// parent writes into either side require revalidation because the speculative
/// realization may no longer be basis-clean.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StrandDivergenceFootprint {
    read_slots: BTreeSet<SlotId>,
    write_slots: BTreeSet<SlotId>,
}

impl StrandDivergenceFootprint {
    /// Records one replay patch as part of the strand-owned divergence.
    pub fn extend_patch(&mut self, patch: &crate::worldline::WorldlineTickPatchV1) {
        self.read_slots.extend(patch.in_slots.iter().copied());
        self.write_slots.extend(patch.out_slots.iter().copied());
    }

    /// Returns the number of unique slots in the closed footprint.
    #[must_use]
    pub fn closed_len(&self) -> usize {
        self.closed_slots().len()
    }

    /// Returns `true` when the closed footprint is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.read_slots.is_empty() && self.write_slots.is_empty()
    }

    /// Returns `true` when `slot` is in the closed read-or-write footprint.
    #[must_use]
    pub fn contains_closed(&self, slot: &SlotId) -> bool {
        self.read_slots.contains(slot) || self.write_slots.contains(slot)
    }

    /// Returns read slots in deterministic order.
    pub fn read_slots(&self) -> impl Iterator<Item = &SlotId> {
        self.read_slots.iter()
    }

    /// Returns write slots in deterministic order.
    pub fn write_slots(&self) -> impl Iterator<Item = &SlotId> {
        self.write_slots.iter()
    }

    fn closed_slots(&self) -> BTreeSet<SlotId> {
        self.read_slots.union(&self.write_slots).copied().collect()
    }

    fn overlapping_parent_writes(&self, parent_movement: &ParentMovementFootprint) -> Vec<SlotId> {
        parent_movement
            .write_slots
            .iter()
            .filter(|slot| self.contains_closed(slot))
            .copied()
            .collect()
    }
}

/// Parent movement after a strand's anchor coordinate.
///
/// v1 tracks parent writes because parent writes into the strand-owned
/// divergence footprint are the condition that forces revalidation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParentMovementFootprint {
    write_slots: BTreeSet<SlotId>,
}

impl ParentMovementFootprint {
    /// Records one parent replay patch as movement after the strand anchor.
    pub fn extend_patch(&mut self, patch: &crate::worldline::WorldlineTickPatchV1) {
        self.write_slots.extend(patch.out_slots.iter().copied());
    }

    /// Returns `true` when no parent writes were observed after the anchor.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.write_slots.is_empty()
    }

    /// Returns the number of unique parent-written slots after the anchor.
    #[must_use]
    pub fn write_len(&self) -> usize {
        self.write_slots.len()
    }

    /// Returns `true` when the parent wrote `slot` after the anchor.
    #[must_use]
    pub fn contains_write(&self, slot: &SlotId) -> bool {
        self.write_slots.contains(slot)
    }

    /// Returns parent-written slots in deterministic order.
    pub fn write_slots(&self) -> impl Iterator<Item = &SlotId> {
        self.write_slots.iter()
    }
}

/// Parent-basis posture for a live strand realization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrandRevalidationState {
    /// The parent has not advanced beyond the strand's anchor coordinate.
    AtAnchor,
    /// The parent advanced, but its writes did not touch the strand-owned footprint.
    ParentAdvancedDisjoint {
        /// Anchor coordinate from which the strand diverged.
        parent_from: ProvenanceRef,
        /// Current parent basis used for realization.
        parent_to: ProvenanceRef,
    },
    /// The parent advanced into the strand-owned footprint.
    RevalidationRequired {
        /// Anchor coordinate from which the strand diverged.
        parent_from: ProvenanceRef,
        /// Current parent basis that must be checked.
        parent_to: ProvenanceRef,
        /// Parent-written slots that overlap the strand-owned closed footprint.
        overlapping_slots: Vec<SlotId>,
    },
}

/// Result of explicitly revalidating parent movement inside a strand-owned footprint.
///
/// This is the inspectable seam between live-basis posture and downstream
/// settlement/reading decisions. The strand report identifies the overlap;
/// callers that can compare concrete target state classify the overlap as
/// clean, obstructed, or conflicting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrandOverlapRevalidation {
    /// The overlapped source patch is already satisfied on the current parent basis.
    Clean {
        /// Parent-written slots that overlapped this source patch.
        overlapping_slots: Vec<SlotId>,
    },
    /// The source patch could not be replayed on the current parent basis.
    Obstructed {
        /// Parent-written slots that overlapped this source patch.
        overlapping_slots: Vec<SlotId>,
    },
    /// Replaying the source patch would mutate overlapped parent state.
    Conflict {
        /// Parent-written slots that overlapped this source patch.
        overlapping_slots: Vec<SlotId>,
    },
}

impl StrandOverlapRevalidation {
    /// Returns the overlapping slots that drove this revalidation outcome.
    #[must_use]
    pub fn overlapping_slots(&self) -> &[SlotId] {
        match self {
            Self::Clean { overlapping_slots }
            | Self::Obstructed { overlapping_slots }
            | Self::Conflict { overlapping_slots } => overlapping_slots,
        }
    }
}

/// Basis-relative realization report for one live strand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrandBasisReport {
    /// Strand being reported.
    pub strand_id: StrandId,
    /// Immutable parent anchor recorded at strand creation.
    pub parent_anchor: ForkBasisRef,
    /// Child worldline currently carrying local divergence.
    pub child_worldline_id: WorldlineId,
    /// First child tick after the anchor.
    pub source_suffix_start_tick: WorldlineTick,
    /// Last child suffix tick, or `None` when the strand has no local suffix yet.
    pub source_suffix_end_tick: Option<WorldlineTick>,
    /// Current parent coordinate against which this strand can be realized.
    pub realized_parent_ref: ProvenanceRef,
    /// Closed footprint owned by the child suffix.
    pub owned_divergence: StrandDivergenceFootprint,
    /// Parent writes after the strand anchor.
    pub parent_movement: ParentMovementFootprint,
    /// Revalidation state induced by parent movement.
    pub parent_revalidation: StrandRevalidationState,
}

/// Errors that can occur during strand operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StrandError {
    /// The strand already exists in the registry.
    #[error("strand already exists: {0:?}")]
    AlreadyExists(StrandId),

    /// The strand was not found in the registry.
    #[error("strand not found: {0:?}")]
    NotFound(StrandId),

    /// The fork tick does not exist in the source worldline.
    #[error("fork tick {tick} not available in source worldline {worldline:?}")]
    ForkTickUnavailable {
        /// Source worldline.
        worldline: WorldlineId,
        /// Requested fork tick.
        tick: WorldlineTick,
    },

    /// The source worldline does not exist.
    #[error("source worldline not found: {0:?}")]
    SourceWorldlineNotFound(WorldlineId),

    /// A provenance operation failed during strand creation or drop.
    #[error("provenance error: {0}")]
    Provenance(String),

    /// The strand fork coordinate cannot advance to a suffix start tick.
    #[error("fork tick overflow for strand {0:?}")]
    ForkTickOverflow(StrandId),

    /// A support pin targeted a strand that is not live in the registry.
    #[error("support pin target strand not found: {0:?}")]
    MissingSupportTarget(StrandId),

    /// A support pin claimed the wrong child worldline for its target strand.
    #[error(
        "support pin worldline mismatch for target {target:?}: expected {expected:?}, got {got:?}"
    )]
    SupportWorldlineMismatch {
        /// The support target strand.
        target: StrandId,
        /// Expected child worldline id for that strand.
        expected: WorldlineId,
        /// Actual worldline id carried by the support pin.
        got: WorldlineId,
    },

    /// A support pin must not target the owning strand.
    #[error("strand must not support-pin itself: {0:?}")]
    SelfSupportPin(StrandId),

    /// A support pin duplicated an already pinned support target.
    #[error("duplicate support pin target: owner {owner:?}, target {target:?}")]
    DuplicateSupportTarget {
        /// Owning strand.
        owner: StrandId,
        /// Support strand that was duplicated.
        target: StrandId,
    },

    /// A pinned coordinate was not available in provenance.
    #[error("support pin tick {tick} unavailable for target strand {target:?}")]
    SupportPinUnavailable {
        /// Support target strand.
        target: StrandId,
        /// Requested pinned tick.
        tick: WorldlineTick,
    },

    /// A strand cannot be removed while another live strand still pins it.
    #[error("strand {strand:?} is pinned by live strand {pinned_by:?}")]
    PinnedByLiveStrand {
        /// Strand being removed.
        strand: StrandId,
        /// Live strand that still references it.
        pinned_by: StrandId,
    },

    /// A contract invariant was violated.
    #[error("invariant violation: {0}")]
    InvariantViolation(&'static str),
}

fn history_len<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
) -> Result<WorldlineTick, StrandError> {
    provenance
        .len(worldline_id)
        .map(WorldlineTick::from_raw)
        .map_err(|err| StrandError::Provenance(err.to_string()))
}

fn tip_ref_at_len<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    len: WorldlineTick,
) -> Result<Option<ProvenanceRef>, StrandError> {
    let Some(tip_tick) = last_tick_before(len) else {
        return Ok(None);
    };
    provenance
        .entry(worldline_id, tip_tick)
        .map(|entry| Some(entry.as_ref()))
        .map_err(|err| StrandError::Provenance(err.to_string()))
}

fn last_tick_before(len: WorldlineTick) -> Option<WorldlineTick> {
    len.checked_sub(1)
}

fn collect_divergence_footprint<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    start_tick: WorldlineTick,
    end_tick: WorldlineTick,
) -> Result<StrandDivergenceFootprint, StrandError> {
    let mut footprint = StrandDivergenceFootprint::default();
    for raw_tick in start_tick.as_u64()..end_tick.as_u64() {
        let entry = provenance
            .entry(worldline_id, WorldlineTick::from_raw(raw_tick))
            .map_err(|err| StrandError::Provenance(err.to_string()))?;
        if let Some(patch) = entry.patch.as_ref() {
            footprint.extend_patch(patch);
        }
    }
    Ok(footprint)
}

fn collect_parent_movement<P: ProvenanceStore>(
    provenance: &P,
    worldline_id: WorldlineId,
    start_tick: WorldlineTick,
    end_tick: WorldlineTick,
) -> Result<ParentMovementFootprint, StrandError> {
    let mut movement = ParentMovementFootprint::default();
    for raw_tick in start_tick.as_u64()..end_tick.as_u64() {
        let entry = provenance
            .entry(worldline_id, WorldlineTick::from_raw(raw_tick))
            .map_err(|err| StrandError::Provenance(err.to_string()))?;
        if let Some(patch) = entry.patch.as_ref() {
            movement.extend_patch(patch);
        }
    }
    Ok(movement)
}

/// Session-scoped registry of live strands.
///
/// Iteration order is by [`StrandId`] (lexicographic over hash bytes).
/// This is deterministic but not semantically meaningful.
#[derive(Clone, Debug, Default)]
pub struct StrandRegistry {
    strands: BTreeMap<StrandId, Strand>,
}

impl StrandRegistry {
    /// Creates an empty strand registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a fully constructed strand into the registry.
    ///
    /// Validates contract invariants before insertion:
    /// - INV-S7: `child_worldline_id != fork_basis_ref.source_lane_id`
    /// - INV-S8: every writer head belongs to `child_worldline_id`
    /// - support pins, when present, reference already-live strands and do not
    ///   duplicate or self-target
    ///
    /// # Errors
    ///
    /// Returns [`StrandError::AlreadyExists`] if a strand with the same ID
    /// is already registered, or [`StrandError::InvariantViolation`] if any
    /// contract invariant is violated.
    pub fn insert(&mut self, strand: Strand) -> Result<(), StrandError> {
        if self.strands.contains_key(&strand.strand_id) {
            return Err(StrandError::AlreadyExists(strand.strand_id));
        }
        // INV-S7: distinct worldlines.
        if strand.child_worldline_id == strand.fork_basis_ref.source_lane_id {
            return Err(StrandError::InvariantViolation(
                "INV-S7: child_worldline_id must differ from fork_basis_ref.source_lane_id",
            ));
        }
        // INV-S8: head ownership.
        for head_key in &strand.writer_heads {
            if head_key.worldline_id != strand.child_worldline_id {
                return Err(StrandError::InvariantViolation(
                    "INV-S8: every writer head must belong to child_worldline_id",
                ));
            }
        }
        self.validate_support_pins(&strand)?;
        self.strands.insert(strand.strand_id, strand);
        Ok(())
    }

    /// Removes a strand from the registry.
    ///
    /// # Errors
    ///
    /// Returns [`StrandError::NotFound`] if the strand is not registered.
    pub fn remove(&mut self, strand_id: &StrandId) -> Result<Strand, StrandError> {
        if let Some(pinned_by) = self.find_pinned_by(strand_id) {
            return Err(StrandError::PinnedByLiveStrand {
                strand: *strand_id,
                pinned_by,
            });
        }
        self.strands
            .remove(strand_id)
            .ok_or(StrandError::NotFound(*strand_id))
    }

    /// Returns a reference to a strand, if it exists.
    #[must_use]
    pub fn get(&self, strand_id: &StrandId) -> Option<&Strand> {
        self.strands.get(strand_id)
    }

    /// Returns a mutable reference to a strand, if it exists.
    #[cfg(test)]
    pub(crate) fn get_mut(&mut self, strand_id: &StrandId) -> Option<&mut Strand> {
        self.strands.get_mut(strand_id)
    }

    /// Returns `true` if the registry contains the given strand.
    #[must_use]
    pub fn contains(&self, strand_id: &StrandId) -> bool {
        self.strands.contains_key(strand_id)
    }

    /// Returns the strand whose child worldline matches `worldline_id`, if any.
    #[must_use]
    pub fn find_by_child_worldline(&self, worldline_id: &WorldlineId) -> Option<&Strand> {
        self.strands
            .values()
            .find(|strand| &strand.child_worldline_id == worldline_id)
    }

    /// Returns a zero-allocation iterator over live strands derived from the
    /// given source lane, ordered by [`StrandId`].
    pub fn iter_by_source_lane<'a>(
        &'a self,
        source_lane_id: &'a WorldlineId,
    ) -> impl Iterator<Item = &'a Strand> + 'a {
        self.strands
            .values()
            .filter(move |s| &s.fork_basis_ref.source_lane_id == source_lane_id)
    }

    /// Returns all live strands derived from the given source lane,
    /// ordered by [`StrandId`]. Allocates; prefer
    /// [`iter_by_source_lane`](Self::iter_by_source_lane) in hot paths.
    pub fn list_by_source_lane<'a>(&'a self, source_lane_id: &'a WorldlineId) -> Vec<&'a Strand> {
        self.iter_by_source_lane(source_lane_id).collect()
    }

    /// Returns the support pins for one strand.
    ///
    /// # Errors
    ///
    /// Returns [`StrandError::NotFound`] if the strand is not registered.
    pub fn list_support_pins(&self, strand_id: &StrandId) -> Result<&[SupportPin], StrandError> {
        Ok(&self
            .strands
            .get(strand_id)
            .ok_or(StrandError::NotFound(*strand_id))?
            .support_pins)
    }

    /// Validates and adds one read-only support pin to a live strand.
    ///
    /// # Errors
    ///
    /// Returns [`StrandError`] if either strand is missing, if the pin would be
    /// self-referential or duplicate, or if the pinned coordinate is not
    /// available in provenance.
    pub fn pin_support(
        &mut self,
        provenance: &ProvenanceService,
        strand_id: StrandId,
        support_strand_id: StrandId,
        pinned_tick: WorldlineTick,
    ) -> Result<SupportPin, StrandError> {
        let support_strand = self
            .strands
            .get(&support_strand_id)
            .ok_or(StrandError::MissingSupportTarget(support_strand_id))?
            .clone();
        let owner = self
            .strands
            .get(&strand_id)
            .ok_or(StrandError::NotFound(strand_id))?;
        if strand_id == support_strand_id {
            return Err(StrandError::SelfSupportPin(strand_id));
        }
        if owner
            .support_pins
            .iter()
            .any(|support_pin| support_pin.strand_id == support_strand_id)
        {
            return Err(StrandError::DuplicateSupportTarget {
                owner: strand_id,
                target: support_strand_id,
            });
        }

        let pinned_entry = provenance
            .entry(support_strand.child_worldline_id, pinned_tick)
            .map_err(|_| StrandError::SupportPinUnavailable {
                target: support_strand_id,
                tick: pinned_tick,
            })?;
        let support_pin = SupportPin {
            strand_id: support_strand_id,
            worldline_id: support_strand.child_worldline_id,
            pinned_tick,
            state_hash: pinned_entry.expected.state_root,
        };

        self.strands
            .get_mut(&strand_id)
            .ok_or(StrandError::NotFound(strand_id))?
            .support_pins
            .push(support_pin);
        Ok(support_pin)
    }

    /// Removes one support pin from a live strand.
    ///
    /// # Errors
    ///
    /// Returns [`StrandError::NotFound`] if the owner strand is missing or the
    /// named support target is not pinned.
    pub fn unpin_support(
        &mut self,
        strand_id: StrandId,
        support_strand_id: StrandId,
    ) -> Result<SupportPin, StrandError> {
        let owner = self
            .strands
            .get_mut(&strand_id)
            .ok_or(StrandError::NotFound(strand_id))?;
        let index = owner
            .support_pins
            .iter()
            .position(|support_pin| support_pin.strand_id == support_strand_id)
            .ok_or(StrandError::MissingSupportTarget(support_strand_id))?;
        Ok(owner.support_pins.remove(index))
    }

    /// Returns the number of live strands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strands.len()
    }

    /// Returns `true` if no strands are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strands.is_empty()
    }

    fn validate_support_pins(&self, strand: &Strand) -> Result<(), StrandError> {
        let mut seen = BTreeSet::new();
        for support_pin in &strand.support_pins {
            if support_pin.strand_id == strand.strand_id {
                return Err(StrandError::SelfSupportPin(strand.strand_id));
            }
            let target = self
                .strands
                .get(&support_pin.strand_id)
                .ok_or(StrandError::MissingSupportTarget(support_pin.strand_id))?;
            if target.child_worldline_id != support_pin.worldline_id {
                return Err(StrandError::SupportWorldlineMismatch {
                    target: support_pin.strand_id,
                    expected: target.child_worldline_id,
                    got: support_pin.worldline_id,
                });
            }
            if !seen.insert(support_pin.strand_id) {
                return Err(StrandError::DuplicateSupportTarget {
                    owner: strand.strand_id,
                    target: support_pin.strand_id,
                });
            }
        }
        Ok(())
    }

    fn find_pinned_by(&self, strand_id: &StrandId) -> Option<StrandId> {
        self.strands.values().find_map(|strand| {
            strand
                .support_pins
                .iter()
                .any(|support_pin| &support_pin.strand_id == strand_id)
                .then_some(strand.strand_id)
        })
    }
}
