// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A scene of independent cells that compare-and-set Actions update.
//!
//! Every cell is linked to the root node by an edge, so it is reachable and
//! the state root covers it, as an application's state is. A scene whose
//! cells are not reachable leaves them out of the state root and makes a
//! tick look far cheaper than it is: set `ECHO_BENCH_UNLINKED` only to show
//! that difference.
//!
//! The Actions are the ones an application submits: the package is the exact
//! output of the Edict compiler that the provider tests retain, admitted and
//! installed through the public host calls. Nothing here reaches into the
//! scheduler or the WAL.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bytes::Bytes;
use echo_edict_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
use sha2::{Digest, Sha256};
use warp_core::{
    echo_operation_action_envelope_v1, echo_operation_anchored_node_application_basis_v1,
    echo_operation_atom_value_digest_v1, echo_operation_package_id_v1, make_edge_id, make_head_id,
    make_node_id, make_type_id, make_warp_id, AtomPayload, AttachmentValue,
    EchoOperationAdmissionPolicyV1, EchoOperationBudgetV1,
    EchoOperationInvocationAdmissionPolicyV1, EchoOperationInvocationV1, EdgeRecord, EngineBuilder,
    GraphStore, InboxPolicy, IngressEnvelope, IngressTarget, NodeId, NodeKey, NodeRecord,
    PlaybackMode, SchedulerKind, TrustedRuntimeHost, TrustedRuntimeWalConfig, TypeId, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

const OPERATION: &str = "examples.cas_echo@1.updateCell";
const GRANT: [u8; 32] = [73; 32];
/// The value every cell holds before the first Action.
pub const INITIAL: &[u8] = b"initial";

/// Where a scene keeps its write-ahead log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Durability {
    /// No log. Nothing survives a crash.
    None,
    /// The in-memory log: every record is built and kept, nothing is synced.
    InMemory,
    /// The filesystem log: every acknowledgement waits for a storage sync.
    Filesystem,
}

impl Durability {
    /// A short name for a results table.
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::InMemory => "in-memory",
            Self::Filesystem => "filesystem",
        }
    }
}

/// A directory the scene owns and removes when it is dropped.
pub struct OwnedDir(PathBuf);

impl OwnedDir {
    /// Creates a fresh directory under `parent`.
    pub fn create_in(parent: &Path, label: &str) -> Self {
        // Creating a directory fails if it exists, so the first ordinal that
        // can be created is this scene's alone, whatever else is running.
        for ordinal in 0..4096_u32 {
            let path = parent.join(format!("echo-bench-{label}-{ordinal}"));
            match fs::create_dir_all(parent).and_then(|()| fs::create_dir(&path)) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("create benchmark directory {}: {error}", path.display()),
            }
        }
        panic!("no unused benchmark directory under {}", parent.display());
    }

    /// The directory path.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for OwnedDir {
    fn drop(&mut self) {
        // Benchmark output is disposable; a failed removal is not a result.
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A trusted host with `cells` independent cells and one writer head.
pub struct Scene {
    pub host: TrustedRuntimeHost,
    head: WriterHeadKey,
    package: Vec<u8>,
    /// The value each cell currently holds, as this driver last set it.
    current: Vec<Vec<u8>>,
    _log: Option<OwnedDir>,
}

fn package() -> Vec<u8> {
    hex::decode(
        include_str!(
            "../../../echo-edict-provider-verifier/tests/fixtures/compiler-produced-cas/built/executable-operation-package.cbor.hex"
        )
        .trim(),
    )
    .expect("the retained compiler package is hexadecimal")
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

fn cell_key(cell: usize) -> String {
    format!("cell-{cell:06}")
}

fn cell_node(cell: usize) -> NodeKey {
    NodeKey {
        warp_id: make_warp_id("edict-cas-public-witness"),
        local_id: NodeId(Sha256::digest(cell_key(cell).as_bytes()).into()),
    }
}

impl Scene {
    /// Builds a scene. `log_parent` is used only for the filesystem log.
    pub fn new(cells: usize, durability: Durability, log_parent: &Path) -> Self {
        let unlinked = std::env::var_os("ECHO_BENCH_UNLINKED").is_some();
        let warp_id = make_warp_id("edict-cas-public-witness");
        let root = make_node_id("edict-cas-public-witness-root");
        let mut store = GraphStore::new(warp_id);
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("cas-witness-root"),
            },
        );
        for cell in 0..cells {
            let node = cell_node(cell);
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
            if !unlinked {
                store.insert_edge(
                    root,
                    EdgeRecord {
                        id: make_edge_id(&cell_key(cell)),
                        from: root,
                        to: node.local_id,
                        ty: make_type_id("cas-witness-cell"),
                    },
                );
            }
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
        let mut host = TrustedRuntimeHost::new(runtime, engine).expect("host");
        let log = match durability {
            Durability::None => None,
            Durability::InMemory => {
                host.enable_runtime_wal(TrustedRuntimeWalConfig::in_memory())
                    .expect("in-memory log");
                None
            }
            Durability::Filesystem => {
                let dir = OwnedDir::create_in(log_parent, "wal");
                host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(dir.path()))
                    .expect("filesystem log");
                Some(dir)
            }
        };
        let package = package();
        let admitted = host
            .admit_echo_operation_package_v1(
                &EchoOperationAdmissionPolicyV1::exact(
                    echo_operation_package_id_v1(&package),
                    OPERATION,
                    profile("cas.echo.authority.local-demo/v1"),
                    budget(),
                ),
                package.clone(),
            )
            .expect("the compiler package is admitted");
        host.install_admitted_echo_operation_package_v1(admitted)
            .expect("the package is installed");
        host.install_echo_operation_action_admission_policy_v1(
            EchoOperationInvocationAdmissionPolicyV1::new(
                profile("cas.echo.authority.local-demo/v1"),
                GRANT,
                budget(),
            ),
        );
        Self {
            host,
            head,
            package,
            current: vec![INITIAL.to_vec(); cells],
            _log: log,
        }
    }

    /// An Action that replaces what `cell` holds now with `next`.
    ///
    /// Two envelopes built for the same cell before either commits expect the
    /// same current value, so their footprints overlap and their expectations
    /// collide. `nonce` keeps their bytes distinct, so intake does not treat
    /// the second as a repeat of the first.
    pub fn envelope(&self, cell: usize, next: &str, nonce: usize) -> IngressEnvelope {
        let node = cell_node(cell);
        let current = &self.current[cell];
        let digest = echo_operation_atom_value_digest_v1(attachment_type(), current);
        let basis = self
            .host
            .echo_operation_evaluation_basis_v1(
                self.head,
                echo_operation_anchored_node_application_basis_v1(node, attachment_type(), current),
            )
            .expect("current causal basis");
        let replacement = format!("{next}#{nonce}");
        let input = encode_canonical_cbor_v1(&Value::Map(vec![
            (Value::Text("basis".into()), Value::Text("explicit".into())),
            (Value::Text("key".into()), Value::Text(cell_key(cell))),
            (
                Value::Text("expected".into()),
                Value::Bytes(digest.to_vec()),
            ),
            (
                Value::Text("message".into()),
                Value::Text(replacement.clone()),
            ),
        ]))
        .expect("application input encodes");
        let invocation =
            EchoOperationInvocationV1::anchored_node_attachment_compare_and_set_with_application_input(
                echo_operation_package_id_v1(&self.package),
                OPERATION,
                basis,
                GRANT,
                budget(),
                node,
                digest,
                replacement.into_bytes(),
                input,
            )
            .to_canonical_bytes()
            .expect("invocation encodes");
        echo_operation_action_envelope_v1(IngressTarget::ExactHead { key: self.head }, invocation)
            .expect("Action envelope")
    }

    /// Records that `cell` now holds the value an Action with `next` and
    /// `nonce` wrote.
    pub fn committed(&mut self, cell: usize, next: &str, nonce: usize) {
        self.current[cell] = format!("{next}#{nonce}").into_bytes();
    }
}

/// The nearest-rank percentile of the samples, in microseconds: the smallest
/// sample with at least `percent` of the samples at or below it. For 100
/// samples, p50 is the 50th smallest and p95 the 95th.
pub fn percentile_us(samples: &[Duration], percent: usize) -> f64 {
    assert!(
        !samples.is_empty(),
        "a percentile needs at least one sample"
    );
    assert!(
        (1..=100).contains(&percent),
        "a percentile is between 1 and 100"
    );
    let mut sorted: Vec<f64> = samples
        .iter()
        .map(|sample| sample.as_secs_f64() * 1e6)
        .collect();
    sorted.sort_by(f64::total_cmp);
    let rank = (sorted.len() * percent).div_ceil(100);
    sorted[rank - 1]
}

fn parse_setting(name: &str, text: &str) -> usize {
    text.trim()
        .parse()
        .unwrap_or_else(|_| panic!("{name}: `{text}` is not a non-negative integer"))
}

/// Reads an integer setting from the environment. A value that is set and
/// does not parse stops the benchmark and names the setting.
pub fn setting(name: &str, default: usize) -> usize {
    std::env::var(name).map_or(default, |value| parse_setting(name, &value))
}

/// Reads a non-empty, comma-separated list of integers from the
/// environment. Any entry that does not parse stops the benchmark.
pub fn setting_list(name: &str, default: &[usize]) -> Vec<usize> {
    let values: Vec<usize> = std::env::var(name).map_or_else(
        |_| default.to_vec(),
        |value| {
            value
                .split(',')
                .map(|part| parse_setting(name, part))
                .collect()
        },
    );
    assert!(!values.is_empty(), "{name} lists no values");
    values
}

/// The directory benchmark logs are written under.
pub fn log_parent() -> PathBuf {
    std::env::var_os("ECHO_BENCH_LOG_DIR").map_or_else(std::env::temp_dir, PathBuf::from)
}
