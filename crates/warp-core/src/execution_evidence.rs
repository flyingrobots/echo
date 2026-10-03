// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Per-Action evidence retained from scheduler-owned execution.
//!
//! Worker completion order is not semantic: workers race to claim work units.
//! [`ExecutionEvidenceKey`] therefore binds each record to identity available
//! before execution and supplies the canonical order used when worker-local
//! records are joined.

use crate::actual_footprint::{ActualFootprint, ActualFootprintPosture};
use crate::ident::{NodeId, WarpId};
use crate::tick_delta::OpOrigin;

/// Stable pre-execution identity for one rule execution.
///
/// `OpOrigin` supplies intent, compact rule, match, and operation coordinates.
/// Warp and scope complete the identity without consulting worker assignment or
/// completion order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionEvidenceKey {
    sequence: u32,
    warp_id: WarpId,
    scope: NodeId,
    origin: OpOrigin,
}

impl ExecutionEvidenceKey {
    /// Binds one execution record to its warp, scope, and scheduler origin.
    #[must_use]
    pub const fn new(sequence: u32, warp_id: WarpId, scope: NodeId, origin: OpOrigin) -> Self {
        Self {
            sequence,
            warp_id,
            scope,
            origin,
        }
    }

    /// Returns the canonical position assigned before worker dispatch.
    #[must_use]
    pub const fn sequence(&self) -> u32 {
        self.sequence
    }

    /// Returns the warp whose graph state the executor observed.
    #[must_use]
    pub const fn warp_id(&self) -> WarpId {
        self.warp_id
    }

    /// Returns the scope node supplied to the executor.
    #[must_use]
    pub const fn scope(&self) -> NodeId {
        self.scope
    }

    /// Returns the scheduler-owned origin assigned before execution.
    #[must_use]
    pub const fn origin(&self) -> OpOrigin {
        self.origin
    }
}

/// Actual read/write footprint and evidence posture for one Action execution.
///
/// The record is useful only under the entitlement expressed by `posture`.
/// In particular, a legacy executor may contribute a known write axis while
/// its read axis remains unknown; callers must not reinterpret that partial
/// record as a complete empty read set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionFootprintEvidence {
    key: ExecutionEvidenceKey,
    actual: ActualFootprint,
    posture: ActualFootprintPosture,
}

impl ExecutionFootprintEvidence {
    /// Creates one per-Action footprint-evidence record.
    #[must_use]
    pub const fn new(
        key: ExecutionEvidenceKey,
        actual: ActualFootprint,
        posture: ActualFootprintPosture,
    ) -> Self {
        Self {
            key,
            actual,
            posture,
        }
    }

    /// Returns the stable pre-execution identity of this record.
    #[must_use]
    pub const fn key(&self) -> ExecutionEvidenceKey {
        self.key
    }

    /// Returns the canonical position assigned before worker dispatch.
    #[must_use]
    pub const fn sequence(&self) -> u32 {
        self.key.sequence()
    }

    /// Returns the warp whose graph state the executor observed.
    #[must_use]
    pub const fn warp_id(&self) -> WarpId {
        self.key.warp_id()
    }

    /// Returns the scope node supplied to the executor.
    #[must_use]
    pub const fn scope(&self) -> NodeId {
        self.key.scope()
    }

    /// Returns the scheduler-owned origin assigned before execution.
    #[must_use]
    pub const fn origin(&self) -> OpOrigin {
        self.key.origin()
    }

    /// Returns the actual resources recorded for this execution.
    #[must_use]
    pub const fn actual(&self) -> &ActualFootprint {
        &self.actual
    }

    /// Returns the entitlement carried by the actual-footprint record.
    #[must_use]
    pub const fn posture(&self) -> ActualFootprintPosture {
        self.posture
    }
}
