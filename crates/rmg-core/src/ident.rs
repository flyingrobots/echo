//! Identifier and hashing utilities.
use blake3::Hasher;

/// Canonical 256-bit hash used throughout the engine for addressing nodes,
/// types, snapshots, and rewrite rules.
pub type Hash = [u8; 32];

/// Strongly typed identifier for a registered entity or structural node.
///
/// `NodeId` values are obtained from [`make_node_id`] and remain stable across
/// runs because they are derived from a BLAKE3 hash of a string label.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub Hash);

/// Strongly typed identifier for the logical kind of a node or component.
///
/// `TypeId` values are produced by [`make_type_id`] which hashes a label; using
/// a dedicated wrapper prevents accidental mixing of node and type identifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeId(pub Hash);

/// Identifier for a directed edge within the graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EdgeId(pub Hash);

/// Produces a stable type identifier derived from a label using BLAKE3.
pub fn make_type_id(label: &str) -> TypeId {
    let mut hasher = Hasher::new();
    hasher.update(label.as_bytes());
    TypeId(hasher.finalize().into())
}

/// Produces a stable node identifier derived from a label using BLAKE3.
pub fn make_node_id(label: &str) -> NodeId {
    let mut hasher = Hasher::new();
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

/// Produces a stable edge identifier derived from a label using BLAKE3.
#[allow(dead_code)]
pub fn make_edge_id(label: &str) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(label.as_bytes());
    EdgeId(hasher.finalize().into())
}
