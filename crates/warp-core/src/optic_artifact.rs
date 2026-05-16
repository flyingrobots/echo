// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-owned registry for Wesley-compiled optic artifacts.
//!
//! This module owns optic artifact registration and the first admission-only
//! invocation gate. [`OpticArtifactRegistry::admit_optic_invocation`] resolves
//! handles internally, checks operation identity, and reports obstruction in a
//! ticket-shaped pre-admission posture without wiring grant validation into
//! invocation admission, issuing success tickets, emitting law witnesses, or
//! executing runtime work. A capability presentation slot is not authority;
//! every presentation posture obstructs until Echo can validate a real bounded
//! grant and admit authority.

use std::collections::BTreeMap;

use crate::{
    digest_invocation_request_bytes, ArtifactRegistrationObstructionKind,
    ArtifactRegistrationReceipt, CapabilityGrantValidationObstructionKind, GraphFact,
    InvocationObstructionKind, PublishedGraphFact, ARTIFACT_REGISTRATION_RECEIPT_KIND,
};
use thiserror::Error;

/// Echo-owned handle kind for registered optic artifacts.
pub const OPTIC_ARTIFACT_HANDLE_KIND: &str = "optic-artifact-handle";

/// Echo-owned kind for a ticket-shaped pre-admission obstruction posture.
pub const OPTIC_ADMISSION_TICKET_POSTURE_KIND: &str = "optic-admission-ticket-posture";

/// Echo-owned kind for a causal refusal receipt.
pub const OBSTRUCTION_RECEIPT_KIND: &str = "obstruction-receipt";

/// Echo-owned kind for capability grant validation obstruction posture.
pub const CAPABILITY_GRANT_VALIDATION_POSTURE_KIND: &str = "capability-grant-validation-posture";

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
    /// Explicit codec id for the opaque requirement bytes.
    pub codec: String,
    /// Wesley-computed digest of the opaque requirement bytes.
    pub digest: String,
    /// Opaque requirement bytes emitted by Wesley.
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
/// without treating a presentation as authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpticCapabilityPresentation {
    /// Presentation identity supplied by the caller.
    pub presentation_id: String,
    /// Grant id the presentation claims to bind to.
    ///
    /// [`OpticArtifactRegistry::admit_optic_invocation`] does not validate this
    /// grant. The validator-aware invocation path may validate it only to
    /// publish sharper refusal evidence. A non-empty value never authorizes
    /// invocation in this slice.
    pub bound_grant_id: Option<String>,
}

/// Opaque principal reference used by authority boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrincipalRef {
    /// Principal identity bytes encoded by the authority layer.
    pub id: String,
}

/// Disposition for submitted rewrite material after Echo evaluates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewriteDisposition {
    /// Echo selected and committed the rewrite.
    Committed,
    /// Echo admitted the rewrite as legal, but the scheduler did not select it.
    LegalUnselectedCounterfactual,
    /// Echo refused the intent before admission.
    Obstructed,
}

impl RewriteDisposition {
    fn receipt_label(self) -> &'static str {
        match self {
            Self::Committed => "rewrite-disposition.committed",
            Self::LegalUnselectedCounterfactual => {
                "rewrite-disposition.legal-unselected-counterfactual"
            }
            Self::Obstructed => "rewrite-disposition.obstructed",
        }
    }
}

/// Causal receipt for a refused intent.
///
/// Refusal is causal evidence, not an unrealized legal world. An
/// [`ObstructionReceipt`] is not an admission ticket, not a law witness, and not
/// a counterfactual candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObstructionReceipt {
    /// Stable discriminator for callers and wire adapters.
    pub kind: String,
    /// Intent id named by the refused intent.
    pub intent_id: String,
    /// Principal that proposed the refused intent.
    pub proposed_by: PrincipalRef,
    /// Subject named by the refused intent.
    pub subject: PrincipalRef,
    /// Artifact hash named by the refused intent.
    pub artifact_hash: String,
    /// Operation id named by the refused intent.
    pub operation_id: String,
    /// Requirements digest named by the refused intent.
    pub requirements_digest: String,
    /// Authority policy id supplied with refusal context, if any.
    pub policy_id: Option<String>,
    /// Obstruction-only policy evaluation posture.
    pub policy_posture: String,
    /// Structured obstruction kind encoded for receipt consumers.
    pub obstruction_kind: String,
    /// Rewrite disposition. Obstruction receipts must remain obstructed.
    pub disposition: RewriteDisposition,
    /// BLAKE3 digest of [`ObstructionReceipt::build_receipt_input_bytes`].
    pub receipt_digest: [u8; 32],
}

impl ObstructionReceipt {
    /// Creates a causal refusal receipt for a capability grant intent.
    #[must_use]
    pub fn for_capability_grant_intent(
        intent: &CapabilityGrantIntent,
        authority_context: &AuthorityContext,
        obstruction: CapabilityGrantIntentObstruction,
    ) -> Self {
        let obstruction_kind = obstruction.receipt_label().to_owned();
        let policy_id = authority_context
            .policy
            .as_ref()
            .map(|policy| policy.policy_id.clone());
        let policy_posture = authority_context
            .policy_evaluation
            .receipt_label()
            .to_owned();
        let mut receipt = Self {
            kind: OBSTRUCTION_RECEIPT_KIND.to_owned(),
            intent_id: intent.intent_id.clone(),
            proposed_by: intent.proposed_by.clone(),
            subject: intent.subject.clone(),
            artifact_hash: intent.artifact_hash.clone(),
            operation_id: intent.operation_id.clone(),
            requirements_digest: intent.requirements_digest.clone(),
            policy_id,
            policy_posture,
            obstruction_kind,
            disposition: RewriteDisposition::Obstructed,
            receipt_digest: [0_u8; 32],
        };
        receipt.receipt_digest = *blake3::hash(&receipt.build_receipt_input_bytes()).as_bytes();
        receipt
    }

    /// Rebuilds the deterministic receipt input bytes represented by this
    /// receipt.
    ///
    /// The input bytes are intentionally not stored on the receipt. Consumers
    /// that need to verify [`ObstructionReceipt::receipt_digest`] can rebuild
    /// the exact digest input on demand.
    #[must_use]
    pub fn build_receipt_input_bytes(&self) -> Vec<u8> {
        let disposition = self.disposition.receipt_label();
        let policy_id = self.policy_id.as_deref();
        let mut bytes = Vec::with_capacity(self.receipt_input_capacity(disposition));

        push_receipt_field(&mut bytes, self.kind.as_bytes());
        push_receipt_field(&mut bytes, disposition.as_bytes());
        push_receipt_field(&mut bytes, self.intent_id.as_bytes());
        push_receipt_field(&mut bytes, self.proposed_by.id.as_bytes());
        push_receipt_field(&mut bytes, self.subject.id.as_bytes());
        push_receipt_field(&mut bytes, self.artifact_hash.as_bytes());
        push_receipt_field(&mut bytes, self.operation_id.as_bytes());
        push_receipt_field(&mut bytes, self.requirements_digest.as_bytes());
        push_optional_receipt_field(&mut bytes, policy_id.map(str::as_bytes));
        push_receipt_field(&mut bytes, self.policy_posture.as_bytes());
        push_receipt_field(&mut bytes, self.obstruction_kind.as_bytes());
        bytes
    }

    fn receipt_input_capacity(&self, disposition: &str) -> usize {
        const LENGTH_PREFIX_BYTES: usize = 8;
        const OPTIONAL_TAG_BYTES: usize = 1;
        const PLAIN_FIELD_COUNT: usize = 10;

        (PLAIN_FIELD_COUNT * LENGTH_PREFIX_BYTES)
            + OPTIONAL_TAG_BYTES
            + self.kind.len()
            + disposition.len()
            + self.intent_id.len()
            + self.proposed_by.id.len()
            + self.subject.id.len()
            + self.artifact_hash.len()
            + self.operation_id.len()
            + self.requirements_digest.len()
            + self
                .policy_id
                .as_ref()
                .map_or(0, |policy_id| LENGTH_PREFIX_BYTES + policy_id.len())
            + self.policy_posture.len()
            + self.obstruction_kind.len()
    }
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

impl AuthorityPolicyEvaluation {
    fn receipt_label(self) -> &'static str {
        match self {
            Self::InvalidDelegation => "authority-policy.invalid-delegation",
            Self::ScopeEscalation => "authority-policy.scope-escalation",
            Self::Unsupported => "authority-policy.unsupported",
        }
    }
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

impl CapabilityGrantIntentObstruction {
    fn receipt_label(self) -> &'static str {
        match self {
            Self::MissingIssuerAuthority => "capability-grant-intent.missing-issuer-authority",
            Self::MalformedGrantIntent => "capability-grant-intent.malformed-grant-intent",
            Self::InvalidDelegation => "capability-grant-intent.invalid-delegation",
            Self::ScopeEscalation => "capability-grant-intent.scope-escalation",
            Self::ReplayOrDuplicateIntent => "capability-grant-intent.replay-or-duplicate-intent",
            Self::UnsupportedAuthorityPolicy => {
                "capability-grant-intent.unsupported-authority-policy"
            }
        }
    }
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
    /// Causal refusal receipt. This is not an admission ticket, law witness, or
    /// counterfactual candidate.
    pub receipt: ObstructionReceipt,
}

/// Submission outcome for a capability grant intent skeleton.
#[must_use = "capability grant intent outcomes carry obstructions that must be handled"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityGrantIntentOutcome {
    /// Echo obstructed the grant intent before admitting authority.
    Obstructed(CapabilityGrantIntentPosture),
}

/// Caller-supplied expiry posture for narrow grant validation.
///
/// Echo does not parse [`CapabilityGrantIntent::expiry_bytes`] in this slice.
/// The posture is explicit validation input so tests and future adapters can
/// prove that an already-known expired grant obstructs without adding clock
/// policy, admission tickets, witnesses, or execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityGrantExpiryPosture {
    /// Expiry was not evaluated for this validation attempt.
    NotEvaluated,
    /// Expiry was evaluated and did not obstruct this validation attempt.
    Current,
    /// Expiry was evaluated and obstructed this validation attempt.
    Expired,
}

/// Obstruction reason for capability grant validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityGrantValidationObstruction {
    /// Presentation supplied unusable shape for grant validation.
    MalformedCapabilityPresentation,
    /// Presentation did not bind to a grant id.
    UnboundCapabilityPresentation,
    /// Presentation named grant material Echo has not recorded.
    UnknownGrant,
    /// Grant artifact hash did not cover the registered artifact.
    ArtifactHashMismatch,
    /// Grant operation id did not cover the registered artifact operation.
    OperationIdMismatch,
    /// Grant requirements digest did not cover the registered artifact
    /// requirements.
    RequirementsDigestMismatch,
    /// Grant expiry posture obstructed validation.
    ExpiredGrant,
}

/// Identity coverage returned when narrow grant validation finds no identity
/// mismatch.
///
/// This is not an authority grant, not an admission ticket, not a witness, and
/// not permission to execute. It only says the recorded grant material names the
/// same artifact hash, operation id, and requirements digest as the registered
/// artifact for this validation attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityGrantIdentityCoverage {
    /// Stable discriminator for callers and wire adapters.
    pub kind: String,
    /// Presentation identity supplied by the caller.
    pub presentation_id: String,
    /// Grant id named by the presentation.
    pub grant_id: String,
    /// Echo-owned runtime-local artifact handle id being covered.
    pub artifact_handle_id: String,
    /// Registered artifact hash covered by the grant material.
    pub artifact_hash: String,
    /// Registered operation id covered by the grant material.
    pub operation_id: String,
    /// Registered requirements digest covered by the grant material.
    pub requirements_digest: String,
}

/// Obstructed posture for capability grant validation.
///
/// Grant validation obstruction is graph evidence, not authority. This posture
/// reports why recorded grant material did not cover a registered artifact
/// identity; it never admits invocation or issues a success ticket.
#[must_use = "capability grant validation postures explain obstructions that must be handled"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityGrantValidationPosture {
    /// Stable discriminator for callers and wire adapters.
    pub kind: String,
    /// Presentation identity supplied by the caller.
    pub presentation_id: String,
    /// Grant id named by the presentation, when structurally available.
    pub grant_id: Option<String>,
    /// Echo-owned runtime-local artifact handle id being covered.
    pub artifact_handle_id: String,
    /// Registered artifact hash Echo expected the grant to cover.
    pub expected_artifact_hash: String,
    /// Artifact hash named by the grant material, when available.
    pub grant_artifact_hash: Option<String>,
    /// Registered operation id Echo expected the grant to cover.
    pub expected_operation_id: String,
    /// Operation id named by the grant material, when available.
    pub grant_operation_id: Option<String>,
    /// Registered requirements digest Echo expected the grant to cover.
    pub expected_requirements_digest: String,
    /// Requirements digest named by the grant material, when available.
    pub grant_requirements_digest: Option<String>,
    /// Structured reason Echo obstructed before treating grant material as
    /// authority.
    pub obstruction: CapabilityGrantValidationObstruction,
}

/// Outcome for narrow capability grant validation.
#[must_use = "capability grant validation outcomes carry obstructions that must be handled"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityGrantValidationOutcome {
    /// Recorded grant material covered the registered artifact identity only.
    ///
    /// This is not successful invocation admission.
    IdentityCovered(CapabilityGrantIdentityCoverage),
    /// Echo obstructed grant validation before authority could be considered.
    Obstructed(CapabilityGrantValidationPosture),
}

/// Narrow validator surface for capability presentations used during optic
/// invocation refusal.
///
/// Validation evidence refines refusal; it does not create authority. A
/// validator may publish graph facts or other causal evidence, but
/// [`CapabilityGrantValidationOutcome::IdentityCovered`] still is not
/// invocation admission.
pub trait CapabilityPresentationValidator {
    /// Validates a capability presentation against a registered artifact and
    /// invocation context.
    fn validate_capability_presentation(
        &mut self,
        registered: &RegisteredOpticArtifact,
        invocation: &OpticInvocation,
        presentation: &OpticCapabilityPresentation,
    ) -> CapabilityGrantValidationOutcome;
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
    /// A placeholder presentation was supplied, but grant validation is not
    /// wired into invocation admission in this slice.
    CapabilityValidationUnavailable,
}

/// Ticket-shaped pre-admission posture for an obstructed optic invocation.
///
/// This is not a successful admission ticket and does not authorize runtime
/// execution. It carries enough invocation context for callers and later
/// witness code to explain why Echo obstructed before grant validation was
/// wired into invocation admission.
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
/// obstructed deterministically. It can validate narrow identity coverage
/// against registered artifact material, but it does not admit grants into
/// witnessed history, issue admission tickets, emit law witnesses, or execute
/// runtime work.
#[derive(Clone, Debug, Default)]
pub struct CapabilityGrantIntentGate {
    intents_by_id: BTreeMap<String, CapabilityGrantIntent>,
    published_graph_facts: Vec<PublishedGraphFact>,
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

        Self::obstructed_grant_intent(&intent, &authority_context, obstruction)
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

    /// Validates recorded grant material against a registered artifact identity.
    ///
    /// This is a refusal-first substrate check. It only compares the bound grant
    /// material's artifact hash, operation id, requirements digest, and explicit
    /// expiry posture against Echo's registered artifact material. It does not
    /// admit authority, issue an admission ticket, emit a law witness, validate
    /// delegation, check quorum, or execute runtime work.
    #[must_use = "capability grant validation outcomes carry obstructions that must be handled"]
    pub fn validate_capability_presentation_for_artifact(
        &mut self,
        presentation: &OpticCapabilityPresentation,
        registered: &RegisteredOpticArtifact,
        expiry_posture: CapabilityGrantExpiryPosture,
    ) -> CapabilityGrantValidationOutcome {
        if presentation.presentation_id.is_empty()
            || presentation
                .bound_grant_id
                .as_ref()
                .is_some_and(String::is_empty)
        {
            return self.obstructed_capability_grant_validation(
                presentation,
                registered,
                None,
                CapabilityGrantValidationObstruction::MalformedCapabilityPresentation,
            );
        }

        let Some(grant_id) = presentation.bound_grant_id.as_deref() else {
            return self.obstructed_capability_grant_validation(
                presentation,
                registered,
                None,
                CapabilityGrantValidationObstruction::UnboundCapabilityPresentation,
            );
        };

        let Some(grant) = self.intents_by_id.get(grant_id).cloned() else {
            return self.obstructed_capability_grant_validation(
                presentation,
                registered,
                None,
                CapabilityGrantValidationObstruction::UnknownGrant,
            );
        };

        if grant.artifact_hash != registered.artifact_hash {
            return self.obstructed_capability_grant_validation(
                presentation,
                registered,
                Some(&grant),
                CapabilityGrantValidationObstruction::ArtifactHashMismatch,
            );
        }
        if grant.operation_id != registered.operation_id {
            return self.obstructed_capability_grant_validation(
                presentation,
                registered,
                Some(&grant),
                CapabilityGrantValidationObstruction::OperationIdMismatch,
            );
        }
        if grant.requirements_digest != registered.requirements_digest {
            return self.obstructed_capability_grant_validation(
                presentation,
                registered,
                Some(&grant),
                CapabilityGrantValidationObstruction::RequirementsDigestMismatch,
            );
        }
        if grant.expiry_bytes.is_some() && expiry_posture == CapabilityGrantExpiryPosture::Expired {
            return self.obstructed_capability_grant_validation(
                presentation,
                registered,
                Some(&grant),
                CapabilityGrantValidationObstruction::ExpiredGrant,
            );
        }

        CapabilityGrantValidationOutcome::IdentityCovered(CapabilityGrantIdentityCoverage {
            kind: "capability-grant-identity-coverage".to_owned(),
            presentation_id: presentation.presentation_id.clone(),
            grant_id: grant.intent_id,
            artifact_handle_id: registered.handle.id.clone(),
            artifact_hash: registered.artifact_hash.clone(),
            operation_id: registered.operation_id.clone(),
            requirements_digest: registered.requirements_digest.clone(),
        })
    }

    /// Returns in-memory graph facts published by this gate instance.
    #[must_use]
    pub fn published_graph_facts(&self) -> &[PublishedGraphFact] {
        &self.published_graph_facts
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

        let Some(policy) = &authority_context.policy else {
            return CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy;
        };
        if policy.policy_id.is_empty() {
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
        authority_context: &AuthorityContext,
        obstruction: CapabilityGrantIntentObstruction,
    ) -> CapabilityGrantIntentOutcome {
        CapabilityGrantIntentOutcome::Obstructed(CapabilityGrantIntentPosture {
            kind: "capability-grant-intent-posture".to_owned(),
            intent_id: intent.intent_id.clone(),
            proposed_by: intent.proposed_by.clone(),
            subject: intent.subject.clone(),
            obstruction,
            receipt: ObstructionReceipt::for_capability_grant_intent(
                intent,
                authority_context,
                obstruction,
            ),
        })
    }

    fn obstructed_capability_grant_validation(
        &mut self,
        presentation: &OpticCapabilityPresentation,
        registered: &RegisteredOpticArtifact,
        grant: Option<&CapabilityGrantIntent>,
        obstruction: CapabilityGrantValidationObstruction,
    ) -> CapabilityGrantValidationOutcome {
        self.publish_capability_grant_validation_obstruction(
            presentation,
            registered,
            grant,
            obstruction,
        );

        CapabilityGrantValidationOutcome::Obstructed(CapabilityGrantValidationPosture {
            kind: CAPABILITY_GRANT_VALIDATION_POSTURE_KIND.to_owned(),
            presentation_id: presentation.presentation_id.clone(),
            grant_id: Self::validation_grant_id(presentation, grant),
            artifact_handle_id: registered.handle.id.clone(),
            expected_artifact_hash: registered.artifact_hash.clone(),
            grant_artifact_hash: grant.map(|grant| grant.artifact_hash.clone()),
            expected_operation_id: registered.operation_id.clone(),
            grant_operation_id: grant.map(|grant| grant.operation_id.clone()),
            expected_requirements_digest: registered.requirements_digest.clone(),
            grant_requirements_digest: grant.map(|grant| grant.requirements_digest.clone()),
            obstruction,
        })
    }

    fn publish_capability_grant_validation_obstruction(
        &mut self,
        presentation: &OpticCapabilityPresentation,
        registered: &RegisteredOpticArtifact,
        grant: Option<&CapabilityGrantIntent>,
        obstruction: CapabilityGrantValidationObstruction,
    ) {
        self.published_graph_facts.push(PublishedGraphFact::new(
            GraphFact::CapabilityGrantValidationObstructed {
                presentation_id: presentation.presentation_id.clone(),
                grant_id: Self::validation_grant_id(presentation, grant),
                artifact_handle_id: registered.handle.id.clone(),
                expected_artifact_hash: registered.artifact_hash.clone(),
                grant_artifact_hash: grant.map(|grant| grant.artifact_hash.clone()),
                expected_operation_id: registered.operation_id.clone(),
                grant_operation_id: grant.map(|grant| grant.operation_id.clone()),
                expected_requirements_digest: registered.requirements_digest.clone(),
                grant_requirements_digest: grant.map(|grant| grant.requirements_digest.clone()),
                obstruction: capability_grant_validation_obstruction_kind(obstruction),
            },
        ));
    }

    fn validation_grant_id(
        presentation: &OpticCapabilityPresentation,
        grant: Option<&CapabilityGrantIntent>,
    ) -> Option<String> {
        grant.map(|grant| grant.intent_id.clone()).or_else(|| {
            presentation
                .bound_grant_id
                .as_ref()
                .filter(|grant_id| !grant_id.is_empty())
                .cloned()
        })
    }
}

impl CapabilityPresentationValidator for CapabilityGrantIntentGate {
    fn validate_capability_presentation(
        &mut self,
        registered: &RegisteredOpticArtifact,
        _invocation: &OpticInvocation,
        presentation: &OpticCapabilityPresentation,
    ) -> CapabilityGrantValidationOutcome {
        self.validate_capability_presentation_for_artifact(
            presentation,
            registered,
            CapabilityGrantExpiryPosture::NotEvaluated,
        )
    }
}

fn push_receipt_field(bytes: &mut Vec<u8>, field: &[u8]) {
    bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
    bytes.extend_from_slice(field);
}

fn push_optional_receipt_field(bytes: &mut Vec<u8>, field: Option<&[u8]>) {
    match field {
        Some(field) => {
            bytes.push(1);
            push_receipt_field(bytes, field);
        }
        None => bytes.push(0),
    }
}

/// Echo-owned runtime-local registry for Wesley-compiled optic artifacts.
#[derive(Clone, Debug, Default)]
pub struct OpticArtifactRegistry {
    next_handle_index: u64,
    artifacts_by_handle: BTreeMap<String, RegisteredOpticArtifact>,
    published_graph_facts: Vec<PublishedGraphFact>,
    artifact_registration_receipts: Vec<ArtifactRegistrationReceipt>,
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
        if let Err(error) = Self::verify_descriptor(&artifact, &descriptor) {
            self.publish_artifact_registration_obstruction(&descriptor, &error);
            return Err(error);
        }

        let handle = self.next_handle();
        let artifact_hash = artifact.artifact_hash;
        let schema_id = artifact.schema_id;
        let operation_id = artifact.operation.operation_id;
        let requirements_digest = artifact.requirements_digest;
        let registered = RegisteredOpticArtifact {
            handle: handle.clone(),
            artifact_id: artifact.artifact_id,
            artifact_hash: artifact_hash.clone(),
            schema_id: schema_id.clone(),
            operation_id: operation_id.clone(),
            requirements_digest: requirements_digest.clone(),
            requirements: artifact.requirements,
        };
        self.artifacts_by_handle
            .insert(handle.id.clone(), registered);
        self.publish_artifact_registered_fact(
            &handle,
            artifact_hash,
            schema_id,
            operation_id,
            requirements_digest,
        );

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
        &mut self,
        invocation: &OpticInvocation,
    ) -> OpticInvocationAdmissionOutcome {
        let Ok(registered) = self.resolve_optic_artifact_handle(&invocation.artifact_handle) else {
            return self
                .obstructed_invocation(invocation, OpticInvocationObstruction::UnknownHandle);
        };

        if invocation.operation_id != registered.operation_id {
            return self
                .obstructed_invocation(invocation, OpticInvocationObstruction::OperationMismatch);
        }

        self.obstructed_invocation(
            invocation,
            Self::classify_capability_presentation(invocation.capability_presentation.as_ref()),
        )
    }

    /// Admits or obstructs an invocation while asking a capability presentation
    /// validator for refusal evidence.
    ///
    /// This remains an obstructed-only path. Validator evidence can publish a
    /// sharper graph fact, but identity coverage is not invocation admission and
    /// does not issue a success ticket, law witness, scheduler work, or
    /// execution.
    #[must_use = "optic invocation admission outcomes carry obstructions that must be handled"]
    pub fn admit_optic_invocation_with_capability_validator(
        &mut self,
        invocation: &OpticInvocation,
        validator: &mut impl CapabilityPresentationValidator,
    ) -> OpticInvocationAdmissionOutcome {
        let registered = match self.resolve_optic_artifact_handle(&invocation.artifact_handle) {
            Ok(registered) => registered.clone(),
            Err(_) => {
                return self
                    .obstructed_invocation(invocation, OpticInvocationObstruction::UnknownHandle);
            }
        };

        if invocation.operation_id != registered.operation_id {
            return self
                .obstructed_invocation(invocation, OpticInvocationObstruction::OperationMismatch);
        }

        let obstruction =
            Self::classify_capability_presentation(invocation.capability_presentation.as_ref());
        if obstruction != OpticInvocationObstruction::CapabilityValidationUnavailable {
            return self.obstructed_invocation(invocation, obstruction);
        }

        if let Some(presentation) = invocation.capability_presentation.as_ref() {
            let _ =
                validator.validate_capability_presentation(&registered, invocation, presentation);
        }

        self.obstructed_invocation(
            invocation,
            OpticInvocationObstruction::CapabilityValidationUnavailable,
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
        &mut self,
        invocation: &OpticInvocation,
        obstruction: OpticInvocationObstruction,
    ) -> OpticInvocationAdmissionOutcome {
        self.publish_invocation_obstruction_fact(invocation, obstruction);
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

    /// Returns in-memory graph facts published by this registry instance.
    #[must_use]
    pub fn published_graph_facts(&self) -> &[PublishedGraphFact] {
        &self.published_graph_facts
    }

    /// Returns in-memory artifact registration receipts emitted by this
    /// registry instance.
    #[must_use]
    pub fn artifact_registration_receipts(&self) -> &[ArtifactRegistrationReceipt] {
        &self.artifact_registration_receipts
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

    fn publish_artifact_registered_fact(
        &mut self,
        handle: &OpticArtifactHandle,
        artifact_hash: String,
        schema_id: String,
        operation_id: String,
        requirements_digest: String,
    ) {
        let published = PublishedGraphFact::new(GraphFact::ArtifactRegistered {
            handle_id: handle.id.clone(),
            artifact_hash: artifact_hash.clone(),
            schema_id,
            operation_id: operation_id.clone(),
            requirements_digest,
        });
        self.artifact_registration_receipts
            .push(ArtifactRegistrationReceipt {
                kind: ARTIFACT_REGISTRATION_RECEIPT_KIND.to_owned(),
                handle_id: handle.id.clone(),
                artifact_hash,
                operation_id,
                fact_digest: published.digest,
            });
        self.published_graph_facts.push(published);
    }

    fn publish_artifact_registration_obstruction(
        &mut self,
        descriptor: &OpticRegistrationDescriptor,
        error: &OpticArtifactRegistrationError,
    ) {
        if let Some(obstruction) = artifact_registration_obstruction_kind(error) {
            self.published_graph_facts.push(PublishedGraphFact::new(
                GraphFact::ArtifactRegistrationObstructed {
                    artifact_hash: Some(descriptor.artifact_hash.clone()),
                    obstruction,
                },
            ));
        }
    }

    fn publish_invocation_obstruction_fact(
        &mut self,
        invocation: &OpticInvocation,
        obstruction: OpticInvocationObstruction,
    ) {
        self.published_graph_facts.push(PublishedGraphFact::new(
            GraphFact::OpticInvocationObstructed {
                artifact_handle_id: invocation.artifact_handle.id.clone(),
                operation_id: invocation.operation_id.clone(),
                canonical_variables_digest: invocation.canonical_variables_digest.clone(),
                basis_request_digest: digest_invocation_request_bytes(
                    b"echo.optic-invocation.basis-request.v0",
                    &invocation.basis_request.bytes,
                ),
                aperture_request_digest: digest_invocation_request_bytes(
                    b"echo.optic-invocation.aperture-request.v0",
                    &invocation.aperture_request.bytes,
                ),
                obstruction: invocation_obstruction_kind(obstruction),
            },
        ));
    }
}

fn artifact_registration_obstruction_kind(
    error: &OpticArtifactRegistrationError,
) -> Option<ArtifactRegistrationObstructionKind> {
    match error {
        OpticArtifactRegistrationError::ArtifactIdMismatch => {
            Some(ArtifactRegistrationObstructionKind::ArtifactIdMismatch)
        }
        OpticArtifactRegistrationError::ArtifactHashMismatch => {
            Some(ArtifactRegistrationObstructionKind::ArtifactHashMismatch)
        }
        OpticArtifactRegistrationError::SchemaIdMismatch => {
            Some(ArtifactRegistrationObstructionKind::SchemaIdMismatch)
        }
        OpticArtifactRegistrationError::OperationIdMismatch => {
            Some(ArtifactRegistrationObstructionKind::OperationIdMismatch)
        }
        OpticArtifactRegistrationError::RequirementsDigestMismatch => {
            Some(ArtifactRegistrationObstructionKind::RequirementsDigestMismatch)
        }
        OpticArtifactRegistrationError::UnknownHandle => None,
    }
}

fn invocation_obstruction_kind(
    obstruction: OpticInvocationObstruction,
) -> InvocationObstructionKind {
    match obstruction {
        OpticInvocationObstruction::UnknownHandle => InvocationObstructionKind::UnknownHandle,
        OpticInvocationObstruction::OperationMismatch => {
            InvocationObstructionKind::OperationMismatch
        }
        OpticInvocationObstruction::MissingCapability => {
            InvocationObstructionKind::MissingCapability
        }
        OpticInvocationObstruction::MalformedCapabilityPresentation => {
            InvocationObstructionKind::MalformedCapabilityPresentation
        }
        OpticInvocationObstruction::UnboundCapabilityPresentation => {
            InvocationObstructionKind::UnboundCapabilityPresentation
        }
        OpticInvocationObstruction::CapabilityValidationUnavailable => {
            InvocationObstructionKind::CapabilityValidationUnavailable
        }
    }
}

fn capability_grant_validation_obstruction_kind(
    obstruction: CapabilityGrantValidationObstruction,
) -> CapabilityGrantValidationObstructionKind {
    match obstruction {
        CapabilityGrantValidationObstruction::MalformedCapabilityPresentation => {
            CapabilityGrantValidationObstructionKind::MalformedCapabilityPresentation
        }
        CapabilityGrantValidationObstruction::UnboundCapabilityPresentation => {
            CapabilityGrantValidationObstructionKind::UnboundCapabilityPresentation
        }
        CapabilityGrantValidationObstruction::UnknownGrant => {
            CapabilityGrantValidationObstructionKind::UnknownGrant
        }
        CapabilityGrantValidationObstruction::ArtifactHashMismatch => {
            CapabilityGrantValidationObstructionKind::ArtifactHashMismatch
        }
        CapabilityGrantValidationObstruction::OperationIdMismatch => {
            CapabilityGrantValidationObstructionKind::OperationIdMismatch
        }
        CapabilityGrantValidationObstruction::RequirementsDigestMismatch => {
            CapabilityGrantValidationObstructionKind::RequirementsDigestMismatch
        }
        CapabilityGrantValidationObstruction::ExpiredGrant => {
            CapabilityGrantValidationObstructionKind::ExpiredGrant
        }
    }
}
