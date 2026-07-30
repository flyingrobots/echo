// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! RED contract for compiler-authored bounded workspace observation.

#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1, CanonicalValueV1,
};
use warp_core::causal_wal::{
    InMemoryWalStore, Lsn, PayloadCodecId, PayloadSchemaId, WalDurabilityMode, WalSegmentId,
    WalStorePort, WalTransactionId, WriterEpochId, WriterEpochRequest,
};
use warp_core::external_action::{
    claim_external_action, record_external_action_request, ExternalActionAdapterIdV1,
    ExternalActionAdapterRegistryV1, ExternalActionClaimGrantV1, ExternalActionCoordinatorV1,
    ExternalActionProtocolErrorV1, ExternalActionSettlementCandidateV1,
    ExternalActionSettlementKindV1, ExternalActionTransactionContextV1,
    RecoveredExternalActionPostureV1,
};
use warp_core::external_action_adapter::{
    admit_edict_external_action_request_v1, bounded_workspace_observation_basis_v1,
    encode_bounded_workspace_observation_input_v1, AdmittedEdictExternalActionRequestV1,
    BoundedWorkspaceObservationAdapterV1, BoundedWorkspaceObservationErrorV1,
    BoundedWorkspaceObservationProfileV1, BoundedWorkspaceObservationReconcilerV1,
    EdictExternalActionAdmissionErrorV1,
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

fn map_field_mut<'a>(value: &'a mut CanonicalValueV1, field: &str) -> &'a mut CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("expected map containing {field}");
    };
    must_some(entries.iter_mut().find_map(|(key, value)| match key {
        CanonicalValueV1::Text(key) if key == field => Some(value),
        _ => None,
    }))
}

fn target_ir_value() -> CanonicalValueV1 {
    must_ok(decode_canonical_cbor_v1(TARGET_IR_BYTES))
}

fn core_value() -> CanonicalValueV1 {
    must_ok(decode_canonical_cbor_v1(CORE_BYTES))
}

fn encoded(value: &CanonicalValueV1) -> Vec<u8> {
    must_ok(encode_canonical_cbor_v1(value))
}

fn intent_mut<'a>(artifact: &'a mut CanonicalValueV1, name: &str) -> &'a mut CanonicalValueV1 {
    map_field_mut(map_field_mut(artifact, "intents"), name)
}

fn target_request_mut(artifact: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    let requests = map_field_mut(intent_mut(artifact, "observe"), "externalActionRequests");
    let CanonicalValueV1::Array(requests) = requests else {
        panic!("expected external request array");
    };
    must_some(requests.first_mut())
}

fn bind_target_to_core(target: &mut CanonicalValueV1, core: &CanonicalValueV1) {
    let reviewed = must_ok(digest_canonical_value_v1("edict.core.module/v1", core));
    let reviewed = must_some(reviewed.strip_prefix("sha256:"));
    let reviewed = must_ok(hex::decode(reviewed));
    let source = map_field_mut(map_field_mut(target, "semanticClosure"), "sourceCore");
    let digest = map_field_mut(source, "digest");
    let CanonicalValueV1::Array(digest) = digest else {
        panic!("expected reviewed digest");
    };
    let Some(CanonicalValueV1::Bytes(bytes)) = digest.get_mut(1) else {
        panic!("expected reviewed digest bytes");
    };
    *bytes = reviewed;
}

fn schema_admission_evidence(schema: Hash, bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo.bounded-observation.schema-evidence/v1");
    hasher.update(&schema);
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}

fn rewrite_first_local_id(value: &mut CanonicalValueV1, replacement: &str) -> bool {
    match value {
        CanonicalValueV1::Map(entries) => {
            let is_local = entries.iter().any(|(key, value)| {
                matches!(
                    (key, value),
                    (CanonicalValueV1::Text(key), CanonicalValueV1::Text(value))
                        if key == "kind" && value == "local"
                )
            });
            if is_local {
                let Some(CanonicalValueV1::Map(reference)) =
                    entries.iter_mut().find_map(|(key, value)| {
                        matches!(key, CanonicalValueV1::Text(key) if key == "ref").then_some(value)
                    })
                else {
                    panic!("local expression must contain a reference");
                };
                let Some(CanonicalValueV1::Text(id)) =
                    reference.iter_mut().find_map(|(key, value)| {
                        matches!(key, CanonicalValueV1::Text(key) if key == "id").then_some(value)
                    })
                else {
                    panic!("local reference must contain an id");
                };
                replacement.clone_into(id);
                true
            } else {
                entries
                    .iter_mut()
                    .any(|(_, value)| rewrite_first_local_id(value, replacement))
            }
        }
        CanonicalValueV1::Array(values) => values
            .iter_mut()
            .any(|value| rewrite_first_local_id(value, replacement)),
        _ => false,
    }
}

fn mutate_resource_digest(resources: &mut CanonicalValueV1, coordinate: &str, byte: u8) {
    let CanonicalValueV1::Array(resources) = resources else {
        panic!("expected resource array");
    };
    let resource = must_some(resources.iter_mut().find(|resource| {
        let CanonicalValueV1::Map(entries) = resource else {
            return false;
        };
        entries.iter().any(|(key, value)| {
            matches!(
                (key, value),
                (CanonicalValueV1::Text(key), CanonicalValueV1::Text(value))
                    if key == "id" && value == coordinate
            )
        })
    }));
    let digest = map_field_mut(resource, "digest");
    let CanonicalValueV1::Array(digest) = digest else {
        panic!("expected reviewed digest");
    };
    let Some(CanonicalValueV1::Bytes(bytes)) = digest.get_mut(1) else {
        panic!("expected digest bytes");
    };
    *bytes = vec![byte; 32];
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

    let mut wrong_domain = target_ir_value();
    *map_field_mut(&mut wrong_domain, "domain") = text("other.span-ir/v1");
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&wrong_domain),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::ArtifactShape)
    );

    let mut wrong_source_coordinate = target_ir_value();
    *map_field_mut(&mut wrong_source_coordinate, "sourceCoreCoordinate") = text("other.source@1");
    let closure = map_field_mut(&mut wrong_source_coordinate, "semanticClosure");
    *map_field_mut(map_field_mut(closure, "sourceCore"), "id") = text("other.source@1");
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&wrong_source_coordinate),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::CoreDigestMismatch)
    );

    let mut wrong_operation_profile = target_ir_value();
    let intent = map_field_mut(
        map_field_mut(&mut wrong_operation_profile, "intents"),
        "observe",
    );
    *map_field_mut(intent, "operationProfile") = text("other.profile/v1");
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&wrong_operation_profile),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::ArtifactShape)
    );

    let mut substituted_capability = target_ir_value();
    let semantic_closure = map_field_mut(&mut substituted_capability, "semanticClosure");
    mutate_resource_digest(
        map_field_mut(semantic_closure, "capabilities"),
        "workspace.snapshot.observe@1",
        0x77,
    );
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&substituted_capability),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::CapabilityClosureMismatch)
    );

    let mut callable = target_ir_value();
    let steps = map_field_mut(
        map_field_mut(map_field_mut(&mut callable, "intents"), "observe"),
        "steps",
    );
    *steps = CanonicalValueV1::Array(vec![CanonicalValueV1::Null]);
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&callable),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::CallableStepsPresent)
    );

    let mut hidden_local = target_ir_value();
    let requests = map_field_mut(
        map_field_mut(map_field_mut(&mut hidden_local, "intents"), "observe"),
        "externalActionRequests",
    );
    let CanonicalValueV1::Array(requests) = requests else {
        panic!("expected external request array");
    };
    assert!(rewrite_first_local_id(
        map_field_mut(&mut requests[0], "input"),
        "local.0"
    ));
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&hidden_local),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch)
    );

    let mut wrong_result = target_ir_value();
    let result = map_field_mut(
        map_field_mut(map_field_mut(&mut wrong_result, "intents"), "observe"),
        "result",
    );
    let result_reference = map_field_mut(result, "ref");
    *map_field_mut(result_reference, "id") = text("other.result");
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&wrong_result),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::ArtifactShape)
    );

    let mut wrong_basis = target_ir_value();
    let intent = map_field_mut(map_field_mut(&mut wrong_basis, "intents"), "observe");
    *map_field_mut(intent, "basis") = CanonicalValueV1::Null;
    assert_eq!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([1; 32]),
            CORE_BYTES,
            &encoded(&wrong_basis),
            "observe",
            &input,
        ),
        Err(EdictExternalActionAdmissionErrorV1::ArtifactShape)
    );
}

#[test]
fn target_profile_digest_is_part_of_the_runtime_operation_identity() {
    let input = application_input(vec![], digest("scope"), digest("basis"), 1024);
    let baseline = must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([2; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "observe",
        &input,
    ));
    let mut substituted_profile = target_ir_value();
    let profile = map_field_mut(&mut substituted_profile, "targetProfile");
    let digest = map_field_mut(profile, "digest");
    let CanonicalValueV1::Array(digest) = digest else {
        panic!("expected reviewed target profile digest");
    };
    let Some(CanonicalValueV1::Bytes(bytes)) = digest.get_mut(1) else {
        panic!("expected target profile digest bytes");
    };
    *bytes = vec![0x55; 32];
    let substituted = must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([2; 32]),
        CORE_BYTES,
        &encoded(&substituted_profile),
        "observe",
        &input,
    ));
    assert_ne!(
        baseline.request().operation_id,
        substituted.request().operation_id
    );
}

#[test]
fn target_request_must_be_derived_from_the_supplied_core() {
    let mut target = target_ir_value();
    *map_field_mut(target_request_mut(&mut target), "input") = map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bytes")),
                ("value", CanonicalValueV1::Bytes(vec![0x44])),
            ]),
        ),
    ]);
    let input = application_input(
        vec![0x44],
        digest("scope:target-derivation"),
        digest("basis:target-derivation"),
        65_536,
    );
    assert!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([5; 32]),
            CORE_BYTES,
            &encoded(&target),
            "observe",
            &input,
        )
        .is_err(),
        "Target IR request fields must be corroborated against Core"
    );
}

#[test]
fn expression_evaluation_obeys_declared_and_host_budgets() {
    for (index, (field, limit)) in [
        ("maxSteps", 1_i128),
        ("maxAllocatedBytes", 1_i128),
        ("maxOutputBytes", 1_i128),
        ("maxSteps", i128::from(u64::MAX)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut core = core_value();
        *map_field_mut(
            map_field_mut(intent_mut(&mut core, "observe"), "coreEvaluationBudget"),
            field,
        ) = CanonicalValueV1::Integer(limit);
        let mut target = target_ir_value();
        *map_field_mut(
            map_field_mut(intent_mut(&mut target, "observe"), "coreEvaluationBudget"),
            field,
        ) = CanonicalValueV1::Integer(limit);
        bind_target_to_core(&mut target, &core);
        let operation_input = must_ok(encode_bounded_workspace_observation_input_v1([
            "metered.txt".to_owned(),
        ]));
        assert!(
            admit_edict_external_action_request_v1(
                WorldlineId::from_bytes([u8::try_from(index + 10).unwrap_or(u8::MAX); 32]),
                &encoded(&core),
                &encoded(&target),
                "observe",
                &application_input(
                    operation_input,
                    digest("scope:metered"),
                    digest("basis:metered"),
                    65_536,
                ),
            )
            .is_err(),
            "{field}={limit} must fail closed"
        );
    }
}

#[test]
fn request_budget_must_fit_a_terminal_settlement() {
    let operation_input = must_ok(encode_bounded_workspace_observation_input_v1([
        "terminal.txt".to_owned(),
    ]));
    assert!(
        admit_edict_external_action_request_v1(
            WorldlineId::from_bytes([20; 32]),
            CORE_BYTES,
            TARGET_IR_BYTES,
            "observe",
            &application_input(
                operation_input,
                digest("scope:terminal"),
                digest("basis:terminal"),
                1,
            ),
        )
        .is_err(),
        "a request must not be admitted when no terminal envelope can fit"
    );
}

#[test]
fn malformed_operation_input_settles_as_rejected() {
    let root = TempRoot::new("malformed-input");
    let admitted = must_ok(admit_edict_external_action_request_v1(
        WorldlineId::from_bytes([21; 32]),
        CORE_BYTES,
        TARGET_IR_BYTES,
        "observe",
        &application_input(
            vec![0xff],
            digest("scope:malformed-input"),
            digest("basis:malformed-input"),
            65_536,
        ),
    ));
    let adapter = adapter(&root, &admitted, ["unused.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "malformed-input",
    );
    assert_eq!(
        must_ok(adapter.observe(&grant, &admitted)).kind,
        ExternalActionSettlementKindV1::Rejected
    );
}

#[test]
fn settlement_admission_revalidates_the_adapter_profile() {
    let root = TempRoot::new("settlement-profile");
    root.write("profile.txt", b"profile");
    let basis = bounded_workspace_observation_basis_v1([("profile.txt", b"profile".as_slice())]);
    let admitted = admitted_request(
        22,
        ["profile.txt".to_owned()],
        digest("scope:settlement-profile"),
        basis,
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["profile.txt".to_owned()]);
    let mut mismatched_profile = profile(&admitted, "bounded-observation:adapter");
    mismatched_profile.operation_id =
        warp_core::external_action::ExternalActionOperationIdV1::from_hash(digest(
            "other-operation",
        ));
    let mismatched = must_ok(BoundedWorkspaceObservationAdapterV1::open(
        root.path(),
        ["profile.txt".to_owned()],
        mismatched_profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "settlement-profile",
    );
    let candidate = must_ok(adapter.observe(&grant, &admitted));
    assert_eq!(
        mismatched.admit_settlement(
            &mut store,
            &mut coordinator,
            context("settlement-profile:settlement"),
            &admitted,
            grant,
            candidate,
        ),
        Err(BoundedWorkspaceObservationErrorV1::ProfileMismatch)
    );
}

#[test]
fn successful_settlement_paths_must_equal_the_requested_aperture() {
    let root = TempRoot::new("settlement-aperture");
    let secret = b"secret";
    let basis = bounded_workspace_observation_basis_v1([("secret.txt", secret.as_slice())]);
    let admitted = admitted_request(
        23,
        ["allowed.txt".to_owned()],
        digest("scope:settlement-aperture"),
        basis,
        65_536,
    );
    let adapter = adapter(
        &root,
        &admitted,
        ["allowed.txt".to_owned(), "secret.txt".to_owned()],
    );
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "settlement-aperture",
    );
    let settlement = encoded(&map([
        ("kind", text("boundedWorkspaceObservationSettlement")),
        ("posture", text("succeeded")),
        ("basis", CanonicalValueV1::Bytes(basis.to_vec())),
        ("evidence", CanonicalValueV1::Bytes(basis.to_vec())),
        (
            "files",
            CanonicalValueV1::Array(vec![map([
                ("path", text("secret.txt")),
                ("bytes", CanonicalValueV1::Bytes(secret.to_vec())),
                (
                    "digest",
                    CanonicalValueV1::Bytes(blake3::hash(secret).as_bytes().to_vec()),
                ),
            ])]),
        ),
        ("obstruction", CanonicalValueV1::Null),
    ]));
    let request = admitted.request();
    let candidate = ExternalActionSettlementCandidateV1::new(
        request.request_id(),
        grant.claim().attempt_id,
        adapter.adapter_binding().adapter_id,
        ExternalActionSettlementKindV1::Succeeded,
        request.settlement_schema_digest,
        basis,
        settlement.clone(),
        schema_admission_evidence(request.settlement_schema_digest, &settlement),
        basis,
    );
    assert_eq!(
        adapter.admit_settlement(
            &mut store,
            &mut coordinator,
            context("settlement-aperture:settlement"),
            &admitted,
            grant,
            candidate,
        ),
        Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)
    );
}

#[test]
fn interior_current_directory_segments_are_not_canonical_paths() {
    let root = TempRoot::new("dot-segment");
    let admitted = admitted_request(
        24,
        ["dir/file.txt".to_owned()],
        digest("scope:dot-segment"),
        digest("basis:dot-segment"),
        65_536,
    );
    assert!(matches!(
        BoundedWorkspaceObservationAdapterV1::open(
            root.path(),
            ["dir/./file.txt".to_owned()],
            profile(&admitted, "bounded-observation:dot-segment"),
        ),
        Err(BoundedWorkspaceObservationErrorV1::InvalidPath)
    ));
}

#[cfg(unix)]
#[test]
fn special_files_are_rejected_before_a_read_attempt() {
    use std::os::unix::net::UnixListener;

    let root = TempRoot::new("special-file");
    let _listener = must_ok(UnixListener::bind(root.path().join("socket")));
    let admitted = admitted_request(
        25,
        ["socket".to_owned()],
        digest("scope:special-file"),
        digest("basis:special-file"),
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["socket".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "special-file",
    );
    assert_eq!(
        must_ok(adapter.observe(&grant, &admitted)).kind,
        ExternalActionSettlementKindV1::Rejected
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
        &admitted,
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
            must_ok(u8::try_from(index + 10)),
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
            &admitted,
            grant,
            candidate,
        ));
    }
}

#[test]
fn duplicate_paths_settle_as_rejected() {
    let root = TempRoot::new("duplicate-path");
    root.write("allowed.txt", b"allowed");
    let admitted = admitted_request(
        19,
        ["allowed.txt".to_owned(), "allowed.txt".to_owned()],
        digest("scope:duplicate-path"),
        digest("basis:duplicate-path"),
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
        "duplicate-path",
    );
    assert_eq!(
        must_ok(adapter.observe(&grant, &admitted)).kind,
        ExternalActionSettlementKindV1::Rejected
    );
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
    let exact_budget = must_ok(u64::try_from(measured.canonical_result_bytes.len()));

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
fn aggregate_file_bytes_cannot_exceed_the_request_budget() {
    let root = TempRoot::new("aggregate-budget");
    let left = vec![0x4c; 3_000];
    let right = vec![0x52; 3_000];
    root.write("left.bin", &left);
    root.write("right.bin", &right);
    let basis = bounded_workspace_observation_basis_v1([
        ("left.bin", left.as_slice()),
        ("right.bin", right.as_slice()),
    ]);
    let admitted = admitted_request(
        33,
        ["left.bin".to_owned(), "right.bin".to_owned()],
        digest("scope:aggregate-budget"),
        basis,
        5_000,
    );
    let adapter = adapter(
        &root,
        &admitted,
        ["left.bin".to_owned(), "right.bin".to_owned()],
    );
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "aggregate-budget",
    );
    assert_eq!(
        must_ok(adapter.observe(&grant, &admitted)).kind,
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
fn definite_io_failure_settles_and_recovers_as_failed() {
    let root = TempRoot::new("failed");
    let admitted = admitted_request(
        41,
        ["missing.txt".to_owned()],
        digest("scope:failed"),
        digest("basis:failed"),
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["missing.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(&mut store, &mut coordinator, &admitted, &adapter, "failed");
    let candidate = must_ok(adapter.observe(&grant, &admitted));
    assert_eq!(candidate.kind, ExternalActionSettlementKindV1::Failed);
    must_ok(adapter.admit_settlement(
        &mut store,
        &mut coordinator,
        context("failed:settlement"),
        &admitted,
        grant,
        candidate,
    ));
    let recovered = must_ok(ExternalActionCoordinatorV1::recover(&store));
    assert_eq!(
        must_some(
            recovered
                .observed_index()
                .get(admitted.request().request_id())
        )
        .posture,
        RecoveredExternalActionPostureV1::Settled(ExternalActionSettlementKindV1::Failed)
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
            &admitted,
            grant,
            candidate,
        ),
        Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)
    );
}

#[test]
fn noncanonical_success_file_order_cannot_bypass_the_profile_validator() {
    let root = TempRoot::new("settlement-order");
    root.write("a.txt", b"a");
    root.write("b.txt", b"b");
    let basis = bounded_workspace_observation_basis_v1([
        ("a.txt", b"a".as_slice()),
        ("b.txt", b"b".as_slice()),
    ]);
    let admitted = admitted_request(
        51,
        ["a.txt".to_owned(), "b.txt".to_owned()],
        digest("scope:settlement-order"),
        basis,
        65_536,
    );
    let adapter = adapter(&root, &admitted, ["a.txt".to_owned(), "b.txt".to_owned()]);
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "settlement-order",
    );
    let mut candidate = must_ok(adapter.observe(&grant, &admitted));
    let mut settlement = must_ok(decode_canonical_cbor_v1(&candidate.canonical_result_bytes));
    let CanonicalValueV1::Array(files) = map_field_mut(&mut settlement, "files") else {
        panic!("expected settlement files");
    };
    files.reverse();
    candidate.canonical_result_bytes = encoded(&settlement);
    candidate.declared_result_digest = blake3::hash(&candidate.canonical_result_bytes).into();
    assert_eq!(
        adapter.admit_settlement(
            &mut store,
            &mut coordinator,
            context("settlement-order:settlement"),
            &admitted,
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
        must_some(
            recovered_requested
                .observed_index()
                .get(request.request_id())
        )
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
        must_some(recovered_claimed.observed_index().get(request.request_id())).posture,
        RecoveredExternalActionPostureV1::Claimed
    );

    assert_eq!(
        adapter.outcome_unknown(&grant, &admitted, [0; 32]),
        Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)
    );
    let candidate =
        must_ok(adapter.outcome_unknown(&grant, &admitted, digest("recovery:ambiguous")));
    must_ok(adapter.admit_settlement(
        &mut store,
        &mut coordinator,
        context("recovery:settlement"),
        &admitted,
        grant,
        candidate,
    ));
    let recovered_settled = must_ok(ExternalActionCoordinatorV1::recover(&store));
    assert_eq!(
        must_some(recovered_settled.observed_index().get(request.request_id())).posture,
        RecoveredExternalActionPostureV1::Settled(ExternalActionSettlementKindV1::OutcomeUnknown)
    );
}

#[test]
fn outcome_unknown_settles_after_workspace_authority_disappears() {
    let root = TempRoot::new("rootless-unknown");
    let root_path = root.path().to_owned();
    let bytes = b"possibly observed";
    root.write("uncertain.txt", bytes);
    let admitted = admitted_request(
        61,
        ["uncertain.txt".to_owned()],
        digest("scope:rootless-unknown"),
        bounded_workspace_observation_basis_v1([("uncertain.txt", bytes.as_slice())]),
        65_536,
    );
    let runtime_profile = profile(&admitted, "bounded-observation:rootless-unknown");
    let adapter = must_ok(BoundedWorkspaceObservationAdapterV1::open(
        root.path(),
        ["uncertain.txt".to_owned()],
        runtime_profile,
    ));
    let mut store = store();
    let mut coordinator = must_ok(ExternalActionCoordinatorV1::recover(&store));
    let _grant = claim(
        &mut store,
        &mut coordinator,
        &admitted,
        &adapter,
        "rootless-unknown",
    );

    drop(adapter);
    drop(root);
    assert!(!root_path.exists());

    let reconciler = must_ok(BoundedWorkspaceObservationReconcilerV1::new(
        runtime_profile,
    ));
    let zero_evidence_grant = must_ok(coordinator.claim_grant(admitted.request().request_id()));
    assert_eq!(
        reconciler.admit_outcome_unknown(
            &mut store,
            &mut coordinator,
            context("rootless-unknown:zero-evidence"),
            &admitted,
            zero_evidence_grant,
            [0; 32],
        ),
        Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)
    );

    let mut substituted_profile = runtime_profile;
    substituted_profile.adapter_id =
        ExternalActionAdapterIdV1::from_hash(digest("rootless-unknown:substituted-adapter"));
    let substituted = must_ok(BoundedWorkspaceObservationReconcilerV1::new(
        substituted_profile,
    ));
    let substituted_grant = must_ok(coordinator.claim_grant(admitted.request().request_id()));
    assert_eq!(
        substituted.admit_outcome_unknown(
            &mut store,
            &mut coordinator,
            context("rootless-unknown:substituted-profile"),
            &admitted,
            substituted_grant,
            digest("rootless-unknown:ambiguous"),
        ),
        Err(BoundedWorkspaceObservationErrorV1::GrantMismatch)
    );
    assert_eq!(store.read_commits().len(), 2);

    let grant = must_ok(coordinator.claim_grant(admitted.request().request_id()));
    let settled = must_ok(reconciler.admit_outcome_unknown(
        &mut store,
        &mut coordinator,
        context("rootless-unknown:settlement"),
        &admitted,
        grant,
        digest("rootless-unknown:ambiguous"),
    ));
    assert_eq!(
        settled.settlement().kind,
        ExternalActionSettlementKindV1::OutcomeUnknown
    );
    assert_eq!(store.read_commits().len(), 3);
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
        &admitted,
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
        let length = must_ok(usize::try_from((state % 257) + 1));
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
            &admitted,
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
