// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! RED contract for compiler-authored bounded workspace observation.

#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use echo_edict_canonical::{encode_canonical_cbor_v1, CanonicalValueV1};
use warp_core::causal_wal::{
    InMemoryWalStore, Lsn, PayloadCodecId, PayloadSchemaId, WalDurabilityMode, WalSegmentId,
    WalStorePort, WalTransactionId, WriterEpochId, WriterEpochRequest,
};
use warp_core::external_action::{
    claim_external_action, record_external_action_request, ExternalActionAdapterIdV1,
    ExternalActionAdapterRegistryV1, ExternalActionClaimGrantV1, ExternalActionCoordinatorV1,
    ExternalActionProtocolErrorV1, ExternalActionSettlementKindV1,
    ExternalActionTransactionContextV1, RecoveredExternalActionPostureV1,
};
use warp_core::external_action_adapter::{
    admit_edict_external_action_request_v1, bounded_workspace_observation_basis_v1,
    encode_bounded_workspace_observation_input_v1, AdmittedEdictExternalActionRequestV1,
    BoundedWorkspaceObservationAdapterV1, BoundedWorkspaceObservationErrorV1,
    BoundedWorkspaceObservationProfileV1, EdictExternalActionAdmissionErrorV1,
};
use warp_core::{Hash, WorldlineId};

const CORE_BYTES: &[u8] = include_bytes!("fixtures/external_action/observe-workspace.core.cbor");
const TARGET_IR_BYTES: &[u8] =
    include_bytes!("fixtures/external_action/observe-workspace.target-ir.cbor");
const CORE_DIGEST: &str = include_str!("fixtures/external_action/observe-workspace.core.sha256");
const TARGET_IR_DIGEST: &str =
    include_str!("fixtures/external_action/observe-workspace.target-ir.sha256");

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn must_ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok(..), got {error:?}"),
    }
}

fn text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn map(entries: impl IntoIterator<Item = (&'static str, CanonicalValueV1)>) -> CanonicalValueV1 {
    CanonicalValueV1::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn application_input(
    operation_input: Vec<u8>,
    scope: Hash,
    basis: Hash,
    max_settlement_bytes: u64,
) -> Vec<u8> {
    must_ok(encode_canonical_cbor_v1(&map([
        ("payload", CanonicalValueV1::Bytes(operation_input)),
        ("scope", CanonicalValueV1::Bytes(scope.to_vec())),
        ("basis", CanonicalValueV1::Bytes(basis.to_vec())),
        (
            "maxSettlementBytes",
            CanonicalValueV1::Integer(i128::from(max_settlement_bytes)),
        ),
        ("maxAttempts", CanonicalValueV1::Integer(1)),
    ])))
}

fn admitted_request(
    worldline_byte: u8,
    paths: impl IntoIterator<Item = String>,
    scope: Hash,
    basis: Hash,
    max_settlement_bytes: u64,
) -> AdmittedEdictExternalActionRequestV1 {
    let operation_input = must_ok(encode_bounded_workspace_observation_input_v1(paths));
    must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([worldline_byte; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "observe",
        &application_input(operation_input, scope, basis, max_settlement_bytes),
    ))
}

fn profile(
    admitted: &AdmittedEdictExternalActionRequestV1,
    adapter_label: &str,
) -> BoundedWorkspaceObservationProfileV1 {
    let request = admitted.request();
    BoundedWorkspaceObservationProfileV1 {
        operation_id: request.operation_id,
        input_schema_digest: request.input_schema_digest,
        settlement_schema_digest: request.settlement_schema_digest,
        reconciliation_law_digest: request.reconciliation_law_digest,
        authority_scope_digest: request.authority_scope_digest,
        adapter_id: ExternalActionAdapterIdV1::from_hash(digest(adapter_label)),
    }
}

fn epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(digest("bounded-observation:epoch"))
}

fn store() -> InMemoryWalStore {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(WriterEpochRequest {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("bounded-observation:fencing"),
        process_identity: digest("bounded-observation:process"),
        host_identity: digest("bounded-observation:host"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("bounded-observation:lease"),
    }));
    store
}

fn context(label: &str) -> ExternalActionTransactionContextV1 {
    ExternalActionTransactionContextV1 {
        writer_epoch: epoch_id(),
        segment_id: WalSegmentId::from_raw(1),
        transaction_id: WalTransactionId::from_hash(digest(label)),
        durability_mode: WalDurabilityMode::Buffered,
        payload_codec_id: PayloadCodecId::from_hash(digest("bounded-observation:codec")),
        payload_schema_id: PayloadSchemaId::from_hash(digest("bounded-observation:schema")),
        payload_schema_version: 1,
        canonical_encoding_version: 1,
        digest_domain: digest("bounded-observation:wal-domain"),
    }
}

fn claim(
    store: &mut InMemoryWalStore,
    coordinator: &mut ExternalActionCoordinatorV1,
    admitted: &AdmittedEdictExternalActionRequestV1,
    adapter: &BoundedWorkspaceObservationAdapterV1,
    label: &str,
) -> ExternalActionClaimGrantV1 {
    let request = admitted.request();
    let recorded = must_ok(record_external_action_request(
        store,
        coordinator,
        context(&format!("{label}:request")),
        request,
    ));
    let registry = ExternalActionAdapterRegistryV1::new([adapter.adapter_binding()]);
    let authorization = must_ok(registry.authorize(&request, adapter.adapter_binding().adapter_id));
    must_ok(claim_external_action(
        store,
        coordinator,
        context(&format!("{label}:claim")),
        recorded,
        authorization,
        request.basis_digest,
        0,
        digest(&format!("{label}:lease")),
    ))
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "echo-bounded-observation-{}-{counter}-{label}",
            std::process::id()
        ));
        if root.exists() {
            must_ok(fs::remove_dir_all(&root));
        }
        must_ok(fs::create_dir_all(&root));
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            must_ok(fs::create_dir_all(parent));
        }
        must_ok(fs::write(path, bytes));
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn adapter(
    root: &TempRoot,
    admitted: &AdmittedEdictExternalActionRequestV1,
    permitted_paths: impl IntoIterator<Item = String>,
) -> BoundedWorkspaceObservationAdapterV1 {
    must_ok(BoundedWorkspaceObservationAdapterV1::open(
        root.path(),
        permitted_paths,
        profile(admitted, "bounded-observation:adapter"),
    ))
}

#[test]
fn exact_compiler_artifacts_admit_one_noncallable_request() {
    let scope = digest("scope:exact");
    let basis = digest("basis:exact");
    let operation_input = must_ok(encode_bounded_workspace_observation_input_v1([
        "src/lib.rs".to_owned(),
    ]));
    let admitted = must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([7; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "observe",
        &application_input(operation_input.clone(), scope, basis, 65_536),
    ));

    assert_eq!(admitted.source_core_digest(), CORE_DIGEST.trim());
    assert_eq!(admitted.target_ir_digest(), TARGET_IR_DIGEST.trim());
    assert_eq!(
        admitted.operation_coordinate(),
        "workspace.snapshot.observe@1"
    );
    assert_eq!(admitted.canonical_operation_input(), operation_input);
    assert_eq!(admitted.request().authority_scope_digest, scope);
    assert_eq!(admitted.request().basis_digest, basis);
    assert_eq!(admitted.request().budget.max_settlement_bytes, 65_536);
    assert_eq!(admitted.request().budget.max_attempts, 1);
}

#[test]
fn malformed_or_substituted_compiler_artifacts_fail_closed() {
    let input = application_input(vec![], digest("scope"), digest("basis"), 1024);
    let mut malformed_core = CORE_BYTES.to_vec();
    malformed_core.push(0);
    assert!(matches!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            &malformed_core,
            TARGET_IR_BYTES,
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::Canonical(_))
    ));

    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            TARGET_IR_BYTES,
            "missing",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::MissingIntent)
    );
}

#[test]
fn request_and_claim_are_durable_before_the_adapter_can_read() {
    let root = TempRoot::new("ordered");
    let bytes = b"durable before effect";
    let basis = bounded_workspace_observation_basis_v1([("notes/evidence.txt", bytes.as_slice())]);
    let admitted = admitted_request(
        2,
        ["notes/evidence.txt".to_owned()],
        digest("scope:ordered"),
        basis,
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["notes/evidence.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(&mut store, &mut coordinator, &admitted, &adapter, "ordered");

    root.write("notes/evidence.txt", bytes);
    let candidate = must_ok(adapter.observe(&grant, &admitted));
    assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Succeeded);
    let settled = must_ok(adapter.admit_settlement(
        &mut store,
        &mut coordinator,
        context("ordered:settlement"),
        grant,
        candidate,
    ));
    assert_eq!(
        settled.settlement().kind,
        ExternalActionSettlementKindV1::Succeeded
    );
    assert_ne!(settled.settlement_commit_digest(), [0; 32]);
}

#[test]
fn absolute_parent_escaped_and_unauthorized_paths_settle_as_rejected() {
    for (index, path) in [
        "/etc/passwd",
        "../outside",
        "nested/../../outside",
        "other.txt",
    ]
    .into_iter()
    .enumerate()
    {
        let root = TempRoot::new(&format!("path-{index}"));
        root.write("allowed.txt", b"allowed");
        let admitted = admitted_request(
            u8::try_from(index + 10).expect("small index"),
            [path.to_owned()],
            digest(&format!("scope:path-{index}")),
            digest(&format!("basis:path-{index}")),
            65_536,
        );
        let adapter = adapter(&root, &admitted, ["allowed.txt".to_owned()]);
        let mut store = store();
        let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            &adapter,
            &format!("path-{index}"),
        );
        let candidate = must_ok(adapter.observe(&grant, &admitted));
        assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Rejected);
        must_ok(adapter.admit_settlement(
            &mut store,
            &mut coordinator,
            context(&format!("path-{index}:settlement")),
            grant,
            candidate,
        ));
    }
}

#[cfg(unix)]
#[test]
fn symlink_components_are_rejected_without_reading_the_target() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("symlink");
    root.write("outside.txt", b"must remain unread");
    must_ok(symlink(
        root.path().join("outside.txt"),
        root.path().join("link.txt"),
    ));
    let admitted = admitted_request(
        20,
        ["link.txt".to_owned()],
        digest("scope:symlink"),
        digest("basis:symlink"),
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["link.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(&mut store, &mut coordinator, &admitted, &adapter, "symlink");
    let candidate = must_ok(adapter.observe(&grant, &admitted));
    assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Rejected);
}

#[test]
fn exact_settlement_boundary_succeeds_and_one_byte_less_rejects() {
    let root = TempRoot::new("budget");
    let bytes = vec![0x5a; 4096];
    root.write("blob.bin", &bytes);
    let basis = bounded_workspace_observation_basis_v1([("blob.bin", bytes.as_slice())]);
    let generous = admitted_request(
        30,
        ["blob.bin".to_owned()],
        digest("scope:budget"),
        basis,
        65_536,
    );
    let generous_adapter = adapter(&root, &generous, ["blob.bin".to_owned()]);
    let mut generous_store = store();
    let mut generous_coordinator = must_ok(ExternalActionCoordinatorV1::recover(&generous_store));
    let generous_grant = claim(
        &mut generous_store,
        &mut generous_coordinator,
        &generous,
        &generous_adapter,
        "budget:measure",
    );
    let measured = must_ok(generous_adapter.observe(&generous_grant, &generous));
    let exact_budget =
        u64::try_from(measured.canonical_result_bytes.len()).expect("result fits u64");

    let exact = admitted_request(
        31,
        ["blob.bin".to_owned()],
        digest("scope:budget"),
        basis,
        exact_budget,
    );
    let exact_adapter = adapter(&root, &exact, ["blob.bin".to_owned()]);
    let mut exact_store = store();
    let mut exact_coordinator = must_ok(ExternalActionCoordinatorV1::recover(&exact_store));
    let exact_grant = claim(
        &mut exact_store,
        &mut exact_coordinator,
        &exact,
        &exact_adapter,
        "budget:exact",
    );
    assert_eq!(
        must_ok(exact_adapter.observe(&exact_grant, &exact)).kind,
        ExternalActionSettlementKindV1::Succeeded
    );

    let below = admitted_request(
        32,
        ["blob.bin".to_owned()],
        digest("scope:budget"),
        basis,
        exact_budget - 1,
    );
    let below_adapter = adapter(&root, &below, ["blob.bin".to_owned()]);
    let mut below_store = store();
    let mut below_coordinator = must_ok(ExternalActionCoordinatorV1::recover(&below_store));
    let below_grant = claim(
        &mut below_store,
        &mut below_coordinator,
        &below,
        &below_adapter,
        "budget:below",
    );
    assert_eq!(
        must_ok(below_adapter.observe(&below_grant, &below)).kind,
        ExternalActionSettlementKindV1::Rejected
    );
}

#[test]
fn stale_basis_settles_as_rejected() {
    let root = TempRoot::new("stale");
    root.write("state.txt", b"current");
    let admitted = admitted_request(
        40,
        ["state.txt".to_owned()],
        digest("scope:stale"),
        bounded_workspace_observation_basis_v1([("state.txt", b"prior".as_slice())]),
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["state.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(&mut store, &mut coordinator, &admitted, &adapter, "stale");
    assert_eq!(
        must_ok(adapter.observe(&grant, &admitted)).kind,
        ExternalActionSettlementKindV1::Rejected
    );
}

#[test]
fn malformed_settlement_cannot_bypass_the_profile_validator() {
    let root = TempRoot::new("malformed");
    root.write("valid.txt", b"valid");
    let basis = bounded_workspace_observation_basis_v1([("valid.txt", b"valid".as_slice())]);
    let admitted = admitted_request(
        50,
        ["valid.txt".to_owned()],
        digest("scope:malformed"),
        basis,
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["valid.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "malformed",
    );
    let mut candidate = must_ok(adapter.observe(&grant, &admitted));
    candidate.canonical_result_bytes.push(0);
    candidate.declared_result_digest = blake3::hash(&candidate.canonical_result_bytes).into();
    assert_eq!(
        adapter.admit_settlement(
            &mut store,
            &mut coordinator,
            context("malformed:settlement"),
            grant,
            candidate,
        ),
        Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)
    );
}

#[test]
fn requested_claimed_unknown_and_settled_postures_recover() {
    let root = TempRoot::new("recovery");
    let bytes = b"recoverable";
    root.write("recover.txt", bytes);
    let admitted = admitted_request(
        60,
        ["recover.txt".to_owned()],
        digest("scope:recovery"),
        bounded_workspace_observation_basis_v1([("recover.txt", bytes.as_slice())]),
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["recover.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let request = admitted.request();
    let recorded = must_ok(record_external_action_request(
        &mut store,
        &mut coordinator,
        context("recovery:request"),
        request,
    ));
    let recovered_requested = must_ok(ExternalActionCoordinatorV1::recover(&store));
    assert_eq!(
        recovered_requested
            .observed_index()
            .get(request.request_id())
            .expect("request recovers")
            .posture,
        RecoveredExternalActionPostureV1::Requested
    );

    let authorization = must_ok(
        ExternalActionAdapterRegistryV1::new([adapter.adapter_binding()])
            .authorize(&request, adapter.adapter_binding().adapter_id),
    );
    let grant = must_ok(claim_external_action(
        &mut store,
        &mut coordinator,
        context("recovery:claim"),
        recorded,
        authorization,
        request.basis_digest,
        0,
        digest("recovery:lease"),
    ));
    let recovered_claimed = must_ok(ExternalActionCoordinatorV1::recover(&store));
    assert_eq!(
        recovered_claimed
            .observed_index()
            .get(request.request_id())
            .expect("claim recovers")
            .posture,
        RecoveredExternalActionPostureV1::Claimed
    );

    let candidate =
        must_ok(adapter.outcome_unknown(&grant, &admitted, digest("recovery:ambiguous")));
    must_ok(adapter.admit_settlement(
        &mut store,
        &mut coordinator,
        context("recovery:settlement"),
        grant,
        candidate,
    ));
    let recovered_settled = must_ok(ExternalActionCoordinatorV1::recover(&store));
    assert_eq!(
        recovered_settled
            .observed_index()
            .get(request.request_id())
            .expect("settlement recovers")
            .posture,
        RecoveredExternalActionPostureV1::Settled(ExternalActionSettlementKindV1::OutcomeUnknown)
    );
}

#[test]
fn settled_replay_uses_wal_bytes_after_the_source_disappears() {
    let root = TempRoot::new("replay");
    let bytes = b"retained";
    root.write("retained.txt", bytes);
    let admitted = admitted_request(
        70,
        ["retained.txt".to_owned()],
        digest("scope:replay"),
        bounded_workspace_observation_basis_v1([("retained.txt", bytes.as_slice())]),
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["retained.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(&mut store, &mut coordinator, &admitted, &adapter, "replay");
    let candidate = must_ok(adapter.observe(&grant, &admitted));
    let expected_bytes = candidate.canonical_result_bytes.clone();
    must_ok(adapter.admit_settlement(
        &mut store,
        &mut coordinator,
        context("replay:settlement"),
        grant,
        candidate,
    ));

    must_ok(fs::remove_file(root.path().join("retained.txt")));
    let recovered = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let replay = must_ok(recovered.admitted_settlement(admitted.request().request_id()));
    assert_eq!(replay.settlement().canonical_result_bytes, expected_bytes);
}

#[test]
fn fixed_seed_property_corpus_is_deterministic() {
    const SEED: u64 = 0x51a7_7e11_cafe_babe;
    let mut state = SEED;
    for case in 0_u8..32 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let length = usize::try_from((state % 257) + 1).expect("bounded length");
        let bytes = vec![case; length];
        let path = format!("property/{case:02}.bin");
        let root = TempRoot::new(&format!("property-{case}"));
        root.write(&path, &bytes);
        let basis = bounded_workspace_observation_basis_v1([(path.as_str(), bytes.as_slice())]);
        let admitted = admitted_request(
            case.saturating_add(80),
            [path.clone()],
            digest(&format!("scope:property-{case}")),
            basis,
            65_536,
        );
        let adapter = adapter(&root, &admitted, [path]);
        let mut store = store();
        let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            &adapter,
            &format!("property-{case}"),
        );
        let first = must_ok(adapter.observe(&grant, &admitted));
        let second = must_ok(adapter.observe(&grant, &admitted));
        assert_eq!(first, second, "fixed seed {SEED:#x}, case {case}");
    }
}

#[test]
fn bounded_stress_settles_many_requests_without_identity_collision() {
    const REQUEST_COUNT: u8 = 64;
    let root = TempRoot::new("stress");
    root.write("stress.txt", b"bounded");
    let basis = bounded_workspace_observation_basis_v1([("stress.txt", b"bounded".as_slice())]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));

    for index in 0..REQUEST_COUNT {
        let admitted = admitted_request(
            index.saturating_add(128),
            ["stress.txt".to_owned()],
            digest("scope:stress"),
            basis,
            65_536,
        );
        let adapter = adapter(&root, &admitted, ["stress.txt".to_owned()]);
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            &adapter,
            &format!("stress-{index}"),
        );
        let candidate = must_ok(adapter.observe(&grant, &admitted));
        must_ok(adapter.admit_settlement(
            &mut store,
            &mut coordinator,
            context(&format!("stress-{index}:settlement")),
            grant,
            candidate,
        ));
    }

    let recovered = must_ok(ExternalActionCoordinatorV1::recover(&store));
    assert_eq!(recovered.observed_index().len(), usize::from(REQUEST_COUNT));
    assert!(matches!(
        recovered.recorded_request(
            admitted_request(
                1,
                ["stress.txt".to_owned()],
                digest("scope:stress"),
                basis,
                65_536,
            )
            .request()
            .request_id()
        ),
        Err(ExternalActionProtocolErrorV1::MissingRequest)
    ));
}
