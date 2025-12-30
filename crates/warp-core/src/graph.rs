// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal in-memory graph store used by the rewrite executor and tests.
use std::collections::BTreeMap;

use crate::attachment::AttachmentValue;
use crate::ident::{EdgeId, NodeId, WarpId};
use crate::record::{EdgeRecord, NodeRecord};

/// In-memory graph storage for the spike.
///
/// The production engine will eventually swap in a content-addressed store,
/// but this structure keeps the motion rewrite spike self-contained.
#[derive(Debug, Clone)]
pub struct GraphStore {
    /// Warp instance identifier for this store (Stage B1).
    pub(crate) warp_id: WarpId,
    /// Mapping from node identifiers to their materialised records.
    pub(crate) nodes: BTreeMap<NodeId, NodeRecord>,
    /// Mapping from source node to outbound edge records.
    pub(crate) edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>,
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
            node_attachments: BTreeMap::new(),
            edge_attachments: BTreeMap::new(),
            edge_index: BTreeMap::new(),
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
        if let Some(prev_from) = self.edge_index.insert(edge_id, from) {
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
        self.edges_from.entry(from).or_default().push(edge);
    }

    /// Deletes a node and removes any attachments and incident edges.
    ///
    /// Returns `true` if the node existed and was removed.
    pub(crate) fn delete_node_cascade(&mut self, node: NodeId) -> bool {
        if self.nodes.remove(&node).is_none() {
            return false;
        }
        self.node_attachments.remove(&node);

        // Remove outgoing edges (the bucket).
        if let Some(out_edges) = self.edges_from.remove(&node) {
            for e in out_edges {
                self.edge_index.remove(&e.id);
                self.edge_attachments.remove(&e.id);
            }
        }

        // Remove inbound edges (scan buckets).
        let mut removed_edge_ids: Vec<EdgeId> = Vec::new();
        let mut empty_buckets: Vec<NodeId> = Vec::new();
        for (from, edges) in &mut self.edges_from {
            let before = edges.len();
            edges.retain(|e| {
                if e.to == node {
                    removed_edge_ids.push(e.id);
                    false
                } else {
                    true
                }
            });
            if before != edges.len() && edges.is_empty() {
                empty_buckets.push(*from);
            }
        }
        for from in empty_buckets {
            self.edges_from.remove(&from);
        }
        for edge_id in removed_edge_ids {
            self.edge_index.remove(&edge_id);
            self.edge_attachments.remove(&edge_id);
        }
        true
    }

    /// Deletes an edge from the specified bucket if it exists and matches the reverse index.
    ///
    /// Returns `true` if an edge was removed; returns `false` if the edge did not exist or
    /// if the reverse index indicates the edge belongs to a different bucket.
    pub(crate) fn delete_edge_exact(&mut self, from: NodeId, edge_id: EdgeId) -> bool {
        match self.edge_index.get(&edge_id) {
            Some(current_from) if *current_from == from => {}
            _ => return false,
        }
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
        if bucket_is_empty {
            self.edges_from.remove(&from);
        }
        self.edge_attachments.remove(&edge_id);
        true
    }
}
