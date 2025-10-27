//! Snapshot type and hash computation.
//!
//! Determinism contract
//! - The snapshot hash is a BLAKE3 digest over a canonical byte stream that
//!   encodes the entire reachable graph state for the current root.
//! - Ordering is explicit and stable: nodes are visited in ascending `NodeId`
//!   order (lexicographic over 32-byte ids). For each node, outbound edges are
//!   sorted by ascending `EdgeId` before being encoded.
//! - Encoding is fixed-size and architecture-independent:
//!   - All ids (`NodeId`, `TypeId`, `EdgeId`) are raw 32-byte values.
//!   - Payloads are prefixed by an 8-byte little-endian length, followed by the
//!     exact payload bytes (or length `0` with no payload).
//! - The root id is included first to bind the subgraph identity.
//!
//! Notes
//! - Little-endian was chosen for length fields to match the rest of the code
//!   base; changing endianness would change hash values and must be treated as a
//!   breaking change. If we decide to adopt big-endian, update the encoding
//!   here and add a migration note in the determinism spec.
//! - The in-memory store uses `BTreeMap`, which guarantees deterministic key
//!   iteration. For vectors (edge lists), we sort explicitly by `EdgeId`.
use std::collections::{BTreeSet, VecDeque};

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
///
/// Algorithm
/// 1) Update with `root` id bytes.
/// 2) For each `(node_id, node)` in `store.nodes` (ascending by `node_id`):
///    - Update with `node_id`, `node.ty`.
///    - Update with 8-byte LE payload length, then payload bytes (if any).
/// 3) For each `(from, edges)` in `store.edges_from` (ascending by `from`):
///    - Update with `from` id and edge count (8-byte LE).
///    - Sort `edges` by `edge.id` ascending and for each edge:
///      - Update with `edge.id`, `edge.ty`, `edge.to`.
///      - Update with 8-byte LE payload length, then payload bytes (if any).
pub(crate) fn compute_snapshot_hash(store: &GraphStore, root: &NodeId) -> Hash {
    // 1) Determine reachable subgraph using a deterministic BFS over outgoing edges.
    let mut reachable: BTreeSet<NodeId> = BTreeSet::new();
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    reachable.insert(*root);
    queue.push_back(*root);
    while let Some(current) = queue.pop_front() {
        for edge in store.edges_from(&current) {
            if reachable.insert(edge.to) {
                queue.push_back(edge.to);
            }
        }
    }

    let mut hasher = Hasher::new();
    hasher.update(&root.0);

    // 2) Hash nodes in ascending NodeId order but only if reachable.
    for (node_id, node) in &store.nodes {
        if !reachable.contains(node_id) {
            continue;
        }
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

    // 3) Hash outgoing edges per reachable source, sorted by EdgeId, and only
    // include edges whose destination is also reachable.
    for (from, edges) in &store.edges_from {
        if !reachable.contains(from) {
            continue;
        }
        // Filter to reachable targets first; length counts included edges only.
        let mut sorted_edges: Vec<&EdgeRecord> =
            edges.iter().filter(|e| reachable.contains(&e.to)).collect();
        sorted_edges.sort_by(|a, b| a.id.0.cmp(&b.id.0));

        hasher.update(&from.0);
        hasher.update(&(sorted_edges.len() as u64).to_le_bytes());
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
