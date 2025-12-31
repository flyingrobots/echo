// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical renderable graph representation shared across Echo tools.
//! Pure data (nodes, edges, payloads) with deterministic hashing/serialization.

use blake3::Hash;
use ciborium::ser::into_writer;
use serde::{Deserialize, Serialize};

/// Monotonic epoch identifier.
pub type EpochId = u64;
/// Blake3 (or equivalent) state hash (32 bytes).
pub type Hash32 = [u8; 32];
/// Canonical node identifier.
pub type NodeId = u64;
/// Canonical edge identifier.
pub type EdgeId = u64;
/// Identifier for a WARP authority/stream.
pub type WarpId = u64;

/// Basic node classification (extend as needed).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeKind {
    /// Unspecified node type.
    Generic,
}

/// Basic edge classification (extend as needed).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdgeKind {
    /// Unspecified edge type.
    Generic,
}

/// Opaque node payload (viewer/engine agree on encoding).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeData {
    /// Opaque payload bytes.
    pub raw: Vec<u8>,
}

/// Opaque edge payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdgeData {
    /// Opaque payload bytes.
    pub raw: Vec<u8>,
}

/// Patch semantics for nodes (start with replace).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeDataPatch {
    /// Replace the entire node payload.
    Replace(NodeData),
}

/// Patch semantics for edges (start with replace).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeDataPatch {
    /// Replace the entire edge payload.
    Replace(EdgeData),
}

/// Structural graph mutations used in diffs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WarpOp {
    /// Create a node.
    AddNode {
        /// Node identifier.
        id: NodeId,
        /// Node classification.
        kind: NodeKind,
        /// Node payload.
        data: NodeData,
    },
    /// Remove a node (incident edges removed implicitly).
    RemoveNode {
        /// Node identifier.
        id: NodeId,
    },
    /// Update node payload.
    UpdateNode {
        /// Node identifier.
        id: NodeId,
        /// Payload patch.
        data: NodeDataPatch,
    },

    /// Create an edge.
    AddEdge {
        /// Edge identifier.
        id: EdgeId,
        /// Source node id.
        src: NodeId,
        /// Destination node id.
        dst: NodeId,
        /// Edge classification.
        kind: EdgeKind,
        /// Edge payload.
        data: EdgeData,
    },
    /// Remove an edge.
    RemoveEdge {
        /// Edge identifier.
        id: EdgeId,
    },
    /// Update edge payload.
    UpdateEdge {
        /// Edge identifier.
        id: EdgeId,
        /// Payload patch.
        data: EdgeDataPatch,
    },
}

/// Renderable node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderNode {
    /// Node identifier.
    pub id: NodeId,
    /// Node classification.
    pub kind: NodeKind,
    /// Node payload.
    pub data: NodeData,
}

/// Renderable edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderEdge {
    /// Edge identifier.
    pub id: EdgeId,
    /// Source node.
    pub src: NodeId,
    /// Destination node.
    pub dst: NodeId,
    /// Edge classification.
    pub kind: EdgeKind,
    /// Edge payload.
    pub data: EdgeData,
}

/// Renderable graph used in snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RenderGraph {
    /// All nodes in the graph.
    pub nodes: Vec<RenderNode>,
    /// All edges in the graph.
    pub edges: Vec<RenderEdge>,
}

impl RenderGraph {
    /// Canonical serialization (sorted by id) for hashing/comparison.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
        #[derive(Serialize)]
        struct Canon<'a> {
            nodes: Vec<&'a RenderNode>,
            edges: Vec<&'a RenderEdge>,
        }

        let mut node_idx: Vec<usize> = (0..self.nodes.len()).collect();
        node_idx.sort_by_key(|&i| self.nodes[i].id);

        let mut edge_idx: Vec<usize> = (0..self.edges.len()).collect();
        edge_idx.sort_by_key(|&i| {
            let e = &self.edges[i];
            (e.src, e.dst, e.id)
        });

        let canon = Canon {
            nodes: node_idx.iter().map(|&i| &self.nodes[i]).collect(),
            edges: edge_idx.iter().map(|&i| &self.edges[i]).collect(),
        };

        let mut bytes = Vec::new();
        into_writer(&canon, &mut bytes)?;
        Ok(bytes)
    }

    /// Compute blake3 hash of the canonical form.
    pub fn compute_hash(&self) -> Result<Hash32, ciborium::ser::Error<std::io::Error>> {
        let h: Hash = blake3::hash(&self.to_canonical_bytes()?);
        Ok(h.into())
    }

    /// Apply a structural op; errors if ids are missing/duplicate.
    pub fn apply_op(&mut self, op: WarpOp) -> anyhow::Result<()> {
        match op {
            WarpOp::AddNode { id, kind, data } => {
                if self.nodes.iter().any(|n| n.id == id) {
                    anyhow::bail!("node already exists: {}", id);
                }
                self.nodes.push(RenderNode { id, kind, data });
            }
            WarpOp::RemoveNode { id } => {
                let before = self.nodes.len();
                self.nodes.retain(|n| n.id != id);
                if self.nodes.len() == before {
                    anyhow::bail!("missing node: {}", id);
                }
                self.edges.retain(|e| e.src != id && e.dst != id);
            }
            WarpOp::UpdateNode { id, data } => {
                let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) else {
                    anyhow::bail!("missing node: {}", id);
                };
                match data {
                    NodeDataPatch::Replace(nd) => node.data = nd,
                }
            }
            WarpOp::AddEdge {
                id,
                src,
                dst,
                kind,
                data,
            } => {
                if self.edges.iter().any(|e| e.id == id) {
                    anyhow::bail!("edge already exists: {}", id);
                }
                if !self.nodes.iter().any(|n| n.id == src) {
                    anyhow::bail!("missing src node: {}", src);
                }
                if !self.nodes.iter().any(|n| n.id == dst) {
                    anyhow::bail!("missing dst node: {}", dst);
                }
                self.edges.push(RenderEdge {
                    id,
                    src,
                    dst,
                    kind,
                    data,
                });
            }
            WarpOp::RemoveEdge { id } => {
                let before = self.edges.len();
                self.edges.retain(|e| e.id != id);
                if self.edges.len() == before {
                    anyhow::bail!("missing edge: {}", id);
                }
            }
            WarpOp::UpdateEdge { id, data } => {
                let Some(edge) = self.edges.iter_mut().find(|e| e.id == id) else {
                    anyhow::bail!("missing edge: {}", id);
                };
                match data {
                    EdgeDataPatch::Replace(ed) => edge.data = ed,
                }
            }
        }
        Ok(())
    }
}

/// Full snapshot of an epoch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarpSnapshot {
    /// Epoch identifier for this snapshot.
    pub epoch: EpochId,
    /// Full renderable graph at this epoch.
    pub graph: RenderGraph,
    /// Optional hash of the canonical graph.
    pub state_hash: Option<Hash32>,
}

/// Diff between consecutive epochs (must be gapless in live streams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarpDiff {
    /// Base epoch (pre-diff).
    pub from_epoch: EpochId,
    /// Target epoch (post-diff, expected = from_epoch + 1 in live streams).
    pub to_epoch: EpochId,
    /// Structural operations to apply.
    pub ops: Vec<WarpOp>,
    /// Optional hash of the post-state (epoch = to_epoch).
    pub state_hash: Option<Hash32>,
}

/// Wire frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WarpFrame {
    /// Full state snapshot for an epoch.
    Snapshot(WarpSnapshot),
    /// Gapless diff between consecutive epochs.
    Diff(WarpDiff),
}

/// Viewer→Engine hello for late join/reconnect.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarpHello {
    /// Viewer’s last known epoch (if any).
    pub last_known_epoch: Option<EpochId>,
    /// Hash of viewer’s last known epoch (if any).
    pub last_known_hash: Option<Hash32>,
    /// Protocol version for compatibility.
    pub protocol_version: u16,
}
