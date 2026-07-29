// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Durable, domain-neutral external-action request and settlement protocol.
//!
//! Edict-authored programs construct request values. Echo records each request
//! before an adapter may act, records one bounded claim, and admits one
//! schema-bound settlement before deterministic resumption. This module does
//! not execute external effects and grants no filesystem, process, network, or
//! model authority.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::causal_wal::{
    AffectedFrontier, RecoveryScanReport, WalBuildError, WalCommittedTransaction, WalDecodeError,
    WalStoreError, WalStorePort, WalTransactionBuilder,
};
use crate::{Hash, WorldlineId};

const REQUEST_ID_DOMAIN: &[u8] = b"echo:external-action:request-id:v1\0";

/// Stable identity of one external operation family.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalActionOperationIdV1(Hash);

impl ExternalActionOperationIdV1 {
    /// Reconstructs an operation identity from its canonical digest.
    #[must_use]
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical digest.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of one authorized external adapter.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalActionAdapterIdV1(Hash);

impl ExternalActionAdapterIdV1 {
    /// Reconstructs an adapter identity from its canonical digest.
    #[must_use]
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical digest.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identity of one canonical external-action request.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalActionRequestIdV1(Hash);

impl ExternalActionRequestIdV1 {
    /// Returns the canonical digest.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }

    const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }
}

/// Stable identity of one bounded adapter attempt.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalActionAttemptIdV1(Hash);

impl ExternalActionAttemptIdV1 {
    /// Reconstructs an attempt identity from its canonical digest.
    #[must_use]
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical digest.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Bounds delegated to one external-action request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionBudgetV1 {
    /// Maximum canonical settlement bytes retained in the WAL.
    pub max_settlement_bytes: u64,
    /// Maximum number of adapter attempts authorized for this request.
    pub max_attempts: u32,
}

/// Deterministic request emitted by an Edict-authored program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionRequestV1 {
    request_id: ExternalActionRequestIdV1,
    /// Worldline whose admitted history produced the request.
    pub worldline_id: WorldlineId,
    /// Declared external operation family.
    pub operation_id: ExternalActionOperationIdV1,
    /// Schema digest for the canonical request input.
    pub input_schema_digest: Hash,
    /// Schema digest required for the canonical settlement.
    pub settlement_schema_digest: Hash,
    /// Requested authority scope.
    pub authority_scope_digest: Hash,
    /// Exact current-world basis.
    pub basis_digest: Hash,
    /// Delegated execution and settlement bounds.
    pub budget: ExternalActionBudgetV1,
    /// Digest of the canonical operation input.
    pub input_digest: Hash,
    /// Named reconciliation law for ambiguous outcomes.
    pub reconciliation_law_digest: Hash,
}

impl ExternalActionRequestV1 {
    /// Constructs a canonical request and derives its identity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        worldline_id: WorldlineId,
        operation_id: ExternalActionOperationIdV1,
        input_schema_digest: Hash,
        settlement_schema_digest: Hash,
        authority_scope_digest: Hash,
        basis_digest: Hash,
        budget: ExternalActionBudgetV1,
        input_digest: Hash,
        reconciliation_law_digest: Hash,
    ) -> Result<Self, ExternalActionProtocolErrorV1> {
        if budget.max_settlement_bytes == 0 || budget.max_attempts == 0 {
            return Err(ExternalActionProtocolErrorV1::EmptyBudget);
        }
        let mut request = Self {
            request_id: ExternalActionRequestIdV1::from_hash([0; 32]),
            worldline_id,
            operation_id,
            input_schema_digest,
            settlement_schema_digest,
            authority_scope_digest,
            basis_digest,
            budget,
            input_digest,
            reconciliation_law_digest,
        };
        request.request_id =
            ExternalActionRequestIdV1::from_hash(request.expected_request_id_digest());
        Ok(request)
    }

    /// Returns the canonical request identity.
    #[must_use]
    pub const fn request_id(&self) -> ExternalActionRequestIdV1 {
        self.request_id
    }

    fn expected_request_id_digest(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(REQUEST_ID_DOMAIN);
        hasher.update(self.worldline_id.as_bytes());
        hasher.update(&self.operation_id.as_hash());
        hasher.update(&self.input_schema_digest);
        hasher.update(&self.settlement_schema_digest);
        hasher.update(&self.authority_scope_digest);
        hasher.update(&self.basis_digest);
        hasher.update(&self.budget.max_settlement_bytes.to_le_bytes());
        hasher.update(&self.budget.max_attempts.to_le_bytes());
        hasher.update(&self.input_digest);
        hasher.update(&self.reconciliation_law_digest);
        hasher.finalize().into()
    }
}

/// Runtime authorization binding one adapter to one operation and scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionAdapterAuthorizationV1 {
    /// Authorized adapter.
    pub adapter_id: ExternalActionAdapterIdV1,
    /// Authorized operation family.
    pub operation_id: ExternalActionOperationIdV1,
    /// Authorized request scope.
    pub authority_scope_digest: Hash,
}

/// One durably recorded adapter claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionClaimV1 {
    /// Request being claimed.
    pub request_id: ExternalActionRequestIdV1,
    /// Stable attempt identity.
    pub attempt_id: ExternalActionAttemptIdV1,
    /// Zero-based attempt ordinal.
    pub attempt_ordinal: u32,
    /// Authorized adapter.
    pub adapter_id: ExternalActionAdapterIdV1,
    /// Lease or fencing evidence.
    pub lease_evidence_digest: Hash,
    /// Request-stable idempotency key.
    pub idempotency_key: Hash,
    /// Named reconciliation law copied from the request.
    pub reconciliation_law_digest: Hash,
    /// Exact basis copied from the request.
    pub basis_digest: Hash,
}

/// Typed terminal observation supplied by an external adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalActionSettlementKindV1 {
    /// The requested external postcondition was established.
    Succeeded,
    /// The external system rejected the request as a typed refusal.
    Rejected,
    /// The adapter established that execution failed.
    Failed,
    /// The adapter cannot establish whether the external effect occurred.
    OutcomeUnknown,
}

impl ExternalActionSettlementKindV1 {
    /// Returns the stable code retained in the WAL payload.
    #[must_use]
    pub const fn stable_code(self) -> u8 {
        match self {
            Self::Succeeded => 1,
            Self::Rejected => 2,
            Self::Failed => 3,
            Self::OutcomeUnknown => 4,
        }
    }
}

/// Untrusted settlement candidate submitted for Echo admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalActionSettlementCandidateV1 {
    /// Request being settled.
    pub request_id: ExternalActionRequestIdV1,
    /// Attempt being settled.
    pub attempt_id: ExternalActionAttemptIdV1,
    /// Adapter claiming the result.
    pub adapter_id: ExternalActionAdapterIdV1,
    /// Typed terminal outcome.
    pub kind: ExternalActionSettlementKindV1,
    /// Claimed settlement schema.
    pub settlement_schema_digest: Hash,
    /// Exact request basis.
    pub basis_digest: Hash,
    /// Canonical result bytes retained for replay.
    pub canonical_result_bytes: Vec<u8>,
    /// Claimed digest of the canonical result bytes.
    pub declared_result_digest: Hash,
    /// Evidence for schema admission.
    pub schema_admission_evidence_digest: Hash,
    /// Adapter-supplied external evidence digest.
    pub external_evidence_digest: Hash,
}

impl ExternalActionSettlementCandidateV1 {
    /// Builds a candidate with a correctly declared result digest.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        request_id: ExternalActionRequestIdV1,
        attempt_id: ExternalActionAttemptIdV1,
        adapter_id: ExternalActionAdapterIdV1,
        kind: ExternalActionSettlementKindV1,
        settlement_schema_digest: Hash,
        basis_digest: Hash,
        canonical_result_bytes: Vec<u8>,
        schema_admission_evidence_digest: Hash,
        external_evidence_digest: Hash,
    ) -> Self {
        let declared_result_digest = blake3::hash(&canonical_result_bytes).into();
        Self {
            request_id,
            attempt_id,
            adapter_id,
            kind,
            settlement_schema_digest,
            basis_digest,
            canonical_result_bytes,
            declared_result_digest,
            schema_admission_evidence_digest,
            external_evidence_digest,
        }
    }
}

/// Admitted settlement retained in Echo history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalActionSettlementV1 {
    /// Request being settled.
    pub request_id: ExternalActionRequestIdV1,
    /// Attempt being settled.
    pub attempt_id: ExternalActionAttemptIdV1,
    /// Authorized adapter identity.
    pub adapter_id: ExternalActionAdapterIdV1,
    /// Typed terminal outcome.
    pub kind: ExternalActionSettlementKindV1,
    /// Admitted settlement schema.
    pub settlement_schema_digest: Hash,
    /// Exact request basis.
    pub basis_digest: Hash,
    /// Canonical result bytes retained for replay.
    pub canonical_result_bytes: Vec<u8>,
    /// Digest of the canonical result bytes.
    pub result_digest: Hash,
    /// Evidence for schema admission.
    pub schema_admission_evidence_digest: Hash,
    /// Adapter-supplied external evidence digest.
    pub external_evidence_digest: Hash,
}

/// Proof that a request was committed before adapter execution became reachable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurablyRecordedExternalActionRequestV1 {
    request: ExternalActionRequestV1,
    request_commit_digest: Hash,
}

impl DurablyRecordedExternalActionRequestV1 {
    /// Returns the recorded request.
    #[must_use]
    pub const fn request(&self) -> ExternalActionRequestV1 {
        self.request
    }

    /// Returns the WAL commit that made the request durable.
    #[must_use]
    pub const fn request_commit_digest(&self) -> Hash {
        self.request_commit_digest
    }
}

/// Adapter work grant returned only after the claim transaction is durable.
#[derive(Debug, PartialEq, Eq)]
pub struct ExternalActionClaimGrantV1 {
    request: ExternalActionRequestV1,
    claim: ExternalActionClaimV1,
    claim_commit_digest: Hash,
}

impl ExternalActionClaimGrantV1 {
    /// Returns the exact request an adapter may attempt.
    #[must_use]
    pub const fn request(&self) -> ExternalActionRequestV1 {
        self.request
    }

    /// Returns the exact attempt claim.
    #[must_use]
    pub const fn claim(&self) -> ExternalActionClaimV1 {
        self.claim
    }

    /// Returns the WAL commit that made the claim durable.
    #[must_use]
    pub const fn claim_commit_digest(&self) -> Hash {
        self.claim_commit_digest
    }
}

/// Settlement value exposed to deterministic execution only after WAL commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedExternalActionSettlementV1 {
    settlement: ExternalActionSettlementV1,
    settlement_commit_digest: Hash,
}

impl AdmittedExternalActionSettlementV1 {
    /// Returns the admitted settlement fact.
    #[must_use]
    pub const fn settlement(&self) -> &ExternalActionSettlementV1 {
        &self.settlement
    }

    /// Returns the WAL commit that made resumption lawful.
    #[must_use]
    pub const fn settlement_commit_digest(&self) -> Hash {
        self.settlement_commit_digest
    }
}

/// Recovered lifecycle posture for one external request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveredExternalActionPostureV1 {
    /// Request is durable and has not been claimed.
    Requested,
    /// Claim is durable; recovery must reconcile rather than reissue.
    Claimed,
    /// Settlement is durable and replayable.
    Settled(ExternalActionSettlementKindV1),
}

/// One request reconstructed entirely from committed WAL history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveredExternalActionV1 {
    /// Canonical request.
    pub request: ExternalActionRequestV1,
    /// Recorded claim, when present.
    pub claim: Option<ExternalActionClaimV1>,
    /// Admitted settlement, when present.
    pub settlement: Option<ExternalActionSettlementV1>,
    /// Lifecycle posture.
    pub posture: RecoveredExternalActionPostureV1,
}

/// Recovered external-action lifecycle index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveredExternalActionIndexV1 {
    entries: BTreeMap<ExternalActionRequestIdV1, RecoveredExternalActionV1>,
}

impl RecoveredExternalActionIndexV1 {
    /// Returns one recovered request.
    #[must_use]
    pub fn get(&self, request_id: ExternalActionRequestIdV1) -> Option<&RecoveredExternalActionV1> {
        self.entries.get(&request_id)
    }

    /// Returns the number of recovered requests.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the index contains no requests.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Fail-closed protocol and admission errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExternalActionProtocolErrorV1 {
    /// RED scaffold until the lifecycle implementation lands.
    #[error("external-action protocol is not implemented")]
    NotImplemented,
    /// A request delegated no usable result or attempt budget.
    #[error("external-action request budget must be non-zero")]
    EmptyBudget,
    /// The request identity did not match its canonical fields.
    #[error("external-action request identity mismatch")]
    RequestIdentityMismatch,
    /// The adapter did not own the requested operation and scope.
    #[error("external-action adapter is unauthorized")]
    UnauthorizedAdapter,
    /// The current basis differed from the request basis.
    #[error("external-action request basis is stale")]
    StaleBasis,
    /// The attempt exceeded the delegated request budget.
    #[error("external-action attempt budget exhausted")]
    AttemptBudgetExhausted,
    /// The settlement named a schema other than the request schema.
    #[error("external-action settlement schema mismatch")]
    SettlementSchemaMismatch,
    /// The settlement result digest did not match its canonical bytes.
    #[error("external-action settlement result digest mismatch")]
    SettlementResultDigestMismatch,
    /// The settlement exceeded the delegated byte budget.
    #[error("external-action settlement byte budget exceeded")]
    SettlementBudgetExceeded,
    /// The settlement did not name the claimed request, attempt, adapter, or basis.
    #[error("external-action settlement claim binding mismatch")]
    SettlementClaimMismatch,
    /// A request was recorded more than once.
    #[error("duplicate external-action request")]
    DuplicateRequest,
    /// A request was claimed more than once.
    #[error("duplicate external-action claim")]
    DuplicateClaim,
    /// A request was settled more than once.
    #[error("duplicate external-action settlement")]
    DuplicateSettlement,
    /// Distinct settlements claimed the same request.
    #[error("conflicting external-action settlement")]
    ConflictingSettlement,
    /// A claim appeared without its request.
    #[error("external-action claim is missing its request")]
    MissingRequest,
    /// A settlement appeared without its claim.
    #[error("external-action settlement is missing its claim")]
    MissingClaim,
    /// Canonical payload decoding failed.
    #[error(transparent)]
    Decode(#[from] WalDecodeError),
    /// WAL transaction construction failed.
    #[error(transparent)]
    WalBuild(#[from] WalBuildError),
    /// Durable WAL append failed.
    #[error(transparent)]
    WalStore(#[from] WalStoreError),
}

/// Builds a request-admission WAL transaction.
pub fn build_external_action_request_transaction(
    _builder: WalTransactionBuilder,
    _request: ExternalActionRequestV1,
    _affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    Err(ExternalActionProtocolErrorV1::NotImplemented)
}

/// Builds a claim WAL transaction.
pub fn build_external_action_claim_transaction(
    _builder: WalTransactionBuilder,
    _claim: ExternalActionClaimV1,
    _affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    Err(ExternalActionProtocolErrorV1::NotImplemented)
}

/// Builds a settlement-admission WAL transaction.
pub fn build_external_action_settlement_transaction(
    _builder: WalTransactionBuilder,
    _settlement: ExternalActionSettlementV1,
    _affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    Err(ExternalActionProtocolErrorV1::NotImplemented)
}

/// Commits a request before returning the only value accepted by claim admission.
pub fn record_external_action_request(
    _store: &mut impl WalStorePort,
    _builder: WalTransactionBuilder,
    _request: ExternalActionRequestV1,
    _affected_frontiers: Vec<AffectedFrontier>,
) -> Result<DurablyRecordedExternalActionRequestV1, ExternalActionProtocolErrorV1> {
    Err(ExternalActionProtocolErrorV1::NotImplemented)
}

/// Commits a bounded claim before returning adapter work authority.
#[allow(clippy::too_many_arguments)]
pub fn claim_external_action(
    _store: &mut impl WalStorePort,
    _builder: WalTransactionBuilder,
    _recorded_request: DurablyRecordedExternalActionRequestV1,
    _authorization: ExternalActionAdapterAuthorizationV1,
    _current_basis_digest: Hash,
    _attempt_ordinal: u32,
    _lease_evidence_digest: Hash,
    _affected_frontiers: Vec<AffectedFrontier>,
) -> Result<ExternalActionClaimGrantV1, ExternalActionProtocolErrorV1> {
    Err(ExternalActionProtocolErrorV1::NotImplemented)
}

/// Validates and commits a settlement before returning a resumable fact.
pub fn admit_external_action_settlement(
    _store: &mut impl WalStorePort,
    _builder: WalTransactionBuilder,
    _claim_grant: ExternalActionClaimGrantV1,
    _candidate: ExternalActionSettlementCandidateV1,
    _affected_frontiers: Vec<AffectedFrontier>,
) -> Result<AdmittedExternalActionSettlementV1, ExternalActionProtocolErrorV1> {
    Err(ExternalActionProtocolErrorV1::NotImplemented)
}

/// Reconstructs request, claim, and settlement posture from committed WAL history.
pub fn recover_external_actions(
    _report: &RecoveryScanReport,
) -> Result<RecoveredExternalActionIndexV1, ExternalActionProtocolErrorV1> {
    Err(ExternalActionProtocolErrorV1::NotImplemented)
}
