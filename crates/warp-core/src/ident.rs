// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Identifier and hashing utilities.
use blake3::Hasher;

/// Canonical 256-bit hash used throughout the engine for addressing nodes,
/// types, snapshots, and rewrite rules.
pub type Hash = [u8; 32];

/// Strongly typed identifier for a node in the skeleton graph.
///
/// `NodeId` is an opaque 32-byte identifier (`Hash`). Many nodes in Echo use
/// stable, label-derived ids via [`make_node_id`] (`blake3("node:" || label)`),
/// but this is a convention, not a global constraint.
///
/// Other subsystems may construct content-addressed `NodeId`s derived from
/// different domain-separated hashes (for example, inbox/ledger event nodes
/// keyed by `intent_id = blake3("intent:" || intent_bytes)`).
///
/// Tooling must not assume that every `NodeId` corresponds to a human-readable
/// label, or that ids are reversible back into strings.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeId(pub Hash);

impl NodeId {
    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Strongly typed identifier for the logical kind of a node or component.
///
/// `TypeId` values are produced by [`make_type_id`] which hashes a label; using
/// a dedicated wrapper prevents accidental mixing of node and type identifiers.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TypeId(pub Hash);

impl TypeId {
    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Identifier for a directed edge within the graph.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeId(pub Hash);

impl EdgeId {
    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Strongly typed identifier for a WARP instance.
///
/// A `WarpId` namespaces node/edge ids for Stage B1 "flattened indirection"
/// descended attachments: nodes and edges live in instance-scoped graphs
/// addressed by `(warp_id, local_id)`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WarpId(pub Hash);

impl WarpId {
    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Produces a stable, domain‑separated type identifier (prefix `b"type:"`) using BLAKE3.
pub fn make_type_id(label: &str) -> TypeId {
    let mut hasher = Hasher::new();
    hasher.update(b"type:");
    hasher.update(label.as_bytes());
    TypeId(hasher.finalize().into())
}

/// Produces a stable, domain‑separated node identifier (prefix `b"node:"`) using BLAKE3.
pub fn make_node_id(label: &str) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(b"node:");
    hasher.update(label.as_bytes());
    NodeId(hasher.finalize().into())
}

/// Compact, process-local rule identifier used on hot paths.
///
/// The engine maps canonical 256-bit rule ids (family ids) to compact u32
/// handles at registration time. These handles are never serialized; they are
/// purely an in-process acceleration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompactRuleId(pub u32);

/// Produces a stable, domain‑separated edge identifier (prefix `b"edge:"`) using BLAKE3.
pub fn make_edge_id(label: &str) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(b"edge:");
    hasher.update(label.as_bytes());
    EdgeId(hasher.finalize().into())
}

/// Produces a stable, domain-separated warp identifier (prefix `b"warp:"`) using BLAKE3.
pub fn make_warp_id(label: &str) -> WarpId {
    let mut hasher = Hasher::new();
    hasher.update(b"warp:");
    hasher.update(label.as_bytes());
    WarpId(hasher.finalize().into())
}

/// Instance-scoped identifier for a node.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeKey {
    /// Warp instance that namespaces the local node id.
    pub warp_id: WarpId,
    /// Local node identifier within the instance.
    pub local_id: NodeId,
}

/// Instance-scoped identifier for an edge.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeKey {
    /// Warp instance that namespaces the local edge id.
    pub warp_id: WarpId,
    /// Local edge identifier within the instance.
    pub local_id: EdgeId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_separation_prevents_cross_type_collisions() {
        let lbl = "foo";
        let t = make_type_id(lbl).0;
        let n = make_node_id(lbl).0;
        let e = make_edge_id(lbl).0;
        let w = make_warp_id(lbl).0;
        assert_ne!(t, n);
        assert_ne!(t, e);
        assert_ne!(t, w);
        assert_ne!(n, e);
        assert_ne!(n, w);
        assert_ne!(e, w);
    }
}
