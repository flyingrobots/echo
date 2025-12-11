// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared WASM-friendly DTOs for Echo/JITOS living specs.
//! These mirror the minimal graph + rewrite shapes used by Spec-000 and future specs.

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

/// Node identifier (stringified for wasm-bindgen/JS interop).
pub type NodeId = String;
/// Field name.
pub type FieldName = String;

/// Simple value bag for demo/spec transfer.
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Stable node identifier.
    pub id: NodeId,
    /// Field map.
    pub fields: HashMap<FieldName, Value>,
}

/// Graph edge (directed).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Source node id.
    pub from: NodeId,
    /// Target node id.
    pub to: NodeId,
}

/// Minimal RMG view for the WASM demo.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rmg {
    /// Node map keyed by id.
    pub nodes: HashMap<NodeId, Node>,
    /// Edges (directed).
    pub edges: Vec<Edge>,
}

/// Semantic operation kinds for rewrites.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SemanticOp {
    /// Set/overwrite a field value.
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
