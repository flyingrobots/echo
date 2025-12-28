// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal RMG + rewrite API exposed for WASM specs.
//!
//! Provides a tiny in-memory kernel for Spec-000 that mirrors the wasm ABI types.

use echo_wasm_abi::{Edge, Node};
pub use echo_wasm_abi::{Rewrite, Rmg, SemanticOp, Value};
use std::collections::HashMap;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Demo kernel with append-only rewrite history.
///
/// This is a **teaching/demo** kernel intended for living specs (e.g. Spec-000):
///
/// - It owns an in-memory [`Rmg`] and an append-only [`Rewrite`] history.
/// - It is designed for JS/WASM interop: when built with `--features wasm`, the type is exposed
///   via `wasm-bindgen` and provides `JsValue` serializers.
/// - It is not the production Echo kernel, does not validate invariants, and does not implement
///   canonical hashing / deterministic encoding.
///
/// Invariants (demo conventions):
///
/// - `history` ids are monotonic (`0..n`) within a single instance.
/// - Each public mutation method updates the graph first, then appends a matching rewrite record.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct DemoKernel {
    graph: Rmg,
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
            graph: Rmg::default(),
            history: Vec::new(),
        }
    }

    /// Add a node by id.
    pub fn add_node(&mut self, id: String) {
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
            old_value: None,
            new_value: None,
        });
    }

    /// Set a field value on a node.
    pub fn set_field(&mut self, target: String, field: String, value: Value) {
        if let Some(node) = self.graph.nodes.get_mut(&target) {
            node.fields.insert(field.clone(), value.clone());
        }
        self.history.push(Rewrite {
            id: self.history.len() as u64,
            op: SemanticOp::Set,
            target,
            old_value: Some(Value::Str(field)),
            new_value: Some(value),
        });
    }

    /// Add a directed edge between two nodes.
    pub fn connect(&mut self, from: String, to: String) {
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
            old_value: None,
            new_value: Some(Value::Str(to_id)),
        });
    }

    /// Delete a node and any incident edges.
    pub fn delete_node(&mut self, target: String) {
        self.graph.nodes.remove(&target);
        self.graph
            .edges
            .retain(|e| e.from != target && e.to != target);
        self.history.push(Rewrite {
            id: self.history.len() as u64,
            op: SemanticOp::DeleteNode,
            target,
            old_value: None,
            new_value: None,
        });
    }

    /// Get a clone of the current graph (host use).
    pub fn graph(&self) -> Rmg {
        self.graph.clone()
    }

    /// Get a clone of the rewrite history.
    pub fn history(&self) -> Vec<Rewrite> {
        self.history.clone()
    }

    /// Serialize graph to JSON (host use).
    pub fn graph_json(&self) -> String {
        serde_json::to_string(&self.graph).unwrap_or_default()
    }

    /// Serialize history to JSON (host use).
    pub fn history_json(&self) -> String {
        serde_json::to_string(&self.history).unwrap_or_default()
    }
}

// Expose JSON helpers to WASM when feature enabled.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl DemoKernel {
    /// Get graph as JsValue (serde).
    #[wasm_bindgen(js_name = serializeGraph)]
    pub fn serialize_graph(&self) -> wasm_bindgen::JsValue {
        wasm_bindgen::JsValue::from_serde(&self.graph).unwrap()
    }

    /// Get history as JsValue (serde).
    #[wasm_bindgen(js_name = serializeHistory)]
    pub fn serialize_history(&self) -> wasm_bindgen::JsValue {
        wasm_bindgen::JsValue::from_serde(&self.history).unwrap()
    }
}
