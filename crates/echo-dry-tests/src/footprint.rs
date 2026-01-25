// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Ergonomic footprint construction for tests.
//!
//! This module provides a builder API for constructing [`Footprint`] values
//! without the verbose `NodeSet`/`EdgeSet`/`AttachmentSet` boilerplate.
//!
//! # Design
//!
//! The builder is intentionally test-focused and lives outside `warp-core` to
//! allow rapid iteration without committing to a stable public API. If the
//! patterns stabilize and prove useful for third parties, it can be promoted.
//!
//! # Example
//!
//! ```
//! use echo_dry_tests::footprint::FootprintBuilder;
//! use warp_core::{make_node_id, make_edge_id, GraphView, GraphStore};
//!
//! let store = GraphStore::default();
//! let view = GraphView::new(&store);
//! let warp_id = view.warp_id();
//! let scope = make_node_id("test/scope");
//!
//! let footprint = FootprintBuilder::new(warp_id)
//!     .reads_node(scope)
//!     .reads_node_alpha(scope)  // attachment
//!     .writes_node(make_node_id("sim/view"))
//!     .writes_edge(make_edge_id("edge:view/op"))
//!     .writes_node_alpha(make_node_id("sim/view/op"))
//!     .build();
//! ```

use warp_core::{
    AttachmentKey, AttachmentSet, EdgeId, EdgeKey, EdgeSet, Footprint, NodeId, NodeKey, NodeSet,
    PortKey, PortSet, WarpId,
};

/// Builder for [`Footprint`] construction in tests.
///
/// All methods take `self` by value and return `Self` for chaining.
/// Call [`build()`](Self::build) to produce the final `Footprint`.
#[derive(Debug, Clone)]
pub struct FootprintBuilder {
    warp_id: WarpId,
    n_read: NodeSet,
    n_write: NodeSet,
    e_read: EdgeSet,
    e_write: EdgeSet,
    a_read: AttachmentSet,
    a_write: AttachmentSet,
    b_in: PortSet,
    b_out: PortSet,
    factor_mask: u64,
}

impl FootprintBuilder {
    /// Creates a new builder scoped to the given warp.
    ///
    /// Most methods use this warp_id implicitly when constructing keys.
    pub fn new(warp_id: WarpId) -> Self {
        Self {
            warp_id,
            n_read: NodeSet::default(),
            n_write: NodeSet::default(),
            e_read: EdgeSet::default(),
            e_write: EdgeSet::default(),
            a_read: AttachmentSet::default(),
            a_write: AttachmentSet::default(),
            b_in: PortSet::default(),
            b_out: PortSet::default(),
            factor_mask: 0,
        }
    }

    /// Creates a builder from a [`GraphView`](warp_core::GraphView), using its warp_id.
    pub fn from_view(view: warp_core::GraphView<'_>) -> Self {
        Self::new(view.warp_id())
    }

    // -------------------------------------------------------------------------
    // Node reads
    // -------------------------------------------------------------------------

    /// Declares a node read (adds to `n_read`).
    pub fn reads_node(mut self, id: NodeId) -> Self {
        self.n_read.insert_with_warp(self.warp_id, id);
        self
    }

    /// Declares multiple node reads.
    pub fn reads_nodes(mut self, ids: impl IntoIterator<Item = NodeId>) -> Self {
        for id in ids {
            self.n_read.insert_with_warp(self.warp_id, id);
        }
        self
    }

    /// Declares a node read using an explicit [`NodeKey`] (for cross-warp or unusual cases).
    pub fn reads_node_key(mut self, key: NodeKey) -> Self {
        self.n_read.insert(key);
        self
    }

    // -------------------------------------------------------------------------
    // Node writes
    // -------------------------------------------------------------------------

    /// Declares a node write (adds to `n_write`).
    pub fn writes_node(mut self, id: NodeId) -> Self {
        self.n_write.insert_with_warp(self.warp_id, id);
        self
    }

    /// Declares multiple node writes.
    pub fn writes_nodes(mut self, ids: impl IntoIterator<Item = NodeId>) -> Self {
        for id in ids {
            self.n_write.insert_with_warp(self.warp_id, id);
        }
        self
    }

    /// Declares a node write using an explicit [`NodeKey`].
    pub fn writes_node_key(mut self, key: NodeKey) -> Self {
        self.n_write.insert(key);
        self
    }

    // -------------------------------------------------------------------------
    // Edge reads
    // -------------------------------------------------------------------------

    /// Declares an edge read (adds to `e_read`).
    pub fn reads_edge(mut self, id: EdgeId) -> Self {
        self.e_read.insert_with_warp(self.warp_id, id);
        self
    }

    /// Declares multiple edge reads.
    pub fn reads_edges(mut self, ids: impl IntoIterator<Item = EdgeId>) -> Self {
        for id in ids {
            self.e_read.insert_with_warp(self.warp_id, id);
        }
        self
    }

    // -------------------------------------------------------------------------
    // Edge writes
    // -------------------------------------------------------------------------

    /// Declares an edge write (adds to `e_write`).
    pub fn writes_edge(mut self, id: EdgeId) -> Self {
        self.e_write.insert_with_warp(self.warp_id, id);
        self
    }

    /// Declares multiple edge writes.
    pub fn writes_edges(mut self, ids: impl IntoIterator<Item = EdgeId>) -> Self {
        for id in ids {
            self.e_write.insert_with_warp(self.warp_id, id);
        }
        self
    }

    // -------------------------------------------------------------------------
    // Attachment reads (alpha = node, beta = edge)
    // -------------------------------------------------------------------------

    /// Declares a node attachment read (alpha plane).
    pub fn reads_node_alpha(mut self, node_id: NodeId) -> Self {
        self.a_read.insert(AttachmentKey::node_alpha(NodeKey {
            warp_id: self.warp_id,
            local_id: node_id,
        }));
        self
    }

    /// Declares multiple node attachment reads (alpha plane).
    pub fn reads_nodes_alpha(mut self, ids: impl IntoIterator<Item = NodeId>) -> Self {
        for id in ids {
            self.a_read.insert(AttachmentKey::node_alpha(NodeKey {
                warp_id: self.warp_id,
                local_id: id,
            }));
        }
        self
    }

    /// Declares an edge attachment read (beta plane).
    pub fn reads_edge_beta(mut self, edge_id: EdgeId) -> Self {
        self.a_read.insert(AttachmentKey::edge_beta(EdgeKey {
            warp_id: self.warp_id,
            local_id: edge_id,
        }));
        self
    }

    /// Declares multiple edge attachment reads (beta plane).
    pub fn reads_edges_beta(mut self, ids: impl IntoIterator<Item = EdgeId>) -> Self {
        for id in ids {
            self.a_read.insert(AttachmentKey::edge_beta(EdgeKey {
                warp_id: self.warp_id,
                local_id: id,
            }));
        }
        self
    }

    /// Declares an attachment read using an explicit [`AttachmentKey`].
    pub fn reads_attachment_key(mut self, key: AttachmentKey) -> Self {
        self.a_read.insert(key);
        self
    }

    // -------------------------------------------------------------------------
    // Attachment writes (alpha = node, beta = edge)
    // -------------------------------------------------------------------------

    /// Declares a node attachment write (alpha plane).
    pub fn writes_node_alpha(mut self, node_id: NodeId) -> Self {
        self.a_write.insert(AttachmentKey::node_alpha(NodeKey {
            warp_id: self.warp_id,
            local_id: node_id,
        }));
        self
    }

    /// Declares multiple node attachment writes (alpha plane).
    pub fn writes_nodes_alpha(mut self, ids: impl IntoIterator<Item = NodeId>) -> Self {
        for id in ids {
            self.a_write.insert(AttachmentKey::node_alpha(NodeKey {
                warp_id: self.warp_id,
                local_id: id,
            }));
        }
        self
    }

    /// Declares an edge attachment write (beta plane).
    pub fn writes_edge_beta(mut self, edge_id: EdgeId) -> Self {
        self.a_write.insert(AttachmentKey::edge_beta(EdgeKey {
            warp_id: self.warp_id,
            local_id: edge_id,
        }));
        self
    }

    /// Declares multiple edge attachment writes (beta plane).
    pub fn writes_edges_beta(mut self, ids: impl IntoIterator<Item = EdgeId>) -> Self {
        for id in ids {
            self.a_write.insert(AttachmentKey::edge_beta(EdgeKey {
                warp_id: self.warp_id,
                local_id: id,
            }));
        }
        self
    }

    /// Declares an attachment write using an explicit [`AttachmentKey`].
    pub fn writes_attachment_key(mut self, key: AttachmentKey) -> Self {
        self.a_write.insert(key);
        self
    }

    // -------------------------------------------------------------------------
    // Boundary ports
    // -------------------------------------------------------------------------

    /// Declares a boundary input port.
    pub fn boundary_in(mut self, port_key: PortKey) -> Self {
        self.b_in.insert(self.warp_id, port_key);
        self
    }

    /// Declares a boundary output port.
    pub fn boundary_out(mut self, port_key: PortKey) -> Self {
        self.b_out.insert(self.warp_id, port_key);
        self
    }

    // -------------------------------------------------------------------------
    // Convenience combos
    // -------------------------------------------------------------------------

    /// Declares both node read and its alpha attachment read.
    ///
    /// Common pattern: reading a scope node and its payload.
    pub fn reads_node_with_alpha(self, node_id: NodeId) -> Self {
        self.reads_node(node_id).reads_node_alpha(node_id)
    }

    /// Declares both node write and its alpha attachment write.
    ///
    /// Common pattern: creating a node with an attachment.
    pub fn writes_node_with_alpha(self, node_id: NodeId) -> Self {
        self.writes_node(node_id).writes_node_alpha(node_id)
    }

    /// Declares node read, node write (same id), plus alpha attachment read/write.
    ///
    /// Common pattern: reading and updating an entity's attachment in place.
    pub fn reads_writes_node_alpha(self, node_id: NodeId) -> Self {
        self.reads_node(node_id)
            .reads_node_alpha(node_id)
            .writes_node_alpha(node_id)
    }

    // -------------------------------------------------------------------------
    // Factor mask
    // -------------------------------------------------------------------------

    /// Sets the factor mask.
    pub fn factor_mask(mut self, mask: u64) -> Self {
        self.factor_mask = mask;
        self
    }

    // -------------------------------------------------------------------------
    // Build
    // -------------------------------------------------------------------------

    /// Consumes the builder and returns the constructed [`Footprint`].
    pub fn build(self) -> Footprint {
        Footprint {
            n_read: self.n_read,
            n_write: self.n_write,
            e_read: self.e_read,
            e_write: self.e_write,
            a_read: self.a_read,
            a_write: self.a_write,
            b_in: self.b_in,
            b_out: self.b_out,
            factor_mask: self.factor_mask,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp_core::{make_edge_id, make_node_id, GraphStore, GraphView};

    #[test]
    fn builder_produces_expected_footprint() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let warp_id = view.warp_id();

        let scope = make_node_id("test/scope");
        let target = make_node_id("test/target");
        let edge = make_edge_id("test/edge");

        let footprint = FootprintBuilder::from_view(view)
            .reads_node_with_alpha(scope)
            .writes_node(target)
            .writes_node_alpha(target)
            .writes_edge(edge)
            .build();

        // Verify reads
        assert!(footprint
            .n_read
            .iter()
            .any(|k| k.warp_id == warp_id && k.local_id == scope));
        assert!(footprint.a_read.iter().any(|k| matches!(
            k.owner,
            warp_core::AttachmentOwner::Node(nk) if nk.local_id == scope
        )));

        // Verify writes
        assert!(footprint
            .n_write
            .iter()
            .any(|k| k.warp_id == warp_id && k.local_id == target));
        assert!(footprint.a_write.iter().any(|k| matches!(
            k.owner,
            warp_core::AttachmentOwner::Node(nk) if nk.local_id == target
        )));
        assert!(footprint
            .e_write
            .iter()
            .any(|k| k.warp_id == warp_id && k.local_id == edge));
    }

    #[test]
    fn reads_writes_node_alpha_combo() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = make_node_id("entity");

        let footprint = FootprintBuilder::from_view(view)
            .reads_writes_node_alpha(scope)
            .build();

        // Should have: n_read(scope), a_read(scope), a_write(scope)
        assert_eq!(footprint.n_read.iter().count(), 1);
        assert_eq!(footprint.a_read.iter().count(), 1);
        assert_eq!(footprint.a_write.iter().count(), 1);
        // No n_write from this combo
        assert!(footprint.n_write.is_empty());
    }
}
