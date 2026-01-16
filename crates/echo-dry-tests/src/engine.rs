// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Engine and GraphStore builder utilities for tests.

use warp_core::{make_node_id, make_type_id, Engine, GraphStore, NodeId, NodeRecord, RewriteRule};

/// Builder for creating test engines with common configurations.
///
/// # Example
///
/// ```
/// use echo_dry_tests::EngineTestBuilder;
///
/// let engine = EngineTestBuilder::new()
///     .with_root("my-root")
///     .build();
/// ```
pub struct EngineTestBuilder {
    root_label: String,
    root_type: String,
    rules: Vec<RewriteRule>,
}

impl Default for EngineTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineTestBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            root_label: "root".to_string(),
            root_type: "root".to_string(),
            rules: Vec::new(),
        }
    }

    /// Set the root node label (used to generate the node ID).
    pub fn with_root(mut self, label: &str) -> Self {
        self.root_label = label.to_string();
        self
    }

    /// Set the root node type ID label.
    pub fn with_root_type(mut self, type_label: &str) -> Self {
        self.root_type = type_label.to_string();
        self
    }

    /// Add a rule to be registered after engine creation.
    pub fn with_rule(mut self, rule: RewriteRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add the standard motion rule.
    pub fn with_motion_rule(self) -> Self {
        self.with_rule(crate::demo_rules::motion_rule())
    }

    /// Add the dispatch inbox rule.
    pub fn with_dispatch_inbox_rule(self) -> Self {
        self.with_rule(warp_core::inbox::dispatch_inbox_rule())
    }

    /// Add the ack pending rule.
    pub fn with_ack_pending_rule(self) -> Self {
        self.with_rule(warp_core::inbox::ack_pending_rule())
    }

    /// Add standard inbox rules (dispatch + ack).
    pub fn with_inbox_rules(self) -> Self {
        self.with_dispatch_inbox_rule().with_ack_pending_rule()
    }

    /// Build the engine with configured settings.
    pub fn build(self) -> Engine {
        let root = make_node_id(&self.root_label);
        let mut store = GraphStore::default();
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id(&self.root_type),
            },
        );
        let mut engine = Engine::new(store, root);

        for rule in self.rules {
            engine.register_rule(rule).expect("register rule");
        }

        engine
    }

    /// Build the engine and return both the engine and the root node ID.
    pub fn build_with_root(self) -> (Engine, NodeId) {
        let root = make_node_id(&self.root_label);
        let mut store = GraphStore::default();
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id(&self.root_type),
            },
        );
        let mut engine = Engine::new(store, root);

        for rule in self.rules {
            engine.register_rule(rule).expect("register rule");
        }

        (engine, root)
    }
}

/// Creates an Engine with a default GraphStore and a single root node.
///
/// This is a shorthand for the common pattern:
/// ```
/// use echo_dry_tests::engine::build_engine_with_root;
/// use warp_core::make_node_id;
///
/// let root = make_node_id("root");
/// let engine = build_engine_with_root(root);
/// ```
pub fn build_engine_with_root(root: NodeId) -> Engine {
    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("root"),
        },
    );
    Engine::new(store, root)
}

/// Create an engine with a root node and custom type.
pub fn build_engine_with_typed_root(root: NodeId, type_label: &str) -> Engine {
    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(type_label),
        },
    );
    Engine::new(store, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_creates_engine_with_root() {
        let engine = EngineTestBuilder::new().with_root("test-root").build();
        let store = engine.store_clone();
        let root = make_node_id("test-root");
        assert!(store.node(&root).is_some());
    }

    #[test]
    fn builder_with_motion_rule_registers_rule() {
        let engine = EngineTestBuilder::new().with_motion_rule().build();
        // Engine should have motion rule registered (no direct way to check,
        // but it shouldn't panic during creation)
        let _ = engine;
    }

    #[test]
    fn build_with_root_returns_both() {
        let (engine, root) = EngineTestBuilder::new()
            .with_root("my-root")
            .build_with_root();
        let store = engine.store_clone();
        assert!(store.node(&root).is_some());
    }
}
