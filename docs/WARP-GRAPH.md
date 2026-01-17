# WARP Graph Store

<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

```rust
//! Minimal in-memory graph store used by the rewrite executor and tests.
use std::collections::BTreeMap;

use crate::attachment::AttachmentValue;
use crate::ident::{EdgeId, Hash, NodeId, WarpId};
use crate::record::{EdgeRecord, NodeRecord};

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
    /// 1. Header: `b"DIND_STATE_HASH_V1\0"`
    /// 2. Node Count (u32 LE)
    /// 3. Nodes (sorted by NodeId): `b"N\0"` + `NodeId` + `TypeId` + Attachment(if any)
    /// 4. Edge Count (u32 LE)
    /// 5. Edges (sorted by EdgeId): `b"E\0"` + `EdgeId` + From + To + Type + Attachment(if any)
    #[allow(clippy::cast_possible_truncation)]
    pub fn canonical_state_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"DIND_STATE_HASH_V1\0");

        // 1. Nodes
        hasher.update(&(self.nodes.len() as u32).to_le_bytes());
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

        // 2. Edges (Global sort by EdgeId)
        // We collect all edges first to sort them definitively.
        let mut all_edges: Vec<&EdgeRecord> = self.edges_from.values().flatten().collect();
        all_edges.sort_by_key(|e| e.id);

        hasher.update(&(all_edges.len() as u32).to_le_bytes());
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

    #[allow(clippy::cast_possible_truncation)]
    fn hash_attachment(hasher: &mut blake3::Hasher, val: &AttachmentValue) {
        match val {
            AttachmentValue::Atom(atom) => {
                hasher.update(b"ATOM"); // Tag
                hasher.update(&atom.type_id.0);
                hasher.update(&(atom.bytes.len() as u32).to_le_bytes());
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
```

---

Yep — your ident newtypes and GraphStore are compatible with the WSC design. We do not need to blow them up.

There are only three changes I'd push, and they're all sane:

1. Add #[repr(transparent)] to NodeId/EdgeId/TypeId/WarpId (so you can safely treat them as "just bytes" everywhere).
2. Add as_bytes() for all IDs (you only have it on NodeId/WarpId right now).
3. For hashing + snapshots: stop truncating lengths/counts to u32 (use u64), or create a *_V2 hash. Your current u32 length hashing is a time bomb.

Everything else can remain as-is.

⸻

Minimal ident tweaks (recommended)

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TypeId(pub Hash);

impl TypeId {
  #[must_use] pub fn as_bytes(&self) -> &Hash { &self.0 }
}

// Do the same for EdgeId too:
#[repr(transparent)]
pub struct EdgeId(pub Hash);
impl EdgeId { pub fn as_bytes(&self) -> &Hash { &self.0 } }

That’s it.

⸻

Build WSC tables from your GraphStore (the missing piece)

Below is the concrete build_one_warp_input() that turns your in-memory GraphStore into the canonical slab tables my writer expects.

Notes:

- I'm not serializing edges_to, edge_index, edge_to_index. They're indexes, not state. We rebuild them during load/overlay anyway.
- GraphStore doesn't store root_node_id, so this takes it as a parameter (you should store that at the "WarpInstance" layer, not inside GraphStore).
- This supports your current "single attachment per node/edge" model (range 0/1). The format supports multiple later.

use std::collections::BTreeMap;

use crate::attachment::AttachmentValue;
use crate::ident::{EdgeId, NodeId, WarpId};
use crate::record::{EdgeRecord, NodeRecord};

// These are the WSC row types from the snapshot module:
use warp_snapshot::wsc::types::{NodeRow, EdgeRow, Range, OutEdgeRef, AttRow};
use warp_snapshot::wsc::write::OneWarpInput;

pub fn build_one_warp_input(store: &crate::graph::GraphStore, root_node_id: NodeId) -> OneWarpInput<'static> {
    // 1) NODES: already sorted by NodeId because BTreeMap
    let nodes: Vec<(NodeId, NodeRecord)> = store
        .nodes
        .iter()
        .map(|(id, rec)| (*id, rec.clone()))
        .collect();

    let node_rows: Vec<NodeRow> = nodes
        .iter()
        .map(|(id, rec)| NodeRow {
            node_id: id.0,
            node_type: rec.ty.0,
        })
        .collect();

    // 2) EDGES: collect globally + sort by EdgeId (canonical)
    let mut edges_all: Vec<EdgeRecord> = store.edges_from.values().flatten().cloned().collect();
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

    // Build EdgeId -> edge_ix map (deterministic builder uses BTreeMap)
    let mut edge_ix: BTreeMap<EdgeId, u64> = BTreeMap::new();
    for (ix, e) in edges_all.iter().enumerate() {
        edge_ix.insert(e.id, ix as u64);
    }

    // 3) OUT_INDEX + OUT_EDGES (group by node order; within group sort by EdgeId)
    let mut out_index: Vec<Range> = Vec::with_capacity(node_rows.len());
    let mut out_edges: Vec<OutEdgeRef> = Vec::new();

    for (node_id, _rec) in &nodes {
        let start = out_edges.len() as u64;

        // Pull this node's outgoing edges (in insertion order), then sort canonically by EdgeId.
        let mut bucket: Vec<&EdgeRecord> = store.edges_from.get(node_id).map(|v| v.iter().collect()).unwrap_or_default();
        bucket.sort_by_key(|e| e.id);

        for e in bucket {
            let ix = *edge_ix.get(&e.id).expect("edge_ix missing for edge in bucket");
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

    // 4) Attachments + blobs
    let mut blobs: Vec<u8> = Vec::new();

    let mut node_atts_index: Vec<Range> = Vec::with_capacity(node_rows.len());
    let mut node_atts: Vec<AttRow> = Vec::new();

    for (node_id, _rec) in &nodes {
        let start = node_atts.len() as u64;

        if let Some(att) = store.node_attachments.get(node_id) {
            node_atts.push(att_to_row(att, &mut blobs));
        }

        let len = (node_atts.len() as u64) - start;
        node_atts_index.push(Range { start_le: start.to_le(), len_le: len.to_le() });
    }

    let mut edge_atts_index: Vec<Range> = Vec::with_capacity(edge_rows.len());
    let mut edge_atts: Vec<AttRow> = Vec::new();

    for e in &edges_all {
        let start = edge_atts.len() as u64;

        if let Some(att) = store.edge_attachments.get(&e.id) {
            edge_atts.push(att_to_row(att, &mut blobs));
        }

        let len = (edge_atts.len() as u64) - start;
        edge_atts_index.push(Range { start_le: start.to_le(), len_le: len.to_le() });
    }

    OneWarpInput {
        warp_id: store.warp_id.0,
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

fn att_to_row(att: &AttachmentValue, blobs: &mut Vec<u8>) -> AttRow {
    match att {
        AttachmentValue::Atom(atom) => {
            // 8-byte align blob starts (good for mmap + SIMD-y consumers)
            align8(blobs);
            let off = blobs.len() as u64;
            let bytes: &[u8] = atom.bytes.as_ref();
            blobs.extend_from_slice(bytes);
            let len = bytes.len() as u64;

            AttRow {
                tag: 1,
                reserved0: [0u8; 7],
                type_or_warp: atom.type_id.0,
                blob_off_le: off.to_le(),
                blob_len_le: len.to_le(),
            }
        }
        AttachmentValue::Descend(warp_id) => AttRow {
            tag: 2,
            reserved0: [0u8; 7],
            type_or_warp: warp_id.0,
            blob_off_le: 0u64.to_le(),
            blob_len_le: 0u64.to_le(),
        },
    }
}

fn align8(v: &mut Vec<u8>) {
    while v.len() % 8 != 0 {
        v.push(0);
    }
}

That function is the bridge between your current spike store and the final WSC snapshot format.

⸻

Fix your canonical hash now (or version it)

Right now you hash:

- node/edge counts as u32
- atom byte length as u32

That’s fine for today, but it’s an unforced footgun. I’d do a v2:

hasher.update(b"DIND_STATE_HASH_V2\0");
hasher.update(&(self.nodes.len() as u64).to_le_bytes());
// ...
hasher.update(&(atom.bytes.len() as u64).to_le_bytes());

If you keep V1 for compatibility, great — just add V2 and move forward.

⸻

One more important note: ID-index sections

Earlier I mentioned optional NODE_ID_INDEX / EDGE_ID_INDEX. With your IDs being BLAKE3 outputs, an all-zero key is theoretically possible (astronomically unlikely, but still). If you ever add open-addressing indexes-on-disk, don’t use “all zeros = empty slot” unless you also include an occupancy bitmap. For v1: skip the on-disk hash index and just binary search the sorted tables (fast enough and 100% clean).

⸻

If you want, I can now stitch this into the writer I gave you earlier so you can do:

let input = build_one_warp_input(&store, root_node_id);
let bytes = write_wsc_one_warp(input, schema_hash, tick)?;
std::fs::write("state.wsc", bytes)?;
let file = WscFile::open("state.wsc")?;
let view = file.warp_view(0)?;

…and then we add a “golden determinism” test that asserts the byte output is stable across permutations.

---

Awesome. Here’s the fully stitched path so you can go from your current GraphStore to a WSC file on disk and back to a zero-copy mmap view, plus tests that prove determinism.

I'm going to make one simplification that helps a lot:

- OneWarpInput should own its vectors (no lifetimes). That keeps the writer clean and avoids 'static hacks.

⸻

0) Minimal ident.rs upgrades (recommended)

Do this so all IDs are “byte clean” everywhere:

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub Hash);
impl NodeId { pub fn as_bytes(&self) -> &Hash { &self.0 } }

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EdgeId(pub Hash);
impl EdgeId { pub fn as_bytes(&self) -> &Hash { &self.0 } }

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TypeId(pub Hash);
impl TypeId { pub fn as_bytes(&self) -> &Hash { &self.0 } }

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WarpId(pub Hash);
impl WarpId { pub fn as_bytes(&self) -> &Hash { &self.0 } }

No behavior changes, just safer ABI expectations.

⸻

1) Add a wsc module (where it belongs)

Put this in warp-core (or whatever crate defines GraphStore/AttachmentValue), e.g.

crates/warp-core/src/wsc/
  mod.rs
  types.rs
  read.rs
  view.rs
  validate.rs
  write.rs
  build.rs

wsc/mod.rs:

pub mod types;
pub mod read;
pub mod view;
pub mod validate;
pub mod write;
pub mod build;

pub use view::{WscFile, WarpView, AttachmentRef};
pub use read::ReadError;


⸻

2) Make OneWarpInput owned (fix the writer interface)

In your wsc/write.rs, define:

use super::types::*;

pub struct OneWarpInput {
    pub warp_id: [u8; 32],
    pub root_node_id: [u8; 32],

    pub nodes: Vec<NodeRow>,
    pub edges: Vec<EdgeRow>,

    pub out_index: Vec<Range>,
    pub out_edges: Vec<OutEdgeRef>,

    pub node_atts_index: Vec<Range>,
    pub node_atts: Vec<AttRow>,

    pub edge_atts_index: Vec<Range>,
    pub edge_atts: Vec<AttRow>,

    pub blobs: Vec<u8>,
}

Then write_wsc_one_warp(input: OneWarpInput, ...) stays the same, just without lifetimes.

⸻

3) The bridge: build_one_warp_input(&GraphStore, root_node_id)

Put this in wsc/build.rs. This is the “money function” that turns your pointer-jungle into canonical slabs.

use std::collections::BTreeMap;

use crate::attachment::AttachmentValue;
use crate::graph::GraphStore;
use crate::ident::{EdgeId, NodeId};
use crate::record::EdgeRecord;

use super::types::{AttRow, EdgeRow, NodeRow, OutEdgeRef, Range};
use super::write::OneWarpInput;

pub fn build_one_warp_input(store: &GraphStore, root_node_id: NodeId) -> OneWarpInput {
    // NODES (already sorted)
    let nodes: Vec<(NodeId, crate::record::NodeRecord)> =
        store.nodes.iter().map(|(id, rec)| (*id, rec.clone())).collect();

    let node_rows: Vec<NodeRow> = nodes
        .iter()
        .map(|(id, rec)| NodeRow {
            node_id: id.0,
            node_type: rec.ty.0,
        })
        .collect();

    // EDGES (collect global, sort by EdgeId)
    let mut edges_all: Vec<EdgeRecord> = store.edges_from.values().flatten().cloned().collect();
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

    // EdgeId -> edge_ix
    let mut edge_ix: BTreeMap<EdgeId, u64> = BTreeMap::new();
    for (ix, e) in edges_all.iter().enumerate() {
        edge_ix.insert(e.id, ix as u64);
    }

    // OUT_INDEX / OUT_EDGES (node_ix order; bucket sorted by EdgeId)
    let mut out_index: Vec<Range> = Vec::with_capacity(node_rows.len());
    let mut out_edges: Vec<OutEdgeRef> = Vec::new();

    for (node_id, _) in &nodes {
        let start = out_edges.len() as u64;

        let mut bucket: Vec<&EdgeRecord> = store
            .edges_from
            .get(node_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default();

        bucket.sort_by_key(|e| e.id);

        for e in bucket {
            let ix = *edge_ix.get(&e.id).expect("edge_ix missing for edge");
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

    // Attachments + blobs
    let mut blobs: Vec<u8> = Vec::new();

    let mut node_atts_index: Vec<Range> = Vec::with_capacity(node_rows.len());
    let mut node_atts: Vec<AttRow> = Vec::new();

    for (node_id, _) in &nodes {
        let start = node_atts.len() as u64;

        if let Some(att) = store.node_attachments.get(node_id) {
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

        if let Some(att) = store.edge_attachments.get(&e.id) {
            edge_atts.push(att_to_row(att, &mut blobs));
        }

        let len = (edge_atts.len() as u64) - start;
        edge_atts_index.push(Range {
            start_le: start.to_le(),
            len_le: len.to_le(),
        });
    }

    OneWarpInput {
        warp_id: store.warp_id.0,
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

fn att_to_row(att: &AttachmentValue, blobs: &mut Vec<u8>) -> AttRow {
    match att {
        AttachmentValue::Atom(atom) => {
            align8(blobs);
            let off = blobs.len() as u64;
            let bytes: &[u8] = atom.bytes.as_ref();
            blobs.extend_from_slice(bytes);
            let len = bytes.len() as u64;

            AttRow {
                tag: 1,
                reserved0: [0u8; 7],
                type_or_warp: atom.type_id.0,
                blob_off_le: off.to_le(),
                blob_len_le: len.to_le(),
            }
        }
        AttachmentValue::Descend(warp_id) => AttRow {
            tag: 2,
            reserved0: [0u8; 7],
            type_or_warp: warp_id.0,
            blob_off_le: 0u64.to_le(),
            blob_len_le: 0u64.to_le(),
        },
    }
}

fn align8(v: &mut Vec<u8>) {
    while v.len() % 8 != 0 {
        v.push(0);
    }
}


⸻

4) One-liner convenience API

Add this helper (e.g. wsc/mod.rs or wsc/write.rs):

use std::io;
use std::path::Path;

use crate::graph::GraphStore;
use crate::ident::NodeId;

use super::build::build_one_warp_input;
use super::write::write_wsc_one_warp;

pub fn save_wsc(
    path: impl AsRef<Path>,
    store: &GraphStore,
    root_node_id: NodeId,
    schema_hash: [u8; 32],
    tick: u64,
) -> io::Result<()> {
    let input = build_one_warp_input(store, root_node_id);
    let bytes = write_wsc_one_warp(input, schema_hash, tick)?;
    std::fs::write(path, bytes)
}

Now you can call:

save_wsc("state.wsc", &store, root_node_id, schema_hash, tick)?;


⸻

5) Determinism tests (this is the “don’t lie to yourself” suite)

5.1 Snapshot bytes identical across insertion order

#[test]
fn wsc_bytes_stable_across_edge_insertion_order() {
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id};
    use crate::record::{EdgeRecord, NodeRecord};

    let node_ty = make_type_id("node");
    let edge_ty = make_type_id("edge");
    let warp = make_warp_id("root");

    let a = make_node_id("a");
    let b = make_node_id("b");
    let c = make_node_id("c");

    let e1 = make_edge_id("a->b");
    let e2 = make_edge_id("a->c");
    let e3 = make_edge_id("c->b");

    let mut s1 = crate::graph::GraphStore::new(warp);
    for n in [a,b,c] { s1.insert_node(n, NodeRecord { ty: node_ty }); }
    // insert edges in one order
    s1.insert_edge(a, EdgeRecord { id: e1, from: a, to: b, ty: edge_ty });
    s1.insert_edge(a, EdgeRecord { id: e2, from: a, to: c, ty: edge_ty });
    s1.insert_edge(c, EdgeRecord { id: e3, from: c, to: b, ty: edge_ty });

    let mut s2 = crate::graph::GraphStore::new(warp);
    for n in [a,b,c] { s2.insert_node(n, NodeRecord { ty: node_ty }); }
    // insert edges in a different order
    s2.insert_edge(c, EdgeRecord { id: e3, from: c, to: b, ty: edge_ty });
    s2.insert_edge(a, EdgeRecord { id: e2, from: a, to: c, ty: edge_ty });
    s2.insert_edge(a, EdgeRecord { id: e1, from: a, to: b, ty: edge_ty });

    let schema_hash = [0u8; 32];
    let tick = 0;

    let bytes1 = crate::wsc::write::write_wsc_one_warp(
        crate::wsc::build::build_one_warp_input(&s1, a),
        schema_hash,
        tick,
    ).unwrap();

    let bytes2 = crate::wsc::write::write_wsc_one_warp(
        crate::wsc::build::build_one_warp_input(&s2, a),
        schema_hash,
        tick,
    ).unwrap();

    assert_eq!(bytes1, bytes2, "WSC bytes must be canonical");
}

5.2 Roundtrip: reopen via mmap and validate invariants

This assumes you’ve wired up WscFile::open() + validate_wsc() (from earlier).

#[test]
fn wsc_roundtrip_open_and_validate() {
    use crate::ident::{make_node_id, make_type_id, make_warp_id};
    use crate::record::NodeRecord;

    let warp = make_warp_id("root");
    let node_ty = make_type_id("node");

    let a = make_node_id("a");
    let mut store = crate::graph::GraphStore::new(warp);
    store.insert_node(a, NodeRecord { ty: node_ty });

    let schema_hash = [0u8; 32];
    let bytes = crate::wsc::write::write_wsc_one_warp(
        crate::wsc::build::build_one_warp_input(&store, a),
        schema_hash,
        0,
    ).unwrap();

    let path = std::env::temp_dir().join("test_state.wsc");
    std::fs::write(&path, &bytes).unwrap();

    let file = crate::wsc::WscFile::open(&path).unwrap();
    let view = file.warp_view(0).unwrap();

    // Basic sanity
    assert_eq!(view.nodes().len(), 1);
    assert!(view.node_ix(&a.0).is_some());
}


⸻

6) (Strongly recommended) Version your canonical hash

Keep DIND_STATE_HASH_V1 but add V2 using u64 counts/lengths. That way your “snapshot hash” and “state hash” won’t diverge later for stupid reasons.
