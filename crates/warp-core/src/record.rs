// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Graph record types: nodes and edges.

use crate::ident::{EdgeId, NodeId, TypeId};

/// Materialised record for a single node stored in the graph.
///
/// Node records are **skeleton-plane only**: they describe structural identity
/// (currently: the node type) but do not carry attachment payloads.
///
/// Attachment-plane payloads are stored separately (see [`crate::AttachmentValue`])
/// and are addressed via [`crate::AttachmentKey`] / [`crate::SlotId`].
///
/// Invariants
/// - `ty` must be a valid type identifier in the current schema.
/// - The node identifier is not embedded here; the store supplies it externally.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeRecord {
    /// Type identifier describing the node.
    pub ty: TypeId,
}

/// Materialised record for a single edge stored in the graph.
///
/// Edge records are **skeleton-plane only**: they describe the structural link
/// between two nodes (from/to) and the link's type, but do not carry
/// attachment payloads.
///
/// Attachment-plane payloads for edges are stored separately (see
/// [`crate::AttachmentValue`]) and are addressed via [`crate::AttachmentKey`]
/// (using the edge's `id`).
///
/// Invariants
/// - `id` is stable across runs because it is derived via [`crate::make_edge_id`].
/// - `from` and `to` reference existing nodes in the same store.
/// - `ty` must be a valid edge type in the current schema.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeRecord {
    /// Stable identifier for the edge (see [`crate::make_edge_id`]).
    pub id: EdgeId,
    /// Source node identifier.
    pub from: NodeId,
    /// Destination node identifier.
    pub to: NodeId,
    /// Type identifier describing the edge.
    pub ty: TypeId,
}
