// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Read-only view over a [`GraphStore`] for Phase 5 BOAW execution.
//!
//! [`GraphView`] provides a strictly read-only interface to the graph, exposing
//! only query methods. This ensures that execute functions cannot accidentally
//! mutate the graph state during pattern matching or read phases.
//!
//! # Design Rationale
//!
//! Phase 5 of the BOAW (Bag-Of-Authoritative-Writes) architecture requires that
//! execute functions read graph state through a controlled interface that:
//!
//! 1. **Prevents mutation** - Execute functions must not modify the graph directly;
//!    all changes flow through the delta accumulator.
//! 2. **Provides efficient lookups** - Common queries (node lookup, edge traversal,
//!    attachment access) are delegated directly to the underlying store.
//! 3. **Maintains borrow safety** - The lifetime `'a` ties the view to the store,
//!    preventing use-after-free scenarios.
//!
//! # Example
//!
//! ```ignore
//! use warp_core::{GraphStore, GraphView};
//!
//! let store = GraphStore::default();
//! let view = GraphView::new(&store);
//!
//! // Read-only access
//! let warp_id = view.warp_id();
//! if let Some(node) = view.node(&some_node_id) {
//!     // Inspect node...
//! }
//! ```

use crate::attachment::AttachmentValue;
use crate::graph::GraphStore;
use crate::ident::{EdgeId, NodeId, WarpId};
use crate::record::{EdgeRecord, NodeRecord};

/// Read-only view over a [`GraphStore`].
///
/// This wrapper exposes only the query methods of the underlying store,
/// enforcing read-only access at compile time. Used by execute functions
/// in Phase 5 BOAW to safely inspect graph state without mutation.
///
/// # BOAW Enforcement Boundary (Phase 5)
///
/// **DO NOT** add any of the following to this type:
/// - `Deref<Target=GraphStore>` or `AsRef<GraphStore>`
/// - `into_inner()`, `as_inner()`, or any method returning `&GraphStore`
/// - `as_mut()` or any method returning `&mut GraphStore`
/// - Interior mutability (`Cell`, `RefCell`, `UnsafeCell`)
///
/// This type is the read-only capability that enforces the BOAW contract:
/// executors observe through `GraphView`, mutate through `TickDelta`.
#[derive(Debug, Clone, Copy)]
pub struct GraphView<'a> {
    store: &'a GraphStore,
}

impl<'a> GraphView<'a> {
    /// Creates a new read-only view over the given store.
    #[must_use]
    pub fn new(store: &'a GraphStore) -> Self {
        Self { store }
    }

    /// Returns the warp instance identifier for this store.
    #[must_use]
    pub fn warp_id(&self) -> WarpId {
        self.store.warp_id()
    }

    /// Returns a shared reference to a node when it exists.
    #[must_use]
    pub fn node(&self, id: &NodeId) -> Option<&'a NodeRecord> {
        self.store.node(id)
    }

    /// Returns the node's attachment value (if any).
    #[must_use]
    pub fn node_attachment(&self, id: &NodeId) -> Option<&'a AttachmentValue> {
        self.store.node_attachment(id)
    }

    /// Returns an iterator over edges that originate from the provided node.
    ///
    /// Edges are yielded in insertion order. For deterministic traversal
    /// (e.g., snapshot hashing), callers must sort by `EdgeId`.
    pub fn edges_from(&self, id: &NodeId) -> impl Iterator<Item = &'a EdgeRecord> {
        self.store.edges_from(id)
    }

    /// Returns `true` if an edge with `edge_id` exists in the store.
    #[must_use]
    pub fn has_edge(&self, id: &EdgeId) -> bool {
        self.store.has_edge(id)
    }

    /// Returns the edge's attachment value (if any).
    #[must_use]
    pub fn edge_attachment(&self, id: &EdgeId) -> Option<&'a AttachmentValue> {
        self.store.edge_attachment(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::{make_edge_id, make_node_id, make_type_id};

    #[test]
    fn graph_view_provides_read_only_access() {
        let mut store = GraphStore::default();
        let node_ty = make_type_id("test_node");
        let edge_ty = make_type_id("test_edge");

        let a = make_node_id("a");
        let b = make_node_id("b");
        store.insert_node(a, NodeRecord { ty: node_ty });
        store.insert_node(b, NodeRecord { ty: node_ty });

        let e1 = make_edge_id("a->b");
        store.insert_edge(
            a,
            EdgeRecord {
                id: e1,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );

        let view = GraphView::new(&store);

        // Verify read access works
        assert_eq!(view.warp_id(), store.warp_id());
        assert!(view.node(&a).is_some());
        assert!(view.node(&b).is_some());
        assert!(view.node(&make_node_id("nonexistent")).is_none());

        let edges: Vec<_> = view.edges_from(&a).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].id, e1);

        assert!(view.has_edge(&e1));
        assert!(!view.has_edge(&make_edge_id("nonexistent")));
    }

    #[test]
    fn graph_view_attachment_access() {
        use crate::attachment::{AtomPayload, AttachmentValue};

        let mut store = GraphStore::default();
        let node_ty = make_type_id("test_node");
        let edge_ty = make_type_id("test_edge");
        let payload_ty = make_type_id("payload");

        let a = make_node_id("a");
        let b = make_node_id("b");
        store.insert_node(a, NodeRecord { ty: node_ty });
        store.insert_node(b, NodeRecord { ty: node_ty });

        let attachment = AttachmentValue::Atom(AtomPayload {
            type_id: payload_ty,
            bytes: vec![1, 2, 3].into(),
        });
        store.set_node_attachment(a, Some(attachment.clone()));

        let e1 = make_edge_id("a->b");
        store.insert_edge(
            a,
            EdgeRecord {
                id: e1,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        store.set_edge_attachment(e1, Some(attachment));

        let view = GraphView::new(&store);

        // Verify attachment access
        assert!(view.node_attachment(&a).is_some());
        assert!(view.node_attachment(&b).is_none());
        assert!(view.edge_attachment(&e1).is_some());
        assert!(view.edge_attachment(&make_edge_id("nonexistent")).is_none());
    }

    /// Invariant: `GraphView` must be exactly one pointer wide.
    ///
    /// This ensures it remains a cheap pass-by-value type (`Copy`).
    /// If someone adds extra fields, this test will fail.
    #[test]
    fn graph_view_is_pointer_sized() {
        use core::mem::size_of;
        assert_eq!(size_of::<GraphView<'_>>(), size_of::<*const ()>());
    }

    /// Invariant: `GraphView` must be `Sync` for Phase 6 parallel execution.
    ///
    /// Workers will share `&GraphStore` across threads; `GraphView` wraps that.
    #[test]
    fn graph_view_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<GraphView<'_>>();
    }

    /// Invariant: `GraphView` must be `Send` for Phase 6 parallel execution.
    #[test]
    fn graph_view_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<GraphView<'_>>();
    }
}
