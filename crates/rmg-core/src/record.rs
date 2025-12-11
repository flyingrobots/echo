// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Graph record types: nodes and edges.
use bytes::Bytes;

use crate::ident::{EdgeId, NodeId, TypeId};

/// Materialised record for a single node stored in the graph.
///
/// The optional `payload` carries domain-specific bytes (component data,
/// attachments, etc) and is interpreted by higher layers.
///
/// Invariants
/// - `ty` must be a valid type identifier in the current schema.
/// - The node identifier is not embedded here; the store supplies it externally.
/// - `payload` encoding is caller-defined and opaque to the store.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeRecord {
    /// Type identifier describing the node.
    pub ty: TypeId,
    /// Optional payload owned by the node (component data, attachments, etc.).
    pub payload: Option<Bytes>,
}

/// Materialised record for a single edge stored in the graph.
///
/// Invariants
/// - `from` and `to` reference existing nodes in the same store.
/// - `id` is stable across runs for the same logical edge.
/// - `ty` must be a valid edge type in the current schema.
/// - `payload` encoding is caller-defined and opaque to the store.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EdgeRecord {
    /// Stable identifier for the edge.
    pub id: EdgeId,
    /// Source node identifier.
    pub from: NodeId,
    /// Destination node identifier.
    pub to: NodeId,
    /// Type identifier describing the edge.
    pub ty: TypeId,
    /// Optional payload owned by the edge.
    pub payload: Option<Bytes>,
}
