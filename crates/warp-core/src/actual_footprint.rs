// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Actual graph footprints observed during a guarded execution.
//!
//! A [`Footprint`] is a *claim*: it says what a rule intends to read and write.
//! [`FootprintGuard`](crate::footprint_guard::FootprintGuard) checks each access
//! against that claim and panics on a miss, but it accumulates nothing — it
//! answers "was this access declared?" and immediately forgets.
//!
//! [`ActualFootprint`] is the other half: an accumulated record of what an
//! execution *actually* touched. Holding both sides makes the soundness relation
//!
//! ```text
//! ActualRead(a)  ⊆ DeclaredRead(a)
//! ActualWrite(a) ⊆ DeclaredWrite(a)
//! ```
//!
//! evaluable as a value rather than only observable as a panic. That is what a
//! read-only property evaluator needs in order to refute a footprint claim
//! without depending on unwind.
//!
//! # Scope
//!
//! Local ids within a single warp, matching the guard's own pre-filtering. An
//! `ActualFootprint` records accesses for exactly one [`WarpId`]; accesses
//! belonging to another warp are a cross-warp concern and are reported by the
//! guard as [`ViolationKind::CrossWarpEmission`], not by this type.
//!
//! # Determinism
//!
//! All sets are [`BTreeSet`]s and [`ActualFootprint::soundness_violations`]
//! emits violations in a fixed axis order, so the same execution yields the
//! same violation sequence on every host.

use std::collections::BTreeSet;

use crate::attachment::{AttachmentKey, AttachmentOwner};
use crate::footprint::Footprint;
use crate::footprint_guard::ViolationKind;
use crate::ident::{EdgeId, NodeId, WarpId};

#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
use crate::footprint_guard::op_write_targets;
#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
use crate::tick_patch::WarpOp;

/// Graph resources an execution actually touched, as local ids within one warp.
///
/// Construct with [`ActualFootprint::new`], accumulate with the `record_*`
/// methods, then compare against a declared [`Footprint`] with
/// [`ActualFootprint::soundness_violations`].
///
/// Recording is additive and never panics. It does not replace footprint
/// enforcement: the guard's panic remains the correct response to an undeclared
/// access during ordinary execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActualFootprint {
    nodes_read: BTreeSet<NodeId>,
    edges_read: BTreeSet<EdgeId>,
    attachments_read: BTreeSet<AttachmentKey>,
    nodes_write: BTreeSet<NodeId>,
    edges_write: BTreeSet<EdgeId>,
    attachments_write: BTreeSet<AttachmentKey>,
}

impl ActualFootprint {
    /// Creates an empty record.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when nothing has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes_read.is_empty()
            && self.edges_read.is_empty()
            && self.attachments_read.is_empty()
            && self.nodes_write.is_empty()
            && self.edges_write.is_empty()
            && self.attachments_write.is_empty()
    }

    /// Records an observed node read.
    pub fn record_node_read(&mut self, id: NodeId) {
        self.nodes_read.insert(id);
    }

    /// Records an observed edge read.
    pub fn record_edge_read(&mut self, id: EdgeId) {
        self.edges_read.insert(id);
    }

    /// Records an observed attachment read.
    pub fn record_attachment_read(&mut self, key: AttachmentKey) {
        self.attachments_read.insert(key);
    }

    /// Records an observed node write.
    pub fn record_node_write(&mut self, id: NodeId) {
        self.nodes_write.insert(id);
    }

    /// Records an observed edge write.
    pub fn record_edge_write(&mut self, id: EdgeId) {
        self.edges_write.insert(id);
    }

    /// Records an observed attachment write.
    pub fn record_attachment_write(&mut self, key: AttachmentKey) {
        self.attachments_write.insert(key);
    }

    /// Returns the recorded node reads in canonical order.
    pub fn nodes_read(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes_read.iter()
    }

    /// Returns the recorded edge reads in canonical order.
    pub fn edges_read(&self) -> impl Iterator<Item = &EdgeId> {
        self.edges_read.iter()
    }

    /// Returns the recorded attachment reads in canonical order.
    pub fn attachments_read(&self) -> impl Iterator<Item = &AttachmentKey> {
        self.attachments_read.iter()
    }

    /// Returns the recorded node writes in canonical order.
    pub fn nodes_write(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes_write.iter()
    }

    /// Returns the recorded edge writes in canonical order.
    pub fn edges_write(&self) -> impl Iterator<Item = &EdgeId> {
        self.edges_write.iter()
    }

    /// Returns the recorded attachment writes in canonical order.
    pub fn attachments_write(&self) -> impl Iterator<Item = &AttachmentKey> {
        self.attachments_write.iter()
    }

    /// Records every write target of one emitted op.
    ///
    /// Uses the same extraction as footprint enforcement, so a recorded write
    /// set and an enforced write check can never disagree about what an op
    /// mutates. Instance-level and cross-warp concerns are deliberately not
    /// recorded here: they are authority and scope questions that the guard
    /// reports as [`ViolationKind::UnauthorizedInstanceOp`] and
    /// [`ViolationKind::CrossWarpEmission`], not footprint-subset questions.
    ///
    /// Targets belonging to another warp are skipped, matching the guard's
    /// pre-filtering.
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    pub fn record_op(&mut self, op: &WarpOp, warp_id: WarpId) {
        let targets = op_write_targets(op);

        if targets.op_warp.is_some_and(|op_warp| op_warp != warp_id) {
            return;
        }

        for node in targets.nodes {
            self.record_node_write(node);
        }
        for edge in targets.edges {
            self.record_edge_write(edge);
        }
        for attachment in targets.attachments {
            self.record_attachment_write(attachment);
        }
    }

    /// Accumulates the actual write footprint of an emitted op sequence.
    ///
    /// This is the write axis of footprint soundness, computable entirely from
    /// material the executor already produced. The read axis requires observing
    /// accesses as they happen and is not derivable from ops.
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    #[must_use]
    pub fn from_ops<'a>(ops: impl IntoIterator<Item = &'a WarpOp>, warp_id: WarpId) -> Self {
        let mut actual = Self::new();
        for op in ops {
            actual.record_op(op, warp_id);
        }
        actual
    }

    /// Returns every recorded access that the declared footprint does not cover.
    ///
    /// An empty result means `Actual ⊆ Declared` on both axes for `warp_id`:
    /// the declaration is sound with respect to what this execution did. A
    /// declaration that covers *more* than the execution touched is sound; this
    /// relation does not demand footprint minimality.
    ///
    /// Violations are emitted in a fixed axis order — node reads, edge reads,
    /// attachment reads, node writes, edge writes, attachment writes — and
    /// canonically within each axis.
    #[must_use]
    pub fn soundness_violations(
        &self,
        declared: &Footprint,
        warp_id: WarpId,
    ) -> Vec<ViolationKind> {
        let mut violations = Vec::new();

        let declared_nodes_read = declared_nodes(declared.n_read.iter(), warp_id);
        let declared_nodes_write = declared_nodes(declared.n_write.iter(), warp_id);
        let declared_edges_read = declared_edges(declared.e_read.iter(), warp_id);
        let declared_edges_write = declared_edges(declared.e_write.iter(), warp_id);
        let declared_attachments_read = declared_attachments(declared.a_read.iter(), warp_id);
        let declared_attachments_write = declared_attachments(declared.a_write.iter(), warp_id);

        for id in &self.nodes_read {
            if !declared_nodes_read.contains(id) {
                violations.push(ViolationKind::NodeReadNotDeclared(*id));
            }
        }
        for id in &self.edges_read {
            if !declared_edges_read.contains(id) {
                violations.push(ViolationKind::EdgeReadNotDeclared(*id));
            }
        }
        for key in &self.attachments_read {
            if !declared_attachments_read.contains(key) {
                violations.push(ViolationKind::AttachmentReadNotDeclared(*key));
            }
        }
        for id in &self.nodes_write {
            if !declared_nodes_write.contains(id) {
                violations.push(ViolationKind::NodeWriteNotDeclared(*id));
            }
        }
        for id in &self.edges_write {
            if !declared_edges_write.contains(id) {
                violations.push(ViolationKind::EdgeWriteNotDeclared(*id));
            }
        }
        for key in &self.attachments_write {
            if !declared_attachments_write.contains(key) {
                violations.push(ViolationKind::AttachmentWriteNotDeclared(*key));
            }
        }

        violations
    }

    /// Returns `true` when the declared footprint covers every recorded access.
    #[must_use]
    pub fn is_sound_under(&self, declared: &Footprint, warp_id: WarpId) -> bool {
        self.soundness_violations(declared, warp_id).is_empty()
    }
}

fn declared_nodes<'a>(
    keys: impl Iterator<Item = &'a crate::ident::NodeKey>,
    warp_id: WarpId,
) -> BTreeSet<NodeId> {
    keys.filter(|key| key.warp_id == warp_id)
        .map(|key| key.local_id)
        .collect()
}

fn declared_edges<'a>(
    keys: impl Iterator<Item = &'a crate::ident::EdgeKey>,
    warp_id: WarpId,
) -> BTreeSet<EdgeId> {
    keys.filter(|key| key.warp_id == warp_id)
        .map(|key| key.local_id)
        .collect()
}

fn declared_attachments<'a>(
    keys: impl Iterator<Item = &'a AttachmentKey>,
    warp_id: WarpId,
) -> BTreeSet<AttachmentKey> {
    // Matched directly rather than via `AttachmentOwner::warp_id`, which is only
    // compiled under enforcement. Soundness comparison must remain available in
    // every build so retained evidence stays inspectable.
    keys.filter(|key| attachment_warp_id(**key) == warp_id)
        .copied()
        .collect()
}

fn attachment_warp_id(key: AttachmentKey) -> WarpId {
    match key.owner {
        AttachmentOwner::Node(node) => node.warp_id,
        AttachmentOwner::Edge(edge) => edge.warp_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::{make_edge_id, make_node_id, make_warp_id, NodeKey};

    fn warp() -> WarpId {
        make_warp_id("actual-footprint-tests")
    }

    fn other_warp() -> WarpId {
        make_warp_id("actual-footprint-tests-other")
    }

    fn node_attachment(node: NodeId) -> AttachmentKey {
        AttachmentKey::node_alpha(NodeKey {
            warp_id: warp(),
            local_id: node,
        })
    }

    #[test]
    fn empty_record_is_sound_under_any_declaration() {
        let actual = ActualFootprint::new();
        assert!(actual.is_empty());
        assert!(actual.is_sound_under(&Footprint::default(), warp()));
    }

    #[test]
    fn declared_access_produces_no_violation() {
        let node = make_node_id("a");
        let mut declared = Footprint::default();
        declared.n_read.insert(NodeKey {
            warp_id: warp(),
            local_id: node,
        });

        let mut actual = ActualFootprint::new();
        actual.record_node_read(node);

        assert_eq!(actual.soundness_violations(&declared, warp()), Vec::new());
    }

    #[test]
    fn undeclared_node_read_is_reported() {
        let declared_node = make_node_id("a");
        let undeclared_node = make_node_id("b");
        let mut declared = Footprint::default();
        declared.n_read.insert(NodeKey {
            warp_id: warp(),
            local_id: declared_node,
        });

        let mut actual = ActualFootprint::new();
        actual.record_node_read(declared_node);
        actual.record_node_read(undeclared_node);

        assert_eq!(
            actual.soundness_violations(&declared, warp()),
            vec![ViolationKind::NodeReadNotDeclared(undeclared_node)]
        );
    }

    #[test]
    fn superset_declaration_is_sound() {
        // The relation is Actual ⊆ Declared. Declaring more than was touched is
        // sound; this type does not demand footprint minimality.
        let touched = make_node_id("a");
        let untouched = make_node_id("b");
        let mut declared = Footprint::default();
        for node in [touched, untouched] {
            declared.n_read.insert(NodeKey {
                warp_id: warp(),
                local_id: node,
            });
        }

        let mut actual = ActualFootprint::new();
        actual.record_node_read(touched);

        assert!(actual.is_sound_under(&declared, warp()));
    }

    #[test]
    fn read_declaration_does_not_authorize_a_write() {
        let node = make_node_id("a");
        let mut declared = Footprint::default();
        declared.n_read.insert(NodeKey {
            warp_id: warp(),
            local_id: node,
        });

        let mut actual = ActualFootprint::new();
        actual.record_node_write(node);

        assert_eq!(
            actual.soundness_violations(&declared, warp()),
            vec![ViolationKind::NodeWriteNotDeclared(node)]
        );
    }

    #[test]
    fn declaration_in_another_warp_does_not_cover_this_warp() {
        let node = make_node_id("a");
        let mut declared = Footprint::default();
        declared.n_read.insert(NodeKey {
            warp_id: other_warp(),
            local_id: node,
        });

        let mut actual = ActualFootprint::new();
        actual.record_node_read(node);

        assert_eq!(
            actual.soundness_violations(&declared, warp()),
            vec![ViolationKind::NodeReadNotDeclared(node)]
        );
    }

    #[test]
    fn violations_are_ordered_by_axis_then_canonically() {
        let node_b = make_node_id("b");
        let node_a = make_node_id("a");
        let edge = make_edge_id("e");
        let attachment = node_attachment(node_a);

        let mut actual = ActualFootprint::new();
        actual.record_node_read(node_b);
        actual.record_node_read(node_a);
        actual.record_edge_read(edge);
        actual.record_attachment_read(attachment);
        actual.record_node_write(node_a);

        let violations = actual.soundness_violations(&Footprint::default(), warp());
        let mut expected_nodes = [node_a, node_b];
        expected_nodes.sort_unstable();

        assert_eq!(
            violations,
            vec![
                ViolationKind::NodeReadNotDeclared(expected_nodes[0]),
                ViolationKind::NodeReadNotDeclared(expected_nodes[1]),
                ViolationKind::EdgeReadNotDeclared(edge),
                ViolationKind::AttachmentReadNotDeclared(attachment),
                ViolationKind::NodeWriteNotDeclared(node_a),
            ]
        );
    }

    #[test]
    fn recording_is_idempotent() {
        let node = make_node_id("a");

        let mut once = ActualFootprint::new();
        once.record_node_read(node);

        let mut twice = ActualFootprint::new();
        twice.record_node_read(node);
        twice.record_node_read(node);

        assert_eq!(once, twice);
    }

    #[test]
    fn recording_order_does_not_affect_the_record() {
        let node_a = make_node_id("a");
        let node_b = make_node_id("b");

        let mut forward = ActualFootprint::new();
        forward.record_node_read(node_a);
        forward.record_node_read(node_b);

        let mut reverse = ActualFootprint::new();
        reverse.record_node_read(node_b);
        reverse.record_node_read(node_a);

        assert_eq!(forward, reverse);
        assert_eq!(
            forward.soundness_violations(&Footprint::default(), warp()),
            reverse.soundness_violations(&Footprint::default(), warp())
        );
    }

    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    mod from_ops {
        use super::*;
        use crate::record::NodeRecord;
        use crate::tick_patch::WarpOp;

        fn upsert(node: NodeId) -> WarpOp {
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id: warp(),
                    local_id: node,
                },
                record: NodeRecord {
                    ty: crate::ident::make_type_id("actual-footprint-test"),
                },
            }
        }

        #[test]
        fn ops_populate_the_write_axis_and_leave_reads_empty() {
            let node = make_node_id("a");
            let actual = ActualFootprint::from_ops([&upsert(node)], warp());

            assert_eq!(
                actual.nodes_write().copied().collect::<Vec<_>>(),
                vec![node]
            );
            assert_eq!(actual.nodes_read().count(), 0);
        }

        #[test]
        fn undeclared_op_write_is_reported() {
            let node = make_node_id("a");
            let actual = ActualFootprint::from_ops([&upsert(node)], warp());

            assert_eq!(
                actual.soundness_violations(&Footprint::default(), warp()),
                vec![ViolationKind::NodeWriteNotDeclared(node)]
            );
        }

        #[test]
        fn declared_op_write_is_sound() {
            let node = make_node_id("a");
            let mut declared = Footprint::default();
            declared.n_write.insert(NodeKey {
                warp_id: warp(),
                local_id: node,
            });

            let actual = ActualFootprint::from_ops([&upsert(node)], warp());

            assert!(actual.is_sound_under(&declared, warp()));
        }

        #[test]
        fn cross_warp_op_targets_are_not_recorded() {
            // Cross-warp emission is a scope violation the guard reports as
            // CrossWarpEmission. Recording it as a local write would misreport
            // it as a footprint-subset failure.
            let node = make_node_id("a");
            let op = WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id: other_warp(),
                    local_id: node,
                },
                record: NodeRecord {
                    ty: crate::ident::make_type_id("actual-footprint-test"),
                },
            };

            let actual = ActualFootprint::from_ops([&op], warp());

            assert!(actual.is_empty());
        }

        #[test]
        fn edge_write_records_both_edge_and_from_node() {
            // op_write_targets treats an edge mutation as writing the edge and
            // its `from` node. The recorded footprint must agree, or a sound
            // declaration would look unsound.
            let from = make_node_id("from");
            let to = make_node_id("to");
            let edge = make_edge_id("e");
            let op = WarpOp::UpsertEdge {
                warp_id: warp(),
                record: crate::record::EdgeRecord {
                    id: edge,
                    from,
                    to,
                    ty: crate::ident::make_type_id("actual-footprint-test-edge"),
                },
            };

            let actual = ActualFootprint::from_ops([&op], warp());

            assert_eq!(
                actual.edges_write().copied().collect::<Vec<_>>(),
                vec![edge]
            );
            assert_eq!(
                actual.nodes_write().copied().collect::<Vec<_>>(),
                vec![from]
            );
        }
    }
}
