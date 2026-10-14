// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public scheduler and recovery witnesses for an actual Edict compiler output.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, encode_canonical_cbor_v1,
    CanonicalValueV1 as Value,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use warp_core::{
    echo_operation_action_envelope_v1, echo_operation_anchored_node_application_basis_v1,
    echo_operation_atom_value_digest_v1, echo_operation_package_id_v1, make_head_id, make_node_id,
    make_type_id, make_warp_id, AtomPayload, AttachmentValue, EchoOperationActionOutcomeV1,
    EchoOperationAdmissionPolicyV1, EchoOperationBudgetV1,
    EchoOperationInvocationAdmissionErrorKindV1, EchoOperationInvocationAdmissionPolicyV1,
    EchoOperationInvocationV1, EchoOperationObstructionKindV1, EngineBuilder, GraphStore,
    InboxPolicy, IngressTarget, NodeId, NodeKey, NodeRecord, PlaybackMode, SchedulerKind,
    TrustedRuntimeHost, TrustedRuntimeWalConfig, TypeId, WorldlineId, WorldlineRuntime,
    WorldlineState, WriterHead, WriterHeadKey,
};

const OPERATION: &str = "examples.cas_echo@1.updateCell";
const KEY: &str = "cell";
const INITIAL: &[u8] = b"initial";
const GRANT: [u8; 32] = [73; 32];
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct WalDir(PathBuf);
impl WalDir {
    fn new() -> Self {
        for _ in 0..1024 {
            let path = std::env::temp_dir().join(format!(
                "edict-cas-{}-{}",
                std::process::id(),
                TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("create owned WAL directory: {error}"),
            }
        }
        panic!("unique WAL directory unavailable");
    }
}
impl Drop for WalDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove owned WAL evidence after test");
    }
}

fn package() -> Vec<u8> {
    let bytes = hex::decode(include_str!("../../echo-edict-provider-verifier/tests/fixtures/compiler-produced-cas/built/executable-operation-package.cbor.hex").trim()).expect("compiler output hex");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "4d671f8d9515c7d60c82e938103e59231eecab3c60782874f2d6fc2ff27c0d17"
    );
    bytes
}
fn profile(label: &str) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"echo:operation-profile:v1\0");
    hash.update(&(label.len() as u64).to_le_bytes());
    hash.update(label.as_bytes());
    hash.finalize().into()
}
fn attachment_type() -> TypeId {
    TypeId(profile("cas.echo.attachment.greeting-message/v1"))
}
fn budget() -> EchoOperationBudgetV1 {
    EchoOperationBudgetV1::new(16, 1024, 320)
}
fn policy() -> EchoOperationInvocationAdmissionPolicyV1 {
    EchoOperationInvocationAdmissionPolicyV1::new(
        profile("cas.echo.authority.local-demo/v1"),
        GRANT,
        budget(),
    )
}
struct Fixture {
    host: TrustedRuntimeHost,
    head: WriterHeadKey,
    node: NodeKey,
}
impl Fixture {
    fn new(present: bool) -> Self {
        let warp_id = make_warp_id("edict-cas-public-witness");
        let root = make_node_id("edict-cas-public-witness-root");
        let node = NodeKey {
            warp_id,
            local_id: NodeId(Sha256::digest(KEY.as_bytes()).into()),
        };
        let mut store = GraphStore::new(warp_id);
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("cas-witness-root"),
            },
        );
        if present {
            store.insert_node(
                node.local_id,
                NodeRecord {
                    ty: TypeId(profile("cas.echo.node.greeting/v1")),
                },
            );
            store.set_node_attachment(
                node.local_id,
                Some(AttachmentValue::Atom(AtomPayload::new(
                    attachment_type(),
                    Bytes::from_static(INITIAL),
                ))),
            );
        }
        let worldline_id = WorldlineId::from_bytes([67; 32]);
        let head = WriterHeadKey {
            worldline_id,
            head_id: make_head_id("edict-cas-public-witness-head"),
        };
        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(
                worldline_id,
                WorldlineState::from_root_store(store, root).expect("root state"),
            )
            .expect("worldline");
        runtime
            .register_writer_head(WriterHead::with_routing(
                head,
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .expect("head");
        let mut engine_store = GraphStore::default();
        engine_store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("cas-witness-engine-root"),
            },
        );
        let engine = EngineBuilder::new(engine_store, root)
            .scheduler(SchedulerKind::Radix)
            .workers(1)
            .build();
        Self {
            host: TrustedRuntimeHost::new(runtime, engine).expect("host"),
            head,
            node,
        }
    }
    fn install(&mut self) {
        let bytes = package();
        let admitted = self
            .host
            .admit_echo_operation_package_v1(
                &EchoOperationAdmissionPolicyV1::exact(
                    echo_operation_package_id_v1(&bytes),
                    OPERATION,
                    profile("cas.echo.authority.local-demo/v1"),
                    budget(),
                ),
                bytes,
            )
            .expect("actual compiler package admitted");
        self.host
            .install_admitted_echo_operation_package_v1(admitted)
            .expect("install");
    }
    fn enable_wal(&mut self, dir: &WalDir) {
        self.host
            .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
            .expect("activate or recover WAL");
    }
    fn value(&self) -> &[u8] {
        let state = self
            .host
            .runtime()
            .worldlines()
            .get(&self.head.worldline_id)
            .expect("worldline")
            .state();
        let store = state.store(&self.node.warp_id).expect("store");
        assert_eq!(
            store.node(&self.node.local_id).expect("target node").ty,
            TypeId(profile("cas.echo.node.greeting/v1"))
        );
        let Some(AttachmentValue::Atom(atom)) = store.node_attachment(&self.node.local_id) else {
            panic!("typed target atom missing")
        };
        assert_eq!(atom.type_id, attachment_type());
        &atom.bytes
    }
    fn invocation(
        &self,
        current: &[u8],
        expected: [u8; 32],
        input_expected: [u8; 32],
        replacement: &str,
    ) -> Vec<u8> {
        let basis = self
            .host
            .echo_operation_evaluation_basis_v1(
                self.head,
                echo_operation_anchored_node_application_basis_v1(
                    self.node,
                    attachment_type(),
                    current,
                ),
            )
            .expect("current causal basis");
        let input = encode_canonical_cbor_v1(&map(&[
            ("basis", Value::Text("explicit".into())),
            ("key", Value::Text(KEY.into())),
            ("expected", Value::Bytes(input_expected.to_vec())),
            ("message", Value::Text(replacement.into())),
        ]))
        .expect("input");
        // The package itself is the exact compiler output, with no test-side
        // rewriting; the invocation uses the public projected constructor.
        EchoOperationInvocationV1::anchored_node_attachment_compare_and_set_with_application_input(
            echo_operation_package_id_v1(&package()),
            OPERATION,
            basis,
            GRANT,
            budget(),
            self.node,
            expected,
            replacement.as_bytes().to_vec(),
            input,
        )
        .to_canonical_bytes()
        .expect("projected invocation")
    }
}
fn map(fields: &[(&str, Value)]) -> Value {
    Value::Map(
        fields
            .iter()
            .map(|(key, value)| (Value::Text((*key).into()), value.clone()))
            .collect(),
    )
}
fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(entries) = value else {
        panic!("{name} must be read from a map")
    };
    entries
        .iter()
        .find_map(|(key, entry)| (key == &Value::Text(name.into())).then_some(entry))
        .expect("field exists")
}

#[test]
fn compiler_cas_report_accepts_exactly_the_retained_package() {
    let report = decode_canonical_cbor_v1(
        &hex::decode(include_str!("../../echo-edict-provider-verifier/tests/fixtures/compiler-produced-cas/built/verification-report.cbor.hex").trim())
            .expect("report hex"),
    )
    .expect("report CBOR");
    let package_digest = digest_canonical_value_bytes_v1(
        "echo.operation-package/v1",
        &decode_canonical_cbor_v1(&package()).expect("package CBOR"),
    )
    .expect("package digest");
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    assert_eq!(field(&report, "diagnosticBytes"), &Value::Bytes(vec![]));
    assert_eq!(
        field(field(&report, "package"), "digest"),
        &Value::Array(vec![
            Value::Text("sha256".into()),
            Value::Bytes(package_digest.into())
        ])
    );
}

/// Exercises the Action staging journal and the scheduler undo handle: a WAL
/// refusal after an executable-operation Action committed in memory must leave
/// no trace of that tick.
#[cfg(feature = "host_test")]
#[test]
fn compiler_cas_tick_wal_failure_restores_exact_runtime_and_provenance() {
    use warp_core::causal_wal::{FilesystemWalFaultPlan, FilesystemWalFaultTarget};

    for target in [
        FilesystemWalFaultTarget::AppendFrame,
        FilesystemWalFaultTarget::FlushCommit,
    ] {
        let dir = WalDir::new();
        let mut fixture = Fixture::new(true);
        fixture.enable_wal(&dir);
        fixture.install();
        fixture
            .host
            .install_echo_operation_action_admission_policy_v1(policy());
        let submit = |fixture: &mut Fixture, current: &[u8], next: &str| {
            let digest = echo_operation_atom_value_digest_v1(attachment_type(), current);
            let envelope = echo_operation_action_envelope_v1(
                IngressTarget::ExactHead { key: fixture.head },
                fixture.invocation(current, digest, digest, next),
            )
            .expect("Action envelope");
            fixture
                .host
                .app()
                .submit_intent_with_runtime_wal_ack(envelope)
                .expect("durable Action acknowledgement")
                .submission_id
        };
        submit(&mut fixture, INITIAL, "first");
        fixture.host.tick_once().expect("first Action commits");
        assert_eq!(fixture.value(), b"first");

        let submission = submit(&mut fixture, b"first", "second");
        let runtime_before = format!("{:?}", fixture.host.runtime());
        let provenance_before = format!("{:?}", fixture.host.provenance());
        fixture
            .host
            .inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(target))
            .expect("inject a filesystem failure");

        fixture
            .host
            .tick_once()
            .expect_err("an undurable Action tick must not remain visible");

        assert_eq!(
            format!("{:?}", fixture.host.runtime()),
            runtime_before,
            "{target:?}: a failed Action tick must restore the exact pre-tick runtime"
        );
        assert_eq!(
            format!("{:?}", fixture.host.provenance()),
            provenance_before,
            "{target:?}: a failed Action tick must restore the exact pre-tick provenance"
        );
        assert_eq!(fixture.value(), b"first");
        assert!(fixture
            .host
            .echo_operation_action_outcome_v1(&submission)
            .is_none());

        let steps = fixture
            .host
            .tick_once()
            .expect("the restored Action must commit once the WAL accepts it");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].admitted_count, 1);
        assert_eq!(fixture.value(), b"second");
        assert!(matches!(
            fixture.host.echo_operation_action_outcome_v1(&submission),
            Some(EchoOperationActionOutcomeV1::Committed(_))
        ));
    }
}

/// A commit record that reaches the file and then fails its sync is readable
/// and not durable. The Action tick must not stay visible, and the host must
/// not reconcile itself from the log.
#[cfg(feature = "host_test")]
#[test]
fn compiler_cas_tick_whose_commit_sync_failed_is_not_visible() {
    use warp_core::causal_wal::{FilesystemWalFaultPlan, FilesystemWalFaultTarget};
    use warp_core::{TrustedRuntimeHostError, TrustedRuntimeWalError};

    let dir = WalDir::new();
    let mut fixture = Fixture::new(true);
    fixture.enable_wal(&dir);
    fixture.install();
    fixture
        .host
        .install_echo_operation_action_admission_policy_v1(policy());
    let submit = |fixture: &mut Fixture, current: &[u8], next: &str| {
        let digest = echo_operation_atom_value_digest_v1(attachment_type(), current);
        let envelope = echo_operation_action_envelope_v1(
            IngressTarget::ExactHead { key: fixture.head },
            fixture.invocation(current, digest, digest, next),
        )
        .expect("Action envelope");
        fixture
            .host
            .app()
            .submit_intent_with_runtime_wal_ack(envelope)
            .expect("durable Action acknowledgement")
            .submission_id
    };
    submit(&mut fixture, INITIAL, "first");
    fixture.host.tick_once().expect("first Action commits");
    assert_eq!(fixture.value(), b"first");

    let submission = submit(&mut fixture, b"first", "second");
    let runtime_before = format!("{:?}", fixture.host.runtime());
    let provenance_before = format!("{:?}", fixture.host.provenance());
    fixture
        .host
        .inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
            FilesystemWalFaultTarget::CommitMarkerSync,
        ))
        .expect("inject a filesystem failure");

    fixture
        .host
        .tick_once()
        .expect_err("an Action tick whose commit sync failed must not remain visible");

    assert_eq!(format!("{:?}", fixture.host.runtime()), runtime_before);
    assert_eq!(
        format!("{:?}", fixture.host.provenance()),
        provenance_before
    );
    assert_eq!(fixture.value(), b"first");
    assert!(fixture
        .host
        .echo_operation_action_outcome_v1(&submission)
        .is_none());
    let error = fixture
        .host
        .tick_once()
        .expect_err("the WAL is failed closed");
    assert!(
        matches!(
            error,
            TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::StorageFailed)
        ),
        "{error:?}"
    );
}

#[cfg(feature = "host_test")]
#[test]
fn compiler_cas_tick_hashes_the_state_once_after_the_first_commit() {
    let dir = WalDir::new();
    let mut fixture = Fixture::new(true);
    fixture.enable_wal(&dir);
    fixture.install();
    fixture
        .host
        .install_echo_operation_action_admission_policy_v1(policy());
    let submit = |fixture: &mut Fixture, current: &[u8], next: &str| {
        let digest = echo_operation_atom_value_digest_v1(attachment_type(), current);
        let envelope = echo_operation_action_envelope_v1(
            IngressTarget::ExactHead { key: fixture.head },
            fixture.invocation(current, digest, digest, next),
        )
        .expect("Action envelope");
        fixture
            .host
            .app()
            .submit_intent_with_runtime_wal_ack(envelope)
            .expect("durable Action acknowledgement");
    };
    let root_computations = |fixture: &Fixture| {
        fixture
            .host
            .runtime()
            .worldlines()
            .get(&fixture.head.worldline_id)
            .expect("the worldline remains live")
            .state()
            .state_root_computation_count_for_test()
    };
    submit(&mut fixture, INITIAL, "first");
    fixture.host.tick_once().expect("first Action commits");
    fixture
        .host
        .runtime()
        .worldlines()
        .get(&fixture.head.worldline_id)
        .expect("the worldline remains live")
        .state()
        .reset_state_root_computation_count_for_test();

    submit(&mut fixture, b"first", "second");
    fixture.host.tick_once().expect("second Action commits");
    assert_eq!(fixture.value(), b"second");
    assert_eq!(
        root_computations(&fixture),
        1,
        "the frontier basis reuses the root the previous commit computed; \
         only the root after this commit is hashed (the count excludes the \
         hash by which a debug build checks the retained root)"
    );

    submit(&mut fixture, b"second", "third");
    fixture.host.tick_once().expect("third Action commits");
    assert_eq!(root_computations(&fixture), 2);
}

fn initial_digest() -> [u8; 32] {
    echo_operation_atom_value_digest_v1(attachment_type(), INITIAL)
}

#[test]
fn compiler_cas_recovers_pending_action_committed_result_and_stale_obstruction() {
    let dir = WalDir::new();
    let mut accepted = Fixture::new(true);
    accepted.enable_wal(&dir);
    accepted.install();
    let invocation = accepted.invocation(INITIAL, initial_digest(), initial_digest(), "updated");
    let envelope = echo_operation_action_envelope_v1(
        IngressTarget::ExactHead { key: accepted.head },
        invocation,
    )
    .expect("Action envelope");
    let submission = accepted
        .host
        .app()
        .submit_intent_with_runtime_wal_ack(envelope)
        .expect("durable Action acknowledgement")
        .submission_id;
    assert_eq!(accepted.value(), INITIAL);
    let wal = accepted
        .host
        .runtime_wal()
        .expect("WAL")
        .recover_read_only()
        .expect("read accepted WAL");
    assert!(wal
        .witnessed_submissions
        .records()
        .iter()
        .any(|record| record.submission.submission_id == submission));
    assert!(wal.provenance_entries.is_empty());
    assert!(wal.receipt_correlations.is_empty());
    drop(accepted);

    let mut pending = Fixture::new(true);
    pending.enable_wal(&dir);
    assert_eq!(
        pending.host.runtime().pending_witnessed_submission_count(),
        1
    );
    assert!(pending
        .host
        .engine()
        .installed_echo_operation_package_v1(echo_operation_package_id_v1(&package()))
        .is_some());
    assert!(pending
        .host
        .echo_operation_action_outcome_v1(&submission)
        .is_none());
    assert_eq!(pending.value(), INITIAL);
    pending
        .host
        .install_echo_operation_action_admission_policy_v1(policy());
    let steps = pending
        .host
        .tick_once()
        .expect("scheduler evaluates recovered CAS");
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].admitted_count, 1);
    assert_eq!(pending.value(), b"updated");
    let Some(EchoOperationActionOutcomeV1::Committed(receipt)) =
        pending.host.echo_operation_action_outcome_v1(&submission)
    else {
        panic!("CAS must commit")
    };
    assert_eq!(receipt.worldline_tick_after().as_u64(), 1);
    assert!(receipt.commit_global_tick().is_some());
    let result = receipt
        .committed_application_result()
        .expect("typed result")
        .canonical_bytes()
        .to_vec();
    assert_eq!(
        decode_canonical_cbor_v1(&result).expect("result CBOR"),
        decode_canonical_cbor_v1(
            &encode_canonical_cbor_v1(&map(&[
                ("key", Value::Text(KEY.into())),
                ("message", Value::Text("updated".into()))
            ]))
            .expect("expected result")
        )
        .expect("expected CBOR")
    );
    let receipt_digest = receipt.digest();
    let commit_id = receipt.commit_id();
    drop(pending);

    let mut recovered = Fixture::new(true);
    recovered.enable_wal(&dir);
    assert_eq!(recovered.value(), b"updated");
    assert_eq!(
        recovered
            .host
            .runtime()
            .pending_witnessed_submission_count(),
        0
    );
    let Some(EchoOperationActionOutcomeV1::Committed(receipt)) =
        recovered.host.echo_operation_action_outcome_v1(&submission)
    else {
        panic!("committed outcome recovers")
    };
    assert_eq!(receipt.digest(), receipt_digest);
    assert_eq!(receipt.commit_id(), commit_id);
    assert_eq!(
        receipt
            .committed_application_result()
            .expect("recovered typed result")
            .canonical_bytes(),
        result
    );
    // Keep the causal/application basis fresh, but the expected value stale.
    // This must reach CAS evaluation rather than fail an unrelated basis check.
    let stale = recovered.invocation(
        b"updated",
        initial_digest(),
        initial_digest(),
        "should not write",
    );
    recovered
        .host
        .admit_echo_operation_invocation_v1(&policy(), &stale)
        .expect("stale precondition passes static admission");
    recovered
        .host
        .install_echo_operation_action_admission_policy_v1(policy());
    let envelope = echo_operation_action_envelope_v1(
        IngressTarget::ExactHead {
            key: recovered.head,
        },
        stale,
    )
    .expect("stale envelope");
    let stale_submission = recovered
        .host
        .app()
        .submit_intent_with_runtime_wal_ack(envelope)
        .expect("accept stale Action")
        .submission_id;
    let steps = recovered.host.tick_once().expect("decide stale Action");
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].admitted_count, 1);
    let Some(EchoOperationActionOutcomeV1::Obstructed(obstruction)) = recovered
        .host
        .echo_operation_action_outcome_v1(&stale_submission)
    else {
        panic!("stale CAS must obstruct")
    };
    assert_eq!(
        obstruction.kind(),
        EchoOperationObstructionKindV1::PreconditionMismatch
    );
    assert_eq!(recovered.value(), b"updated");
    drop(recovered);
    let mut recovered_again = Fixture::new(true);
    recovered_again.enable_wal(&dir);
    assert_eq!(recovered_again.value(), b"updated");
    let Some(EchoOperationActionOutcomeV1::Obstructed(obstruction)) = recovered_again
        .host
        .echo_operation_action_outcome_v1(&stale_submission)
    else {
        panic!("obstruction recovers")
    };
    assert_eq!(
        obstruction.kind(),
        EchoOperationObstructionKindV1::PreconditionMismatch
    );
}

#[test]
fn compiler_cas_refuses_substituted_digest_and_missing_node_without_mutation() {
    let mut fixture = Fixture::new(true);
    fixture.install();
    let substituted = fixture.invocation(INITIAL, initial_digest(), [99; 32], "updated");
    assert_eq!(
        fixture
            .host
            .admit_echo_operation_invocation_v1(&policy(), &substituted)
            .expect_err("substituted digest refused")
            .kind(),
        EchoOperationInvocationAdmissionErrorKindV1::ApplicationInputMismatch
    );
    assert_eq!(fixture.value(), INITIAL);
    let wrong_basis =
        fixture.invocation(b"different", initial_digest(), initial_digest(), "updated");
    assert_eq!(
        fixture
            .host
            .admit_echo_operation_invocation_v1(&policy(), &wrong_basis)
            .expect_err("uncorroborated application basis refused")
            .kind(),
        EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch
    );
    let valid = fixture.invocation(INITIAL, initial_digest(), initial_digest(), "updated");
    let other_grant = EchoOperationInvocationAdmissionPolicyV1::new(
        profile("cas.echo.authority.local-demo/v1"),
        [74; 32],
        budget(),
    );
    assert_eq!(
        fixture
            .host
            .admit_echo_operation_invocation_v1(&other_grant, &valid)
            .expect_err("ungranted authority refused")
            .kind(),
        EchoOperationInvocationAdmissionErrorKindV1::AuthorityGrantMismatch
    );
    assert_eq!(fixture.value(), INITIAL);
    let mut missing = Fixture::new(false);
    missing.install();
    let invocation = missing.invocation(INITIAL, initial_digest(), initial_digest(), "updated");
    assert_eq!(
        missing
            .host
            .admit_echo_operation_invocation_v1(&policy(), &invocation)
            .expect_err("missing node refused")
            .kind(),
        EchoOperationInvocationAdmissionErrorKindV1::BasisMismatch
    );
    assert!(missing
        .host
        .runtime()
        .worldlines()
        .get(&missing.head.worldline_id)
        .expect("worldline")
        .state()
        .store(&missing.node.warp_id)
        .expect("store")
        .node(&missing.node.local_id)
        .is_none());
}

#[test]
fn compiler_cas_obstructs_oversized_replacement_without_mutation() {
    let mut fixture = Fixture::new(true);
    fixture.install();
    fixture
        .host
        .install_echo_operation_action_admission_policy_v1(policy());
    let invocation = fixture.invocation(
        INITIAL,
        initial_digest(),
        initial_digest(),
        &"x".repeat(257),
    );
    let envelope = echo_operation_action_envelope_v1(
        IngressTarget::ExactHead { key: fixture.head },
        invocation,
    )
    .expect("envelope");
    let submission = fixture
        .host
        .app()
        .submit_intent(envelope)
        .expect("accept Action")
        .submission_id;
    let steps = fixture
        .host
        .tick_once()
        .expect("decide oversized replacement");
    assert_eq!(steps.len(), 1);
    let Some(EchoOperationActionOutcomeV1::Obstructed(obstruction)) =
        fixture.host.echo_operation_action_outcome_v1(&submission)
    else {
        panic!("oversized replacement obstructs")
    };
    assert_eq!(
        obstruction.kind(),
        EchoOperationObstructionKindV1::ReplacementTooLarge
    );
    assert_eq!(fixture.value(), INITIAL);
}

#[test]
fn compiler_cas_accepts_replacement_at_exact_declared_bound() {
    let mut fixture = Fixture::new(true);
    fixture.install();
    fixture
        .host
        .install_echo_operation_action_admission_policy_v1(policy());
    let replacement = "x".repeat(256);
    let invocation = fixture.invocation(INITIAL, initial_digest(), initial_digest(), &replacement);
    let envelope = echo_operation_action_envelope_v1(
        IngressTarget::ExactHead { key: fixture.head },
        invocation,
    )
    .expect("bounded envelope");
    let submission = fixture
        .host
        .app()
        .submit_intent(envelope)
        .expect("accept boundary Action")
        .submission_id;
    let steps = fixture.host.tick_once().expect("execute boundary Action");
    assert_eq!(steps.len(), 1);
    assert!(matches!(
        fixture.host.echo_operation_action_outcome_v1(&submission),
        Some(EchoOperationActionOutcomeV1::Committed(_))
    ));
    assert_eq!(fixture.value(), replacement.as_bytes());
}

#[test]
fn compiler_cas_refuses_aliased_projection_before_installation() {
    let fixture = Fixture::new(true);
    let Value::Map(mut fields) = decode_canonical_cbor_v1(&package()).expect("compiler package")
    else {
        panic!("package map")
    };
    let (_, Value::Map(projection)) = fields
        .iter_mut()
        .find(|(key, _)| key == &Value::Text("application_result_projection".into()))
        .expect("projection")
    else {
        panic!("projection map")
    };
    let (_, expected) = projection
        .iter_mut()
        .find(|(key, _)| key == &Value::Text("application_input_expected_value_digest_path".into()))
        .expect("digest binding");
    *expected = Value::Array(vec![Value::Text("key".into())]);
    let bytes = encode_canonical_cbor_v1(&Value::Map(fields)).expect("mutated package");
    let id = echo_operation_package_id_v1(&bytes);
    // Pin the mutated identity so refusal proves structural validation, not
    // merely a mismatch against the original package's hash.
    let error = fixture
        .host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                id,
                OPERATION,
                profile("cas.echo.authority.local-demo/v1"),
                budget(),
            ),
            bytes,
        )
        .expect_err("aliased projection refused");
    assert_eq!(
        error.kind(),
        warp_core::EchoOperationAdmissionErrorKindV1::ArtifactInvalid
    );
    assert_eq!(
        error.artifact().expect("artifact error").kind(),
        warp_core::EchoOperationArtifactErrorKindV1::InvalidStructure
    );
    assert!(fixture
        .host
        .engine()
        .installed_echo_operation_package_v1(id)
        .is_none());
    assert_eq!(fixture.value(), INITIAL);
}
