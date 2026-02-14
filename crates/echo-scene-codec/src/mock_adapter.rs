// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Mock adapter for headless testing of ScenePort implementations.
//!
//! MockAdapter tracks scene state in HashMaps without any GPU rendering.
//! Use it to verify delta application logic and epoch semantics.

use std::collections::HashMap;

use echo_scene_port::{
    CameraState, EdgeDef, HighlightState, LabelAnchor, LabelDef, NodeDef, SceneDelta, SceneOp,
    ScenePort,
};

/// Convert a 32-byte hash to a hex string for use as HashMap key.
fn hash_to_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Mock scene adapter for testing.
///
/// Implements `ScenePort` by tracking state in HashMaps.
/// Useful for verifying delta sequences without GPU rendering.
#[derive(Debug, Default)]
pub struct MockAdapter {
    /// Current nodes in the scene.
    pub nodes: HashMap<String, NodeDef>,
    /// Current edges in the scene.
    pub edges: HashMap<String, EdgeDef>,
    /// Current labels in the scene.
    pub labels: HashMap<String, LabelDef>,
    /// Current camera state.
    pub camera: CameraState,
    /// Current highlight state.
    pub highlight: HighlightState,
    /// Last epoch processed per cursor (hex string key).
    last_epoch_by_cursor: HashMap<String, u64>,
    /// Number of render calls.
    pub render_count: u32,
    /// Current viewport dimensions.
    pub viewport: (u32, u32, f32),
    /// Whether dispose has been called.
    pub disposed: bool,
}

impl MockAdapter {
    /// Create a new mock adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of nodes in the scene.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the scene.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get the number of labels in the scene.
    pub fn label_count(&self) -> usize {
        self.labels.len()
    }

    /// Check if an edge is valid (both endpoints exist).
    pub fn is_edge_valid(&self, edge_key: &str) -> bool {
        if let Some(edge) = self.edges.get(edge_key) {
            let a_hex = hash_to_hex(&edge.a);
            let b_hex = hash_to_hex(&edge.b);
            self.nodes.contains_key(&a_hex) && self.nodes.contains_key(&b_hex)
        } else {
            false
        }
    }

    /// Get node by hex key.
    pub fn get_node(&self, key_hex: &str) -> Option<&NodeDef> {
        self.nodes.get(key_hex)
    }

    /// Get edge by hex key.
    pub fn get_edge(&self, key_hex: &str) -> Option<&EdgeDef> {
        self.edges.get(key_hex)
    }

    /// Get label by hex key.
    pub fn get_label(&self, key_hex: &str) -> Option<&LabelDef> {
        self.labels.get(key_hex)
    }

    /// Get the last epoch for a cursor (by hex key).
    pub fn last_epoch(&self, cursor_hex: &str) -> Option<u64> {
        self.last_epoch_by_cursor.get(cursor_hex).copied()
    }
}

impl ScenePort for MockAdapter {
    fn apply_scene_delta(&mut self, delta: &SceneDelta) {
        let cursor_hex = hash_to_hex(&delta.cursor_id);
        let last_epoch = self.last_epoch_by_cursor.get(&cursor_hex).copied();

        // Idempotency check: skip if epoch already processed
        if let Some(last) = last_epoch {
            if delta.epoch <= last {
                return;
            }
        }

        // Apply operations in order
        for op in &delta.ops {
            match op {
                SceneOp::UpsertNode(node) => {
                    let key_hex = hash_to_hex(&node.key);
                    self.nodes.insert(key_hex, node.clone());
                }
                SceneOp::RemoveNode { key } => {
                    let key_hex = hash_to_hex(key);
                    self.nodes.remove(&key_hex);
                    // Remove labels anchored to this node
                    self.labels.retain(|_, label| {
                        if let LabelAnchor::Node { key: anchor_key } = &label.anchor {
                            hash_to_hex(anchor_key) != key_hex
                        } else {
                            true
                        }
                    });
                }
                SceneOp::UpsertEdge(edge) => {
                    let key_hex = hash_to_hex(&edge.key);
                    self.edges.insert(key_hex, edge.clone());
                }
                SceneOp::RemoveEdge { key } => {
                    let key_hex = hash_to_hex(key);
                    self.edges.remove(&key_hex);
                }
                SceneOp::UpsertLabel(label) => {
                    let key_hex = hash_to_hex(&label.key);
                    self.labels.insert(key_hex, label.clone());
                }
                SceneOp::RemoveLabel { key } => {
                    let key_hex = hash_to_hex(key);
                    self.labels.remove(&key_hex);
                }
                SceneOp::Clear => {
                    self.nodes.clear();
                    self.edges.clear();
                    self.labels.clear();
                }
            }
        }

        // Update epoch tracking
        self.last_epoch_by_cursor.insert(cursor_hex, delta.epoch);
    }

    fn set_camera(&mut self, camera: &CameraState) {
        self.camera = camera.clone();
    }

    fn set_highlight(&mut self, highlight: &HighlightState) {
        self.highlight = highlight.clone();
    }

    fn render(&mut self) {
        self.render_count += 1;
    }

    fn resize(&mut self, width: u32, height: u32, dpr: f32) {
        self.viewport = (width, height, dpr);
    }

    fn reset_cursor(&mut self, cursor_id: &[u8; 32]) {
        let cursor_hex = hash_to_hex(cursor_id);
        self.last_epoch_by_cursor.remove(&cursor_hex);
    }

    fn dispose(&mut self) {
        self.disposed = true;
        self.nodes.clear();
        self.edges.clear();
        self.labels.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use echo_scene_port::{EdgeStyle, NodeShape, ProjectionKind};

    fn make_test_hash(seed: u8) -> [u8; 32] {
        let mut hash = [0u8; 32];
        for (i, byte) in hash.iter_mut().enumerate() {
            *byte = seed.wrapping_add(i as u8);
        }
        hash
    }

    fn make_node(seed: u8, pos: [f32; 3]) -> NodeDef {
        NodeDef {
            key: make_test_hash(seed),
            position: pos,
            radius: 1.0,
            shape: NodeShape::Sphere,
            color: [255, 255, 255, 255],
        }
    }

    fn make_edge(seed: u8, a_seed: u8, b_seed: u8) -> EdgeDef {
        EdgeDef {
            key: make_test_hash(seed),
            a: make_test_hash(a_seed),
            b: make_test_hash(b_seed),
            width: 0.1,
            style: EdgeStyle::Solid,
            color: [255, 255, 255, 255],
        }
    }

    fn make_label(seed: u8, anchor_seed: u8, text: &str) -> LabelDef {
        LabelDef {
            key: make_test_hash(seed),
            text: text.into(),
            font_size: 12.0,
            color: [255, 255, 255, 255],
            anchor: LabelAnchor::Node {
                key: make_test_hash(anchor_seed),
            },
            offset: [0.0, 0.5, 0.0],
        }
    }

    #[test]
    fn test_upsert_node() {
        let mut adapter = MockAdapter::new();
        let delta = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 0,
            ops: vec![SceneOp::UpsertNode(make_node(10, [1.0, 2.0, 3.0]))],
        };
        adapter.apply_scene_delta(&delta);
        assert_eq!(adapter.node_count(), 1);

        let key_hex = hash_to_hex(&make_test_hash(10));
        let node = adapter.get_node(&key_hex).unwrap();
        assert_eq!(node.position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_remove_node_removes_anchored_labels() {
        let mut adapter = MockAdapter::new();

        // Add node and label
        let delta1 = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 0,
            ops: vec![
                SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0])),
                SceneOp::UpsertLabel(make_label(20, 10, "Node Label")),
            ],
        };
        adapter.apply_scene_delta(&delta1);
        assert_eq!(adapter.node_count(), 1);
        assert_eq!(adapter.label_count(), 1);

        // Remove node
        let delta2 = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 1,
            ops: vec![SceneOp::RemoveNode {
                key: make_test_hash(10),
            }],
        };
        adapter.apply_scene_delta(&delta2);
        assert_eq!(adapter.node_count(), 0);
        assert_eq!(adapter.label_count(), 0); // Label should be removed too
    }

    #[test]
    fn test_upsert_edge() {
        let mut adapter = MockAdapter::new();

        // Add two nodes and an edge
        let delta = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 0,
            ops: vec![
                SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0])),
                SceneOp::UpsertNode(make_node(11, [1.0, 0.0, 0.0])),
                SceneOp::UpsertEdge(make_edge(20, 10, 11)),
            ],
        };
        adapter.apply_scene_delta(&delta);

        assert_eq!(adapter.edge_count(), 1);
        let edge_key = hash_to_hex(&make_test_hash(20));
        assert!(adapter.is_edge_valid(&edge_key));
    }

    #[test]
    fn test_clear() {
        let mut adapter = MockAdapter::new();

        // Add some items
        let delta1 = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 0,
            ops: vec![
                SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0])),
                SceneOp::UpsertNode(make_node(11, [1.0, 0.0, 0.0])),
                SceneOp::UpsertEdge(make_edge(20, 10, 11)),
            ],
        };
        adapter.apply_scene_delta(&delta1);
        assert_eq!(adapter.node_count(), 2);
        assert_eq!(adapter.edge_count(), 1);

        // Clear
        let delta2 = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 1,
            ops: vec![SceneOp::Clear],
        };
        adapter.apply_scene_delta(&delta2);
        assert_eq!(adapter.node_count(), 0);
        assert_eq!(adapter.edge_count(), 0);
        assert_eq!(adapter.label_count(), 0);
    }

    #[test]
    fn test_epoch_idempotency() {
        let mut adapter = MockAdapter::new();

        // Apply epoch 0
        let delta = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 0,
            ops: vec![SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0]))],
        };
        adapter.apply_scene_delta(&delta);
        assert_eq!(adapter.node_count(), 1);

        // Apply epoch 0 again with different data - should be ignored
        let delta_dup = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 0,
            ops: vec![SceneOp::UpsertNode(make_node(11, [1.0, 1.0, 1.0]))],
        };
        adapter.apply_scene_delta(&delta_dup);
        assert_eq!(adapter.node_count(), 1); // Still just 1 node
    }

    #[test]
    fn test_reset_cursor_allows_epoch_restart() {
        let mut adapter = MockAdapter::new();
        let cursor_id = make_test_hash(1);

        // Apply epoch 5
        let delta = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id,
            epoch: 5,
            ops: vec![SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0]))],
        };
        adapter.apply_scene_delta(&delta);
        assert_eq!(adapter.node_count(), 1);

        // Try epoch 3 - should be ignored
        let delta_old = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id,
            epoch: 3,
            ops: vec![SceneOp::UpsertNode(make_node(11, [1.0, 1.0, 1.0]))],
        };
        adapter.apply_scene_delta(&delta_old);
        assert_eq!(adapter.node_count(), 1);

        // Reset cursor
        adapter.reset_cursor(&cursor_id);

        // Now epoch 0 should work
        let delta_new = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id,
            epoch: 0,
            ops: vec![SceneOp::UpsertNode(make_node(12, [2.0, 2.0, 2.0]))],
        };
        adapter.apply_scene_delta(&delta_new);
        assert_eq!(adapter.node_count(), 2); // Now 2 nodes
    }

    #[test]
    fn test_different_cursors_independent() {
        let mut adapter = MockAdapter::new();
        let cursor_a = make_test_hash(1);
        let cursor_b = make_test_hash(2);

        // Apply to cursor A at epoch 0
        let delta_a = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: cursor_a,
            epoch: 0,
            ops: vec![SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0]))],
        };
        adapter.apply_scene_delta(&delta_a);
        assert_eq!(adapter.node_count(), 1);

        // Apply to cursor B at epoch 0 - different cursor, should work
        let delta_b = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: cursor_b,
            epoch: 0,
            ops: vec![SceneOp::UpsertNode(make_node(11, [1.0, 1.0, 1.0]))],
        };
        adapter.apply_scene_delta(&delta_b);
        assert_eq!(adapter.node_count(), 2);
    }

    #[test]
    fn test_two_adapters_same_deltas_same_state() {
        let mut adapter1 = MockAdapter::new();
        let mut adapter2 = MockAdapter::new();

        let deltas = vec![
            SceneDelta {
                session_id: make_test_hash(0),
                cursor_id: make_test_hash(1),
                epoch: 0,
                ops: vec![
                    SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0])),
                    SceneOp::UpsertNode(make_node(11, [1.0, 0.0, 0.0])),
                ],
            },
            SceneDelta {
                session_id: make_test_hash(0),
                cursor_id: make_test_hash(1),
                epoch: 1,
                ops: vec![SceneOp::UpsertEdge(make_edge(20, 10, 11))],
            },
            SceneDelta {
                session_id: make_test_hash(0),
                cursor_id: make_test_hash(1),
                epoch: 2,
                ops: vec![SceneOp::RemoveNode {
                    key: make_test_hash(10),
                }],
            },
        ];

        for delta in &deltas {
            adapter1.apply_scene_delta(delta);
            adapter2.apply_scene_delta(delta);
        }

        assert_eq!(adapter1.node_count(), adapter2.node_count());
        assert_eq!(adapter1.edge_count(), adapter2.edge_count());
        assert_eq!(adapter1.label_count(), adapter2.label_count());

        // Verify specific state
        assert_eq!(adapter1.node_count(), 1);
        assert_eq!(adapter1.edge_count(), 1);
    }

    #[test]
    fn test_set_camera() {
        let mut adapter = MockAdapter::new();
        let camera = CameraState {
            position: [10.0, 20.0, 30.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            projection: ProjectionKind::Orthographic,
            fov_y_radians: 1.0,
            ortho_scale: 25.0,
            near: 1.0,
            far: 500.0,
        };
        adapter.set_camera(&camera);
        assert_eq!(adapter.camera.position, [10.0, 20.0, 30.0]);
        assert_eq!(adapter.camera.projection, ProjectionKind::Orthographic);
    }

    #[test]
    fn test_set_highlight() {
        let mut adapter = MockAdapter::new();
        let highlight = HighlightState {
            selected_nodes: vec![make_test_hash(1), make_test_hash(2)],
            selected_edges: vec![],
            hovered_node: Some(make_test_hash(3)),
            hovered_edge: None,
        };
        adapter.set_highlight(&highlight);
        assert_eq!(adapter.highlight.selected_nodes.len(), 2);
        assert!(adapter.highlight.hovered_node.is_some());
    }

    #[test]
    fn test_render_count() {
        let mut adapter = MockAdapter::new();
        assert_eq!(adapter.render_count, 0);
        adapter.render();
        adapter.render();
        adapter.render();
        assert_eq!(adapter.render_count, 3);
    }

    #[test]
    fn test_resize() {
        let mut adapter = MockAdapter::new();
        adapter.resize(1920, 1080, 2.0);
        assert_eq!(adapter.viewport, (1920, 1080, 2.0));
    }

    #[test]
    fn test_dispose() {
        let mut adapter = MockAdapter::new();

        // Add some items
        let delta = SceneDelta {
            session_id: make_test_hash(0),
            cursor_id: make_test_hash(1),
            epoch: 0,
            ops: vec![SceneOp::UpsertNode(make_node(10, [0.0, 0.0, 0.0]))],
        };
        adapter.apply_scene_delta(&delta);
        assert!(!adapter.disposed);
        assert_eq!(adapter.node_count(), 1);

        adapter.dispose();
        assert!(adapter.disposed);
        assert_eq!(adapter.node_count(), 0);
    }
}
