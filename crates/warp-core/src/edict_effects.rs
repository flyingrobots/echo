// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted-host recording reads for the authored-effect interpreter under construction.
//!
//! This is not an installed program profile or admission capability. The host
//! selects the immutable frontier and policy. No mutation or caller callback is
//! available. Preparing the basis and policy is outside the per-read budget.
//! The interpreter, complete footprint law, bounded arena and portal authority
//! validation still need to be connected before an authored-effect profile ships.
use std::collections::{BTreeMap, BTreeSet};

use crate::echo_operation::EchoOperationBudgetMeterV1;
use crate::edict_read::ReadBasis;
use crate::{
    EchoOperationBudgetV1, EdgeRecord, Footprint, OutgoingEdgeRange, SlotId, SlotReading, TypeId,
    WorldlineFrontier,
};

/// A read refusal, without an Action outcome or any mutation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectRangeReadError {
    /// The selected edge type is outside the host's range-read policy.
    #[error("range edge type is outside the footprint law")]
    FootprintLawViolation,
    /// The whole source bucket cannot be scanned within the remaining limits.
    #[error("range read exceeds its budget")]
    BudgetExceeded,
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
        Self {
            frontier,
            basis: ReadBasis::at(frontier),
            allowed_types: allowed_types.iter().copied().collect(),
            max_range_edges,
            meter: EchoOperationBudgetMeterV1::new(budget),
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
    pub fn read(
        &mut self,
        range: OutgoingEdgeRange,
    ) -> Result<Option<Vec<EdgeRecord>>, EffectRangeReadError> {
        if !self.allowed_types.contains(&range.edge_type) {
            return Err(EffectRangeReadError::FootprintLawViolation);
        }
        if !self.meter.charge(1, 64, 0) {
            return Err(EffectRangeReadError::BudgetExceeded);
        }
        let store = self.frontier.state().store(&range.source.warp_id);
        let node = store.and_then(|store| store.node(&range.source.local_id));
        self.readings
            .insert(SlotId::Node(range.source), SlotReading::Node(node.cloned()));
        self.footprint.n_read.insert(range.source);
        let value = if let (Some(store), Some(_)) = (store, node) {
            let bucket = store.edges_from(&range.source.local_id);
            let count =
                u64::try_from(bucket.len()).map_err(|_| EffectRangeReadError::BudgetExceeded)?;
            let bytes = count
                .checked_mul(64)
                .ok_or(EffectRangeReadError::BudgetExceeded)?;
            if count > self.max_range_edges || !self.meter.charge(count, bytes, 0) {
                return Err(EffectRangeReadError::BudgetExceeded);
            }
            let mut rows: Vec<_> = bucket
                .filter(|edge| edge.ty == range.edge_type)
                .cloned()
                .collect();
            rows.sort_by_key(|edge| edge.id);
            Some(rows)
        } else {
            None
        };
        self.readings.insert(
            SlotId::OutgoingEdges(range),
            SlotReading::OutgoingEdges(value.clone()),
        );
        self.footprint.r_read.insert(range);
        Ok(value)
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
