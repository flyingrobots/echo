// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Footprints and independence checks for MWMR scheduling.
//!
//! A footprint summarises the read/write sets of a pending rewrite over nodes,
//! edges, and boundary ports (typed interfaces), plus a coarse-grained
//! `factor_mask` used as an O(1) prefilter for spatial or subsystem
//! partitioning.
//!
//! ## Warp-Scoped Resource Sets
//!
//! All resource sets are **warp-scoped**: they store `(WarpId, LocalId)` pairs
//! rather than just local identifiers. This ensures rewrites in different warps
//! don't cause false conflict detection when they happen to touch resources
//! with the same local ID.
//!
//! Example of the problem solved:
//! ```text
//! R1 in W_sim reads node 0xABC123...
//! R2 in W_ttd reads node 0xABC123...
//! Without warp scoping: CONFLICT (same local ID) ← FALSE POSITIVE
//! With warp scoping:    NO CONFLICT (different warps)
//! ```
//!
//! This module intentionally uses simple set types for clarity; a future
//! optimisation replaces them with block‑sparse bitmaps and SIMD kernels.

use std::collections::BTreeSet;

use crate::attachment::AttachmentKey;
use crate::ident::{EdgeId, EdgeKey, NodeId, NodeKey, WarpId};

// =============================================================================
// Packed Port Key
// =============================================================================

/// Packed 64‑bit key for a boundary port (local to a warp).
///
/// This is an opaque, caller-supplied stable identifier used to detect
/// conflicts on boundary interfaces. The engine only requires stable equality
/// and ordering; it does not rely on a specific bit layout.
///
/// For demos/tests, use [`pack_port_key`](crate::footprint::pack_port_key) to derive a
/// deterministic 64‑bit key from a [`NodeId`], a `port_id`, and a direction flag.
pub type PortKey = u64;

/// Warp-scoped port identifier combining a [`WarpId`] with a packed port key.
///
/// This ensures ports in different warps don't cause false conflicts during
/// scheduling. The ordering is `(WarpId, PortKey)` for deterministic iteration.
pub type WarpScopedPortKey = (WarpId, PortKey);

// =============================================================================
// Generic BTreeSet intersection helper
// =============================================================================

/// Early-exit intersection check for two ordered `BTreeSet`s.
///
/// Uses the merge algorithm on sorted iterators for O(n+m) complexity with
/// early exit on first match.
fn intersects_btree<T: Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> bool {
    let mut it_a = a.iter();
    let mut it_b = b.iter();
    let mut va = it_a.next();
    let mut vb = it_b.next();
    while let (Some(x), Some(y)) = (va, vb) {
        match x.cmp(y) {
            core::cmp::Ordering::Less => va = it_a.next(),
            core::cmp::Ordering::Greater => vb = it_b.next(),
            core::cmp::Ordering::Equal => return true,
        }
    }
    false
}

// =============================================================================
// Warp-scoped resource sets (Phase 5 BOAW)
// =============================================================================

/// Ordered set of warp-scoped node identifiers.
///
/// Each entry is a `NodeKey` containing both `warp_id` and `local_id`, ensuring
/// nodes in different warps don't cause false conflicts during scheduling.
#[derive(Debug, Clone, Default)]
pub struct NodeSet(BTreeSet<NodeKey>);

impl NodeSet {
    /// Inserts a warp-scoped node key.
    pub fn insert(&mut self, key: NodeKey) {
        self.0.insert(key);
    }

    /// Inserts a node with explicit `warp_id`.
    pub fn insert_with_warp(&mut self, warp_id: WarpId, node_id: NodeId) {
        self.0.insert(NodeKey {
            warp_id,
            local_id: node_id,
        });
    }

    /// Returns an iterator over the node keys in the set.
    pub fn iter(&self) -> impl Iterator<Item = &NodeKey> {
        self.0.iter()
    }

    /// Returns true if any element is shared with `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        intersects_btree(&self.0, &other.0)
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Ordered set of warp-scoped edge identifiers.
///
/// Each entry is an `EdgeKey` containing both `warp_id` and `local_id`, ensuring
/// edges in different warps don't cause false conflicts during scheduling.
#[derive(Debug, Clone, Default)]
pub struct EdgeSet(BTreeSet<EdgeKey>);

impl EdgeSet {
    /// Inserts a warp-scoped edge key.
    pub fn insert(&mut self, key: EdgeKey) {
        self.0.insert(key);
    }

    /// Inserts an edge with explicit `warp_id`.
    pub fn insert_with_warp(&mut self, warp_id: WarpId, edge_id: EdgeId) {
        self.0.insert(EdgeKey {
            warp_id,
            local_id: edge_id,
        });
    }

    /// Returns an iterator over the edge keys in the set.
    pub fn iter(&self) -> impl Iterator<Item = &EdgeKey> {
        self.0.iter()
    }

    /// Returns true if any element is shared with `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        intersects_btree(&self.0, &other.0)
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Ordered set of warp-scoped boundary ports.
///
/// Each entry is a `(WarpId, PortKey)` tuple ensuring ports in different warps
/// don't cause false conflicts during scheduling.
#[derive(Debug, Clone, Default)]
pub struct PortSet(BTreeSet<WarpScopedPortKey>);

impl PortSet {
    /// Inserts a warp-scoped port key.
    pub fn insert(&mut self, warp_id: WarpId, key: PortKey) {
        self.0.insert((warp_id, key));
    }

    /// Inserts a pre-packed warp-scoped port key.
    pub fn insert_scoped(&mut self, key: WarpScopedPortKey) {
        self.0.insert(key);
    }

    /// Returns an iterator over the warp-scoped port keys in the set.
    pub fn iter(&self) -> impl Iterator<Item = &WarpScopedPortKey> {
        self.0.iter()
    }

    /// Alias for iterating keys; provided for call sites that prefer explicit naming.
    pub fn keys(&self) -> impl Iterator<Item = &WarpScopedPortKey> {
        self.0.iter()
    }

    /// Returns true if any element is shared with `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        intersects_btree(&self.0, &other.0)
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// =============================================================================
// Attachment set
// =============================================================================

/// Ordered set of attachment slots.
///
/// [`AttachmentKey`] is already warp-scoped (contains [`NodeKey`] or [`EdgeKey`]).
#[derive(Debug, Clone, Default)]
pub struct AttachmentSet(BTreeSet<AttachmentKey>);

impl AttachmentSet {
    /// Inserts an attachment key.
    pub fn insert(&mut self, key: AttachmentKey) {
        let _ = self.0.insert(key);
    }

    /// Returns an iterator over attachment keys.
    pub fn iter(&self) -> impl Iterator<Item = &AttachmentKey> {
        self.0.iter()
    }

    /// Returns true if any element is shared with `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        intersects_btree(&self.0, &other.0)
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// =============================================================================
// Footprint struct
// =============================================================================

/// Footprint capturing the read/write sets and factor mask of a rewrite.
///
/// All resource sets are warp-scoped to prevent false conflicts between
/// rewrites in different warps that happen to touch resources with the
/// same local identifier.
#[derive(Debug, Clone, Default)]
pub struct Footprint {
    /// Nodes read by the rewrite (warp-scoped).
    pub n_read: NodeSet,
    /// Nodes written/created/deleted by the rewrite (warp-scoped).
    pub n_write: NodeSet,
    /// Edges read by the rewrite (warp-scoped).
    pub e_read: EdgeSet,
    /// Edges written/created/deleted by the rewrite (warp-scoped).
    pub e_write: EdgeSet,
    /// Attachment slots read by the rewrite.
    pub a_read: AttachmentSet,
    /// Attachment slots written by the rewrite.
    pub a_write: AttachmentSet,
    /// Boundary input ports touched (warp-scoped).
    pub b_in: PortSet,
    /// Boundary output ports touched (warp-scoped).
    pub b_out: PortSet,
    /// Coarse partition mask; used as an O(1) prefilter.
    pub factor_mask: u64,
}

impl Footprint {
    /// Returns `true` when this footprint is independent of `other`.
    ///
    /// Fast path checks the factor mask; then boundary ports; then edges and
    /// nodes. The check is symmetric but implemented with early exits.
    /// Disjoint `factor_mask` values guarantee independence by construction
    /// (the mask is a coarse superset of touched partitions).
    ///
    /// All comparisons are warp-scoped, so resources in different warps
    /// never conflict (even if they share the same local identifier).
    pub fn independent(&self, other: &Self) -> bool {
        if (self.factor_mask & other.factor_mask) == 0 {
            return true;
        }
        if self.b_in.intersects(&other.b_in)
            || self.b_in.intersects(&other.b_out)
            || self.b_out.intersects(&other.b_in)
            || self.b_out.intersects(&other.b_out)
        {
            return false;
        }
        if self.e_write.intersects(&other.e_write)
            || self.e_write.intersects(&other.e_read)
            || other.e_write.intersects(&self.e_read)
        {
            return false;
        }
        if self.a_write.intersects(&other.a_write)
            || self.a_write.intersects(&other.a_read)
            || other.a_write.intersects(&self.a_read)
        {
            return false;
        }
        if self.n_write.intersects(&other.n_write)
            || self.n_write.intersects(&other.n_read)
            || other.n_write.intersects(&self.n_read)
        {
            return false;
        }
        true
    }
}

// =============================================================================
// Port key packing helper
// =============================================================================

/// Helper to derive a deterministic [`PortKey`] from node, port id, and direction.
///
/// Layout used by this helper:
/// - bits 63..32: lower 32 bits of the node's first 8 bytes (LE) — a stable
///   per-node fingerprint, not reversible
/// - bits 31..2: `port_id` (u30; must be < 2^30)
/// - bit 1: reserved (0)
/// - bit 0: direction flag (1 = input, 0 = output)
///
/// This is sufficient for tests and demos; production code may adopt a
/// different stable scheme as long as equality and ordering are preserved.
///
/// **Note:** The returned `PortKey` is local-only. When inserting into a
/// [`PortSet`], pair it with a `WarpId` for proper warp scoping.
#[inline]
pub fn pack_port_key(node: &NodeId, port_id: u32, dir_in: bool) -> PortKey {
    let mut first8 = [0u8; 8];
    first8.copy_from_slice(&node.0[0..8]);
    let node_fingerprint = u64::from_le_bytes(first8) & 0xFFFF_FFFF;
    let dir_bit = u64::from(dir_in);
    debug_assert!(port_id < (1 << 30), "port_id must fit in 30 bits");
    let port30 = u64::from(port_id & 0x3FFF_FFFF);
    (node_fingerprint << 32) | (port30 << 2) | dir_bit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::make_warp_id;

    #[test]
    fn pack_port_key_is_stable_and_distinct_by_inputs() {
        let a = NodeId(blake3::hash(b"node-a").into());
        let b = NodeId(blake3::hash(b"node-b").into());
        let k1 = pack_port_key(&a, 0, true);
        let k2 = pack_port_key(&a, 1, true);
        let k3 = pack_port_key(&a, 0, false);
        let k4 = pack_port_key(&b, 0, true);
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
        assert_ne!(k1, k4);
        // Stability
        assert_eq!(k1, pack_port_key(&a, 0, true));
    }

    #[test]
    fn pack_port_key_masks_port_id_to_u30() {
        let a = NodeId(blake3::hash(b"node-a").into());
        let hi = (1u32 << 30) - 1;
        let k_ok = pack_port_key(&a, hi, true);
        if !cfg!(debug_assertions) {
            // Same node/dir; port_id above u30 must not alter higher fields.
            let k_over = pack_port_key(&a, hi + 1, true);
            assert_eq!(
                k_ok & !0b11,
                k_over & !0b11,
                "overflow must not spill into fingerprint"
            );
        }
    }

    #[test]
    fn node_set_warp_scoped_no_false_conflict() {
        let warp_a = make_warp_id("warp-a");
        let warp_b = make_warp_id("warp-b");
        let node_id = NodeId(blake3::hash(b"same-local-id").into());

        let mut set_a = NodeSet::default();
        let mut set_b = NodeSet::default();

        // Same local ID in different warps should NOT conflict
        set_a.insert_with_warp(warp_a, node_id);
        set_b.insert_with_warp(warp_b, node_id);

        assert!(
            !set_a.intersects(&set_b),
            "nodes in different warps should not conflict"
        );

        // Same local ID in same warp SHOULD conflict
        let mut set_same_warp = NodeSet::default();
        set_same_warp.insert_with_warp(warp_a, node_id);
        assert!(
            set_a.intersects(&set_same_warp),
            "nodes in same warp should conflict"
        );
    }

    #[test]
    fn edge_set_warp_scoped_no_false_conflict() {
        let warp_a = make_warp_id("warp-a");
        let warp_b = make_warp_id("warp-b");
        let edge_id = crate::ident::EdgeId(blake3::hash(b"same-local-edge").into());

        let mut set_a = EdgeSet::default();
        let mut set_b = EdgeSet::default();

        set_a.insert_with_warp(warp_a, edge_id);
        set_b.insert_with_warp(warp_b, edge_id);

        assert!(
            !set_a.intersects(&set_b),
            "edges in different warps should not conflict"
        );
    }

    #[test]
    fn port_set_warp_scoped_no_false_conflict() {
        let warp_a = make_warp_id("warp-a");
        let warp_b = make_warp_id("warp-b");
        let node_id = NodeId(blake3::hash(b"port-node").into());
        let port_key = pack_port_key(&node_id, 0, true);

        let mut set_a = PortSet::default();
        let mut set_b = PortSet::default();

        // Same port key in different warps should NOT conflict
        set_a.insert(warp_a, port_key);
        set_b.insert(warp_b, port_key);

        assert!(
            !set_a.intersects(&set_b),
            "ports in different warps should not conflict"
        );

        // Same port key in same warp SHOULD conflict
        let mut set_same_warp = PortSet::default();
        set_same_warp.insert(warp_a, port_key);
        assert!(
            set_a.intersects(&set_same_warp),
            "ports in same warp should conflict"
        );
    }

    #[test]
    fn footprint_independent_respects_warp_scoping() {
        let warp_a = make_warp_id("warp-a");
        let warp_b = make_warp_id("warp-b");
        let node_id = NodeId(blake3::hash(b"shared-local-id").into());

        // Footprint A writes a node in warp A
        let mut fp_a = Footprint {
            factor_mask: 1,
            ..Default::default()
        };
        fp_a.n_write.insert_with_warp(warp_a, node_id);

        // Footprint B writes the same local node ID in warp B
        let mut fp_b = Footprint {
            factor_mask: 1,
            ..Default::default()
        };
        fp_b.n_write.insert_with_warp(warp_b, node_id);

        // Should be independent (different warps)
        assert!(
            fp_a.independent(&fp_b),
            "footprints in different warps should be independent"
        );

        // But footprints in the same warp SHOULD conflict
        let mut fp_same_warp = Footprint {
            factor_mask: 1,
            ..Default::default()
        };
        fp_same_warp.n_write.insert_with_warp(warp_a, node_id);

        assert!(
            !fp_a.independent(&fp_same_warp),
            "footprints writing same node in same warp should conflict"
        );
    }
}
