// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Contract-hosted obstruction taxonomy.
//!
//! This module names generic obstruction posture for contract-hosted
//! applications. It does not replace domain errors, does not invent
//! application-specific failure names, and does not turn runtime faults into
//! lawful domain rejections.

use crate::coordinator::RuntimeError;
use crate::ident::Hash;
use crate::observation::{ObservationError, ReadingResidualPosture};
use crate::{ContractEvidenceIdentity, SchedulerFaultId};

/// Generic contract-hosted obstruction kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractObstructionKind {
    /// No installed contract package supports the requested mutation operation.
    UnsupportedOperation,
    /// No installed contract query observer supports the requested query.
    UnsupportedQuery,
    /// Admission or request posture prevented contract work or reading.
    AdmissionObstruction,
    /// Runtime safety posture or an internal failure prevented progress.
    RuntimeFault,
    /// Required retained material was unavailable.
    MissingRetention,
    /// The requested basis is stale, invalid, or unavailable.
    StaleBasis,
    /// The observer emitted an explicit residual reading.
    ResidualReading,
    /// The requested read exceeded its declared budget.
    BudgetExceeded,
}

/// Generic subject named by a contract obstruction.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ContractObstructionSubject {
    /// No narrower subject is available yet.
    #[default]
    Unspecified,
    /// Generated mutation operation id.
    Operation {
        /// Generated mutation operation id.
        op_id: u32,
    },
    /// Generated query operation id.
    Query {
        /// Generated query operation id.
        query_id: u32,
    },
    /// Witnessed submission id.
    Submission {
        /// Witnessed submission id.
        submission_id: Hash,
    },
    /// Admission ticket digest.
    Ticket {
        /// Admission ticket digest.
        ticket_digest: Hash,
    },
    /// Query reading identity.
    Reading {
        /// Query reading identity.
        reading_id: Hash,
    },
    /// Retained material coordinate or digest.
    Retention {
        /// Retained material coordinate or digest.
        retention_id: Hash,
    },
    /// Scheduler fault evidence.
    SchedulerFault {
        /// Scheduler fault evidence id.
        fault_id: SchedulerFaultId,
    },
}

/// Product-facing contract obstruction posture.
///
/// The optional contract evidence names the installed package boundary when the
/// caller already has it. It is evidence metadata only; it does not authorize
/// execution and does not grant query rights.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractObstruction {
    /// Stable generic obstruction kind.
    pub kind: ContractObstructionKind,
    /// Generic subject associated with the obstruction.
    pub subject: ContractObstructionSubject,
    /// Installed package evidence, when known.
    pub contract: Option<ContractEvidenceIdentity>,
}

impl ContractObstruction {
    /// Builds an obstruction with no installed package evidence.
    #[must_use]
    pub fn new(kind: ContractObstructionKind, subject: ContractObstructionSubject) -> Self {
        Self {
            kind,
            subject,
            contract: None,
        }
    }

    /// Attaches installed package evidence to the obstruction.
    #[must_use]
    pub fn with_contract(mut self, contract: ContractEvidenceIdentity) -> Self {
        self.contract = Some(contract);
        self
    }

    /// No installed package supports the mutation operation id.
    #[must_use]
    pub fn unsupported_operation(op_id: u32) -> Self {
        Self::new(
            ContractObstructionKind::UnsupportedOperation,
            ContractObstructionSubject::Operation { op_id },
        )
    }

    /// No installed query observer supports the query id.
    #[must_use]
    pub fn unsupported_query(query_id: u32) -> Self {
        Self::new(
            ContractObstructionKind::UnsupportedQuery,
            ContractObstructionSubject::Query { query_id },
        )
    }

    /// Contract admission or request posture obstructed the work.
    #[must_use]
    pub fn admission_obstruction(subject: ContractObstructionSubject) -> Self {
        Self::new(ContractObstructionKind::AdmissionObstruction, subject)
    }

    /// Runtime safety posture obstructed progress.
    #[must_use]
    pub fn runtime_fault(subject: ContractObstructionSubject) -> Self {
        Self::new(ContractObstructionKind::RuntimeFault, subject)
    }

    /// Required retained material was unavailable.
    #[must_use]
    pub fn missing_retention(retention_id: Hash) -> Self {
        Self::new(
            ContractObstructionKind::MissingRetention,
            ContractObstructionSubject::Retention { retention_id },
        )
    }

    /// The requested causal basis is stale, invalid, or unavailable.
    #[must_use]
    pub fn stale_basis() -> Self {
        Self::new(
            ContractObstructionKind::StaleBasis,
            ContractObstructionSubject::Unspecified,
        )
    }

    /// The reading completed only as an explicit residual.
    #[must_use]
    pub fn residual_reading(reading_id: Hash) -> Self {
        Self::new(
            ContractObstructionKind::ResidualReading,
            ContractObstructionSubject::Reading { reading_id },
        )
    }

    /// The read exceeded its declared budget.
    #[must_use]
    pub fn budget_exceeded() -> Self {
        Self::new(
            ContractObstructionKind::BudgetExceeded,
            ContractObstructionSubject::Unspecified,
        )
    }

    /// Classifies an observation error without importing application-domain
    /// failure names into core.
    #[must_use]
    pub fn from_observation_error(error: &ObservationError) -> Self {
        match error {
            ObservationError::UnsupportedQuery { query_id } => Self::unsupported_query(*query_id),
            ObservationError::BudgetExceeded { .. } => Self::budget_exceeded(),
            ObservationError::InvalidWorldline(_)
            | ObservationError::InvalidTick { .. }
            | ObservationError::ObservationUnavailable { .. } => Self::stale_basis(),
            ObservationError::UnsupportedFrameProjection { .. }
            | ObservationError::UnsupportedObserverPlan(_)
            | ObservationError::UnsupportedObserverInstance(_)
            | ObservationError::UnsupportedRights(_)
            | ObservationError::ContractQueryObserverFailed { .. } => {
                Self::admission_obstruction(ContractObstructionSubject::Unspecified)
            }
            ObservationError::CodecFailure(_) => {
                Self::runtime_fault(ContractObstructionSubject::Unspecified)
            }
        }
    }

    /// Classifies a runtime error for contract-hosted product surfaces.
    #[must_use]
    pub fn from_runtime_error(error: &RuntimeError) -> Self {
        match error {
            RuntimeError::UnsupportedInstalledContractMutation { op_id } => {
                Self::unsupported_operation(*op_id)
            }
            RuntimeError::SchedulerRuntimeFaultActive(fault_id)
            | RuntimeError::UnknownSchedulerFault(fault_id)
            | RuntimeError::SchedulerFaultAlreadyResolved(fault_id) => {
                Self::runtime_fault(ContractObstructionSubject::SchedulerFault {
                    fault_id: *fault_id,
                })
            }
            RuntimeError::MalformedInstalledContractIntent
            | RuntimeError::UnknownIntentSubmission(_)
            | RuntimeError::TicketedIngressSubmissionMismatch(_)
            | RuntimeError::TicketedIngressAlreadyStaged(_)
            | RuntimeError::TicketedIngressDuplicateRuntimeIngress { .. } => {
                Self::admission_obstruction(ContractObstructionSubject::Unspecified)
            }
            RuntimeError::SchedulerFaultGenerationOverflow
            | RuntimeError::DuplicateWorldline(_)
            | RuntimeError::DuplicateHead(_)
            | RuntimeError::UnknownWorldline(_)
            | RuntimeError::UnknownHead(_)
            | RuntimeError::DuplicateDefaultWriter(_)
            | RuntimeError::DuplicateInboxAddress { .. }
            | RuntimeError::MissingDefaultWriter(_)
            | RuntimeError::MissingInboxAddress { .. }
            | RuntimeError::RejectedByPolicy(_)
            | RuntimeError::Engine(_)
            | RuntimeError::Provenance(_)
            | RuntimeError::Replay(_)
            | RuntimeError::Strand(_)
            | RuntimeError::FrontierTickOverflow(_)
            | RuntimeError::GlobalTickOverflow
            | RuntimeError::IntentSubmissionGenerationOverflow
            | RuntimeError::IntentSubmissionReplayMismatch(_) => {
                Self::runtime_fault(ContractObstructionSubject::Unspecified)
            }
        }
    }

    /// Classifies reading residual posture when a product surface needs a
    /// contract obstruction only for non-complete readings.
    #[must_use]
    pub fn from_residual_posture(
        posture: ReadingResidualPosture,
        reading_id: Option<Hash>,
    ) -> Option<Self> {
        match posture {
            ReadingResidualPosture::Complete | ReadingResidualPosture::PluralityPreserved => None,
            ReadingResidualPosture::Residual => {
                Some(Self::residual_reading(reading_id.unwrap_or([0; 32])))
            }
            ReadingResidualPosture::Obstructed => Some(Self::admission_obstruction(
                ContractObstructionSubject::Reading {
                    reading_id: reading_id.unwrap_or([0; 32]),
                },
            )),
        }
    }
}
