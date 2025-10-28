//! Footprints and independence checks for MWMR scheduling.
//!
//! A footprint summarises the read/write sets of a pending rewrite over nodes,
//! edges, and boundary ports (typed interfaces), plus a coarse-grained
//! `factor_mask` used as an O(1) prefilter for spatial or subsystem
//! partitioning.
//!
//! This module intentionally uses simple set types for clarity; a future
//! optimisation replaces them with block‑sparse bitmaps and SIMD kernels.

use std::collections::BTreeSet;

use crate::ident::{EdgeId, Hash, NodeId};

/// Packed 64‑bit key for a boundary port.
///
/// This is an opaque, caller-supplied stable identifier used to detect
/// conflicts on boundary interfaces. The engine only requires stable equality
/// and ordering; it does not rely on a specific bit layout.
///
/// For demos/tests, use [`pack_port_key`] to derive a deterministic 64‑bit key
/// from a [`NodeId`], a `port_id`, and a direction flag.
pub type PortKey = u64;

/// Simple ordered set of 256‑bit ids based on `BTreeSet` for deterministic
/// iteration. Optimised representations (Roaring + SIMD) can back this API in
/// the future without changing call‑sites.
#[derive(Debug, Clone, Default)]
pub struct IdSet(BTreeSet<Hash>);

impl IdSet {
    /// Inserts an identifier.
    pub fn insert_node(&mut self, id: &NodeId) {
        self.0.insert(id.0);
    }
    /// Inserts an identifier.
    pub fn insert_edge(&mut self, id: &EdgeId) {
        self.0.insert(id.0);
    }
    /// Returns true if any element is shared with `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        // Early‑exit by zipping ordered sets.
        let mut a = self.0.iter();
        let mut b = other.0.iter();
        let mut va = a.next();
        let mut vb = b.next();
        while let (Some(x), Some(y)) = (va, vb) {
            match x.cmp(y) {
                core::cmp::Ordering::Less => va = a.next(),
                core::cmp::Ordering::Greater => vb = b.next(),
                core::cmp::Ordering::Equal => return true,
            }
        }
        false
    }
}

/// Ordered set of boundary ports.
#[derive(Debug, Clone, Default)]
pub struct PortSet(BTreeSet<PortKey>);

impl PortSet {
    /// Inserts a port key.
    pub fn insert(&mut self, key: PortKey) {
        let _ = self.0.insert(key);
    }
    /// Returns true if any element is shared with `other`.
    pub fn intersects(&self, other: &Self) -> bool {
        let mut a = self.0.iter();
        let mut b = other.0.iter();
        let mut va = a.next();
        let mut vb = b.next();
        while let (Some(x), Some(y)) = (va, vb) {
            match x.cmp(y) {
                core::cmp::Ordering::Less => va = a.next(),
                core::cmp::Ordering::Greater => vb = b.next(),
                core::cmp::Ordering::Equal => return true,
            }
        }
        false
    }
}

/// Footprint capturing the read/write sets and factor mask of a rewrite.
#[derive(Debug, Clone, Default)]
pub struct Footprint {
    /// Nodes read by the rewrite.
    pub n_read: IdSet,
    /// Nodes written/created/deleted by the rewrite.
    pub n_write: IdSet,
    /// Edges read by the rewrite.
    pub e_read: IdSet,
    /// Edges written/created/deleted by the rewrite.
    pub e_write: IdSet,
    /// Boundary input ports touched.
    pub b_in: PortSet,
    /// Boundary output ports touched.
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
        if self.n_write.intersects(&other.n_write)
            || self.n_write.intersects(&other.n_read)
            || other.n_write.intersects(&self.n_read)
        {
            return false;
        }
        true
    }
}

/// Helper to derive a deterministic [`PortKey`] from node, port id, and direction.
///
/// Layout used by this helper:
/// - bits 63..32: lower 32 bits of the node's first 8 bytes (LE) — a stable
///   per-node fingerprint, not reversible
/// - bits 31..1: `port_id` (u31; must be < 2^31)
/// - bit 0: direction flag (1 = input, 0 = output)
///
/// This is sufficient for tests and demos; production code may adopt a
/// different stable scheme as long as equality and ordering are preserved.
#[inline]
pub fn pack_port_key(node: &NodeId, port_id: u32, dir_in: bool) -> PortKey {
    let mut first8 = [0u8; 8];
    first8.copy_from_slice(&node.0[0..8]);
    let node_fingerprint = u64::from_le_bytes(first8) & 0xFFFF_FFFF;
    let dir_bit = u64::from(dir_in);
    debug_assert!(port_id < (1 << 31), "port_id must fit in 31 bits");
    let port31 = u64::from(port_id & 0x7FFF_FFFF);
    (node_fingerprint << 32) | (port31 << 1) | dir_bit
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn pack_port_key_masks_port_id_to_u31() {
        let a = NodeId(blake3::hash(b"node-a").into());
        let hi = (1u32 << 31) - 1;
        let k_ok = pack_port_key(&a, hi, true);
        if !cfg!(debug_assertions) {
            // Same node/dir; port_id above u31 must not alter higher fields.
            let k_over = pack_port_key(&a, hi + 1, true);
            assert_eq!(
                k_ok & !0b1,
                k_over & !0b1,
                "overflow must not spill into fingerprint"
            );
        }
    }
}
