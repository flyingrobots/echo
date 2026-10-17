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

/// Prior values of the slots written while a [`GraphStore`] records touches.
///
/// Each slot's value is captured the first time the slot is written after
/// recording begins, so the log holds, for exactly the slots that may have
/// changed, the value they had when recording began. A tick's delta is then
/// computed over those slots alone instead of over a copy of the whole graph
/// (see `tick_patch::touched_diff`). The log is bookkeeping, never part of the
/// store's content: a clone starts with an empty, idle log, and `Debug` does
/// not print it.
#[derive(Default)]
pub(crate) struct TouchLog {
    pub(crate) recording: bool,
    /// The state's recording epoch this log belongs to. A log from an older
    /// epoch (left by an aborted commit) is stale and is never read.
    pub(crate) epoch: u64,
    pub(crate) nodes: BTreeMap<NodeId, Option<NodeRecord>>,
    pub(crate) node_attachments: BTreeMap<NodeId, Option<AttachmentValue>>,
    pub(crate) edges: BTreeMap<EdgeId, Option<EdgeRecord>>,
    pub(crate) edge_attachments: BTreeMap<EdgeId, Option<AttachmentValue>>,
    /// Every bucket mutation, in order, so a restore can undo each one
    /// exactly and keep edge order. Its size tracks the edges changed, never
    /// the size of the buckets they sit in.
    pub(crate) bucket_undo: Vec<BucketUndo>,
    /// Prior `edge_index` entries of touched edges.
    pub(crate) edge_froms: BTreeMap<EdgeId, Option<NodeId>>,
    /// Prior `edge_to_index` entries of touched edges.
    pub(crate) edge_tos: BTreeMap<EdgeId, Option<NodeId>>,
}

/// One adjacency-bucket mutation recorded while touches are recorded.
#[derive(Clone)]
pub(crate) enum BucketUndo {
    /// An outbound edge removed from position `index` of `node`'s bucket.
    FromRemoved {
        node: NodeId,
        index: usize,
        edge: EdgeRecord,
    },
    /// An outbound edge pushed onto `node`'s bucket; `created` when the push
    /// created the bucket.
    FromPushed { node: NodeId, created: bool },
    /// `node`'s whole outbound bucket removed, moved here rather than copied.
    FromTaken {
        node: NodeId,
        edges: Vec<EdgeRecord>,
    },
    /// An inbound edge id removed from position `index` of `node`'s bucket.
    ToRemoved {
        node: NodeId,
        index: usize,
        id: EdgeId,
    },
    /// An inbound edge id pushed onto `node`'s bucket.
    ToPushed { node: NodeId, created: bool },
    /// `node`'s whole inbound bucket removed, moved here rather than copied.
    ToTaken { node: NodeId, ids: Vec<EdgeId> },
}

impl Clone for TouchLog {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl TouchLog {
    /// Copies every prior value, unlike `clone`, which starts an empty log for
    /// a cloned store.
    pub(crate) fn duplicate(&self) -> Self {
        Self {
            recording: self.recording,
            nodes: self.nodes.clone(),
            node_attachments: self.node_attachments.clone(),
            edges: self.edges.clone(),
            edge_attachments: self.edge_attachments.clone(),
            bucket_undo: self.bucket_undo.clone(),
            edge_froms: self.edge_froms.clone(),
            edge_tos: self.edge_tos.clone(),
        }
    }
}

impl std::fmt::Debug for TouchLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TouchLog")
    }
}

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
    /// Prior values of the slots written since recording began.
    pub(crate) touched: TouchLog,
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
            touched: TouchLog::default(),
        }
    }

    /// Starts recording touches with an empty log.
    pub(crate) fn begin_touch_log(&mut self, epoch: u64) {
        self.touched = TouchLog {
            recording: true,
            epoch,
            ..TouchLog::default()
        };
    }

    /// Stops recording touches and discards the log.
    pub(crate) fn end_touch_log(&mut self) {
        self.touched = TouchLog::default();
    }

    /// Returns the edge record with `id`, wherever it is stored.
    pub(crate) fn edge_record(&self, id: &EdgeId) -> Option<&EdgeRecord> {
        let from = self.edge_index.get(id)?;
        self.edges_from.get(from)?.iter().find(|e| e.id == *id)
    }

    fn touch_node(&mut self, id: NodeId) {
        if self.touched.recording {
            if let std::collections::btree_map::Entry::Vacant(slot) = self.touched.nodes.entry(id) {
                slot.insert(self.nodes.get(&id).cloned());
            }
        }
    }

    fn touch_node_attachment(&mut self, id: NodeId) {
        if self.touched.recording {
            if let std::collections::btree_map::Entry::Vacant(slot) =
                self.touched.node_attachments.entry(id)
            {
                slot.insert(self.node_attachments.get(&id).cloned());
            }
        }
    }

    fn touch_edge(&mut self, id: EdgeId) {
        if self.touched.recording && !self.touched.edges.contains_key(&id) {
            let prior = self.edge_record(&id).cloned();
            self.touched.edges.insert(id, prior);
            self.touched
                .edge_froms
                .insert(id, self.edge_index.get(&id).copied());
            self.touched
                .edge_tos
                .insert(id, self.edge_to_index.get(&id).copied());
        }
    }

    fn touch_edge_attachment(&mut self, id: EdgeId) {
        if self.touched.recording {
            if let std::collections::btree_map::Entry::Vacant(slot) =
                self.touched.edge_attachments.entry(id)
            {
                slot.insert(self.edge_attachments.get(&id).cloned());
            }
        }
    }

    fn log_bucket(&mut self, undo: BucketUndo) {
        if self.touched.recording {
            self.touched.bucket_undo.push(undo);
        }
    }

    /// Removes edge `id` from `node`'s outbound bucket, dropping the bucket
    /// when it empties. Returns whether the edge was there.
    fn out_remove(&mut self, node: NodeId, id: EdgeId) -> bool {
        let Some(edges) = self.edges_from.get_mut(&node) else {
            return false;
        };
        let Some(index) = edges.iter().position(|e| e.id == id) else {
            return false;
        };
        let edge = edges.remove(index);
        let emptied = edges.is_empty();
        self.log_bucket(BucketUndo::FromRemoved { node, index, edge });
        if emptied {
            self.out_take(node);
        }
        true
    }

    fn out_push(&mut self, node: NodeId, edge: EdgeRecord) {
        let created = !self.edges_from.contains_key(&node);
        self.edges_from.entry(node).or_default().push(edge);
        self.log_bucket(BucketUndo::FromPushed { node, created });
    }

    fn out_take(&mut self, node: NodeId) {
        if let Some(edges) = self.edges_from.remove(&node) {
            self.log_bucket(BucketUndo::FromTaken { node, edges });
        }
    }

    fn in_remove(&mut self, node: NodeId, id: EdgeId) -> bool {
        let Some(ids) = self.edges_to.get_mut(&node) else {
            return false;
        };
        let Some(index) = ids.iter().position(|candidate| *candidate == id) else {
            return false;
        };
        ids.remove(index);
        let emptied = ids.is_empty();
        self.log_bucket(BucketUndo::ToRemoved { node, index, id });
        if emptied {
            self.in_take(node);
        }
        true
    }

    fn in_push(&mut self, node: NodeId, id: EdgeId) {
        let created = !self.edges_to.contains_key(&node);
        self.edges_to.entry(node).or_default().push(id);
        self.log_bucket(BucketUndo::ToPushed { node, created });
    }

    fn in_take(&mut self, node: NodeId) {
        if let Some(ids) = self.edges_to.remove(&node) {
            self.log_bucket(BucketUndo::ToTaken { node, ids });
        }
    }

    /// Restores the store exactly as it was when recording began, then stops
    /// recording.
    ///
    /// Every touched entry of every map, and every touched bucket whole, gets
    /// its prior value back, so the store is identical to its state at the
    /// start, including the order of edges within each bucket.
    pub(crate) fn restore_touches(&mut self) {
        fn put_back<K: Ord, V>(map: &mut BTreeMap<K, V>, priors: BTreeMap<K, Option<V>>) {
            for (key, prior) in priors {
                match prior {
                    Some(value) => {
                        map.insert(key, value);
                    }
                    None => {
                        map.remove(&key);
                    }
                }
            }
        }
        let log = std::mem::take(&mut self.touched);
        put_back(&mut self.nodes, log.nodes);
        put_back(&mut self.node_attachments, log.node_attachments);
        put_back(&mut self.edge_attachments, log.edge_attachments);
        for undo in log.bucket_undo.into_iter().rev() {
            match undo {
                BucketUndo::FromRemoved { node, index, edge } => {
                    self.edges_from.entry(node).or_default().insert(index, edge);
                }
                BucketUndo::FromPushed { node, created } => {
                    if let Some(edges) = self.edges_from.get_mut(&node) {
                        edges.pop();
                        if created && edges.is_empty() {
                            self.edges_from.remove(&node);
                        }
                    }
                }
                BucketUndo::FromTaken { node, edges } => {
                    self.edges_from.insert(node, edges);
                }
                BucketUndo::ToRemoved { node, index, id } => {
                    self.edges_to.entry(node).or_default().insert(index, id);
                }
                BucketUndo::ToPushed { node, created } => {
                    if let Some(ids) = self.edges_to.get_mut(&node) {
                        ids.pop();
                        if created && ids.is_empty() {
                            self.edges_to.remove(&node);
                        }
                    }
                }
                BucketUndo::ToTaken { node, ids } => {
                    self.edges_to.insert(node, ids);
                }
            }
        }
        put_back(&mut self.edge_index, log.edge_froms);
        put_back(&mut self.edge_to_index, log.edge_tos);
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
        self.touch_node_attachment(*id);
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
        self.touch_node(*id);
        self.nodes.get_mut(id)
    }

    /// Sets the node's attachment value.
    ///
    /// Passing `None` clears any existing attachment.
    pub fn set_node_attachment(&mut self, id: NodeId, value: Option<AttachmentValue>) {
        self.touch_node_attachment(id);
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
        self.touch_edge_attachment(*id);
        self.edge_attachments.get_mut(id)
    }

    /// Sets the edge's attachment value.
    ///
    /// Passing `None` clears any existing attachment.
    pub fn set_edge_attachment(&mut self, id: EdgeId, value: Option<AttachmentValue>) {
        self.touch_edge_attachment(id);
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
        self.touch_node(id);
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
        self.touch_edge(edge_id);
        let prev_from = self.edge_index.insert(edge_id, from);
        let prev_to = self.edge_to_index.insert(edge_id, to);
        if let Some(prev_from) = prev_from {
            let removed = self.out_remove(prev_from, edge_id);
            debug_assert!(
                removed,
                "edge index referenced an edge missing from its bucket: {edge_id:?}"
            );
        }
        if let Some(prev_to) = prev_to {
            let removed = self.in_remove(prev_to, edge_id);
            debug_assert!(
                removed,
                "edge-to index referenced an edge missing from its bucket: {edge_id:?}"
            );
        }
        self.out_push(from, edge);
        self.in_push(to, edge_id);
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
        if !self.nodes.contains_key(&node) {
            return false;
        }
        self.touch_node(node);
        self.touch_node_attachment(node);
        self.nodes.remove(&node);
        self.node_attachments.remove(&node);

        // Remove outgoing edges: the whole bucket goes, since every edge in it
        // is deleted.
        let outbound: Vec<(EdgeId, NodeId)> = self
            .edges_from
            .get(&node)
            .into_iter()
            .flatten()
            .map(|e| (e.id, e.to))
            .collect();
        // Capture every outbound edge's prior record before its bucket goes.
        for (edge_id, _) in &outbound {
            self.touch_edge(*edge_id);
            self.touch_edge_attachment(*edge_id);
        }
        self.out_take(node);
        for (edge_id, to) in outbound {
            self.edge_index.remove(&edge_id);
            self.edge_to_index.remove(&edge_id);
            let removed = self.in_remove(to, edge_id);
            debug_assert!(
                removed,
                "edge-to index missing inbound bucket entry for edge id: {edge_id:?}"
            );
            self.edge_attachments.remove(&edge_id);
        }

        // Remove inbound edges (reverse adjacency).
        let inbound: Vec<EdgeId> = self
            .edges_to
            .get(&node)
            .into_iter()
            .flatten()
            .copied()
            .collect();
        self.in_take(node);
        for edge_id in inbound {
            self.touch_edge(edge_id);
            self.touch_edge_attachment(edge_id);
            let Some(from) = self.edge_index.remove(&edge_id) else {
                debug_assert!(
                    false,
                    "edge index missing inbound edge id during delete_node_cascade: {edge_id:?}"
                );
                continue;
            };
            let to = self.edge_to_index.remove(&edge_id);
            debug_assert_eq!(
                to,
                Some(node),
                "inbound edge-to index desynced for edge id: {edge_id:?}"
            );
            let removed = self.out_remove(from, edge_id);
            debug_assert!(
                removed,
                "edge index referenced an inbound edge missing from its bucket: {edge_id:?}"
            );
            self.edge_attachments.remove(&edge_id);
        }
        true
    }

    /// Deletes an isolated node and its alpha attachment.
    ///
    /// Unlike `delete_node_cascade`, this method **rejects** deletion if the node
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
        self.touch_node(node);
        self.touch_node_attachment(node);
        self.nodes.remove(&node);
        self.node_attachments.remove(&node);

        // Clean up empty edge buckets (defensive; should already be empty)
        self.out_take(node);
        self.in_take(node);

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
        self.touch_edge(edge_id);
        self.touch_edge_attachment(edge_id);
        if !self.out_remove(from, edge_id) {
            debug_assert!(
                false,
                "edge index referenced an edge missing from its bucket: {edge_id:?}"
            );
            return false;
        }
        self.edge_index.remove(&edge_id);
        self.edge_to_index.remove(&edge_id);
        let removed = self.in_remove(to, edge_id);
        debug_assert!(
            removed,
            "edge-to index referenced an edge missing from its bucket: {edge_id:?}"
        );
        self.edge_attachments.remove(&edge_id);
        true
    }

    /// Computes a canonical hash of the entire graph state.
    ///
    /// This traversal is strictly deterministic:
    /// 1. Header: `b"echo:state_root:v1\0"`
    /// 2. Node Count (u64 LE)
    /// 3. Nodes (sorted by `NodeId`): `b"N\0"` + `NodeId` + `TypeId` + Attachment(if any)
    /// 4. Edge Count (u64 LE)
    /// 5. Edges (sorted by `EdgeId`): `b"E\0"` + `EdgeId` + From + To + Type + Attachment(if any)
    ///
    /// # Layout Notes
    ///
    /// Uses `u64` for all counts and lengths (node count, edge count, blob length)
    /// to align with the WSC format and avoid truncation issues with large graphs.
    #[must_use]
    pub fn canonical_state_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(crate::domain::STATE_ROOT_V1);

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
                // u64 blob length (aligned with WSC format)
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
