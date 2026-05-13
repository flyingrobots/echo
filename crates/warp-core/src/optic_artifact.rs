// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-owned registry for Wesley-compiled optic artifacts.
//!
//! This module owns optic artifact registration and the first admission-only
//! invocation gate. [`OpticArtifactRegistry::admit_optic_invocation`] resolves
//! handles internally, checks operation identity, and reports obstruction in a
//! ticket-shaped pre-admission posture without validating grants, issuing
//! success tickets, emitting law witnesses, or executing runtime work. A
//! capability presentation slot is not authority; every presentation posture
//! obstructs until Echo can validate a real bounded grant.

use std::collections::BTreeMap;

use thiserror::Error;

/// Echo-owned handle kind for registered optic artifacts.
pub const OPTIC_ARTIFACT_HANDLE_KIND: &str = "optic-artifact-handle";

/// Echo-owned kind for a ticket-shaped pre-admission obstruction posture.
pub const OPTIC_ADMISSION_TICKET_POSTURE_KIND: &str = "optic-admission-ticket-posture";

const OPTIC_ARTIFACT_HANDLE_ID_PREFIX: &str = "optic-artifact-handle:";

/// Opaque Echo-owned runtime handle for a registered optic artifact.
///
/// The handle proves registration, not authority. It is not a capability grant,
/// not a basis, and not permission to invoke the operation.
/// Handle ids are runtime-local registry identifiers and are not content hashes,
/// capabilities, or stable cross-runtime references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticArtifactHandle {
    /// Stable discriminator for callers and wire adapters.
    pub kind: String,
    /// Runtime-local opaque identifier.
    pub id: String,
}

/// Wesley-compiled operation identity carried by an optic artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticArtifactOperation {
    /// Stable operation id compiled by Wesley.
    pub operation_id: String,
}

/// Opaque admission requirements compiled by Wesley and stored by Echo.
///
/// Echo stores these requirements at registration time. Invocation-time callers
/// must not provide replacement requirements or footprint law.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticAdmissionRequirements {
    /// Opaque requirement bytes. Later slices can replace this fixture-shaped
    /// payload with shared Wesley/Continuum types without changing ownership.
    pub bytes: Vec<u8>,
}

/// Wesley-compiled optic artifact as consumed by Echo registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticArtifact {
    /// Wesley artifact id.
    pub artifact_id: String,
    /// Content/address hash for the compiled artifact.
    pub artifact_hash: String,
    /// Schema identity used to compile the artifact.
    pub schema_id: String,
    /// Digest of admission requirements and law claims.
    pub requirements_digest: String,
    /// Compiled operation identity.
    pub operation: OpticArtifactOperation,
    /// Compiled requirements to store inside Echo.
    pub requirements: OpticAdmissionRequirements,
}

/// Wesley-owned registration descriptor presented to Echo.
///
/// This is not an Echo runtime handle. Echo verifies this descriptor against
/// the artifact, stores the artifact requirements, and returns its own
/// [`OpticArtifactHandle`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticRegistrationDescriptor {
    /// Wesley artifact id.
    pub artifact_id: String,
    /// Content/address hash for the compiled artifact.
    pub artifact_hash: String,
    /// Schema identity used to compile the artifact.
    pub schema_id: String,
    /// Stable operation id compiled by Wesley.
    pub operation_id: String,
    /// Digest of admission requirements and law claims.
    pub requirements_digest: String,
}

/// Echo-owned registered artifact metadata and stored requirements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredOpticArtifact {
    /// Echo-owned runtime-local handle.
    pub handle: OpticArtifactHandle,
    /// Wesley artifact id.
    pub artifact_id: String,
    /// Verified artifact hash.
    pub artifact_hash: String,
    /// Verified schema id.
    pub schema_id: String,
    /// Verified operation id.
    pub operation_id: String,
    /// Verified requirements digest.
    pub requirements_digest: String,
    /// Requirements stored internally by Echo at registration time.
    pub requirements: OpticAdmissionRequirements,
}

/// Opaque basis request bytes supplied at optic invocation time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticBasisRequest {
    /// Request bytes interpreted only below Echo's runtime admission boundary.
    pub bytes: Vec<u8>,
}

/// Opaque aperture request bytes supplied at optic invocation time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticApertureRequest {
    /// Request bytes interpreted only below Echo's runtime admission boundary.
    pub bytes: Vec<u8>,
}

/// Placeholder capability presentation supplied at optic invocation time.
///
/// This v0 shape is intentionally not sufficient to authorize invocation. It
/// exists only so the admission skeleton can classify presentation posture
/// without inventing grant validation semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticCapabilityPresentation {
    /// Presentation identity supplied by the caller.
    pub presentation_id: String,
    /// Grant id the presentation claims to bind to.
    ///
    /// Echo does not validate this grant in this slice. A non-empty value only
    /// moves the presentation from unbound to validation-unavailable
    /// obstruction; it never authorizes invocation.
    pub bound_grant_id: Option<String>,
}

/// Opaque principal reference used by authority boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrincipalRef {
    /// Principal identity bytes encoded by the authority layer.
    pub id: String,
}

/// Authority policy selected for grant-intent evaluation.
///
/// No policy is implemented in this slice. The shape exists so Echo can name
/// the meta-authority boundary without granting success.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityPolicy {
    /// Authority policy identity.
    pub policy_id: String,
}

/// Obstruction-only authority policy evaluation posture.
///
/// This is vocabulary, not governance. It lets Echo name policy failure
/// surfaces without accepting a grant intent, issuing a receipt, or treating a
/// policy shape as trusted authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityPolicyEvaluation {
    /// The proposed delegation basis is not valid for the authority change.
    InvalidDelegation,
    /// The proposed grant would exceed the issuer's authority scope.
    ScopeEscalation,
    /// Echo does not have a supported authority policy implementation yet.
    Unsupported,
}

/// Authority context supplied when proposing a capability grant intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityContext {
    /// Principal whose prior authority is claimed for proposing the grant.
    pub issuer: Option<PrincipalRef>,
    /// Policy that should evaluate the issuer's authority.
    pub policy: Option<AuthorityPolicy>,
    /// Obstruction-only policy evaluation posture.
    pub policy_evaluation: AuthorityPolicyEvaluation,
}

/// Causal authority intent submitted to Echo for future grant admission.
///
/// A grant intent proposes authority; it is not admitted authority. No
/// principal can mint authority from nowhere. Future slices must authorize the
/// proposer through prior authority, host root policy, quorum, or governance
/// rule before any accepted grant receipt can exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityGrantIntent {
    /// Deterministic intent id used for replay/duplicate obstruction.
    pub intent_id: String,
    /// Principal proposing the authority change.
    pub proposed_by: PrincipalRef,
    /// Subject that would receive authority if a future policy admits it.
    pub subject: PrincipalRef,
    /// Compiled artifact hash the proposed grant would cover.
    pub artifact_hash: String,
    /// Operation id the proposed grant would cover.
    pub operation_id: String,
    /// Requirements digest the proposed grant would cover.
    pub requirements_digest: String,
    /// Rights named by the authority layer.
    pub rights: Vec<String>,
    /// Opaque scope bytes proposed for later validation.
    pub scope_bytes: Vec<u8>,
    /// Opaque expiry bytes proposed for later validation.
    pub expiry_bytes: Option<Vec<u8>>,
    /// Opaque delegation-basis bytes proposed for later validation.
    pub delegation_basis_bytes: Option<Vec<u8>>,
}

/// Obstruction reason for a capability grant intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityGrantIntentObstruction {
    /// The issuer has no authority context from which to propose the grant.
    MissingIssuerAuthority,
    /// The grant intent is structurally unusable.
    MalformedGrantIntent,
    /// The proposed delegation basis is invalid.
    InvalidDelegation,
    /// The proposed scope would exceed the issuer's authority.
    ScopeEscalation,
    /// Echo already saw a grant intent with the supplied intent id.
    ReplayOrDuplicateIntent,
    /// No real authority policy exists in this slice.
    UnsupportedAuthorityPolicy,
}

/// Obstructed posture for a submitted capability grant intent.
///
/// This is not an admitted grant receipt and does not make the grant authority.
/// It carries enough context for future admission/witness code to explain why
/// Echo did not admit the grant intent into witnessed history.
#[must_use = "capability grant intent postures explain obstructions that must be handled"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityGrantIntentPosture {
    /// Stable discriminator for callers and wire adapters.
    pub kind: String,
    /// Intent id named by the grant intent.
    pub intent_id: String,
    /// Principal proposing the authority change.
    pub proposed_by: PrincipalRef,
    /// Subject that would receive authority if the intent were admitted.
    pub subject: PrincipalRef,
    /// Structured reason Echo obstructed before admitting the grant.
    pub obstruction: CapabilityGrantIntentObstruction,
}

/// Submission outcome for a capability grant intent skeleton.
#[must_use = "capability grant intent outcomes carry obstructions that must be handled"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityGrantIntentOutcome {
    /// Echo obstructed the grant intent before admitting authority.
    Obstructed(CapabilityGrantIntentPosture),
}

/// Runtime invocation request against a registered optic artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticInvocation {
    /// Echo-owned runtime-local artifact handle.
    pub artifact_handle: OpticArtifactHandle,
    /// Operation id the caller intends to invoke.
    pub operation_id: String,
    /// Digest of canonical invocation variable bytes.
    pub canonical_variables_digest: Vec<u8>,
    /// Requested causal basis for the invocation.
    pub basis_request: OpticBasisRequest,
    /// Requested aperture for the invocation.
    pub aperture_request: OpticApertureRequest,
    /// Caller authority presentation. Registration alone is not authority.
    pub capability_presentation: Option<OpticCapabilityPresentation>,
}

/// Admission obstruction for an optic invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpticInvocationObstruction {
    /// Echo did not issue or cannot resolve the artifact handle.
    UnknownHandle,
    /// The invocation operation id does not match the registered artifact.
    OperationMismatch,
    /// The invocation does not carry authority to use the registered artifact.
    MissingCapability,
    /// The invocation carries a presentation that is structurally unusable.
    MalformedCapabilityPresentation,
    /// The invocation carries a presentation that is not bound to any grant.
    UnboundCapabilityPresentation,
    /// A placeholder presentation was supplied, but real grant validation does
    /// not exist in this slice.
    CapabilityValidationUnavailable,
}

/// Ticket-shaped pre-admission posture for an obstructed optic invocation.
///
/// This is not a successful admission ticket and does not authorize runtime
/// execution. It carries enough invocation context for callers and later
/// witness code to explain why Echo obstructed before grant validation exists.
#[must_use = "optic admission ticket postures explain obstructions that must be handled"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticAdmissionTicketPosture {
    /// Stable discriminator for callers and wire adapters.
    pub kind: String,
    /// Echo-owned runtime-local artifact handle used by the invocation.
    pub artifact_handle: OpticArtifactHandle,
    /// Operation id the caller requested.
    pub operation_id: String,
    /// Digest of canonical invocation variable bytes.
    pub canonical_variables_digest: Vec<u8>,
    /// Requested causal basis for the invocation.
    pub basis_request: OpticBasisRequest,
    /// Requested aperture for the invocation.
    pub aperture_request: OpticApertureRequest,
    /// Structured reason Echo obstructed before runtime execution.
    pub obstruction: OpticInvocationObstruction,
}

/// Admission outcome for a v0 optic invocation skeleton.
#[must_use = "optic invocation admission outcomes carry obstructions that must be handled"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpticInvocationAdmissionOutcome {
    /// Echo obstructed the invocation before issuing any success ticket.
    Obstructed(OpticAdmissionTicketPosture),
}

/// Registration and lookup errors for Echo optic artifact handles.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OpticArtifactRegistrationError {
    /// Descriptor artifact id did not match the artifact.
    #[error("optic artifact id mismatch")]
    ArtifactIdMismatch,
    /// Descriptor artifact hash did not match the artifact.
    #[error("optic artifact hash mismatch")]
    ArtifactHashMismatch,
    /// Descriptor requirements digest did not match the artifact.
    #[error("optic artifact requirements digest mismatch")]
    RequirementsDigestMismatch,
    /// Descriptor operation id did not match the artifact operation id.
    #[error("optic artifact operation id mismatch")]
    OperationIdMismatch,
    /// Descriptor schema id did not match the artifact schema id.
    #[error("optic artifact schema id mismatch")]
    SchemaIdMismatch,
    /// Echo could not resolve the opaque artifact handle.
    #[error("unknown optic artifact handle")]
    UnknownHandle,
}

/// Echo-owned deterministic intake for capability grant intents.
///
/// This registry records submitted grant intents so duplicate intent ids can be
/// obstructed deterministically. It does not validate grant applicability,
/// admit grants into witnessed history, issue admission tickets, emit law
/// witnesses, or execute runtime work.
#[derive(Clone, Debug, Default)]
pub struct CapabilityGrantIntentGate {
    intents_by_id: BTreeMap<String, CapabilityGrantIntent>,
}

impl CapabilityGrantIntentGate {
    /// Creates an empty capability grant intent gate.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Submits a capability grant intent for obstruction classification.
    ///
    /// This v0 skeleton intentionally has no success path. Well-formed unique
    /// intents are recorded as submitted intent material, but still obstruct
    /// because grant admission/witnessing does not exist in this slice.
    #[must_use = "capability grant intent outcomes carry obstructions that must be handled"]
    pub fn submit_grant_intent(
        &mut self,
        intent: CapabilityGrantIntent,
        authority_context: AuthorityContext,
    ) -> CapabilityGrantIntentOutcome {
        let obstruction = self.classify_capability_grant_intent(&intent, &authority_context);
        if Self::records_submitted_intent(obstruction) {
            self.intents_by_id
                .insert(intent.intent_id.clone(), intent.clone());
        }

        Self::obstructed_grant_intent(&intent, obstruction)
    }

    /// Returns the number of submitted grant intents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.intents_by_id.len()
    }

    /// Returns `true` if no grant intents are recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.intents_by_id.is_empty()
    }

    fn classify_capability_grant_intent(
        &self,
        intent: &CapabilityGrantIntent,
        authority_context: &AuthorityContext,
    ) -> CapabilityGrantIntentObstruction {
        if intent.intent_id.is_empty()
            || intent.proposed_by.id.is_empty()
            || intent.subject.id.is_empty()
            || intent.artifact_hash.is_empty()
            || intent.operation_id.is_empty()
            || intent.requirements_digest.is_empty()
            || intent.rights.is_empty()
            || intent.rights.iter().any(String::is_empty)
            || intent.scope_bytes.is_empty()
            || intent.expiry_bytes.as_ref().is_some_and(Vec::is_empty)
            || intent
                .delegation_basis_bytes
                .as_ref()
                .is_some_and(Vec::is_empty)
        {
            return CapabilityGrantIntentObstruction::MalformedGrantIntent;
        }

        if self.intents_by_id.contains_key(&intent.intent_id) {
            return CapabilityGrantIntentObstruction::ReplayOrDuplicateIntent;
        }

        let Some(issuer) = &authority_context.issuer else {
            return CapabilityGrantIntentObstruction::MissingIssuerAuthority;
        };
        if issuer.id.is_empty() || issuer != &intent.proposed_by {
            return CapabilityGrantIntentObstruction::MissingIssuerAuthority;
        }

        if authority_context.policy.is_none() {
            return CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy;
        }

        match authority_context.policy_evaluation {
            AuthorityPolicyEvaluation::InvalidDelegation => {
                CapabilityGrantIntentObstruction::InvalidDelegation
            }
            AuthorityPolicyEvaluation::ScopeEscalation => {
                CapabilityGrantIntentObstruction::ScopeEscalation
            }
            AuthorityPolicyEvaluation::Unsupported => {
                CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
            }
        }
    }

    fn records_submitted_intent(obstruction: CapabilityGrantIntentObstruction) -> bool {
        matches!(
            obstruction,
            CapabilityGrantIntentObstruction::InvalidDelegation
                | CapabilityGrantIntentObstruction::ScopeEscalation
                | CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
        )
    }

    fn obstructed_grant_intent(
        intent: &CapabilityGrantIntent,
        obstruction: CapabilityGrantIntentObstruction,
    ) -> CapabilityGrantIntentOutcome {
        CapabilityGrantIntentOutcome::Obstructed(CapabilityGrantIntentPosture {
            kind: "capability-grant-intent-posture".to_owned(),
            intent_id: intent.intent_id.clone(),
            proposed_by: intent.proposed_by.clone(),
            subject: intent.subject.clone(),
            obstruction,
        })
    }
}

/// Echo-owned runtime-local registry for Wesley-compiled optic artifacts.
#[derive(Clone, Debug, Default)]
pub struct OpticArtifactRegistry {
    next_handle_index: u64,
    artifacts_by_handle: BTreeMap<String, RegisteredOpticArtifact>,
}

impl OpticArtifactRegistry {
    /// Creates an empty optic artifact registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a Wesley-compiled artifact and returns an opaque Echo handle.
    ///
    /// # Errors
    ///
    /// Returns a registration error if the Wesley descriptor does not match the
    /// artifact identity or requirements digest.
    pub fn register_optic_artifact(
        &mut self,
        artifact: OpticArtifact,
        descriptor: OpticRegistrationDescriptor,
    ) -> Result<OpticArtifactHandle, OpticArtifactRegistrationError> {
        Self::verify_descriptor(&artifact, &descriptor)?;

        let handle = self.next_handle();
        let registered = RegisteredOpticArtifact {
            handle: handle.clone(),
            artifact_id: artifact.artifact_id,
            artifact_hash: artifact.artifact_hash,
            schema_id: artifact.schema_id,
            operation_id: artifact.operation.operation_id,
            requirements_digest: artifact.requirements_digest,
            requirements: artifact.requirements,
        };
        self.artifacts_by_handle
            .insert(handle.id.clone(), registered);

        Ok(handle)
    }

    /// Resolves an opaque Echo handle to registered artifact metadata.
    ///
    /// # Errors
    ///
    /// Returns [`OpticArtifactRegistrationError::UnknownHandle`] if Echo did not
    /// issue the handle in this registry instance.
    pub fn resolve_optic_artifact_handle(
        &self,
        handle: &OpticArtifactHandle,
    ) -> Result<&RegisteredOpticArtifact, OpticArtifactRegistrationError> {
        if handle.kind != OPTIC_ARTIFACT_HANDLE_KIND {
            return Err(OpticArtifactRegistrationError::UnknownHandle);
        }
        self.artifacts_by_handle
            .get(&handle.id)
            .ok_or(OpticArtifactRegistrationError::UnknownHandle)
    }

    /// Admits or obstructs an invocation against a registered optic artifact.
    ///
    /// This v0 skeleton intentionally has no success path. It proves that Echo
    /// resolves handles internally, that a registered handle is not authority,
    /// and that obstruction is reported as a structured pre-ticket posture.
    #[must_use = "optic invocation admission outcomes carry obstructions that must be handled"]
    pub fn admit_optic_invocation(
        &self,
        invocation: &OpticInvocation,
    ) -> OpticInvocationAdmissionOutcome {
        let Ok(registered) = self.resolve_optic_artifact_handle(&invocation.artifact_handle) else {
            return Self::obstructed_invocation(
                invocation,
                OpticInvocationObstruction::UnknownHandle,
            );
        };

        if invocation.operation_id != registered.operation_id {
            return Self::obstructed_invocation(
                invocation,
                OpticInvocationObstruction::OperationMismatch,
            );
        }

        Self::obstructed_invocation(
            invocation,
            Self::classify_capability_presentation(invocation.capability_presentation.as_ref()),
        )
    }

    fn classify_capability_presentation(
        presentation: Option<&OpticCapabilityPresentation>,
    ) -> OpticInvocationObstruction {
        let Some(presentation) = presentation else {
            return OpticInvocationObstruction::MissingCapability;
        };

        if presentation.presentation_id.is_empty()
            || presentation
                .bound_grant_id
                .as_ref()
                .is_some_and(String::is_empty)
        {
            return OpticInvocationObstruction::MalformedCapabilityPresentation;
        }

        if presentation.bound_grant_id.is_none() {
            return OpticInvocationObstruction::UnboundCapabilityPresentation;
        }

        OpticInvocationObstruction::CapabilityValidationUnavailable
    }

    fn obstructed_invocation(
        invocation: &OpticInvocation,
        obstruction: OpticInvocationObstruction,
    ) -> OpticInvocationAdmissionOutcome {
        OpticInvocationAdmissionOutcome::Obstructed(OpticAdmissionTicketPosture {
            kind: OPTIC_ADMISSION_TICKET_POSTURE_KIND.to_owned(),
            artifact_handle: invocation.artifact_handle.clone(),
            operation_id: invocation.operation_id.clone(),
            canonical_variables_digest: invocation.canonical_variables_digest.clone(),
            basis_request: invocation.basis_request.clone(),
            aperture_request: invocation.aperture_request.clone(),
            obstruction,
        })
    }

    /// Returns the number of registered artifacts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.artifacts_by_handle.len()
    }

    /// Returns `true` if no artifacts are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.artifacts_by_handle.is_empty()
    }

    fn verify_descriptor(
        artifact: &OpticArtifact,
        descriptor: &OpticRegistrationDescriptor,
    ) -> Result<(), OpticArtifactRegistrationError> {
        if descriptor.artifact_id != artifact.artifact_id {
            return Err(OpticArtifactRegistrationError::ArtifactIdMismatch);
        }
        if descriptor.artifact_hash != artifact.artifact_hash {
            return Err(OpticArtifactRegistrationError::ArtifactHashMismatch);
        }
        if descriptor.requirements_digest != artifact.requirements_digest {
            return Err(OpticArtifactRegistrationError::RequirementsDigestMismatch);
        }
        if descriptor.operation_id != artifact.operation.operation_id {
            return Err(OpticArtifactRegistrationError::OperationIdMismatch);
        }
        if descriptor.schema_id != artifact.schema_id {
            return Err(OpticArtifactRegistrationError::SchemaIdMismatch);
        }
        Ok(())
    }

    fn next_handle(&mut self) -> OpticArtifactHandle {
        self.next_handle_index = self.next_handle_index.saturating_add(1);
        OpticArtifactHandle {
            kind: OPTIC_ARTIFACT_HANDLE_KIND.to_owned(),
            id: format!(
                "{OPTIC_ARTIFACT_HANDLE_ID_PREFIX}{:016x}",
                self.next_handle_index
            ),
        }
    }
}
