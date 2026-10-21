// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `ContentAddressV1`: the identities Echo admits for authored graph effects.
//!
//! This module is the single implementation of identity law `1`. Effect
//! admission recomputes every id a proposal creates with it, and authored
//! programs will compute the same ids through Echo's pure identity intrinsics.
//! Every value below is part of the law: changing any of them is a new law.
//!
//! Every hash is BLAKE3 over the listed fields, concatenated with no separators.
//! Ids, warps and type ids are their 32 raw bytes; lengths and counts are
//! unsigned 64-bit little-endian.
//!
//! - **Attachment digest** of an atom: [`ATTACHMENT_VALUE_DOMAIN`] (with its NUL
//!   terminator), the tag byte [`ATOM_ATTACHMENT_TAG`], the atom type id, the
//!   atom's byte length, then the atom bytes.
//! - **Content node id:** [`NODE_ID_DOMAIN`], the warp id, the node type id, the
//!   attachment digest of its atom, the number of outgoing edges, then for each
//!   outgoing edge in ascending byte order of (edge type id, target node id)
//!   the edge type id followed by the target node id.
//! - **Content edge id** (an edge whose source is a content node):
//!   [`CONTENT_EDGE_ID_DOMAIN`], the warp id, the source node id, the edge type
//!   id, the target node id.
//! - **Slot edge id** (an edge whose source is a mutable node):
//!   [`SLOT_EDGE_ID_DOMAIN`], the warp id, the source node id, the edge type id.
//!   The target is not part of it, so retargeting the slot keeps its id.
//!
//! **Duplicate outgoing edges.** A content node's outgoing edges are a set:
//! two edges with the same (edge type, target) from one content node would
//! have the same content edge id, which admission refuses as a duplicate
//! effect target. [`content_node_id`] therefore refuses a duplicate tuple
//! instead of hashing or silently dropping it, so it never names an edge set
//! admission could not accept. One edge type to several targets, or several
//! edge types to one target, are distinct edges.
//!
//! The law has no edge-count ceiling. Authoring interfaces may impose their
//! own; this module does not.
use crate::{EdgeId, Hash, NodeId, TypeId, WarpId};

/// Domain of an attachment value digest.
pub const ATTACHMENT_VALUE_DOMAIN: &[u8] = b"echo.bridge.attachment-value.v1\0";
/// Attachment tag of an atom value.
pub const ATOM_ATTACHMENT_TAG: u8 = 0;
/// Attachment tag of a descended-warp value, used by read evidence.
pub const DESCEND_ATTACHMENT_TAG: u8 = 1;
/// Domain of a content node id.
pub const NODE_ID_DOMAIN: &[u8] = b"echo.bridge.node-id.v1\0";
/// Domain of an edge id whose source is a content node.
pub const CONTENT_EDGE_ID_DOMAIN: &[u8] = b"echo.bridge.edge-id.v1\0";
/// Domain of an edge id whose source is a mutable node.
pub const SLOT_EDGE_ID_DOMAIN: &[u8] = b"echo.bridge.slot-edge-id.v1\0";

/// One outgoing edge of a content node, as it enters the node's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutgoingEdge {
    /// The edge's type id.
    pub edge_type: TypeId,
    /// The edge's target node id.
    pub to: NodeId,
}

/// Inputs that name no content node admission could accept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentAddressError {
    /// The same (edge type, target) appears more than once.
    DuplicateOutgoingEdge {
        /// The repeated edge type id.
        edge_type: TypeId,
        /// The repeated target node id.
        to: NodeId,
    },
}

/// The attachment value digest of an atom.
#[must_use]
pub fn attachment_digest(atom_type: TypeId, atom_bytes: &[u8]) -> Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(ATTACHMENT_VALUE_DOMAIN);
    hash.update(&[ATOM_ATTACHMENT_TAG]);
    hash.update(&atom_type.0);
    hash.update(&(atom_bytes.len() as u64).to_le_bytes());
    hash.update(atom_bytes);
    hash.finalize().into()
}

/// The attachment value digest of a descended warp, used by read evidence.
#[must_use]
pub fn descend_attachment_digest(warp: WarpId) -> Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(ATTACHMENT_VALUE_DOMAIN);
    hash.update(&[DESCEND_ATTACHMENT_TAG]);
    hash.update(&warp.0);
    hash.finalize().into()
}

/// The content address of a node with this atom and complete outgoing edge
/// set, in any order.
///
/// # Errors
///
/// [`ContentAddressError::DuplicateOutgoingEdge`] when two edges share an
/// (edge type, target).
pub fn content_node_id(
    warp: WarpId,
    node_type: TypeId,
    atom_type: TypeId,
    atom_bytes: &[u8],
    outgoing: &[OutgoingEdge],
) -> Result<NodeId, ContentAddressError> {
    // A strictly ascending input is already canonical and duplicate-free, so
    // it is hashed in place; only an unordered input is copied and sorted.
    let sorted;
    let edges: &[OutgoingEdge] = if outgoing.windows(2).all(|pair| pair[0] < pair[1]) {
        outgoing
    } else {
        let mut copy = outgoing.to_vec();
        copy.sort_unstable();
        if let Some(pair) = copy.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(ContentAddressError::DuplicateOutgoingEdge {
                edge_type: pair[0].edge_type,
                to: pair[0].to,
            });
        }
        sorted = copy;
        &sorted
    };
    let mut hash = blake3::Hasher::new();
    hash.update(NODE_ID_DOMAIN);
    hash.update(&warp.0);
    hash.update(&node_type.0);
    hash.update(&attachment_digest(atom_type, atom_bytes));
    hash.update(&(edges.len() as u64).to_le_bytes());
    for edge in edges {
        hash.update(&edge.edge_type.0);
        hash.update(&edge.to.0);
    }
    Ok(NodeId(hash.finalize().into()))
}

/// The id of an edge whose source is a content node.
#[must_use]
pub fn content_edge_id(warp: WarpId, from: NodeId, edge_type: TypeId, to: NodeId) -> EdgeId {
    let mut hash = blake3::Hasher::new();
    hash.update(CONTENT_EDGE_ID_DOMAIN);
    hash.update(&warp.0);
    hash.update(&from.0);
    hash.update(&edge_type.0);
    hash.update(&to.0);
    EdgeId(hash.finalize().into())
}

/// The id of an edge whose source is a mutable node; it names the slot, not
/// the target.
#[must_use]
pub fn slot_edge_id(warp: WarpId, from: NodeId, edge_type: TypeId) -> EdgeId {
    let mut hash = blake3::Hasher::new();
    hash.update(SLOT_EDGE_ID_DOMAIN);
    hash.update(&warp.0);
    hash.update(&from.0);
    hash.update(&edge_type.0);
    EdgeId(hash.finalize().into())
}
