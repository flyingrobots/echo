// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! One runtime-owned completed authored execution, without proposal or Action authority.
use super::{EffectMutation, EffectReadImageError, EffectReadSetImage};
use crate::{EchoOperationApplicationBasisV1, EchoOperationEvaluationBasisV1, Hash};
#[derive(Debug)]
pub(super) struct AuthoredExecution {
    pub(super) program: Hash,
    pub(super) input_basis: EchoOperationApplicationBasisV1,
    pub(super) output: Vec<u8>,
    pub(super) mutations: Vec<EffectMutation>,
}
/// Refusal while consuming a frame into completed authored evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectCompletionError {
    /// The frame has no successful whole authored operation.
    #[error("no completed authored operation")]
    NoCompletedOperation,
    /// The exact canonical input does not bind the runtime parent's application basis.
    #[error("authored input does not match the bound application basis")]
    InputBasisMismatch,
    /// Bounded read-image export failed; no completed evidence is delivered.
    #[error(transparent)]
    ReadImage(#[from] EffectReadImageError),
}
/// Owned evidence from one successful program on one runtime-selected parent.
///
/// There is no public constructor. This binds completed private evaluation,
/// not installed meaning, validated mutation preconditions, a patch or an Action.
/// Structural data getters grant no invocation or mutation authority.
#[derive(Debug)]
pub struct CompletedEffectEvaluation {
    execution: AuthoredExecution,
    reads: EffectReadSetImage,
}
impl CompletedEffectEvaluation {
    pub(super) fn new(execution: AuthoredExecution, reads: EffectReadSetImage) -> Self {
        Self { execution, reads }
    }
    /// Identity of the exact producing current operation program.
    #[must_use]
    pub const fn program_identity(&self) -> Hash {
        self.execution.program
    }
    /// Input binding using the same recipe as current invocation admission.
    #[must_use]
    pub const fn input_basis(&self) -> EchoOperationApplicationBasisV1 {
        self.execution.input_basis
    }
    /// Runtime-selected full parent retained by the recorded reads.
    #[must_use]
    pub const fn evaluation_basis(&self) -> EchoOperationEvaluationBasisV1 {
        self.reads.evaluation_basis()
    }
    /// Actual successful canonical program result, retained without a second copy.
    #[must_use]
    pub fn output(&self) -> &[u8] {
        &self.execution.output
    }
    /// Emitted intents in execution order; still unvalidated and not a patch.
    #[must_use]
    pub fn mutations(&self) -> &[EffectMutation] {
        &self.execution.mutations
    }
    /// Bounded canonical actual observations with the same parent and usage.
    #[must_use]
    pub const fn read_set_image(&self) -> &EffectReadSetImage {
        &self.reads
    }
}
