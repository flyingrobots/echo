//! Minimal in-memory graph store used by the rewrite executor and tests.
use std::collections::BTreeMap;

use crate::ident::NodeId;
use crate::record::{EdgeRecord, NodeRecord};

/// In-memory graph storage for the spike.
///
/// The production engine will eventually swap in a content-addressed store,
/// but this structure keeps the motion rewrite spike self-contained.
#[derive(Default, Clone)]
pub struct GraphStore {
    /// Mapping from node identifiers to their materialised records.
    pub(crate) nodes: BTreeMap<NodeId, NodeRecord>,
    /// Mapping from source node to outbound edge records.
    pub(crate) edges_from: BTreeMap<NodeId, Vec<EdgeRecord>>,
}

impl GraphStore {
    /// Returns a shared reference to a node when it exists.
    pub fn node(&self, id: &NodeId) -> Option<&NodeRecord> {
        self.nodes.get(id)
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

    /// Inserts or replaces a node in the store.
    pub fn insert_node(&mut self, id: NodeId, record: NodeRecord) {
        self.nodes.insert(id, record);
    }

    /// Inserts a directed edge into the store in insertion order.
    ///
    /// Ordering note: The underlying vector preserves insertion order. When
    /// deterministic ordering is required (e.g., snapshot hashing), callers
    /// must sort by `EdgeId` explicitly.
    pub fn insert_edge(&mut self, from: NodeId, edge: EdgeRecord) {
        self.edges_from.entry(from).or_default().push(edge);
    }
}
