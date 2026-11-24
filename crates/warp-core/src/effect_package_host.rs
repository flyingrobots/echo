// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted host route for authored effect packages.
//!
//! This module owns the runtime-owner route that runs an authored Edict
//! effect package on generic Echo: admit exact package bytes under an exact
//! policy, install the admitted meaning, resolve the exact parent basis, admit
//! one invocation, evaluate it in a recording frame and derive the proposal
//! without applying it (echo#1017).
//!
//! [`EffectPackageHost`] exposes that route under `trusted_runtime` alone. It
//! has no native rule authoring, no scheduler, no WAL and no commit path, so
//! it grants no authority beyond the route itself. [`crate::TrustedRuntimeHost`]
//! delegates the same steps to [`EffectPackageRoute`], so both hosts enforce
//! one admission, basis, frame and derivation law.

use thiserror::Error;

use crate::coordinator::resolve_echo_operation_evaluation_basis_v1;
use crate::echo_operation::{
    admit_invocation_v1, admit_operation_package, decode_invocation_route_v1,
    installed_from_admitted, invocation_runtime_error, EchoOperationEvaluationAuthorityV1,
};
use crate::edict_effects::{
    derive::derive, CompletedEffectEvaluation, DerivedEffectPatch, EffectDerivationError,
    EffectDerivationKind, EffectEvaluationFrame, EffectFrameSetupError, EffectRangeReadSession,
};
use crate::edict_pure::EvaluationLimits;
use crate::{
    AdmittedEchoOperationInvocation, AdmittedExecutableOperationPackage,
    EchoOperationAdmissionError, EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1,
    EchoOperationBudgetV1, EchoOperationEvaluationBasisV1, EchoOperationInstallationErrorV1,
    EchoOperationInvocationAdmissionError, EchoOperationInvocationAdmissionPolicy, Engine,
    HistoryError, InstalledEchoOperation, ProvenanceService, RuntimeError, TypeId,
    WorldlineFrontier, WorldlineRuntime, WriterHeadKey,
};

/// Error returned by [`EffectPackageHost`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EffectPackageHostError {
    /// Provenance could not register a runtime worldline.
    #[error("effect package host provenance error: {0}")]
    Provenance(#[from] HistoryError),
    /// The runtime could not resolve the writer head or its worldline.
    #[error("effect package host runtime error: {0}")]
    Runtime(Box<RuntimeError>),
    /// A recording frame could not bind its parent or resource limits.
    #[error("effect package host effect frame setup error: {0}")]
    EffectFrameSetup(#[from] EffectFrameSetupError),
    /// Installation conflicted with an installed package or operation.
    #[error("effect package host installation error: {0}")]
    EchoOperationInstallation(#[from] EchoOperationInstallationErrorV1),
}

impl From<RuntimeError> for EffectPackageHostError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(Box::new(error))
    }
}

/// Registers every runtime worldline with a fresh provenance service.
pub(crate) fn provenance_from_runtime(
    runtime: &WorldlineRuntime,
) -> Result<ProvenanceService, HistoryError> {
    let mut provenance = ProvenanceService::new();
    for (worldline_id, frontier) in runtime.worldlines().iter() {
        provenance.register_worldline(*worldline_id, frontier.state())?;
    }
    Ok(provenance)
}

/// Borrowed runtime-owner state that the effect-package route reads.
///
/// Both hosts build this view over their own runtime, provenance, engine and
/// evaluation authority, so they share one implementation of each step.
pub(crate) struct EffectPackageRoute<'a> {
    pub(crate) runtime: &'a WorldlineRuntime,
    pub(crate) provenance: &'a ProvenanceService,
    pub(crate) engine: &'a Engine,
    pub(crate) authority: &'a EchoOperationEvaluationAuthorityV1,
}

impl<'a> EffectPackageRoute<'a> {
    pub(crate) fn evaluation_basis(
        &self,
        writer_head: WriterHeadKey,
        application_basis: EchoOperationApplicationBasisV1,
    ) -> Result<EchoOperationEvaluationBasisV1, RuntimeError> {
        resolve_echo_operation_evaluation_basis_v1(
            self.runtime,
            self.provenance,
            writer_head,
            application_basis,
        )
    }

    pub(crate) fn evaluation_frame(
        &self,
        expected: EchoOperationEvaluationBasisV1,
        allowed_range_types: &[TypeId],
        max_range_edges: u64,
        budget: EchoOperationBudgetV1,
        limits: EvaluationLimits,
    ) -> Result<EffectEvaluationFrame<'a>, EffectPackageHostError> {
        let current =
            self.evaluation_basis(expected.writer_head(), expected.application_basis())?;
        if current != expected {
            return Err(EffectFrameSetupError::BasisMismatch.into());
        }
        let worldline_id = current.writer_head().worldline_id;
        let frontier = self
            .runtime
            .worldlines()
            .get(&worldline_id)
            .ok_or(RuntimeError::UnknownWorldline(worldline_id))?;
        let reads =
            EffectRangeReadSession::new(frontier, allowed_range_types, max_range_edges, budget);
        Ok(EffectEvaluationFrame::at_parent(
            reads,
            limits,
            current,
            self.authority.clone(),
        )?)
    }

    pub(crate) fn derive_effects(
        &self,
        admitted: &AdmittedEchoOperationInvocation,
        completed: CompletedEffectEvaluation,
    ) -> Result<DerivedEffectPatch, EffectDerivationError> {
        let fail = |kind| EffectDerivationError::new(kind, EchoOperationBudgetV1::new(0, 0, 0));
        if !admitted.is_owned_by(self.authority) || !completed.is_owned_by(self.authority) {
            return Err(fail(EffectDerivationKind::AuthorityMismatch));
        }
        let installed = self
            .engine
            .installed_echo_operation_package(admitted.package_id())
            .filter(|installed| {
                installed.installed_operation_id() == admitted.installed_operation_id()
            })
            .ok_or_else(|| fail(EffectDerivationKind::OperationUnavailable))?;
        let program = installed
            .program()
            .as_effect_program()
            .ok_or_else(|| fail(EffectDerivationKind::OperationUnavailable))?;
        if completed.program_identity() != installed.program_identity()
            || completed.evaluation_basis() != admitted.evaluation_basis()
        {
            return Err(fail(EffectDerivationKind::OriginMismatch));
        }
        let basis = admitted.evaluation_basis();
        let current = self
            .evaluation_basis(basis.writer_head(), basis.application_basis())
            .map_err(|_| fail(EffectDerivationKind::BasisChanged))?;
        if current != basis {
            return Err(fail(EffectDerivationKind::BasisChanged));
        }
        let state = self
            .runtime
            .worldlines()
            .get(&current.writer_head().worldline_id)
            .ok_or_else(|| fail(EffectDerivationKind::BasisChanged))?
            .state();
        let invocation = admitted
            .invocation()
            .identity()
            .map_err(|_| fail(EffectDerivationKind::OriginMismatch))?
            .as_hash();
        derive(
            state,
            program,
            completed,
            admitted.invocation().delegated_budget(),
            self.engine.echo_operation_policy_id(),
            installed.identity(),
            invocation,
        )
    }

    pub(crate) fn admit_invocation(
        &self,
        policy: &EchoOperationInvocationAdmissionPolicy,
        canonical_invocation_bytes: &[u8],
    ) -> Result<AdmittedEchoOperationInvocation, EchoOperationInvocationAdmissionError> {
        let (package_id, claimed_basis) = decode_invocation_route_v1(canonical_invocation_bytes)?;
        let current_basis = self
            .evaluation_basis(
                claimed_basis.writer_head(),
                claimed_basis.application_basis(),
            )
            .map_err(|error| invocation_runtime_error(error.to_string()))?;
        let current_state = self
            .runtime
            .worldlines()
            .get(&claimed_basis.writer_head().worldline_id)
            .map(WorldlineFrontier::state)
            .ok_or_else(|| {
                invocation_runtime_error("operation invocation worldline is unavailable")
            })?;
        admit_invocation_v1(
            self.engine.installed_echo_operation_package(package_id),
            *policy,
            canonical_invocation_bytes,
            current_basis,
            current_state,
            self.authority.clone(),
        )
    }
}

/// Trusted host that runs authored effect packages on generic Echo.
///
/// It is the `trusted_runtime` route for a consumer that executes an
/// authored Edict effect package without Echo's internal
/// `native_rule_bootstrap` feature. Each step is the runtime-owner step that
/// [`crate::TrustedRuntimeHost`] performs, under the same law:
///
/// 1. [`Self::admit_echo_operation_package`] admits exact package bytes under
///    an exact [`EchoOperationAdmissionPolicy`];
/// 2. [`Self::install_admitted_echo_operation_package`] installs the admitted
///    meaning, which still authorizes no invocation;
/// 3. [`Self::echo_operation_evaluation_basis_v1`] resolves the exact current
///    parent of a writer head;
/// 4. [`Self::admit_echo_operation_invocation`] admits one canonical
///    invocation under an exact [`EchoOperationInvocationAdmissionPolicy`];
/// 5. [`Self::effect_evaluation_frame_v1`] opens a recording frame at that
///    parent, and [`Self::derive_echo_operation_effects`] derives the
///    proposal from the completed evaluation without applying it.
///
/// The host has no scheduler, WAL, commit path or native rule authoring.
/// Its evaluation authority is private to this host value: evidence from
/// another host is refused.
pub struct EffectPackageHost {
    runtime: WorldlineRuntime,
    provenance: ProvenanceService,
    engine: Engine,
    authority: EchoOperationEvaluationAuthorityV1,
}

impl core::fmt::Debug for EffectPackageHost {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EffectPackageHost")
            .finish_non_exhaustive()
    }
}

impl EffectPackageHost {
    /// Builds a host over `runtime` and `engine`, registering every runtime
    /// worldline with fresh provenance.
    ///
    /// # Errors
    ///
    /// Returns [`EffectPackageHostError::Provenance`] if a runtime worldline
    /// cannot be registered.
    pub fn new(runtime: WorldlineRuntime, engine: Engine) -> Result<Self, EffectPackageHostError> {
        let provenance = provenance_from_runtime(&runtime)?;
        Ok(Self {
            runtime,
            provenance,
            engine,
            authority: EchoOperationEvaluationAuthorityV1::new(),
        })
    }

    /// Returns the host-owned runtime as read-only evidence.
    #[must_use]
    pub const fn runtime(&self) -> &WorldlineRuntime {
        &self.runtime
    }

    fn route(&self) -> EffectPackageRoute<'_> {
        EffectPackageRoute {
            runtime: &self.runtime,
            provenance: &self.provenance,
            engine: &self.engine,
            authority: &self.authority,
        }
    }

    /// Admits exact executable-operation package bytes under an exact policy.
    ///
    /// The result is an opaque admission token. It installs nothing, and the
    /// program digest inside the package confers no coordinate, invocability
    /// or authority.
    ///
    /// # Errors
    ///
    /// Refuses bytes whose identity, coordinate, authority profile, budget or
    /// structure differ from the policy.
    pub fn admit_echo_operation_package(
        &self,
        policy: &EchoOperationAdmissionPolicy,
        canonical_package_bytes: Vec<u8>,
    ) -> Result<AdmittedExecutableOperationPackage, EchoOperationAdmissionError> {
        admit_operation_package(policy, canonical_package_bytes)
    }

    /// Installs the exact meaning an admitted package carries.
    ///
    /// Installation stores no native callback and authorizes no invocation.
    /// Installing the same package again returns the installed operation.
    ///
    /// # Errors
    ///
    /// Refuses a package or operation coordinate that conflicts with one
    /// already installed; the catalog is unchanged on refusal.
    pub fn install_admitted_echo_operation_package(
        &mut self,
        admitted: AdmittedExecutableOperationPackage,
    ) -> Result<InstalledEchoOperation, EffectPackageHostError> {
        let installed = installed_from_admitted(admitted);
        let batch = core::slice::from_ref(&installed);
        self.engine
            .preflight_recovered_echo_operation_packages(batch)?;
        if self
            .engine
            .installed_echo_operation_package(installed.package_id())
            != Some(&installed)
        {
            self.engine
                .restore_recovered_echo_operation_packages(batch)?;
        }
        Ok(installed)
    }

    /// Resolves the exact current parent basis of `writer_head` for an
    /// invocation with `application_basis`.
    ///
    /// # Errors
    ///
    /// Refuses an unknown writer head or worldline, or unreadable provenance.
    pub fn echo_operation_evaluation_basis_v1(
        &self,
        writer_head: WriterHeadKey,
        application_basis: EchoOperationApplicationBasisV1,
    ) -> Result<EchoOperationEvaluationBasisV1, EffectPackageHostError> {
        self.route()
            .evaluation_basis(writer_head, application_basis)
            .map_err(EffectPackageHostError::from)
    }

    /// Admits one canonical invocation of an installed operation.
    ///
    /// Admission checks the exact installed package, operation coordinate,
    /// authority profile and grant, delegated budget and current parent
    /// basis. It neither evaluates nor mutates the graph.
    ///
    /// # Errors
    ///
    /// Refuses malformed bytes, an uninstalled package, a policy mismatch or
    /// a stale parent basis.
    pub fn admit_echo_operation_invocation(
        &self,
        policy: &EchoOperationInvocationAdmissionPolicy,
        canonical_invocation_bytes: &[u8],
    ) -> Result<AdmittedEchoOperationInvocation, EchoOperationInvocationAdmissionError> {
        self.route()
            .admit_invocation(policy, canonical_invocation_bytes)
    }

    /// Opens a recording evaluation frame at the exact current parent.
    ///
    /// The host resolves the parent itself and lends the frame its own
    /// frontier; it never accepts caller-supplied storage. The shared borrow
    /// keeps the graph fixed while the frame lives. This is not package
    /// admission, installed dispatch or a mutation path.
    ///
    /// # Errors
    ///
    /// Refuses an unknown writer, a stale or substituted parent, or invalid
    /// resource declarations before recording any read.
    pub fn effect_evaluation_frame_v1(
        &self,
        expected: EchoOperationEvaluationBasisV1,
        allowed_range_types: &[TypeId],
        max_range_edges: u64,
        budget: EchoOperationBudgetV1,
        limits: EvaluationLimits,
    ) -> Result<EffectEvaluationFrame<'_>, EffectPackageHostError> {
        self.route().evaluation_frame(
            expected,
            allowed_range_types,
            max_range_edges,
            budget,
            limits,
        )
    }

    /// Derives the patch a completed evaluation proposes, without applying it.
    ///
    /// # Errors
    ///
    /// Refuses evidence from another host, an operation no longer installed,
    /// evidence whose origin differs from the invocation, a changed parent
    /// and invalid mutation proposals.
    pub fn derive_echo_operation_effects(
        &self,
        admitted: &AdmittedEchoOperationInvocation,
        completed: CompletedEffectEvaluation,
    ) -> Result<DerivedEffectPatch, EffectDerivationError> {
        self.route().derive_effects(admitted, completed)
    }
}
