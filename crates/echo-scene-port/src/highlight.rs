// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Highlight state types for selection and hover feedback.

use crate::types::{EdgeKey, NodeKey};
use alloc::vec::Vec;

/// Highlight state for selection/hover feedback.
///
/// Renderers use this to apply visual emphasis (glow, outline, etc.)
/// to selected or hovered elements.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightState {
    /// Currently selected node keys.
    pub selected_nodes: Vec<NodeKey>,
    /// Currently selected edge keys.
    pub selected_edges: Vec<EdgeKey>,
    /// Currently hovered node (if any).
    pub hovered_node: Option<NodeKey>,
    /// Currently hovered edge (if any).
    pub hovered_edge: Option<EdgeKey>,
}
