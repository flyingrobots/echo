// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Narrow Edict `echo.span-ir/v1` fixture acceptance and attempt receipts.
//!
//! This module is a fixture bridge for the Edict obstruction-strand corridor. It
//! accepts the small Target IR subset Edict currently emits for Echo pre-step
//! `continueObstructed` requirements and produces deterministic attempt receipts
//! bound to the supplied Target IR artifact digest. It is not general Edict
//! bundle admission, target plugin dispatch, or scheduler counterfactual
//! retention.

use std::collections::BTreeMap;

use crate::ident::Hash;
use crate::{ContractObstruction, ContractObstructionKind};

/// Supported Edict Target IR artifact domain for this bridge.
pub const EDICT_ECHO_TARGET_IR_DOMAIN: &str = "echo.span-ir/v1";

/// Version marker for the non-canonical Echo attempt receipt shape.
///
/// This string versions the Rust review/fixture shape only. It does not freeze
/// canonical Echo receipt bytes and must not be treated as a receipt digest
/// domain.
pub const EDICT_ECHO_ATTEMPT_RECEIPT_SCHEMA: &str = "echo.edict-target-ir.attempt-receipt/v0";

/// Test-owned representation of the Edict Target IR artifact envelope Echo
/// accepts in this slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdictEchoTargetIrArtifact {
    /// Target IR domain. The only supported value is
    /// [`EDICT_ECHO_TARGET_IR_DOMAIN`].
    pub domain: String,
    /// Target profile coordinate selected by Edict.
    pub target_profile_coordinate: String,
    /// Lowercase strict `sha256:<64-lower-hex>` target profile digest.
    pub target_profile_digest: String,
    /// Lowercase strict `sha256:<64-lower-hex>` Target IR artifact digest.
    pub target_ir_digest: String,
    /// Coordinate or review handle for the source Core artifact.
    pub source_core_coordinate: String,
    /// Source-ordered Target IR intents represented by the fixture.
    pub intents: Vec<EdictEchoTargetIrIntent>,
}

/// Test-owned representation of an Edict Target IR intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdictEchoTargetIrIntent {
    /// Intent name emitted by Edict.
    pub name: String,
    /// Pre-step requirements emitted for this intent.
    pub requirements: Vec<EdictEchoTargetIrRequirement>,
}

/// A pre-step Target IR requirement accepted by the Echo fixture bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdictEchoTargetIrRequirement {
    /// Requirement identifier emitted by Edict.
    pub id: String,
    /// Predicate Echo can evaluate for this fixture bridge.
    pub predicate: EdictEchoTargetIrPredicate,
    /// Failure disposition emitted by Edict.
    pub on_failure: EdictEchoTargetIrRequirementFailure,
}

/// Requirement predicates supported by this fixture bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdictEchoTargetIrPredicate {
    /// Checks whether the supplied input basis is fresh at execution time.
    BasisFresh,
}

/// Requirement failure dispositions represented in Edict Target IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdictEchoTargetIrRequirementFailure {
    /// Terminal hard rejection disposition.
    Terminal {
        /// Opaque Edict obstruction reason attached to the disposition.
        reason: EdictEchoObstructionReason,
    },
    /// Continue into an obstructed attempt rather than hard rejection.
    ContinueObstructed {
        /// Opaque Edict obstruction reason attached to the disposition.
        reason: EdictEchoObstructionReason,
    },
}

/// Opaque Edict obstruction reason carried through Echo receipt evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdictEchoObstructionReason {
    /// Stable reason kind emitted by Edict.
    pub kind: String,
    /// Opaque deterministic payload fields emitted by Edict for review.
    pub payload: BTreeMap<String, String>,
}

/// Stable digest field names used by acceptance errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdictEchoTargetIrDigestField {
    /// `target_profile_digest` failed lowercase strict `sha256:` validation.
    TargetProfileDigest,
    /// `target_ir_digest` failed lowercase strict `sha256:` validation.
    TargetIrDigest,
}

/// Stable acceptance error kind for the narrow Edict Echo fixture bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdictEchoTargetIrAcceptanceErrorKind {
    /// The artifact domain was not `echo.span-ir/v1`.
    WrongDomain,
    /// A required digest field was absent, malformed, or not lowercase strict.
    MalformedDigest,
    /// The fixture carried no accepted pre-step requirements.
    MissingRequirement,
    /// The fixture carried more than one pre-step requirement, which this
    /// bridge does not execute yet.
    UnsupportedRequirementCount,
    /// The fixture carried a requirement disposition not supported by this
    /// bridge.
    UnsupportedRequirementDisposition,
    /// The fixture carried an unsupported requirement predicate.
    UnsupportedRequirementPredicate,
}

/// Structured acceptance failure for an Edict Echo Target IR fixture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdictEchoTargetIrAcceptanceError {
    /// Stable failure kind.
    pub kind: EdictEchoTargetIrAcceptanceErrorKind,
    /// Digest field associated with [`EdictEchoTargetIrAcceptanceErrorKind::MalformedDigest`].
    pub digest_field: Option<EdictEchoTargetIrDigestField>,
}

impl EdictEchoTargetIrAcceptanceError {
    /// Returns the attempt outcome family associated with this acceptance
    /// failure.
    #[must_use]
    pub const fn outcome_kind(&self) -> EdictEchoAttemptOutcomeKind {
        EdictEchoAttemptOutcomeKind::InvalidProposal
    }

    const fn new(kind: EdictEchoTargetIrAcceptanceErrorKind) -> Self {
        Self {
            kind,
            digest_field: None,
        }
    }

    const fn malformed_digest(field: EdictEchoTargetIrDigestField) -> Self {
        Self {
            kind: EdictEchoTargetIrAcceptanceErrorKind::MalformedDigest,
            digest_field: Some(field),
        }
    }
}

/// Accepted fixture state. Acceptance is separate from execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedEdictEchoTargetIr {
    target_profile_digest: Hash,
    target_ir_digest: Hash,
    requirement: AcceptedEdictEchoRequirement,
}

impl AcceptedEdictEchoTargetIr {
    /// Returns the accepted target profile digest bytes.
    #[must_use]
    pub const fn target_profile_digest(&self) -> Hash {
        self.target_profile_digest
    }

    /// Returns the accepted Target IR digest bytes.
    #[must_use]
    pub const fn target_ir_digest(&self) -> Hash {
        self.target_ir_digest
    }

    /// Returns the number of pre-step requirements accepted by this bridge.
    #[must_use]
    pub const fn requirement_count(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AcceptedEdictEchoRequirement {
    predicate: EdictEchoTargetIrPredicate,
    on_failure: AcceptedEdictEchoRequirementFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AcceptedEdictEchoRequirementFailure {
    ContinueObstructed { reason: EdictEchoObstructionReason },
}

/// Deterministic input facts supplied by the Echo test harness for one attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdictEchoAttemptInput {
    /// Whether the supplied basis is fresh under the host's deterministic test
    /// facts.
    pub basis_is_fresh: bool,
    /// Digest of the basis supplied by the caller.
    pub input_basis_digest: Hash,
    /// Digest of the basis observed by Echo during the attempt.
    pub observed_basis_digest: Hash,
}

/// Attempt outcome vocabulary used by the Edict Echo fixture bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdictEchoAttemptOutcomeKind {
    /// Echo accepted and executed the fixture path successfully.
    CommittedSuccess,
    /// Echo accepted the fixture but the attempt continued into an obstruction.
    ObstructedAttempt,
    /// Echo rejected the artifact before execution as invalid for this bridge.
    InvalidProposal,
    /// Reserved for admitted legal candidates not selected by a scheduler.
    LegalUnselectedCounterfactual,
    /// Reserved for runtime faults rather than lawful domain obstructions.
    RuntimeFault,
}

/// Obstruction evidence attached to an obstructed Edict Echo attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdictEchoAttemptObstruction {
    /// Generic Echo contract obstruction posture.
    pub contract: ContractObstruction,
    /// Opaque Edict reason carried through for review.
    pub reason: EdictEchoObstructionReason,
}

/// Deterministic attempt receipt for the narrow Edict Echo Target IR bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdictEchoAttemptReceipt {
    /// Version marker for this Rust fixture receipt shape.
    pub receipt_schema: &'static str,
    /// Target IR digest supplied by and checked during fixture acceptance.
    pub target_ir_digest: Hash,
    /// Attempt outcome family.
    pub outcome_kind: EdictEchoAttemptOutcomeKind,
    /// Caller-supplied basis digest used for basis freshness evaluation.
    pub input_basis_digest: Hash,
    /// Echo-observed basis digest used for basis freshness evaluation.
    pub observed_basis_digest: Hash,
    /// Obstruction evidence for [`EdictEchoAttemptOutcomeKind::ObstructedAttempt`].
    pub obstruction: Option<EdictEchoAttemptObstruction>,
}

/// Accepts the supported Edict `echo.span-ir/v1` Target IR fixture subset.
///
/// Acceptance validates artifact identity and shape only. It does not execute
/// the requirement, tick the scheduler, admit bundles, or produce an attempt
/// receipt.
pub fn accept_edict_echo_target_ir(
    artifact: &EdictEchoTargetIrArtifact,
) -> Result<AcceptedEdictEchoTargetIr, EdictEchoTargetIrAcceptanceError> {
    if artifact.domain != EDICT_ECHO_TARGET_IR_DOMAIN {
        return Err(EdictEchoTargetIrAcceptanceError::new(
            EdictEchoTargetIrAcceptanceErrorKind::WrongDomain,
        ));
    }

    let target_profile_digest = parse_sha256_review_string(
        &artifact.target_profile_digest,
        EdictEchoTargetIrDigestField::TargetProfileDigest,
    )?;
    let target_ir_digest = parse_sha256_review_string(
        &artifact.target_ir_digest,
        EdictEchoTargetIrDigestField::TargetIrDigest,
    )?;

    let mut requirements = artifact
        .intents
        .iter()
        .flat_map(|intent| intent.requirements.iter());
    let Some(requirement) = requirements.next() else {
        return Err(EdictEchoTargetIrAcceptanceError::new(
            EdictEchoTargetIrAcceptanceErrorKind::MissingRequirement,
        ));
    };
    if requirements.next().is_some() {
        return Err(EdictEchoTargetIrAcceptanceError::new(
            EdictEchoTargetIrAcceptanceErrorKind::UnsupportedRequirementCount,
        ));
    }

    let on_failure = match &requirement.on_failure {
        EdictEchoTargetIrRequirementFailure::ContinueObstructed { reason } => {
            AcceptedEdictEchoRequirementFailure::ContinueObstructed {
                reason: reason.clone(),
            }
        }
        EdictEchoTargetIrRequirementFailure::Terminal { .. } => {
            return Err(EdictEchoTargetIrAcceptanceError::new(
                EdictEchoTargetIrAcceptanceErrorKind::UnsupportedRequirementDisposition,
            ));
        }
    };

    match requirement.predicate {
        EdictEchoTargetIrPredicate::BasisFresh => {}
    }

    Ok(AcceptedEdictEchoTargetIr {
        target_profile_digest,
        target_ir_digest,
        requirement: AcceptedEdictEchoRequirement {
            predicate: requirement.predicate,
            on_failure,
        },
    })
}

/// Executes one accepted Edict Echo Target IR fixture attempt.
///
/// Execution here means evaluating the supported pre-step requirement against
/// deterministic input facts. It does not dispatch a general Echo scheduler
/// candidate set and never produces legal-unselected counterfactuals.
#[must_use]
pub fn execute_accepted_edict_echo_target_ir(
    accepted: &AcceptedEdictEchoTargetIr,
    input: EdictEchoAttemptInput,
) -> EdictEchoAttemptReceipt {
    let requirement_satisfied = match accepted.requirement.predicate {
        EdictEchoTargetIrPredicate::BasisFresh => input.basis_is_fresh,
    };

    if requirement_satisfied {
        return EdictEchoAttemptReceipt {
            receipt_schema: EDICT_ECHO_ATTEMPT_RECEIPT_SCHEMA,
            target_ir_digest: accepted.target_ir_digest,
            outcome_kind: EdictEchoAttemptOutcomeKind::CommittedSuccess,
            input_basis_digest: input.input_basis_digest,
            observed_basis_digest: input.observed_basis_digest,
            obstruction: None,
        };
    }

    let AcceptedEdictEchoRequirementFailure::ContinueObstructed { reason } =
        &accepted.requirement.on_failure;
    EdictEchoAttemptReceipt {
        receipt_schema: EDICT_ECHO_ATTEMPT_RECEIPT_SCHEMA,
        target_ir_digest: accepted.target_ir_digest,
        outcome_kind: EdictEchoAttemptOutcomeKind::ObstructedAttempt,
        input_basis_digest: input.input_basis_digest,
        observed_basis_digest: input.observed_basis_digest,
        obstruction: Some(EdictEchoAttemptObstruction {
            contract: ContractObstruction::new(
                ContractObstructionKind::StaleBasis,
                crate::ContractObstructionSubject::Unspecified,
            ),
            reason: reason.clone(),
        }),
    }
}

fn parse_sha256_review_string(
    value: &str,
    field: EdictEchoTargetIrDigestField,
) -> Result<Hash, EdictEchoTargetIrAcceptanceError> {
    let Some(hex_value) = value.strip_prefix("sha256:") else {
        return Err(EdictEchoTargetIrAcceptanceError::malformed_digest(field));
    };
    if hex_value.len() != 64 || !hex_value.bytes().all(is_lowercase_hex_digit) {
        return Err(EdictEchoTargetIrAcceptanceError::malformed_digest(field));
    }

    let mut digest = [0_u8; 32];
    hex::decode_to_slice(hex_value, &mut digest)
        .map_err(|_| EdictEchoTargetIrAcceptanceError::malformed_digest(field))?;
    Ok(digest)
}

fn is_lowercase_hex_digit(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}
