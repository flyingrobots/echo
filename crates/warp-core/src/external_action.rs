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
use std::sync::OnceLock;

use thiserror::Error;

use crate::causal_wal::{
    affected_frontiers_root, recover_from_frames_and_commits, AffectedFrontier,
    AffectedFrontierKind, Lsn, PayloadCodecId, PayloadSchemaId, RecoveryAccessMode,
    RecoveryScanReport, RecoveryTailPosture, WalBuildError, WalCommittedTransaction,
    WalDecodeError, WalDurabilityMode, WalRecordKind, WalRecoveryError, WalSegmentId,
    WalStoreError, WalStorePort, WalTransactionBuilder, WalTransactionId, WalTransactionKind,
    WriterEpochId,
};
use crate::{Hash, WorldlineId};

const REQUEST_ID_DOMAIN: &[u8] = b"echo:external-action:request-id:v1\0";
const ATTEMPT_ID_DOMAIN: &[u8] = b"echo:external-action:attempt-id:v1\0";
const IDEMPOTENCY_KEY_DOMAIN: &[u8] = b"echo:external-action:idempotency-key:v1\0";
const ADAPTER_REGISTRY_ID_DOMAIN: &[u8] = b"echo:external-action:adapter-registry-id:v1\0";
const INDEX_EMPTY_LEAF_DOMAIN: &[u8] = b"echo:external-action:index-empty-leaf:v1\0";
const INDEX_LEAF_DOMAIN: &[u8] = b"echo:external-action:index-leaf:v1\0";
const INDEX_NODE_DOMAIN: &[u8] = b"echo:external-action:index-node:v1\0";
const REQUEST_PAYLOAD_MAGIC: &[u8; 4] = b"EAR1";
const CLAIM_PAYLOAD_MAGIC: &[u8; 4] = b"EAC1";
const SETTLEMENT_PAYLOAD_MAGIC: &[u8; 4] = b"EAS1";

/// Absolute v1 ceiling for settlement bytes retained directly in the WAL.
pub const MAX_EXTERNAL_ACTION_SETTLEMENT_BYTES_V1: u64 = 1_048_576;

/// Non-causal transaction metadata supplied to the external-action coordinator.
///
/// LSN and predecessor coordinates are intentionally absent: the coordinator
/// derives them from its checked local WAL continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionTransactionContextV1 {
    /// Active writer epoch.
    pub writer_epoch: WriterEpochId,
    /// Active WAL segment.
    pub segment_id: WalSegmentId,
    /// Identity of this lifecycle transaction.
    pub transaction_id: WalTransactionId,
    /// Required durability mode.
    pub durability_mode: WalDurabilityMode,
    /// Canonical payload codec.
    pub payload_codec_id: PayloadCodecId,
    /// Canonical payload schema.
    pub payload_schema_id: PayloadSchemaId,
    /// Payload schema version.
    pub payload_schema_version: u16,
    /// Canonical encoding version.
    pub canonical_encoding_version: u16,
    /// Digest domain for WAL framing.
    pub digest_domain: Hash,
}

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
            request_id: request.request_id,
            basis_digest: request.basis_digest,
            registry_policy_digest: self.identity_digest(),
        })
    }

    /// Returns the canonical identity of the complete runtime-owned registry.
    #[must_use]
    pub fn identity_digest(&self) -> Hash {
        let binding_count = self
            .bindings
            .values()
            .map(BTreeSet::len)
            .fold(0_u64, |count, len| {
                count.saturating_add(u64::try_from(len).unwrap_or(u64::MAX))
            });
        let mut hasher = blake3::Hasher::new();
        hasher.update(ADAPTER_REGISTRY_ID_DOMAIN);
        hasher.update(&binding_count.to_le_bytes());
        for ((operation_id, authority_scope_digest), adapters) in &self.bindings {
            for adapter_id in adapters {
                hasher.update(&operation_id.as_hash());
                hasher.update(authority_scope_digest);
                hasher.update(&adapter_id.as_hash());
            }
        }
        hasher.finalize().into()
    }
}

/// Request-specific authorization attenuated from the runtime-owned registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalActionAdapterAuthorizationV1 {
    adapter_id: ExternalActionAdapterIdV1,
    operation_id: ExternalActionOperationIdV1,
    authority_scope_digest: Hash,
    request_id: ExternalActionRequestIdV1,
    basis_digest: Hash,
    registry_policy_digest: Hash,
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
    /// Runtime-owned registry policy that admitted this exact request.
    pub authorization_policy_digest: Hash,
}

impl ExternalActionClaimV1 {
    fn for_request(
        request: &ExternalActionRequestV1,
        adapter_id: ExternalActionAdapterIdV1,
        attempt_ordinal: u32,
        lease_evidence_digest: Hash,
        authorization_policy_digest: Hash,
    ) -> Self {
        let idempotency_key = external_action_idempotency_key(request);
        let attempt_id = external_action_attempt_id(
            request.request_id,
            attempt_ordinal,
            adapter_id,
            lease_evidence_digest,
            authorization_policy_digest,
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
            authorization_policy_digest,
        }
    }

    fn to_payload_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + (8 * 32) + 4);
        out.extend_from_slice(CLAIM_PAYLOAD_MAGIC);
        out.extend_from_slice(&self.request_id.as_hash());
        out.extend_from_slice(&self.attempt_id.as_hash());
        out.extend_from_slice(&self.attempt_ordinal.to_le_bytes());
        out.extend_from_slice(&self.adapter_id.as_hash());
        out.extend_from_slice(&self.lease_evidence_digest);
        out.extend_from_slice(&self.idempotency_key);
        out.extend_from_slice(&self.reconciliation_law_digest);
        out.extend_from_slice(&self.basis_digest);
        out.extend_from_slice(&self.authorization_policy_digest);
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
            authorization_policy_digest: cursor.read_hash()?,
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

/// Observation-only lifecycle reconstructed from a supplied recovery report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveredExternalActionV1 {
    /// Canonical request.
    pub request: ExternalActionRequestV1,
    /// Commit that durably admitted the request.
    pub request_commit_digest: Hash,
    /// Recorded claim, when present.
    pub claim: Option<ExternalActionClaimV1>,
    /// Commit that durably admitted the claim, when present.
    pub claim_commit_digest: Option<Hash>,
    /// Admitted settlement, when present.
    pub settlement: Option<ExternalActionSettlementV1>,
    /// Commit that durably admitted the settlement, when present.
    pub settlement_commit_digest: Option<Hash>,
    /// Lifecycle posture.
    pub posture: RecoveredExternalActionPostureV1,
}

/// Observation-only external-action lifecycle index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveredExternalActionIndexV1 {
    entries: BTreeMap<ExternalActionRequestIdV1, RecoveredExternalActionV1>,
    merkle_nodes: BTreeMap<ExternalActionIndexNodeKeyV1, Hash>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ExternalActionIndexNodeKeyV1 {
    depth: u16,
    prefix: Hash,
}

/// Trusted local coordinator state recovered from one fallible WAL snapshot.
///
/// Arbitrary recovery reports expose observation-only lifecycle values.
/// Transition grants and resumable settlement facts can be reconstructed only
/// through this locally recovered coordinator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalActionCoordinatorV1 {
    index: RecoveredExternalActionIndexV1,
    next_lsn: Lsn,
    previous_frame_digest: Hash,
    previous_commit_digest: Hash,
    ready: bool,
}

impl ExternalActionCoordinatorV1 {
    /// Recovers coordinator authority from one checked local-store snapshot.
    pub fn recover(store: &impl WalStorePort) -> Result<Self, ExternalActionProtocolErrorV1> {
        let snapshot = store.read_snapshot()?;
        let report = recover_from_frames_and_commits(
            &snapshot.frames,
            &snapshot.commits,
            RecoveryAccessMode::ReadOnly,
        )?;
        if report.tail_posture != RecoveryTailPosture::Clean {
            return Err(ExternalActionProtocolErrorV1::WalTailNotClean);
        }
        let index = observe_external_actions(&report)?;
        let (next_lsn, previous_frame_digest, previous_commit_digest) =
            external_action_wal_continuation(&report)?;
        Ok(Self {
            index,
            next_lsn,
            previous_frame_digest,
            previous_commit_digest,
            ready: true,
        })
    }

    /// Returns the observation-only lifecycle index.
    #[must_use]
    pub const fn observed_index(&self) -> &RecoveredExternalActionIndexV1 {
        &self.index
    }

    /// Reconstructs request-transition authority after a request-commit crash.
    pub fn recorded_request(
        &self,
        request_id: ExternalActionRequestIdV1,
    ) -> Result<DurablyRecordedExternalActionRequestV1, ExternalActionProtocolErrorV1> {
        self.ensure_ready()?;
        let entry = self
            .index
            .get(request_id)
            .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
        if entry.claim.is_some() {
            return Err(ExternalActionProtocolErrorV1::DuplicateClaim);
        }
        Ok(DurablyRecordedExternalActionRequestV1 {
            request: entry.request,
            request_commit_digest: entry.request_commit_digest,
        })
    }

    /// Reconstructs adapter settlement authority after a claim-commit crash.
    pub fn claim_grant(
        &self,
        request_id: ExternalActionRequestIdV1,
    ) -> Result<ExternalActionClaimGrantV1, ExternalActionProtocolErrorV1> {
        self.ensure_ready()?;
        let entry = self
            .index
            .get(request_id)
            .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
        let claim = entry
            .claim
            .ok_or(ExternalActionProtocolErrorV1::MissingClaim)?;
        if entry.settlement.is_some() {
            return Err(ExternalActionProtocolErrorV1::DuplicateSettlement);
        }
        Ok(ExternalActionClaimGrantV1 {
            request: entry.request,
            claim,
            claim_commit_digest: entry
                .claim_commit_digest
                .ok_or(ExternalActionProtocolErrorV1::MissingClaim)?,
        })
    }

    /// Reconstructs the deterministic resumption fact after settlement commit.
    pub fn admitted_settlement(
        &self,
        request_id: ExternalActionRequestIdV1,
    ) -> Result<AdmittedExternalActionSettlementV1, ExternalActionProtocolErrorV1> {
        self.ensure_ready()?;
        let entry = self
            .index
            .get(request_id)
            .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
        Ok(AdmittedExternalActionSettlementV1 {
            settlement: entry
                .settlement
                .clone()
                .ok_or(ExternalActionProtocolErrorV1::MissingSettlement)?,
            settlement_commit_digest: entry
                .settlement_commit_digest
                .ok_or(ExternalActionProtocolErrorV1::MissingSettlement)?,
        })
    }

    fn ensure_ready(&self) -> Result<(), ExternalActionProtocolErrorV1> {
        if self.ready {
            Ok(())
        } else {
            Err(ExternalActionProtocolErrorV1::CoordinatorRecoveryRequired)
        }
    }

    fn transaction_builder(
        &self,
        context: ExternalActionTransactionContextV1,
        expected_kind: WalTransactionKind,
    ) -> Result<WalTransactionBuilder, ExternalActionProtocolErrorV1> {
        self.ensure_ready()?;
        Ok(WalTransactionBuilder::new_external_action(
            context.writer_epoch,
            context.segment_id,
            context.transaction_id,
            expected_kind,
            self.next_lsn,
            self.previous_frame_digest,
            self.previous_commit_digest,
            context.durability_mode,
            context.payload_codec_id,
            context.payload_schema_id,
            context.payload_schema_version,
            context.canonical_encoding_version,
            context.digest_domain,
        ))
    }

    fn append_transaction(
        &mut self,
        store: &mut impl WalStorePort,
        transaction: WalCommittedTransaction,
    ) -> Result<Hash, ExternalActionProtocolErrorV1> {
        transaction.validate().map_err(WalBuildError::Validation)?;
        let capability = transaction
            .external_action_coordinator_capability()
            .ok_or(WalBuildError::ExternalActionCoordinatorCapabilityRequired)?;
        let epoch_id = transaction.commit.writer_epoch;
        let commit = transaction.commit;
        let last_lsn = commit.last_lsn;
        let last_frame_digest = transaction
            .frames
            .last()
            .map(crate::causal_wal::WalFrame::digest)
            .ok_or(WalBuildError::EmptyTransaction)?;
        self.ready = false;
        for frame in transaction.frames {
            store.append_frame(epoch_id, frame)?;
        }
        store.flush_external_action_commit(epoch_id, commit.clone(), capability)?;
        self.next_lsn = last_lsn.checked_next().ok_or(WalBuildError::LsnOverflow)?;
        self.previous_frame_digest = last_frame_digest;
        self.previous_commit_digest = commit.commit_digest;
        self.ready = true;
        Ok(commit.commit_digest)
    }
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

    /// Commits the complete authoritative external-action lifecycle index.
    #[must_use]
    pub fn root_digest(&self) -> Hash {
        self.merkle_nodes
            .get(&ExternalActionIndexNodeKeyV1 {
                depth: 0,
                prefix: [0; 32],
            })
            .copied()
            .unwrap_or_else(|| external_action_empty_hashes()[0])
    }

    fn plan_entry(&self, entry: RecoveredExternalActionV1) -> ExternalActionIndexMutationV1 {
        let request_hash = entry.request.request_id.as_hash();
        let mut child_hash = external_action_index_leaf(&entry);
        let mut node_updates = Vec::with_capacity(257);
        node_updates.push((
            ExternalActionIndexNodeKeyV1 {
                depth: 256,
                prefix: request_hash,
            },
            child_hash,
        ));
        for depth in (0_u16..256).rev() {
            let child_depth = depth + 1;
            let mut sibling_prefix = external_action_index_prefix(request_hash, child_depth);
            external_action_toggle_index_bit(&mut sibling_prefix, depth);
            let sibling_hash = self
                .merkle_nodes
                .get(&ExternalActionIndexNodeKeyV1 {
                    depth: child_depth,
                    prefix: sibling_prefix,
                })
                .copied()
                .unwrap_or_else(|| external_action_empty_hashes()[usize::from(child_depth)]);
            child_hash = if external_action_index_bit(request_hash, depth) {
                external_action_index_node_hash(depth, sibling_hash, child_hash)
            } else {
                external_action_index_node_hash(depth, child_hash, sibling_hash)
            };
            node_updates.push((
                ExternalActionIndexNodeKeyV1 {
                    depth,
                    prefix: external_action_index_prefix(request_hash, depth),
                },
                child_hash,
            ));
        }
        ExternalActionIndexMutationV1 {
            entry,
            node_updates,
            root_digest: child_hash,
        }
    }

    fn insert_entry(&mut self, entry: RecoveredExternalActionV1) -> bool {
        let request_id = entry.request.request_id;
        if self.entries.contains_key(&request_id) {
            return false;
        }
        let mutation = self.plan_entry(entry);
        self.apply_mutation(mutation);
        true
    }

    fn replace_entry(&mut self, entry: RecoveredExternalActionV1) {
        let request_id = entry.request.request_id;
        debug_assert!(self.entries.contains_key(&request_id));
        let mutation = self.plan_entry(entry);
        self.apply_mutation(mutation);
    }

    fn apply_mutation(&mut self, mutation: ExternalActionIndexMutationV1) {
        self.entries
            .insert(mutation.entry.request.request_id, mutation.entry);
        for (key, digest) in mutation.node_updates {
            self.merkle_nodes.insert(key, digest);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExternalActionIndexMutationV1 {
    entry: RecoveredExternalActionV1,
    node_updates: Vec<(ExternalActionIndexNodeKeyV1, Hash)>,
    root_digest: Hash,
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
    /// The adapter authorization named a different request, basis, or policy.
    #[error("external-action adapter authorization binding mismatch")]
    AuthorizationBindingMismatch,
    /// The adapter claim omitted lease or fencing evidence.
    #[error("external-action claim omitted lease evidence")]
    MissingLeaseEvidence,
    /// The adapter claim omitted runtime registry policy evidence.
    #[error("external-action claim omitted authorization policy evidence")]
    MissingAuthorizationPolicyEvidence,
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
    /// Deterministic resumption was requested before settlement admission.
    #[error("external-action request is missing its settlement")]
    MissingSettlement,
    /// A prior append failed after mutation may have begun; local recovery is required.
    #[error("external-action coordinator requires trusted local recovery")]
    CoordinatorRecoveryRequired,
    /// WAL recovery found an uncommitted tail; lifecycle admission must stop.
    #[error("external-action admission requires a clean committed WAL tail")]
    WalTailNotClean,
    /// Schema admission evidence was absent.
    #[error("external-action settlement omitted schema admission evidence")]
    MissingSchemaAdmissionEvidence,
    /// External observation or reconciliation evidence was absent.
    #[error("external-action settlement omitted external evidence")]
    MissingExternalEvidence,
    /// The WAL frontier did not commit the Echo-derived lifecycle index roots.
    #[error("external-action frontier mismatch")]
    ExternalActionFrontierMismatch {
        /// Frontier root required by the reconstructed lifecycle transition.
        expected: Hash,
        /// Frontier root retained by the WAL commit.
        actual: Hash,
    },
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

fn build_external_action_request_transaction(
    mut builder: WalTransactionBuilder,
    request: &ExternalActionRequestV1,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    request.validate_identity()?;
    builder.push_record(
        WalRecordKind::ExternalActionRequestRecorded,
        request.to_payload_bytes(),
    )?;
    Ok(builder.commit(affected_frontiers)?)
}

fn build_external_action_claim_transaction(
    mut builder: WalTransactionBuilder,
    claim: &ExternalActionClaimV1,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, ExternalActionProtocolErrorV1> {
    builder.push_record(
        WalRecordKind::ExternalActionClaimRecorded,
        claim.to_payload_bytes(),
    )?;
    Ok(builder.commit(affected_frontiers)?)
}

fn build_external_action_settlement_transaction(
    mut builder: WalTransactionBuilder,
    settlement: &ExternalActionSettlementV1,
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
    coordinator: &mut ExternalActionCoordinatorV1,
    context: ExternalActionTransactionContextV1,
    request: ExternalActionRequestV1,
) -> Result<DurablyRecordedExternalActionRequestV1, ExternalActionProtocolErrorV1> {
    coordinator.ensure_ready()?;
    if coordinator.index.get(request.request_id).is_some() {
        return Err(ExternalActionProtocolErrorV1::DuplicateRequest);
    }
    let next_entry = RecoveredExternalActionV1 {
        request,
        request_commit_digest: [0; 32],
        claim: None,
        claim_commit_digest: None,
        settlement: None,
        settlement_commit_digest: None,
        posture: RecoveredExternalActionPostureV1::Requested,
    };
    let mut mutation = coordinator.index.plan_entry(next_entry);
    let builder =
        coordinator.transaction_builder(context, WalTransactionKind::ExternalActionRequest)?;
    let transaction = build_external_action_request_transaction(
        builder,
        &request,
        external_action_index_frontier(coordinator.index.root_digest(), mutation.root_digest),
    )?;
    let request_commit_digest = coordinator.append_transaction(store, transaction)?;
    mutation.entry.request_commit_digest = request_commit_digest;
    coordinator.index.apply_mutation(mutation);
    Ok(DurablyRecordedExternalActionRequestV1 {
        request,
        request_commit_digest,
    })
}

/// Commits a bounded claim before returning adapter work authority.
#[allow(clippy::too_many_arguments)]
pub fn claim_external_action(
    store: &mut impl WalStorePort,
    coordinator: &mut ExternalActionCoordinatorV1,
    context: ExternalActionTransactionContextV1,
    recorded_request: DurablyRecordedExternalActionRequestV1,
    authorization: ExternalActionAdapterAuthorizationV1,
    current_basis_digest: Hash,
    attempt_ordinal: u32,
    lease_evidence_digest: Hash,
) -> Result<ExternalActionClaimGrantV1, ExternalActionProtocolErrorV1> {
    coordinator.ensure_ready()?;
    let request = recorded_request.request;
    request.validate_identity()?;
    let recovered = coordinator
        .index
        .get(request.request_id)
        .cloned()
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
    if authorization.request_id != request.request_id
        || authorization.basis_digest != request.basis_digest
        || authorization.registry_policy_digest == [0; 32]
    {
        return Err(ExternalActionProtocolErrorV1::AuthorizationBindingMismatch);
    }
    if current_basis_digest != request.basis_digest {
        return Err(ExternalActionProtocolErrorV1::StaleBasis);
    }
    if attempt_ordinal >= request.budget.max_attempts {
        return Err(ExternalActionProtocolErrorV1::AttemptBudgetExhausted);
    }
    if lease_evidence_digest == [0; 32] {
        return Err(ExternalActionProtocolErrorV1::MissingLeaseEvidence);
    }
    let claim = ExternalActionClaimV1::for_request(
        &request,
        authorization.adapter_id,
        attempt_ordinal,
        lease_evidence_digest,
        authorization.registry_policy_digest,
    );
    let mut next_entry = recovered;
    next_entry.claim = Some(claim);
    next_entry.claim_commit_digest = None;
    next_entry.posture = RecoveredExternalActionPostureV1::Claimed;
    let mut mutation = coordinator.index.plan_entry(next_entry);
    let builder =
        coordinator.transaction_builder(context, WalTransactionKind::ExternalActionClaim)?;
    let transaction = build_external_action_claim_transaction(
        builder,
        &claim,
        external_action_index_frontier(coordinator.index.root_digest(), mutation.root_digest),
    )?;
    let claim_commit_digest = coordinator.append_transaction(store, transaction)?;
    mutation.entry.claim_commit_digest = Some(claim_commit_digest);
    coordinator.index.apply_mutation(mutation);
    Ok(ExternalActionClaimGrantV1 {
        request,
        claim,
        claim_commit_digest,
    })
}

/// Validates and commits a settlement before returning a resumable fact.
pub fn admit_external_action_settlement(
    store: &mut impl WalStorePort,
    coordinator: &mut ExternalActionCoordinatorV1,
    context: ExternalActionTransactionContextV1,
    claim_grant: ExternalActionClaimGrantV1,
    candidate: ExternalActionSettlementCandidateV1,
) -> Result<AdmittedExternalActionSettlementV1, ExternalActionProtocolErrorV1> {
    coordinator.ensure_ready()?;
    let recovered = coordinator
        .index
        .get(claim_grant.request.request_id)
        .cloned()
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
    validate_settlement_candidate(&claim_grant.request, &claim_grant.claim, &candidate)?;
    let settlement = ExternalActionSettlementV1::from_candidate(candidate);
    let mut next_entry = recovered;
    next_entry.posture = RecoveredExternalActionPostureV1::Settled(settlement.kind);
    next_entry.settlement = Some(settlement.clone());
    next_entry.settlement_commit_digest = None;
    let mut mutation = coordinator.index.plan_entry(next_entry);
    let builder =
        coordinator.transaction_builder(context, WalTransactionKind::ExternalActionSettlement)?;
    let transaction = build_external_action_settlement_transaction(
        builder,
        &settlement,
        external_action_index_frontier(coordinator.index.root_digest(), mutation.root_digest),
    )?;
    let settlement_commit_digest = coordinator.append_transaction(store, transaction)?;
    mutation.entry.settlement_commit_digest = Some(settlement_commit_digest);
    coordinator.index.apply_mutation(mutation);
    Ok(AdmittedExternalActionSettlementV1 {
        settlement,
        settlement_commit_digest,
    })
}

/// Reconciles one retained adapter settlement after acknowledgement loss.
///
/// This path exposes no WAL store, transition context, or claim grant. It can
/// therefore return only the exact settlement that is already durable. A
/// different valid candidate is a conflict; a malformed candidate fails the
/// ordinary request-and-claim validation before comparison.
pub fn reconcile_external_action_settlement_retry(
    coordinator: &ExternalActionCoordinatorV1,
    candidate: ExternalActionSettlementCandidateV1,
) -> Result<AdmittedExternalActionSettlementV1, ExternalActionProtocolErrorV1> {
    coordinator.ensure_ready()?;
    let recovered = coordinator
        .index
        .get(candidate.request_id)
        .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
    let claim = recovered
        .claim
        .ok_or(ExternalActionProtocolErrorV1::MissingClaim)?;
    validate_settlement_candidate(&recovered.request, &claim, &candidate)?;
    let candidate = ExternalActionSettlementV1::from_candidate(candidate);
    let settlement = recovered
        .settlement
        .as_ref()
        .ok_or(ExternalActionProtocolErrorV1::MissingSettlement)?;
    let settlement_commit_digest = recovered
        .settlement_commit_digest
        .ok_or(ExternalActionProtocolErrorV1::MissingSettlement)?;
    if settlement != &candidate {
        return Err(ExternalActionProtocolErrorV1::ConflictingSettlement);
    }
    Ok(AdmittedExternalActionSettlementV1 {
        settlement: settlement.clone(),
        settlement_commit_digest,
    })
}

/// Observes request, claim, and settlement posture in an arbitrary recovery report.
///
/// This projection carries no transition or replay authority. Use
/// [`ExternalActionCoordinatorV1::recover`] to reconstruct trusted local
/// transition grants and resumable settlements.
pub fn observe_external_actions(
    report: &RecoveryScanReport,
) -> Result<RecoveredExternalActionIndexV1, ExternalActionProtocolErrorV1> {
    let mut index = RecoveredExternalActionIndexV1::default();
    for transaction in &report.transactions {
        let Some(frame) =
            external_action_frame(transaction.commit.transaction_kind, &transaction.frames)?
        else {
            continue;
        };
        let before_root = index.root_digest();
        match frame.header.record_kind {
            WalRecordKind::ExternalActionRequestRecorded => {
                let request =
                    ExternalActionRequestV1::from_payload_bytes(&frame.payload.canonical_bytes)?;
                if !index.insert_entry(RecoveredExternalActionV1 {
                    request,
                    request_commit_digest: transaction.commit.commit_digest,
                    claim: None,
                    claim_commit_digest: None,
                    settlement: None,
                    settlement_commit_digest: None,
                    posture: RecoveredExternalActionPostureV1::Requested,
                }) {
                    return Err(ExternalActionProtocolErrorV1::DuplicateRequest);
                }
            }
            WalRecordKind::ExternalActionClaimRecorded => {
                let claim =
                    ExternalActionClaimV1::from_payload_bytes(&frame.payload.canonical_bytes)?;
                let mut entry = index
                    .get(claim.request_id)
                    .cloned()
                    .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
                if entry.claim.is_some() {
                    return Err(ExternalActionProtocolErrorV1::DuplicateClaim);
                }
                validate_claim(&entry.request, &claim)?;
                entry.claim = Some(claim);
                entry.claim_commit_digest = Some(transaction.commit.commit_digest);
                entry.posture = RecoveredExternalActionPostureV1::Claimed;
                index.replace_entry(entry);
            }
            WalRecordKind::ExternalActionSettlementRecorded => {
                let settlement =
                    ExternalActionSettlementV1::from_payload_bytes(&frame.payload.canonical_bytes)?;
                apply_recovered_settlement(
                    &mut index,
                    settlement,
                    transaction.commit.commit_digest,
                )?;
            }
            _ => unreachable!("external_action_frame filters record kinds"),
        }
        let after_root = index.root_digest();
        let expected_frontier_root = affected_frontiers_root(&[AffectedFrontier {
            kind: AffectedFrontierKind::ExternalActionIndex,
            before_digest: before_root,
            after_digest: after_root,
        }]);
        if transaction.commit.affected_frontiers_root != expected_frontier_root {
            return Err(
                ExternalActionProtocolErrorV1::ExternalActionFrontierMismatch {
                    expected: expected_frontier_root,
                    actual: transaction.commit.affected_frontiers_root,
                },
            );
        }
    }
    Ok(index)
}

fn apply_recovered_settlement(
    index: &mut RecoveredExternalActionIndexV1,
    settlement: ExternalActionSettlementV1,
    commit_digest: Hash,
) -> Result<(), ExternalActionProtocolErrorV1> {
    let mut entry = index
        .get(settlement.request_id)
        .cloned()
        .ok_or(ExternalActionProtocolErrorV1::MissingRequest)?;
    let claim = entry
        .claim
        .ok_or(ExternalActionProtocolErrorV1::MissingClaim)?;
    validate_settlement(&entry.request, &claim, &settlement)?;
    if let Some(existing) = &entry.settlement {
        let existing_commit = entry
            .settlement_commit_digest
            .ok_or(ExternalActionProtocolErrorV1::MissingSettlement)?;
        return if existing == &settlement && existing_commit == commit_digest {
            Err(ExternalActionProtocolErrorV1::DuplicateSettlement)
        } else {
            Err(ExternalActionProtocolErrorV1::ConflictingSettlement)
        };
    }
    entry.posture = RecoveredExternalActionPostureV1::Settled(settlement.kind);
    entry.settlement = Some(settlement);
    entry.settlement_commit_digest = Some(commit_digest);
    index.replace_entry(entry);
    Ok(())
}

fn external_action_index_frontier(
    before_digest: Hash,
    after_digest: Hash,
) -> Vec<AffectedFrontier> {
    vec![AffectedFrontier {
        kind: AffectedFrontierKind::ExternalActionIndex,
        before_digest,
        after_digest,
    }]
}

fn hash_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
}

fn external_action_index_leaf(entry: &RecoveredExternalActionV1) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(INDEX_LEAF_DOMAIN);
    hasher.update(&entry.request.request_id.as_hash());
    hash_len_prefixed(&mut hasher, &entry.request.to_payload_bytes());
    match entry.claim {
        Some(claim) => {
            hasher.update(&[1]);
            hash_len_prefixed(&mut hasher, &claim.to_payload_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    match &entry.settlement {
        Some(settlement) => {
            hasher.update(&[1]);
            hash_len_prefixed(&mut hasher, &settlement.to_payload_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.finalize().into()
}

fn external_action_index_node_hash(depth: u16, left: Hash, right: Hash) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(INDEX_NODE_DOMAIN);
    hasher.update(&depth.to_le_bytes());
    hasher.update(&left);
    hasher.update(&right);
    hasher.finalize().into()
}

fn external_action_empty_hashes() -> &'static [Hash; 257] {
    static EMPTY_HASHES: OnceLock<[Hash; 257]> = OnceLock::new();
    EMPTY_HASHES.get_or_init(|| {
        let mut hashes = [[0; 32]; 257];
        hashes[256] = blake3::hash(INDEX_EMPTY_LEAF_DOMAIN).into();
        for depth in (0_u16..256).rev() {
            let child = hashes[usize::from(depth + 1)];
            hashes[usize::from(depth)] = external_action_index_node_hash(depth, child, child);
        }
        hashes
    })
}

fn external_action_index_prefix(mut request_id: Hash, depth: u16) -> Hash {
    if depth == 256 {
        return request_id;
    }
    let byte_index = usize::from(depth / 8);
    let retained_bits = depth % 8;
    if retained_bits == 0 {
        request_id[byte_index..].fill(0);
    } else {
        request_id[byte_index] &= u8::MAX << (8 - retained_bits);
        request_id[(byte_index + 1)..].fill(0);
    }
    request_id
}

fn external_action_index_bit(request_id: Hash, depth: u16) -> bool {
    let byte_index = usize::from(depth / 8);
    let bit_index = 7 - (depth % 8);
    request_id[byte_index] & (1_u8 << bit_index) != 0
}

fn external_action_toggle_index_bit(prefix: &mut Hash, depth: u16) {
    let byte_index = usize::from(depth / 8);
    let bit_index = 7 - (depth % 8);
    prefix[byte_index] ^= 1_u8 << bit_index;
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
    authorization_policy_digest: Hash,
) -> ExternalActionAttemptIdV1 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(ATTEMPT_ID_DOMAIN);
    hasher.update(&request_id.as_hash());
    hasher.update(&attempt_ordinal.to_le_bytes());
    hasher.update(&adapter_id.as_hash());
    hasher.update(&lease_evidence_digest);
    hasher.update(&authorization_policy_digest);
    ExternalActionAttemptIdV1::from_hash(hasher.finalize().into())
}

fn validate_claim(
    request: &ExternalActionRequestV1,
    claim: &ExternalActionClaimV1,
) -> Result<(), ExternalActionProtocolErrorV1> {
    let expected = ExternalActionClaimV1::for_request(
        request,
        claim.adapter_id,
        claim.attempt_ordinal,
        claim.lease_evidence_digest,
        claim.authorization_policy_digest,
    );
    if *claim != expected {
        return Err(ExternalActionProtocolErrorV1::ClaimBindingMismatch);
    }
    if claim.attempt_ordinal >= request.budget.max_attempts {
        return Err(ExternalActionProtocolErrorV1::AttemptBudgetExhausted);
    }
    if claim.lease_evidence_digest == [0; 32] {
        return Err(ExternalActionProtocolErrorV1::MissingLeaseEvidence);
    }
    if claim.authorization_policy_digest == [0; 32] {
        return Err(ExternalActionProtocolErrorV1::MissingAuthorizationPolicyEvidence);
    }
    Ok(())
}

fn validate_settlement_candidate(
    request: &ExternalActionRequestV1,
    claim: &ExternalActionClaimV1,
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
    if candidate.external_evidence_digest == [0; 32] {
        return Err(ExternalActionProtocolErrorV1::MissingExternalEvidence);
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
    claim: &ExternalActionClaimV1,
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

fn external_action_wal_continuation(
    report: &RecoveryScanReport,
) -> Result<(Lsn, Hash, Hash), ExternalActionProtocolErrorV1> {
    let Some(last_transaction) = report.transactions.last() else {
        return Ok((Lsn::from_raw(0), [0; 32], [0; 32]));
    };
    let next_lsn = last_transaction
        .commit
        .last_lsn
        .checked_next()
        .ok_or(WalBuildError::LsnOverflow)?;
    let previous_frame_digest = last_transaction
        .frames
        .last()
        .map(crate::causal_wal::WalFrame::digest)
        .ok_or(WalBuildError::EmptyTransaction)?;
    Ok((
        next_lsn,
        previous_frame_digest,
        last_transaction.commit.commit_digest,
    ))
}

fn external_action_frame(
    transaction_kind: WalTransactionKind,
    frames: &[crate::causal_wal::WalFrame],
) -> Result<Option<&crate::causal_wal::WalFrame>, ExternalActionProtocolErrorV1> {
    let expected = transaction_kind.external_action_record_kind();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(label: &str) -> Hash {
        blake3::hash(label.as_bytes()).into()
    }

    fn request() -> ExternalActionRequestV1 {
        match ExternalActionRequestV1::new(
            WorldlineId::from_bytes([41; 32]),
            ExternalActionOperationIdV1::from_hash(digest("test.operation@1")),
            digest("test.operation@1.input"),
            digest("test.operation@1.settlement"),
            digest("test.scope"),
            digest("test.basis"),
            ExternalActionBudgetV1 {
                max_settlement_bytes: 64,
                max_attempts: 1,
            },
            digest("test.input"),
            digest("test.reconciliation"),
        ) {
            Ok(request) => request,
            Err(error) => panic!("request fixture failed: {error:?}"),
        }
    }

    fn claimed_index() -> (
        ExternalActionRequestV1,
        ExternalActionClaimV1,
        RecoveredExternalActionIndexV1,
    ) {
        let request = request();
        let claim = ExternalActionClaimV1::for_request(
            &request,
            ExternalActionAdapterIdV1::from_hash(digest("test.adapter")),
            0,
            digest("test.lease"),
            digest("test.policy"),
        );
        let mut index = RecoveredExternalActionIndexV1::default();
        assert!(index.insert_entry(RecoveredExternalActionV1 {
            request,
            request_commit_digest: digest("request.commit"),
            claim: Some(claim),
            claim_commit_digest: Some(digest("claim.commit")),
            settlement: None,
            settlement_commit_digest: None,
            posture: RecoveredExternalActionPostureV1::Claimed,
        }));
        (request, claim, index)
    }

    fn settlement(
        request: &ExternalActionRequestV1,
        claim: &ExternalActionClaimV1,
        bytes: &[u8],
    ) -> ExternalActionSettlementV1 {
        ExternalActionSettlementV1::from_candidate(ExternalActionSettlementCandidateV1::new(
            request.request_id,
            claim.attempt_id,
            claim.adapter_id,
            ExternalActionSettlementKindV1::Succeeded,
            request.settlement_schema_digest,
            request.basis_digest,
            bytes.to_vec(),
            digest("test.schema-evidence"),
            digest("test.external-evidence"),
        ))
    }

    #[test]
    fn identical_recovered_settlement_is_a_duplicate() {
        let (request, claim, mut index) = claimed_index();
        let settlement = settlement(&request, &claim, b"same");
        let commit = digest("settlement.commit");

        assert_eq!(
            apply_recovered_settlement(&mut index, settlement.clone(), commit),
            Ok(())
        );
        let root = index.root_digest();
        assert_eq!(
            apply_recovered_settlement(&mut index, settlement, commit),
            Err(ExternalActionProtocolErrorV1::DuplicateSettlement)
        );
        assert_eq!(index.len(), 1);
        assert_eq!(index.root_digest(), root);
    }

    #[test]
    fn conflicting_recovered_settlement_is_obstructed() {
        let (request, claim, mut index) = claimed_index();
        let first = settlement(&request, &claim, b"first");
        assert_eq!(
            apply_recovered_settlement(&mut index, first, digest("settlement.commit")),
            Ok(())
        );
        let conflicting = settlement(&request, &claim, b"second");
        assert_eq!(
            apply_recovered_settlement(&mut index, conflicting, digest("conflict.commit")),
            Err(ExternalActionProtocolErrorV1::ConflictingSettlement)
        );
    }

    #[test]
    fn identical_settlement_under_another_commit_is_conflicting() {
        let (request, claim, mut index) = claimed_index();
        let settlement = settlement(&request, &claim, b"same");
        assert_eq!(
            apply_recovered_settlement(&mut index, settlement.clone(), digest("settlement.commit")),
            Ok(())
        );
        assert_eq!(
            apply_recovered_settlement(
                &mut index,
                settlement,
                digest("different-settlement.commit")
            ),
            Err(ExternalActionProtocolErrorV1::ConflictingSettlement)
        );
    }

    #[test]
    fn fixed_seed_settlement_mutations_are_conflicting() {
        const SEED: u64 = 0x5e77_1e5e_77e5_0001;
        let (request, claim, mut index) = claimed_index();
        let first = settlement(&request, &claim, &SEED.to_le_bytes());
        assert_eq!(
            apply_recovered_settlement(&mut index, first, digest("property-settlement.commit")),
            Ok(())
        );

        let mut state = SEED;
        for ordinal in 0_u8..32 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let mut bytes = state.to_le_bytes().to_vec();
            bytes.push(ordinal);
            let conflicting = settlement(&request, &claim, &bytes);
            assert_eq!(
                apply_recovered_settlement(
                    &mut index,
                    conflicting,
                    digest("property-conflict.commit")
                ),
                Err(ExternalActionProtocolErrorV1::ConflictingSettlement)
            );
        }
    }
}
