// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal in-memory graph store used by the rewrite executor and tests.
use std::collections::BTreeMap;

use thiserror::Error;

use crate::attachment::AttachmentValue;
use crate::ident::{EdgeId, Hash, NodeId, WarpId};
use crate::record::{EdgeRecord, NodeRecord};

/// Error returned by [`GraphStore::delete_node_isolated`].
///
/// `DeleteNode` must not cascade. If the node has incident edges, the caller
/// must emit explicit `DeleteEdge` ops first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DeleteNodeError {
    /// The node does not exist in the store.
    #[error("node not found")]
    NodeNotFound,
    /// The node has outgoing edges; delete them first.
    #[error("node has outgoing edges")]
    HasOutgoingEdges,
    /// The node has incoming edges; delete them first.
    #[error("node has incoming edges")]
    HasIncomingEdges,
}

/// In-memory graph storage for the spike.
///
/// The production engine will eventually swap in a content-addressed store,
/// but this structure keeps the motion rewrite spike self-contained.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GraphStore {
    /// Warp instance identifier for this store (Stage B1).
    pub(crate) warp_id: WarpId,
    /// Mapping from node identifiers to their materialised records.
    pub(crate) nodes: BTreeMap<NodeId, NodeRecord>,
    /// Mapping from source node to outbound edge records.
    pub(crate) edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>,
    /// Reverse adjacency: mapping from destination node to inbound edge ids.
    ///
    /// This allows `delete_node_cascade` to remove inbound edges without scanning
    /// every `edges_from` bucket (removal becomes `O(inbound_edges)`).
    pub(crate) edges_to: BTreeMap<NodeId, Vec<EdgeId>>,
    /// Attachment plane payloads for nodes (Paper I `α` plane).
    ///
    /// Entries are present only when the attachment is `Some(...)`.
    pub(crate) node_attachments: BTreeMap<NodeId, AttachmentValue>,
    /// Attachment plane payloads for edges (Paper I `β` plane).
    ///
    /// Entries are present only when the attachment is `Some(...)`.
    pub(crate) edge_attachments: BTreeMap<EdgeId, AttachmentValue>,
    /// Reverse index of `EdgeId -> from NodeId`.
    ///
    /// This enables efficient edge migration/removal by id (used by tick patch replay),
    /// avoiding `O(total_edges)` scans across all buckets.
    pub(crate) edge_index: BTreeMap<EdgeId, NodeId>,
    /// Reverse index of `EdgeId -> to NodeId`.
    ///
    /// This enables efficient maintenance of [`GraphStore::edges_to`] during
    /// edge migration and deletion.
    pub(crate) edge_to_index: BTreeMap<EdgeId, NodeId>,
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new(crate::ident::make_warp_id("root"))
    }
}

impl GraphStore {
    /// Creates an empty store for `warp_id`.
    #[must_use]
    pub fn new(warp_id: WarpId) -> Self {
        Self {
            warp_id,
            nodes: BTreeMap::new(),
            edges_from: BTreeMap::new(),
            edges_to: BTreeMap::new(),
            node_attachments: BTreeMap::new(),
            edge_attachments: BTreeMap::new(),
            edge_index: BTreeMap::new(),
            edge_to_index: BTreeMap::new(),
        }
    }

    /// Returns the warp instance identifier for this store.
    #[must_use]
    pub fn warp_id(&self) -> WarpId {
        self.warp_id
    }

    /// Iterate over all nodes (id, record) in deterministic order.
    pub fn iter_nodes(&self) -> impl Iterator<Item = (&NodeId, &NodeRecord)> {
        self.nodes.iter()
    }

    /// Iterate over all outbound edge lists per source node.
    pub fn iter_edges(&self) -> impl Iterator<Item = (&NodeId, &Vec<EdgeRecord>)> {
        self.edges_from.iter()
    }

    /// Returns a shared reference to a node when it exists.
    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.nodes.get(id)
    }

    /// Returns the node's attachment value (if any).
    pub fn node_attachment(&self, id: &NodeId) -> Option<&AttachmentValue> {
        self.node_attachments.get(id)
    }

    /// Iterate over all node attachment entries (id, value) in deterministic order.
    ///
    /// The attachment plane stores entries only when a value exists (`Some`);
    /// this iterator yields those present values.
    pub fn iter_node_attachments(&self) -> impl Iterator<Item = (&NodeId, &AttachmentValue)> {
        self.node_attachments.iter()
    }

    /// Returns a mutable reference to the node's attachment value (if any).
    pub fn node_attachment_mut(&mut self, id: &NodeId) -> Option<&mut AttachmentValue> {
        self.node_attachments.get_mut(id)
    }

    /// Returns an iterator over edges that originate from the provided node.
    ///
    /// Edges are yielded in insertion order. For deterministic traversal
    /// (e.g., snapshot hashing), callers must sort by `EdgeId`.
    pub fn edges_from(&self, id: &NodeId) -> impl Iterator<Item = &EdgeRecord> {
        self.edges_from.get(id).into_iter().flatten()
    }

    /// Returns a mutable reference to a node when it exists.
    pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut NodeRecord> {
        self.nodes.get_mut(id)
    }

    /// Sets the node's attachment value.
    ///
    /// Passing `None` clears any existing attachment.
    pub fn set_node_attachment(&mut self, id: NodeId, value: Option<AttachmentValue>) {
        match value {
            None => {
                self.node_attachments.remove(&id);
            }
            Some(v) => {
                self.node_attachments.insert(id, v);
            }
        }
    }

    /// Returns the edge's attachment value (if any).
    pub fn edge_attachment(&self, id: &EdgeId) -> Option<&AttachmentValue> {
        self.edge_attachments.get(id)
    }

    /// Iterate over all edge attachment entries (id, value) in deterministic order.
    ///
    /// The attachment plane stores entries only when a value exists (`Some`);
    /// this iterator yields those present values.
    pub fn iter_edge_attachments(&self) -> impl Iterator<Item = (&EdgeId, &AttachmentValue)> {
        self.edge_attachments.iter()
    }

    /// Returns `true` if an edge with `edge_id` exists in the store.
    #[must_use]
    pub fn has_edge(&self, edge_id: &EdgeId) -> bool {
        self.edge_index.contains_key(edge_id)
    }

    /// Returns a mutable reference to the edge's attachment value (if any).
    pub fn edge_attachment_mut(&mut self, id: &EdgeId) -> Option<&mut AttachmentValue> {
        self.edge_attachments.get_mut(id)
    }

    /// Sets the edge's attachment value.
    ///
    /// Passing `None` clears any existing attachment.
    pub fn set_edge_attachment(&mut self, id: EdgeId, value: Option<AttachmentValue>) {
        match value {
            None => {
                self.edge_attachments.remove(&id);
            }
            Some(v) => {
                self.edge_attachments.insert(id, v);
            }
        }
    }

    /// Inserts or replaces a node in the store.
    pub fn insert_node(&mut self, id: NodeId, record: NodeRecord) {
        self.nodes.insert(id, record);
    }

    /// Inserts or replaces a directed edge in the store.
    ///
    /// If an edge with the same `EdgeId` already exists (in any bucket), the
    /// old edge is removed before inserting the new one. This maintains `EdgeId`
    /// uniqueness across the entire store.
    ///
    /// Ordering note: Edges within a bucket preserve insertion order. When
    /// deterministic ordering is required (e.g., snapshot hashing), callers must
    /// sort by `EdgeId` explicitly.
    pub fn insert_edge(&mut self, from: NodeId, edge: EdgeRecord) {
        self.upsert_edge_record(from, edge);
    }

    /// Inserts or replaces an edge record and maintains the reverse `EdgeId -> from` index.
    ///
    /// If an edge with the same id already exists (possibly in a different bucket),
    /// this removes the old record first so that `EdgeId` remains unique across the store.
    pub(crate) fn upsert_edge_record(&mut self, from: NodeId, mut edge: EdgeRecord) {
        if edge.from != from {
            debug_assert_eq!(
                edge.from, from,
                "edge.from must match the bucket key passed to insert_edge"
            );
            // Preserve store invariants even if the caller passed an inconsistent record.
            edge.from = from;
        }
        let edge_id = edge.id;
        let to = edge.to;
        let prev_from = self.edge_index.insert(edge_id, from);
        let prev_to = self.edge_to_index.insert(edge_id, to);
        if let Some(prev_from) = prev_from {
            let bucket_is_empty = self.edges_from.get_mut(&prev_from).map_or_else(
                || {
                    debug_assert!(
                        false,
                        "edge index referenced a missing bucket for edge id: {edge_id:?}"
                    );
                    false
                },
                |edges| {
                    let before = edges.len();
                    edges.retain(|e| e.id != edge_id);
                    if edges.len() == before {
                        debug_assert!(
                            false,
                            "edge index referenced an edge missing from its bucket: {edge_id:?}"
                        );
                    }
                    edges.is_empty()
                },
            );
            if bucket_is_empty {
                self.edges_from.remove(&prev_from);
            }
        }
        if let Some(prev_to) = prev_to {
            let bucket_is_empty = self.edges_to.get_mut(&prev_to).map_or_else(
                || {
                    debug_assert!(
                        false,
                        "edge-to index referenced a missing bucket for edge id: {edge_id:?}"
                    );
                    false
                },
                |edges| {
                    let before = edges.len();
                    edges.retain(|id| *id != edge_id);
                    if edges.len() == before {
                        debug_assert!(
                            false,
                            "edge-to index referenced an edge missing from its bucket: {edge_id:?}"
                        );
                    }
                    edges.is_empty()
                },
            );
            if bucket_is_empty {
                self.edges_to.remove(&prev_to);
            }
        }
        self.edges_from.entry(from).or_default().push(edge);
        self.edges_to.entry(to).or_default().push(edge_id);
    }

    /// Deletes a node and removes any attachments and incident edges.
    ///
    /// This is a cascading delete: all edges where this node is the source (`from`)
    /// or target (`to`) are also removed, along with their attachments. Use this
    /// when completely removing an entity from the graph.
    ///
    /// # Returns
    ///
    /// `true` if the node existed and was removed, `false` if the node was not found.
    ///
    /// # Note
    ///
    /// This operation is not transactional on its own. If used during a transaction,
    /// the caller must ensure consistency with the transaction's isolation guarantees.
    pub fn delete_node_cascade(&mut self, node: NodeId) -> bool {
        if self.nodes.remove(&node).is_none() {
            return false;
        }
        self.node_attachments.remove(&node);

        // Remove outgoing edges (the bucket).
        if let Some(out_edges) = self.edges_from.remove(&node) {
            for e in out_edges {
                self.edge_index.remove(&e.id);
                self.edge_to_index.remove(&e.id);
                let remove_bucket = self.edges_to.get_mut(&e.to).map_or_else(
                    || {
                        debug_assert!(
                            false,
                            "edge-to index missing inbound bucket for edge id: {:?}",
                            e.id
                        );
                        false
                    },
                    |edges| {
                        edges.retain(|id| *id != e.id);
                        edges.is_empty()
                    },
                );
                if remove_bucket {
                    self.edges_to.remove(&e.to);
                }
                self.edge_attachments.remove(&e.id);
            }
        }

        // Remove inbound edges (reverse adjacency).
        if let Some(inbound) = self.edges_to.remove(&node) {
            for edge_id in inbound {
                let Some(from) = self.edge_index.remove(&edge_id) else {
                    debug_assert!(
                        false,
                        "edge index missing inbound edge id during delete_node_cascade: {edge_id:?}"
                    );
                    continue;
                };
                let Some(to) = self.edge_to_index.remove(&edge_id) else {
                    debug_assert!(
                        false,
                        "edge-to index missing inbound edge id during delete_node_cascade: {edge_id:?}"
                    );
                    continue;
                };
                debug_assert_eq!(
                    to, node,
                    "inbound edge-to index desynced for edge id: {edge_id:?}"
                );
                let Some(edges) = self.edges_from.get_mut(&from) else {
                    debug_assert!(
                        false,
                        "edge index referenced a missing bucket for inbound edge id: {edge_id:?}"
                    );
                    continue;
                };
                let before = edges.len();
                edges.retain(|e| e.id != edge_id);
                if edges.len() == before {
                    debug_assert!(
                        false,
                        "edge index referenced an inbound edge missing from its bucket: {edge_id:?}"
                    );
                    continue;
                }
                let bucket_is_empty = edges.is_empty();
                if bucket_is_empty {
                    self.edges_from.remove(&from);
                }
                self.edge_attachments.remove(&edge_id);
            }
        }
        true
    }

    /// Deletes an isolated node and its alpha attachment.
    ///
    /// Unlike [`delete_node_cascade`], this method **rejects** deletion if the node
    /// has any incident edges (outgoing or incoming). This ensures that `WarpOp`s
    /// accurately describe the mutation—no hidden side effects on edges.
    ///
    /// # Errors
    ///
    /// - [`DeleteNodeError::NodeNotFound`] if the node does not exist
    /// - [`DeleteNodeError::HasOutgoingEdges`] if the node has outgoing edges
    /// - [`DeleteNodeError::HasIncomingEdges`] if the node has incoming edges
    ///
    /// # Allowed Mini-Cascade
    ///
    /// The node's alpha attachment is deleted as part of this operation. This is
    /// enforceable because the attachment key is derivable from the node key.
    /// Footprint enforcement requires `a_write` to include the alpha attachment.
    pub fn delete_node_isolated(&mut self, node: NodeId) -> Result<(), DeleteNodeError> {
        // Check node exists
        if !self.nodes.contains_key(&node) {
            return Err(DeleteNodeError::NodeNotFound);
        }

        // Check for outgoing edges
        if self.edges_from.get(&node).is_some_and(|e| !e.is_empty()) {
            return Err(DeleteNodeError::HasOutgoingEdges);
        }

        // Check for incoming edges
        if self.edges_to.get(&node).is_some_and(|e| !e.is_empty()) {
            return Err(DeleteNodeError::HasIncomingEdges);
        }

        // Safe to delete: remove node and its attachment
        self.nodes.remove(&node);
        self.node_attachments.remove(&node);

        // Clean up empty edge buckets (defensive; should already be empty)
        self.edges_from.remove(&node);
        self.edges_to.remove(&node);

        Ok(())
    }

    /// Deletes an edge from the specified bucket if it exists and matches the reverse index.
    ///
    /// Returns `true` if an edge was removed; returns `false` if the edge did not exist or
    /// if the reverse index indicates the edge belongs to a different bucket.
    pub fn delete_edge_exact(&mut self, from: NodeId, edge_id: EdgeId) -> bool {
        match self.edge_index.get(&edge_id) {
            Some(current_from) if *current_from == from => {}
            _ => return false,
        }
        let Some(to) = self.edge_to_index.get(&edge_id).copied() else {
            debug_assert!(
                false,
                "edge-to index missing edge id referenced by edge_index: {edge_id:?}"
            );
            return false;
        };
        let Some(edges) = self.edges_from.get_mut(&from) else {
            debug_assert!(
                false,
                "edge index referenced a missing bucket for edge id: {edge_id:?}"
            );
            return false;
        };
        let before = edges.len();
        edges.retain(|e| e.id != edge_id);
        if edges.len() == before {
            debug_assert!(
                false,
                "edge index referenced an edge missing from its bucket: {edge_id:?}"
            );
            return false;
        }
        let bucket_is_empty = edges.is_empty();
        self.edge_index.remove(&edge_id);
        self.edge_to_index.remove(&edge_id);
        if bucket_is_empty {
            self.edges_from.remove(&from);
        }
        let remove_bucket = self.edges_to.get_mut(&to).map_or_else(
            || {
                debug_assert!(
                    false,
                    "edge-to index referenced a missing bucket for edge id: {edge_id:?}"
                );
                false
            },
            |edges| {
                edges.retain(|id| *id != edge_id);
                edges.is_empty()
            },
        );
        if remove_bucket {
            self.edges_to.remove(&to);
        }
        self.edge_attachments.remove(&edge_id);
        true
    }

    /// Computes a canonical hash of the entire graph state.
    ///
    /// This traversal is strictly deterministic:
    /// 1. Header: `b"DIND_STATE_HASH_V2\0"`
    /// 2. Node Count (u64 LE)
    /// 3. Nodes (sorted by `NodeId`): `b"N\0"` + `NodeId` + `TypeId` + Attachment(if any)
    /// 4. Edge Count (u64 LE)
    /// 5. Edges (sorted by `EdgeId`): `b"E\0"` + `EdgeId` + From + To + Type + Attachment(if any)
    ///
    /// # V2 Changes
    ///
    /// V2 uses `u64` for all counts and lengths (node count, edge count, blob length)
    /// to align with the WSC format and avoid truncation issues with large graphs.
    #[must_use]
    pub fn canonical_state_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"DIND_STATE_HASH_V2\0");

        // 1. Nodes (u64 count)
        hasher.update(&(self.nodes.len() as u64).to_le_bytes());
        for (node_id, record) in &self.nodes {
            hasher.update(b"N\0");
            hasher.update(&node_id.0);
            hasher.update(&record.ty.0);

            if let Some(att) = self.node_attachments.get(node_id) {
                hasher.update(b"\x01"); // Has attachment
                Self::hash_attachment(&mut hasher, att);
            } else {
                hasher.update(b"\x00"); // No attachment
            }
        }

        // 2. Edges (Global sort by EdgeId, u64 count)
        // We collect all edges first to sort them definitively.
        let mut all_edges: Vec<&EdgeRecord> = self.edges_from.values().flatten().collect();
        all_edges.sort_by_key(|e| e.id);

        hasher.update(&(all_edges.len() as u64).to_le_bytes());
        for edge in all_edges {
            hasher.update(b"E\0");
            hasher.update(&edge.id.0);
            hasher.update(&edge.from.0);
            hasher.update(&edge.to.0);
            hasher.update(&edge.ty.0);

            if let Some(att) = self.edge_attachments.get(&edge.id) {
                hasher.update(b"\x01"); // Has attachment
                Self::hash_attachment(&mut hasher, att);
            } else {
                hasher.update(b"\x00"); // No attachment
            }
        }

        *hasher.finalize().as_bytes()
    }

    fn hash_attachment(hasher: &mut blake3::Hasher, val: &AttachmentValue) {
        match val {
            AttachmentValue::Atom(atom) => {
                hasher.update(b"ATOM"); // Tag
                hasher.update(&atom.type_id.0);
                // V2: u64 blob length
                hasher.update(&(atom.bytes.len() as u64).to_le_bytes());
                hasher.update(&atom.bytes);
            }
            AttachmentValue::Descend(warp_id) => {
                hasher.update(b"DESC"); // Tag
                hasher.update(&warp_id.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::{make_edge_id, make_node_id, make_type_id};

    #[test]
    fn delete_node_cascade_updates_reverse_indexes() {
        let mut store = GraphStore::default();
        let node_ty = make_type_id("node");
        let edge_ty = make_type_id("edge");

        let a = make_node_id("a");
        let b = make_node_id("b");
        let c = make_node_id("c");
        store.insert_node(a, NodeRecord { ty: node_ty });
        store.insert_node(b, NodeRecord { ty: node_ty });
        store.insert_node(c, NodeRecord { ty: node_ty });

        let e1 = make_edge_id("a->b");
        let e2 = make_edge_id("c->b");
        let e3 = make_edge_id("b->a");
        let e4 = make_edge_id("a->c");
        store.insert_edge(
            a,
            EdgeRecord {
                id: e1,
                from: a,
                to: b,
                ty: edge_ty,
            },
        );
        store.insert_edge(
            c,
            EdgeRecord {
                id: e2,
                from: c,
                to: b,
                ty: edge_ty,
            },
        );
        store.insert_edge(
            b,
            EdgeRecord {
                id: e3,
                from: b,
                to: a,
                ty: edge_ty,
            },
        );
        store.insert_edge(
            a,
            EdgeRecord {
                id: e4,
                from: a,
                to: c,
                ty: edge_ty,
            },
        );

        assert!(store.delete_node_cascade(b));
        assert!(store.node(&b).is_none());

        assert!(!store.has_edge(&e1));
        assert!(!store.has_edge(&e2));
        assert!(!store.has_edge(&e3));
        assert!(store.has_edge(&e4));

        assert!(!store.edge_index.contains_key(&e1));
        assert!(!store.edge_to_index.contains_key(&e1));
        assert!(!store.edge_index.contains_key(&e2));
        assert!(!store.edge_to_index.contains_key(&e2));
        assert!(!store.edge_index.contains_key(&e3));
        assert!(!store.edge_to_index.contains_key(&e3));
        assert_eq!(store.edge_index.get(&e4), Some(&a));
        assert_eq!(store.edge_to_index.get(&e4), Some(&c));

        assert!(!store.edges_to.contains_key(&b));
        assert!(!store.edges_to.contains_key(&a));
        assert_eq!(store.edges_to.get(&c), Some(&vec![e4]));
    }
}
