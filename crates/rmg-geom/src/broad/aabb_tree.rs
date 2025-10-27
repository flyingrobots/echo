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

/// A minimal AABB-based broad-phase using an `O(n^2)` sweep for simplicity.
///
/// Intended for early correctness and determinism tests; real engines should
/// replace this with SAP or BVH.
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
