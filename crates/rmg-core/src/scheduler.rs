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

/// Deterministic scheduler with O(n) radix-based drain.
#[derive(Debug, Default)]
pub(crate) struct DeterministicScheduler {
    /// Pending rewrites per transaction, stored for O(1) enqueue and O(n) drain.
    pub(crate) pending: HashMap<TxId, PendingTx<PendingRewrite>>,
    /// Generation-stamped conflict sets for O(1) independence checks.
    pub(crate) active: HashMap<TxId, GenSet<NodeId>>,
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

    /// Attempts to reserve a rewrite by checking independence against the
    /// active generation set for `tx`. On success, marks written scopes and
    /// transitions the phase to `Reserved`.
    ///
    /// Uses O(1) generation-stamped conflict detection per node.
    pub(crate) fn reserve(&mut self, tx: TxId, pr: &mut PendingRewrite) -> bool {
        let gen_set = self.active.entry(tx).or_insert_with(GenSet::new);

        // Check for conflicts on all written nodes
        for node_hash in pr.footprint.n_write.iter() {
            let node_id = NodeId(*node_hash);
            if gen_set.conflict_or_mark(node_id) {
                pr.phase = RewritePhase::Aborted;
                #[cfg(feature = "telemetry")]
                {
                    let entry = self.counters.entry(tx).or_default();
                    entry.1 += 1;
                }
                #[cfg(feature = "telemetry")]
                telemetry::conflict(tx, &pr.rule_id);
                return false;
            }
        }

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
pub(crate) struct PendingTx<P> {
    next_nonce: u32,
    /// Last-wins dedupe on (`scope_hash`, `compact_rule`).
    index: FxHashMap<([u8; 32], u32), usize>,
    /// Thin keys + handles (sorted during drain).
    thin: Vec<RewriteThin>,
    /// Fat payloads (indexed by handle).
    fat: Vec<Option<P>>,
    /// Scratch buffer for radix passes (reused).
    scratch: Vec<RewriteThin>,
    /// Counting array for 16-bit radix (65536 buckets, reused). Uses `usize`
    /// to avoid truncation and casts during prefix-sum scatter.
    counts16: Vec<usize>,
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
            self.counts16 = vec![0usize; 1 << 16];
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
            let mut sum: usize = 0;
            for c in counts.iter_mut() {
                let t = *c;
                *c = sum;
                sum = sum.wrapping_add(t);
            }

            // Stable scatter
            for r in src {
                let b = bucket16(r, pass) as usize;
                let idx = counts[b];
                dst[idx] = *r;
                counts[b] = idx + 1;
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
            let payload_opt = self.fat[r.handle].take();
            // Invariant: every thin handle points to a live payload. Avoid
            // panicking on release builds; assert in debug to surface issues.
            if let Some(p) = payload_opt {
                out.push(p);
            } else {
                debug_assert!(false, "payload must exist");
            }
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
#[derive(Debug)]
pub struct GenSet<K> {
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

    /// Returns true if `key` conflicts with current generation, otherwise marks it.
    #[inline]
    pub fn conflict_or_mark(&mut self, key: K) -> bool {
        match self.seen.get(&key) {
            Some(&g) if g == self.gen => true, // Conflict!
            _ => {
                self.seen.insert(key, self.gen);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::make_node_id;

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
}
