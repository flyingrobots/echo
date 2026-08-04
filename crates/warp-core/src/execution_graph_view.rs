// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Observed read capability for one item's execution frame.
//!
//! [`GraphView`](crate::GraphView) is the immutable read capability used by
//! matchers, footprint computation, and frozen legacy executors. It is `Copy`,
//! and its accessors take `&self`. That shape is why it cannot record. Recording
//! an access mutates the execution transcript, and a `&self` accessor cannot
//! mutate anything it does not own without interior mutability — which
//! `GraphView` explicitly forbids and which would cost it `Sync`.
//!
//! [`ExecutionGraphView`] is the narrower capability that executors use when
//! Echo needs evidence of what an execution actually read. It borrows the
//! immutable declared [`FootprintGuard`] and *exclusively* borrows the
//! worker-local [`ActualFootprint`], so its accessors take `&mut self`. That is
//! ordinary inherited mutability, not interior mutability: it matches the
//! scheduler's existing one-item/one-worker discipline exactly, needs no lock,
//! and leaves `GraphView` and `WorkUnit` untouched.
//!
//! ```text
//! FootprintGuard   prepared declared-access contract   shared     immutable
//! ActualFootprint  observed access transcript          worker     mutable, exclusive
//! GraphStore       basis state being observed          shared     immutable
//! ```
//!
//! # Record before check
//!
//! Every accessor records the attempted access *before* consulting the guard.
//! The guard panics on an undeclared access, so checking first would unwind
//! before the violating coordinate entered the record — leaving Echo holding a
//! purported actual footprint that omits its own counterexample.
//!
//! The order is:
//!
//! ```text
//! canonicalize access key
//! record attempted access
//! check declared authorization
//! perform graph lookup
//! ```
//!
//! # What counts as an access
//!
//! A graph resource coordinate presented to this capability, whether or not the
//! resource exists. Asking whether absent node `N` exists is still an
//! observation of coordinate `N`; defining it otherwise would let a rule probe
//! undeclared coordinates for free whenever the resource happened to be absent.
//!
//! # Axis mapping
//!
//! The recorded axis mirrors the guard's enforcement mapping exactly. It does
//! not invent a finer one, because a recorded access the declaration cannot
//! express would manufacture a false mismatch.
//!
//! | Accessor              | Recorded as                          |
//! | --------------------- | ------------------------------------ |
//! | `node`                | node read                            |
//! | `edges_from`          | node read (adjacency is node-granted) |
//! | `has_edge`            | edge read                            |
//! | `node_attachment`     | canonical node-alpha attachment read |
//! | `edge_attachment`     | canonical edge-beta attachment read  |
//!
//! In particular `edges_from` records a *node* read. Today a rule that declares
//! a node in `n_read` is thereby granted that node's outbound edge list, so
//! recording each returned edge as an edge read would report violations against
//! a declaration that is sound under the enforced contract.

use crate::actual_footprint::ActualFootprint;
use crate::attachment::{AttachmentKey, AttachmentValue};
use crate::graph::GraphStore;
use crate::ident::{EdgeId, EdgeKey, NodeId, NodeKey, WarpId};
use crate::record::{EdgeRecord, NodeRecord};

#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
use crate::footprint_guard::FootprintGuard;

/// Read capability that records what an execution actually touched.
///
/// Deliberately **not** `Copy` and **not** `Clone`. An observation session is
/// owned by exactly one executor for exactly one item; duplicating the
/// capability would imply two writers to one transcript, which is precisely the
/// thing the exclusive borrow is expressing.
///
/// Construct with [`ExecutionGraphView::new_guarded`] when footprint
/// enforcement is active and [`ExecutionGraphView::new`] otherwise. The two
/// constructors correspond to distinct evidence postures: only a guarded view
/// can support a claim that a footprint property was *tested under
/// enforcement*.
#[derive(Debug)]
pub struct ExecutionGraphView<'store, 'frame> {
    store: &'store GraphStore,
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    declared: Option<&'frame FootprintGuard>,
    actual: &'frame mut ActualFootprint,
}

impl<'store, 'frame> ExecutionGraphView<'store, 'frame> {
    /// Creates a recording view with no declared-footprint enforcement.
    ///
    /// Reads are recorded but nothing is checked. Evidence produced through
    /// this constructor cannot support a footprint-soundness claim, because an
    /// empty violation set proves only that nothing was compared.
    pub fn new(store: &'store GraphStore, actual: &'frame mut ActualFootprint) -> Self {
        Self {
            store,
            #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
            #[cfg(not(feature = "unsafe_graph"))]
            declared: None,
            actual,
        }
    }

    /// Creates a recording view that also enforces the declared footprint.
    ///
    /// Each accessor records the attempted coordinate and then applies the
    /// guard, which panics with a typed
    /// [`FootprintViolation`](crate::footprint_guard::FootprintViolation) on an
    /// undeclared access. The record survives that unwind because it lives on
    /// the caller's frame, not inside this view.
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    // Kept `pub(crate)` because `FootprintGuard` is crate private and exporting
    // the checker would leak an enforcement detail.
    pub(crate) fn new_guarded(
        store: &'store GraphStore,
        declared: &'frame FootprintGuard,
        actual: &'frame mut ActualFootprint,
    ) -> Self {
        Self {
            store,
            declared: Some(declared),
            actual,
        }
    }

    /// Returns the warp instance identifier for this store.
    ///
    /// Not an access: this names the observation scope rather than a resource
    /// within it, and the guard does not check it.
    #[must_use]
    pub fn warp_id(&self) -> WarpId {
        self.store.warp_id()
    }

    /// Returns a shared reference to a node when it exists.
    ///
    /// Records a node read for `id` whether or not the node exists.
    pub fn node(&mut self, id: &NodeId) -> Option<&'store NodeRecord> {
        self.actual.record_node_read(*id);
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(declared) = self.declared {
            declared.check_node_read(id);
        }
        self.store.node(id)
    }

    /// Returns an iterator over edges that originate from the provided node.
    ///
    /// Records a **node** read, matching the enforced contract: declaring a
    /// node in `n_read` grants its outbound adjacency.
    pub fn edges_from(&mut self, id: &NodeId) -> impl Iterator<Item = &'store EdgeRecord> {
        self.actual.record_node_read(*id);
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(declared) = self.declared {
            declared.check_node_read(id);
        }
        self.store.edges_from(id)
    }

    /// Returns `true` if an edge with `id` exists in the store.
    ///
    /// Records an edge read for `id` whether or not the edge exists.
    pub fn has_edge(&mut self, id: &EdgeId) -> bool {
        self.actual.record_edge_read(*id);
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(declared) = self.declared {
            declared.check_edge_read(id);
        }
        self.store.has_edge(id)
    }

    /// Returns the node's attachment value (if any).
    ///
    /// Records a read of the canonical node-alpha attachment key, the same key
    /// the guard derives.
    pub fn node_attachment(&mut self, id: &NodeId) -> Option<&'store AttachmentValue> {
        let key = AttachmentKey::node_alpha(NodeKey {
            warp_id: self.store.warp_id(),
            local_id: *id,
        });
        self.actual.record_attachment_read(key);
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(declared) = self.declared {
            declared.check_attachment_read(&key);
        }
        self.store.node_attachment(id)
    }

    /// Returns the edge's attachment value (if any).
    ///
    /// Records a read of the canonical edge-beta attachment key, the same key
    /// the guard derives.
    pub fn edge_attachment(&mut self, id: &EdgeId) -> Option<&'store AttachmentValue> {
        let key = AttachmentKey::edge_beta(EdgeKey {
            warp_id: self.store.warp_id(),
            local_id: *id,
        });
        self.actual.record_attachment_read(key);
        #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
        #[cfg(not(feature = "unsafe_graph"))]
        if let Some(declared) = self.declared {
            declared.check_attachment_read(&key);
        }
        self.store.edge_attachment(id)
    }
}

#[cfg(all(
    test,
    any(debug_assertions, feature = "footprint_enforce_release"),
    not(feature = "unsafe_graph")
))]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::attachment::{AtomPayload, AttachmentValue};
    use crate::footprint::Footprint;
    use crate::ident::{make_edge_id, make_node_id, make_type_id};
    use crate::record::{EdgeRecord, NodeRecord};

    fn store_with_a_and_b() -> (GraphStore, NodeId, NodeId, EdgeId) {
        let mut store = GraphStore::default();
        let node_ty = make_type_id("execution-view-node");
        let edge_ty = make_type_id("execution-view-edge");
        let a = make_node_id("a");
        let b = make_node_id("b");
        store.insert_node(a, NodeRecord { ty: node_ty });
        store.insert_node(b, NodeRecord { ty: node_ty });

        let edge = make_edge_id("a->b");
        store.insert_edge(
            a,
            EdgeRecord {
                id: edge,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        let attachment = AttachmentValue::Atom(AtomPayload {
            type_id: make_type_id("execution-view-payload"),
            bytes: vec![7].into(),
        });
        store.set_node_attachment(a, Some(attachment.clone()));
        store.set_edge_attachment(edge, Some(attachment));

        (store, a, b, edge)
    }

    fn nodes_read(actual: &ActualFootprint) -> Vec<NodeId> {
        actual.nodes_read().copied().collect()
    }

    #[test]
    fn a_declared_node_read_is_recorded_once_and_succeeds() {
        let (store, a, _b, _edge) = store_with_a_and_b();
        let mut declared = Footprint::default();
        declared.n_read.insert(NodeKey {
            warp_id: store.warp_id(),
            local_id: a,
        });
        let guard = FootprintGuard::new(&declared, store.warp_id(), "declared-read", false);

        let mut actual = ActualFootprint::new();
        let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
        assert!(view.node(&a).is_some());
        assert!(view.node(&a).is_some());

        assert_eq!(nodes_read(&actual), vec![a]);
        assert!(actual.is_sound_under(&declared, store.warp_id()));
    }

    #[test]
    fn an_absent_node_is_still_an_observed_coordinate() {
        // Asking whether an absent node exists is an observation of that
        // coordinate. Recording only present resources would let a rule probe
        // undeclared coordinates for free whenever they happened to be empty.
        let (store, a, _b, _edge) = store_with_a_and_b();
        let missing = make_node_id("missing");
        let mut declared = Footprint::default();
        for node in [a, missing] {
            declared.n_read.insert(NodeKey {
                warp_id: store.warp_id(),
                local_id: node,
            });
        }
        let guard = FootprintGuard::new(&declared, store.warp_id(), "absent-read", false);

        let mut actual = ActualFootprint::new();
        let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
        assert!(view.node(&missing).is_none());

        assert_eq!(nodes_read(&actual), vec![missing]);
    }

    #[test]
    fn an_undeclared_read_is_recorded_before_the_guard_panics() {
        // This is the load-bearing ordering. The access that falsifies
        // footprint soundness is exactly the one that unwinds; recording after
        // the check would leave Echo holding an actual footprint missing its
        // own counterexample.
        let (store, a, b, _edge) = store_with_a_and_b();
        let mut declared = Footprint::default();
        declared.n_read.insert(NodeKey {
            warp_id: store.warp_id(),
            local_id: a,
        });
        let guard = FootprintGuard::new(&declared, store.warp_id(), "undeclared-read", false);

        let mut actual = ActualFootprint::new();
        let violation = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
            let _ = view.node(&a);
            let _ = view.node(&b);
            unreachable!("the undeclared read must panic");
        }))
        .expect_err("an undeclared read must trip the guard");

        let violation = violation
            .downcast_ref::<crate::footprint_guard::FootprintViolation>()
            .expect("the guard panics with a typed payload");
        assert_eq!(
            violation.kind,
            crate::footprint_guard::ViolationKind::NodeReadNotDeclared(b)
        );

        // Both the lawful read and the violating one survive the unwind.
        let mut expected = vec![a, b];
        expected.sort_unstable();
        assert_eq!(nodes_read(&actual), expected);
        assert_eq!(
            actual.soundness_violations(&declared, store.warp_id()),
            vec![crate::footprint_guard::ViolationKind::NodeReadNotDeclared(
                b
            )]
        );
    }

    #[test]
    fn the_first_access_violating_is_still_retained() {
        let (store, _a, b, _edge) = store_with_a_and_b();
        let guard = FootprintGuard::new(
            &Footprint::default(),
            store.warp_id(),
            "first-access-violates",
            false,
        );

        let mut actual = ActualFootprint::new();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
            let _ = view.node(&b);
        }))
        .expect_err("an undeclared first read must trip the guard");

        assert_eq!(nodes_read(&actual), vec![b]);
    }

    #[test]
    fn edges_from_records_a_node_read_not_edge_reads() {
        // Enforcement grants outbound adjacency through `n_read`. Recording the
        // returned edges as edge reads would report a violation against a
        // declaration that is sound under the enforced contract.
        let (store, a, _b, _edge) = store_with_a_and_b();
        let mut declared = Footprint::default();
        declared.n_read.insert(NodeKey {
            warp_id: store.warp_id(),
            local_id: a,
        });
        let guard = FootprintGuard::new(&declared, store.warp_id(), "adjacency-read", false);

        let mut actual = ActualFootprint::new();
        let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
        assert_eq!(view.edges_from(&a).count(), 1);

        assert_eq!(nodes_read(&actual), vec![a]);
        assert_eq!(actual.edges_read().count(), 0);
        assert!(actual.is_sound_under(&declared, store.warp_id()));
    }

    #[test]
    fn an_edge_existence_query_records_an_edge_read() {
        let (store, _a, _b, edge) = store_with_a_and_b();
        let mut declared = Footprint::default();
        declared.e_read.insert(EdgeKey {
            warp_id: store.warp_id(),
            local_id: edge,
        });
        let guard = FootprintGuard::new(&declared, store.warp_id(), "edge-read", false);

        let mut actual = ActualFootprint::new();
        let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
        assert!(view.has_edge(&edge));

        assert_eq!(actual.edges_read().copied().collect::<Vec<_>>(), vec![edge]);
        assert_eq!(nodes_read(&actual), Vec::new());
    }

    #[test]
    fn a_node_attachment_query_records_the_canonical_node_alpha_key() {
        let (store, a, _b, _edge) = store_with_a_and_b();
        let key = AttachmentKey::node_alpha(NodeKey {
            warp_id: store.warp_id(),
            local_id: a,
        });
        let mut declared = Footprint::default();
        declared.a_read.insert(key);
        let guard = FootprintGuard::new(&declared, store.warp_id(), "node-attachment", false);

        let mut actual = ActualFootprint::new();
        let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
        assert!(view.node_attachment(&a).is_some());

        assert_eq!(
            actual.attachments_read().copied().collect::<Vec<_>>(),
            vec![key]
        );
        // The attachment axis is distinct: reading a node's attachment is not a
        // node read.
        assert_eq!(nodes_read(&actual), Vec::new());
    }

    #[test]
    fn an_edge_attachment_query_records_the_canonical_edge_beta_key() {
        let (store, _a, _b, edge) = store_with_a_and_b();
        let key = AttachmentKey::edge_beta(EdgeKey {
            warp_id: store.warp_id(),
            local_id: edge,
        });
        let mut declared = Footprint::default();
        declared.a_read.insert(key);
        let guard = FootprintGuard::new(&declared, store.warp_id(), "edge-attachment", false);

        let mut actual = ActualFootprint::new();
        let mut view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
        assert!(view.edge_attachment(&edge).is_some());

        assert_eq!(
            actual.attachments_read().copied().collect::<Vec<_>>(),
            vec![key]
        );
        assert_eq!(actual.edges_read().count(), 0);
    }

    #[test]
    fn an_unguarded_view_records_without_enforcing() {
        // Recording and enforcement are separable, and the separation is why
        // posture must be reported: an empty violation set from an unguarded
        // view proves only that nothing was compared.
        let (store, _a, b, _edge) = store_with_a_and_b();

        let mut actual = ActualFootprint::new();
        let mut view = ExecutionGraphView::new(&store, &mut actual);
        assert!(view.node(&b).is_some());

        assert_eq!(nodes_read(&actual), vec![b]);
        assert_eq!(
            actual.soundness_violations(&Footprint::default(), store.warp_id()),
            vec![crate::footprint_guard::ViolationKind::NodeReadNotDeclared(
                b
            )]
        );
    }

    #[test]
    fn warp_id_is_not_an_access() {
        let (store, _a, _b, _edge) = store_with_a_and_b();
        let guard = FootprintGuard::new(
            &Footprint::default(),
            store.warp_id(),
            "warp-id-not-access",
            false,
        );

        let mut actual = ActualFootprint::new();
        let view = ExecutionGraphView::new_guarded(&store, &guard, &mut actual);
        assert_eq!(view.warp_id(), store.warp_id());

        assert!(actual.is_empty());
    }
}
