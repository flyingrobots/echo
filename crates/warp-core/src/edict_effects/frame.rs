// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! One logical work/storage envelope for computation and recorded reads.
use super::{EffectRangeReadError, EffectRangeReadSession, EFFECT_READ_STORAGE_CEILING_V1};
use crate::edict_pure::evaluate::Meter;
use crate::edict_pure::{self, EvaluationError, EvaluationLimits};
use crate::{
    EchoOperationBudgetV1, EchoOperationEvaluationBasisV1, EdgeRecord, Footprint,
    OutgoingEdgeRange, SlotId, SlotReading,
};
use std::collections::BTreeMap;

/// Refusal before constructing a shared evaluation frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectFrameSetupError {
    /// The requested full runtime parent differs from the current parent.
    #[error("effect frame parent basis does not match the runtime parent")]
    BasisMismatch,
    /// Already-observed resources cannot be imported without accounting for them.
    #[error("read session has already been used")]
    ReadSessionAlreadyUsed,
    /// Read storage or the operation budget exceeds the fixed effect ceilings.
    #[error("frame resource declaration exceeds the fixed ceiling")]
    ResourceCeilingExceeded,
}

/// Terminal frame failure, without any mutation or admission authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectFrameError {
    /// Failure from the unchanged compiler-package interpreter.
    Evaluation(EvaluationError),
    /// Read refusal or host fault; host allocation faults must not become Actions.
    Read(EffectRangeReadError),
    /// A prior failure ended this frame; no further work or reads are allowed.
    Terminated,
}

/// Trusted-host composition with one cumulative interpreter/read resource meter.
///
/// This reuses the real pure interpreter and independently pinned packages. It
/// is not a graph-effect program, installed profile, physical arena or admission
/// capability. The caller must select verified package pins independently.
#[derive(Debug)]
pub struct EffectEvaluationFrame<'a> {
    reads: EffectRangeReadSession<'a>,
    meter: Meter,
    limits: EvaluationLimits,
    terminated: bool,
    evaluation_basis: Option<EchoOperationEvaluationBasisV1>,
}

impl<'a> EffectEvaluationFrame<'a> {
    /// Starts an unused session under intersected computation/read ceilings.
    ///
    /// # Errors
    ///
    /// Refuses used sessions or declarations above the fixed effect ceilings.
    pub fn new(
        reads: EffectRangeReadSession<'a>,
        mut limits: EvaluationLimits,
    ) -> Result<Self, EffectFrameSetupError> {
        if reads.allocated_bytes != 0
            || !reads.readings.is_empty()
            || reads.consumed() != EchoOperationBudgetV1::new(0, 0, 0)
        {
            return Err(EffectFrameSetupError::ReadSessionAlreadyUsed);
        }
        let budget = reads.meter.delegated();
        if limits.max_allocated_bytes > EFFECT_READ_STORAGE_CEILING_V1
            || budget.steps() > 1_048_576
            || budget.read_bytes() > 67_108_864
            || budget.write_bytes() > 16_777_216
        {
            return Err(EffectFrameSetupError::ResourceCeilingExceeded);
        }
        limits.max_steps = limits.max_steps.min(budget.steps());
        limits.max_allocated_bytes = limits.max_allocated_bytes.min(reads.max_allocated_bytes);
        Ok(Self {
            reads,
            meter: Meter::new_atomic(limits),
            limits,
            terminated: false,
            evaluation_basis: None,
        })
    }

    // Only the runtime owner may pair a frame with its resolved parent.
    // A caller-provided frontier cannot acquire this coordinate through `new`.
    #[cfg(feature = "native_rule_bootstrap")]
    pub(crate) fn at_parent(
        reads: EffectRangeReadSession<'a>,
        limits: EvaluationLimits,
        basis: EchoOperationEvaluationBasisV1,
    ) -> Result<Self, EffectFrameSetupError> {
        let reading = reads.basis();
        if reading.worldline_id() != basis.writer_head().worldline_id
            || reading.tick() != basis.worldline_tick()
            || reading.state_root() != basis.state_root()
        {
            return Err(EffectFrameSetupError::BasisMismatch);
        }
        let mut frame = Self::new(reads, limits)?;
        frame.evaluation_basis = Some(basis);
        Ok(frame)
    }

    /// Runtime-selected parent, present only for a host-created bound frame.
    ///
    /// Low-level [`Self::new`] frames remain unbound. This coordinate does not
    /// authorize an invocation or establish an installed effect profile.
    #[must_use]
    pub const fn evaluation_basis(&self) -> Option<EchoOperationEvaluationBasisV1> {
        self.evaluation_basis
    }

    /// Reads a range through the recording view under the shared meter.
    ///
    /// # Errors
    ///
    /// A refusal or host fault ends the frame. Subsequent calls do no work.
    pub fn read_range(
        &mut self,
        range: OutgoingEdgeRange,
    ) -> Result<Option<Vec<EdgeRecord>>, EffectFrameError> {
        if self.terminated {
            return Err(EffectFrameError::Terminated);
        }
        self.terminated = true;
        let result = self
            .reads
            .read_with_meter(range, Some(&mut self.meter))
            .map_err(EffectFrameError::Read);
        self.terminated = result.is_err();
        result
    }

    /// Evaluates exact compiler-package bytes using the same cumulative meter.
    ///
    /// Package/input decoding is bounded host preparation. This is private
    /// computation; matching a caller-selected pin does not establish approval.
    ///
    /// # Errors
    ///
    /// Any decode, identity, execution or resource failure ends this frame.
    /// Host allocation failure must stop the host without deciding an Action.
    pub fn evaluate_pure(
        &mut self,
        package: &[u8],
        pin: [u8; 32],
        input: &[u8],
    ) -> Result<Vec<u8>, EffectFrameError> {
        if self.terminated {
            return Err(EffectFrameError::Terminated);
        }
        self.terminated = true;
        let result = edict_pure::prepare(package, pin, input, self.limits)
            .and_then(|(program, input)| {
                edict_pure::evaluate::run_with_meter(&program, input, &mut self.meter)
            })
            .map(|result| result.output)
            .map_err(EffectFrameError::Evaluation);
        self.terminated = result.is_err();
        result
    }

    /// Global computation plus read steps, and the view's I/O consumption.
    #[must_use]
    pub fn consumed(&self) -> EchoOperationBudgetV1 {
        EchoOperationBudgetV1::new(self.meter.usage().0, self.reads.consumed().read_bytes(), 0)
    }

    /// Global cumulative expression and read-value storage reservations.
    #[must_use]
    pub fn allocated_bytes(&self) -> u64 {
        self.meter.usage().1
    }

    /// Values observed at the immutable source frontier, including any prefix.
    #[must_use]
    pub const fn readings(&self) -> &BTreeMap<SlotId, SlotReading> {
        self.reads.readings()
    }

    /// Runtime-recorded graph dependencies; pure computation contributes none.
    #[must_use]
    pub const fn footprint(&self) -> &Footprint {
        self.reads.footprint()
    }
}
