// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests: DeleteNode must NOT cascade.
//!
//! DeleteNode may only delete an isolated node (no in-edges, no out-edges).
//! If edges exist, the caller must emit explicit DeleteEdge ops first.
//!
//! This enforces the invariant that WarpOps describe their mutations explicitly—
//! no hidden side effects that break footprint enforcement.

use warp_core::{
    make_edge_id, make_node_id, make_type_id, DeleteNodeError, EdgeRecord, GraphStore, NodeRecord,
};

// =============================================================================
// GraphStore::delete_node_isolated semantics
// =============================================================================

#[test]
fn delete_node_isolated_succeeds_for_isolated_node() {
    let mut store = GraphStore::default();
    let node = make_node_id("isolated");

    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id("ty"),
        },
    );
    assert!(store.node(&node).is_some());

    let result = store.delete_node_isolated(node);
    assert!(result.is_ok(), "isolated node delete should succeed");
    assert!(store.node(&node).is_none(), "node should be gone");
}

#[test]
fn delete_node_isolated_clears_alpha_attachment() {
    use warp_core::{AtomPayload, AttachmentValue};

    let mut store = GraphStore::default();
    let node = make_node_id("with-attachment");

    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id("ty"),
        },
    );
    store.set_node_attachment(
        node,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("payload"),
            bytes::Bytes::from_static(b"data"),
        ))),
    );

    assert!(store.node_attachment(&node).is_some());

    let result = store.delete_node_isolated(node);
    assert!(result.is_ok());
    assert!(store.node(&node).is_none());
    assert!(
        store.node_attachment(&node).is_none(),
        "alpha attachment must be cleared"
    );
}

#[test]
fn delete_node_isolated_rejects_if_outgoing_edges_exist() {
    let mut store = GraphStore::default();
    let a = make_node_id("a");
    let b = make_node_id("b");
    let ty = make_type_id("ty");

    store.insert_node(a, NodeRecord { ty });
    store.insert_node(b, NodeRecord { ty });
    store.insert_edge(
        a,
        EdgeRecord {
            id: make_edge_id("a->b"),
            from: a,
            to: b,
            ty: make_type_id("edge"),
        },
    );

    let result = store.delete_node_isolated(a);
    assert!(
        matches!(result, Err(DeleteNodeError::HasOutgoingEdges)),
        "should reject: node has outgoing edge"
    );
    assert!(store.node(&a).is_some(), "node must not be deleted");
}

#[test]
fn delete_node_isolated_rejects_if_incoming_edges_exist() {
    let mut store = GraphStore::default();
    let a = make_node_id("a");
    let b = make_node_id("b");
    let ty = make_type_id("ty");

    store.insert_node(a, NodeRecord { ty });
    store.insert_node(b, NodeRecord { ty });
    store.insert_edge(
        b,
        EdgeRecord {
            id: make_edge_id("b->a"),
            from: b,
            to: a,
            ty: make_type_id("edge"),
        },
    );

    let result = store.delete_node_isolated(a);
    assert!(
        matches!(result, Err(DeleteNodeError::HasIncomingEdges)),
        "should reject: node has incoming edge"
    );
    assert!(store.node(&a).is_some(), "node must not be deleted");
}

#[test]
fn delete_node_isolated_rejects_missing_node() {
    let mut store = GraphStore::default();
    let missing = make_node_id("missing");

    let result = store.delete_node_isolated(missing);
    assert!(
        matches!(result, Err(DeleteNodeError::NodeNotFound)),
        "should reject: node doesn't exist"
    );
}

// NOTE: tick_patch and worldline internal functions (apply_op_to_state,
// apply_warp_op_to_store) are tested implicitly through the GraphStore tests.
// Both paths call delete_node_isolated() which we've tested above.
