// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal WARP graph + rewrite API exposed for WASM specs.
//!
//! Provides a tiny in-memory kernel for Spec-000 that mirrors the wasm ABI types.

use echo_wasm_abi::{Edge, Node};
pub use echo_wasm_abi::{Rewrite, SemanticOp, Value, WarpGraph};
use std::collections::HashMap;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Demo kernel with append-only rewrite history.
///
/// This is a **teaching/demo** kernel intended for living specs (e.g. Spec-000):
///
/// - It owns an in-memory [`WarpGraph`] and an append-only [`Rewrite`] history.
/// - It is designed for JS/WASM interop: when built with `--features wasm`, the type is exposed
///   via `wasm-bindgen` and provides JS object accessors.
/// - It is not the production Echo kernel, does not validate invariants, and does not implement
///   canonical hashing / deterministic encoding.
///
/// Invariants (demo conventions):
///
/// - `history` ids are monotonic (`0..n`) within a single instance.
/// - Each public mutation method updates the graph first, then appends a matching rewrite record.
/// - Operations that cannot be applied (e.g., missing node ids) are treated as no-ops and are not
///   recorded in `history`.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct DemoKernel {
    graph: WarpGraph,
    history: Vec<Rewrite>,
}

impl Default for DemoKernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl DemoKernel {
    /// Create a new empty kernel.
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self {
            graph: WarpGraph::default(),
            history: Vec::new(),
        }
    }

    /// Add a node by id.
    ///
    /// If the id already exists, this operation is a no-op and is not recorded in `history`.
    pub fn add_node(&mut self, id: String) {
        if self.graph.nodes.contains_key(&id) {
            return;
        }

        let node_id = id;
        self.graph.nodes.insert(
            node_id.clone(),
            Node {
                id: node_id.clone(),
                fields: HashMap::new(),
            },
        );
        self.history.push(Rewrite {
            id: self.history.len() as u64,
            op: SemanticOp::AddNode,
            target: node_id,
            subject: None,
            old_value: None,
            new_value: None,
        });
    }

    /// Add a directed edge between two nodes.
    pub fn connect(&mut self, from: String, to: String) {
        if !self.graph.nodes.contains_key(&from) || !self.graph.nodes.contains_key(&to) {
            return;
        }

        let from_id = from;
        let to_id = to;
        self.graph.edges.push(Edge {
            from: from_id.clone(),
            to: to_id.clone(),
        });
        self.history.push(Rewrite {
            id: self.history.len() as u64,
            op: SemanticOp::Connect,
            target: from_id,
            subject: None,
            old_value: None,
            new_value: Some(Value::Str(to_id)),
        });
    }

    /// Delete a node and any incident edges.
    pub fn delete_node(&mut self, target: String) {
        if self.graph.nodes.remove(&target).is_some() {
            self.graph
                .edges
                .retain(|e| e.from != target && e.to != target);
            self.history.push(Rewrite {
                id: self.history.len() as u64,
                op: SemanticOp::DeleteNode,
                target,
                subject: None,
                old_value: None,
                new_value: None,
            });
        }
    }
}

// Separate impl for set_field to handle WASM vs Host types for 'value'.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl DemoKernel {
    /// Set a field value on a node (WASM version).
    #[wasm_bindgen(js_name = setField)]
    pub fn set_field_wasm(&mut self, target: String, field: String, value: JsValue) {
        let val: Value = serde_wasm_bindgen::from_value(value).unwrap_or(Value::Null);
        self.set_field(target, field, val);
    }

    /// Export the current graph state as a JS object.
    #[wasm_bindgen(js_name = getGraph)]
    pub fn get_graph_wasm(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.graph).unwrap_or(JsValue::NULL)
    }

    /// Export the rewrite history as a JS array.
    #[wasm_bindgen(js_name = getHistory)]
    pub fn get_history_wasm(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.history).unwrap_or(JsValue::NULL)
    }
}

impl DemoKernel {
    /// Set a field value on a node (Host version).
    pub fn set_field(&mut self, target: String, field: String, value: Value) {
        if let Some(node) = self.graph.nodes.get_mut(&target) {
            let prior_value = node.fields.get(&field).cloned();
            node.fields.insert(field.clone(), value.clone());
            self.history.push(Rewrite {
                id: self.history.len() as u64,
                op: SemanticOp::Set,
                target,
                subject: Some(field),
                old_value: prior_value,
                new_value: Some(value),
            });
        }
    }

    /// Get a reference to the current graph.
    pub fn graph(&self) -> &WarpGraph {
        &self.graph
    }

    /// Get a reference to the rewrite history.
    pub fn history(&self) -> &[Rewrite] {
        &self.history
    }
}
