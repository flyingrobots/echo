// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Durable external-action request and settlement protocol tests.

#![allow(clippy::panic)]

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use warp_core::causal_wal::{
    recover_filesystem_store, recover_from_frames_and_commits, recover_in_memory_store,
    AffectedFrontierKind, ExternalActionCoordinatorCapability, FilesystemWalStore,
    InMemoryWalStore, Lsn, PayloadCodecId, PayloadSchemaId, RecoveryAccessMode,
    RecoveryTailPosture, WalAppendAuthority, WalBuildError, WalDurabilityMode, WalFrame,
    WalManifest, WalRecordKind, WalSegmentId, WalSegmentSeal, WalStoreError, WalStorePort,
    WalStoreSnapshot, WalTransactionBuilder, WalTransactionCommit, WalTransactionId,
    WalTransactionKind, WriterEpoch, WriterEpochId, WriterEpochRequest,
};
use warp_core::external_action::{
    admit_external_action_settlement, claim_external_action, observe_external_actions,
    record_external_action_request, ExternalActionAdapterAuthorizationV1,
    ExternalActionAdapterBindingV1, ExternalActionAdapterIdV1, ExternalActionAdapterRegistryV1,
    ExternalActionBudgetV1, ExternalActionClaimGrantV1, ExternalActionCoordinatorV1,
    ExternalActionOperationIdV1, ExternalActionProtocolErrorV1, ExternalActionRequestV1,
    ExternalActionSettlementCandidateV1, ExternalActionSettlementKindV1,
    ExternalActionTransactionContextV1, RecoveredExternalActionPostureV1,
};
use warp_core::{Hash, WorldlineId};

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn must_ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok(..), got {error:?}"),
    }
}

fn epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(digest("external-action:epoch"))
}

fn store() -> InMemoryWalStore {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(WriterEpochRequest {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("external-action:fencing"),
        process_identity: digest("external-action:process"),
        host_identity: digest("external-action:host"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("external-action:lease"),
    }));
    store
}

fn coordinator(store: &impl WalStorePort) -> ExternalActionCoordinatorV1 {
    must_ok(ExternalActionCoordinatorV1::recover(store))
}

fn raw_builder(label: &str, first_lsn: u64, kind: WalTransactionKind) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        epoch_id(),
        WalSegmentId::from_raw(1),
        WalTransactionId::from_hash(digest(label)),
        kind,
        WalAppendAuthority::ExternalActionCoordinator,
        Lsn::from_raw(first_lsn),
        [0; 32],
        [0; 32],
        WalDurabilityMode::Buffered,
        PayloadCodecId::from_hash(digest("external-action:codec")),
        PayloadSchemaId::from_hash(digest("external-action:schema")),
        1,
        1,
        digest("external-action:domain"),
    )
}

fn context(label: &str) -> ExternalActionTransactionContextV1 {
    context_with_durability(label, WalDurabilityMode::Buffered)
}

fn context_with_durability(
    label: &str,
    durability_mode: WalDurabilityMode,
) -> ExternalActionTransactionContextV1 {
    ExternalActionTransactionContextV1 {
        writer_epoch: epoch_id(),
        segment_id: WalSegmentId::from_raw(1),
        transaction_id: WalTransactionId::from_hash(digest(label)),
        durability_mode,
        payload_codec_id: PayloadCodecId::from_hash(digest("external-action:codec")),
        payload_schema_id: PayloadSchemaId::from_hash(digest("external-action:schema")),
        payload_schema_version: 1,
        canonical_encoding_version: 1,
        digest_domain: digest("external-action:domain"),
    }
}

static TEMP_WAL_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempWalDir(PathBuf);

impl TempWalDir {
    fn new(label: &str) -> Self {
        let counter = TEMP_WAL_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "echo-external-action-{}-{counter}-{label}",
            std::process::id()
        ));
        if path.exists() {
            must_ok(std::fs::remove_dir_all(&path));
        }
        must_ok(std::fs::create_dir_all(&path));
        Self(path)
    }
}

impl Drop for TempWalDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn request_with(
    label: &str,
    worldline_byte: u8,
    max_settlement_bytes: u64,
) -> ExternalActionRequestV1 {
    must_ok(ExternalActionRequestV1::new(
        WorldlineId::from_bytes([worldline_byte; 32]),
        ExternalActionOperationIdV1::from_hash(digest("workspace.observe@1")),
        digest("workspace.observe@1.input"),
        digest("workspace.observe@1.settlement"),
        digest("workspace:/bounded"),
        digest(&format!("basis:{label}")),
        ExternalActionBudgetV1 {
            max_settlement_bytes,
            max_attempts: 1,
        },
        digest(&format!("input:{label}")),
        digest("workspace.observe@1.reconcile"),
    ))
}

fn adapter_id() -> ExternalActionAdapterIdV1 {
    ExternalActionAdapterIdV1::from_hash(digest("adapter:workspace-observer"))
}

fn adapter_registry() -> ExternalActionAdapterRegistryV1 {
    ExternalActionAdapterRegistryV1::new([ExternalActionAdapterBindingV1 {
        adapter_id: adapter_id(),
        operation_id: ExternalActionOperationIdV1::from_hash(digest("workspace.observe@1")),
        authority_scope_digest: digest("workspace:/bounded"),
    }])
}

fn authorization(request: &ExternalActionRequestV1) -> ExternalActionAdapterAuthorizationV1 {
    must_ok(adapter_registry().authorize(request, adapter_id()))
}

#[allow(clippy::large_types_passed_by_value)]
fn record(
    store: &mut impl WalStorePort,
    coordinator: &mut ExternalActionCoordinatorV1,
    request: ExternalActionRequestV1,
    label: &str,
) -> warp_core::external_action::DurablyRecordedExternalActionRequestV1 {
    must_ok(record_external_action_request(
        store,
        coordinator,
        context(label),
        request,
    ))
}

#[allow(clippy::large_types_passed_by_value)]
fn claim(
    store: &mut impl WalStorePort,
    coordinator: &mut ExternalActionCoordinatorV1,
    recorded: warp_core::external_action::DurablyRecordedExternalActionRequestV1,
    label: &str,
) -> ExternalActionClaimGrantV1 {
    let basis = recorded.request().basis_digest;
    let authorization = authorization(&recorded.request());
    must_ok(claim_external_action(
        store,
        coordinator,
        context(label),
        recorded,
        authorization,
        basis,
        0,
        digest(&format!("{label}:lease")),
    ))
}

fn candidate(
    grant: &ExternalActionClaimGrantV1,
    kind: ExternalActionSettlementKindV1,
    bytes: Vec<u8>,
) -> ExternalActionSettlementCandidateV1 {
    let request = grant.request();
    let claim = grant.claim();
    ExternalActionSettlementCandidateV1::new(
        request.request_id(),
        claim.attempt_id,
        claim.adapter_id,
        kind,
        request.settlement_schema_digest,
        request.basis_digest,
        bytes,
        digest("settlement:schema-admission"),
        digest("settlement:external-evidence"),
    )
}

#[test]
fn request_and_settlement_are_committed_before_authority_crosses_the_boundary() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("golden", 7, 128);
    let recorded = record(&mut store, &mut coordinator, request, "request:golden");
    assert_eq!(store.read_commits().len(), 1);
    assert_eq!(
        store.read_commits()[0].commit_digest,
        recorded.request_commit_digest()
    );

    let grant = claim(&mut store, &mut coordinator, recorded, "claim:golden");
    assert_eq!(store.read_commits().len(), 2);
    assert_eq!(
        store.read_commits()[1].commit_digest,
        grant.claim_commit_digest()
    );
    let result_bytes = b"observed workspace bytes".to_vec();
    let candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        result_bytes.clone(),
    );
    let admitted = must_ok(admit_external_action_settlement(
        &mut store,
        &mut coordinator,
        context("settlement:golden"),
        grant,
        candidate,
    ));
    assert_eq!(store.read_commits().len(), 3);
    assert_eq!(
        store.read_commits()[2].commit_digest,
        admitted.settlement_commit_digest()
    );
    assert_eq!(admitted.settlement().canonical_result_bytes, result_bytes);
}

#[test]
fn unauthorized_adapter_and_stale_basis_obstruct_before_claim_commit() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("claim-obstructions", 8, 64);
    let recorded = record(
        &mut store,
        &mut coordinator,
        request,
        "request:claim-obstructions",
    );
    let commits_before = store.read_commits().len();
    assert_eq!(
        adapter_registry().authorize(
            &request,
            ExternalActionAdapterIdV1::from_hash(digest("adapter:unauthorized")),
        ),
        Err(ExternalActionProtocolErrorV1::UnauthorizedAdapter)
    );
    assert_eq!(store.read_commits().len(), commits_before);

    assert_eq!(
        claim_external_action(
            &mut store,
            &mut coordinator,
            context("claim:stale"),
            recorded,
            authorization(&request),
            digest("basis:changed"),
            0,
            digest("claim:stale:lease"),
        ),
        Err(ExternalActionProtocolErrorV1::StaleBasis)
    );
    assert_eq!(store.read_commits().len(), commits_before);
}

#[test]
fn adapter_authorization_is_bound_to_the_exact_request() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let authorized_request = request_with("authorization-source", 8, 64);
    let claimed_request = request_with("authorization-target", 8, 64);
    let recorded = record(
        &mut store,
        &mut coordinator,
        claimed_request,
        "request:authorization-target",
    );
    let commits_before = store.read_commits().len();

    assert_eq!(
        claim_external_action(
            &mut store,
            &mut coordinator,
            context("claim:authorization-target"),
            recorded,
            authorization(&authorized_request),
            claimed_request.basis_digest,
            0,
            digest("claim:authorization-target:lease"),
        ),
        Err(ExternalActionProtocolErrorV1::AuthorizationBindingMismatch)
    );
    assert_eq!(store.read_commits().len(), commits_before);
}

#[test]
fn claims_and_settlements_require_nonzero_external_evidence() {
    let mut claim_store = store();
    let mut claim_coordinator = coordinator(&claim_store);
    let claim_request = request_with("missing-lease-evidence", 8, 64);
    let claim_recorded = record(
        &mut claim_store,
        &mut claim_coordinator,
        claim_request,
        "request:missing-lease-evidence",
    );
    let claim_commits_before = claim_store.read_commits().len();
    assert_eq!(
        claim_external_action(
            &mut claim_store,
            &mut claim_coordinator,
            context("claim:missing-lease-evidence"),
            claim_recorded,
            authorization(&claim_request),
            claim_request.basis_digest,
            0,
            [0; 32],
        ),
        Err(ExternalActionProtocolErrorV1::MissingLeaseEvidence)
    );
    assert_eq!(claim_store.read_commits().len(), claim_commits_before);

    let mut settlement_store = store();
    let mut settlement_coordinator = coordinator(&settlement_store);
    let settlement_request = request_with("missing-external-evidence", 8, 64);
    let settlement_recorded = record(
        &mut settlement_store,
        &mut settlement_coordinator,
        settlement_request,
        "request:missing-external-evidence",
    );
    let grant = claim(
        &mut settlement_store,
        &mut settlement_coordinator,
        settlement_recorded,
        "claim:missing-external-evidence",
    );
    let mut candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"observed".to_vec(),
    );
    candidate.external_evidence_digest = [0; 32];
    let settlement_commits_before = settlement_store.read_commits().len();
    assert_eq!(
        admit_external_action_settlement(
            &mut settlement_store,
            &mut settlement_coordinator,
            context("settlement:missing-external-evidence"),
            grant,
            candidate,
        ),
        Err(ExternalActionProtocolErrorV1::MissingExternalEvidence)
    );
    assert_eq!(
        settlement_store.read_commits().len(),
        settlement_commits_before
    );
}

#[test]
fn recovery_rejects_forged_external_action_frontier_evidence() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("forged-frontier", 8, 64);
    record(
        &mut store,
        &mut coordinator,
        request,
        "request:forged-frontier",
    );
    let mut report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    match report.transactions.first_mut() {
        Some(transaction) => {
            transaction.commit.affected_frontiers_root = digest("forged-frontier-root");
        }
        None => panic!("request transaction was not recovered"),
    }
    assert!(matches!(
        observe_external_actions(&report),
        Err(ExternalActionProtocolErrorV1::ExternalActionFrontierMismatch { .. })
    ));
}

#[test]
fn malformed_schema_digest_and_oversized_settlements_fail_closed() {
    for (label, mutation, expected) in [
        (
            "schema",
            1_u8,
            ExternalActionProtocolErrorV1::SettlementSchemaMismatch,
        ),
        (
            "digest",
            2_u8,
            ExternalActionProtocolErrorV1::SettlementResultDigestMismatch,
        ),
        (
            "budget",
            3_u8,
            ExternalActionProtocolErrorV1::SettlementBudgetExceeded,
        ),
        (
            "schema-evidence",
            4_u8,
            ExternalActionProtocolErrorV1::MissingSchemaAdmissionEvidence,
        ),
    ] {
        let mut store = store();
        let mut coordinator = coordinator(&store);
        let request = request_with(label, 9, 4);
        let recorded = record(
            &mut store,
            &mut coordinator,
            request,
            &format!("request:{label}"),
        );
        let grant = claim(
            &mut store,
            &mut coordinator,
            recorded,
            &format!("claim:{label}"),
        );
        let mut candidate = candidate(
            &grant,
            ExternalActionSettlementKindV1::Succeeded,
            b"four".to_vec(),
        );
        match mutation {
            1 => candidate.settlement_schema_digest = digest("wrong-schema"),
            2 => candidate.declared_result_digest = digest("wrong-result"),
            3 => candidate.canonical_result_bytes.push(b'!'),
            4 => candidate.schema_admission_evidence_digest = [0; 32],
            _ => unreachable!(),
        }
        let commits_before = store.read_commits().len();
        assert_eq!(
            admit_external_action_settlement(
                &mut store,
                &mut coordinator,
                context(&format!("settlement:{label}")),
                grant,
                candidate,
            ),
            Err(expected)
        );
        assert_eq!(store.read_commits().len(), commits_before);
    }
}

#[test]
fn request_and_attempt_budget_boundaries_obstruct_before_commit() {
    assert_eq!(
        ExternalActionRequestV1::new(
            WorldlineId::from_bytes([19; 32]),
            ExternalActionOperationIdV1::from_hash(digest("workspace.observe@1")),
            digest("workspace.observe@1.input"),
            digest("workspace.observe@1.settlement"),
            digest("workspace:/bounded"),
            digest("basis:empty-budget"),
            ExternalActionBudgetV1 {
                max_settlement_bytes: 0,
                max_attempts: 1,
            },
            digest("input:empty-budget"),
            digest("workspace.observe@1.reconcile"),
        ),
        Err(ExternalActionProtocolErrorV1::EmptyBudget)
    );
    assert_eq!(
        ExternalActionRequestV1::new(
            WorldlineId::from_bytes([19; 32]),
            ExternalActionOperationIdV1::from_hash(digest("workspace.observe@1")),
            digest("workspace.observe@1.input"),
            digest("workspace.observe@1.settlement"),
            digest("workspace:/bounded"),
            digest("basis:oversized-budget"),
            ExternalActionBudgetV1 {
                max_settlement_bytes:
                    warp_core::external_action::MAX_EXTERNAL_ACTION_SETTLEMENT_BYTES_V1 + 1,
                max_attempts: 1,
            },
            digest("input:oversized-budget"),
            digest("workspace.observe@1.reconcile"),
        ),
        Err(ExternalActionProtocolErrorV1::RequestBudgetLimitExceeded)
    );
    assert_eq!(
        ExternalActionRequestV1::new(
            WorldlineId::from_bytes([19; 32]),
            ExternalActionOperationIdV1::from_hash(digest("workspace.observe@1")),
            digest("workspace.observe@1.input"),
            digest("workspace.observe@1.settlement"),
            digest("workspace:/bounded"),
            digest("basis:multi-attempt-budget"),
            ExternalActionBudgetV1 {
                max_settlement_bytes: 64,
                max_attempts: 2,
            },
            digest("input:multi-attempt-budget"),
            digest("workspace.observe@1.reconcile"),
        ),
        Err(ExternalActionProtocolErrorV1::UnsupportedAttemptBudget)
    );

    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("attempt-budget", 19, 64);
    let recorded = record(
        &mut store,
        &mut coordinator,
        request,
        "request:attempt-budget",
    );
    let commits_before = store.read_commits().len();
    assert_eq!(
        claim_external_action(
            &mut store,
            &mut coordinator,
            context("claim:attempt-budget"),
            recorded,
            authorization(&request),
            request.basis_digest,
            1,
            digest("claim:attempt-budget:lease"),
        ),
        Err(ExternalActionProtocolErrorV1::AttemptBudgetExhausted)
    );
    assert_eq!(store.read_commits().len(), commits_before);
}

#[test]
fn second_claim_for_one_request_is_obstructed_without_commit() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("duplicate-claim", 19, 64);
    let recorded = record(
        &mut store,
        &mut coordinator,
        request,
        "request:duplicate-claim",
    );
    let duplicate_token = must_ok(coordinator.recorded_request(request.request_id()));
    let _grant = claim(
        &mut store,
        &mut coordinator,
        recorded,
        "claim:duplicate-claim:first",
    );
    let commits_before = store.commit_count();
    assert_eq!(
        claim_external_action(
            &mut store,
            &mut coordinator,
            context("claim:duplicate-claim:second"),
            duplicate_token,
            authorization(&request),
            request.basis_digest,
            0,
            digest("claim:duplicate-claim:second:lease"),
        ),
        Err(ExternalActionProtocolErrorV1::DuplicateClaim)
    );
    assert_eq!(store.commit_count(), commits_before);
}

#[test]
fn recovery_distinguishes_unclaimed_claimed_settled_and_ambiguous_requests() {
    let mut store = store();
    let mut coordinator = coordinator(&store);

    let requested = request_with("requested", 10, 64);
    record(&mut store, &mut coordinator, requested, "request:requested");

    let claimed = request_with("claimed", 10, 64);
    let claimed_recorded = record(&mut store, &mut coordinator, claimed, "request:claimed");
    claim(
        &mut store,
        &mut coordinator,
        claimed_recorded,
        "claim:claimed",
    );

    let settled = request_with("settled", 10, 64);
    let settled_recorded = record(&mut store, &mut coordinator, settled, "request:settled");
    let settled_grant = claim(
        &mut store,
        &mut coordinator,
        settled_recorded,
        "claim:settled",
    );
    let settled_candidate = candidate(
        &settled_grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"settled".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        &mut coordinator,
        context("settlement:settled"),
        settled_grant,
        settled_candidate,
    ));

    let ambiguous = request_with("ambiguous", 10, 64);
    let ambiguous_recorded = record(&mut store, &mut coordinator, ambiguous, "request:ambiguous");
    let ambiguous_grant = claim(
        &mut store,
        &mut coordinator,
        ambiguous_recorded,
        "claim:ambiguous",
    );
    let ambiguous_candidate = candidate(
        &ambiguous_grant,
        ExternalActionSettlementKindV1::OutcomeUnknown,
        b"connection-lost".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        &mut coordinator,
        context("settlement:ambiguous"),
        ambiguous_grant,
        ambiguous_candidate,
    ));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(observe_external_actions(&report));
    assert_eq!(index.len(), 4);
    assert_eq!(
        index.get(requested.request_id()).map(|entry| entry.posture),
        Some(RecoveredExternalActionPostureV1::Requested)
    );
    assert_eq!(
        index.get(claimed.request_id()).map(|entry| entry.posture),
        Some(RecoveredExternalActionPostureV1::Claimed)
    );
    assert_eq!(
        index.get(settled.request_id()).map(|entry| entry.posture),
        Some(RecoveredExternalActionPostureV1::Settled(
            ExternalActionSettlementKindV1::Succeeded
        ))
    );
    assert_eq!(
        index.get(ambiguous.request_id()).map(|entry| entry.posture),
        Some(RecoveredExternalActionPostureV1::Settled(
            ExternalActionSettlementKindV1::OutcomeUnknown
        ))
    );
}

#[test]
fn replay_returns_admitted_bytes_without_reissuing_an_effect() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("replay", 11, 64);
    let recorded = record(&mut store, &mut coordinator, request, "request:replay");
    let grant = claim(&mut store, &mut coordinator, recorded, "claim:replay");
    let candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"recorded-result".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        &mut coordinator,
        context("settlement:replay"),
        grant,
        candidate,
    ));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let recovered_coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let admitted = must_ok(recovered_coordinator.admitted_settlement(request.request_id()));
    assert_eq!(
        admitted.settlement().canonical_result_bytes.as_slice(),
        b"recorded-result"
    );
    let index = must_ok(observe_external_actions(&report));
    let recovered = match index.get(request.request_id()) {
        Some(recovered) => recovered,
        None => panic!("request was not recovered"),
    };
    assert_eq!(
        recovered
            .settlement
            .as_ref()
            .map(|value| value.canonical_result_bytes.as_slice()),
        Some(b"recorded-result".as_slice())
    );
}

#[test]
fn trusted_recovery_reconstructs_each_interrupted_transition_grant() {
    let mut store = store();
    let request = request_with("recover-grants", 11, 64);

    let request_commit_digest = {
        let mut request_coordinator = coordinator(&store);
        let recorded = record(
            &mut store,
            &mut request_coordinator,
            request,
            "request:recover-grants",
        );
        recorded.request_commit_digest()
    };

    let claim_commit_digest = {
        let mut claim_coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let recovered_request = must_ok(claim_coordinator.recorded_request(request.request_id()));
        assert_eq!(
            recovered_request.request_commit_digest(),
            request_commit_digest
        );
        let grant = claim(
            &mut store,
            &mut claim_coordinator,
            recovered_request,
            "claim:recover-grants",
        );
        grant.claim_commit_digest()
    };

    let settlement_commit_digest = {
        let mut settlement_coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let recovered_grant = must_ok(settlement_coordinator.claim_grant(request.request_id()));
        assert_eq!(recovered_grant.claim_commit_digest(), claim_commit_digest);
        let candidate = candidate(
            &recovered_grant,
            ExternalActionSettlementKindV1::Succeeded,
            b"recovered-result".to_vec(),
        );
        let admitted = must_ok(admit_external_action_settlement(
            &mut store,
            &mut settlement_coordinator,
            context("settlement:recover-grants"),
            recovered_grant,
            candidate,
        ));
        admitted.settlement_commit_digest()
    };

    let recovered_coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let resumed = must_ok(recovered_coordinator.admitted_settlement(request.request_id()));
    assert_eq!(resumed.settlement_commit_digest(), settlement_commit_digest);
    assert_eq!(
        resumed.settlement().canonical_result_bytes.as_slice(),
        b"recovered-result"
    );
}

#[test]
fn filesystem_reopen_recovers_settlement_without_adapter_reexecution() {
    let wal_dir = TempWalDir::new("reopen");
    let request = request_with("filesystem-reopen", 21, 128);
    let result_bytes = b"durable observed bytes".to_vec();
    {
        let mut store = must_ok(FilesystemWalStore::open(
            &wal_dir.0,
            WalSegmentId::from_raw(1),
        ));
        must_ok(store.acquire_writer_epoch(WriterEpochRequest {
            epoch_id: epoch_id(),
            storage_fencing_token: digest("external-action:fencing"),
            process_identity: digest("external-action:process"),
            host_identity: digest("external-action:host"),
            started_at_lsn: Lsn::from_raw(0),
            previous_epoch_id: None,
            previous_epoch_final_commit_digest: None,
            lease_or_lock_evidence: digest("external-action:lease"),
        }));
        let mut coordinator = coordinator(&store);
        let recorded = must_ok(record_external_action_request(
            &mut store,
            &mut coordinator,
            context_with_durability(
                "request:filesystem-reopen",
                WalDurabilityMode::StrictFilesystem,
            ),
            request,
        ));
        let grant = must_ok(claim_external_action(
            &mut store,
            &mut coordinator,
            context_with_durability(
                "claim:filesystem-reopen",
                WalDurabilityMode::StrictFilesystem,
            ),
            recorded,
            authorization(&request),
            request.basis_digest,
            0,
            digest("claim:filesystem-reopen:lease"),
        ));
        let candidate = candidate(
            &grant,
            ExternalActionSettlementKindV1::Succeeded,
            result_bytes.clone(),
        );
        must_ok(admit_external_action_settlement(
            &mut store,
            &mut coordinator,
            context_with_durability(
                "settlement:filesystem-reopen",
                WalDurabilityMode::StrictFilesystem,
            ),
            grant,
            candidate,
        ));
    }

    let report = must_ok(recover_filesystem_store(
        &wal_dir.0,
        RecoveryAccessMode::ReadOnly,
    ));
    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    let index = must_ok(observe_external_actions(&report));
    let recovered = match index.get(request.request_id()) {
        Some(recovered) => recovered,
        None => panic!("filesystem settlement was not recovered"),
    };
    assert_eq!(
        recovered.posture,
        RecoveredExternalActionPostureV1::Settled(ExternalActionSettlementKindV1::Succeeded)
    );
    assert_eq!(
        recovered
            .settlement
            .as_ref()
            .map(|settlement| settlement.canonical_result_bytes.as_slice()),
        Some(result_bytes.as_slice())
    );
}

#[test]
fn filesystem_scan_failure_cannot_be_admitted_as_genesis() {
    let wal_dir = TempWalDir::new("corrupt-snapshot");
    let store = must_ok(FilesystemWalStore::open(
        &wal_dir.0,
        WalSegmentId::from_raw(1),
    ));
    must_ok(std::fs::write(store.segment_path(), b"not-a-wal-segment"));

    assert!(matches!(
        ExternalActionCoordinatorV1::recover(&store),
        Err(ExternalActionProtocolErrorV1::WalStore(_))
    ));
}

#[test]
fn request_identity_is_deterministic_and_worldline_scoped() {
    let first = request_with("identity", 12, 64);
    let same = request_with("identity", 12, 64);
    let fork = request_with("identity", 13, 64);
    assert_eq!(first.request_id(), same.request_id());
    assert_ne!(first.request_id(), fork.request_id());
}

#[test]
fn raw_wal_builder_cannot_mint_external_action_authority() {
    let mut builder = raw_builder(
        "raw-external-action",
        0,
        WalTransactionKind::ExternalActionRequest,
    );
    assert_eq!(
        builder.push_record(WalRecordKind::ExternalActionRequestRecorded, Vec::new()),
        Err(WalBuildError::ExternalActionCoordinatorCapabilityRequired)
    );
}

#[test]
fn lifecycle_index_root_is_independent_of_request_insertion_order() {
    let first = request_with("index-order:first", 22, 64);
    let second = request_with("index-order:second", 22, 64);
    let mut left = store();
    let mut left_coordinator = coordinator(&left);
    record(
        &mut left,
        &mut left_coordinator,
        first,
        "request:index-order:left:first",
    );
    record(
        &mut left,
        &mut left_coordinator,
        second,
        "request:index-order:left:second",
    );
    let mut right = store();
    let mut right_coordinator = coordinator(&right);
    record(
        &mut right,
        &mut right_coordinator,
        second,
        "request:index-order:right:second",
    );
    record(
        &mut right,
        &mut right_coordinator,
        first,
        "request:index-order:right:first",
    );

    let left_report = must_ok(recover_in_memory_store(
        &mut left,
        RecoveryAccessMode::ReadOnly,
    ));
    let right_report = must_ok(recover_in_memory_store(
        &mut right,
        RecoveryAccessMode::ReadOnly,
    ));
    assert_eq!(
        must_ok(observe_external_actions(&left_report)).root_digest(),
        must_ok(observe_external_actions(&right_report)).root_digest()
    );
}

#[test]
fn fixed_seed_request_property_round_trips_unique_identities() {
    const SEED: u64 = 0x5eed_cafe_f00d_beef;
    const CASES: usize = 32;
    let mut state = SEED;
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let mut request_ids = std::collections::BTreeSet::new();
    for index in 0..CASES {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let request = request_with(&format!("property:{state:016x}"), 14, 64);
        assert!(request_ids.insert(request.request_id()));
        record(
            &mut store,
            &mut coordinator,
            request,
            &format!("request:property:{index}"),
        );
    }
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(observe_external_actions(&report));
    assert_eq!(index.len(), CASES);
}

#[test]
fn bounded_stress_recovers_all_requests_without_adapter_execution() {
    const REQUESTS: usize = 64;
    let mut store = store();
    let mut coordinator = coordinator(&store);
    for index in 0..REQUESTS {
        let request = request_with(&format!("stress:{index}"), 15, 64);
        record(
            &mut store,
            &mut coordinator,
            request,
            &format!("request:stress:{index}"),
        );
    }
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(observe_external_actions(&report));
    assert_eq!(index.len(), REQUESTS);
}

#[test]
fn duplicate_identical_settlement_is_idempotent_during_recovery() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("duplicate", 16, 64);
    let recorded = record(&mut store, &mut coordinator, request, "request:duplicate");
    let grant = claim(&mut store, &mut coordinator, recorded, "claim:duplicate");
    let first = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"first".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        &mut coordinator,
        context("settlement:duplicate"),
        grant,
        first,
    ));
    let mut report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let duplicate = match report.transactions.last().cloned() {
        Some(transaction) => transaction,
        None => panic!("settlement transaction was not recovered"),
    };
    report.transactions.push(duplicate);
    let recovered = must_ok(observe_external_actions(&report));
    assert_eq!(recovered.len(), 1);
    assert_eq!(
        recovered
            .get(request.request_id())
            .map(|entry| entry.posture),
        Some(RecoveredExternalActionPostureV1::Settled(
            ExternalActionSettlementKindV1::Succeeded
        ))
    );
}

#[derive(Debug)]
struct SnapshotCountingStore {
    inner: InMemoryWalStore,
    snapshot_reads: Cell<usize>,
}

impl WalStorePort for SnapshotCountingStore {
    fn acquire_writer_epoch(
        &mut self,
        request: WriterEpochRequest,
    ) -> Result<WriterEpoch, WalStoreError> {
        self.inner.acquire_writer_epoch(request)
    }

    fn append_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: WalFrame,
    ) -> Result<(), WalStoreError> {
        self.inner.append_frame(epoch_id, frame)
    }

    fn flush_commit(
        &mut self,
        epoch_id: WriterEpochId,
        commit: WalTransactionCommit,
    ) -> Result<(), WalStoreError> {
        self.inner.flush_commit(epoch_id, commit)
    }

    fn flush_external_action_commit(
        &mut self,
        epoch_id: WriterEpochId,
        commit: WalTransactionCommit,
        capability: ExternalActionCoordinatorCapability,
    ) -> Result<(), WalStoreError> {
        self.inner
            .flush_external_action_commit(epoch_id, commit, capability)
    }

    fn read_frames(&self) -> Vec<WalFrame> {
        self.inner.read_frames()
    }

    fn read_commits(&self) -> Vec<WalTransactionCommit> {
        self.inner.read_commits()
    }

    fn read_snapshot(&self) -> Result<WalStoreSnapshot, WalStoreError> {
        self.snapshot_reads
            .set(self.snapshot_reads.get().saturating_add(1));
        self.inner.read_snapshot()
    }

    fn seal_segment(
        &mut self,
        epoch_id: WriterEpochId,
        segment_id: WalSegmentId,
    ) -> Result<WalSegmentSeal, WalStoreError> {
        self.inner.seal_segment(epoch_id, segment_id)
    }

    fn truncate_tail_after(&mut self, after_lsn: Lsn) -> Result<(), WalStoreError> {
        self.inner.truncate_tail_after(after_lsn)
    }

    fn publish_manifest(
        &mut self,
        epoch_id: WriterEpochId,
        manifest: WalManifest,
    ) -> Result<(), WalStoreError> {
        self.inner.publish_manifest(epoch_id, manifest)
    }

    fn close_epoch(&mut self, epoch_id: WriterEpochId) -> Result<(), WalStoreError> {
        self.inner.close_epoch(epoch_id)
    }
}

#[test]
fn hot_path_advances_the_recovered_index_without_replaying_the_wal() {
    let mut store = SnapshotCountingStore {
        inner: store(),
        snapshot_reads: Cell::new(0),
    };
    let mut coordinator = coordinator(&store);
    assert_eq!(store.snapshot_reads.get(), 1);

    let request = request_with("incremental-index", 16, 64);
    let recorded = record(
        &mut store,
        &mut coordinator,
        request,
        "request:incremental-index",
    );
    let grant = claim(
        &mut store,
        &mut coordinator,
        recorded,
        "claim:incremental-index",
    );
    let candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"incremental".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        &mut coordinator,
        context("settlement:incremental-index"),
        grant,
        candidate,
    ));

    assert_eq!(store.snapshot_reads.get(), 1);
    assert_eq!(coordinator.observed_index().len(), 1);
}

#[derive(Debug)]
struct CommitFailingStore {
    inner: InMemoryWalStore,
    fail_on_commit_ordinal: usize,
}

impl WalStorePort for CommitFailingStore {
    fn acquire_writer_epoch(
        &mut self,
        request: WriterEpochRequest,
    ) -> Result<WriterEpoch, WalStoreError> {
        self.inner.acquire_writer_epoch(request)
    }

    fn append_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: WalFrame,
    ) -> Result<(), WalStoreError> {
        self.inner.append_frame(epoch_id, frame)
    }

    fn flush_commit(
        &mut self,
        epoch_id: WriterEpochId,
        commit: WalTransactionCommit,
    ) -> Result<(), WalStoreError> {
        if self.inner.commit_count() == self.fail_on_commit_ordinal {
            Err(WalStoreError::Io(
                "injected external-action commit failure".to_owned(),
            ))
        } else {
            self.inner.flush_commit(epoch_id, commit)
        }
    }

    fn flush_external_action_commit(
        &mut self,
        epoch_id: WriterEpochId,
        commit: WalTransactionCommit,
        capability: ExternalActionCoordinatorCapability,
    ) -> Result<(), WalStoreError> {
        if self.inner.commit_count() == self.fail_on_commit_ordinal {
            Err(WalStoreError::Io(
                "injected external-action commit failure".to_owned(),
            ))
        } else {
            self.inner
                .flush_external_action_commit(epoch_id, commit, capability)
        }
    }

    fn read_frames(&self) -> Vec<WalFrame> {
        self.inner.read_frames()
    }

    fn read_commits(&self) -> Vec<WalTransactionCommit> {
        self.inner.read_commits()
    }

    fn seal_segment(
        &mut self,
        epoch_id: WriterEpochId,
        segment_id: WalSegmentId,
    ) -> Result<WalSegmentSeal, WalStoreError> {
        self.inner.seal_segment(epoch_id, segment_id)
    }

    fn truncate_tail_after(&mut self, after_lsn: Lsn) -> Result<(), WalStoreError> {
        self.inner.truncate_tail_after(after_lsn)
    }

    fn publish_manifest(
        &mut self,
        epoch_id: WriterEpochId,
        manifest: WalManifest,
    ) -> Result<(), WalStoreError> {
        self.inner.publish_manifest(epoch_id, manifest)
    }

    fn close_epoch(&mut self, epoch_id: WriterEpochId) -> Result<(), WalStoreError> {
        self.inner.close_epoch(epoch_id)
    }
}

#[test]
fn failed_request_commit_exposes_no_adapter_reachable_token() {
    let mut store = CommitFailingStore {
        inner: store(),
        fail_on_commit_ordinal: 0,
    };
    let mut coordinator = coordinator(&store);
    let request = request_with("commit-failure", 17, 64);
    assert_eq!(
        record_external_action_request(
            &mut store,
            &mut coordinator,
            context("request:commit-failure"),
            request,
        ),
        Err(ExternalActionProtocolErrorV1::WalStore(WalStoreError::Io(
            "injected external-action commit failure".to_owned()
        )))
    );
    assert_eq!(store.read_commits().len(), 0);
    assert_eq!(store.read_frames().len(), 1);
    let report = must_ok(recover_from_frames_and_commits(
        &store.read_frames(),
        &store.read_commits(),
        RecoveryAccessMode::ReadOnly,
    ));
    assert_eq!(report.tail_posture, RecoveryTailPosture::WouldTruncateAll);
    assert!(must_ok(observe_external_actions(&report)).is_empty());
    assert_eq!(
        coordinator.recorded_request(request.request_id()),
        Err(ExternalActionProtocolErrorV1::CoordinatorRecoveryRequired)
    );
}

#[test]
fn failed_claim_commit_exposes_no_adapter_work_grant() {
    let mut store = CommitFailingStore {
        inner: store(),
        fail_on_commit_ordinal: 1,
    };
    let mut coordinator = coordinator(&store);
    let request = request_with("claim-commit-failure", 17, 64);
    let recorded = must_ok(record_external_action_request(
        &mut store,
        &mut coordinator,
        context("request:claim-commit-failure"),
        request,
    ));
    assert_eq!(
        claim_external_action(
            &mut store,
            &mut coordinator,
            context("claim:commit-failure"),
            recorded,
            authorization(&request),
            request.basis_digest,
            0,
            digest("claim:commit-failure:lease"),
        ),
        Err(ExternalActionProtocolErrorV1::WalStore(WalStoreError::Io(
            "injected external-action commit failure".to_owned()
        )))
    );
    assert_eq!(store.read_commits().len(), 1);
    assert_eq!(store.read_frames().len(), 2);
}

#[test]
fn failed_settlement_commit_exposes_no_resumable_fact() {
    let mut store = CommitFailingStore {
        inner: store(),
        fail_on_commit_ordinal: 2,
    };
    let mut coordinator = coordinator(&store);
    let request = request_with("settlement-commit-failure", 17, 64);
    let recorded = must_ok(record_external_action_request(
        &mut store,
        &mut coordinator,
        context("request:settlement-commit-failure"),
        request,
    ));
    let grant = must_ok(claim_external_action(
        &mut store,
        &mut coordinator,
        context("claim:settlement-commit-failure"),
        recorded,
        authorization(&request),
        request.basis_digest,
        0,
        digest("claim:settlement-commit-failure:lease"),
    ));
    let candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"uncommitted-result".to_vec(),
    );
    assert_eq!(
        admit_external_action_settlement(
            &mut store,
            &mut coordinator,
            context("settlement:commit-failure"),
            grant,
            candidate,
        ),
        Err(ExternalActionProtocolErrorV1::WalStore(WalStoreError::Io(
            "injected external-action commit failure".to_owned()
        )))
    );
    assert_eq!(store.read_commits().len(), 2);
    assert_eq!(store.read_frames().len(), 3);
}

#[test]
fn malformed_committed_settlement_payload_is_rejected() {
    let mut store = store();
    let mut coordinator = coordinator(&store);
    let request = request_with("malformed-payload", 18, 64);
    let recorded = record(
        &mut store,
        &mut coordinator,
        request,
        "request:malformed-payload",
    );
    let grant = claim(
        &mut store,
        &mut coordinator,
        recorded,
        "claim:malformed-payload",
    );
    let candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"retained".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        &mut coordinator,
        context("settlement:malformed-payload"),
        grant,
        candidate,
    ));
    let mut report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let settlement_frame = match report
        .transactions
        .last_mut()
        .and_then(|transaction| transaction.frames.first_mut())
    {
        Some(frame) => frame,
        None => panic!("settlement frame was not recovered"),
    };
    settlement_frame.payload.canonical_bytes.truncate(7);
    assert!(matches!(
        observe_external_actions(&report),
        Err(ExternalActionProtocolErrorV1::Decode(
            warp_core::causal_wal::WalDecodeError::UnexpectedEof
        ))
    ));
}

#[test]
fn external_action_wal_codes_and_frontier_are_stable() {
    assert_eq!(WalTransactionKind::ExternalActionRequest.stable_code(), 10);
    assert_eq!(WalTransactionKind::ExternalActionClaim.stable_code(), 11);
    assert_eq!(
        WalTransactionKind::ExternalActionSettlement.stable_code(),
        12
    );
    assert_eq!(
        warp_core::causal_wal::WalRecordKind::ExternalActionRequestRecorded.stable_code(),
        29
    );
    assert_eq!(
        warp_core::causal_wal::WalRecordKind::ExternalActionClaimRecorded.stable_code(),
        30
    );
    assert_eq!(
        warp_core::causal_wal::WalRecordKind::ExternalActionSettlementRecorded.stable_code(),
        31
    );
    assert_eq!(AffectedFrontierKind::ExternalActionIndex.stable_code(), 11);
}
