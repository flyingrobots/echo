// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Preparing an Action must not copy its installed package (#903, E1).
//! Copying the installed operation allocates nothing, and no allocation made
//! while a real Action is prepared has the size of the package bytes, which is
//! what a copy of them allocates.
//!
//! This file holds one test and its own allocator, so that nothing else in
//! the process allocates while the measurement runs. It uses the actual
//! compiler-produced CAS package.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use bytes::Bytes;
use echo_edict_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
use sha2::{Digest, Sha256};
use warp_core::{
    echo_operation_anchored_node_application_basis_v1, echo_operation_atom_value_digest_v1,
    echo_operation_package_id_v1, make_head_id, make_node_id, make_type_id, make_warp_id,
    AtomPayload, AttachmentValue, EchoOperationAdmissionPolicy, EchoOperationBudgetV1,
    EchoOperationInvocationAdmissionPolicyV1, EchoOperationInvocationV1,
    EchoOperationPreparationV1, EngineBuilder, GraphStore, InboxPolicy, NodeId, NodeKey,
    NodeRecord, PlaybackMode, SchedulerKind, TrustedRuntimeHost, TypeId, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

/// Counts bytes requested while `ACTIVE` is set.
struct CountingAllocator;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static BYTES: AtomicUsize = AtomicUsize::new(0);
/// Allocations of exactly `WATCHED` bytes, counted while `ACTIVE` is set.
static WATCHED: AtomicUsize = AtomicUsize::new(usize::MAX);
static WATCHED_HITS: AtomicUsize = AtomicUsize::new(0);

fn count(size: usize) {
    BYTES.fetch_add(size, Ordering::SeqCst);
    if size == WATCHED.load(Ordering::SeqCst) {
        WATCHED_HITS.fetch_add(1, Ordering::SeqCst);
    }
}

// SAFETY: every call is forwarded unchanged to the system allocator; the
// counter does not affect what is allocated or freed.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::SeqCst) {
            count(layout.size());
        }
        // SAFETY: `layout` is the caller's layout, passed through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` and `layout` are the caller's, passed through.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ACTIVE.load(Ordering::SeqCst) {
            count(new_size);
        }
        // SAFETY: the caller's arguments, passed through.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure<T>(f: impl FnOnce() -> T) -> (T, usize) {
    BYTES.store(0, Ordering::SeqCst);
    WATCHED_HITS.store(0, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
    let value = f();
    ACTIVE.store(false, Ordering::SeqCst);
    (value, BYTES.load(Ordering::SeqCst))
}

const OPERATION: &str = "examples.cas_echo@1.updateCell";
const KEY: &str = "cell";
const INITIAL: &[u8] = b"initial";
const GRANT: [u8; 32] = [73; 32];

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

#[test]
fn preparing_an_action_does_not_copy_its_installed_package() {
    // A worldline holding the CAS target node, as the warp-core CAS tests do.
    let warp_id = make_warp_id("alloc-witness-cas");
    let root = make_node_id("alloc-witness-cas-root");
    let node = NodeKey {
        warp_id,
        local_id: NodeId(Sha256::digest(KEY.as_bytes()).into()),
    };
    let mut store = GraphStore::new(warp_id);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("alloc-witness-cas-root"),
        },
    );
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
    let worldline_id = WorldlineId::from_bytes([67; 32]);
    let head = WriterHeadKey {
        worldline_id,
        head_id: make_head_id("alloc-witness-cas-head"),
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
            ty: make_type_id("alloc-witness-engine-root"),
        },
    );
    let engine = EngineBuilder::new(engine_store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let mut host = TrustedRuntimeHost::new(runtime, engine).expect("host");

    // Install the actual compiler-produced package.
    let bytes = package();
    let package_len = bytes.len();
    let admitted = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicy::exact(
                echo_operation_package_id_v1(&bytes).as_hash(),
                OPERATION,
                profile("cas.echo.authority.local-demo/v1"),
                budget(),
            ),
            bytes,
        )
        .expect("package admitted");
    host.install_admitted_echo_operation_package_v1(admitted)
        .expect("install");

    // Copying the installed operation allocates nothing.
    let installed = host
        .engine()
        .installed_echo_operation_package_v1(echo_operation_package_id_v1(&package()))
        .expect("installed")
        .clone();
    let (copy, copy_bytes) = measure(|| installed.clone());
    std::hint::black_box(copy);

    // A whole preparation through the real path makes no allocation the size
    // of the package bytes. Its total is reported, not bounded: with a package
    // this small, ordinary preparation work outweighs one copy, so the total
    // says nothing about copying.
    let expected = echo_operation_atom_value_digest_v1(attachment_type(), INITIAL);
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            echo_operation_anchored_node_application_basis_v1(node, attachment_type(), INITIAL),
        )
        .expect("basis");
    let input = encode_canonical_cbor_v1(&Value::Map(vec![
        (Value::Text("basis".into()), Value::Text("explicit".into())),
        (Value::Text("key".into()), Value::Text(KEY.into())),
        (
            Value::Text("expected".into()),
            Value::Bytes(expected.to_vec()),
        ),
        (Value::Text("message".into()), Value::Text("updated".into())),
    ]))
    .expect("input");
    let invocation =
        EchoOperationInvocationV1::anchored_node_attachment_compare_and_set_with_application_input(
            echo_operation_package_id_v1(&package()),
            OPERATION,
            basis,
            GRANT,
            budget(),
            node,
            expected,
            b"updated".to_vec(),
            input,
        )
        .to_canonical_bytes()
        .expect("invocation");
    let admitted = host
        .admit_echo_operation_invocation_v1(
            &EchoOperationInvocationAdmissionPolicyV1::new(
                profile("cas.echo.authority.local-demo/v1"),
                GRANT,
                budget(),
            ),
            &invocation,
        )
        .expect("invocation admitted");
    WATCHED.store(package_len, Ordering::SeqCst);
    let (preparation, prepare_bytes) = measure(|| host.prepare_echo_operation_v1(admitted));
    let package_sized = WATCHED_HITS.load(Ordering::SeqCst);
    assert!(
        matches!(preparation, EchoOperationPreparationV1::Prepared(_)),
        "the valid invocation prepares"
    );
    eprintln!(
        "preparation allocated {prepare_bytes} bytes in total; package is {package_len} bytes"
    );
    eprintln!("copying the installed operation allocated {copy_bytes} bytes");
    assert!(prepare_bytes > 0, "the allocator measured nothing");
    assert!(
        copy_bytes == 0 && package_sized == 0,
        "copying the installed operation allocated {copy_bytes} bytes, and preparation made \
         {package_sized} allocation(s) of exactly {package_len} bytes, the size of a package copy"
    );
}
