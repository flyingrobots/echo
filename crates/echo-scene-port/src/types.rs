// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Core scene types for the rendering port contract.
//!
//! These types are pure domain objects with no serialization logic.
//! CBOR encoding is handled by echo-scene-codec.

use alloc::string::String;
use alloc::vec::Vec;

/// 32-byte content-addressed key.
pub type Hash = [u8; 32];

/// Key type for nodes (32-byte hash).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeKey(pub Hash);

impl AsRef<Hash> for NodeKey {
    fn as_ref(&self) -> &Hash {
        &self.0
    }
}

/// Key type for edges (32-byte hash).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeKey(pub Hash);

impl AsRef<Hash> for EdgeKey {
    fn as_ref(&self) -> &Hash {
        &self.0
    }
}

/// Key type for labels (32-byte hash).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabelKey(pub Hash);

impl AsRef<Hash> for LabelKey {
    fn as_ref(&self) -> &Hash {
        &self.0
    }
}

/// Node shape for rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeShape {
    /// Spherical node shape.
    Sphere = 0,
    /// Cubic node shape.
    Cube = 1,
}

/// Edge rendering style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EdgeStyle {
    /// Solid line.
    Solid = 0,
    /// Dashed line.
    Dashed = 1,
}

/// RGBA color with 8-bit components.
pub type ColorRgba8 = [u8; 4];

/// Node definition for the scene graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeDef {
    /// Unique key for this node.
    pub key: NodeKey,
    /// Position in world space [x, y, z].
    pub position: [f32; 3],
    /// Radius of the node.
    pub radius: f32,
    /// Visual shape of the node.
    pub shape: NodeShape,
    /// RGBA color of the node.
    pub color: ColorRgba8,
}

/// Edge definition connecting two nodes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeDef {
    /// Unique key for this edge.
    pub key: EdgeKey,
    /// Source node key.
    pub a: NodeKey,
    /// Target node key.
    pub b: NodeKey,
    /// Line width.
    pub width: f32,
    /// Visual style of the edge.
    pub style: EdgeStyle,
    /// RGBA color of the edge.
    pub color: ColorRgba8,
}

/// Anchor point for labels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LabelAnchor {
    /// Anchored to a node.
    Node {
        /// The node key to anchor to.
        key: NodeKey,
    },
    /// Anchored to a world position.
    World {
        /// World position [x, y, z].
        position: [f32; 3],
    },
}

/// Label definition for text overlays.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelDef {
    /// Unique key for this label.
    pub key: LabelKey,
    /// Text content.
    pub text: String,
    /// Font size in world units.
    pub font_size: f32,
    /// RGBA color of the text.
    pub color: ColorRgba8,
    /// Anchor point for the label.
    pub anchor: LabelAnchor,
    /// Offset from anchor [x, y, z].
    pub offset: [f32; 3],
}

/// Scene operation (MVP set).
///
/// Operations are applied in order within a SceneDelta batch.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneOp {
    /// Insert or update a node.
    UpsertNode(NodeDef),
    /// Remove a node by key.
    RemoveNode {
        /// Key of the node to remove.
        key: NodeKey,
    },
    /// Insert or update an edge.
    UpsertEdge(EdgeDef),
    /// Remove an edge by key.
    RemoveEdge {
        /// Key of the edge to remove.
        key: EdgeKey,
    },
    /// Insert or update a label.
    UpsertLabel(LabelDef),
    /// Remove a label by key.
    RemoveLabel {
        /// Key of the label to remove.
        key: LabelKey,
    },
    /// Clear the entire scene.
    Clear,
}

/// Maximum number of operations allowed in a single SceneDelta.
///
/// This limit prevents malicious or runaway deltas from triggering
/// excessive memory allocations during decoding.
pub const MAX_OPS: usize = 10_000;

/// Scene delta: a batch of operations scoped to a cursor epoch.
///
/// Deltas are idempotent per (cursor_id, epoch) pair.
/// The number of operations is capped at [`MAX_OPS`].
#[derive(Clone, Debug, PartialEq)]
pub struct SceneDelta {
    /// Session identifier.
    pub session_id: Hash,
    /// Cursor identifier (enables parallel cursors).
    pub cursor_id: Hash,
    /// Epoch counter (monotonically increasing per cursor).
    pub epoch: u64,
    /// Operations to apply.
    pub ops: Vec<SceneOp>,
}
