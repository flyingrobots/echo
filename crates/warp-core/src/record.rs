// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Graph record types: nodes and edges.

use crate::attachment::AtomPayload;
use crate::ident::{EdgeId, NodeId, TypeId};

/// Materialised record for a single node stored in the graph.
///
/// The optional `payload` carries a **typed atom** for the attachment plane and
/// is interpreted by higher layers. The core store treats attachment payloads
/// as opaque and does not decode them during matching/indexing unless a rule
/// explicitly chooses to.
///
/// Invariants
/// - `ty` must be a valid type identifier in the current schema.
/// - The node identifier is not embedded here; the store supplies it externally.
/// - `payload` bytes are opaque to the store; any dependency that matters for
///   matching/slicing/causality must be represented as explicit skeleton
///   structure, not hidden inside payload bytes.
#[derive(Clone, Debug)]
pub struct NodeRecord {
    /// Type identifier describing the node.
    pub ty: TypeId,
    /// Optional attachment-plane payload owned by the node.
    pub payload: Option<AtomPayload>,
}

/// Materialised record for a single edge stored in the graph.
///
/// Invariants
/// - `from` and `to` reference existing nodes in the same store.
/// - `id` is stable across runs for the same logical edge.
/// - `ty` must be a valid edge type in the current schema.
/// - `payload` bytes are opaque to the store; any dependency that matters for
///   matching/slicing/causality must be represented as explicit skeleton
///   structure, not hidden inside payload bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeRecord {
    /// Stable identifier for the edge.
    pub id: EdgeId,
    /// Source node identifier.
    pub from: NodeId,
    /// Destination node identifier.
    pub to: NodeId,
    /// Type identifier describing the edge.
    pub ty: TypeId,
    /// Optional attachment-plane payload owned by the edge.
    pub payload: Option<AtomPayload>,
}
