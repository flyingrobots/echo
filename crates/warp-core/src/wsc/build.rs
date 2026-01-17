// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bridge from `GraphStore` to WSC format.
//!
//! This module provides the canonical conversion from the in-memory
//! `GraphStore` representation to the columnar `OneWarpInput` format
//! suitable for WSC serialization.
//!
//! # Determinism
//!
//! The conversion is fully deterministic:
//! - Nodes are ordered by `NodeId` (via `BTreeMap` iteration)
//! - Edges are globally sorted by `EdgeId`
//! - Outbound edge lists are sorted by `EdgeId` within each node
//! - Attachments follow the same ordering as their owners

use std::collections::BTreeMap;

use crate::attachment::AttachmentValue;
use crate::graph::GraphStore;
use crate::ident::{EdgeId, NodeId};
use crate::record::EdgeRecord;

use super::types::{AttRow, EdgeRow, NodeRow, OutEdgeRef, Range};
use super::write::OneWarpInput;

/// Converts a `GraphStore` into a `OneWarpInput` for WSC serialization.
///
/// # Arguments
///
/// * `store` - The graph store to convert
/// * `root_node_id` - The root node identifier for this WARP instance.
///   Must refer to an existing node in the store, or be the zero ID for empty stores.
///
/// # Returns
///
/// A `OneWarpInput` containing all graph data in canonical order.
///
/// # Panics
///
/// - Panics if `root_node_id` does not exist in the store (unless both the store
///   is empty and `root_node_id` is the zero ID).
/// - Panics if the store's internal edge index is inconsistent (indicates
///   a store invariant violation, not user error).
///
/// # Determinism
///
/// This function produces identical output for identical graph content,
/// regardless of the order in which nodes/edges were inserted into the store.
pub fn build_one_warp_input(store: &GraphStore, root_node_id: NodeId) -> OneWarpInput {
    // Precondition: root_node_id must exist in the store (or be zero for empty stores)
    let is_empty_store = store.iter_nodes().next().is_none();
    let is_zero_root = root_node_id.0 == [0u8; 32];
    assert!(
        store.node(&root_node_id).is_some() || (is_empty_store && is_zero_root),
        "root_node_id {root_node_id:?} does not exist in GraphStore"
    );

    // 1. NODES: Already sorted by NodeId because BTreeMap
    let nodes: Vec<(NodeId, crate::record::NodeRecord)> = store
        .iter_nodes()
        .map(|(id, rec)| (*id, rec.clone()))
        .collect();

    let node_rows: Vec<NodeRow> = nodes
        .iter()
        .map(|(id, rec)| NodeRow {
            node_id: id.0,
            node_type: rec.ty.0,
        })
        .collect();

    // 2. EDGES: Collect globally + sort by EdgeId (canonical)
    let mut edges_all: Vec<EdgeRecord> = store
        .iter_edges()
        .flat_map(|(_, edges)| edges.iter().cloned())
        .collect();
    edges_all.sort_by_key(|e| e.id);

    let edge_rows: Vec<EdgeRow> = edges_all
        .iter()
        .map(|e| EdgeRow {
            edge_id: e.id.0,
            from_node_id: e.from.0,
            to_node_id: e.to.0,
            edge_type: e.ty.0,
        })
        .collect();

    // Build EdgeId -> edge_ix map (deterministic via BTreeMap)
    let mut edge_ix: BTreeMap<EdgeId, u64> = BTreeMap::new();
    for (ix, e) in edges_all.iter().enumerate() {
        edge_ix.insert(e.id, ix as u64);
    }

    // 3. OUT_INDEX / OUT_EDGES (node_ix order; bucket sorted by EdgeId)
    let mut out_index: Vec<Range> = Vec::with_capacity(node_rows.len());
    let mut out_edges: Vec<OutEdgeRef> = Vec::new();

    for (node_id, _) in &nodes {
        let start = out_edges.len() as u64;

        // Pull this node's outgoing edges, then sort canonically by EdgeId
        let mut bucket: Vec<&EdgeRecord> = store.edges_from(node_id).collect();
        bucket.sort_by_key(|e| e.id);

        for e in bucket {
            // edge_ix is built from the same edges we're iterating, so this
            // can only fail if there's a bug in this function's logic.
            #[allow(clippy::expect_used)]
            let &ix = edge_ix
                .get(&e.id)
                .expect("edge_ix missing entry for edge in bucket - internal invariant violated");
            out_edges.push(OutEdgeRef {
                edge_ix_le: ix.to_le(),
                edge_id: e.id.0,
            });
        }

        let len = (out_edges.len() as u64) - start;
        out_index.push(Range {
            start_le: start.to_le(),
            len_le: len.to_le(),
        });
    }

    // 4. Attachments + blobs
    let mut blobs: Vec<u8> = Vec::new();

    let mut node_atts_index: Vec<Range> = Vec::with_capacity(node_rows.len());
    let mut node_atts: Vec<AttRow> = Vec::new();

    for (node_id, _) in &nodes {
        let start = node_atts.len() as u64;

        if let Some(att) = store.node_attachment(node_id) {
            node_atts.push(att_to_row(att, &mut blobs));
        }

        let len = (node_atts.len() as u64) - start;
        node_atts_index.push(Range {
            start_le: start.to_le(),
            len_le: len.to_le(),
        });
    }

    let mut edge_atts_index: Vec<Range> = Vec::with_capacity(edge_rows.len());
    let mut edge_atts: Vec<AttRow> = Vec::new();

    for e in &edges_all {
        let start = edge_atts.len() as u64;

        if let Some(att) = store.edge_attachment(&e.id) {
            edge_atts.push(att_to_row(att, &mut blobs));
        }

        let len = (edge_atts.len() as u64) - start;
        edge_atts_index.push(Range {
            start_le: start.to_le(),
            len_le: len.to_le(),
        });
    }

    OneWarpInput {
        warp_id: store.warp_id().0,
        root_node_id: root_node_id.0,

        nodes: node_rows,
        edges: edge_rows,

        out_index,
        out_edges,

        node_atts_index,
        node_atts,

        edge_atts_index,
        edge_atts,

        blobs,
    }
}

/// Converts an `AttachmentValue` to an `AttRow`, appending blob data as needed.
fn att_to_row(att: &AttachmentValue, blobs: &mut Vec<u8>) -> AttRow {
    match att {
        AttachmentValue::Atom(atom) => {
            // 8-byte align blob starts (good for mmap + SIMD consumers)
            align8_vec(blobs);
            let off = blobs.len() as u64;
            let bytes: &[u8] = atom.bytes.as_ref();
            blobs.extend_from_slice(bytes);
            let len = bytes.len() as u64;

            AttRow {
                tag: AttRow::TAG_ATOM,
                reserved0: [0u8; 7],
                type_or_warp: atom.type_id.0,
                blob_off_le: off.to_le(),
                blob_len_le: len.to_le(),
            }
        }
        AttachmentValue::Descend(warp_id) => AttRow {
            tag: AttRow::TAG_DESCEND,
            reserved0: [0u8; 7],
            type_or_warp: warp_id.0,
            blob_off_le: 0u64.to_le(),
            blob_len_le: 0u64.to_le(),
        },
    }
}

/// Pads a vector to 8-byte alignment.
fn align8_vec(v: &mut Vec<u8>) {
    let target_len = (v.len() + 7) & !7;
    v.resize(target_len, 0);
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id};
    use crate::record::{EdgeRecord, NodeRecord};

    #[test]
    fn empty_store_builds_empty_input() {
        let warp = make_warp_id("test");
        let store = GraphStore::new(warp);
        // Empty stores require zero root (no nodes = no valid root)
        let root = NodeId([0u8; 32]);

        let input = build_one_warp_input(&store, root);

        assert!(input.nodes.is_empty());
        assert!(input.edges.is_empty());
        assert!(input.out_index.is_empty());
        assert!(input.out_edges.is_empty());
        assert!(input.blobs.is_empty());
    }

    #[test]
    fn single_node_builds_correctly() {
        let warp = make_warp_id("test");
        let mut store = GraphStore::new(warp);
        let node_ty = make_type_id("TestNode");
        let root = make_node_id("root");

        store.insert_node(root, NodeRecord { ty: node_ty });

        let input = build_one_warp_input(&store, root);

        assert_eq!(input.nodes.len(), 1);
        assert_eq!(input.nodes[0].node_id, root.0);
        assert_eq!(input.nodes[0].node_type, node_ty.0);
        assert_eq!(input.out_index.len(), 1);
        assert!(input.out_index[0].is_empty()); // No outgoing edges
    }

    #[test]
    fn edges_sorted_by_id_regardless_of_insertion_order() {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("Node");
        let edge_ty = make_type_id("Edge");

        let a = make_node_id("a");
        let b = make_node_id("b");

        let e1 = make_edge_id("edge1");
        let e2 = make_edge_id("edge2");
        let e3 = make_edge_id("edge3");

        // Build store 1: insert edges in one order
        let mut s1 = GraphStore::new(warp);
        s1.insert_node(a, NodeRecord { ty: node_ty });
        s1.insert_node(b, NodeRecord { ty: node_ty });
        s1.insert_edge(
            a,
            EdgeRecord {
                id: e1,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        s1.insert_edge(
            a,
            EdgeRecord {
                id: e2,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        s1.insert_edge(
            a,
            EdgeRecord {
                id: e3,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );

        // Build store 2: insert edges in different order
        let mut s2 = GraphStore::new(warp);
        s2.insert_node(a, NodeRecord { ty: node_ty });
        s2.insert_node(b, NodeRecord { ty: node_ty });
        s2.insert_edge(
            a,
            EdgeRecord {
                id: e3,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        s2.insert_edge(
            a,
            EdgeRecord {
                id: e1,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        s2.insert_edge(
            a,
            EdgeRecord {
                id: e2,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );

        let input1 = build_one_warp_input(&s1, a);
        let input2 = build_one_warp_input(&s2, a);

        // Edge order should be identical (sorted by EdgeId)
        assert_eq!(input1.edges.len(), input2.edges.len());
        for (e1, e2) in input1.edges.iter().zip(input2.edges.iter()) {
            assert_eq!(e1.edge_id, e2.edge_id);
        }

        // Out edges should also be identical
        assert_eq!(input1.out_edges.len(), input2.out_edges.len());
        for (o1, o2) in input1.out_edges.iter().zip(input2.out_edges.iter()) {
            assert_eq!(o1.edge_id, o2.edge_id);
        }
    }

    /// Test that repeated serialization produces byte-identical output.
    #[test]
    fn repeated_serialization_produces_identical_bytes() {
        use super::super::write::write_wsc_one_warp;
        use crate::ident::Hash;

        let warp = make_warp_id("test");
        let node_ty = make_type_id("TestNode");
        let edge_ty = make_type_id("TestEdge");
        let schema_hash: Hash = [0xAB; 32];

        let a = make_node_id("a");
        let b = make_node_id("b");
        let c = make_node_id("c");

        let mut store = GraphStore::new(warp);
        store.insert_node(a, NodeRecord { ty: node_ty });
        store.insert_node(b, NodeRecord { ty: node_ty });
        store.insert_node(c, NodeRecord { ty: node_ty });
        store.insert_edge(
            a,
            EdgeRecord {
                id: make_edge_id("a_to_b"),
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        store.insert_edge(
            b,
            EdgeRecord {
                id: make_edge_id("b_to_c"),
                from: b,
                to: c,
                ty: edge_ty,
            },
        );

        // Serialize 100 times and verify all outputs are identical
        let first_bytes = {
            let input = build_one_warp_input(&store, a);
            write_wsc_one_warp(&input, schema_hash, 42).expect("write failed")
        };

        for i in 1..100 {
            let bytes = {
                let input = build_one_warp_input(&store, a);
                write_wsc_one_warp(&input, schema_hash, 42).expect("write failed")
            };
            assert_eq!(
                first_bytes, bytes,
                "Serialization mismatch at iteration {i}"
            );
        }
    }

    /// Test that different insertion orders produce byte-identical WSC output.
    #[test]
    fn insertion_order_produces_identical_bytes() {
        use super::super::write::write_wsc_one_warp;
        use crate::ident::Hash;

        let warp = make_warp_id("test");
        let node_ty = make_type_id("TestNode");
        let edge_ty = make_type_id("TestEdge");
        let schema_hash: Hash = [0xCD; 32];

        let a = make_node_id("a");
        let b = make_node_id("b");
        let c = make_node_id("c");

        let e1 = make_edge_id("edge1");
        let e2 = make_edge_id("edge2");
        let e3 = make_edge_id("edge3");

        // Store 1: Insert nodes a, b, c and edges in order 1, 2, 3
        let mut s1 = GraphStore::new(warp);
        s1.insert_node(a, NodeRecord { ty: node_ty });
        s1.insert_node(b, NodeRecord { ty: node_ty });
        s1.insert_node(c, NodeRecord { ty: node_ty });
        s1.insert_edge(
            a,
            EdgeRecord {
                id: e1,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        s1.insert_edge(
            a,
            EdgeRecord {
                id: e2,
                from: a,
                to: c,
                ty: edge_ty,
            },
        );
        s1.insert_edge(
            b,
            EdgeRecord {
                id: e3,
                from: b,
                to: c,
                ty: edge_ty,
            },
        );

        // Store 2: Insert nodes c, a, b and edges in order 3, 1, 2
        let mut s2 = GraphStore::new(warp);
        s2.insert_node(c, NodeRecord { ty: node_ty });
        s2.insert_node(a, NodeRecord { ty: node_ty });
        s2.insert_node(b, NodeRecord { ty: node_ty });
        s2.insert_edge(
            b,
            EdgeRecord {
                id: e3,
                from: b,
                to: c,
                ty: edge_ty,
            },
        );
        s2.insert_edge(
            a,
            EdgeRecord {
                id: e1,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        s2.insert_edge(
            a,
            EdgeRecord {
                id: e2,
                from: a,
                to: c,
                ty: edge_ty,
            },
        );

        let bytes1 = {
            let input = build_one_warp_input(&s1, a);
            write_wsc_one_warp(&input, schema_hash, 100).expect("write failed")
        };

        let bytes2 = {
            let input = build_one_warp_input(&s2, a);
            write_wsc_one_warp(&input, schema_hash, 100).expect("write failed")
        };

        assert_eq!(
            bytes1, bytes2,
            "Different insertion orders MUST produce identical WSC bytes"
        );
    }

    /// Test that attachments (blobs) are deterministically serialized.
    #[test]
    fn attachments_serialized_deterministically() {
        use super::super::write::write_wsc_one_warp;
        use crate::attachment::AtomPayload;
        use crate::ident::Hash;

        let warp = make_warp_id("test");
        let node_ty = make_type_id("TestNode");
        let payload_ty = make_type_id("TestPayload");
        let schema_hash: Hash = [0xEF; 32];

        let a = make_node_id("a");
        let b = make_node_id("b");

        // Store 1: Insert nodes in order a, b
        let mut s1 = GraphStore::new(warp);
        s1.insert_node(a, NodeRecord { ty: node_ty });
        s1.insert_node(b, NodeRecord { ty: node_ty });
        let payload = AtomPayload::new(payload_ty, vec![1, 2, 3, 4, 5, 6, 7, 8].into());
        s1.set_node_attachment(a, Some(AttachmentValue::Atom(payload.clone())));

        // Store 2: Insert nodes in order b, a (attachment still on 'a')
        let mut s2 = GraphStore::new(warp);
        s2.insert_node(b, NodeRecord { ty: node_ty });
        s2.insert_node(a, NodeRecord { ty: node_ty });
        s2.set_node_attachment(a, Some(AttachmentValue::Atom(payload)));

        let bytes1 = {
            let input = build_one_warp_input(&s1, a);
            write_wsc_one_warp(&input, schema_hash, 0).expect("write failed")
        };

        let bytes2 = {
            let input = build_one_warp_input(&s2, a);
            write_wsc_one_warp(&input, schema_hash, 0).expect("write failed")
        };

        assert_eq!(
            bytes1, bytes2,
            "Node insertion order MUST NOT affect WSC output with attachments"
        );
    }

    #[test]
    #[should_panic(expected = "does not exist in GraphStore")]
    fn build_panics_on_missing_root() {
        let warp = make_warp_id("test");
        let node_ty = make_type_id("TestNode");

        let existing = make_node_id("existing");
        let missing = make_node_id("missing");

        let mut store = GraphStore::new(warp);
        store.insert_node(existing, NodeRecord { ty: node_ty });

        // This should panic because 'missing' is not in the store
        let _ = build_one_warp_input(&store, missing);
    }

    #[test]
    fn build_accepts_zero_root_for_empty_store() {
        let warp = make_warp_id("test");
        let store = GraphStore::new(warp);
        let zero_root = NodeId([0u8; 32]);

        // This should NOT panic - zero root is valid for empty store
        let input = build_one_warp_input(&store, zero_root);
        assert!(input.nodes.is_empty());
    }
}
