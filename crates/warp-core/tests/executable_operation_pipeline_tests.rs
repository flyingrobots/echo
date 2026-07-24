#![allow(clippy::expect_used, clippy::panic)]
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! External-consumer witnesses for Echo-owned executable operation semantics.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use bytes::Bytes;
use echo_edict_canonical::{decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueV1};
use warp_core::causal_wal::{
    AffectedFrontier, AffectedFrontierKind, Lsn, PayloadCodecId, PayloadSchemaId,
    WalAppendAuthority, WalBuildError, WalDurabilityMode, WalRecordKind, WalSegmentId,
    WalTransactionBuilder, WalTransactionId, WalTransactionKind, WalValidationError, WriterEpochId,
};
use warp_core::{
    make_head_id, make_node_id, make_type_id, make_warp_id, AtomPayload, AttachmentValue,
    EchoOperationAdmissionErrorKindV1, EchoOperationAdmissionPolicyV1,
    EchoOperationApplicationBasisV1, EchoOperationArtifactErrorKindV1, EchoOperationBudgetV1,
    EchoOperationInvocationAdmissionErrorKindV1, EchoOperationInvocationAdmissionPolicyV1,
    EchoOperationInvocationV1, EchoOperationObstructionKindV1, EchoOperationPreparationV1,
    EchoOperationProgramV1, EchoOperationSemanticClosureV1, EchoOperationTerminalPostureV1,
    EngineBuilder, ExecutableOperationPackageV1, GraphStore, InboxPolicy, InstalledEchoOperationV1,
    NodeKey, NodeRecord, PlaybackMode, RuntimeWalActivationGap, SchedulerKind, TrustedRuntimeHost,
    TrustedRuntimeHostError, TrustedRuntimeWalConfig, TrustedRuntimeWalError, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

const OPERATION_COORDINATE: &str = "echo.fixture.SetAnchoredAtom.v1";
static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempWalDir(PathBuf);

impl TempWalDir {
    fn new() -> Self {
        let root = PathBuf::from("target").join("warp-core-test-tmp");
        fs::create_dir_all(&root).expect("the test WAL fixture root is creatable");
        for _ in 0..1024 {
            let ordinal = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!("echo-executable-operation-wal-{ordinal}"));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!(
                    "failed to create executable-operation WAL fixture {}: {error}",
                    path.display()
                ),
            }
        }
        panic!("exhausted deterministic executable-operation WAL fixture attempts");
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempWalDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn digest(label: &str) -> [u8; 32] {
    *blake3::hash(label.as_bytes()).as_bytes()
}

fn operation_wal_builder(
    label: &str,
    kind: WalTransactionKind,
    authority: WalAppendAuthority,
) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        WriterEpochId::from_hash(digest("operation-wal-writer")),
        WalSegmentId::from_raw(1),
        WalTransactionId::from_hash(digest(label)),
        kind,
        authority,
        Lsn::from_raw(0),
        digest("operation-wal-previous-frame"),
        digest("operation-wal-previous-commit"),
        WalDurabilityMode::StrictFilesystem,
        PayloadCodecId::from_hash(digest("operation-wal-codec")),
        PayloadSchemaId::from_hash(digest("operation-wal-schema")),
        1,
        1,
        digest("operation-wal-domain"),
    )
}

fn operation_frontier(kind: AffectedFrontierKind, label: &str) -> AffectedFrontier {
    AffectedFrontier {
        kind,
        before_digest: digest(&format!("{label}:before")),
        after_digest: digest(&format!("{label}:after")),
    }
}

fn fixture_host() -> (TrustedRuntimeHost, WriterHeadKey, NodeKey) {
    let warp_id = make_warp_id("operation-fixture");
    let node_id = make_node_id("operation-fixture-root");
    let node_type = make_type_id("operation-fixture-node");
    let attachment_type = make_type_id("operation-fixture-atom");
    let node = NodeKey {
        warp_id,
        local_id: node_id,
    };
    let mut store = GraphStore::new(warp_id);
    store.insert_node(node_id, NodeRecord { ty: node_type });
    store.set_node_attachment(
        node_id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"before"),
        ))),
    );
    let state = WorldlineState::from_root_store(store, node_id)
        .expect("the fixture state has one lawful root");
    let worldline_id = WorldlineId::from_bytes(digest("operation-fixture-worldline"));
    let head_key = WriterHeadKey {
        worldline_id,
        head_id: make_head_id("operation-fixture-writer"),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(worldline_id, state)
        .expect("the fixture worldline registers");
    runtime
        .register_writer_head(WriterHead::with_routing(
            head_key,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("the fixture writer registers");

    let mut engine_store = GraphStore::default();
    let engine_root = make_node_id("root");
    engine_store.insert_node(
        engine_root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    let engine = EngineBuilder::new(engine_store, engine_root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let host = TrustedRuntimeHost::new(runtime, engine)
        .expect("the trusted Echo runtime host initializes");
    (host, head_key, node)
}

/// Like [`fixture_host`], but the warp-scoped store also contains a second,
/// bare node -- present, but with no alpha attachment set -- at
/// `second_node_local_id`, typed `second_node_type`. Used to exercise
/// create-from-absence's ADR 0024 "existing node, absent attachment" and
/// "existing node, wrong type" refusal paths, which the standard fixture's
/// single fully-populated node cannot reach.
fn fixture_host_with_bare_node(
    second_node_local_id: warp_core::NodeId,
    second_node_type: warp_core::TypeId,
) -> (TrustedRuntimeHost, WriterHeadKey, NodeKey, NodeKey) {
    let warp_id = make_warp_id("operation-fixture");
    let node_id = make_node_id("operation-fixture-root");
    let node_type = make_type_id("operation-fixture-node");
    let attachment_type = make_type_id("operation-fixture-atom");
    let node = NodeKey {
        warp_id,
        local_id: node_id,
    };
    let second_node = NodeKey {
        warp_id,
        local_id: second_node_local_id,
    };
    let mut store = GraphStore::new(warp_id);
    store.insert_node(node_id, NodeRecord { ty: node_type });
    store.set_node_attachment(
        node_id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"before"),
        ))),
    );
    store.insert_node(
        second_node_local_id,
        NodeRecord {
            ty: second_node_type,
        },
    );
    let state = WorldlineState::from_root_store(store, node_id)
        .expect("the fixture state has one lawful root");
    let worldline_id = WorldlineId::from_bytes(digest("operation-fixture-worldline"));
    let head_key = WriterHeadKey {
        worldline_id,
        head_id: make_head_id("operation-fixture-writer"),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(worldline_id, state)
        .expect("the fixture worldline registers");
    runtime
        .register_writer_head(WriterHead::with_routing(
            head_key,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("the fixture writer registers");

    let mut engine_store = GraphStore::default();
    let engine_root = make_node_id("root");
    engine_store.insert_node(
        engine_root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    let engine = EngineBuilder::new(engine_store, engine_root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let host = TrustedRuntimeHost::new(runtime, engine)
        .expect("the trusted Echo runtime host initializes");
    (host, head_key, node, second_node)
}

fn semantic_closure() -> EchoOperationSemanticClosureV1 {
    EchoOperationSemanticClosureV1::new(
        digest("fixture-edict-source"),
        digest("edict-semantic-identity"),
        digest("fixture-edict-core"),
        digest("fixture-echo-target-ir"),
        "echo.fixture.GraphSchema.v1",
        digest("fixture-graph-schema"),
        "echo.fixture.Lawpack.v1",
        digest("fixture-lawpack"),
    )
}

fn operation_package(
    node_type: warp_core::TypeId,
    attachment_type: warp_core::TypeId,
) -> ExecutableOperationPackageV1 {
    operation_package_at(OPERATION_COORDINATE, node_type, attachment_type)
}

fn operation_package_at(
    operation_coordinate: &str,
    node_type: warp_core::TypeId,
    attachment_type: warp_core::TypeId,
) -> ExecutableOperationPackageV1 {
    ExecutableOperationPackageV1::new(
        operation_coordinate,
        semantic_closure(),
        warp_core::echo_operation_target_profile_identity_v1(),
        digest("fixture-authority-profile"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
            node_type,
            attachment_type,
            1_024,
        ),
    )
}

fn install_fixture_operation(host: &mut TrustedRuntimeHost) -> InstalledEchoOperationV1 {
    install_fixture_operation_at(host, OPERATION_COORDINATE)
}

fn install_fixture_operation_at(
    host: &mut TrustedRuntimeHost,
    operation_coordinate: &str,
) -> InstalledEchoOperationV1 {
    let package_bytes = operation_package_at(
        operation_coordinate,
        make_type_id("operation-fixture-node"),
        make_type_id("operation-fixture-atom"),
    )
    .to_canonical_bytes()
    .expect("fixture package encodes");
    let package_id = warp_core::echo_operation_package_id_v1(&package_bytes);
    let admitted = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                package_id,
                operation_coordinate,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            package_bytes,
        )
        .expect("the exact fixture package is admitted");
    host.install_admitted_echo_operation_package_v1(admitted)
        .expect("the admitted fixture package installs")
}

fn application_basis() -> EchoOperationApplicationBasisV1 {
    warp_core::echo_operation_anchored_node_application_basis_v1(
        NodeKey {
            warp_id: make_warp_id("operation-fixture"),
            local_id: make_node_id("operation-fixture-root"),
        },
        make_type_id("operation-fixture-atom"),
        b"before",
    )
}

fn invocation_policy() -> EchoOperationInvocationAdmissionPolicyV1 {
    EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    )
}

fn canonical_invocation(
    installed: &InstalledEchoOperationV1,
    basis: warp_core::EchoOperationEvaluationBasisV1,
    node: NodeKey,
    expected_value_digest: [u8; 32],
    replacement: Vec<u8>,
    budget: EchoOperationBudgetV1,
) -> Vec<u8> {
    EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        installed.operation_coordinate(),
        basis,
        digest("fixture-authority-grant"),
        budget,
        node,
        Some(expected_value_digest),
        replacement,
    )
    .to_canonical_bytes()
    .expect("fixture invocation encodes canonically")
}

fn replace_canonical_map_field(
    bytes: &[u8],
    field_name: &str,
    replacement: CanonicalValueV1,
) -> Vec<u8> {
    let value = decode_canonical_cbor_v1(bytes).expect("fixture bytes decode");
    let CanonicalValueV1::Map(mut fields) = value else {
        panic!("fixture value must be a canonical map");
    };
    let field = fields
        .iter_mut()
        .find(|(key, _)| key == &CanonicalValueV1::Text(field_name.to_owned()))
        .expect("fixture field exists");
    field.1 = replacement;
    encode_canonical_cbor_v1(&CanonicalValueV1::Map(fields))
        .expect("modified fixture map re-encodes canonically")
}

fn canonical_map_bytes_field(bytes: &[u8], field_name: &str) -> Vec<u8> {
    let value = decode_canonical_cbor_v1(bytes).expect("fixture bytes decode");
    let CanonicalValueV1::Map(fields) = value else {
        panic!("fixture value must be a canonical map");
    };
    let value = fields
        .into_iter()
        .find_map(|(key, value)| {
            (key == CanonicalValueV1::Text(field_name.to_owned())).then_some(value)
        })
        .expect("fixture field exists");
    let CanonicalValueV1::Bytes(value) = value else {
        panic!("fixture field must carry canonical bytes");
    };
    value
}

#[test]
fn admitted_data_only_program_commits_one_typed_attachment_patch() {
    let (mut host, head_key, node) = fixture_host();
    let node_type = make_type_id("operation-fixture-node");
    let attachment_type = make_type_id("operation-fixture-atom");
    let package = operation_package(node_type, attachment_type);
    let package_bytes = package
        .to_canonical_bytes()
        .expect("package has one canonical Edict encoding");
    let package_id = warp_core::echo_operation_package_id_v1(&package_bytes);
    let package_policy = EchoOperationAdmissionPolicyV1::exact(
        package_id,
        OPERATION_COORDINATE,
        digest("fixture-authority-profile"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted_package = host
        .admit_echo_operation_package_v1(&package_policy, package_bytes)
        .expect("the independently pinned package is admitted");
    let installed = host
        .install_admitted_echo_operation_package_v1(admitted_package)
        .expect("admitted executable meaning installs without callbacks");

    let application_basis = application_basis();
    let evaluation_basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis)
        .expect("Echo resolves the exact current parent basis");
    let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        installed.operation_coordinate(),
        evaluation_basis,
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        node,
        Some(warp_core::echo_operation_atom_value_digest_v1(
            attachment_type,
            b"before",
        )),
        b"after".to_vec(),
    );
    let invocation_id = invocation
        .identity()
        .expect("the generated-style invocation has one identity");
    let invocation_bytes = invocation
        .to_canonical_bytes()
        .expect("the generated-style invocation is canonical");
    let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted_invocation = host
        .admit_echo_operation_invocation_v1(&invocation_policy, &invocation_bytes)
        .expect("Echo independently admits the installed operation invocation");
    let preparation = host.prepare_echo_operation_v1(admitted_invocation);
    let EchoOperationPreparationV1::Prepared(prepared) = preparation else {
        panic!("the lawful fixture invocation must prepare");
    };
    assert_eq!(prepared.evaluation_basis(), &evaluation_basis);
    assert_eq!(prepared.declared_footprint(), prepared.actual_footprint());
    let invocation_admission_id = prepared.invocation_admission_id();
    let private_evaluation_id = prepared.private_evaluation_id();
    let preparation_id = prepared.preparation_id();
    let result_id = prepared.result_id();
    let prepared_patch_digest = prepared.patch().digest();

    let execution = host
        .commit_prepared_echo_operation_v1(prepared)
        .expect("Echo commits one privately evaluated patch");
    assert_eq!(
        execution.receipt().terminal_posture(),
        EchoOperationTerminalPostureV1::Committed
    );
    assert_eq!(execution.receipt().package_id(), package_id);
    assert_eq!(
        execution.receipt().package_admission_id(),
        installed.package_admission_id()
    );
    assert_eq!(
        execution.receipt().installed_operation_id(),
        installed.installed_operation_id()
    );
    assert_eq!(
        execution.receipt().operation_coordinate(),
        OPERATION_COORDINATE
    );
    assert_eq!(execution.receipt().invocation_id(), invocation_id);
    assert_ne!(
        execution.receipt().invocation_bytes_digest(),
        invocation_id.as_hash(),
        "the exact-bytes digest must remain domain-separated from invocation identity"
    );
    assert_ne!(
        execution.receipt().package_id().as_hash(),
        execution.receipt().program_id().as_hash(),
        "a subordinate program digest must not collapse package identity"
    );
    assert_eq!(
        execution.receipt().evaluation_basis_id(),
        evaluation_basis.identity()
    );
    assert_eq!(execution.receipt().evaluation_basis(), evaluation_basis);
    assert_eq!(execution.receipt().program_id(), installed.program_id());
    assert_eq!(
        execution
            .committed_patch()
            .expect("the committed outcome carries its exact patch")
            .rule_pack_id(),
        installed.installed_operation_id().as_hash(),
        "the causal patch must bind the admitted installation, not a naked program"
    );
    assert_eq!(
        execution
            .tick_receipt()
            .expect("the committed outcome carries its singleton tick receipt")
            .entries()[0]
            .rule_id,
        installed.installed_operation_id().as_hash(),
        "generic tick evidence must not promote a program digest to rule authority"
    );
    assert_eq!(
        execution.receipt().declared_footprint_digest(),
        execution.receipt().actual_footprint_digest()
    );
    assert_eq!(
        execution.receipt().delegated_budget(),
        EchoOperationBudgetV1::new(16, 4_096, 4_096)
    );
    assert_eq!(
        execution.receipt().consumed_budget(),
        EchoOperationBudgetV1::new(4, 70, 37)
    );
    assert_eq!(
        execution.receipt().invocation_admission_id(),
        invocation_admission_id
    );
    assert_eq!(
        execution.receipt().private_evaluation_id(),
        private_evaluation_id
    );
    assert_eq!(execution.receipt().preparation_id(), preparation_id);
    assert_eq!(execution.receipt().prepared_result_id(), result_id);
    assert_eq!(execution.receipt().committed_result_id(), Some(result_id));
    assert_eq!(
        execution.receipt().prepared_patch_digest(),
        prepared_patch_digest
    );
    assert_eq!(
        execution.receipt().committed_patch_digest(),
        Some(prepared_patch_digest)
    );
    assert!(execution.receipt().composition_digest().is_some());
    assert_ne!(execution.receipt().terminal_outcome_digest(), [0; 32]);
    assert_eq!(
        execution.receipt().state_root_before(),
        evaluation_basis.state_root()
    );
    assert_ne!(execution.receipt().commit_id(), [0; 32]);
    assert_eq!(
        execution
            .receipt()
            .commit_global_tick()
            .expect("a committed operation receives a global tick")
            .as_u64(),
        1
    );
    assert_eq!(execution.receipt().worldline_tick_after().as_u64(), 1);

    let state = host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the committed worldline remains registered")
        .state();
    assert_eq!(state.current_tick().as_u64(), 1);
    assert_eq!(execution.receipt().state_root_after(), state.state_root());
    assert_eq!(
        state
            .store(&node.warp_id)
            .and_then(|store| store.node_attachment(&node.local_id)),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"after"),
        )))
    );
}

#[test]
fn a_prepared_operation_cannot_commit_after_its_parent_basis_changes() {
    let (mut host, head_key, node) = fixture_host();
    let node_type = make_type_id("operation-fixture-node");
    let attachment_type = make_type_id("operation-fixture-atom");
    let package_bytes = operation_package(node_type, attachment_type)
        .to_canonical_bytes()
        .expect("package encodes");
    let package_id = warp_core::echo_operation_package_id_v1(&package_bytes);
    let admitted_package = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                package_id,
                OPERATION_COORDINATE,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            package_bytes,
        )
        .expect("the exact package is admitted");
    let installed = host
        .install_admitted_echo_operation_package_v1(admitted_package)
        .expect("the exact package installs");
    let application_basis = application_basis();

    let prepare = |host: &TrustedRuntimeHost, replacement: &[u8]| {
        let basis = host
            .echo_operation_evaluation_basis_v1(head_key, application_basis)
            .expect("Echo resolves the current basis");
        let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
            installed.package_id(),
            installed.operation_coordinate(),
            basis,
            digest("fixture-authority-grant"),
            EchoOperationBudgetV1::new(16, 4_096, 4_096),
            node,
            Some(warp_core::echo_operation_atom_value_digest_v1(
                attachment_type,
                b"before",
            )),
            replacement.to_vec(),
        );
        let bytes = invocation.to_canonical_bytes().expect("invocation encodes");
        let admitted = host
            .admit_echo_operation_invocation_v1(
                &EchoOperationInvocationAdmissionPolicyV1::new(
                    digest("fixture-authority-profile"),
                    digest("fixture-authority-grant"),
                    EchoOperationBudgetV1::new(16, 4_096, 4_096),
                ),
                &bytes,
            )
            .expect("invocation is admitted");
        match host.prepare_echo_operation_v1(admitted) {
            EchoOperationPreparationV1::Prepared(prepared) => prepared,
            EchoOperationPreparationV1::Obstructed(obstruction) => {
                panic!("lawful invocation obstructed: {obstruction:?}")
            }
        }
    };

    let stale_preparation = prepare(&host, b"stale-result");
    let winning_preparation = prepare(&host, b"winning-result");
    host.commit_prepared_echo_operation_v1(winning_preparation)
        .expect("one exact-basis operation commits");
    let evidence = host
        .commit_prepared_echo_operation_v1(stale_preparation)
        .expect("basis refusal is typed evidence, not a host fault");
    assert_eq!(
        evidence.receipt().terminal_posture(),
        EchoOperationTerminalPostureV1::NotCommittedBasisChanged
    );
    assert_eq!(evidence.receipt().committed_patch_digest(), None);
    assert_eq!(evidence.receipt().committed_result_id(), None);
    assert_eq!(evidence.receipt().composition_digest(), None);

    let state = host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the worldline remains registered")
        .state();
    assert_eq!(state.current_tick().as_u64(), 1);
    assert_eq!(
        state
            .store(&node.warp_id)
            .and_then(|store| store.node_attachment(&node.local_id)),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"winning-result"),
        )))
    );
}

#[test]
fn a_prepared_program_cannot_substitute_for_installation_authority() {
    let (mut evaluating_host, head_key, node) = fixture_host();
    let installed = install_fixture_operation(&mut evaluating_host);
    let basis = evaluating_host
        .echo_operation_evaluation_basis_v1(head_key, application_basis())
        .expect("Echo resolves the evaluation basis");
    let bytes = canonical_invocation(
        &installed,
        basis,
        node,
        warp_core::echo_operation_atom_value_digest_v1(
            make_type_id("operation-fixture-atom"),
            b"before",
        ),
        b"uninstalled-after".to_vec(),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted = evaluating_host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &bytes)
        .expect("the installed operation invocation is admitted");
    let EchoOperationPreparationV1::Prepared(prepared) =
        evaluating_host.prepare_echo_operation_v1(admitted)
    else {
        panic!("the installed operation must prepare");
    };
    let foreign_prepared = prepared.clone();

    let (mut uninstalled_host, _, _) = fixture_host();
    let evidence = uninstalled_host
        .commit_prepared_echo_operation_v1(prepared)
        .expect("missing installation produces typed noncommit evidence");
    assert_eq!(
        evidence.receipt().terminal_posture(),
        EchoOperationTerminalPostureV1::NotCommittedInstallationUnavailable
    );
    assert_eq!(evidence.receipt().committed_patch_digest(), None);
    assert_eq!(evidence.receipt().committed_result_id(), None);
    assert_eq!(evidence.receipt().composition_digest(), None);
    let state = uninstalled_host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the parent worldline remains registered")
        .state();
    assert_eq!(state.current_tick().as_u64(), 0);
    assert_eq!(
        state
            .store(&node.warp_id)
            .and_then(|store| store.node_attachment(&node.local_id)),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            make_type_id("operation-fixture-atom"),
            Bytes::from_static(b"before"),
        )))
    );

    let (mut foreign_host, _, _) = fixture_host();
    let foreign_installation = install_fixture_operation(&mut foreign_host);
    assert_eq!(
        foreign_installation.installed_operation_id(),
        installed.installed_operation_id(),
        "identical retained installation evidence must remain deterministic"
    );
    let evidence = foreign_host
        .commit_prepared_echo_operation_v1(foreign_prepared)
        .expect("foreign runtime ownership produces typed noncommit evidence");
    assert_eq!(
        evidence.receipt().terminal_posture(),
        EchoOperationTerminalPostureV1::NotCommittedEvaluationAuthorityMismatch
    );
    assert_eq!(evidence.receipt().committed_patch_digest(), None);
}

#[test]
fn identical_program_bytes_do_not_merge_admitted_operation_identities() {
    const ALTERNATE_COORDINATE: &str = "echo.fixture.SetAnchoredAtomAlternate.v1";

    let (mut first_host, first_head, first_node) = fixture_host();
    let first_installed = install_fixture_operation_at(&mut first_host, OPERATION_COORDINATE);
    let (mut second_host, second_head, second_node) = fixture_host();
    let second_installed = install_fixture_operation_at(&mut second_host, ALTERNATE_COORDINATE);

    assert_eq!(first_installed.program_id(), second_installed.program_id());
    assert_ne!(first_installed.package_id(), second_installed.package_id());
    assert_ne!(
        first_installed.installed_operation_id(),
        second_installed.installed_operation_id()
    );

    let execute =
        |host: &mut TrustedRuntimeHost, installed: &InstalledEchoOperationV1, head, node| {
            let basis = host
                .echo_operation_evaluation_basis_v1(head, application_basis())
                .expect("Echo resolves the exact package-specific basis");
            let invocation = canonical_invocation(
                installed,
                basis,
                node,
                warp_core::echo_operation_atom_value_digest_v1(
                    make_type_id("operation-fixture-atom"),
                    b"before",
                ),
                b"same-consequence".to_vec(),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            );
            let admitted = host
                .admit_echo_operation_invocation_v1(&invocation_policy(), &invocation)
                .expect("the exact package-specific invocation is admitted");
            let EchoOperationPreparationV1::Prepared(prepared) =
                host.prepare_echo_operation_v1(admitted)
            else {
                panic!("the exact package-specific invocation must prepare");
            };
            host.commit_prepared_echo_operation_v1(prepared)
                .expect("the exact package-specific preparation commits")
        };

    let first = execute(&mut first_host, &first_installed, first_head, first_node);
    let second = execute(
        &mut second_host,
        &second_installed,
        second_head,
        second_node,
    );
    assert_eq!(
        first.receipt().state_root_after(),
        second.receipt().state_root_after()
    );
    assert_ne!(first.committed_patch(), second.committed_patch());
    assert_ne!(first.receipt().commit_id(), second.receipt().commit_id());
    assert_eq!(
        first
            .committed_patch()
            .expect("first operation committed")
            .rule_pack_id(),
        first_installed.installed_operation_id().as_hash()
    );
    assert_eq!(
        second
            .committed_patch()
            .expect("second operation committed")
            .rule_pack_id(),
        second_installed.installed_operation_id().as_hash()
    );
}

#[test]
fn package_admission_rejects_malformed_unsupported_mismatched_and_over_budget_bytes() {
    let (host, _, _) = fixture_host();
    let package = operation_package(
        make_type_id("operation-fixture-node"),
        make_type_id("operation-fixture-atom"),
    );
    let package_bytes = package
        .to_canonical_bytes()
        .expect("fixture package encodes");
    let package_id = warp_core::echo_operation_package_id_v1(&package_bytes);

    let digest_mismatch = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                warp_core::echo_operation_package_id_v1(b"different package bytes"),
                OPERATION_COORDINATE,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            package_bytes.clone(),
        )
        .expect_err("a package digest cannot confer a match on different bytes");
    assert_eq!(
        digest_mismatch.kind(),
        EchoOperationAdmissionErrorKindV1::PackageIdentityMismatch
    );

    let malformed_bytes = vec![0xff];
    let malformed = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                warp_core::echo_operation_package_id_v1(&malformed_bytes),
                OPERATION_COORDINATE,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            malformed_bytes,
        )
        .expect_err("malformed canonical bytes fail before installation");
    assert_eq!(
        malformed.kind(),
        EchoOperationAdmissionErrorKindV1::ArtifactInvalid
    );

    let unsupported_target_bytes = ExecutableOperationPackageV1::new(
        OPERATION_COORDINATE,
        semantic_closure(),
        digest("unsupported-target-profile"),
        digest("fixture-authority-profile"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
            make_type_id("operation-fixture-node"),
            make_type_id("operation-fixture-atom"),
            1_024,
        ),
    )
    .to_canonical_bytes()
    .expect("unsupported target package still has canonical source bytes");
    let unsupported_target = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                warp_core::echo_operation_package_id_v1(&unsupported_target_bytes),
                OPERATION_COORDINATE,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            unsupported_target_bytes,
        )
        .expect_err("an unimplemented target profile fails closed");
    assert_eq!(
        unsupported_target
            .artifact()
            .expect("the target refusal retains its artifact cause")
            .kind(),
        EchoOperationArtifactErrorKindV1::UnsupportedTargetProfile
    );

    let program_bytes = canonical_map_bytes_field(&package_bytes, "program");
    let unsupported_program_bytes = replace_canonical_map_field(
        &program_bytes,
        "intrinsic_profile_identity",
        CanonicalValueV1::Bytes(digest("unsupported-program-intrinsics").to_vec()),
    );
    let unsupported_program_package = replace_canonical_map_field(
        &package_bytes,
        "program",
        CanonicalValueV1::Bytes(unsupported_program_bytes),
    );
    let unsupported_program = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                warp_core::echo_operation_package_id_v1(&unsupported_program_package),
                OPERATION_COORDINATE,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            unsupported_program_package,
        )
        .expect_err("program bytes must bind the supported intrinsic profile directly");
    assert_eq!(
        unsupported_program
            .artifact()
            .expect("the program-profile refusal retains its artifact cause")
            .kind(),
        EchoOperationArtifactErrorKindV1::UnsupportedTargetProfile
    );

    let unsupported_schema_bytes = replace_canonical_map_field(
        &package_bytes,
        "input_schema_identity",
        CanonicalValueV1::Bytes(digest("unsupported-input-schema").to_vec()),
    );
    let unsupported_schema = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                warp_core::echo_operation_package_id_v1(&unsupported_schema_bytes),
                OPERATION_COORDINATE,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            unsupported_schema_bytes,
        )
        .expect_err("an unimplemented input schema fails closed");
    assert_eq!(
        unsupported_schema
            .artifact()
            .expect("the schema refusal retains its artifact cause")
            .kind(),
        EchoOperationArtifactErrorKindV1::UnsupportedSchema
    );

    let authority_mismatch = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                package_id,
                OPERATION_COORDINATE,
                digest("different-authority-profile"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            package_bytes.clone(),
        )
        .expect_err("package identity cannot substitute for authority policy");
    assert_eq!(
        authority_mismatch.kind(),
        EchoOperationAdmissionErrorKindV1::AuthorityProfileMismatch
    );

    let over_budget = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                package_id,
                OPERATION_COORDINATE,
                digest("fixture-authority-profile"),
                EchoOperationBudgetV1::new(15, 4_096, 4_096),
            ),
            package_bytes,
        )
        .expect_err("a package cannot widen the runtime-owned budget ceiling");
    assert_eq!(
        over_budget.kind(),
        EchoOperationAdmissionErrorKindV1::BudgetExceedsPolicy
    );

    let impossible_package = ExecutableOperationPackageV1::new(
        OPERATION_COORDINATE,
        semantic_closure(),
        warp_core::echo_operation_target_profile_identity_v1(),
        digest("fixture-authority-profile"),
        EchoOperationBudgetV1::new(1, 1, 1),
        EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
            make_type_id("operation-fixture-node"),
            make_type_id("operation-fixture-atom"),
            1_024,
        ),
    )
    .to_canonical_bytes()
    .expect_err("a package budget must permit the smallest lawful evaluation");
    assert_eq!(
        impossible_package.kind(),
        EchoOperationArtifactErrorKindV1::InvalidBudget
    );
}

#[test]
fn invocation_admission_keeps_contract_authority_budget_and_basis_separate() {
    let (mut host, head_key, node) = fixture_host();
    let installed = install_fixture_operation(&mut host);
    let basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis())
        .expect("Echo resolves the exact invocation basis");
    let valid_bytes = canonical_invocation(
        &installed,
        basis,
        node,
        warp_core::echo_operation_atom_value_digest_v1(
            make_type_id("operation-fixture-atom"),
            b"before",
        ),
        b"after".to_vec(),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );

    let malformed = host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &[0xff])
        .expect_err("malformed invocation bytes fail before routing");
    assert_eq!(
        malformed.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::MalformedInvocation
    );

    let wrong_profile = host
        .admit_echo_operation_invocation_v1(
            &EchoOperationInvocationAdmissionPolicyV1::new(
                digest("wrong-authority-profile"),
                digest("fixture-authority-grant"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            &valid_bytes,
        )
        .expect_err("runtime authority policy remains independent of package identity");
    assert_eq!(
        wrong_profile.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::AuthorityProfileMismatch
    );

    let wrong_grant = host
        .admit_echo_operation_invocation_v1(
            &EchoOperationInvocationAdmissionPolicyV1::new(
                digest("fixture-authority-profile"),
                digest("wrong-authority-grant"),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            &valid_bytes,
        )
        .expect_err("the operation package does not confer an invocation grant");
    assert_eq!(
        wrong_grant.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::AuthorityGrantMismatch
    );

    let over_budget_bytes = canonical_invocation(
        &installed,
        basis,
        node,
        warp_core::echo_operation_atom_value_digest_v1(
            make_type_id("operation-fixture-atom"),
            b"before",
        ),
        b"after".to_vec(),
        EchoOperationBudgetV1::new(17, 4_096, 4_096),
    );
    let over_budget = host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &over_budget_bytes)
        .expect_err("delegation cannot exceed package or runtime policy");
    assert_eq!(
        over_budget.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::BudgetExceeded
    );

    let below_minimum_bytes = canonical_invocation(
        &installed,
        basis,
        node,
        warp_core::echo_operation_atom_value_digest_v1(
            make_type_id("operation-fixture-atom"),
            b"before",
        ),
        b"after".to_vec(),
        EchoOperationBudgetV1::new(1, 1, 1),
    );
    let below_minimum = host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &below_minimum_bytes)
        .expect_err("delegation below the program minimum fails during admission");
    assert_eq!(
        below_minimum.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::BudgetExceeded
    );

    let unbounded_basis_read_bytes = canonical_invocation(
        &installed,
        basis,
        node,
        warp_core::echo_operation_atom_value_digest_v1(
            make_type_id("operation-fixture-atom"),
            b"before",
        ),
        Vec::new(),
        EchoOperationBudgetV1::new(4, 64, 32),
    );
    let unbounded_basis_read = host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &unbounded_basis_read_bytes)
        .expect_err("basis corroboration must fit inside the delegated read budget");
    assert_eq!(
        unbounded_basis_read.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::BudgetExceeded
    );

    let wrong_operation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        "echo.fixture.DifferentOperation.v1",
        basis,
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        node,
        Some(warp_core::echo_operation_atom_value_digest_v1(
            make_type_id("operation-fixture-atom"),
            b"before",
        )),
        b"after".to_vec(),
    )
    .to_canonical_bytes()
    .expect("the structurally valid wrong operation invocation encodes");
    let wrong_operation = host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &wrong_operation)
        .expect_err("a package digest does not confer another operation coordinate");
    assert_eq!(
        wrong_operation.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::OperationCoordinateMismatch
    );

    let uncorroborated_basis = host
        .echo_operation_evaluation_basis_v1(
            head_key,
            EchoOperationApplicationBasisV1::new(
                digest("invented-application-basis-schema"),
                digest("invented-application-basis-value"),
            ),
        )
        .expect("a client may claim basis bytes before invocation admission");
    let uncorroborated_basis_bytes = canonical_invocation(
        &installed,
        uncorroborated_basis,
        node,
        warp_core::echo_operation_atom_value_digest_v1(
            make_type_id("operation-fixture-atom"),
            b"before",
        ),
        b"after".to_vec(),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let uncorroborated_basis = host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &uncorroborated_basis_bytes)
        .expect_err("Echo must corroborate the application basis from current graph state");
    assert_eq!(
        uncorroborated_basis.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch
    );
}

#[test]
fn private_evaluation_returns_typed_obstructions_without_parent_mutation() {
    let (mut host, head_key, node) = fixture_host();
    let installed = install_fixture_operation(&mut host);
    let basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis())
        .expect("Echo resolves the exact invocation basis");
    let state_root_before = host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the fixture worldline exists")
        .state()
        .state_root();
    let attachment_type = make_type_id("operation-fixture-atom");

    let cases = [
        (
            canonical_invocation(
                &installed,
                basis,
                node,
                digest("wrong-value-precondition"),
                b"after".to_vec(),
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            EchoOperationObstructionKindV1::PreconditionMismatch,
        ),
        (
            canonical_invocation(
                &installed,
                basis,
                node,
                warp_core::echo_operation_atom_value_digest_v1(attachment_type, b"before"),
                vec![0x61; 1_025],
                EchoOperationBudgetV1::new(16, 4_096, 4_096),
            ),
            EchoOperationObstructionKindV1::ReplacementTooLarge,
        ),
        (
            canonical_invocation(
                &installed,
                basis,
                node,
                warp_core::echo_operation_atom_value_digest_v1(attachment_type, b"before"),
                b"after".to_vec(),
                EchoOperationBudgetV1::new(4, 70, 32),
            ),
            EchoOperationObstructionKindV1::BudgetExceeded,
        ),
    ];

    for (invocation_bytes, expected_kind) in cases {
        let admitted = host
            .admit_echo_operation_invocation_v1(&invocation_policy(), &invocation_bytes)
            .expect("structurally lawful input reaches private evaluation");
        let EchoOperationPreparationV1::Obstructed(obstruction) =
            host.prepare_echo_operation_v1(admitted)
        else {
            panic!("the negative fixture must not prepare a patch");
        };
        assert_eq!(obstruction.kind(), expected_kind);
    }

    let unresolved_basis = canonical_invocation(
        &installed,
        basis,
        NodeKey {
            warp_id: node.warp_id,
            local_id: make_node_id("missing-operation-node"),
        },
        warp_core::echo_operation_atom_value_digest_v1(attachment_type, b"before"),
        b"after".to_vec(),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let error = host
        .admit_echo_operation_invocation_v1(&invocation_policy(), &unresolved_basis)
        .expect_err("Echo must independently resolve the claimed application basis");
    assert_eq!(
        error.kind(),
        EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch
    );

    let state = host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the parent worldline remains registered")
        .state();
    assert_eq!(state.current_tick().as_u64(), 0);
    assert_eq!(state.state_root(), state_root_before);
    assert_eq!(
        state
            .store(&node.warp_id)
            .and_then(|store| store.node_attachment(&node.local_id)),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"before"),
        )))
    );
}

#[test]
fn obstruction_identity_binds_the_exact_invocation_admission() {
    let (mut host, head_key, node) = fixture_host();
    let installed = install_fixture_operation(&mut host);
    let basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis())
        .expect("Echo resolves the exact invocation basis");
    let invocation_bytes = canonical_invocation(
        &installed,
        basis,
        node,
        digest("wrong-value-precondition"),
        b"after".to_vec(),
        EchoOperationBudgetV1::new(8, 512, 512),
    );
    let first_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let second_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(32, 8_192, 8_192),
    );

    let obstruct = |host: &mut TrustedRuntimeHost,
                    policy: &EchoOperationInvocationAdmissionPolicyV1| {
        let admitted = host
            .admit_echo_operation_invocation_v1(policy, &invocation_bytes)
            .expect("both runtime policies admit the same invocation bytes");
        let EchoOperationPreparationV1::Obstructed(obstruction) =
            host.prepare_echo_operation_v1(admitted)
        else {
            panic!("the false precondition must obstruct");
        };
        assert_eq!(
            obstruction.kind(),
            EchoOperationObstructionKindV1::PreconditionMismatch
        );
        obstruction
    };
    let first = obstruct(&mut host, &first_policy);
    let second = obstruct(&mut host, &second_policy);

    assert_eq!(
        first.installed_operation_id(),
        second.installed_operation_id()
    );
    assert_ne!(
        first.invocation_admission_id(),
        second.invocation_admission_id()
    );
    assert_ne!(
        first.identity(),
        second.identity(),
        "different invocation-admission evidence must not alias one obstruction identity"
    );
}

#[test]
fn identical_admitted_basis_and_inputs_produce_identical_consequences() {
    let execute = || {
        let (mut host, head_key, node) = fixture_host();
        let installed = install_fixture_operation(&mut host);
        let basis = host
            .echo_operation_evaluation_basis_v1(head_key, application_basis())
            .expect("Echo resolves the exact invocation basis");
        let bytes = canonical_invocation(
            &installed,
            basis,
            node,
            warp_core::echo_operation_atom_value_digest_v1(
                make_type_id("operation-fixture-atom"),
                b"before",
            ),
            b"deterministic-after".to_vec(),
            EchoOperationBudgetV1::new(16, 4_096, 4_096),
        );
        let admitted = host
            .admit_echo_operation_invocation_v1(&invocation_policy(), &bytes)
            .expect("the exact invocation is admitted");
        let EchoOperationPreparationV1::Prepared(prepared) =
            host.prepare_echo_operation_v1(admitted)
        else {
            panic!("the deterministic fixture must prepare");
        };
        host.commit_prepared_echo_operation_v1(prepared)
            .expect("the deterministic fixture commits")
    };

    let first = execute();
    let second = execute();
    assert_eq!(first.receipt(), second.receipt());
    assert_eq!(first.snapshot(), second.snapshot());
    assert_eq!(first.tick_receipt(), second.tick_receipt());
    assert_eq!(first.committed_patch(), second.committed_patch());
}

#[test]
fn wal_activation_refuses_process_only_executable_installation_authority() {
    let (mut host, _, _) = fixture_host();
    install_fixture_operation(&mut host);
    let error = host
        .enable_in_memory_runtime_wal()
        .expect_err("WAL activation must not forget a process-only operation installation");
    assert!(matches!(
        error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::ExecutableOperationInstallation,
        })
    ));
}

#[test]
fn operation_wal_codes_append_without_renumbering_legacy_evidence() {
    assert_eq!(WalTransactionKind::SubmissionIntake.stable_code(), 1);
    assert_eq!(WalTransactionKind::SchedulerTick.stable_code(), 2);
    assert_eq!(WalTransactionKind::CausalAnchorAdmission.stable_code(), 7);
    assert_eq!(
        WalTransactionKind::ExecutableOperationInstallation.stable_code(),
        8
    );
    assert_eq!(WalTransactionKind::ExecutableOperationTick.stable_code(), 9);

    assert_eq!(WalRecordKind::TickReceiptRecorded.stable_code(), 6);
    assert_eq!(WalRecordKind::RuntimeStateDeltaRecorded.stable_code(), 7);
    assert_eq!(
        WalRecordKind::CausalAnchorAdmissionReceiptRecorded.stable_code(),
        24
    );
    assert_eq!(
        WalRecordKind::ExecutableOperationPackageInstalled.stable_code(),
        25
    );
    assert_eq!(
        WalRecordKind::ExecutableOperationExecutionRecorded.stable_code(),
        26
    );
    assert_eq!(
        WalRecordKind::ExecutableOperationStateDeltaRecorded.stable_code(),
        27
    );

    assert_eq!(AffectedFrontierKind::RuntimeState.stable_code(), 2);
    assert_eq!(AffectedFrontierKind::CausalAnchorIndex.stable_code(), 8);
    assert_eq!(
        AffectedFrontierKind::ExecutableOperationCatalog.stable_code(),
        9
    );
    assert_eq!(
        AffectedFrontierKind::ExecutableOperationReceiptIndex.stable_code(),
        10
    );
}

#[test]
fn operation_wal_transactions_reject_noncanonical_shapes() {
    let mut duplicate_installation = operation_wal_builder(
        "duplicate-operation-installation",
        WalTransactionKind::ExecutableOperationInstallation,
        WalAppendAuthority::RuntimeControl,
    );
    for _ in 0..2 {
        duplicate_installation
            .push_record(
                WalRecordKind::ExecutableOperationPackageInstalled,
                b"retained-installation".to_vec(),
            )
            .expect("the raw fixture record has installation authority");
    }
    assert!(matches!(
        duplicate_installation.commit(vec![operation_frontier(
            AffectedFrontierKind::ExecutableOperationCatalog,
            "duplicate-operation-installation",
        )]),
        Err(WalBuildError::Validation(
            WalValidationError::ExecutableOperationInstallationFrameShapeMismatch
        ))
    ));

    let mut missing_installation_frontier = operation_wal_builder(
        "missing-operation-installation-frontier",
        WalTransactionKind::ExecutableOperationInstallation,
        WalAppendAuthority::RuntimeControl,
    );
    missing_installation_frontier
        .push_record(
            WalRecordKind::ExecutableOperationPackageInstalled,
            b"retained-installation".to_vec(),
        )
        .expect("the raw fixture record has installation authority");
    assert!(matches!(
        missing_installation_frontier.commit(Vec::new()),
        Err(WalBuildError::Validation(
            WalValidationError::ExecutableOperationInstallationFrontierShapeMismatch
        ))
    ));

    let mut reversed_tick = operation_wal_builder(
        "reversed-operation-tick",
        WalTransactionKind::ExecutableOperationTick,
        WalAppendAuthority::ExecutionKernel,
    );
    reversed_tick
        .push_record(
            WalRecordKind::ExecutableOperationStateDeltaRecorded,
            b"retained-state-delta".to_vec(),
        )
        .expect("the raw fixture record has execution-kernel authority");
    reversed_tick
        .push_record(
            WalRecordKind::ExecutableOperationExecutionRecorded,
            b"retained-receipt".to_vec(),
        )
        .expect("the raw fixture record has execution-kernel authority");
    assert!(matches!(
        reversed_tick.commit(vec![
            operation_frontier(
                AffectedFrontierKind::ExecutableOperationReceiptIndex,
                "reversed-operation-receipt",
            ),
            operation_frontier(
                AffectedFrontierKind::RuntimeState,
                "reversed-operation-runtime",
            ),
        ]),
        Err(WalBuildError::Validation(
            WalValidationError::ExecutableOperationTickFrameShapeMismatch
        ))
    ));

    let mut reversed_frontiers = operation_wal_builder(
        "reversed-operation-frontiers",
        WalTransactionKind::ExecutableOperationTick,
        WalAppendAuthority::ExecutionKernel,
    );
    reversed_frontiers
        .push_record(
            WalRecordKind::ExecutableOperationExecutionRecorded,
            b"retained-receipt".to_vec(),
        )
        .expect("the raw fixture record has execution-kernel authority");
    reversed_frontiers
        .push_record(
            WalRecordKind::ExecutableOperationStateDeltaRecorded,
            b"retained-state-delta".to_vec(),
        )
        .expect("the raw fixture record has execution-kernel authority");
    assert!(matches!(
        reversed_frontiers.commit(vec![
            operation_frontier(
                AffectedFrontierKind::RuntimeState,
                "reversed-frontier-runtime",
            ),
            operation_frontier(
                AffectedFrontierKind::ExecutableOperationReceiptIndex,
                "reversed-frontier-receipt",
            ),
        ]),
        Err(WalBuildError::Validation(
            WalValidationError::ExecutableOperationTickFrontierShapeMismatch
        ))
    ));
}

#[test]
fn filesystem_wal_recovers_installed_meaning_consequence_and_typed_receipt() {
    let wal_dir = TempWalDir::new();
    let node_type = make_type_id("operation-fixture-node");
    let attachment_type = make_type_id("operation-fixture-atom");
    let package_bytes = operation_package(node_type, attachment_type)
        .to_canonical_bytes()
        .expect("package encodes");
    let package_id = warp_core::echo_operation_package_id_v1(&package_bytes);
    let application_basis = application_basis();
    let receipt_digests;

    {
        let (mut host, head_key, node) = fixture_host();
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(wal_dir.path()))
            .expect("the fresh filesystem WAL opens");
        let admitted_package = host
            .admit_echo_operation_package_v1(
                &EchoOperationAdmissionPolicyV1::exact(
                    package_id,
                    OPERATION_COORDINATE,
                    digest("fixture-authority-profile"),
                    EchoOperationBudgetV1::new(16, 4_096, 4_096),
                ),
                package_bytes,
            )
            .expect("the package is admitted");
        let installed = host
            .install_admitted_echo_operation_package_v1(admitted_package)
            .expect("installation enters the WAL before the live catalog");
        let evaluation_basis = host
            .echo_operation_evaluation_basis_v1(head_key, application_basis)
            .expect("Echo resolves the exact basis");
        let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
            installed.package_id(),
            installed.operation_coordinate(),
            evaluation_basis,
            digest("fixture-authority-grant"),
            EchoOperationBudgetV1::new(16, 4_096, 4_096),
            node,
            Some(warp_core::echo_operation_atom_value_digest_v1(
                attachment_type,
                b"before",
            )),
            b"recovered-after".to_vec(),
        );
        let invocation_bytes = invocation.to_canonical_bytes().expect("invocation encodes");
        let admitted = host
            .admit_echo_operation_invocation_v1(
                &EchoOperationInvocationAdmissionPolicyV1::new(
                    digest("fixture-authority-profile"),
                    digest("fixture-authority-grant"),
                    EchoOperationBudgetV1::new(16, 4_096, 4_096),
                ),
                &invocation_bytes,
            )
            .expect("the invocation is independently admitted");
        let EchoOperationPreparationV1::Prepared(prepared) =
            host.prepare_echo_operation_v1(admitted)
        else {
            panic!("the lawful invocation must prepare");
        };
        let evidence = host
            .commit_prepared_echo_operation_v1(prepared)
            .expect("the durable operation commits");
        let first_receipt_digest = evidence.receipt().digest();

        let second_application_basis = warp_core::echo_operation_anchored_node_application_basis_v1(
            node,
            attachment_type,
            b"recovered-after",
        );
        let second_evaluation_basis = host
            .echo_operation_evaluation_basis_v1(head_key, second_application_basis)
            .expect("Echo resolves the retained non-genesis parent basis");
        let second_invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
            installed.package_id(),
            installed.operation_coordinate(),
            second_evaluation_basis,
            digest("fixture-authority-grant"),
            EchoOperationBudgetV1::new(16, 4_096, 4_096),
            node,
            Some(warp_core::echo_operation_atom_value_digest_v1(
                attachment_type,
                b"recovered-after",
            )),
            b"recovered-twice".to_vec(),
        );
        let second_invocation_bytes = second_invocation
            .to_canonical_bytes()
            .expect("the second invocation encodes");
        let second_admitted = host
            .admit_echo_operation_invocation_v1(
                &EchoOperationInvocationAdmissionPolicyV1::new(
                    digest("fixture-authority-profile"),
                    digest("fixture-authority-grant"),
                    EchoOperationBudgetV1::new(16, 4_096, 4_096),
                ),
                &second_invocation_bytes,
            )
            .expect("the second invocation admits against the exact retained parent");
        let EchoOperationPreparationV1::Prepared(second_prepared) =
            host.prepare_echo_operation_v1(second_admitted)
        else {
            panic!("the second lawful invocation must prepare");
        };
        let second_evidence = host
            .commit_prepared_echo_operation_v1(second_prepared)
            .expect("the second durable operation commits");
        receipt_digests = vec![first_receipt_digest, second_evidence.receipt().digest()];

        let wal = host.runtime_wal().expect("the WAL remains enabled");
        assert_eq!(
            wal.commits()
                .iter()
                .map(|commit| commit.transaction_kind)
                .collect::<Vec<_>>(),
            vec![
                WalTransactionKind::ExecutableOperationInstallation,
                WalTransactionKind::ExecutableOperationTick,
                WalTransactionKind::ExecutableOperationTick,
            ]
        );
        let recovery = wal
            .recover_read_only()
            .expect("live read-only recovery works");
        assert_eq!(recovery.installed_echo_operations.len(), 1);
        assert_eq!(
            recovery
                .echo_operation_receipts
                .iter()
                .map(warp_core::EchoOperationReceiptV1::digest)
                .collect::<Vec<_>>(),
            receipt_digests
        );
        assert_eq!(
            recovery.recomputed_indexes_root().expect("indexes rehash"),
            recovery.certificate.recovered_indexes_root
        );
    }

    let (mut recovered, head_key, node) = fixture_host();
    recovered
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(wal_dir.path()))
        .expect("a fresh host recovers without executing application callbacks");
    assert!(recovered
        .engine()
        .installed_echo_operation_package_v1(package_id)
        .is_some());
    let state = recovered
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the recovered worldline exists")
        .state();
    assert_eq!(state.current_tick().as_u64(), 2);
    assert_eq!(
        state
            .store(&node.warp_id)
            .and_then(|store| store.node_attachment(&node.local_id)),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"recovered-twice"),
        )))
    );
    let recovery = recovered
        .runtime_wal()
        .expect("the recovered WAL remains enabled")
        .recover_read_only()
        .expect("fresh-host read-only recovery remains stable");
    assert_eq!(
        recovery
            .echo_operation_receipts
            .iter()
            .map(warp_core::EchoOperationReceiptV1::digest)
            .collect::<Vec<_>>(),
        receipt_digests
    );
}

/// ADR 0024: an invocation whose precondition is `None` (create-from-absence)
/// against a node and attachment that genuinely do not exist yet commits one
/// atomic patch that creates both, using exactly the program's declared
/// `required_node_type`/`required_attachment_type`.
#[test]
fn create_from_absence_commits_one_new_node_and_attachment_patch() {
    let (mut host, head_key, _existing_node) = fixture_host();
    let node_type = make_type_id("operation-fixture-node");
    let attachment_type = make_type_id("operation-fixture-atom");
    let installed = install_fixture_operation(&mut host);

    let new_node = NodeKey {
        warp_id: make_warp_id("operation-fixture"),
        local_id: make_node_id("operation-fixture-created"),
    };
    let application_basis =
        warp_core::echo_operation_anchored_node_absent_application_basis_v1(new_node);
    let evaluation_basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis)
        .expect("Echo resolves the exact current parent basis");
    let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        installed.operation_coordinate(),
        evaluation_basis,
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        new_node,
        None,
        b"created".to_vec(),
    );
    let invocation_bytes = invocation
        .to_canonical_bytes()
        .expect("the create-from-absence invocation is canonical");
    let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted_invocation = host
        .admit_echo_operation_invocation_v1(&invocation_policy, &invocation_bytes)
        .expect("Echo independently admits the create-from-absence invocation");
    let preparation = host.prepare_echo_operation_v1(admitted_invocation);
    let EchoOperationPreparationV1::Prepared(prepared) = preparation else {
        panic!("a lawful create-from-absence invocation must prepare");
    };
    assert_eq!(prepared.declared_footprint(), prepared.actual_footprint());

    let execution = host
        .commit_prepared_echo_operation_v1(prepared)
        .expect("Echo commits the create-from-absence patch");
    assert_eq!(
        execution.receipt().terminal_posture(),
        EchoOperationTerminalPostureV1::Committed
    );
    assert_eq!(
        execution.receipt().consumed_budget(),
        EchoOperationBudgetV1::new(3, 64, 39)
    );

    let state = host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the committed worldline remains registered")
        .state();
    let store = state
        .store(&new_node.warp_id)
        .expect("the warp-scoped store exists");
    assert_eq!(
        store.node(&new_node.local_id),
        Some(&NodeRecord { ty: node_type })
    );
    assert_eq!(
        store.node_attachment(&new_node.local_id),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"created"),
        )))
    );
}

/// ADR 0024: create-from-absence is a precondition like any other -- it
/// refuses, rather than silently updating, when the node already exists.
#[test]
fn create_from_absence_refuses_when_the_node_already_exists() {
    let (mut host, head_key, node) = fixture_host();
    let attachment_type = make_type_id("operation-fixture-atom");
    let installed = install_fixture_operation(&mut host);

    // The evaluation basis honestly reflects the real, present current value
    // ("before") so that admission's independent basis corroboration passes.
    // The invocation's own precondition (`None`, create-from-absence) is the
    // thing under test: it must still refuse against real existing state,
    // exercising `prepare_operation_v1`'s defense-in-depth check rather than
    // the coarser admission-time basis check.
    let application_basis = application_basis();
    let evaluation_basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis)
        .expect("Echo resolves the exact current parent basis");
    let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        installed.operation_coordinate(),
        evaluation_basis,
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        node,
        None,
        b"clobbered".to_vec(),
    );
    let invocation_bytes = invocation
        .to_canonical_bytes()
        .expect("the create-from-absence invocation is canonical");
    let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted_invocation = host
        .admit_echo_operation_invocation_v1(&invocation_policy, &invocation_bytes)
        .expect("Echo independently admits the invocation");
    let preparation = host.prepare_echo_operation_v1(admitted_invocation);
    let EchoOperationPreparationV1::Obstructed(obstruction) = preparation else {
        panic!("create-from-absence against an existing node must not prepare a patch");
    };
    assert_eq!(
        obstruction.kind(),
        EchoOperationObstructionKindV1::PreconditionMismatch
    );

    let state = host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the untouched worldline remains registered")
        .state();
    assert_eq!(
        state
            .store(&node.warp_id)
            .and_then(|store| store.node_attachment(&node.local_id)),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"before"),
        ))),
        "an obstructed create-from-absence must leave the existing attachment untouched"
    );
}

/// ADR 0024: an update-shaped invocation (`Some(digest)`) still refuses with
/// `NodeMissing` against a genuinely absent node -- widening the primitive to
/// support creation must not weaken the existing update precondition.
#[test]
fn update_precondition_still_refuses_when_the_node_is_absent() {
    let (mut host, head_key, _existing_node) = fixture_host();
    let attachment_type = make_type_id("operation-fixture-atom");
    let installed = install_fixture_operation(&mut host);

    let absent_node = NodeKey {
        warp_id: make_warp_id("operation-fixture"),
        local_id: make_node_id("operation-fixture-never-created"),
    };
    let application_basis =
        warp_core::echo_operation_anchored_node_absent_application_basis_v1(absent_node);
    let evaluation_basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis)
        .expect("Echo resolves the exact current parent basis");
    let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        installed.operation_coordinate(),
        evaluation_basis,
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        absent_node,
        Some(warp_core::echo_operation_atom_value_digest_v1(
            attachment_type,
            b"before",
        )),
        b"after".to_vec(),
    );
    let invocation_bytes = invocation
        .to_canonical_bytes()
        .expect("the update invocation is canonical");
    let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted_invocation = host
        .admit_echo_operation_invocation_v1(&invocation_policy, &invocation_bytes)
        .expect("Echo independently admits the invocation");
    let preparation = host.prepare_echo_operation_v1(admitted_invocation);
    let EchoOperationPreparationV1::Obstructed(obstruction) = preparation else {
        panic!("an update precondition against an absent node must not prepare a patch");
    };
    assert_eq!(
        obstruction.kind(),
        EchoOperationObstructionKindV1::NodeMissing
    );
}

/// ADR 0024: create-from-absence refuses with `NodeTypeMismatch`, not a
/// generic precondition failure, when a node exists at the claimed-absent
/// coordinate but with a different `NodeRecord.ty` than the installed
/// package declares. The node has no attachment set, so admission's coarser
/// check (which never inspects node type) still sees "absent" and admits;
/// `prepare_operation_v1`'s finer check is what refuses.
#[test]
fn create_from_absence_refuses_when_the_node_exists_with_the_wrong_type() {
    let wrong_type_node_id = make_node_id("operation-fixture-wrong-type");
    let wrong_node_type = make_type_id("operation-fixture-wrong-node-type");
    let (mut host, head_key, _existing_node, bare_node) =
        fixture_host_with_bare_node(wrong_type_node_id, wrong_node_type);
    let installed = install_fixture_operation(&mut host);

    let application_basis =
        warp_core::echo_operation_anchored_node_absent_application_basis_v1(bare_node);
    let evaluation_basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis)
        .expect("Echo resolves the exact current parent basis");
    let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        installed.operation_coordinate(),
        evaluation_basis,
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        bare_node,
        None,
        b"created".to_vec(),
    );
    let invocation_bytes = invocation
        .to_canonical_bytes()
        .expect("the create-from-absence invocation is canonical");
    let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted_invocation = host
        .admit_echo_operation_invocation_v1(&invocation_policy, &invocation_bytes)
        .expect("Echo admits the invocation -- node type is not an admission-time check");
    let preparation = host.prepare_echo_operation_v1(admitted_invocation);
    let EchoOperationPreparationV1::Obstructed(obstruction) = preparation else {
        panic!("create-from-absence against a wrong-typed existing node must not prepare a patch");
    };
    assert_eq!(
        obstruction.kind(),
        EchoOperationObstructionKindV1::NodeTypeMismatch
    );
}

/// ADR 0024: create-from-absence refuses with `PreconditionMismatch`, not
/// silent success, when a node exists at the claimed-absent coordinate with
/// the correct type but no alpha attachment set yet. Creation is atomic over
/// both slots or it refuses -- there is no path that attaches onto a
/// pre-existing bare node.
#[test]
fn create_from_absence_refuses_when_the_node_exists_without_its_attachment() {
    let bare_node_id = make_node_id("operation-fixture-bare");
    let node_type = make_type_id("operation-fixture-node");
    let (mut host, head_key, _existing_node, bare_node) =
        fixture_host_with_bare_node(bare_node_id, node_type);
    let installed = install_fixture_operation(&mut host);

    let application_basis =
        warp_core::echo_operation_anchored_node_absent_application_basis_v1(bare_node);
    let evaluation_basis = host
        .echo_operation_evaluation_basis_v1(head_key, application_basis)
        .expect("Echo resolves the exact current parent basis");
    let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
        installed.package_id(),
        installed.operation_coordinate(),
        evaluation_basis,
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
        bare_node,
        None,
        b"created".to_vec(),
    );
    let invocation_bytes = invocation
        .to_canonical_bytes()
        .expect("the create-from-absence invocation is canonical");
    let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        digest("fixture-authority-profile"),
        digest("fixture-authority-grant"),
        EchoOperationBudgetV1::new(16, 4_096, 4_096),
    );
    let admitted_invocation = host
        .admit_echo_operation_invocation_v1(&invocation_policy, &invocation_bytes)
        .expect("Echo admits the invocation -- a bare node still corroborates as absent");
    let preparation = host.prepare_echo_operation_v1(admitted_invocation);
    let EchoOperationPreparationV1::Obstructed(obstruction) = preparation else {
        panic!("create-from-absence against a bare existing node must not prepare a patch");
    };
    assert_eq!(
        obstruction.kind(),
        EchoOperationObstructionKindV1::PreconditionMismatch
    );
}

/// ADR 0024: the existing basis-changed TOCTOU protection covers a prepared
/// create-from-absence patch exactly as it already covers a prepared update,
/// via the same generic exact-basis commit check -- not a create-specific
/// carve-out.
#[test]
fn create_from_absence_cannot_commit_after_its_parent_basis_changes() {
    let (mut host, head_key, _existing_node) = fixture_host();
    let installed = install_fixture_operation(&mut host);
    let attachment_type = make_type_id("operation-fixture-atom");
    let new_node = NodeKey {
        warp_id: make_warp_id("operation-fixture"),
        local_id: make_node_id("operation-fixture-race-created"),
    };
    let application_basis =
        warp_core::echo_operation_anchored_node_absent_application_basis_v1(new_node);

    let prepare = |host: &TrustedRuntimeHost, replacement: &[u8]| {
        let basis = host
            .echo_operation_evaluation_basis_v1(head_key, application_basis)
            .expect("Echo resolves the current basis");
        let invocation = EchoOperationInvocationV1::anchored_node_attachment_compare_and_set(
            installed.package_id(),
            installed.operation_coordinate(),
            basis,
            digest("fixture-authority-grant"),
            EchoOperationBudgetV1::new(16, 4_096, 4_096),
            new_node,
            None,
            replacement.to_vec(),
        );
        let bytes = invocation.to_canonical_bytes().expect("invocation encodes");
        let admitted = host
            .admit_echo_operation_invocation_v1(
                &EchoOperationInvocationAdmissionPolicyV1::new(
                    digest("fixture-authority-profile"),
                    digest("fixture-authority-grant"),
                    EchoOperationBudgetV1::new(16, 4_096, 4_096),
                ),
                &bytes,
            )
            .expect("invocation is admitted");
        match host.prepare_echo_operation_v1(admitted) {
            EchoOperationPreparationV1::Prepared(prepared) => prepared,
            EchoOperationPreparationV1::Obstructed(obstruction) => {
                panic!("lawful create-from-absence invocation obstructed: {obstruction:?}")
            }
        }
    };

    let stale_preparation = prepare(&host, b"stale-created");
    let winning_preparation = prepare(&host, b"winning-created");
    host.commit_prepared_echo_operation_v1(winning_preparation)
        .expect("one exact-basis create-from-absence operation commits");
    let evidence = host
        .commit_prepared_echo_operation_v1(stale_preparation)
        .expect("basis refusal is typed evidence, not a host fault");
    assert_eq!(
        evidence.receipt().terminal_posture(),
        EchoOperationTerminalPostureV1::NotCommittedBasisChanged
    );
    assert_eq!(evidence.receipt().committed_patch_digest(), None);
    assert_eq!(evidence.receipt().committed_result_id(), None);
    assert_eq!(evidence.receipt().composition_digest(), None);

    let state = host
        .runtime()
        .worldlines()
        .get(&head_key.worldline_id)
        .expect("the worldline remains registered")
        .state();
    assert_eq!(state.current_tick().as_u64(), 1);
    assert_eq!(
        state
            .store(&new_node.warp_id)
            .and_then(|store| store.node_attachment(&new_node.local_id)),
        Some(&AttachmentValue::Atom(AtomPayload::new(
            attachment_type,
            Bytes::from_static(b"winning-created"),
        )))
    );
}
