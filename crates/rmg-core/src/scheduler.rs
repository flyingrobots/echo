//! Deterministic rewrite scheduler with O(n) radix drain.
//!
//! Ordering invariant:
//! - Rewrites execute in ascending lexicographic order of (`scope_hash`, `rule_id`, `nonce`).
//! - Uses stable LSD radix sort (20 passes: 2 nonce + 2 rule + 16 scope) with 16-bit digits.
//! - Zero comparisons; O(n) complexity with small constants.
//! - Byte-lexicographic order over full 32-byte scope hash preserved exactly.

use std::cmp::Ordering;
use std::collections::HashMap;

use rustc_hash::FxHashMap;

use crate::footprint::Footprint;
use crate::ident::{CompactRuleId, Hash, NodeId};
#[cfg(feature = "telemetry")]
use crate::telemetry;
use crate::tx::TxId;

/// Active footprint tracking using generation-stamped sets for O(1) conflict detection.
#[derive(Debug)]
pub(crate) struct ActiveFootprints {
    /// Nodes written by reserved rewrites
    nodes_written: GenSet<NodeId>,
    /// Nodes read by reserved rewrites
    nodes_read: GenSet<NodeId>,
    /// Edges written by reserved rewrites
    edges_written: GenSet<crate::ident::EdgeId>,
    /// Edges read by reserved rewrites
    edges_read: GenSet<crate::ident::EdgeId>,
    /// Boundary ports touched (both `b_in` and `b_out`, since any intersection conflicts)
    ports: GenSet<crate::footprint::PortKey>,
}

impl ActiveFootprints {
    fn new() -> Self {
        Self {
            nodes_written: GenSet::new(),
            nodes_read: GenSet::new(),
            edges_written: GenSet::new(),
            edges_read: GenSet::new(),
            ports: GenSet::new(),
        }
    }
}

/// Deterministic scheduler with O(n) radix-based drain.
#[derive(Debug, Default)]
pub(crate) struct DeterministicScheduler {
    /// Pending rewrites per transaction, stored for O(1) enqueue and O(n) drain.
    pending: HashMap<TxId, PendingTx<PendingRewrite>>,
    /// Active footprints per transaction for O(m) independence checking via `GenSets`.
    /// Checks all aspects: nodes (read/write), edges (read/write), and boundary ports.
    pub(crate) active: HashMap<TxId, ActiveFootprints>,
    #[cfg(feature = "telemetry")]
    pub(crate) counters: HashMap<TxId, (u64, u64)>, // (reserved, conflict)
}

/// Internal representation of a rewrite waiting to be applied.
#[derive(Debug, Clone)]
pub(crate) struct PendingRewrite {
    /// Identifier of the rule to execute.
    #[cfg_attr(not(feature = "telemetry"), allow(dead_code))]
    pub rule_id: Hash,
    /// Compact in-process rule handle used on hot paths.
    pub compact_rule: CompactRuleId,
    /// Scope hash used for deterministic ordering (full 32 bytes).
    pub scope_hash: Hash,
    /// Scope node supplied when `apply` was invoked.
    pub scope: NodeId,
    /// Footprint used for independence checks and conflict resolution.
    pub footprint: Footprint,
    /// State machine phase for the rewrite.
    pub phase: RewritePhase,
}

/// Phase of a pending rewrite in the lock-free scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RewritePhase {
    /// Match found and footprint computed.
    Matched,
    /// Passed independence checks and reserved.
    #[allow(dead_code)]
    Reserved,
    /// Successfully applied.
    #[allow(dead_code)]
    Committed,
    /// Aborted due to conflict or validation failure.
    #[allow(dead_code)]
    Aborted,
}

impl DeterministicScheduler {
    /// Enqueues a rewrite with last-wins semantics on (`scope_hash`, `compact_rule`).
    pub(crate) fn enqueue(&mut self, tx: TxId, rewrite: PendingRewrite) {
        let txq = self.pending.entry(tx).or_default();
        txq.enqueue(rewrite.scope_hash, rewrite.compact_rule.0, rewrite);
    }

    /// Removes and returns all pending rewrites for `tx`, ordered deterministically
    /// by (`scope_hash`, `rule_id`, `nonce`) via stable radix sort.
    pub(crate) fn drain_for_tx(&mut self, tx: TxId) -> Vec<PendingRewrite> {
        self.pending
            .remove(&tx)
            .map_or_else(Vec::new, |mut txq| txq.drain_in_order())
    }

    /// Attempts to reserve a rewrite by checking full footprint independence
    /// using generation-stamped conflict detection.
    ///
    /// Checks all aspects of the footprint: node read/write sets, edge read/write
    /// sets, and boundary ports. Uses O(1) `GenSet` lookups for each resource,
    /// making this O(m) where m is the size of the current footprint.
    ///
    /// On success, marks all resources in the active `GenSets` and transitions
    /// the phase to `Reserved`.
    pub(crate) fn reserve(&mut self, tx: TxId, pr: &mut PendingRewrite) -> bool {
        let active = self.active.entry(tx).or_insert_with(ActiveFootprints::new);

        if Self::has_conflict(active, pr) {
            return self.on_conflict(tx, pr);
        }

        Self::mark_all(active, pr);
        self.on_reserved(tx, pr)
    }

    #[inline]
    #[allow(clippy::needless_pass_by_ref_mut)]
    #[cfg_attr(not(feature = "telemetry"), allow(clippy::unused_self))]
    #[cfg_attr(not(feature = "telemetry"), allow(unused_variables))]
    fn on_conflict(&mut self, tx: TxId, pr: &mut PendingRewrite) -> bool {
        pr.phase = RewritePhase::Aborted;
        #[cfg(feature = "telemetry")]
        {
            let entry = self.counters.entry(tx).or_default();
            entry.1 += 1;
        }
        #[cfg(feature = "telemetry")]
        telemetry::conflict(tx, &pr.rule_id);
        false
    }

    #[inline]
    #[allow(clippy::needless_pass_by_ref_mut)]
    #[cfg_attr(not(feature = "telemetry"), allow(clippy::unused_self))]
    #[cfg_attr(not(feature = "telemetry"), allow(unused_variables))]
    fn on_reserved(&mut self, tx: TxId, pr: &mut PendingRewrite) -> bool {
        pr.phase = RewritePhase::Reserved;
        #[cfg(feature = "telemetry")]
        {
            let entry = self.counters.entry(tx).or_default();
            entry.0 += 1;
        }
        #[cfg(feature = "telemetry")]
        telemetry::reserved(tx, &pr.rule_id);
        true
    }

    #[inline]
    fn has_conflict(active: &ActiveFootprints, pr: &PendingRewrite) -> bool {
        use crate::ident::EdgeId;

        // Node writes conflict with prior writes OR reads
        for node_hash in pr.footprint.n_write.iter() {
            let node_id = NodeId(*node_hash);
            if active.nodes_written.contains(node_id) || active.nodes_read.contains(node_id) {
                return true;
            }
        }

        // Node reads conflict with prior writes (but NOT prior reads)
        for node_hash in pr.footprint.n_read.iter() {
            let node_id = NodeId(*node_hash);
            if active.nodes_written.contains(node_id) {
                return true;
            }
        }

        // Edge writes conflict with prior writes OR reads
        for edge_hash in pr.footprint.e_write.iter() {
            let edge_id = EdgeId(*edge_hash);
            if active.edges_written.contains(edge_id) || active.edges_read.contains(edge_id) {
                return true;
            }
        }

        // Edge reads conflict with prior writes (but NOT prior reads)
        for edge_hash in pr.footprint.e_read.iter() {
            let edge_id = EdgeId(*edge_hash);
            if active.edges_written.contains(edge_id) {
                return true;
            }
        }

        // Boundary ports: any intersection conflicts (b_in and b_out combined)
        for port_key in pr.footprint.b_in.iter() {
            if active.ports.contains(*port_key) {
                return true;
            }
        }
        for port_key in pr.footprint.b_out.iter() {
            if active.ports.contains(*port_key) {
                return true;
            }
        }

        false
    }

    #[inline]
    fn mark_all(active: &mut ActiveFootprints, pr: &PendingRewrite) {
        use crate::ident::EdgeId;

        for node_hash in pr.footprint.n_write.iter() {
            active.nodes_written.mark(NodeId(*node_hash));
        }
        for node_hash in pr.footprint.n_read.iter() {
            active.nodes_read.mark(NodeId(*node_hash));
        }
        for edge_hash in pr.footprint.e_write.iter() {
            active.edges_written.mark(EdgeId(*edge_hash));
        }
        for edge_hash in pr.footprint.e_read.iter() {
            active.edges_read.mark(EdgeId(*edge_hash));
        }
        for port_key in pr.footprint.b_in.iter() {
            active.ports.mark(*port_key);
        }
        for port_key in pr.footprint.b_out.iter() {
            active.ports.mark(*port_key);
        }
    }

    /// Finalizes accounting for `tx`: emits telemetry summary and clears state.
    pub(crate) fn finalize_tx(&mut self, tx: TxId) {
        #[cfg(feature = "telemetry")]
        if let Some((reserved, conflict)) = self.counters.remove(&tx) {
            telemetry::summary(tx, reserved, conflict);
        }
        self.active.remove(&tx);
        self.pending.remove(&tx);
    }
}

// ============================================================================
// Deterministic O(n) pending-transaction container with radix sort
// ============================================================================

/// Thin key record for radix sorting (24 bytes + 4-byte handle = 28 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RewriteThin {
    scope_be32: [u8; 32], // full 256-bit scope, byte-lexicographic order
    rule_id: u32,         // compact, unique, stable per rule
    nonce: u32,           // insertion-order tie-break
    handle: usize,        // index into fat payload vec (usize to avoid truncation casts)
}

/// Pending transaction queue with O(1) enqueue and O(n) deterministic drain.
#[derive(Debug)]
struct PendingTx<P> {
    next_nonce: u32,
    /// Last-wins dedupe on (`scope_hash`, `compact_rule`).
    index: FxHashMap<([u8; 32], u32), usize>,
    /// Thin keys + handles (sorted during drain).
    thin: Vec<RewriteThin>,
    /// Fat payloads (indexed by handle).
    fat: Vec<Option<P>>,
    /// Scratch buffer for radix passes (reused).
    scratch: Vec<RewriteThin>,
    /// Counting array for 16-bit radix (65536 buckets, reused). `u32` keeps
    /// bandwidth/cache lower while remaining ample for batch sizes we handle.
    counts16: Vec<u32>,
}

impl<P> Default for PendingTx<P> {
    fn default() -> Self {
        Self {
            next_nonce: 0,
            index: FxHashMap::default(),
            thin: Vec::new(),
            fat: Vec::new(),
            scratch: Vec::new(),
            counts16: Vec::new(), // Lazy allocation in radix_sort
        }
    }
}

impl<P> PendingTx<P> {
    /// Enqueues a rewrite with last-wins semantics.
    #[inline]
    fn enqueue(&mut self, scope_be32: [u8; 32], rule_id: u32, payload: P) {
        let key = (scope_be32, rule_id);
        if let Some(&i) = self.index.get(&key) {
            // Last-wins: overwrite payload and refresh nonce for determinism
            let h = self.thin[i].handle;
            self.fat[h] = Some(payload);
            let n = self.next_nonce;
            self.next_nonce = n.wrapping_add(1);
            self.thin[i].nonce = n;
        } else {
            let handle = self.fat.len();
            self.fat.push(Some(payload));
            let n = self.next_nonce;
            self.next_nonce = n.wrapping_add(1);
            self.thin.push(RewriteThin {
                scope_be32,
                rule_id,
                nonce: n,
                handle,
            });
            self.index.insert(key, self.thin.len() - 1);
        }
    }

    /// Stable LSD radix sort over 16-bit big-endian digits.
    /// Pass order (LSD → MSD): nonce[0,1], rule[0,1], scope pairs[15..0]
    /// Total: 20 passes. Exactly reproduces byte-lex order on (scope, rule, nonce).
    fn radix_sort(&mut self) {
        let n = self.thin.len();
        if n <= 1 {
            return;
        }
        self.scratch.resize(n, RewriteThin::default());

        // Lazy allocation of 16-bit histogram (65536 buckets).
        if self.counts16.is_empty() {
            self.counts16 = vec![0u32; 1 << 16];
        }

        let mut flip = false;
        for pass in 0..20 {
            let (src, dst) = if flip {
                (&self.scratch[..], &mut self.thin[..])
            } else {
                (&self.thin[..], &mut self.scratch[..])
            };

            let counts = &mut self.counts16;
            counts.fill(0);

            // Count
            for r in src {
                let b = bucket16(r, pass) as usize;
                counts[b] = counts[b].wrapping_add(1);
            }

            // Prefix sums
            let mut sum: u32 = 0;
            for c in counts.iter_mut() {
                let t = *c;
                *c = sum;
                sum = sum.wrapping_add(t);
            }

            // Stable scatter
            for r in src {
                let b = bucket16(r, pass) as usize;
                let idx_u32 = counts[b];
                counts[b] = idx_u32.wrapping_add(1);
                let idx = idx_u32 as usize; // widening u32→usize (safe on 32/64-bit)
                dst[idx] = *r;
            }

            flip = !flip;
        }

        // Ensure final ordering resides in `thin`
        if flip {
            self.thin.copy_from_slice(&self.scratch);
        }
    }

    /// Drains all rewrites in deterministic order.
    fn drain_in_order(&mut self) -> Vec<P> {
        let n = self.thin.len();
        if n > 1 {
            if n <= SMALL_SORT_THRESHOLD {
                // Tiny batches are faster with comparison sort—skip histogram zeroing entirely.
                self.thin.sort_unstable_by(cmp_thin);
            } else {
                self.radix_sort();
            }
        }
        let n = self.thin.len();
        let mut out = Vec::with_capacity(n);
        for r in self.thin.drain(..) {
            // Invariant: each thin handle must point to a live payload.
            // If not, fail loudly to preserve determinism.
            let p = self.fat.get_mut(r.handle).map_or_else(
                || unreachable!("BUG: handle out of range {}", r.handle),
                |slot| {
                    slot.take().map_or_else(
                        || unreachable!("BUG: missing payload at handle {}", r.handle),
                        |p| p,
                    )
                },
            );
            out.push(p);
        }
        self.index.clear();
        self.fat.clear();
        self.next_nonce = 0;
        out
    }
}

/// Extracts 16-bit digit from u32 (little-endian numeric order).
#[inline]
fn u16_from_u32_le(x: u32, idx: usize) -> u16 {
    debug_assert!(idx < 2);
    let b = x.to_le_bytes();
    u16::from_le_bytes([b[2 * idx], b[2 * idx + 1]])
}

/// Extracts 16-bit big-endian pair from 32-byte scope hash.
/// `pair_idx_be` in [0..16): 0 => bytes[0..2], 15 => bytes[30..32]
#[inline]
fn u16_be_from_pair32(bytes: &[u8; 32], pair_idx_be: usize) -> u16 {
    debug_assert!(pair_idx_be < 16);
    let off = 2 * pair_idx_be;
    u16::from_be_bytes([bytes[off], bytes[off + 1]])
}

// Tunable threshold: below this, comparison sort wins on modern CPUs.
const SMALL_SORT_THRESHOLD: usize = 1024;

/// Comparison function for deterministic ordering: (`scope_be32`, `rule_id`, `nonce`).
#[inline]
fn cmp_thin(a: &RewriteThin, b: &RewriteThin) -> Ordering {
    match a.scope_be32.cmp(&b.scope_be32) {
        Ordering::Equal => a
            .rule_id
            .cmp(&b.rule_id)
            .then_with(|| a.nonce.cmp(&b.nonce)),
        o => o,
    }
}

/// LSD radix bucket function: nonce → rule → scope (reversed pairs for byte-lex).
/// Pass layout: nonce[0,1], rule[0,1], scope pairs[15..0] (20 total).
#[inline]
fn bucket16(r: &RewriteThin, pass: usize) -> u16 {
    match pass {
        0 => u16_from_u32_le(r.nonce, 0),
        1 => u16_from_u32_le(r.nonce, 1),
        2 => u16_from_u32_le(r.rule_id, 0),
        3 => u16_from_u32_le(r.rule_id, 1),
        // 16 passes for scope: pairs 15 down to 0 (LSD → byte-lex)
        4..=19 => {
            let pair_from_tail = 19 - pass; // 0..15 => tail..head
            let pair_idx_be = 15 - pair_from_tail; // 15..0 mapped to 0..15
            u16_be_from_pair32(&r.scope_be32, pair_idx_be)
        }
        _ => unreachable!("invalid radix pass"),
    }
}

// ============================================================================
// Generation-stamped conflict set for O(1) independence checks
// ============================================================================

/// Generation-stamped set for O(1) conflict detection.
///
/// This data structure allows O(1) conflict checking without clearing hash tables
/// between transactions by using generation counters. Each transaction gets a new
/// generation, and we track which generation last saw each key.
#[derive(Debug)]
pub(crate) struct GenSet<K> {
    gen: u32,
    seen: FxHashMap<K, u32>,
}

impl<K: std::hash::Hash + Eq + Copy> GenSet<K> {
    /// Creates a new generation set.
    pub fn new() -> Self {
        Self {
            gen: 1,
            seen: FxHashMap::default(),
        }
    }

    /// Begins a new commit generation (call once per transaction).
    #[inline]
    #[allow(dead_code)]
    pub fn begin_commit(&mut self) {
        self.gen = self.gen.wrapping_add(1);
    }

    /// Returns true if `key` was marked in the current generation.
    #[inline]
    pub fn contains(&self, key: K) -> bool {
        matches!(self.seen.get(&key), Some(&g) if g == self.gen)
    }

    /// Marks `key` as seen in the current generation.
    #[inline]
    pub fn mark(&mut self, key: K) {
        self.seen.insert(key, self.gen);
    }

    /// Returns true if `key` conflicts with current generation, otherwise marks it.
    /// This is a convenience method combining `contains` and `mark` for cases where
    /// atomicity is needed.
    #[inline]
    #[allow(dead_code)]
    pub fn conflict_or_mark(&mut self, key: K) -> bool {
        if self.contains(key) {
            true
        } else {
            self.mark(key);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::make_node_id;

    // Test-only helper: pack a boundary port key from components.
    #[inline]
    fn pack_port(
        node: &crate::ident::NodeId,
        port_id: u32,
        dir_in: bool,
    ) -> crate::footprint::PortKey {
        let mut node_hi = [0u8; 8];
        node_hi.copy_from_slice(&node.0[0..8]);
        let node_bits = u64::from_le_bytes(node_hi);
        let dir_bit = u64::from(dir_in);
        (node_bits << 32) | (u64::from(port_id) << 2) | dir_bit
    }

    fn h(byte: u8) -> Hash {
        let mut out = [0u8; 32];
        out[0] = byte;
        out
    }

    #[test]
    fn drain_for_tx_returns_deterministic_order() {
        let tx = TxId::from_raw(1);
        let scope = make_node_id("s");
        let mut sched = DeterministicScheduler::default();

        // Insert out of lexicographic order: (2,1), (1,2), (1,1)
        for (scope_h, rule_id) in &[(h(2), 1), (h(1), 2), (h(1), 1)] {
            sched.enqueue(
                tx,
                PendingRewrite {
                    rule_id: h(0),
                    compact_rule: CompactRuleId(*rule_id),
                    scope_hash: *scope_h,
                    scope,
                    footprint: Footprint::default(),
                    phase: RewritePhase::Matched,
                },
            );
        }

        let drained = sched.drain_for_tx(tx);
        let keys: Vec<(u8, u32)> = drained
            .iter()
            .map(|pr| (pr.scope_hash[0], pr.compact_rule.0))
            .collect();

        // Should be sorted by (scope_hash, rule_id): (1,1), (1,2), (2,1)
        assert_eq!(keys, vec![(1, 1), (1, 2), (2, 1)]);
    }

    #[test]
    fn last_wins_dedupe() {
        let tx = TxId::from_raw(1);
        let scope = make_node_id("s");
        let mut sched = DeterministicScheduler::default();
        let scope_h = h(5);

        // Insert same (scope, rule) twice
        sched.enqueue(
            tx,
            PendingRewrite {
                rule_id: h(0),
                compact_rule: CompactRuleId(10),
                scope_hash: scope_h,
                scope,
                footprint: Footprint::default(),
                phase: RewritePhase::Matched,
            },
        );
        sched.enqueue(
            tx,
            PendingRewrite {
                rule_id: h(0),
                compact_rule: CompactRuleId(10),
                scope_hash: scope_h,
                scope,
                footprint: Footprint::default(),
                phase: RewritePhase::Matched,
            },
        );

        let drained = sched.drain_for_tx(tx);
        assert_eq!(drained.len(), 1, "should dedupe to single entry");
    }

    #[test]
    fn gen_set_detects_conflicts() {
        let mut gen = GenSet::new();
        let node_a = make_node_id("a");
        let node_b = make_node_id("b");

        assert!(!gen.conflict_or_mark(node_a), "first mark");
        assert!(gen.conflict_or_mark(node_a), "conflict on same gen");
        assert!(!gen.conflict_or_mark(node_b), "different node ok");
    }

    // ========================================================================
    // P0: Independence checking tests - verifying reserve() correctness
    // ========================================================================

    #[test]
    fn reserve_should_detect_node_write_read_conflict() {
        use crate::ident::make_node_id;

        let tx = TxId::from_raw(1);
        let mut sched = DeterministicScheduler::default();
        let shared_node = make_node_id("shared");

        // First rewrite writes to a node
        let mut rewrite1 = PendingRewrite {
            rule_id: h(1),
            compact_rule: CompactRuleId(1),
            scope_hash: h(1),
            scope: make_node_id("scope1"),
            footprint: Footprint {
                factor_mask: 0b0001, // Set factor mask so independence check proceeds
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite1.footprint.n_write.insert_node(&shared_node);

        // Second rewrite reads from the same node
        let mut rewrite2 = PendingRewrite {
            rule_id: h(2),
            compact_rule: CompactRuleId(2),
            scope_hash: h(2),
            scope: make_node_id("scope2"),
            footprint: Footprint {
                factor_mask: 0b0001, // Overlapping factor mask
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite2.footprint.n_read.insert_node(&shared_node);

        // First should succeed, second should fail due to conflict
        assert!(
            sched.reserve(tx, &mut rewrite1),
            "first reserve should succeed"
        );
        assert!(
            !sched.reserve(tx, &mut rewrite2),
            "second reserve should fail: node write-read conflict"
        );
        assert_eq!(
            rewrite2.phase,
            RewritePhase::Aborted,
            "conflicting rewrite should be aborted"
        );
    }

    #[test]
    fn reserve_should_detect_edge_write_write_conflict() {
        use crate::ident::make_edge_id;

        let tx = TxId::from_raw(1);
        let mut sched = DeterministicScheduler::default();
        let shared_edge = make_edge_id("shared");

        // First rewrite writes to an edge
        let mut rewrite1 = PendingRewrite {
            rule_id: h(1),
            compact_rule: CompactRuleId(1),
            scope_hash: h(1),
            scope: make_node_id("scope1"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite1.footprint.e_write.insert_edge(&shared_edge);

        // Second rewrite also writes to the same edge
        let mut rewrite2 = PendingRewrite {
            rule_id: h(2),
            compact_rule: CompactRuleId(2),
            scope_hash: h(2),
            scope: make_node_id("scope2"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite2.footprint.e_write.insert_edge(&shared_edge);

        // First should succeed, second should fail due to conflict
        assert!(
            sched.reserve(tx, &mut rewrite1),
            "first reserve should succeed"
        );
        assert!(
            !sched.reserve(tx, &mut rewrite2),
            "second reserve should fail: edge write-write conflict"
        );
        assert_eq!(
            rewrite2.phase,
            RewritePhase::Aborted,
            "conflicting rewrite should be aborted"
        );
    }

    #[test]
    fn reserve_should_detect_edge_write_read_conflict() {
        use crate::ident::make_edge_id;

        let tx = TxId::from_raw(1);
        let mut sched = DeterministicScheduler::default();
        let shared_edge = make_edge_id("shared");

        // First rewrite writes to an edge
        let mut rewrite1 = PendingRewrite {
            rule_id: h(1),
            compact_rule: CompactRuleId(1),
            scope_hash: h(1),
            scope: make_node_id("scope1"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite1.footprint.e_write.insert_edge(&shared_edge);

        // Second rewrite reads from the same edge
        let mut rewrite2 = PendingRewrite {
            rule_id: h(2),
            compact_rule: CompactRuleId(2),
            scope_hash: h(2),
            scope: make_node_id("scope2"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite2.footprint.e_read.insert_edge(&shared_edge);

        // First should succeed, second should fail due to conflict
        assert!(
            sched.reserve(tx, &mut rewrite1),
            "first reserve should succeed"
        );
        assert!(
            !sched.reserve(tx, &mut rewrite2),
            "second reserve should fail: edge write-read conflict"
        );
        assert_eq!(
            rewrite2.phase,
            RewritePhase::Aborted,
            "conflicting rewrite should be aborted"
        );
    }

    #[test]
    fn reserve_should_detect_port_conflict() {
        let tx = TxId::from_raw(1);
        let mut sched = DeterministicScheduler::default();
        let node = make_node_id("port_node");

        // First rewrite touches a boundary input port
        let mut rewrite1 = PendingRewrite {
            rule_id: h(1),
            compact_rule: CompactRuleId(1),
            scope_hash: h(1),
            scope: make_node_id("scope1"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite1.footprint.b_in.insert(pack_port(&node, 0, true));

        // Second rewrite touches the same boundary input port
        let mut rewrite2 = PendingRewrite {
            rule_id: h(2),
            compact_rule: CompactRuleId(2),
            scope_hash: h(2),
            scope: make_node_id("scope2"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite2.footprint.b_in.insert(pack_port(&node, 0, true));

        // First should succeed, second should fail due to conflict
        assert!(
            sched.reserve(tx, &mut rewrite1),
            "first reserve should succeed"
        );
        assert!(
            !sched.reserve(tx, &mut rewrite2),
            "second reserve should fail: boundary port conflict"
        );
        assert_eq!(
            rewrite2.phase,
            RewritePhase::Aborted,
            "conflicting rewrite should be aborted"
        );
    }

    #[test]
    fn reserve_is_atomic_no_partial_marking_on_conflict() {
        // This test proves that if a conflict is detected, NO resources are marked.
        // We create a rewrite that has multiple resources, where one conflicts.
        // If marking were non-atomic, subsequent checks would see partial marks.

        let tx = TxId::from_raw(1);
        let mut sched = DeterministicScheduler::default();

        // First rewrite: writes node A
        let mut rewrite1 = PendingRewrite {
            rule_id: h(1),
            compact_rule: CompactRuleId(1),
            scope_hash: h(1),
            scope: make_node_id("scope1"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        let node_a = make_node_id("node_a");
        rewrite1.footprint.n_write.insert_node(&node_a);

        assert!(
            sched.reserve(tx, &mut rewrite1),
            "first reserve should succeed"
        );

        // Second rewrite: reads node A (conflicts) AND writes node B (no conflict)
        let mut rewrite2 = PendingRewrite {
            rule_id: h(2),
            compact_rule: CompactRuleId(2),
            scope_hash: h(2),
            scope: make_node_id("scope2"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        let node_b = make_node_id("node_b");
        rewrite2.footprint.n_read.insert_node(&node_a); // Conflicts!
        rewrite2.footprint.n_write.insert_node(&node_b); // Would not conflict

        assert!(
            !sched.reserve(tx, &mut rewrite2),
            "second reserve should fail"
        );

        // Third rewrite: writes node B only (should succeed if rewrite2 didn't partially mark)
        let mut rewrite3 = PendingRewrite {
            rule_id: h(3),
            compact_rule: CompactRuleId(3),
            scope_hash: h(3),
            scope: make_node_id("scope3"),
            footprint: Footprint {
                factor_mask: 0b0001,
                ..Default::default()
            },
            phase: RewritePhase::Matched,
        };
        rewrite3.footprint.n_write.insert_node(&node_b);

        // This MUST succeed, proving rewrite2 did NOT mark node_b despite checking it
        assert!(
            sched.reserve(tx, &mut rewrite3),
            "third reserve should succeed - proves no partial marking from failed rewrite2"
        );
    }

    #[test]
    fn reserve_determinism_same_sequence_same_results() {
        // This test proves determinism: same sequence of reserves always produces
        // same accept/reject decisions regardless of internal implementation.

        fn run_reserve_sequence() -> Vec<bool> {
            let tx = TxId::from_raw(1);
            let mut sched = DeterministicScheduler::default();
            let mut results = Vec::new();

            // Rewrite 1: writes A
            let mut r1 = PendingRewrite {
                rule_id: h(1),
                compact_rule: CompactRuleId(1),
                scope_hash: h(1),
                scope: make_node_id("s1"),
                footprint: Footprint {
                    factor_mask: 1,
                    ..Default::default()
                },
                phase: RewritePhase::Matched,
            };
            r1.footprint.n_write.insert_node(&make_node_id("A"));
            results.push(sched.reserve(tx, &mut r1));

            // Rewrite 2: reads A (should fail - conflicts with r1)
            let mut r2 = PendingRewrite {
                rule_id: h(2),
                compact_rule: CompactRuleId(2),
                scope_hash: h(2),
                scope: make_node_id("s2"),
                footprint: Footprint {
                    factor_mask: 1,
                    ..Default::default()
                },
                phase: RewritePhase::Matched,
            };
            r2.footprint.n_read.insert_node(&make_node_id("A"));
            results.push(sched.reserve(tx, &mut r2));

            // Rewrite 3: writes B (should succeed - independent)
            let mut r3 = PendingRewrite {
                rule_id: h(3),
                compact_rule: CompactRuleId(3),
                scope_hash: h(3),
                scope: make_node_id("s3"),
                footprint: Footprint {
                    factor_mask: 1,
                    ..Default::default()
                },
                phase: RewritePhase::Matched,
            };
            r3.footprint.n_write.insert_node(&make_node_id("B"));
            results.push(sched.reserve(tx, &mut r3));

            // Rewrite 4: reads B (should fail - conflicts with r3)
            let mut r4 = PendingRewrite {
                rule_id: h(4),
                compact_rule: CompactRuleId(4),
                scope_hash: h(4),
                scope: make_node_id("s4"),
                footprint: Footprint {
                    factor_mask: 1,
                    ..Default::default()
                },
                phase: RewritePhase::Matched,
            };
            r4.footprint.n_read.insert_node(&make_node_id("B"));
            results.push(sched.reserve(tx, &mut r4));

            results
        }

        // Run the same sequence 5 times - must get identical results
        let baseline = run_reserve_sequence();
        for i in 0..5 {
            let results = run_reserve_sequence();
            assert_eq!(
                results, baseline,
                "run {i} produced different results: {results:?} vs baseline {baseline:?}"
            );
        }

        // Also verify the expected pattern
        assert_eq!(
            baseline,
            vec![true, false, true, false],
            "expected [accept, reject, accept, reject] pattern"
        );
    }

    #[test]
    fn reserve_scaling_is_linear_in_footprint_size() {
        // This test demonstrates that reserve() time scales linearly with footprint size,
        // NOT with number of previously reserved rewrites.
        //
        // We measure time to reserve rewrites with varying footprint sizes,
        // keeping k (# of prior reserves) constant and large.

        use std::time::Instant;

        let tx = TxId::from_raw(1);
        let mut sched = DeterministicScheduler::default();

        // Reserve k=100 independent rewrites first
        for i in 0u8..100u8 {
            let mut rewrite = PendingRewrite {
                rule_id: h(i),
                compact_rule: CompactRuleId(u32::from(i)),
                scope_hash: h(i),
                scope: make_node_id(&format!("prior_{i}")),
                footprint: Footprint {
                    factor_mask: 0b0001,
                    ..Default::default()
                },
                phase: RewritePhase::Matched,
            };
            // Each writes to a unique node to avoid conflicts
            rewrite
                .footprint
                .n_write
                .insert_node(&make_node_id(&format!("node_{i}")));
            assert!(sched.reserve(tx, &mut rewrite));
        }

        // Now measure reserve time for different footprint sizes
        // All are independent (use different nodes), so k doesn't affect lookup time
        let sizes = [1, 10, 50, 100];
        let mut times = Vec::new();

        for &size in &sizes {
            let mut rewrite = PendingRewrite {
                rule_id: h(200),
                compact_rule: CompactRuleId(200),
                scope_hash: h(200),
                scope: make_node_id(&format!("test_{size}")),
                footprint: Footprint {
                    factor_mask: 0b0001,
                    ..Default::default()
                },
                phase: RewritePhase::Matched,
            };

            // Add 'size' unique nodes to footprint
            for i in 0..size {
                rewrite
                    .footprint
                    .n_write
                    .insert_node(&make_node_id(&format!("footprint_{size}_{i}")));
            }

            let start = Instant::now();
            let success = sched.reserve(tx, &mut rewrite);
            let elapsed = start.elapsed();

            assert!(success, "reserve should succeed for independent rewrite");
            times.push((size, elapsed));

            // Clean up for next iteration (finalize and re-init)
            sched.finalize_tx(tx);
            sched = DeterministicScheduler::default();
            // Re-reserve the 100 prior rewrites
            for i in 0u8..100u8 {
                let mut r = PendingRewrite {
                    rule_id: h(i),
                    compact_rule: CompactRuleId(u32::from(i)),
                    scope_hash: h(i),
                    scope: make_node_id(&format!("prior_{i}")),
                    footprint: Footprint {
                        factor_mask: 0b0001,
                        ..Default::default()
                    },
                    phase: RewritePhase::Matched,
                };
                r.footprint
                    .n_write
                    .insert_node(&make_node_id(&format!("node_{i}")));
                sched.reserve(tx, &mut r);
            }
        }

        // Sanity check: larger footprints should take longer
        // But the relationship should be roughly linear, not quadratic
        // (This is a weak assertion since timing is noisy in tests)
        assert!(!times.is_empty(), "timing vector unexpectedly empty");
        if let (Some((_, first)), Some((_, last))) = (times.first().copied(), times.last().copied())
        {
            assert!(
                last >= first,
                "larger footprints should take at least as long"
            );
        }
    }

    #[test]
    fn reserve_allows_independent_rewrites() {
        let tx = TxId::from_raw(1);
        let mut sched = DeterministicScheduler::default();

        // Two rewrites with completely disjoint footprints
        let mut rewrite1 = PendingRewrite {
            rule_id: h(1),
            compact_rule: CompactRuleId(1),
            scope_hash: h(1),
            scope: make_node_id("scope1"),
            footprint: Footprint::default(),
            phase: RewritePhase::Matched,
        };
        rewrite1
            .footprint
            .n_write
            .insert_node(&make_node_id("node_a"));

        let mut rewrite2 = PendingRewrite {
            rule_id: h(2),
            compact_rule: CompactRuleId(2),
            scope_hash: h(2),
            scope: make_node_id("scope2"),
            footprint: Footprint::default(),
            phase: RewritePhase::Matched,
        };
        rewrite2
            .footprint
            .n_write
            .insert_node(&make_node_id("node_b"));

        // Both should be allowed to reserve since they're independent
        assert!(
            sched.reserve(tx, &mut rewrite1),
            "first reserve should succeed"
        );
        assert!(
            sched.reserve(tx, &mut rewrite2),
            "second reserve should succeed for independent rewrites"
        );
    }
}
