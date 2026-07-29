// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Durable, domain-neutral external-action request and settlement protocol.
//!
//! Edict-authored programs construct request values. Echo records each request
//! before an adapter may act, records one bounded claim, and admits one
//! schema-bound settlement before deterministic resumption. This module does
//! not execute external effects and grants no filesystem, process, network, or
//! model authority.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::causal_wal::{
    recover_from_frames_and_commits, AffectedFrontier, RecoveryAccessMode, RecoveryScanReport,
    RecoveryTailPosture, WalBuildError, WalCommittedTransaction, WalDecodeError, WalRecordKind,
    WalRecoveryError, WalStoreError, WalStorePort, WalTransactionBuilder, WalTransactionKind,
};
use crate::{Hash, WorldlineId};

const REQUEST_ID_DOMAIN: &[u8] = b"echo:external-action:request-id:v1\0";
const ATTEMPT_ID_DOMAIN: &[u8] = b"echo:external-action:attempt-id:v1\0";
const IDEMPOTENCY_KEY_DOMAIN: &[u8] = b"echo:external-action:idempotency-key:v1\0";
const REQUEST_PAYLOAD_MAGIC: &[u8; 4] = b"EAR1";
const CLAIM_PAYLOAD_MAGIC: &[u8; 4] = b"EAC1";
const SETTLEMENT_PAYLOAD_MAGIC: &[u8; 4] = b"EAS1";

/// Absolute v1 ceiling for settlement bytes retained directly in the WAL.
pub const MAX_EXTERNAL_ACTION_SETTLEMENT_BYTES_V1: u64 = 1_048_576;

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
        if budget.max_attempts != 1 {
            return Err(ExternalActionProtocolErrorV1::UnsupportedAttemptBudget);
        }
        if budget.max_settlement_bytes > MAX_EXTERNAL_ACTION_SETTLEMENT_BYTES_V1 {
            return Err(ExternalActionProtocolErrorV1::RequestBudgetLimitExceeded);
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

    fn validate_identity(&self) -> Result<(), ExternalActionProtocolErrorV1> {
        if self.request_id.as_hash() != self.expected_request_id_digest() {
            return Err(ExternalActionProtocolErrorV1::RequestIdentityMismatch);
        }
        if self.budget.max_settlement_bytes == 0 || self.budget.max_attempts == 0 {
            return Err(ExternalActionProtocolErrorV1::EmptyBudget);
        }
        if self.budget.max_attempts != 1 {
            return Err(ExternalActionProtocolErrorV1::UnsupportedAttemptBudget);
        }
        if self.budget.max_settlement_bytes > MAX_EXTERNAL_ACTION_SETTLEMENT_BYTES_V1 {
            return Err(ExternalActionProtocolErrorV1::RequestBudgetLimitExceeded);
        }
        Ok(())
    }

    fn to_payload_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + (9 * 32) + 12);
        out.extend_from_slice(REQUEST_PAYLOAD_MAGIC);
        out.extend_from_slice(&self.request_id.as_hash());
        out.extend_from_slice(self.worldline_id.as_bytes());
        out.extend_from_slice(&self.operation_id.as_hash());
        out.extend_from_slice(&self.input_schema_digest);
        out.extend_from_slice(&self.settlement_schema_digest);
        out.extend_from_slice(&self.authority_scope_digest);
        out.extend_from_slice(&self.basis_digest);
        out.extend_from_slice(&self.budget.max_settlement_bytes.to_le_bytes());
        out.extend_from_slice(&self.budget.max_attempts.to_le_bytes());
        out.extend_from_slice(&self.input_digest);
        out.extend_from_slice(&self.reconciliation_law_digest);
        out
    }

    fn from_payload_bytes(bytes: &[u8]) -> Result<Self, ExternalActionProtocolErrorV1> {
        let mut cursor = ExternalActionPayloadCursor::new(bytes);
        cursor.expect_magic(REQUEST_PAYLOAD_MAGIC, "ExternalActionRequestV1")?;
        let request = Self {
            request_id: ExternalActionRequestIdV1::from_hash(cursor.read_hash()?),
            worldline_id: WorldlineId::from_bytes(cursor.read_hash()?),
            operation_id: ExternalActionOperationIdV1::from_hash(cursor.read_hash()?),
            input_schema_digest: cursor.read_hash()?,
            settlement_schema_digest: cursor.read_hash()?,
            authority_scope_digest: cursor.read_hash()?,
            basis_digest: cursor.read_hash()?,
            budget: ExternalActionBudgetV1 {
                max_settlement_bytes: cursor.read_u64()?,
                max_attempts: cursor.read_u32()?,
            },
            input_digest: cursor.read_hash()?,
            reconciliation_law_digest: cursor.read_hash()?,
        };
        cursor.finish()?;
        request.validate_identity()?;
        Ok(request)
    }
}

/// Runtime-owner binding that permits one adapter for one operation and scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionAdapterBindingV1 {
    /// Adapter admitted by the runtime owner.
    pub adapter_id: ExternalActionAdapterIdV1,
    /// Operation family the adapter may perform.
    pub operation_id: ExternalActionOperationIdV1,
    /// Maximum authority scope admitted for the adapter.
    pub authority_scope_digest: Hash,
}

/// Runtime-owned adapter registry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalActionAdapterRegistryV1 {
    bindings: BTreeMap<(ExternalActionOperationIdV1, Hash), BTreeSet<ExternalActionAdapterIdV1>>,
}

impl ExternalActionAdapterRegistryV1 {
    /// Builds a runtime-owned registry from explicit domain-specific bindings.
    #[must_use]
    pub fn new(bindings: impl IntoIterator<Item = ExternalActionAdapterBindingV1>) -> Self {
        let mut registry = Self::default();
        for binding in bindings {
            registry
                .bindings
                .entry((binding.operation_id, binding.authority_scope_digest))
                .or_default()
                .insert(binding.adapter_id);
        }
        registry
    }

    /// Attenuates runtime-owner policy into one request-specific authorization.
    pub fn authorize(
        &self,
        request: &ExternalActionRequestV1,
        adapter_id: ExternalActionAdapterIdV1,
    ) -> Result<ExternalActionAdapterAuthorizationV1, ExternalActionProtocolErrorV1> {
        let admitted = self
            .bindings
            .get(&(request.operation_id, request.authority_scope_digest))
            .is_some_and(|adapters| adapters.contains(&adapter_id));
        if !admitted {
            return Err(ExternalActionProtocolErrorV1::UnauthorizedAdapter);
        }
        Ok(ExternalActionAdapterAuthorizationV1 {
            adapter_id,
            operation_id: request.operation_id,
            authority_scope_digest: request.authority_scope_digest,
        })
    }
}

/// Request-specific authorization attenuated from the runtime-owned registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionAdapterAuthorizationV1 {
    adapter_id: ExternalActionAdapterIdV1,
    operation_id: ExternalActionOperationIdV1,
    authority_scope_digest: Hash,
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

impl ExternalActionClaimV1 {
    fn for_request(
        request: &ExternalActionRequestV1,
        adapter_id: ExternalActionAdapterIdV1,
        attempt_ordinal: u32,
        lease_evidence_digest: Hash,
    ) -> Self {
        let idempotency_key = external_action_idempotency_key(request);
        let attempt_id = external_action_attempt_id(
            request.request_id,
            attempt_ordinal,
            adapter_id,
            lease_evidence_digest,
        );
        Self {
            request_id: request.request_id,
            attempt_id,
            attempt_ordinal,
            adapter_id,
            lease_evidence_digest,
            idempotency_key,
            reconciliation_law_digest: request.reconciliation_law_digest,
            basis_digest: request.basis_digest,
        }
    }

    fn to_payload_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + (7 * 32) + 4);
        out.extend_from_slice(CLAIM_PAYLOAD_MAGIC);
        out.extend_from_slice(&self.request_id.as_hash());
        out.extend_from_slice(&self.attempt_id.as_hash());
        out.extend_from_slice(&self.attempt_ordinal.to_le_bytes());
        out.extend_from_slice(&self.adapter_id.as_hash());
        out.extend_from_slice(&self.lease_evidence_digest);
        out.extend_from_slice(&self.idempotency_key);
        out.extend_from_slice(&self.reconciliation_law_digest);
        out.extend_from_slice(&self.basis_digest);
        out
    }

    fn from_payload_bytes(bytes: &[u8]) -> Result<Self, ExternalActionProtocolErrorV1> {
        let mut cursor = ExternalActionPayloadCursor::new(bytes);
        cursor.expect_magic(CLAIM_PAYLOAD_MAGIC, "ExternalActionClaimV1")?;
        let claim = Self {
            request_id: ExternalActionRequestIdV1::from_hash(cursor.read_hash()?),
            attempt_id: ExternalActionAttemptIdV1::from_hash(cursor.read_hash()?),
            attempt_ordinal: cursor.read_u32()?,
            adapter_id: ExternalActionAdapterIdV1::from_hash(cursor.read_hash()?),
            lease_evidence_digest: cursor.read_hash()?,
            idempotency_key: cursor.read_hash()?,
            reconciliation_law_digest: cursor.read_hash()?,
            basis_digest: cursor.read_hash()?,
        };
        cursor.finish()?;
        Ok(claim)
    }
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

    fn from_stable_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::Succeeded),
            2 => Ok(Self::Rejected),
            3 => Ok(Self::Failed),
            4 => Ok(Self::OutcomeUnknown),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "ExternalActionSettlementKindV1",
                code,
            }),
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

impl ExternalActionSettlementV1 {
    fn from_candidate(candidate: ExternalActionSettlementCandidateV1) -> Self {
        Self {
            request_id: candidate.request_id,
            attempt_id: candidate.attempt_id,
            adapter_id: candidate.adapter_id,
            kind: candidate.kind,
            settlement_schema_digest: candidate.settlement_schema_digest,
            basis_digest: candidate.basis_digest,
            canonical_result_bytes: candidate.canonical_result_bytes,
            result_digest: candidate.declared_result_digest,
            schema_admission_evidence_digest: candidate.schema_admission_evidence_digest,
            external_evidence_digest: candidate.external_evidence_digest,
        }
    }

    fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + 1 + (8 * 32) + 8 + self.canonical_result_bytes.len());
        out.extend_from_slice(SETTLEMENT_PAYLOAD_MAGIC);
        out.extend_from_slice(&self.request_id.as_hash());
        out.extend_from_slice(&self.attempt_id.as_hash());
        out.extend_from_slice(&self.adapter_id.as_hash());
        out.push(self.kind.stable_code());
        out.extend_from_slice(&self.settlement_schema_digest);
        out.extend_from_slice(&self.basis_digest);
        out.extend_from_slice(
            &u64::try_from(self.canonical_result_bytes.len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        out.extend_from_slice(&self.canonical_result_bytes);
        out.extend_from_slice(&self.result_digest);
        out.extend_from_slice(&self.schema_admission_evidence_digest);
        out.extend_from_slice(&self.external_evidence_digest);
        out
    }

    fn from_payload_bytes(bytes: &[u8]) -> Result<Self, ExternalActionProtocolErrorV1> {
        let mut cursor = ExternalActionPayloadCursor::new(bytes);
        cursor.expect_magic(SETTLEMENT_PAYLOAD_MAGIC, "ExternalActionSettlementV1")?;
        let request_id = ExternalActionRequestIdV1::from_hash(cursor.read_hash()?);
        let attempt_id = ExternalActionAttemptIdV1::from_hash(cursor.read_hash()?);
        let adapter_id = ExternalActionAdapterIdV1::from_hash(cursor.read_hash()?);
        let kind = ExternalActionSettlementKindV1::from_stable_code(cursor.read_u8()?)?;
        let settlement_schema_digest = cursor.read_hash()?;
        let basis_digest = cursor.read_hash()?;
        let result_len = cursor.read_u64()?;
        if result_len > MAX_EXTERNAL_ACTION_SETTLEMENT_BYTES_V1 {
            return Err(ExternalActionProtocolErrorV1::SettlementBudgetExceeded);
        }
        let result_len = usize::try_from(result_len)
            .map_err(|_| ExternalActionProtocolErrorV1::SettlementBudgetExceeded)?;
        let canonical_result_bytes = cursor.read_bytes(result_len)?.to_vec();
        let settlement = Self {
            request_id,
            attempt_id,
            adapter_id,
            kind,
            settlement_schema_digest,
            basis_digest,
            canonical_result_bytes,
            result_digest: cursor.read_hash()?,
            schema_admission_evidence_digest: cursor.read_hash()?,
            external_evidence_digest: cursor.read_hash()?,
        };
        cursor.finish()?;
        if Hash::from(blake3::hash(&settlement.canonical_result_bytes)) != settlement.result_digest
        {
            return Err(ExternalActionProtocolErrorV1::SettlementResultDigestMismatch);
        }
        Ok(settlement)
    }
}

/// Proof that a request was committed before adapter execution became reachable.
#[derive(Debug, PartialEq, Eq)]
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
    /// A request delegated no usable result or attempt budget.
    #[error("external-action request budget must be non-zero")]
    EmptyBudget,
    /// A request exceeded Echo's absolute retained-settlement ceiling.
    #[error("external-action request settlement budget exceeds the v1 limit")]
    RequestBudgetLimitExceeded,
    /// Protocol v1 permits exactly one claim per request.
    #[error("external-action protocol v1 requires exactly one attempt per request")]
    UnsupportedAttemptBudget,
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
    /// A recovered claim did not match its exact request.
    #[error("external-action claim binding mismatch")]
    ClaimBindingMismatch,
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
    /// WAL recovery found an uncommitted tail; lifecycle admission must stop.
    #[error("external-action admission requires a clean committed WAL tail")]
    WalTailNotClean,
    /// Schema admission evidence was absent.
    #[error("external-action settlement omitted schema admission evidence")]
    MissingSchemaAdmissionEvidence,
    /// Canonical payload decoding failed.
    #[error(transparent)]
    Decode(#[from] WalDecodeError),
    /// WAL transaction construction failed.
    #[error(transparent)]
    WalBuild(#[from] WalBuildError),
    /// Durable WAL append failed.
    #[error(transparent)]
    WalStore(#[from] WalStoreError),
    /// Reading current committed WAL posture failed.
    #[error(transparent)]
    WalRecovery(#[from] WalRecoveryError),
}

/// Builds a request-admission WAL transaction.
pub fn build_external_action_request_transaction(
    mut builder: WalTransactionBuilder,
    request: ExternalActionRequestV1,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    request.validate_identity()?;
    builder.push_record(
        WalRecordKind::ExternalActionRequestRecorded,
        request.to_payload_bytes(),
    )?;
    Ok(builder.commit(affected_frontiers)?)
}

/// Builds a claim WAL transaction.
pub fn build_external_action_claim_transaction(
    mut builder: WalTransactionBuilder,
    claim: ExternalActionClaimV1,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    builder.push_record(
        WalRecordKind::ExternalActionClaimRecorded,
        claim.to_payload_bytes(),
    )?;
    Ok(builder.commit(affected_frontiers)?)
}

/// Builds a settlement-admission WAL transaction.
pub fn build_external_action_settlement_transaction(
    mut builder: WalTransactionBuilder,
    settlement: ExternalActionSettlementV1,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    builder.push_record(
        WalRecordKind::ExternalActionSettlementRecorded,
        settlement.to_payload_bytes(),
    )?;
    Ok(builder.commit(affected_frontiers)?)
}

/// Commits a request before returning the only value accepted by claim admission.
pub fn record_external_action_request(
    store: &mut impl WalStorePort,
    builder: WalTransactionBuilder,
    request: ExternalActionRequestV1,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<DurablyRecordedExternalActionRequestV1, ExternalActionProtocolErrorV1> {
    let index = recover_external_action_index_from_store(store)?;
    if index.get(request.request_id).is_some() {
        return Err(ExternalActionProtocolErrorV1::DuplicateRequest);
    }
    let transaction =
        build_external_action_request_transaction(builder, request, affected_frontiers)?;
    let request_commit_digest = append_external_action_transaction(store, transaction)?;
    Ok(DurablyRecordedExternalActionRequestV1 {
        request,
        request_commit_digest,
    })
}

/// Commits a bounded claim before returning adapter work authority.
#[allow(clippy::too_many_arguments)]
pub fn claim_external_action(
    store: &mut impl WalStorePort,
    builder: WalTransactionBuilder,
    recorded_request: DurablyRecordedExternalActionRequestV1,
    authorization: ExternalActionAdapterAuthorizationV1,
    current_basis_digest: Hash,
    attempt_ordinal: u32,
    lease_evidence_digest: Hash,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<ExternalActionClaimGrantV1, ExternalActionProtocolErrorV1> {
    let request = recorded_request.request;
    request.validate_identity()?;
    let index = recover_external_action_index_from_store(store)?;
    let recovered = index
        .get(request.request_id)
        .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
    if recovered.request != request {
        return Err(ExternalActionProtocolErrorV1::RequestIdentityMismatch);
    }
    if recovered.claim.is_some() {
        return Err(ExternalActionProtocolErrorV1::DuplicateClaim);
    }
    if authorization.operation_id != request.operation_id
        || authorization.authority_scope_digest != request.authority_scope_digest
    {
        return Err(ExternalActionProtocolErrorV1::UnauthorizedAdapter);
    }
    if current_basis_digest != request.basis_digest {
        return Err(ExternalActionProtocolErrorV1::StaleBasis);
    }
    if attempt_ordinal >= request.budget.max_attempts {
        return Err(ExternalActionProtocolErrorV1::AttemptBudgetExhausted);
    }
    let claim = ExternalActionClaimV1::for_request(
        &request,
        authorization.adapter_id,
        attempt_ordinal,
        lease_evidence_digest,
    );
    let transaction = build_external_action_claim_transaction(builder, claim, affected_frontiers)?;
    let claim_commit_digest = append_external_action_transaction(store, transaction)?;
    Ok(ExternalActionClaimGrantV1 {
        request,
        claim,
        claim_commit_digest,
    })
}

/// Validates and commits a settlement before returning a resumable fact.
pub fn admit_external_action_settlement(
    store: &mut impl WalStorePort,
    builder: WalTransactionBuilder,
    claim_grant: ExternalActionClaimGrantV1,
    candidate: ExternalActionSettlementCandidateV1,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<AdmittedExternalActionSettlementV1, ExternalActionProtocolErrorV1> {
    let index = recover_external_action_index_from_store(store)?;
    let recovered = index
        .get(claim_grant.request.request_id)
        .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
    let recovered_claim = recovered
        .claim
        .ok_or(ExternalActionProtocolErrorV1::MissingClaim)?;
    if recovered.request != claim_grant.request || recovered_claim != claim_grant.claim {
        return Err(ExternalActionProtocolErrorV1::SettlementClaimMismatch);
    }
    if recovered.settlement.is_some() {
        return Err(ExternalActionProtocolErrorV1::DuplicateSettlement);
    }
    validate_settlement_candidate(&claim_grant.request, claim_grant.claim, &candidate)?;
    let settlement = ExternalActionSettlementV1::from_candidate(candidate);
    let transaction = build_external_action_settlement_transaction(
        builder,
        settlement.clone(),
        affected_frontiers,
    )?;
    let settlement_commit_digest = append_external_action_transaction(store, transaction)?;
    Ok(AdmittedExternalActionSettlementV1 {
        settlement,
        settlement_commit_digest,
    })
}

/// Reconstructs request, claim, and settlement posture from committed WAL history.
pub fn recover_external_actions(
    report: &RecoveryScanReport,
) -> Result<RecoveredExternalActionIndexV1, ExternalActionProtocolErrorV1> {
    let mut entries = BTreeMap::<ExternalActionRequestIdV1, RecoveredExternalActionV1>::new();
    for transaction in &report.transactions {
        let Some(frame) =
            external_action_frame(transaction.commit.transaction_kind, &transaction.frames)?
        else {
            continue;
        };
        match frame.header.record_kind {
            WalRecordKind::ExternalActionRequestRecorded => {
                let request =
                    ExternalActionRequestV1::from_payload_bytes(&frame.payload.canonical_bytes)?;
                if entries.contains_key(&request.request_id) {
                    return Err(ExternalActionProtocolErrorV1::DuplicateRequest);
                }
                entries.insert(
                    request.request_id,
                    RecoveredExternalActionV1 {
                        request,
                        claim: None,
                        settlement: None,
                        posture: RecoveredExternalActionPostureV1::Requested,
                    },
                );
            }
            WalRecordKind::ExternalActionClaimRecorded => {
                let claim =
                    ExternalActionClaimV1::from_payload_bytes(&frame.payload.canonical_bytes)?;
                let entry = entries
                    .get_mut(&claim.request_id)
                    .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
                if entry.claim.is_some() {
                    return Err(ExternalActionProtocolErrorV1::DuplicateClaim);
                }
                validate_claim(&entry.request, claim)?;
                entry.claim = Some(claim);
                entry.posture = RecoveredExternalActionPostureV1::Claimed;
            }
            WalRecordKind::ExternalActionSettlementRecorded => {
                let settlement =
                    ExternalActionSettlementV1::from_payload_bytes(&frame.payload.canonical_bytes)?;
                let entry = entries
                    .get_mut(&settlement.request_id)
                    .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
                let claim = entry
                    .claim
                    .ok_or(ExternalActionProtocolErrorV1::MissingClaim)?;
                validate_settlement(&entry.request, claim, &settlement)?;
                if let Some(existing) = &entry.settlement {
                    return if existing == &settlement {
                        Err(ExternalActionProtocolErrorV1::DuplicateSettlement)
                    } else {
                        Err(ExternalActionProtocolErrorV1::ConflictingSettlement)
                    };
                }
                entry.posture = RecoveredExternalActionPostureV1::Settled(settlement.kind);
                entry.settlement = Some(settlement);
            }
            _ => unreachable!("external_action_frame filters record kinds"),
        }
    }
    Ok(RecoveredExternalActionIndexV1 { entries })
}

fn external_action_idempotency_key(request: &ExternalActionRequestV1) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(IDEMPOTENCY_KEY_DOMAIN);
    hasher.update(&request.request_id.as_hash());
    hasher.update(&request.reconciliation_law_digest);
    hasher.finalize().into()
}

fn external_action_attempt_id(
    request_id: ExternalActionRequestIdV1,
    attempt_ordinal: u32,
    adapter_id: ExternalActionAdapterIdV1,
    lease_evidence_digest: Hash,
) -> ExternalActionAttemptIdV1 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(ATTEMPT_ID_DOMAIN);
    hasher.update(&request_id.as_hash());
    hasher.update(&attempt_ordinal.to_le_bytes());
    hasher.update(&adapter_id.as_hash());
    hasher.update(&lease_evidence_digest);
    ExternalActionAttemptIdV1::from_hash(hasher.finalize().into())
}

fn validate_claim(
    request: &ExternalActionRequestV1,
    claim: ExternalActionClaimV1,
) -> Result<(), ExternalActionProtocolErrorV1> {
    let expected = ExternalActionClaimV1::for_request(
        request,
        claim.adapter_id,
        claim.attempt_ordinal,
        claim.lease_evidence_digest,
    );
    if claim != expected {
        return Err(ExternalActionProtocolErrorV1::ClaimBindingMismatch);
    }
    if claim.attempt_ordinal >= request.budget.max_attempts {
        return Err(ExternalActionProtocolErrorV1::AttemptBudgetExhausted);
    }
    Ok(())
}

fn validate_settlement_candidate(
    request: &ExternalActionRequestV1,
    claim: ExternalActionClaimV1,
    candidate: &ExternalActionSettlementCandidateV1,
) -> Result<(), ExternalActionProtocolErrorV1> {
    if candidate.request_id != request.request_id
        || candidate.attempt_id != claim.attempt_id
        || candidate.adapter_id != claim.adapter_id
        || candidate.basis_digest != request.basis_digest
    {
        return Err(ExternalActionProtocolErrorV1::SettlementClaimMismatch);
    }
    if candidate.settlement_schema_digest != request.settlement_schema_digest {
        return Err(ExternalActionProtocolErrorV1::SettlementSchemaMismatch);
    }
    if candidate.schema_admission_evidence_digest == [0; 32] {
        return Err(ExternalActionProtocolErrorV1::MissingSchemaAdmissionEvidence);
    }
    if u64::try_from(candidate.canonical_result_bytes.len()).unwrap_or(u64::MAX)
        > request.budget.max_settlement_bytes
    {
        return Err(ExternalActionProtocolErrorV1::SettlementBudgetExceeded);
    }
    if Hash::from(blake3::hash(&candidate.canonical_result_bytes))
        != candidate.declared_result_digest
    {
        return Err(ExternalActionProtocolErrorV1::SettlementResultDigestMismatch);
    }
    Ok(())
}

fn validate_settlement(
    request: &ExternalActionRequestV1,
    claim: ExternalActionClaimV1,
    settlement: &ExternalActionSettlementV1,
) -> Result<(), ExternalActionProtocolErrorV1> {
    validate_settlement_candidate(
        request,
        claim,
        &ExternalActionSettlementCandidateV1 {
            request_id: settlement.request_id,
            attempt_id: settlement.attempt_id,
            adapter_id: settlement.adapter_id,
            kind: settlement.kind,
            settlement_schema_digest: settlement.settlement_schema_digest,
            basis_digest: settlement.basis_digest,
            canonical_result_bytes: settlement.canonical_result_bytes.clone(),
            declared_result_digest: settlement.result_digest,
            schema_admission_evidence_digest: settlement.schema_admission_evidence_digest,
            external_evidence_digest: settlement.external_evidence_digest,
        },
    )
}

fn append_external_action_transaction(
    store: &mut impl WalStorePort,
    transaction: WalCommittedTransaction,
) -> Result<Hash, ExternalActionProtocolErrorV1> {
    transaction.validate().map_err(WalBuildError::Validation)?;
    let epoch_id = transaction.commit.writer_epoch;
    let commit = transaction.commit;
    for frame in transaction.frames {
        store.append_frame(epoch_id, frame)?;
    }
    store.flush_commit(epoch_id, commit.clone())?;
    Ok(commit.commit_digest)
}

fn recover_external_action_index_from_store(
    store: &impl WalStorePort,
) -> Result<RecoveredExternalActionIndexV1, ExternalActionProtocolErrorV1> {
    let report = recover_from_frames_and_commits(
        &store.read_frames(),
        &store.read_commits(),
        RecoveryAccessMode::ReadOnly,
    )?;
    if report.tail_posture != RecoveryTailPosture::Clean {
        return Err(ExternalActionProtocolErrorV1::WalTailNotClean);
    }
    recover_external_actions(&report)
}

fn external_action_frame(
    transaction_kind: WalTransactionKind,
    frames: &[crate::causal_wal::WalFrame],
) -> Result<Option<&crate::causal_wal::WalFrame>, ExternalActionProtocolErrorV1> {
    let expected = match transaction_kind {
        WalTransactionKind::ExternalActionRequest => {
            Some(WalRecordKind::ExternalActionRequestRecorded)
        }
        WalTransactionKind::ExternalActionClaim => Some(WalRecordKind::ExternalActionClaimRecorded),
        WalTransactionKind::ExternalActionSettlement => {
            Some(WalRecordKind::ExternalActionSettlementRecorded)
        }
        _ => None,
    };
    let Some(expected) = expected else {
        return Ok(None);
    };
    if frames.len() != 1 || frames[0].header.record_kind != expected {
        return Err(WalBuildError::Validation(
            crate::causal_wal::WalValidationError::ExternalActionFrameShapeMismatch,
        )
        .into());
    }
    Ok(frames.first())
}

struct ExternalActionPayloadCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ExternalActionPayloadCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], WalDecodeError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        self.offset = end;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, WalDecodeError> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, WalDecodeError> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.read_bytes(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, WalDecodeError> {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.read_bytes(8)?);
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_hash(&mut self) -> Result<Hash, WalDecodeError> {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(self.read_bytes(32)?);
        Ok(bytes)
    }

    fn expect_magic(
        &mut self,
        magic: &[u8],
        record_kind: &'static str,
    ) -> Result<(), WalDecodeError> {
        if self.read_bytes(magic.len())? != magic {
            return Err(WalDecodeError::InvalidRecordMagic { record_kind });
        }
        Ok(())
    }

    fn finish(self) -> Result<(), WalDecodeError> {
        if self.offset != self.bytes.len() {
            return Err(WalDecodeError::TrailingBytes);
        }
        Ok(())
    }
}
