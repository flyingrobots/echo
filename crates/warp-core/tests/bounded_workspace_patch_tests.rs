// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! RED contract for compiler-authored basis-bound workspace patches.

#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use echo_edict_canonical::{decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueV1};
use warp_core::causal_wal::{
    InMemoryWalStore, Lsn, PayloadCodecId, PayloadSchemaId, WalDurabilityMode, WalSegmentId,
    WalStorePort, WalTransactionId, WriterEpochId, WriterEpochRequest,
};
use warp_core::external_action::{
    claim_external_action, reconcile_external_action_settlement_retry,
    record_external_action_request, ExternalActionAdapterBindingV1, ExternalActionAdapterIdV1,
    ExternalActionAdapterRegistryV1, ExternalActionClaimGrantV1, ExternalActionCoordinatorV1,
    ExternalActionProtocolErrorV1, ExternalActionSettlementCandidateV1,
    ExternalActionSettlementKindV1, ExternalActionTransactionContextV1,
    RecoveredExternalActionPostureV1,
};
use warp_core::external_action_adapter::{
    admit_edict_external_action_request_v1, bounded_workspace_observation_basis_v1,
    AdmittedEdictExternalActionRequestV1, EdictExternalActionAdmissionErrorV1,
};
use warp_core::validated_workspace_patch::{
    encode_validated_workspace_patch_input_v1, validated_workspace_patch_authority_v1,
    validated_workspace_patch_basis_v1, ValidatedWorkspacePatchAdapterV1,
    ValidatedWorkspacePatchErrorV1, ValidatedWorkspacePatchProfileV1,
    ValidatedWorkspacePatchReconcilerV1,
};
use warp_core::{Hash, WorldlineId};

const CORE_BYTES: &[u8] =
    include_bytes!("fixtures/external_action_patch/apply-validated-patch.core.cbor");
const TARGET_IR_BYTES: &[u8] =
    include_bytes!("fixtures/external_action_patch/apply-validated-patch.target-ir.cbor");
const CORE_DIGEST: &str =
    include_str!("fixtures/external_action_patch/apply-validated-patch.core.sha256");
const TARGET_IR_DIGEST: &str =
    include_str!("fixtures/external_action_patch/apply-validated-patch.target-ir.sha256");

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn must_ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok(..), got {error:?}"),
    }
}

fn must_some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected Some(..), got None"),
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
    patch: Vec<u8>,
    authority: Hash,
    basis: Hash,
    max_settlement_bytes: u64,
) -> Vec<u8> {
    must_ok(encode_canonical_cbor_v1(&map([
        ("patch", CanonicalValueV1::Bytes(patch)),
        ("authority", CanonicalValueV1::Bytes(authority.to_vec())),
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
    path: &str,
    before: &[u8],
    replacement: &[u8],
    authority: Hash,
    max_settlement_bytes: u64,
) -> AdmittedEdictExternalActionRequestV1 {
    let patch = must_ok(encode_validated_workspace_patch_input_v1(
        path.to_owned(),
        blake3::hash(before).into(),
        replacement.to_vec(),
    ));
    must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([worldline_byte; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "applyValidated",
        &application_input(
            patch,
            authority,
            validated_workspace_patch_basis_v1(path, before),
            max_settlement_bytes,
        ),
    ))
}

fn raw_patch_input(path: &str, before: &[u8], replacement: &[u8]) -> Vec<u8> {
    must_ok(encode_canonical_cbor_v1(&map([
        (
            "kind",
            CanonicalValueV1::Text("validatedWorkspacePatchInput".to_owned()),
        ),
        ("path", CanonicalValueV1::Text(path.to_owned())),
        (
            "expectedContentDigest",
            CanonicalValueV1::Bytes(blake3::hash(before).as_bytes().to_vec()),
        ),
        ("replacement", CanonicalValueV1::Bytes(replacement.to_vec())),
        (
            "replacementDigest",
            CanonicalValueV1::Bytes(blake3::hash(replacement).as_bytes().to_vec()),
        ),
    ])))
}

fn profile(
    admitted: &AdmittedEdictExternalActionRequestV1,
    adapter_label: &str,
    max_file_bytes: u64,
) -> ValidatedWorkspacePatchProfileV1 {
    let request = admitted.request();
    ValidatedWorkspacePatchProfileV1 {
        operation_id: request.operation_id,
        input_schema_digest: request.input_schema_digest,
        settlement_schema_digest: request.settlement_schema_digest,
        reconciliation_law_digest: request.reconciliation_law_digest,
        authority_scope_digest: request.authority_scope_digest,
        adapter_id: ExternalActionAdapterIdV1::from_hash(digest(adapter_label)),
        max_file_bytes,
    }
}

fn epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(digest("bounded-patch:epoch"))
}

fn store() -> InMemoryWalStore {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(WriterEpochRequest {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("bounded-patch:fencing"),
        process_identity: digest("bounded-patch:process"),
        host_identity: digest("bounded-patch:host"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("bounded-patch:lease"),
    }));
    store
}

fn context(label: &str) -> ExternalActionTransactionContextV1 {
    ExternalActionTransactionContextV1 {
        writer_epoch: epoch_id(),
        segment_id: WalSegmentId::from_raw(1),
        transaction_id: WalTransactionId::from_hash(digest(label)),
        durability_mode: WalDurabilityMode::Buffered,
        payload_codec_id: PayloadCodecId::from_hash(digest("bounded-patch:codec")),
        payload_schema_id: PayloadSchemaId::from_hash(digest("bounded-patch:schema")),
        payload_schema_version: 1,
        canonical_encoding_version: 1,
        digest_domain: digest("bounded-patch:wal-domain"),
    }
}

fn claim(
    store: &mut InMemoryWalStore,
    coordinator: &mut ExternalActionCoordinatorV1,
    admitted: &AdmittedEdictExternalActionRequestV1,
    binding: ExternalActionAdapterBindingV1,
    label: &str,
) -> ExternalActionClaimGrantV1 {
    let request = admitted.request();
    let recorded = must_ok(record_external_action_request(
        store,
        coordinator,
        context(&format!("{label}:request")),
        request,
    ));
    let registry = ExternalActionAdapterRegistryV1::new([binding]);
    let authorization = must_ok(registry.authorize(&request, binding.adapter_id));
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
            "echo-bounded-patch-{}-{counter}-{label}",
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

    fn read(&self, relative: &str) -> Vec<u8> {
        must_ok(fs::read(self.0.join(relative)))
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn field(candidate: &ExternalActionSettlementCandidateV1, name: &str) -> CanonicalValueV1 {
    let value = must_ok(decode_canonical_cbor_v1(&candidate.canonical_result_bytes));
    let CanonicalValueV1::Map(entries) = value else {
        panic!("expected settlement map");
    };
    let value = entries.into_iter().find_map(|(key, value)| match key {
        CanonicalValueV1::Text(key) if key == name => Some(value),
        _ => None,
    });
    match value {
        Some(value) => value,
        None => panic!("missing settlement field {name}"),
    }
}

fn obstruction(candidate: &ExternalActionSettlementCandidateV1) -> String {
    match field(candidate, "obstruction") {
        CanonicalValueV1::Text(value) => value,
        value => panic!("expected obstruction text, got {value:?}"),
    }
}

#[test]
fn exact_compiler_artifacts_admit_one_noncallable_patch_request() {
    let authority = digest("authority:exact");
    let basis = digest("basis:exact");
    let patch = must_ok(encode_canonical_cbor_v1(&map([])));
    let admitted = must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([23; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "applyValidated",
        &application_input(patch.clone(), authority, basis, 65_536),
    ));

    assert_eq!(admitted.source_core_digest(), CORE_DIGEST.trim());
    assert_eq!(admitted.target_ir_digest(), TARGET_IR_DIGEST.trim());
    assert_eq!(
        admitted.operation_coordinate(),
        "workspace.patch.applyValidated@1"
    );
    assert_eq!(admitted.canonical_operation_input(), patch);
    assert_eq!(admitted.request().authority_scope_digest, authority);
    assert_eq!(admitted.request().basis_digest, basis);
    assert_eq!(admitted.request().budget.max_settlement_bytes, 65_536);
    assert_eq!(admitted.request().budget.max_attempts, 1);
    assert_eq!(
        validated_workspace_patch_basis_v1("src/lib.rs", b"before"),
        bounded_workspace_observation_basis_v1([("src/lib.rs", b"before".as_slice())])
    );
}

#[test]
fn request_only_settlement_budget_has_an_exact_admission_floor() {
    let authority = digest("authority:budget-floor");
    let basis = digest("basis:budget-floor");
    let patch = raw_patch_input("src/value.txt", b"before", b"after");

    let exact = must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([24; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "applyValidated",
        &application_input(patch.clone(), authority, basis, 1_024),
    ));
    assert_eq!(exact.request().budget.max_settlement_bytes, 1_024);
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([25; 32]),
            CORE_BYTES,
            TARGET_IR_BYTES,
            "applyValidated",
            &application_input(patch, authority, basis, 1_023),
        ),
        Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue)
    );
}

#[test]
fn durable_claim_precedes_mutation_and_settlement_precedes_replay() {
    let root = TempRoot::new("golden");
    let path = "src/message.txt";
    let before = b"hello";
    let replacement = b"hello from Echo";
    root.write(path, before);
    let authority = validated_workspace_patch_authority_v1([path]);
    let admitted = admitted_request(30, path, before, replacement, authority, 65_536);
    let profile = profile(&admitted, "bounded-patch:golden-adapter", 65_536);
    let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));

    assert_eq!(root.read(path), before);
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        adapter.adapter_binding(),
        "golden",
    );
    let request_id = grant.request().request_id();
    assert_eq!(
        must_some(coordinator.observed_index().get(request_id)).posture,
        RecoveredExternalActionPostureV1::Claimed
    );
    assert_eq!(root.read(path), before);

    let candidate = must_ok(adapter.apply(&grant, &admitted));
    assert_eq!(
        candidate.kind,
        ExternalActionSettlementKindV1::Succeeded,
        "unexpected settlement obstruction: {}",
        obstruction(&candidate)
    );
    assert_eq!(root.read(path), replacement);
    let staged_name = format!(
        "src/.message.txt.echo-patch-{}",
        hex::encode(grant.claim().attempt_id.as_hash())
    );
    assert!(!root.path().join(staged_name).exists());
    assert!(matches!(
        must_ok(coordinator.claim_grant(request_id)).claim(),
        claim if claim == grant.claim()
    ));

    let settled = must_ok(adapter.admit_settlement(
        &mut store,
        &mut coordinator,
        context("golden:settlement"),
        &admitted,
        grant,
        candidate.clone(),
    ));
    assert_eq!(
        settled.settlement().canonical_result_bytes,
        candidate.canonical_result_bytes
    );
    root.write(path, b"changed after settlement");
    let retried = must_ok(reconcile_external_action_settlement_retry(
        &coordinator,
        candidate.clone(),
    ));
    assert_eq!(
        retried.settlement().canonical_result_bytes,
        candidate.canonical_result_bytes
    );
    assert_eq!(root.read(path), b"changed after settlement");
    let mut conflicting = candidate.clone();
    conflicting.external_evidence_digest = digest("conflicting-settlement");
    assert_eq!(
        reconcile_external_action_settlement_retry(&coordinator, conflicting),
        Err(ExternalActionProtocolErrorV1::ConflictingSettlement)
    );

    let recovered = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let recovered_entry = must_some(recovered.observed_index().get(request_id));
    assert_eq!(
        recovered_entry.posture,
        RecoveredExternalActionPostureV1::Settled(ExternalActionSettlementKindV1::Succeeded)
    );
    assert_eq!(
        must_some(recovered_entry.settlement.as_ref()).canonical_result_bytes,
        candidate.canonical_result_bytes
    );
    assert_eq!(root.read(path), b"changed after settlement");
}

#[test]
fn malformed_and_invalid_inputs_settle_as_refusals() {
    let root = TempRoot::new("invalid-input");
    let path = "src/value.txt";
    let before = b"before";
    root.write(path, before);
    let authority = validated_workspace_patch_authority_v1([path]);

    for (ordinal, input, code) in [
        (35_u8, vec![0xff], "malformed-input"),
        (
            36_u8,
            raw_patch_input("../outside.txt", before, b"after"),
            "invalid-path",
        ),
    ] {
        let admitted = must_ok(admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([ordinal; 32]),
            CORE_BYTES,
            TARGET_IR_BYTES,
            "applyValidated",
            &application_input(
                input,
                authority,
                validated_workspace_patch_basis_v1(path, before),
                65_536,
            ),
        ));
        let profile = profile(
            &admitted,
            &format!("bounded-patch:invalid-{ordinal}"),
            65_536,
        );
        let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
            root.path(),
            [path.to_owned()],
            profile,
        ));
        let mut store = store();
        let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            adapter.adapter_binding(),
            &format!("invalid-{ordinal}"),
        );
        let candidate = must_ok(adapter.apply(&grant, &admitted));
        assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Rejected);
        assert_eq!(obstruction(&candidate), code);
        must_ok(adapter.admit_settlement(
            &mut store,
            &mut coordinator,
            context(&format!("invalid-{ordinal}:settlement")),
            &admitted,
            grant,
            candidate,
        ));
        assert_eq!(root.read(path), before);
    }
}

#[test]
fn stale_basis_and_path_policy_refuse_before_mutation() {
    let root = TempRoot::new("refusals");
    let permitted = "src/permitted.txt";
    root.write(permitted, b"current");
    root.write("src/other.txt", b"other");
    root.write(".github/workflows/ci.yml", b"name: ci");
    root.write(".GITHUB/Workflows/upper.yml", b"name: upper");
    let authority = validated_workspace_patch_authority_v1([permitted]);

    for (ordinal, requested_path, expected_before, code) in [
        (40_u8, permitted, b"stale".as_slice(), "stale-basis"),
        (
            41,
            "src/other.txt",
            b"other".as_slice(),
            "unauthorized-path",
        ),
        (
            42,
            ".github/workflows/ci.yml",
            b"name: ci".as_slice(),
            "ci-workflow-refused",
        ),
        (
            43,
            ".GITHUB/Workflows/upper.yml",
            b"name: upper".as_slice(),
            "ci-workflow-refused",
        ),
    ] {
        let admitted = admitted_request(
            ordinal,
            requested_path,
            expected_before,
            b"replacement",
            authority,
            65_536,
        );
        let profile = profile(
            &admitted,
            &format!("bounded-patch:refusal-{ordinal}"),
            65_536,
        );
        let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
            root.path(),
            [permitted.to_owned()],
            profile,
        ));
        let mut store = store();
        let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            adapter.adapter_binding(),
            &format!("refusal-{ordinal}"),
        );
        let before_bytes = root.read(requested_path);
        let candidate = must_ok(adapter.apply(&grant, &admitted));
        assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Rejected);
        assert_eq!(obstruction(&candidate), code);
        assert_eq!(root.read(requested_path), before_bytes);
    }
}

#[cfg(unix)]
#[test]
fn symlinks_and_special_files_refuse_before_mutation() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("file-kinds");
    root.write("src/real.txt", b"real");
    must_ok(symlink(
        root.path().join("src/real.txt"),
        root.path().join("src/link.txt"),
    ));
    must_ok(fs::create_dir_all(root.path().join("src/directory")));

    for (ordinal, path, code) in [
        (50_u8, "src/link.txt", "symlink-refused"),
        (51_u8, "src/directory", "not-regular-file"),
    ] {
        let authority = validated_workspace_patch_authority_v1([path]);
        let admitted = admitted_request(ordinal, path, b"real", b"replacement", authority, 65_536);
        let profile = profile(
            &admitted,
            &format!("bounded-patch:file-kind-{ordinal}"),
            65_536,
        );
        let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
            root.path(),
            [path.to_owned()],
            profile,
        ));
        let mut store = store();
        let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            adapter.adapter_binding(),
            &format!("file-kind-{ordinal}"),
        );
        let candidate = must_ok(adapter.apply(&grant, &admitted));
        assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Rejected);
        assert_eq!(obstruction(&candidate), code);
    }
    assert_eq!(root.read("src/real.txt"), b"real");
    assert!(root.path().join("src/directory").is_dir());
}

#[test]
fn file_budget_and_grant_substitution_fail_closed() {
    let root = TempRoot::new("budgets");
    let path = "bounded.txt";
    root.write(path, b"1234");
    let authority = validated_workspace_patch_authority_v1([path]);
    let admitted = admitted_request(60, path, b"1234", b"56789", authority, 65_536);
    let profile = profile(&admitted, "bounded-patch:budget", 4);
    let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        adapter.adapter_binding(),
        "budget",
    );
    let candidate = must_ok(adapter.apply(&grant, &admitted));
    assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Rejected);
    assert_eq!(obstruction(&candidate), "replacement-budget-exceeded");
    assert_eq!(root.read(path), b"1234");

    let other = admitted_request(61, path, b"1234", b"other", authority, 65_536);
    assert_eq!(
        adapter.apply(&grant, &other),
        Err(ValidatedWorkspacePatchErrorV1::GrantMismatch)
    );
    assert_eq!(root.read(path), b"1234");
}

#[test]
fn settlement_budget_is_preflighted_before_mutation() {
    let root = TempRoot::new("settlement-budget");
    let segment = "a".repeat(80);
    let path = format!(
        "{segment}/{segment}/{segment}/{segment}/{segment}/{segment}/{segment}/{segment}/{segment}/{segment}/value.txt"
    );
    let before = b"before";
    root.write(&path, before);
    let authority = validated_workspace_patch_authority_v1([path.as_str()]);
    let admitted = admitted_request(62, &path, before, b"after", authority, 1_024);
    let profile = profile(&admitted, "bounded-patch:settlement-budget", 65_536);
    let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
        root.path(),
        [path.clone()],
        profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        adapter.adapter_binding(),
        "settlement-budget",
    );

    let candidate = must_ok(adapter.apply(&grant, &admitted));
    assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Rejected);
    assert_eq!(obstruction(&candidate), "settlement-budget-exceeded");
    assert_eq!(root.read(&path), before);
}

#[test]
fn reconciliation_observes_postcondition_without_reapplying() {
    let root = TempRoot::new("reconcile-success");
    let path = "src/reconcile.txt";
    let before = b"before";
    let replacement = b"after";
    root.write(path, before);
    let authority = validated_workspace_patch_authority_v1([path]);
    let admitted = admitted_request(70, path, before, replacement, authority, 65_536);
    let profile = profile(&admitted, "bounded-patch:reconciler", 65_536);
    let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        adapter.adapter_binding(),
        "reconcile-success",
    );
    let request_id = grant.request().request_id();

    let first_candidate = must_ok(adapter.apply(&grant, &admitted));
    assert_eq!(
        first_candidate.kind,
        ExternalActionSettlementKindV1::Succeeded
    );
    assert_eq!(root.read(path), replacement);
    let recovered_claimed = must_ok(ExternalActionCoordinatorV1::recover(&store));
    assert_eq!(
        must_some(recovered_claimed.observed_index().get(request_id)).posture,
        RecoveredExternalActionPostureV1::Claimed
    );

    let reconciler = must_ok(ValidatedWorkspacePatchReconcilerV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let candidate = must_ok(reconciler.reconcile(&grant, &admitted));
    assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Succeeded);
    assert_eq!(root.read(path), replacement);
    must_ok(reconciler.admit_settlement(
        &mut store,
        &mut coordinator,
        context("reconcile-success:settlement"),
        &admitted,
        grant,
        candidate,
    ));
    assert_eq!(root.read(path), replacement);
}

#[test]
fn reconciliation_reports_unknown_when_postcondition_is_absent() {
    let root = TempRoot::new("reconcile-unknown");
    let path = "src/reconcile.txt";
    let before = b"before";
    root.write(path, before);
    let authority = validated_workspace_patch_authority_v1([path]);
    let admitted = admitted_request(71, path, before, b"intended", authority, 65_536);
    let profile = profile(&admitted, "bounded-patch:unknown-reconciler", 65_536);
    let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        adapter.adapter_binding(),
        "reconcile-unknown",
    );
    root.write(path, b"neither before nor intended");

    let reconciler = must_ok(ValidatedWorkspacePatchReconcilerV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let candidate = must_ok(reconciler.reconcile(&grant, &admitted));
    assert_eq!(
        candidate.kind,
        ExternalActionSettlementKindV1::OutcomeUnknown
    );
    assert_eq!(obstruction(&candidate), "postcondition-not-observed");
    assert_eq!(root.read(path), b"neither before nor intended");
}

#[test]
fn reconciliation_reports_unknown_when_postcondition_is_unreadable() {
    let root = TempRoot::new("reconcile-unreadable");
    let path = "src/reconcile.txt";
    let before = b"before";
    root.write(path, before);
    let authority = validated_workspace_patch_authority_v1([path]);
    let admitted = admitted_request(72, path, before, b"intended", authority, 65_536);
    let profile = profile(&admitted, "bounded-patch:unreadable-reconciler", 65_536);
    let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        adapter.adapter_binding(),
        "reconcile-unreadable",
    );
    must_ok(fs::remove_file(root.path().join(path)));

    let reconciler = must_ok(ValidatedWorkspacePatchReconcilerV1::open(
        root.path(),
        [path.to_owned()],
        profile,
    ));
    let candidate = must_ok(reconciler.reconcile(&grant, &admitted));
    assert_eq!(
        candidate.kind,
        ExternalActionSettlementKindV1::OutcomeUnknown
    );
    assert_eq!(obstruction(&candidate), "postcondition-unreadable");
    assert!(!root.path().join(path).exists());
}

#[test]
fn fixed_seed_patch_property_covers_binary_replacements() {
    let root = TempRoot::new("property");
    let path = "property.bin";
    let authority = validated_workspace_patch_authority_v1([path]);
    let mut state = 0x5eed_cafe_f00d_beef_u64;

    for ordinal in 0_u8..32 {
        let before = vec![ordinal; usize::from(ordinal % 7) + 1];
        let mut replacement = vec![0_u8; usize::from(ordinal % 19) + 1];
        for byte in &mut replacement {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            *byte = (state >> 56) as u8;
        }
        root.write(path, &before);
        let admitted =
            admitted_request(ordinal + 80, path, &before, &replacement, authority, 65_536);
        let profile = profile(
            &admitted,
            &format!("bounded-patch:property-{ordinal}"),
            65_536,
        );
        let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
            root.path(),
            [path.to_owned()],
            profile,
        ));
        let mut store = store();
        let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            adapter.adapter_binding(),
            &format!("property-{ordinal}"),
        );
        let candidate = must_ok(adapter.apply(&grant, &admitted));
        assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Succeeded);
        assert_eq!(root.read(path), replacement);
    }
}

#[test]
fn bounded_stress_applies_sixty_four_independent_patches() {
    let root = TempRoot::new("stress");

    for ordinal in 0_u8..64 {
        let path = format!("stress/file-{ordinal:02}.txt");
        let before = format!("before-{ordinal}").into_bytes();
        let replacement = format!("after-{ordinal}").into_bytes();
        root.write(&path, &before);
        let authority = validated_workspace_patch_authority_v1([path.as_str()]);
        let admitted = admitted_request(
            ordinal + 120,
            &path,
            &before,
            &replacement,
            authority,
            65_536,
        );
        let profile = profile(
            &admitted,
            &format!("bounded-patch:stress-{ordinal}"),
            65_536,
        );
        let adapter = must_ok(ValidatedWorkspacePatchAdapterV1::open(
            root.path(),
            [path.clone()],
            profile,
        ));
        let mut store = store();
        let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
        let grant = claim(
            &mut store,
            &mut coordinator,
            &admitted,
            adapter.adapter_binding(),
            &format!("stress-{ordinal}"),
        );
        let candidate = must_ok(adapter.apply(&grant, &admitted));
        assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Succeeded);
        assert_eq!(root.read(&path), replacement);
    }
}
