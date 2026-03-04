// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC → GraphStore reconstruction.
//!
//! Bridges the gap between the on-disk WSC columnar format and the in-memory
//! `GraphStore` used by warp-core's hash computation APIs. This is the inverse
//! of `warp_core::wsc::build_one_warp_input`.

use bytes::Bytes;

use warp_core::wsc::types::{AttRow, EdgeRow, NodeRow};
use warp_core::wsc::view::WarpView;
use warp_core::{
    AtomPayload, AttachmentValue, EdgeId, EdgeRecord, GraphStore, NodeId, NodeRecord, TypeId,
    WarpId,
};

/// Reconstructs a `GraphStore` from a `WarpView`.
///
/// Iterates the columnar WSC data (nodes, edges, attachments) and populates
/// an in-memory `GraphStore` suitable for hash recomputation via
/// `GraphStore::canonical_state_hash()`.
pub fn graph_store_from_warp_view(view: &WarpView<'_>) -> GraphStore {
    let warp_id = WarpId(*view.warp_id());
    let mut store = GraphStore::new(warp_id);

    // 1. Insert all nodes.
    for node_row in view.nodes() {
        let (node_id, record) = node_row_to_record(node_row);
        store.insert_node(node_id, record);
    }

    // 2. Insert all edges.
    for edge_row in view.edges() {
        let (from, record) = edge_row_to_record(edge_row);
        store.insert_edge(from, record);
    }

    // 3. Reconstruct node attachments.
    for (node_ix, node_row) in view.nodes().iter().enumerate() {
        let node_id = NodeId(node_row.node_id);
        let atts = view.node_attachments(node_ix);
        // WSC stores at most one attachment per node (alpha plane).
        debug_assert!(
            atts.len() <= 1,
            "expected ≤1 node attachment, got {}",
            atts.len()
        );
        if let Some(att) = atts.first() {
            let value = att_row_to_value(att, view);
            store.set_node_attachment(node_id, Some(value));
        }
    }

    // 4. Reconstruct edge attachments.
    for (edge_ix, edge_row) in view.edges().iter().enumerate() {
        let edge_id = EdgeId(edge_row.edge_id);
        let atts = view.edge_attachments(edge_ix);
        // WSC stores at most one attachment per edge (beta plane).
        debug_assert!(
            atts.len() <= 1,
            "expected ≤1 edge attachment, got {}",
            atts.len()
        );
        if let Some(att) = atts.first() {
            let value = att_row_to_value(att, view);
            store.set_edge_attachment(edge_id, Some(value));
        }
    }

    store
}

fn node_row_to_record(row: &NodeRow) -> (NodeId, NodeRecord) {
    (
        NodeId(row.node_id),
        NodeRecord {
            ty: TypeId(row.node_type),
        },
    )
}

fn edge_row_to_record(row: &EdgeRow) -> (NodeId, EdgeRecord) {
    let from = NodeId(row.from_node_id);
    let record = EdgeRecord {
        id: EdgeId(row.edge_id),
        from,
        to: NodeId(row.to_node_id),
        ty: TypeId(row.edge_type),
    };
    (from, record)
}

fn att_row_to_value(att: &AttRow, view: &WarpView<'_>) -> AttachmentValue {
    if att.is_atom() {
        let blob = match view.blob_for_attachment(att) {
            Some(b) => b,
            None => {
                eprintln!("warning: missing blob for atom attachment; using empty payload");
                &[]
            }
        };
        AttachmentValue::Atom(AtomPayload::new(
            TypeId(att.type_or_warp),
            Bytes::copy_from_slice(blob),
        ))
    } else {
        AttachmentValue::Descend(WarpId(att.type_or_warp))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use warp_core::wsc::build::build_one_warp_input;
    use warp_core::wsc::write::write_wsc_one_warp;
    use warp_core::wsc::WscFile;
    use warp_core::{make_edge_id, make_node_id, make_type_id, make_warp_id, Hash};

    /// Creates a simple graph, serializes to WSC, reconstructs, and verifies
    /// the state root hash matches the original.
    #[test]
    fn roundtrip_state_root_matches() {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("TestNode");
        let edge_ty = make_type_id("TestEdge");
        let root = make_node_id("root");
        let child = make_node_id("child");

        let mut store = GraphStore::new(warp);
        store.insert_node(root, NodeRecord { ty: node_ty });
        store.insert_node(child, NodeRecord { ty: node_ty });
        store.insert_edge(
            root,
            EdgeRecord {
                id: make_edge_id("root->child"),
                from: root,
                to: child,
                ty: edge_ty,
            },
        );

        let original_hash = store.canonical_state_hash();

        // Serialize to WSC
        let input = build_one_warp_input(&store, root);
        let schema: Hash = [0u8; 32];
        let wsc_bytes = write_wsc_one_warp(&input, schema, 1).expect("WSC write failed");

        // Reconstruct from WSC
        let file = WscFile::from_bytes(wsc_bytes).expect("WSC load failed");
        let view = file.warp_view(0).expect("warp_view failed");
        let reconstructed = graph_store_from_warp_view(&view);

        let reconstructed_hash = reconstructed.canonical_state_hash();
        assert_eq!(
            original_hash, reconstructed_hash,
            "state root must survive WSC roundtrip"
        );
    }

    /// Verifies that attachments survive the WSC roundtrip.
    #[test]
    fn roundtrip_with_attachments() {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("TestNode");
        let payload_ty = make_type_id("Payload");
        let root = make_node_id("root");

        let mut store = GraphStore::new(warp);
        store.insert_node(root, NodeRecord { ty: node_ty });
        store.set_node_attachment(
            root,
            Some(AttachmentValue::Atom(AtomPayload::new(
                payload_ty,
                Bytes::from_static(&[1, 2, 3, 4, 5, 6, 7, 8]),
            ))),
        );

        let original_hash = store.canonical_state_hash();

        let input = build_one_warp_input(&store, root);
        let wsc_bytes = write_wsc_one_warp(&input, [0u8; 32], 0).expect("WSC write failed");

        let file = WscFile::from_bytes(wsc_bytes).expect("WSC load failed");
        let view = file.warp_view(0).expect("warp_view failed");
        let reconstructed = graph_store_from_warp_view(&view);

        assert_eq!(original_hash, reconstructed.canonical_state_hash());
    }

    /// Empty graph (0 nodes) roundtrips successfully.
    #[test]
    fn roundtrip_empty_graph() {
        let warp = make_warp_id("test");
        let store = GraphStore::new(warp);
        let zero_root = NodeId([0u8; 32]);

        let original_hash = store.canonical_state_hash();

        let input = build_one_warp_input(&store, zero_root);
        let wsc_bytes = write_wsc_one_warp(&input, [0u8; 32], 0).expect("WSC write failed");

        let file = WscFile::from_bytes(wsc_bytes).expect("WSC load failed");
        let view = file.warp_view(0).expect("warp_view failed");
        let reconstructed = graph_store_from_warp_view(&view);

        assert_eq!(original_hash, reconstructed.canonical_state_hash());
    }
}
