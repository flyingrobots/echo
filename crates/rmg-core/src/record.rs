//! Graph record types: nodes and edges.
use bytes::Bytes;

use crate::ident::{EdgeId, NodeId, TypeId};

/// Materialised record for a single node stored in the graph.
///
/// The optional `payload` carries domain-specific bytes (component data,
/// attachments, etc) and is interpreted by higher layers.
#[derive(Clone, Debug)]
pub struct NodeRecord {
    /// Type identifier describing the node.
    pub ty: TypeId,
    /// Optional payload owned by the node (component data, attachments, etc.).
    pub payload: Option<Bytes>,
}

/// Materialised record for a single edge stored in the graph.
#[derive(Clone, Debug)]
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

