// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WarpSnapshot and WarpDiff builders for tests.

use echo_graph::{
    EdgeData, EdgeKind, EpochId, Hash32, NodeData, NodeKind, RenderEdge, RenderGraph, RenderNode,
    WarpDiff, WarpFrame, WarpOp, WarpSnapshot,
};

/// Builder for creating [`WarpSnapshot`] instances in tests.
///
/// # Example
///
/// ```
/// use echo_dry_tests::SnapshotBuilder;
///
/// let snap = SnapshotBuilder::new()
///     .epoch(5)
///     .with_node(1, vec![1, 2, 3])
///     .build();
///
/// assert_eq!(snap.epoch, 5);
/// assert_eq!(snap.graph.nodes.len(), 1);
/// ```
#[derive(Default)]
pub struct SnapshotBuilder {
    epoch: EpochId,
    graph: RenderGraph,
    state_hash: Option<Hash32>,
}

impl SnapshotBuilder {
    /// Create a new builder with defaults (epoch 0, empty graph).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the epoch ID.
    pub fn epoch(mut self, epoch: EpochId) -> Self {
        self.epoch = epoch;
        self
    }

    /// Set the full graph.
    pub fn graph(mut self, graph: RenderGraph) -> Self {
        self.graph = graph;
        self
    }

    /// Set the state hash.
    pub fn state_hash(mut self, hash: Hash32) -> Self {
        self.state_hash = Some(hash);
        self
    }

    /// Add a node with the given ID and raw data.
    pub fn with_node(mut self, id: u64, raw: Vec<u8>) -> Self {
        self.graph.nodes.push(RenderNode {
            id,
            kind: NodeKind::Generic,
            data: NodeData { raw },
        });
        self
    }

    /// Add an edge between two nodes.
    pub fn with_edge(mut self, id: u64, src: u64, dst: u64) -> Self {
        self.graph.edges.push(RenderEdge {
            id,
            src,
            dst,
            kind: EdgeKind::Generic,
            data: EdgeData { raw: Vec::new() },
        });
        self
    }

    /// Build the snapshot.
    pub fn build(self) -> WarpSnapshot {
        WarpSnapshot {
            epoch: self.epoch,
            graph: self.graph,
            state_hash: self.state_hash,
        }
    }

    /// Build and wrap in a WarpFrame::Snapshot.
    pub fn build_frame(self) -> WarpFrame {
        WarpFrame::Snapshot(self.build())
    }
}

/// Builder for creating [`WarpDiff`] instances in tests.
///
/// # Example
///
/// ```
/// use echo_dry_tests::DiffBuilder;
///
/// let diff = DiffBuilder::new(0, 1)
///     .with_add_node(1, vec![1, 2, 3])
///     .build();
///
/// assert_eq!(diff.from_epoch, 0);
/// assert_eq!(diff.to_epoch, 1);
/// assert_eq!(diff.ops.len(), 1);
/// ```
pub struct DiffBuilder {
    from_epoch: EpochId,
    to_epoch: EpochId,
    ops: Vec<WarpOp>,
    state_hash: Option<Hash32>,
}

impl DiffBuilder {
    /// Create a new builder for a diff from `from_epoch` to `to_epoch`.
    pub fn new(from_epoch: EpochId, to_epoch: EpochId) -> Self {
        Self {
            from_epoch,
            to_epoch,
            ops: Vec::new(),
            state_hash: None,
        }
    }

    /// Create a sequential diff (from_epoch to from_epoch + 1).
    pub fn sequential(from_epoch: EpochId) -> Self {
        Self::new(from_epoch, from_epoch + 1)
    }

    /// Set the state hash.
    pub fn state_hash(mut self, hash: Hash32) -> Self {
        self.state_hash = Some(hash);
        self
    }

    /// Add an AddNode operation.
    pub fn with_add_node(mut self, id: u64, raw: Vec<u8>) -> Self {
        self.ops.push(WarpOp::AddNode {
            id,
            kind: NodeKind::Generic,
            data: NodeData { raw },
        });
        self
    }

    /// Add a RemoveNode operation.
    pub fn with_remove_node(mut self, id: u64) -> Self {
        self.ops.push(WarpOp::RemoveNode { id });
        self
    }

    /// Add an UpdateNode operation.
    pub fn with_update_node(mut self, id: u64, raw: Vec<u8>) -> Self {
        self.ops.push(WarpOp::UpdateNode {
            id,
            data: echo_graph::NodeDataPatch::Replace(NodeData { raw }),
        });
        self
    }

    /// Add an AddEdge operation.
    pub fn with_add_edge(mut self, id: u64, src: u64, dst: u64) -> Self {
        self.ops.push(WarpOp::AddEdge {
            id,
            src,
            dst,
            kind: EdgeKind::Generic,
            data: EdgeData { raw: Vec::new() },
        });
        self
    }

    /// Add a RemoveEdge operation.
    pub fn with_remove_edge(mut self, id: u64) -> Self {
        self.ops.push(WarpOp::RemoveEdge { id });
        self
    }

    /// Add a raw WarpOp.
    pub fn with_op(mut self, op: WarpOp) -> Self {
        self.ops.push(op);
        self
    }

    /// Build the diff.
    pub fn build(self) -> WarpDiff {
        WarpDiff {
            from_epoch: self.from_epoch,
            to_epoch: self.to_epoch,
            ops: self.ops,
            state_hash: self.state_hash,
        }
    }

    /// Build and wrap in a WarpFrame::Diff.
    pub fn build_frame(self) -> WarpFrame {
        WarpFrame::Diff(self.build())
    }
}

/// Create an empty snapshot at epoch 0 (common test fixture).
pub fn empty_snapshot() -> WarpSnapshot {
    SnapshotBuilder::new().build()
}

/// Create an empty snapshot at the given epoch.
pub fn empty_snapshot_at(epoch: EpochId) -> WarpSnapshot {
    SnapshotBuilder::new().epoch(epoch).build()
}

/// Create an empty diff from one epoch to the next.
pub fn empty_diff(from: EpochId, to: EpochId) -> WarpDiff {
    DiffBuilder::new(from, to).build()
}

/// Create a sequence of empty sequential diffs.
pub fn sequential_empty_diffs(start: EpochId, count: usize) -> Vec<WarpDiff> {
    (0..count)
        .map(|i| empty_diff(start + i as u64, start + i as u64 + 1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_builder_defaults() {
        let snap = SnapshotBuilder::new().build();
        assert_eq!(snap.epoch, 0);
        assert!(snap.graph.nodes.is_empty());
        assert!(snap.graph.edges.is_empty());
        assert!(snap.state_hash.is_none());
    }

    #[test]
    fn snapshot_builder_with_nodes_and_edges() {
        let snap = SnapshotBuilder::new()
            .epoch(5)
            .with_node(1, vec![1, 2, 3])
            .with_node(2, vec![4, 5, 6])
            .with_edge(100, 1, 2)
            .build();

        assert_eq!(snap.epoch, 5);
        assert_eq!(snap.graph.nodes.len(), 2);
        assert_eq!(snap.graph.edges.len(), 1);
    }

    #[test]
    fn diff_builder_sequential() {
        let diff = DiffBuilder::sequential(5).build();
        assert_eq!(diff.from_epoch, 5);
        assert_eq!(diff.to_epoch, 6);
    }

    #[test]
    fn diff_builder_with_ops() {
        let diff = DiffBuilder::new(0, 1)
            .with_add_node(1, vec![])
            .with_remove_node(2)
            .build();

        assert_eq!(diff.ops.len(), 2);
    }

    #[test]
    fn sequential_empty_diffs_creates_correct_sequence() {
        let diffs = sequential_empty_diffs(5, 3);
        assert_eq!(diffs.len(), 3);
        assert_eq!(diffs[0].from_epoch, 5);
        assert_eq!(diffs[0].to_epoch, 6);
        assert_eq!(diffs[1].from_epoch, 6);
        assert_eq!(diffs[1].to_epoch, 7);
        assert_eq!(diffs[2].from_epoch, 7);
        assert_eq!(diffs[2].to_epoch, 8);
    }
}
