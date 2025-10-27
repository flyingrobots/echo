//! Snapshot type and hash computation.
use blake3::Hasher;

use crate::graph::GraphStore;
use crate::ident::{Hash, NodeId};
use crate::record::EdgeRecord;
use crate::tx::TxId;

/// Snapshot returned after a successful commit.
///
/// The `hash` value is deterministic and reflects the entire canonicalised
/// graph state (root + payloads).
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Node identifier that serves as the root of the snapshot.
    pub root: NodeId,
    /// Canonical hash derived from the entire graph state.
    pub hash: Hash,
    /// Optional parent snapshot hash (if one exists).
    pub parent: Option<Hash>,
    /// Transaction identifier associated with the snapshot.
    pub tx: TxId,
}

/// Computes a canonical hash for the current graph state.
pub(crate) fn compute_snapshot_hash(store: &GraphStore, root: &NodeId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(&root.0);
    for (node_id, node) in &store.nodes {
        hasher.update(&node_id.0);
        hasher.update(&(node.ty).0);
        match &node.payload {
            Some(payload) => {
                hasher.update(&(payload.len() as u64).to_le_bytes());
                hasher.update(payload);
            }
            None => {
                hasher.update(&0u64.to_le_bytes());
            }
        }
    }
    for (from, edges) in &store.edges_from {
        hasher.update(&from.0);
        hasher.update(&(edges.len() as u64).to_le_bytes());
        let mut sorted_edges: Vec<&EdgeRecord> = edges.iter().collect();
        sorted_edges.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        for edge in sorted_edges {
            hasher.update(&(edge.id).0);
            hasher.update(&(edge.ty).0);
            hasher.update(&(edge.to).0);
            match &edge.payload {
                Some(payload) => {
                    hasher.update(&(payload.len() as u64).to_le_bytes());
                    hasher.update(payload);
                }
                None => {
                    hasher.update(&0u64.to_le_bytes());
                }
            }
        }
    }
    hasher.finalize().into()
}
