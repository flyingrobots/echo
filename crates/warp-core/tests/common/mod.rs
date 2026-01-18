// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(dead_code)]

use warp_core::{Engine, GraphStore, NodeId};

// =============================================================================
// BOAW COMPLIANCE TEST UTILITIES (ADR-0007)
// =============================================================================

/// 32-byte hash type alias for clarity.
pub type Hash32 = [u8; 32];

/// Tiny deterministic RNG (xorshift64*) so tests don't need `rand`.
#[derive(Clone)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

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

pub fn hex32(h: &Hash32) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// For comparing hashes with readable diffs.
pub fn assert_hash_eq(a: &Hash32, b: &Hash32, msg: &str) {
    if a != b {
        panic!("{msg}\n  a: {}\n  b: {}", hex32(a), hex32(b));
    }
}

/// Results from BOAW execution that can be compared deterministically.
#[derive(Clone)]
pub struct BoawExecResult {
    pub commit_hash: Hash32,
    pub state_root: Hash32,
    pub patch_digest: Hash32,
    pub wsc_bytes: Option<Vec<u8>>,
}

/// Deterministic scenarios so we can scale tests without random blobs.
#[derive(Clone, Copy, Debug)]
pub enum BoawScenario {
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

/// A minimal test façade so tests don't hard-couple to evolving BOAW API.
/// Implement this once (or provide a real harness builder).
pub trait BoawTestHarness {
    type Snapshot;
    type IngressItem;

    /// Build a base snapshot (reachable-only) from a deterministic scenario.
    fn build_base_snapshot(&self, scenario: BoawScenario) -> Self::Snapshot;

    /// Generate canonical ingress for a scenario and tick.
    fn make_ingress(&self, scenario: BoawScenario, tick: u64) -> Vec<Self::IngressItem>;

    /// Execute with 1 worker (serial path).
    fn execute_serial(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        tick: u64,
    ) -> BoawExecResult;

    /// Execute with N workers (parallel path).
    fn execute_parallel(
        &self,
        base: &Self::Snapshot,
        ingress: &[Self::IngressItem],
        tick: u64,
        workers: usize,
    ) -> BoawExecResult;

    /// Verify WSC roundtrip yields same state_root.
    fn wsc_roundtrip_state_root(&self, wsc: &[u8]) -> Hash32;
}

/// Temporary default harness so tests compile immediately.
/// Replace this by constructing your real engine harness.
pub fn boaw_harness() -> impl BoawTestHarness {
    PanicHarness
}

struct PanicHarness;

impl BoawTestHarness for PanicHarness {
    type Snapshot = ();
    type IngressItem = ();

    fn build_base_snapshot(&self, _scenario: BoawScenario) -> Self::Snapshot {}

    fn make_ingress(&self, _scenario: BoawScenario, _tick: u64) -> Vec<Self::IngressItem> {
        vec![()]
    }

    fn execute_serial(
        &self,
        _base: &Self::Snapshot,
        _ingress: &[Self::IngressItem],
        _tick: u64,
    ) -> BoawExecResult {
        unimplemented!("wire BoawTestHarness::execute_serial to BOAW engine")
    }

    fn execute_parallel(
        &self,
        _base: &Self::Snapshot,
        _ingress: &[Self::IngressItem],
        _tick: u64,
        _workers: usize,
    ) -> BoawExecResult {
        unimplemented!("wire BoawTestHarness::execute_parallel to BOAW engine")
    }

    fn wsc_roundtrip_state_root(&self, _wsc: &[u8]) -> Hash32 {
        unimplemented!("wire BoawTestHarness::wsc_roundtrip_state_root to WSC reader")
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
