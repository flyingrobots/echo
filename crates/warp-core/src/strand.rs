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
