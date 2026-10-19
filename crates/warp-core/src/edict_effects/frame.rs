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
    /// A typed law requires an unused runtime-bound frame.
    #[error("typed law requires an unused runtime-bound frame")]
    InvalidLawBinding,
    /// Read storage or the operation budget exceeds the fixed effect ceilings.
    #[error("frame resource declaration exceeds the fixed ceiling")]
    ResourceCeilingExceeded,
}

/// Terminal frame failure, without any mutation or admission authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectFrameError {
    /// A caller-frontier frame cannot run recorded compiled reads as runtime-bound work.
    UnboundParent,
    /// Failure from the existing bounded-read interpreter or aperture validation.
    BoundedRead(crate::edict_read::ReadError),
    /// Failure from the unchanged compiler-package interpreter.
    Evaluation(EvaluationError),
    /// Read refusal or host fault; host allocation faults must not become Actions.
    Read(EffectRangeReadError),
    /// Failure or completed whole authored execution ended this frame.
    Terminated,
    /// Whole authored evaluation cannot adopt another private component's work.
    UsedFrame,
    /// The fixed effect-count ceiling or delegated emission budget is exhausted.
    BudgetExceeded,
    /// A resolved node/atom pair violates the exact prepared graph schema.
    GraphSchemaViolation,
    /// An authored terminal reason or mapped read failure, without an Action decision.
    AuthoredObstruction {
        /// Authored reason name in the bound source context.
        reason_kind: String,
        /// Canonical payload bytes.
        payload: Vec<u8>,
    },
}

impl EffectRangeReadSession<'_> {
    pub(super) fn charge_emission(
        &mut self,
        shared: &mut Meter,
        write_bytes: u64,
    ) -> Result<(), EffectFrameError> {
        let mut io = self.meter;
        let mut candidate = *shared;
        if !io.charge(1, 0, write_bytes) {
            return Err(EffectFrameError::BudgetExceeded);
        }
        candidate.charge_steps(1)?;
        candidate.allocate(280)?;
        self.meter = io;
        *shared = candidate;
        Ok(())
    }
}

impl From<EffectRangeReadError> for EffectFrameError {
    fn from(error: EffectRangeReadError) -> Self {
        Self::Read(error)
    }
}

impl From<crate::edict_read::ReadError> for EffectFrameError {
    fn from(error: crate::edict_read::ReadError) -> Self {
        Self::BoundedRead(error)
    }
}
impl From<EvaluationError> for EffectFrameError {
    fn from(error: EvaluationError) -> Self {
        Self::BoundedRead(error.into())
    }
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
    completed: Option<super::completion::AuthoredExecution>,
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
            completed: None,
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

    /// Binds host-selected typed read permissions before any evaluation work.
    ///
    /// This does not admit a package or bind the law into package identity.
    /// # Errors
    /// Refuses unbound, used, terminated or already law-bound frames.
    pub fn with_footprint_law(
        mut self,
        law: super::EffectFootprintLaw,
    ) -> Result<Self, EffectFrameSetupError> {
        if self.evaluation_basis.is_none()
            || self.terminated
            || self.reads.law.is_some()
            || self.consumed() != EchoOperationBudgetV1::new(0, 0, 0)
            || self.allocated_bytes() != 0
        {
            return Err(EffectFrameSetupError::InvalidLawBinding);
        }
        self.reads.law = Some(law);
        Ok(self)
    }

    /// Resolves a node using its actual type, or the named type for absence.
    /// # Errors
    /// Refuses a missing law, unreachable warp, denied type or exhausted limits.
    /// Any refusal terminates the frame without delivering a node value.
    pub fn read_typed_node(
        &mut self,
        node: crate::NodeKey,
        absent_type: crate::TypeId,
    ) -> Result<Option<crate::NodeRecord>, EffectFrameError> {
        if self.terminated {
            return Err(EffectFrameError::Terminated);
        }
        self.terminated = true;
        if self.reads.law.is_none() {
            return Err(EffectRangeReadError::FootprintLawViolation.into());
        }
        let result = self
            .reads
            .typed_node(node, absent_type, &mut self.meter)
            .map_err(EffectFrameError::Read);
        self.terminated = result.is_err();
        result
    }

    /// Distinct authorized slot/type pairs, including separately named absence.
    ///
    /// This is host-private authorization evidence, not the canonical persisted
    /// ReadRow image. Permission is rechecked even when a pair was seen before.
    #[must_use]
    pub const fn authorized_reads(&self) -> &std::collections::BTreeSet<(SlotId, crate::TypeId)> {
        &self.reads.authorized_reads
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
        if self.reads.law.is_some() {
            return Err(EffectRangeReadError::FootprintLawViolation.into());
        }
        let result = self
            .reads
            .read_with_meter(range, Some(&mut self.meter))
            .map_err(EffectFrameError::Read);
        self.terminated = result.is_err();
        result
    }

    /// Reads a range under both host and typed-law source/edge permissions.
    ///
    /// The law's bucket bound intersects the host limit. Matching and
    /// nonmatching edges consume the same precharged scan budget.
    /// # Errors
    /// Refuses missing law, unreachability, denied types or exhausted limits.
    /// Every refusal terminates the frame before delivering a range value.
    pub fn read_typed_range(
        &mut self,
        range: OutgoingEdgeRange,
        absent_source_type: crate::TypeId,
    ) -> Result<Option<Vec<EdgeRecord>>, EffectFrameError> {
        if self.terminated {
            return Err(EffectFrameError::Terminated);
        }
        self.terminated = true;
        if self.reads.law.is_none() {
            return Err(EffectRangeReadError::FootprintLawViolation.into());
        }
        let result = self
            .reads
            .read_range_inner(range, Some(&mut self.meter), Some(absent_source_type))
            .map_err(EffectFrameError::Read);
        self.terminated = result.is_err();
        result
    }

    /// Privately evaluates the prepared current authored program against its bound parent.
    /// Reads are recorded and mutations are buffered as unvalidated intents.
    /// This grants no invocation or mutation authority.
    /// # Errors
    /// Requires an unused frame. Failure or successful whole authored execution
    /// ends evaluation; only retention/inspection remains afterward.
    pub fn evaluate_effect(
        &mut self,
        program: &super::EffectProgram,
        input: &[u8],
        aperture: &[super::TypedNodeRead],
    ) -> Result<&[u8], EffectFrameError> {
        if self.terminated {
            return Err(EffectFrameError::Terminated);
        }
        self.terminated = true;
        if self.evaluation_basis.is_none() {
            return Err(EffectFrameError::UnboundParent);
        }
        if self.consumed() != EchoOperationBudgetV1::new(0, 0, 0) || self.allocated_bytes() != 0 {
            return Err(EffectFrameError::UsedFrame);
        }
        if self.reads.law.as_ref() != Some(program.footprint_law()) {
            return Err(EffectRangeReadError::FootprintLawViolation.into());
        }
        let aperture = super::NodeAperture::Typed(aperture);
        aperture.validate()?;
        if program.compiled_program().len() > self.limits.max_package_bytes {
            return Err(EvaluationError::PackageTooLarge.into());
        }
        if input.len() as u64 > (self.limits.max_input_bytes as u64).min(program.max_input_bytes())
        {
            return Err(EvaluationError::InputTooLarge.into());
        }
        let executable = program.execution(self.limits)?;
        let input_basis = crate::echo_operation::effect_input_basis(input);
        let input = echo_edict_canonical::decode_canonical_cbor_v1(input)
            .map_err(|_| EffectFrameError::Evaluation(EvaluationError::InvalidInput))?;
        let mut access = super::RecordedNodeAccess {
            reads: &mut self.reads,
            aperture,
        };
        let result = executable.run(input, &mut access, program.graph_schema(), &mut self.meter)?;
        let completed = self.completed.insert(super::completion::AuthoredExecution {
            program: program.identity(),
            input_basis,
            output: result.output,
            mutations: result.mutations,
        });
        Ok(&completed.output)
    }

    /// Intents from the completed whole authored operation, in execution order.
    /// These have not passed proposal validation and are not writes or a patch.
    /// Failure never publishes a partial intent buffer.
    #[must_use]
    pub fn buffered_mutations(&self) -> &[super::EffectMutation] {
        self.completed
            .as_ref()
            .map_or(&[], |value| value.mutations.as_slice())
    }

    /// Consumes one completed authored operation into its exact origin evidence.
    /// Failed, unused and mixed-component frames cannot produce this value.
    /// This is bounded host retention, not proposal validation or Action admission.
    /// # Errors
    /// Refuses missing completion, input-basis mismatch or read-image export failure.
    pub fn into_effect_evaluation(
        mut self,
        max_read_bytes: usize,
    ) -> Result<super::CompletedEffectEvaluation, super::EffectCompletionError> {
        let execution = self
            .completed
            .take()
            .ok_or(super::EffectCompletionError::NoCompletedOperation)?;
        let parent = self
            .evaluation_basis
            .ok_or(super::EffectCompletionError::NoCompletedOperation)?;
        if parent.application_basis() != execution.input_basis {
            return Err(super::EffectCompletionError::InputBasisMismatch);
        }
        let reads = self.into_read_set_image(max_read_bytes)?;
        Ok(super::CompletedEffectEvaluation::new(execution, reads))
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

    /// Interprets a pinned bounded-read package over the bound runtime frontier.
    ///
    /// The trusted host selects a sorted node aperture. Actual node and alpha
    /// attachment resolutions are charged and retained, including absence and
    /// semantic failures. Expression work, value storage and previous components
    /// remain under the shared frame meter. This does not implement the complete
    /// node-type/portal footprint law, installation or admission of effect programs.
    ///
    /// # Errors
    ///
    /// Refuses an unbound frame, malformed aperture/package/input, authored
    /// obstruction or exhausted limits. Every failure terminates the frame.
    /// Host allocation faults must stop the host rather than decide an Action.
    pub fn evaluate_read(
        &mut self,
        package: &[u8],
        pin: [u8; 32],
        input: &[u8],
        aperture: &[crate::NodeKey],
        limits: crate::edict_read::ReadLimits,
    ) -> Result<Vec<u8>, EffectFrameError> {
        if self.reads.law.is_some() {
            if self.terminated {
                return Err(EffectFrameError::Terminated);
            }
            self.terminated = true;
            return Err(EffectRangeReadError::FootprintLawViolation.into());
        }
        self.evaluate_read_aperture(
            package,
            pin,
            input,
            super::NodeAperture::Untyped(aperture),
            limits,
        )
    }

    /// Evaluates a pinned bounded-read package under typed host permissions.
    ///
    /// Present records authorize their actual types; named node types authorize
    /// absence only. Portal chains are authorized before target resolution.
    /// # Errors
    /// Refuses missing law, invalid aperture, denied types/reachability, authored
    /// obstructions and resource failures. Every refusal terminates the frame.
    pub fn evaluate_typed_read(
        &mut self,
        package: &[u8],
        pin: [u8; 32],
        input: &[u8],
        aperture: &[super::TypedNodeRead],
        limits: crate::edict_read::ReadLimits,
    ) -> Result<Vec<u8>, EffectFrameError> {
        if self.reads.law.is_none() {
            if self.terminated {
                return Err(EffectFrameError::Terminated);
            }
            self.terminated = true;
            return Err(EffectRangeReadError::FootprintLawViolation.into());
        }
        self.evaluate_read_aperture(
            package,
            pin,
            input,
            super::NodeAperture::Typed(aperture),
            limits,
        )
    }

    fn evaluate_read_aperture(
        &mut self,
        package: &[u8],
        pin: [u8; 32],
        input: &[u8],
        aperture: super::NodeAperture<'_>,
        mut limits: crate::edict_read::ReadLimits,
    ) -> Result<Vec<u8>, EffectFrameError> {
        if self.terminated {
            return Err(EffectFrameError::Terminated);
        }
        self.terminated = true;
        if self.evaluation_basis.is_none() {
            return Err(EffectFrameError::UnboundParent);
        }
        limits.evaluation.max_package_bytes = limits
            .evaluation
            .max_package_bytes
            .min(self.limits.max_package_bytes);
        limits.evaluation.max_input_bytes = limits
            .evaluation
            .max_input_bytes
            .min(self.limits.max_input_bytes);
        limits.evaluation.max_output_bytes = limits
            .evaluation
            .max_output_bytes
            .min(self.limits.max_output_bytes);
        limits.evaluation.max_steps = limits.evaluation.max_steps.min(self.limits.max_steps);
        limits.evaluation.max_allocated_bytes = limits
            .evaluation
            .max_allocated_bytes
            .min(self.limits.max_allocated_bytes);
        let result = crate::edict_read::evaluate_recorded(
            package,
            pin,
            input,
            &mut self.reads,
            aperture,
            &mut self.meter,
            limits,
        )
        .map(|result| result.output);
        self.terminated = result.is_err();
        result
    }

    /// Consumes this frame into bounded canonical completed-read evidence.
    ///
    /// Failed typed frames may export their completed observation prefix. A
    /// permission reservation without a completed value resolution is omitted.
    /// This is bounded host retention work outside the evaluation meter, not a
    /// physical interpreter arena, installed capability or historical decision.
    /// The output limit intersects the fixed 16 MiB read-storage ceiling.
    /// # Errors
    /// Refuses unbound/untyped frames, inconsistent observations, an output
    /// exceeding the host limit, or host allocation failure. No image is
    /// published on failure; a retention host must not turn it into an Action.
    pub fn into_read_set_image(
        self,
        max_bytes: usize,
    ) -> Result<super::EffectReadSetImage, super::EffectReadImageError> {
        let basis = self
            .evaluation_basis
            .ok_or(super::EffectReadImageError::MissingTypedParent)?;
        if self.reads.law.is_none() {
            return Err(super::EffectReadImageError::MissingTypedParent);
        }
        super::read_image::export(
            &self.reads,
            basis,
            self.consumed(),
            self.allocated_bytes(),
            max_bytes,
        )
    }

    /// Global computation/read/emission steps and all admitted I/O charges.
    #[must_use]
    pub fn consumed(&self) -> EchoOperationBudgetV1 {
        EchoOperationBudgetV1::new(
            self.meter.usage().0,
            self.reads.consumed().read_bytes(),
            self.reads.consumed().write_bytes(),
        )
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
