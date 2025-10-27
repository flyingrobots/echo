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
/// Layout: `(node_hi: u32 | node_lo: u16) << 32 | (port_id << 2) | dir_bits`.
/// Callers should pack using a stable convention within the rule pack. The
/// footprint logic only needs stable equality and ordering.
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
    pub fn independent(&self, other: &Self) -> bool {
        if (self.factor_mask & other.factor_mask) == 0 {
            return true;
        }
        if self.b_in.intersects(&other.b_in) || self.b_out.intersects(&other.b_out) {
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
