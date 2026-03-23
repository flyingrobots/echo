// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::cast_possible_truncation,
    clippy::format_collect,
    clippy::match_same_arms
)]

use warp_core::{
    compute_commit_hash_v2, make_edge_id, make_head_id, make_node_id, make_type_id, make_warp_id,
    ApplyResult, AtomPayload, AtomWriteSet, AttachmentKey, AttachmentSet, AttachmentValue,
    ConflictPolicy, CursorId, EdgeId, EdgeRecord, Engine, EngineBuilder, Footprint, GlobalTick,
    GraphStore, Hash, HashTriplet, LocalProvenanceStore, NodeId, NodeKey, NodeRecord,
    OutputFrameSet, PatternGraph, ProvenanceEntry, ProvenanceStore, RewriteRule, SessionId,
    TickCommitStatus, WarpId, WarpOp, WarpTickPatchV1, WorldlineId, WorldlineState, WorldlineTick,
    WorldlineTickHeaderV1, WorldlineTickPatchV1, WriterHeadKey,
};

// =============================================================================
// PARALLEL EXECUTION COMPLIANCE TEST UTILITIES (ADR-0007)
// =============================================================================

/// 32-byte hash type alias for clarity.
pub type Hash32 = [u8; 32];

/// Tiny deterministic RNG (xorshift64*) so tests don't need `rand`.
#[derive(Clone)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    /// Creates a new PRNG with the given seed.
    ///
    /// If `seed` is 0, it is replaced with 1 (zero seeds would produce
    /// all-zero output in xorshift).
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    /// Returns the next pseudo-random `u64` in the xorshift64* sequence.
    ///
    /// The output is the internal state multiplied by the xorshift64*
    /// constant after applying three shift-xor operations.
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Returns a pseudo-random value in `[0, upper)`.
    ///
    /// Uses simple modulo reduction, which introduces slight bias when
    /// `upper` is not a power of two. This is acceptable for test usage.
    pub fn gen_range_usize(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            return 0;
        }
        (self.next_u64() as usize) % upper
    }
}

/// Fisher–Yates shuffle (deterministic).
pub fn shuffle<T>(rng: &mut XorShift64, items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let j = rng.gen_range_usize(i + 1);
        items.swap(i, j);
    }
}

/// Generate a random 32-byte hash from the RNG.
///
/// Fills each 8-byte chunk with `rng.next_u64()` bytes in little-endian order.
pub fn random_hash(rng: &mut XorShift64) -> [u8; 32] {
    let mut h = [0u8; 32];
    for chunk in h.chunks_mut(8) {
        let bytes = rng.next_u64().to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
    h
}

/// Generate a random footprint for testing independence checks.
///
/// Creates a [`Footprint`] with random nodes, edges, and attachments,
/// using the provided RNG. Suitable for fuzz-like symmetry tests.
pub fn random_footprint(rng: &mut XorShift64) -> Footprint {
    let mut fp = Footprint::default();
    let warp_id = WarpId([0u8; 32]); // Use a fixed WarpId for testing

    // Add random nodes (warp-scoped)
    for _ in 0..(rng.gen_range_usize(5)) {
        let node_id = NodeId(random_hash(rng));
        if rng.next_u64().is_multiple_of(2) {
            fp.n_read.insert_with_warp(warp_id, node_id);
        } else {
            fp.n_write.insert_with_warp(warp_id, node_id);
        }
    }

    // Add random edges (warp-scoped)
    for _ in 0..(rng.gen_range_usize(3)) {
        let edge_id = EdgeId(random_hash(rng));
        if rng.next_u64().is_multiple_of(2) {
            fp.e_read.insert_with_warp(warp_id, edge_id);
        } else {
            fp.e_write.insert_with_warp(warp_id, edge_id);
        }
    }

    // Add random attachments
    for _ in 0..(rng.gen_range_usize(3)) {
        let node_id = NodeId(random_hash(rng));
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };
        let key = AttachmentKey::node_alpha(node_key);
        if rng.next_u64().is_multiple_of(2) {
            fp.a_read.insert(key);
        } else {
            fp.a_write.insert(key);
        }
    }

    fp
}

/// Useful seed set for determinism drills.
pub const SEEDS: &[u64] = &[
    0x0000_0000_0000_0001,
    0x1234_5678_9ABC_DEF0,
    0xDEAD_BEEF_CAFE_BABE,
    0xFEED_FACE_0123_4567,
    0x0F0F_0F0F_F0F0_F0F0,
];

/// Worker counts to prove "doesn't depend on num_cpus".
pub const WORKER_COUNTS: &[usize] = &[1, 2, 4, 8, 16, 32];

/// Converts a 32-byte hash (`Hash32`) to its lowercase hexadecimal string representation.
///
/// # Arguments
/// * `h` - A reference to a 32-byte hash array
///
/// # Returns
/// A 64-character lowercase hexadecimal string
pub fn hex32(h: &Hash32) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// For comparing hashes with readable diffs.
pub fn assert_hash_eq(a: &Hash32, b: &Hash32, msg: &str) {
    assert!((a == b), "{msg}\n  a: {}\n  b: {}", hex32(a), hex32(b));
}

/// Results from parallel execution that can be compared deterministically.
#[derive(Clone)]
pub struct ParallelExecResult {
    /// The commit identifier (hash of the commit).
    pub commit_hash: Hash32,
    /// The resulting state root hash after execution.
    pub state_root: Hash32,
    /// Digest of the produced patch (operations applied).
    pub patch_digest: Hash32,
    /// Optional WebStateChange bytes for roundtrip verification.
    pub wsc_bytes: Option<Vec<u8>>,
}

/// Deterministic scenarios so we can scale tests without random blobs.
#[derive(Clone, Copy, Debug)]
pub enum ParallelScenario {
    /// Tiny graph with edges/attachments; good for correctness.
    Small,

    /// Lots of independent rewrites; good for throughput/parallel admission.
    ManyIndependent,

    /// High collision rate; ensures admission/rejection is deterministic.
    ManyConflicts,

    /// Deletes/unlinks + attachments; exercises unlink semantics.
    DeletesAndAttachments,

    /// Privacy claims/proofs; mind-mode rules.
    PrivacyClaims,
}

/// Snapshot state for parallel execution compliance tests.
pub struct ParallelSnapshot {
    /// GraphStore holding the snapshot data.
    pub store: GraphStore,
    /// NodeId for the scenario root node.
    pub root: NodeId,
    /// ParallelScenario describing the test setup.
    pub scenario: ParallelScenario,
}

/// Ingress item: (rule_name, scope_node_id)
pub type IngressItem = (&'static str, NodeId);

/// A minimal test façade so tests don't hard-couple to evolving parallel execution API.
/// Implement this once (or provide a real harness builder).
pub trait ParallelTestHarness {
    type Snapshot;
    type IngressItem;

    /// Build a base snapshot (reachable-only) from a deterministic scenario.
    fn build_base_snapshot(&self, scenario: ParallelScenario) -> Self::Snapshot;

    /// Generate canonical ingress for a scenario and tick.
    fn make_ingress(&self, scenario: ParallelScenario, tick: u64) -> Vec<Self::IngressItem>;

    /// Execute with 1 worker (serial path).
    fn execute_serial(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        tick: u64,
    ) -> ParallelExecResult;

    /// Execute with N workers (parallel path).
    fn execute_parallel(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        tick: u64,
        workers: usize,
    ) -> ParallelExecResult;

    /// Verify WSC roundtrip yields same state_root.
    fn wsc_roundtrip_state_root(&self, wsc: &[u8]) -> Hash32;
}

/// Returns the real `EngineHarness` for parallel execution compliance tests.
pub fn parallel_harness() -> impl ParallelTestHarness {
    EngineHarness
}

/// Real parallel execution test harness backed by `warp_core::Engine`.
pub struct EngineHarness;

/// Rule name used by the parallel test harness.
const PARALLEL_TOUCH_RULE_NAME: &str = "parallel/touch";

/// Marker type ID for the parallel touch attachment.
fn parallel_marker_type_id() -> warp_core::TypeId {
    make_type_id("parallel/marker")
}

/// Create the "parallel/touch" rule that sets a marker attachment on the scope node.
fn make_parallel_touch_rule() -> RewriteRule {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(PARALLEL_TOUCH_RULE_NAME.as_bytes());
    let id: Hash = hasher.finalize().into();

    RewriteRule {
        id,
        name: PARALLEL_TOUCH_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |view, scope| {
            // Match if the node exists
            view.node(scope).is_some()
        },
        executor: |view, scope, delta| {
            // Phase 5: read from view, emit ops to delta (no direct mutation).
            let marker_payload = AtomPayload::new(
                parallel_marker_type_id(),
                bytes::Bytes::from_static(b"touched"),
            );
            let value = Some(AttachmentValue::Atom(marker_payload));

            let key = AttachmentKey::node_alpha(NodeKey {
                warp_id: view.warp_id(),
                local_id: *scope,
            });
            delta.push(WarpOp::SetAttachment { key, value });
        },
        compute_footprint: |view, scope| {
            let mut a_write = AttachmentSet::default();
            if view.node(scope).is_some() {
                a_write.insert(AttachmentKey::node_alpha(NodeKey {
                    warp_id: view.warp_id(),
                    local_id: *scope,
                }));
            }
            Footprint {
                n_read: warp_core::NodeSet::default(),
                n_write: warp_core::NodeSet::default(),
                e_read: warp_core::EdgeSet::default(),
                e_write: warp_core::EdgeSet::default(),
                a_read: AttachmentSet::default(),
                a_write,
                b_in: warp_core::PortSet::default(),
                b_out: warp_core::PortSet::default(),
                factor_mask: 1,
            }
        },
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Create a deterministic node ID for parallel execution tests.
fn parallel_node_id(label: &str) -> NodeId {
    make_node_id(label)
}

/// Create a deterministic edge ID for parallel execution tests.
fn parallel_edge_id(label: &str) -> warp_core::EdgeId {
    make_edge_id(label)
}

impl ParallelTestHarness for EngineHarness {
    type Snapshot = ParallelSnapshot;
    type IngressItem = IngressItem;

    fn build_base_snapshot(&self, scenario: ParallelScenario) -> Self::Snapshot {
        let node_ty = make_type_id("parallel/node");
        let edge_ty = make_type_id("parallel/edge");
        let attachment_ty = make_type_id("parallel/attachment");

        let mut store = GraphStore::default();

        // Create deterministic graphs based on scenario
        let root = match scenario {
            ParallelScenario::Small => {
                // 5 nodes, 4 edges, some attachments
                let n0 = parallel_node_id("small/n0");
                let n1 = parallel_node_id("small/n1");
                let n2 = parallel_node_id("small/n2");
                let n3 = parallel_node_id("small/n3");
                let n4 = parallel_node_id("small/n4");

                store.insert_node(n0, NodeRecord { ty: node_ty });
                store.insert_node(n1, NodeRecord { ty: node_ty });
                store.insert_node(n2, NodeRecord { ty: node_ty });
                store.insert_node(n3, NodeRecord { ty: node_ty });
                store.insert_node(n4, NodeRecord { ty: node_ty });

                // 4 edges: n0->n1, n1->n2, n2->n3, n3->n4
                store.insert_edge(
                    n0,
                    EdgeRecord {
                        id: parallel_edge_id("small/e0"),
                        from: n0,
                        to: n1,
                        ty: edge_ty,
                    },
                );
                store.insert_edge(
                    n1,
                    EdgeRecord {
                        id: parallel_edge_id("small/e1"),
                        from: n1,
                        to: n2,
                        ty: edge_ty,
                    },
                );
                store.insert_edge(
                    n2,
                    EdgeRecord {
                        id: parallel_edge_id("small/e2"),
                        from: n2,
                        to: n3,
                        ty: edge_ty,
                    },
                );
                store.insert_edge(
                    n3,
                    EdgeRecord {
                        id: parallel_edge_id("small/e3"),
                        from: n3,
                        to: n4,
                        ty: edge_ty,
                    },
                );

                // Some attachments on n1 and n3
                let payload = AtomPayload::new(attachment_ty, bytes::Bytes::from_static(b"data"));
                store.set_node_attachment(n1, Some(AttachmentValue::Atom(payload.clone())));
                store.set_node_attachment(n3, Some(AttachmentValue::Atom(payload)));

                n0
            }

            ParallelScenario::ManyIndependent => {
                // 20 disjoint nodes (no edges between them)
                let n0 = parallel_node_id("indep/n0");
                store.insert_node(n0, NodeRecord { ty: node_ty });

                for i in 1..20 {
                    let node = parallel_node_id(&format!("indep/n{i}"));
                    store.insert_node(node, NodeRecord { ty: node_ty });
                }

                n0
            }

            ParallelScenario::ManyConflicts => {
                // 10 nodes all sharing attachment on node 0
                let n0 = parallel_node_id("conflict/n0");
                store.insert_node(n0, NodeRecord { ty: node_ty });

                // Add shared attachment on n0
                let shared_payload =
                    AtomPayload::new(attachment_ty, bytes::Bytes::from_static(b"shared"));
                store.set_node_attachment(n0, Some(AttachmentValue::Atom(shared_payload)));

                // Add 9 more nodes that point to n0
                for i in 1..10 {
                    let node = parallel_node_id(&format!("conflict/n{i}"));
                    store.insert_node(node, NodeRecord { ty: node_ty });
                    store.insert_edge(
                        node,
                        EdgeRecord {
                            id: parallel_edge_id(&format!("conflict/e{i}")),
                            from: node,
                            to: n0,
                            ty: edge_ty,
                        },
                    );
                }

                n0
            }

            ParallelScenario::DeletesAndAttachments => {
                // 5 nodes with attachments to delete
                let n0 = parallel_node_id("delete/n0");
                store.insert_node(n0, NodeRecord { ty: node_ty });

                for i in 1..5 {
                    let node = parallel_node_id(&format!("delete/n{i}"));
                    store.insert_node(node, NodeRecord { ty: node_ty });

                    // Each node has an attachment
                    let payload = AtomPayload::new(
                        attachment_ty,
                        bytes::Bytes::from(format!("delete-data-{i}")),
                    );
                    store.set_node_attachment(node, Some(AttachmentValue::Atom(payload)));

                    // Edge from n0 to each
                    store.insert_edge(
                        n0,
                        EdgeRecord {
                            id: parallel_edge_id(&format!("delete/e{i}")),
                            from: n0,
                            to: node,
                            ty: edge_ty,
                        },
                    );
                }

                n0
            }

            ParallelScenario::PrivacyClaims => {
                // Same as Small for now (placeholder)
                let n0 = parallel_node_id("privacy/n0");
                let n1 = parallel_node_id("privacy/n1");
                let n2 = parallel_node_id("privacy/n2");
                let n3 = parallel_node_id("privacy/n3");
                let n4 = parallel_node_id("privacy/n4");

                store.insert_node(n0, NodeRecord { ty: node_ty });
                store.insert_node(n1, NodeRecord { ty: node_ty });
                store.insert_node(n2, NodeRecord { ty: node_ty });
                store.insert_node(n3, NodeRecord { ty: node_ty });
                store.insert_node(n4, NodeRecord { ty: node_ty });

                store.insert_edge(
                    n0,
                    EdgeRecord {
                        id: parallel_edge_id("privacy/e0"),
                        from: n0,
                        to: n1,
                        ty: edge_ty,
                    },
                );
                store.insert_edge(
                    n1,
                    EdgeRecord {
                        id: parallel_edge_id("privacy/e1"),
                        from: n1,
                        to: n2,
                        ty: edge_ty,
                    },
                );
                store.insert_edge(
                    n2,
                    EdgeRecord {
                        id: parallel_edge_id("privacy/e2"),
                        from: n2,
                        to: n3,
                        ty: edge_ty,
                    },
                );
                store.insert_edge(
                    n3,
                    EdgeRecord {
                        id: parallel_edge_id("privacy/e3"),
                        from: n3,
                        to: n4,
                        ty: edge_ty,
                    },
                );

                n0
            }
        };

        ParallelSnapshot {
            store,
            root,
            scenario,
        }
    }

    fn make_ingress(&self, scenario: ParallelScenario, _tick: u64) -> Vec<Self::IngressItem> {
        // Ignore tick for now (keep constant per scenario)
        match scenario {
            ParallelScenario::Small => {
                // Touch nodes n0, n1, n2, n3, n4
                vec![
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("small/n0")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("small/n1")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("small/n2")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("small/n3")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("small/n4")),
                ]
            }

            ParallelScenario::ManyIndependent => {
                // Touch all 20 independent nodes
                (0..20)
                    .map(|i| {
                        (
                            PARALLEL_TOUCH_RULE_NAME,
                            parallel_node_id(&format!("indep/n{i}")),
                        )
                    })
                    .collect()
            }

            ParallelScenario::ManyConflicts => {
                // Touch all 10 nodes (they all share attachment on n0)
                (0..10)
                    .map(|i| {
                        (
                            PARALLEL_TOUCH_RULE_NAME,
                            parallel_node_id(&format!("conflict/n{i}")),
                        )
                    })
                    .collect()
            }

            ParallelScenario::DeletesAndAttachments => {
                // Touch all 5 nodes
                (0..5)
                    .map(|i| {
                        (
                            PARALLEL_TOUCH_RULE_NAME,
                            parallel_node_id(&format!("delete/n{i}")),
                        )
                    })
                    .collect()
            }

            ParallelScenario::PrivacyClaims => {
                // Touch nodes n0, n1, n2, n3, n4
                vec![
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("privacy/n0")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("privacy/n1")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("privacy/n2")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("privacy/n3")),
                    (PARALLEL_TOUCH_RULE_NAME, parallel_node_id("privacy/n4")),
                ]
            }
        }
    }

    fn execute_serial(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        _tick: u64,
    ) -> ParallelExecResult {
        // Clone the base store
        let store = base.store.clone();

        // Create Engine with EngineBuilder
        let mut engine = EngineBuilder::new(store, base.root).build();

        // Register the "parallel/touch" rule
        engine
            .register_rule(make_parallel_touch_rule())
            .expect("failed to register parallel/touch rule");

        // Begin transaction
        let tx = engine.begin();

        // Apply each ingress item (NoMatch is expected for non-matching rules)
        for (rule_name, scope) in ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {
                    // NoMatch is expected for non-matching rules, continue
                }
                Err(e) => panic!("unexpected error applying rule '{rule_name}': {e:?}"),
            }
        }

        // Commit and get receipt
        let (snapshot, _receipt, _patch) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        ParallelExecResult {
            commit_hash: snapshot.hash,
            state_root: snapshot.state_root,
            patch_digest: snapshot.patch_digest,
            // Phase 2: WSC bytes
            wsc_bytes: None,
        }
    }

    #[allow(unused_variables)]
    fn execute_parallel(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        _tick: u64,
        workers: usize,
    ) -> ParallelExecResult {
        // Clone the base store
        let store = base.store.clone();

        // Create Engine with specified worker count via EngineBuilder
        let mut engine = EngineBuilder::new(store, base.root)
            .workers(workers)
            .build();

        // Register the "parallel/touch" rule
        engine
            .register_rule(make_parallel_touch_rule())
            .expect("failed to register parallel/touch rule");

        // Begin transaction
        let tx = engine.begin();

        // Apply each ingress item (NoMatch is expected for non-matching rules)
        for (rule_name, scope) in ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {
                    // NoMatch is expected for non-matching rules, continue
                }
                Err(e) => panic!("unexpected error applying rule '{rule_name}': {e:?}"),
            }
        }

        // Commit and get receipt
        let (snapshot, _receipt, _patch) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        ParallelExecResult {
            commit_hash: snapshot.hash,
            state_root: snapshot.state_root,
            patch_digest: snapshot.patch_digest,
            // Phase 2: WSC bytes
            wsc_bytes: None,
        }
    }

    fn wsc_roundtrip_state_root(&self, _wsc: &[u8]) -> Hash32 {
        // WSC roundtrip not yet implemented (SnapshotBuilder does not produce wsc_bytes yet).
        // Fail fast so callers discover missing implementation immediately.
        unimplemented!(
            "wsc_roundtrip_state_root: implement once SnapshotBuilder produces wsc_bytes"
        )
    }
}

/// Compute the snapshot hash for a graph rooted at the given node.
///
/// Constructs an [`Engine`] from the provided `store` and `root`, then
/// returns the 32-byte hash of its current snapshot.
///
/// # Panics
///
/// Panics if the `root` node does not exist in `store`.
pub fn snapshot_hash_of(store: GraphStore, root: NodeId) -> [u8; 32] {
    let engine = Engine::new(store, root);
    engine.snapshot().hash
}

// =============================================================================
// MATERIALIZATION TEST UTILITIES
// =============================================================================

use warp_core::materialization::EmitKey;

/// Create a deterministic 32-byte hash for tests.
/// Sets the last byte to `n`, all other bytes are zero.
pub fn h(n: u8) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[31] = n;
    bytes
}

/// Create an EmitKey for tests with subkey=0.
pub fn key(scope: u8, rule: u32) -> EmitKey {
    EmitKey::new(h(scope), rule)
}

/// Create an EmitKey for tests with explicit subkey.
pub fn key_sub(scope: u8, rule: u32, subkey: u32) -> EmitKey {
    EmitKey::with_subkey(h(scope), rule, subkey)
}

// =============================================================================
// WORLDLINE / PLAYBACK TEST UTILITIES (SPEC-0004)
// =============================================================================

/// Creates a deterministic worldline ID for testing.
pub fn test_worldline_id() -> WorldlineId {
    WorldlineId::from_bytes([1u8; 32])
}

/// Creates a deterministic cursor ID for testing.
pub fn test_cursor_id(n: u8) -> CursorId {
    CursorId([n; 32])
}

/// Creates a deterministic session ID for testing.
pub fn test_session_id(n: u8) -> SessionId {
    SessionId([n; 32])
}

/// Creates a test warp ID.
pub fn test_warp_id() -> WarpId {
    make_warp_id("test-warp")
}

/// Creates a test header for a specific explicit commit-global tick.
pub fn test_header(commit_global_tick: GlobalTick) -> WorldlineTickHeaderV1 {
    WorldlineTickHeaderV1 {
        commit_global_tick,
        policy_id: 0,
        rule_pack_id: [0u8; 32],
        plan_digest: [0u8; 32],
        decision_digest: [0u8; 32],
        rewrites_digest: [0u8; 32],
    }
}

/// Creates an initial store with a root node.
pub fn create_initial_store(warp_id: WarpId) -> GraphStore {
    let mut store = GraphStore::new(warp_id);
    let root_id = make_node_id("root");
    let ty = make_type_id("RootType");
    store.insert_node(root_id, NodeRecord { ty });
    store
}

/// Creates an initial full worldline state with a deterministic root node.
pub fn create_initial_worldline_state(warp_id: WarpId) -> WorldlineState {
    let store = create_initial_store(warp_id);
    let root_id = make_node_id("root");
    WorldlineState::from_root_store(store, root_id)
        .expect("initial worldline state should be valid")
}

/// Registers a test worldline with its real deterministic initial boundary hash.
pub fn register_fixture_worldline(
    provenance: &mut LocalProvenanceStore,
    worldline_id: WorldlineId,
    initial_state: &WorldlineState,
) -> Result<(), warp_core::HistoryError> {
    provenance.register_worldline_with_boundary(
        worldline_id,
        initial_state.root().warp_id,
        initial_state.state_root(),
    )
}

/// Creates a patch that adds a node at a specific worldline tick and explicit
/// commit-global tick.
pub fn create_add_node_patch(
    warp_id: WarpId,
    tick: u64,
    commit_global_tick: GlobalTick,
    node_name: &str,
) -> WorldlineTickPatchV1 {
    let node_id = make_node_id(node_name);
    let node_key = NodeKey {
        warp_id,
        local_id: node_id,
    };
    let ty = make_type_id(&format!("Type{tick}"));
    let ops = vec![WarpOp::UpsertNode {
        node: node_key,
        record: NodeRecord { ty },
    }];
    let header = test_header(commit_global_tick);
    let patch_digest = WarpTickPatchV1::new(
        header.policy_id,
        header.rule_pack_id,
        TickCommitStatus::Committed,
        Vec::new(),
        Vec::new(),
        ops.clone(),
    )
    .digest();

    WorldlineTickPatchV1 {
        header,
        warp_id,
        ops,
        in_slots: vec![],
        out_slots: vec![],
        patch_digest,
    }
}

/// Sets up a worldline with N ticks and returns the provenance store and initial store.
///
/// Commit hashes are computed using `compute_commit_hash_v2` to form a valid Merkle chain,
/// matching what `seek_to` will recompute during verification.
pub fn setup_worldline_with_ticks(
    num_ticks: u64,
) -> (LocalProvenanceStore, WorldlineState, WarpId, WorldlineId) {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_state = create_initial_worldline_state(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state).unwrap();

    // Build up the worldline by applying patches and recording correct hashes
    let mut current_state = initial_state.clone();
    let mut parents: Vec<Hash> = Vec::new();

    for tick in 0..num_ticks {
        let patch = create_add_node_patch(
            warp_id,
            tick,
            GlobalTick::from_raw(tick + 1),
            &format!("node-{tick}"),
        );

        // Apply patch to get the resulting state
        patch
            .apply_to_worldline_state(&mut current_state)
            .expect("apply should succeed");

        // Compute the actual state root after applying
        let state_root = current_state.state_root();

        // Compute real commit_hash for Merkle chain verification
        let commit_hash = compute_commit_hash_v2(
            &state_root,
            &parents,
            &patch.patch_digest,
            patch.header.policy_id,
        );

        let triplet = HashTriplet {
            state_root,
            patch_digest: patch.patch_digest,
            commit_hash,
        };

        append_fixture_entry(&mut provenance, worldline_id, patch, triplet, vec![])
            .expect("append should succeed");

        // Advance parent chain
        parents = vec![commit_hash];
    }

    (provenance, initial_state, warp_id, worldline_id)
}

/// Returns the canonical writer head key used by provenance test fixtures.
#[must_use]
pub fn fixture_head_key(worldline_id: WorldlineId) -> WriterHeadKey {
    WriterHeadKey {
        worldline_id,
        head_id: make_head_id("fixture"),
    }
}

/// Builds a fixture provenance entry from a worldline patch/triplet pair.
///
/// Parent refs are derived from the current provenance tip so the resulting
/// entry forms a valid deterministic linear history for test replay.
pub fn fixture_entry(
    provenance: &LocalProvenanceStore,
    worldline_id: WorldlineId,
    patch: WorldlineTickPatchV1,
    expected: HashTriplet,
    outputs: OutputFrameSet,
    atom_writes: AtomWriteSet,
) -> Result<ProvenanceEntry, warp_core::HistoryError> {
    let commit_global_tick = patch.commit_global_tick();
    let worldline_tick = WorldlineTick::from_raw(provenance.len(worldline_id)?);
    let parents = provenance.tip_ref(worldline_id)?.into_iter().collect();
    Ok(ProvenanceEntry::local_commit(
        worldline_id,
        worldline_tick,
        commit_global_tick,
        fixture_head_key(worldline_id),
        parents,
        expected,
        patch,
        outputs,
        atom_writes,
    ))
}

/// Appends a fixture provenance entry using the entry-based Phase 4 API.
pub fn append_fixture_entry(
    provenance: &mut LocalProvenanceStore,
    worldline_id: WorldlineId,
    patch: WorldlineTickPatchV1,
    expected: HashTriplet,
    outputs: OutputFrameSet,
) -> Result<(), warp_core::HistoryError> {
    append_fixture_entry_with_writes(
        provenance,
        worldline_id,
        patch,
        expected,
        outputs,
        Vec::new(),
    )
}

/// Appends a fixture provenance entry with explicit atom-write provenance.
pub fn append_fixture_entry_with_writes(
    provenance: &mut LocalProvenanceStore,
    worldline_id: WorldlineId,
    patch: WorldlineTickPatchV1,
    expected: HashTriplet,
    outputs: OutputFrameSet,
    atom_writes: AtomWriteSet,
) -> Result<(), warp_core::HistoryError> {
    let entry = fixture_entry(
        provenance,
        worldline_id,
        patch,
        expected,
        outputs,
        atom_writes,
    )?;
    provenance.append_local_commit(entry)
}

/// Creates a "touch" rewrite rule for worker invariance tests.
///
/// The rule sets a marker attachment on the scope node, exercising the
/// Parallel parallel execution path while remaining deterministic.
///
/// Because `RewriteRule` fields are function pointers (not closures), parameters
/// must be string/byte literals known at compile time. Use this macro to avoid
/// duplicating the 47-line rule body across tests.
///
/// # Usage
/// ```ignore
/// let rule = make_touch_rule!("t16/touch", "t16/marker", b"touched-t16");
/// ```
#[macro_export]
macro_rules! make_touch_rule {
    ($rule_name:expr, $marker_type:expr, $marker_bytes:expr) => {{
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"rule:");
        hasher.update(b"t16s/touch");
        let id: warp_core::Hash = hasher.finalize().into();

        warp_core::RewriteRule {
            id,
            name: $rule_name,
            left: warp_core::PatternGraph { nodes: vec![] },
            matcher: |view, scope| view.node(scope).is_some(),
            executor: |view, scope, delta| {
                let marker_payload = warp_core::AtomPayload::new(
                    warp_core::make_type_id($marker_type),
                    bytes::Bytes::from_static($marker_bytes),
                );
                let value = Some(warp_core::AttachmentValue::Atom(marker_payload));
                let key = warp_core::AttachmentKey::node_alpha(warp_core::NodeKey {
                    warp_id: view.warp_id(),
                    local_id: *scope,
                });
                delta.push(warp_core::WarpOp::SetAttachment { key, value });
            },
            compute_footprint: |view, scope| {
                let mut a_write = warp_core::AttachmentSet::default();
                if view.node(scope).is_some() {
                    a_write.insert(warp_core::AttachmentKey::node_alpha(warp_core::NodeKey {
                        warp_id: view.warp_id(),
                        local_id: *scope,
                    }));
                }
                warp_core::Footprint {
                    n_read: warp_core::NodeSet::default(),
                    n_write: warp_core::NodeSet::default(),
                    e_read: warp_core::EdgeSet::default(),
                    e_write: warp_core::EdgeSet::default(),
                    a_read: warp_core::AttachmentSet::default(),
                    a_write,
                    b_in: warp_core::PortSet::default(),
                    b_out: warp_core::PortSet::default(),
                    factor_mask: 1,
                }
            },
            factor_mask: 1,
            conflict_policy: warp_core::ConflictPolicy::Abort,
            join_fn: None,
        }
    }};
}

/// Calls `f` for every permutation of `items` in-place.
/// Deterministic: Heap's algorithm generates all N! permutations.
pub fn for_each_permutation<T: Clone>(items: &mut [T], mut f: impl FnMut(&[T])) {
    let n = items.len();
    if n == 0 {
        f(items);
        return;
    }

    let mut c = vec![0usize; n];

    // First permutation (original order)
    f(items);

    let mut i = 0usize;
    while i < n {
        if c[i] < i {
            if i.is_multiple_of(2) {
                items.swap(0, i);
            } else {
                items.swap(c[i], i);
            }
            f(items);
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
}
