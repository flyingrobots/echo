// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared WASM-friendly DTOs for Echo/JITOS living specs.
//!
//! This crate is intentionally small and **WASM-friendly**:
//!
//! - The types are designed to cross the JS boundary (via `serde` + `wasm-bindgen` wrappers).
//! - The shapes are used by Spec-000 (and future interactive specs) to render and mutate a tiny
//!   “teaching graph” in the browser.
//!
//! Determinism note:
//!
//! - These DTOs are *not* the canonical deterministic wire format for Echo networking.
//! - In particular, maps are stored as `HashMap` for ergonomic interop; ordering is not stable.
//! - For canonical/deterministic transport and hashing, prefer `echo-session-proto` / `echo-graph`.

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

/// Node identifier used in the living-spec demos.
///
/// Uses a `String` rather than an integer to keep JS/WASM interop simple and ergonomic.
pub type NodeId = String;

/// Field name used in the living-spec demos.
pub type FieldName = String;

/// Simple tagged value for demo/spec transfer.
///
/// Serialized as `{ "kind": "...", "value": ... }` to make the JS-side shape explicit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    /// UTF-8 string value.
    Str(String),
    /// 64-bit integer.
    Num(i64),
    /// Boolean value.
    Bool(bool),
    /// Explicit null.
    Null,
}

/// Graph node with arbitrary fields.
///
/// Invariants:
///
/// - `id` should be unique within an [`Rmg`] (not enforced by the type).
/// - `fields` is an unordered bag of per-node values intended for UI/demo state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Stable node identifier.
    pub id: NodeId,
    /// Field map (unordered).
    pub fields: HashMap<FieldName, Value>,
}

/// Graph edge (directed).
///
/// In the demo, edges are not required to be unique and are not validated against the node set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Source node id.
    pub from: NodeId,
    /// Target node id.
    pub to: NodeId,
}

/// Minimal render-metagraph (RMG) view for the WASM demo.
///
/// This is the “teaching graph” representation used by Spec-000 and friends, not the canonical
/// engine graph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rmg {
    /// Node map keyed by id (unordered).
    pub nodes: HashMap<NodeId, Node>,
    /// Edges (directed).
    pub edges: Vec<Edge>,
}

/// Semantic operation kinds for rewrites.
///
/// These are high-level demo operations, used to label [`Rewrite`] records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SemanticOp {
    /// Set/overwrite a field value on a node.
    Set,
    /// Add a new node.
    AddNode,
    /// Delete/tombstone a node.
    DeleteNode,
    /// Add a directed edge.
    Connect,
    /// Remove a directed edge.
    Disconnect,
}

/// Rewrite record (append-only).
///
/// This is the minimal “history entry” the living specs append when mutating the demo graph.
///
/// Invariants and conventions:
///
/// - `id` is expected to be monotonic within a single history (the demo kernel uses `0..n`).
/// - `target` is the primary node id the operation is about.
/// - `old_value` / `new_value` are intentionally generic to keep the DTO small; their meaning is
///   operation-dependent (see below).
///
/// Operation field semantics (Spec-000 demo conventions):
///
/// - [`SemanticOp::AddNode`]: `target = node_id`, values are `None`.
/// - [`SemanticOp::DeleteNode`]: `target = node_id`, values are `None`.
/// - [`SemanticOp::Set`]: `target = node_id`, `old_value = Some(Value::Str(field_name))`,
///   `new_value = Some(new_field_value)`.
/// - [`SemanticOp::Connect`]: `target = from_id`, `new_value = Some(Value::Str(to_id))`.
/// - [`SemanticOp::Disconnect`]: same encoding as `Connect`, but interpreted as removal.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rewrite {
    /// Monotonic rewrite id within history.
    pub id: u64,
    /// Operation kind.
    pub op: SemanticOp,
    /// Target node id.
    pub target: NodeId,
    /// Prior value (if any).
    pub old_value: Option<Value>,
    /// New value (if any).
    pub new_value: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_rewrite_round_trip() {
        let rw = Rewrite {
            id: 1,
            op: SemanticOp::AddNode,
            target: "A".into(),
            old_value: None,
            new_value: None,
        };
        let json = serde_json::to_string(&rw).expect("serialize");
        let back: Rewrite = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rw, back);
    }
}
