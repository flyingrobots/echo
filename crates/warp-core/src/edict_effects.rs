// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted-host recording reads for the authored-effect interpreter under construction.
//!
//! This is not an installed program profile or admission capability. The host
//! selects the immutable frontier and policy. No mutation or caller callback is
//! available. Preparing the basis and policy is outside the per-read budget.
//! The interpreter, complete footprint law, bounded arena and portal authority
//! validation still need to be connected before an authored-effect profile ships.
mod frame;
pub use frame::{EffectEvaluationFrame, EffectFrameError, EffectFrameSetupError};

use crate::edict_pure::evaluate::Meter;
use std::collections::{BTreeMap, BTreeSet};

use crate::echo_operation::EchoOperationBudgetMeterV1;
use crate::edict_read::ReadBasis;
use crate::{
    EchoOperationBudgetV1, EdgeRecord, Footprint, OutgoingEdgeRange, SlotId, SlotReading, TypeId,
    WorldlineFrontier,
};

/// Maximum declared value-storage reservation for this read primitive.
///
/// This is a deterministic logical storage ceiling, not an OS memory limit.
pub const EFFECT_READ_STORAGE_CEILING_V1: u64 = 16 * 1024 * 1024;
const OBSERVATION_STORAGE_BYTES: u64 = 128;
const EDGE_RECORD_STORAGE_BYTES: u64 = 128;

/// Invalid trusted-host preparation, before any read is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectReadSetupError {
    /// The declared reservation exceeds the fixed read-storage ceiling.
    #[error("declared read storage exceeds the fixed ceiling")]
    AllocationLimitTooLarge,
}

/// A read failure, without an Action outcome or any mutation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectRangeReadError {
    /// The selected edge type is outside the host's range-read policy.
    #[error("range edge type is outside the footprint law")]
    FootprintLawViolation,
    /// The whole source bucket cannot be scanned within the remaining limits.
    #[error("range read exceeds its budget")]
    BudgetExceeded,
    /// Host storage could not be provisioned despite a successful reservation.
    ///
    /// This is a host fault. It must never become an Action obstruction or a
    /// retained decision; an integrating host must stop the evaluation.
    #[error("host could not provision reserved range storage")]
    HostAllocationFailed,
}

/// Immutable frontier plus an owned, runtime-recorded dependency transcript.
///
/// Reads resolve by full WARP/source/type keys, including unreachable records.
/// The caller cannot supply or alter the transcript. Values are host-owned
/// materializations; this API does not claim a bounded interpreter arena.
#[derive(Debug)]
pub struct EffectRangeReadSession<'a> {
    frontier: &'a WorldlineFrontier,
    basis: ReadBasis,
    allowed_types: BTreeSet<TypeId>,
    max_range_edges: u64,
    meter: EchoOperationBudgetMeterV1,
    max_allocated_bytes: u64,
    allocated_bytes: u64,
    readings: BTreeMap<SlotId, SlotReading>,
    footprint: Footprint,
}

impl<'a> EffectRangeReadSession<'a> {
    /// Prepares a trusted-host view, hashing the real frontier and indexing policy.
    ///
    /// This setup cost precedes the read budget; the immutable borrow binds all
    /// subsequent observations to the same state. This is not package admission.
    #[must_use]
    pub fn new(
        frontier: &'a WorldlineFrontier,
        allowed_types: &[TypeId],
        max_range_edges: u64,
        budget: EchoOperationBudgetV1,
    ) -> Self {
        Self::prepare(
            frontier,
            allowed_types,
            max_range_edges,
            budget,
            EFFECT_READ_STORAGE_CEILING_V1,
        )
    }

    /// Prepares a view with an explicit value-storage reservation.
    ///
    /// # Errors
    ///
    /// Refuses a declaration above the fixed ceiling before preparing the basis.
    pub fn with_allocation_limit(
        frontier: &'a WorldlineFrontier,
        allowed_types: &[TypeId],
        max_range_edges: u64,
        budget: EchoOperationBudgetV1,
        max_allocated_bytes: u64,
    ) -> Result<Self, EffectReadSetupError> {
        if max_allocated_bytes > EFFECT_READ_STORAGE_CEILING_V1 {
            return Err(EffectReadSetupError::AllocationLimitTooLarge);
        }
        Ok(Self::prepare(
            frontier,
            allowed_types,
            max_range_edges,
            budget,
            max_allocated_bytes,
        ))
    }

    fn prepare(
        frontier: &'a WorldlineFrontier,
        allowed_types: &[TypeId],
        max_range_edges: u64,
        budget: EchoOperationBudgetV1,
        max_allocated_bytes: u64,
    ) -> Self {
        Self {
            frontier,
            basis: ReadBasis::at(frontier),
            allowed_types: allowed_types.iter().copied().collect(),
            max_range_edges,
            meter: EchoOperationBudgetMeterV1::new(budget),
            max_allocated_bytes,
            allocated_bytes: 0,
            readings: BTreeMap::new(),
            footprint: Footprint::default(),
        }
    }

    /// Resolves a typed outgoing range after authorizing and precharging its scan.
    ///
    /// Source presence costs one step and 64 read bytes, present or absent.
    /// Every edge in the entire source bucket then costs one step and 64 read
    /// bytes, including nonmatching types. A failed charge is never added.
    /// Source absence is distinct from a present source with no matching edges.
    ///
    /// # Errors
    ///
    /// A forbidden type is refused before state lookup or charging. A scan that
    /// exceeds the whole-bucket limit or remaining budget is refused before any
    /// edge is inspected; its already-resolved node dependency remains visible.
    /// Storage and I/O reservations are atomic. Host allocation failure is
    /// separately reported as a host fault, never as budget exhaustion.
    pub fn read(
        &mut self,
        range: OutgoingEdgeRange,
    ) -> Result<Option<Vec<EdgeRecord>>, EffectRangeReadError> {
        self.read_with_meter(range, None)
    }

    fn read_with_meter(
        &mut self,
        range: OutgoingEdgeRange,
        mut shared: Option<&mut Meter>,
    ) -> Result<Option<Vec<EdgeRecord>>, EffectRangeReadError> {
        if !self.allowed_types.contains(&range.edge_type) {
            return Err(EffectRangeReadError::FootprintLawViolation);
        }
        if !self.charge(1, 64, OBSERVATION_STORAGE_BYTES, shared.as_deref_mut()) {
            return Err(EffectRangeReadError::BudgetExceeded);
        }
        let store = self.frontier.state().store(&range.source.warp_id);
        let node = store.and_then(|store| store.node(&range.source.local_id));
        self.readings
            .insert(SlotId::Node(range.source), SlotReading::Node(node.cloned()));
        self.footprint.n_read.insert(range.source);
        let bucket = store
            .filter(|_| node.is_some())
            .map(|store| store.edges_from(&range.source.local_id));
        let capacity = bucket.as_ref().map_or(0, ExactSizeIterator::len);
        let count = u64::try_from(capacity).map_err(|_| EffectRangeReadError::BudgetExceeded)?;
        let bytes = count
            .checked_mul(64)
            .ok_or(EffectRangeReadError::BudgetExceeded)?;
        let storage = count
            .checked_mul(2 * EDGE_RECORD_STORAGE_BYTES)
            .and_then(|rows| rows.checked_add(OBSERVATION_STORAGE_BYTES))
            .ok_or(EffectRangeReadError::BudgetExceeded)?;
        if count > self.max_range_edges || !self.charge(count, bytes, storage, shared) {
            return Err(EffectRangeReadError::BudgetExceeded);
        }
        let (value, observed) = if let Some(bucket) = bucket {
            // Provision both worst-case row buffers before inspecting any edge.
            // Host inability to provide these bytes is not a semantic refusal.
            let mut rows = Vec::new();
            rows.try_reserve_exact(capacity)
                .map_err(|_| EffectRangeReadError::HostAllocationFailed)?;
            let mut observed = Vec::new();
            observed
                .try_reserve_exact(capacity)
                .map_err(|_| EffectRangeReadError::HostAllocationFailed)?;
            rows.extend(bucket.filter(|edge| edge.ty == range.edge_type).cloned());
            // Edge IDs are unique; unstable sorting is canonical and allocates
            // no additional temporary buffer.
            rows.sort_unstable_by_key(|edge| edge.id);
            observed.extend_from_slice(&rows);
            (Some(rows), Some(observed))
        } else {
            (None, None)
        };
        self.readings.insert(
            SlotId::OutgoingEdges(range),
            SlotReading::OutgoingEdges(observed),
        );
        self.footprint.r_read.insert(range);
        Ok(value)
    }

    fn charge(
        &mut self,
        steps: u64,
        read_bytes: u64,
        storage: u64,
        shared: Option<&mut Meter>,
    ) -> bool {
        let Some(allocated) = self.allocated_bytes.checked_add(storage) else {
            return false;
        };
        let mut meter = self.meter;
        if allocated > self.max_allocated_bytes || !meter.charge(steps, read_bytes, 0) {
            return false;
        }
        if let Some(shared) = shared {
            let mut candidate = *shared;
            if candidate.charge_steps(steps).is_err() || candidate.allocate(storage).is_err() {
                return false;
            }
            *shared = candidate;
        }
        self.meter = meter;
        self.allocated_bytes = allocated;
        true
    }

    /// Cumulative reserved value storage, including both worst-case row buffers.
    ///
    /// Each node/range observation reserves 128 logical bytes. Each bucket edge
    /// reserves 128 bytes in each of the returned and recorded row buffers,
    /// including nonmatching edges. Index/allocator overhead and trusted-host
    /// preparation are separate; this is not a complete interpreter arena.
    #[must_use]
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Reading coordinate computed from the borrowed frontier during preparation.
    ///
    /// This is not the complete commit-parent basis: its state root covers the
    /// reachable graph, while by-key reads may observe unreachable records.
    /// Admission must separately bind the writer head and commit identity.
    #[must_use]
    pub const fn basis(&self) -> ReadBasis {
        self.basis
    }

    /// Successful charges, including reads resolved before a later refusal.
    #[must_use]
    pub const fn consumed(&self) -> EchoOperationBudgetV1 {
        self.meter.consumed()
    }

    /// Real values observed at this basis, including absence and empty ranges.
    #[must_use]
    pub const fn readings(&self) -> &BTreeMap<SlotId, SlotReading> {
        &self.readings
    }

    /// Runtime-derived read claims. No writes or application claims are inserted.
    #[must_use]
    pub const fn footprint(&self) -> &Footprint {
        &self.footprint
    }
}
