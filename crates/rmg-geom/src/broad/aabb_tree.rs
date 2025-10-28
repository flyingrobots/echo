use crate::types::aabb::Aabb;
use core::cmp::Ordering;
use std::collections::BTreeMap;

/// Broad-phase interface for inserting proxies and querying overlapping pairs.
///
/// Implementations must return pairs deterministically: the pair `(a, b)` is
/// canonicalized such that `a < b`, and the full list is sorted ascending by
/// `(a, b)`.
pub trait BroadPhase {
    /// Inserts or updates the proxy with the given `id` and `aabb`.
    fn upsert(&mut self, id: usize, aabb: Aabb);
    /// Removes a proxy if present.
    fn remove(&mut self, id: usize);
    /// Returns a canonical, deterministically-ordered list of overlapping pairs.
    fn pairs(&self) -> Vec<(usize, usize)>;
}

/// A minimal AABB-based broad-phase using an `O(n^2)` all-pairs sweep.
///
/// Why this exists:
/// - Serves as a correctness and determinism baseline while API surfaces
///   stabilize (canonical pair identity and ordering, inclusive face overlap).
/// - Keeps the algorithm small and easy to reason about for early tests.
///
/// Performance plan (to be replaced):
/// - Sweep-and-Prune (aka Sort-and-Sweep) with stable endpoint arrays per
///   axis. Determinism ensured via:
///   - fixed axis order (e.g., X→Y→Z) or a deterministic axis choice
///     (variance with ID tie-breakers),
///   - stable sort and explicit ID tie-breaks,
///   - final pair list sorted lexicographically by `(min_id, max_id)`.
/// - Dynamic AABB Tree (BVH): deterministic insert/rotation heuristics with
///   ID-based tie-breakers; canonical pair set post-sorted by `(min_id,max_id)`.
///
/// Complexity notes:
/// - Any broad phase degenerates to `O(n^2)` when all proxies overlap (k≈n²).
///   The goal of SAP/BVH is near-linear behavior when the true overlap count
///   `k` is small and motion is temporally coherent.
///
/// TODO(geom): replace this reference implementation with a deterministic
/// Sweep-and-Prune (Phase 1), and optionally a Dynamic AABB Tree. Preserve
/// canonical pair ordering and inclusive face-touch semantics.
#[derive(Default)]
pub struct AabbTree {
    items: BTreeMap<usize, Aabb>,
}

impl AabbTree {
    /// Creates an empty tree.
    #[must_use]
    pub fn new() -> Self { Self { items: BTreeMap::new() } }
}

impl BroadPhase for AabbTree {
    fn upsert(&mut self, id: usize, aabb: Aabb) {
        self.items.insert(id, aabb);
    }

    fn remove(&mut self, id: usize) {
        self.items.remove(&id);
    }

    fn pairs(&self) -> Vec<(usize, usize)> {
        // BTreeMap iteration is already sorted by key; copy to a vector for indexed loops.
        let items: Vec<(usize, Aabb)> = self.items.iter().map(|(id, aabb)| (*id, *aabb)).collect();
        let mut out: Vec<(usize, usize)> = Vec::new();
        for (i, (a_id, a_bb)) in items.iter().enumerate() {
            for (b_id, b_bb) in items.iter().skip(i + 1) {
                if a_bb.overlaps(b_bb) {
                    out.push((*a_id, *b_id)); // canonical since a_id < b_id
                }
            }
        }
        out.sort_unstable_by(|x, y| match x.0.cmp(&y.0) {
            Ordering::Equal => x.1.cmp(&y.1),
            o => o,
        });
        out
    }
}
