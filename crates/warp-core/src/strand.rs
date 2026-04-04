// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Strand contract for speculative execution lanes.
//!
//! A strand is a named, ephemeral, speculative execution lane derived from a
//! base worldline at a specific tick. It is a relation over a child worldline,
//! not a separate substrate.
//!
//! # Lifecycle
//!
//! A strand either exists in the `StrandRegistry` (live) or does not
//! (dropped). There is no tombstone state. Operational control (paused,
//! admitted, ticking) is derived from the writer heads — the heads are the
//! single source of truth for control state.
//!
//! # Invariants
//!
//! See `docs/invariants/STRAND-CONTRACT.md` for the full normative list.
//! Key invariants enforced by this module:
//!
//! - **INV-S1:** `base_ref` is immutable after creation.
//! - **INV-S2:** Writer heads are created fresh for the child worldline.
//! - **INV-S4:** Writer heads are created Dormant (manual tick only).
//! - **INV-S5:** All `base_ref` fields are verified against provenance.
//! - **INV-S7:** `child_worldline_id != base_ref.source_worldline_id`.
//! - **INV-S8:** Every writer head key belongs to `child_worldline_id`.
//! - **INV-S9:** `support_pins` is empty in v1.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::clock::WorldlineTick;
use crate::ident::Hash;
use crate::provenance_store::ProvenanceRef;
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
pub struct BaseRef {
    /// Source worldline this strand was forked from.
    pub source_worldline_id: WorldlineId,
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
/// **v1: not implemented.** The `support_pins` field on [`Strand`] MUST be
/// empty (INV-S9). No mutation API exists.
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
    pub base_ref: BaseRef,
    /// Child worldline created by fork.
    pub child_worldline_id: WorldlineId,
    /// Writer heads for the child worldline (cardinality 1 in v1).
    pub writer_heads: Vec<WriterHeadKey>,
    /// Support pins for braid geometry (MUST be empty in v1).
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
    /// - INV-S7: `child_worldline_id != base_ref.source_worldline_id`
    /// - INV-S8: every writer head belongs to `child_worldline_id`
    /// - INV-S9: `support_pins` is empty in v1
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
        if strand.child_worldline_id == strand.base_ref.source_worldline_id {
            return Err(StrandError::InvariantViolation(
                "INV-S7: child_worldline_id must differ from base_ref.source_worldline_id",
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
        // INV-S9: no support pins in v1.
        if !strand.support_pins.is_empty() {
            return Err(StrandError::InvariantViolation(
                "INV-S9: support_pins must be empty in v1",
            ));
        }
        self.strands.insert(strand.strand_id, strand);
        Ok(())
    }

    /// Removes a strand from the registry, returning it if it existed.
    pub fn remove(&mut self, strand_id: &StrandId) -> Option<Strand> {
        self.strands.remove(strand_id)
    }

    /// Returns a reference to a strand, if it exists.
    #[must_use]
    pub fn get(&self, strand_id: &StrandId) -> Option<&Strand> {
        self.strands.get(strand_id)
    }

    /// Returns `true` if the registry contains the given strand.
    #[must_use]
    pub fn contains(&self, strand_id: &StrandId) -> bool {
        self.strands.contains_key(strand_id)
    }

    /// Returns all live strands derived from the given base worldline,
    /// ordered by [`StrandId`].
    pub fn list_by_base(&self, base_worldline_id: &WorldlineId) -> Vec<&Strand> {
        self.strands
            .values()
            .filter(|s| &s.base_ref.source_worldline_id == base_worldline_id)
            .collect()
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
}
