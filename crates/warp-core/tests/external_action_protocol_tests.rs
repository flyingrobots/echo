// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Durable external-action request and settlement protocol tests.

#![allow(clippy::panic)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use warp_core::causal_wal::{
    recover_filesystem_store, recover_from_frames_and_commits, recover_in_memory_store,
    AffectedFrontier, AffectedFrontierKind, FilesystemWalStore, InMemoryWalStore, Lsn,
    PayloadCodecId, PayloadSchemaId, RecoveryAccessMode, RecoveryTailPosture, WalAppendAuthority,
    WalDurabilityMode, WalFrame, WalManifest, WalSegmentId, WalSegmentSeal, WalStoreError,
    WalStorePort, WalTransactionBuilder, WalTransactionCommit, WalTransactionId,
    WalTransactionKind, WriterEpoch, WriterEpochId, WriterEpochRequest,
};
use warp_core::external_action::{
    admit_external_action_settlement, build_external_action_request_transaction,
    build_external_action_settlement_transaction, claim_external_action,
    record_external_action_request, recover_external_actions, ExternalActionAdapterAuthorizationV1,
    ExternalActionAdapterBindingV1, ExternalActionAdapterIdV1, ExternalActionAdapterRegistryV1,
    ExternalActionBudgetV1, ExternalActionClaimGrantV1, ExternalActionOperationIdV1,
    ExternalActionProtocolErrorV1, ExternalActionRequestV1, ExternalActionSettlementCandidateV1,
    ExternalActionSettlementKindV1, ExternalActionSettlementV1, RecoveredExternalActionPostureV1,
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

fn builder(label: &str, first_lsn: u64, kind: WalTransactionKind) -> WalTransactionBuilder {
    builder_with_durability(label, first_lsn, kind, WalDurabilityMode::Buffered)
}

fn builder_with_durability(
    label: &str,
    first_lsn: u64,
    kind: WalTransactionKind,
    durability_mode: WalDurabilityMode,
) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        epoch_id(),
        WalSegmentId::from_raw(1),
        WalTransactionId::from_hash(digest(label)),
        kind,
        WalAppendAuthority::ExternalActionCoordinator,
        Lsn::from_raw(first_lsn),
        digest("external-action:previous-frame"),
        digest("external-action:previous-commit"),
        durability_mode,
        PayloadCodecId::from_hash(digest("external-action:codec")),
        PayloadSchemaId::from_hash(digest("external-action:schema")),
        1,
        1,
        digest("external-action:domain"),
    )
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

fn frontier(label: &str) -> Vec<AffectedFrontier> {
    vec![AffectedFrontier {
        kind: AffectedFrontierKind::ExternalActionIndex,
        before_digest: digest(&format!("{label}:before")),
        after_digest: digest(&format!("{label}:after")),
    }]
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
    store: &mut InMemoryWalStore,
    request: ExternalActionRequestV1,
    lsn: u64,
    label: &str,
) -> warp_core::external_action::DurablyRecordedExternalActionRequestV1 {
    must_ok(record_external_action_request(
        store,
        builder(label, lsn, WalTransactionKind::ExternalActionRequest),
        request,
    ))
}

#[allow(clippy::large_types_passed_by_value)]
fn claim(
    store: &mut InMemoryWalStore,
    recorded: warp_core::external_action::DurablyRecordedExternalActionRequestV1,
    lsn: u64,
    label: &str,
) -> ExternalActionClaimGrantV1 {
    let basis = recorded.request().basis_digest;
    let authorization = authorization(&recorded.request());
    must_ok(claim_external_action(
        store,
        builder(label, lsn, WalTransactionKind::ExternalActionClaim),
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

fn settlement(candidate: &ExternalActionSettlementCandidateV1) -> ExternalActionSettlementV1 {
    ExternalActionSettlementV1 {
        request_id: candidate.request_id,
        attempt_id: candidate.attempt_id,
        adapter_id: candidate.adapter_id,
        kind: candidate.kind,
        settlement_schema_digest: candidate.settlement_schema_digest,
        basis_digest: candidate.basis_digest,
        canonical_result_bytes: candidate.canonical_result_bytes.clone(),
        result_digest: candidate.declared_result_digest,
        schema_admission_evidence_digest: candidate.schema_admission_evidence_digest,
        external_evidence_digest: candidate.external_evidence_digest,
    }
}

#[test]
fn request_and_settlement_are_committed_before_authority_crosses_the_boundary() {
    let mut store = store();
    let request = request_with("golden", 7, 128);
    let recorded = record(&mut store, request, 0, "request:golden");
    assert_eq!(store.read_commits().len(), 1);
    assert_eq!(
        store.read_commits()[0].commit_digest,
        recorded.request_commit_digest()
    );

    let grant = claim(&mut store, recorded, 1, "claim:golden");
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
        builder(
            "settlement:golden",
            2,
            WalTransactionKind::ExternalActionSettlement,
        ),
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
    let request = request_with("claim-obstructions", 8, 64);
    let recorded = record(&mut store, request, 0, "request:claim-obstructions");
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
            builder("claim:stale", 1, WalTransactionKind::ExternalActionClaim),
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
    let authorized_request = request_with("authorization-source", 8, 64);
    let claimed_request = request_with("authorization-target", 8, 64);
    let recorded = record(
        &mut store,
        claimed_request,
        0,
        "request:authorization-target",
    );
    let commits_before = store.read_commits().len();

    assert!(claim_external_action(
        &mut store,
        builder(
            "claim:authorization-target",
            1,
            WalTransactionKind::ExternalActionClaim,
        ),
        recorded,
        authorization(&authorized_request),
        claimed_request.basis_digest,
        0,
        digest("claim:authorization-target:lease"),
    )
    .is_err());
    assert_eq!(store.read_commits().len(), commits_before);
}

#[test]
fn claims_and_settlements_require_nonzero_external_evidence() {
    let mut claim_store = store();
    let claim_request = request_with("missing-lease-evidence", 8, 64);
    let claim_recorded = record(
        &mut claim_store,
        claim_request,
        0,
        "request:missing-lease-evidence",
    );
    let claim_commits_before = claim_store.read_commits().len();
    assert!(claim_external_action(
        &mut claim_store,
        builder(
            "claim:missing-lease-evidence",
            1,
            WalTransactionKind::ExternalActionClaim,
        ),
        claim_recorded,
        authorization(&claim_request),
        claim_request.basis_digest,
        0,
        [0; 32],
    )
    .is_err());
    assert_eq!(claim_store.read_commits().len(), claim_commits_before);

    let mut settlement_store = store();
    let settlement_request = request_with("missing-external-evidence", 8, 64);
    let settlement_recorded = record(
        &mut settlement_store,
        settlement_request,
        0,
        "request:missing-external-evidence",
    );
    let grant = claim(
        &mut settlement_store,
        settlement_recorded,
        1,
        "claim:missing-external-evidence",
    );
    let mut candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"observed".to_vec(),
    );
    candidate.external_evidence_digest = [0; 32];
    let settlement_commits_before = settlement_store.read_commits().len();
    assert!(admit_external_action_settlement(
        &mut settlement_store,
        builder(
            "settlement:missing-external-evidence",
            2,
            WalTransactionKind::ExternalActionSettlement,
        ),
        grant,
        candidate,
    )
    .is_err());
    assert_eq!(
        settlement_store.read_commits().len(),
        settlement_commits_before
    );
}

#[test]
fn recovery_rejects_forged_external_action_frontier_evidence() {
    let mut store = store();
    let request = request_with("forged-frontier", 8, 64);
    let transaction = must_ok(build_external_action_request_transaction(
        builder(
            "request:forged-frontier",
            0,
            WalTransactionKind::ExternalActionRequest,
        ),
        request,
        frontier("forged-frontier"),
    ));
    must_ok(store.append_transaction(transaction));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    assert!(recover_external_actions(&report).is_err());
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
        let request = request_with(label, 9, 4);
        let recorded = record(&mut store, request, 0, &format!("request:{label}"));
        let grant = claim(&mut store, recorded, 1, &format!("claim:{label}"));
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
                builder(
                    &format!("settlement:{label}"),
                    2,
                    WalTransactionKind::ExternalActionSettlement,
                ),
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
    let request = request_with("attempt-budget", 19, 64);
    let recorded = record(&mut store, request, 0, "request:attempt-budget");
    let commits_before = store.read_commits().len();
    assert_eq!(
        claim_external_action(
            &mut store,
            builder(
                "claim:attempt-budget",
                1,
                WalTransactionKind::ExternalActionClaim,
            ),
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
fn recovery_distinguishes_unclaimed_claimed_settled_and_ambiguous_requests() {
    let mut store = store();

    let requested = request_with("requested", 10, 64);
    record(&mut store, requested, 0, "request:requested");

    let claimed = request_with("claimed", 10, 64);
    let claimed_recorded = record(&mut store, claimed, 1, "request:claimed");
    claim(&mut store, claimed_recorded, 2, "claim:claimed");

    let settled = request_with("settled", 10, 64);
    let settled_recorded = record(&mut store, settled, 3, "request:settled");
    let settled_grant = claim(&mut store, settled_recorded, 4, "claim:settled");
    let settled_candidate = candidate(
        &settled_grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"settled".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        builder(
            "settlement:settled",
            5,
            WalTransactionKind::ExternalActionSettlement,
        ),
        settled_grant,
        settled_candidate,
    ));

    let ambiguous = request_with("ambiguous", 10, 64);
    let ambiguous_recorded = record(&mut store, ambiguous, 6, "request:ambiguous");
    let ambiguous_grant = claim(&mut store, ambiguous_recorded, 7, "claim:ambiguous");
    let ambiguous_candidate = candidate(
        &ambiguous_grant,
        ExternalActionSettlementKindV1::OutcomeUnknown,
        b"connection-lost".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        builder(
            "settlement:ambiguous",
            8,
            WalTransactionKind::ExternalActionSettlement,
        ),
        ambiguous_grant,
        ambiguous_candidate,
    ));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_external_actions(&report));
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
    let request = request_with("replay", 11, 64);
    let recorded = record(&mut store, request, 0, "request:replay");
    let grant = claim(&mut store, recorded, 1, "claim:replay");
    let candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"recorded-result".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        builder(
            "settlement:replay",
            2,
            WalTransactionKind::ExternalActionSettlement,
        ),
        grant,
        candidate,
    ));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_external_actions(&report));
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
        let recorded = must_ok(record_external_action_request(
            &mut store,
            builder_with_durability(
                "request:filesystem-reopen",
                0,
                WalTransactionKind::ExternalActionRequest,
                WalDurabilityMode::StrictFilesystem,
            ),
            request,
        ));
        let grant = must_ok(claim_external_action(
            &mut store,
            builder_with_durability(
                "claim:filesystem-reopen",
                1,
                WalTransactionKind::ExternalActionClaim,
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
            builder_with_durability(
                "settlement:filesystem-reopen",
                2,
                WalTransactionKind::ExternalActionSettlement,
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
    let index = must_ok(recover_external_actions(&report));
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
fn request_identity_is_deterministic_and_worldline_scoped() {
    let first = request_with("identity", 12, 64);
    let same = request_with("identity", 12, 64);
    let fork = request_with("identity", 13, 64);
    assert_eq!(first.request_id(), same.request_id());
    assert_ne!(first.request_id(), fork.request_id());
}

#[test]
fn lifecycle_index_root_is_independent_of_request_insertion_order() {
    let first = request_with("index-order:first", 22, 64);
    let second = request_with("index-order:second", 22, 64);
    let mut left = store();
    record(&mut left, first, 0, "request:index-order:left:first");
    record(&mut left, second, 1, "request:index-order:left:second");
    let mut right = store();
    record(&mut right, second, 0, "request:index-order:right:second");
    record(&mut right, first, 1, "request:index-order:right:first");

    let left_report = must_ok(recover_in_memory_store(
        &mut left,
        RecoveryAccessMode::ReadOnly,
    ));
    let right_report = must_ok(recover_in_memory_store(
        &mut right,
        RecoveryAccessMode::ReadOnly,
    ));
    assert_eq!(
        must_ok(recover_external_actions(&left_report)).root_digest(),
        must_ok(recover_external_actions(&right_report)).root_digest()
    );
}

#[test]
fn fixed_seed_request_property_round_trips_unique_identities() {
    const SEED: u64 = 0x5eed_cafe_f00d_beef;
    const CASES: usize = 32;
    let mut state = SEED;
    let mut store = store();
    let mut request_ids = std::collections::BTreeSet::new();
    for index in 0..CASES {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let request = request_with(&format!("property:{state:016x}"), 14, 64);
        assert!(request_ids.insert(request.request_id()));
        record(
            &mut store,
            request,
            index as u64,
            &format!("request:property:{index}"),
        );
    }
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_external_actions(&report));
    assert_eq!(index.len(), CASES);
}

#[test]
fn bounded_stress_recovers_all_requests_without_adapter_execution() {
    const REQUESTS: usize = 64;
    let mut store = store();
    for index in 0..REQUESTS {
        let request = request_with(&format!("stress:{index}"), 15, 64);
        record(
            &mut store,
            request,
            index as u64,
            &format!("request:stress:{index}"),
        );
    }
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_external_actions(&report));
    assert_eq!(index.len(), REQUESTS);
}

#[test]
fn duplicate_and_conflicting_settlements_are_recovery_obstructions() {
    for (label, second_bytes, expected) in [
        (
            "duplicate",
            b"first".to_vec(),
            ExternalActionProtocolErrorV1::DuplicateSettlement,
        ),
        (
            "conflict",
            b"second".to_vec(),
            ExternalActionProtocolErrorV1::ConflictingSettlement,
        ),
    ] {
        let mut store = store();
        let request = request_with(label, 16, 64);
        let recorded = record(&mut store, request, 0, &format!("request:{label}"));
        let grant = claim(&mut store, recorded, 1, &format!("claim:{label}"));
        let first = candidate(
            &grant,
            ExternalActionSettlementKindV1::Succeeded,
            b"first".to_vec(),
        );
        let mut second = first.clone();
        second.canonical_result_bytes = second_bytes;
        second.declared_result_digest = blake3::hash(&second.canonical_result_bytes).into();
        must_ok(admit_external_action_settlement(
            &mut store,
            builder(
                &format!("settlement:{label}:first"),
                2,
                WalTransactionKind::ExternalActionSettlement,
            ),
            grant,
            first,
        ));
        let first_report = must_ok(recover_in_memory_store(
            &mut store,
            RecoveryAccessMode::ReadOnly,
        ));
        let settled_root = must_ok(recover_external_actions(&first_report)).root_digest();
        let duplicate_transaction = must_ok(build_external_action_settlement_transaction(
            builder(
                &format!("settlement:{label}:second"),
                3,
                WalTransactionKind::ExternalActionSettlement,
            ),
            settlement(&second),
            vec![AffectedFrontier {
                kind: AffectedFrontierKind::ExternalActionIndex,
                before_digest: settled_root,
                after_digest: settled_root,
            }],
        ));
        must_ok(store.append_transaction(duplicate_transaction));

        let report = must_ok(recover_in_memory_store(
            &mut store,
            RecoveryAccessMode::ReadOnly,
        ));
        assert_eq!(recover_external_actions(&report), Err(expected));
    }
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
        if self.inner.read_commits().len() == self.fail_on_commit_ordinal {
            Err(WalStoreError::Io(
                "injected external-action commit failure".to_owned(),
            ))
        } else {
            self.inner.flush_commit(epoch_id, commit)
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
    let request = request_with("commit-failure", 17, 64);
    assert_eq!(
        record_external_action_request(
            &mut store,
            builder(
                "request:commit-failure",
                0,
                WalTransactionKind::ExternalActionRequest,
            ),
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
    assert!(must_ok(recover_external_actions(&report)).is_empty());
}

#[test]
fn failed_claim_commit_exposes_no_adapter_work_grant() {
    let mut store = CommitFailingStore {
        inner: store(),
        fail_on_commit_ordinal: 1,
    };
    let request = request_with("claim-commit-failure", 17, 64);
    let recorded = must_ok(record_external_action_request(
        &mut store,
        builder(
            "request:claim-commit-failure",
            0,
            WalTransactionKind::ExternalActionRequest,
        ),
        request,
    ));
    assert_eq!(
        claim_external_action(
            &mut store,
            builder(
                "claim:commit-failure",
                1,
                WalTransactionKind::ExternalActionClaim,
            ),
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
    let request = request_with("settlement-commit-failure", 17, 64);
    let recorded = must_ok(record_external_action_request(
        &mut store,
        builder(
            "request:settlement-commit-failure",
            0,
            WalTransactionKind::ExternalActionRequest,
        ),
        request,
    ));
    let grant = must_ok(claim_external_action(
        &mut store,
        builder(
            "claim:settlement-commit-failure",
            1,
            WalTransactionKind::ExternalActionClaim,
        ),
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
            builder(
                "settlement:commit-failure",
                2,
                WalTransactionKind::ExternalActionSettlement,
            ),
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
    let request = request_with("malformed-payload", 18, 64);
    let recorded = record(&mut store, request, 0, "request:malformed-payload");
    let grant = claim(&mut store, recorded, 1, "claim:malformed-payload");
    let candidate = candidate(
        &grant,
        ExternalActionSettlementKindV1::Succeeded,
        b"retained".to_vec(),
    );
    must_ok(admit_external_action_settlement(
        &mut store,
        builder(
            "settlement:malformed-payload",
            2,
            WalTransactionKind::ExternalActionSettlement,
        ),
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
        recover_external_actions(&report),
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
