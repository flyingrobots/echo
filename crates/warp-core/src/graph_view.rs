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
//! 4. **Enforces declared footprints** (debug/opt-in) - When a [`FootprintGuard`]
//!    is attached, each accessor validates that the accessed resource was declared
//!    in the rule's footprint.
//!
//! # Example
//!
//! ```rust
//! use warp_core::{make_node_id, make_type_id, GraphStore, GraphView, NodeRecord};
//!
//! let mut store = GraphStore::default();
//! let root = make_node_id("root");
//! store.insert_node(root, NodeRecord { ty: make_type_id("demo:root") });
//!
//! let view = GraphView::new(&store);
//!
//! // Read-only access
//! let _warp_id = view.warp_id();
//! assert!(view.node(&root).is_some());
//! ```

#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
use crate::attachment::AttachmentKey;
use crate::attachment::AttachmentValue;
#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
use crate::footprint_guard::FootprintGuard;
use crate::graph::GraphStore;
use crate::ident::{EdgeId, NodeId, WarpId};
#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
use crate::ident::{EdgeKey, NodeKey};
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
///
/// # Adjacency Invariant (Critical)
///
/// **DO NOT** add `edges_to()` or any incoming-edge accessor to this type.
///
/// The footprint enforcement model (`FootprintGuard`) relies on the fact that
/// rules can only observe outgoing edges via `edges_from()`. Reverse adjacency
/// (`to`) is maintained internally by `GraphStore` but deliberately NOT exposed
/// here. If `edges_to()` were added, the adjacency invariant in
/// `op_write_targets()` would need to change: edge mutations would require
/// declaring BOTH `from` AND `to` nodes in `n_write`, significantly complicating
/// footprint declarations.
///
/// See `footprint_guard.rs::op_write_targets()` doc comment for details.
///
/// # Footprint Enforcement (cfg-gated)
///
/// When `debug_assertions` or `footprint_enforce_release` is enabled (and
/// `unsafe_graph` is NOT), each accessor validates that the accessed resource
/// was declared in the rule's footprint. Violations panic with a typed
/// [`FootprintViolation`](crate::footprint_guard::FootprintViolation) payload.
#[derive(Debug, Clone, Copy)]
pub struct GraphView<'a> {
    store: &'a GraphStore,
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    guard: Option<&'a FootprintGuard>,
}

impl<'a> GraphView<'a> {
    /// Creates a new read-only view over the given store (unguarded).
    ///
    /// Used for match/footprint phases where enforcement is not needed.
    #[must_use]
    pub fn new(store: &'a GraphStore) -> Self {
        Self {
            store,
            #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
            #[cfg(not(feature = "unsafe_graph"))]
            guard: None,
        }
    }

    /// Creates a new read-only view with a footprint guard attached.
    ///
    /// Every read accessor will validate against the guard's declared read set.
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    pub(crate) fn new_guarded(store: &'a GraphStore, guard: &'a FootprintGuard) -> Self {
        Self {
            store,
            guard: Some(guard),
        }
    }

    /// Returns the warp instance identifier for this store.
    #[must_use]
    pub fn warp_id(&self) -> WarpId {
        self.store.warp_id()
    }

    /// Returns a shared reference to a node when it exists.
    ///
    /// # Footprint Enforcement
    ///
    /// When guarded, panics if `id` is not in the declared `n_read` set.
    #[must_use]
    pub fn node(&self, id: &NodeId) -> Option<&'a NodeRecord> {
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(guard) = self.guard {
            guard.check_node_read(id);
        }
        self.store.node(id)
    }

    /// Returns the node's attachment value (if any).
    ///
    /// # Footprint Enforcement
    ///
    /// When guarded, panics if the attachment key (constructed from `id` and the
    /// store's `warp_id`) is not in the declared `a_read` set.
    ///
    /// # Single-Slot API Invariant
    ///
    /// The current `GraphStore` has exactly ONE attachment per node (alpha plane).
    /// The `AttachmentKey` is therefore deterministically constructed as
    /// `AttachmentKey::node_alpha(NodeKey { warp_id, local_id: *id })`.
    /// If the API expands to multi-plane attachments, enforcement must expand with it.
    #[must_use]
    pub fn node_attachment(&self, id: &NodeId) -> Option<&'a AttachmentValue> {
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(guard) = self.guard {
            let key = AttachmentKey::node_alpha(NodeKey {
                warp_id: self.store.warp_id(),
                local_id: *id,
            });
            guard.check_attachment_read(&key);
        }
        self.store.node_attachment(id)
    }

    /// Returns an iterator over edges that originate from the provided node.
    ///
    /// Edges are yielded in insertion order. For deterministic traversal
    /// (e.g., snapshot hashing), callers must sort by `EdgeId`.
    ///
    /// # Footprint Enforcement
    ///
    /// When guarded, panics if `id` is not in the declared `n_read` set.
    /// Adjacency queries are implied by node-read access — declaring a node
    /// in `n_read` grants access to its outbound edge list.
    pub fn edges_from(&self, id: &NodeId) -> impl Iterator<Item = &'a EdgeRecord> {
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(guard) = self.guard {
            guard.check_node_read(id);
        }
        self.store.edges_from(id)
    }

    /// Returns `true` if an edge with `edge_id` exists in the store.
    ///
    /// # Footprint Enforcement
    ///
    /// When guarded, panics if `id` is not in the declared `e_read` set.
    #[must_use]
    pub fn has_edge(&self, id: &EdgeId) -> bool {
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(guard) = self.guard {
            guard.check_edge_read(id);
        }
        self.store.has_edge(id)
    }

    /// Returns the edge's attachment value (if any).
    ///
    /// # Footprint Enforcement
    ///
    /// When guarded, panics if the attachment key (constructed from `id` and the
    /// store's `warp_id`) is not in the declared `a_read` set.
    ///
    /// # Single-Slot API Invariant
    ///
    /// The current `GraphStore` has exactly ONE attachment per edge (beta plane).
    /// The `AttachmentKey` is therefore deterministically constructed as
    /// `AttachmentKey::edge_beta(EdgeKey { warp_id, local_id: *id })`.
    #[must_use]
    pub fn edge_attachment(&self, id: &EdgeId) -> Option<&'a AttachmentValue> {
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(guard) = self.guard {
            let key = AttachmentKey::edge_beta(EdgeKey {
                warp_id: self.store.warp_id(),
                local_id: *id,
            });
            guard.check_attachment_read(&key);
        }
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

    /// Invariant: `GraphView` must be exactly one pointer wide in release builds
    /// without footprint enforcement.
    ///
    /// When enforcement is active (debug or feature-gated), the guard field
    /// adds a second pointer. This test is gated to only run in the unguarded
    /// configuration.
    #[cfg(not(any(debug_assertions, feature = "footprint_enforce_release")))]
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

    /// Invariant: `GraphView` exposes `edges_from()` but NOT `edges_to()`.
    ///
    /// This is enforced by the type system (the method simply doesn't exist),
    /// but this test documents the invariant. If you're seeing this test and
    /// considering adding `edges_to()`, **stop and read the struct doc comment**.
    ///
    /// The footprint enforcement model relies on rules only observing outgoing
    /// edges. Adding `edges_to()` would break the adjacency invariant in
    /// `FootprintGuard::op_write_targets()`.
    #[test]
    fn graph_view_no_edges_to_method() {
        // Compile-time invariant: GraphView has edges_from but not edges_to.
        // This test exists to document the invariant; the method's absence
        // is enforced by the type system.
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let node_id = make_node_id("test");

        // edges_from exists and returns an iterator
        let _ = view.edges_from(&node_id);

        // If you add edges_to() to GraphView, this comment is a reminder:
        // you MUST update op_write_targets() to require `to` nodes in n_write
        // for edge mutations, and update all existing footprint declarations.
    }
}
