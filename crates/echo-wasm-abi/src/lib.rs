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
use std::collections::HashMap;

pub mod canonical;
pub use canonical::{CanonError, decode_value, encode_value};

/// Encode any serde value into deterministic CBOR bytes.
pub fn encode_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonError> {
    let val = serde_value::to_value(value).map_err(|e| CanonError::Decode(e.to_string()))?;
    let canon = sv_to_cv(val)?;
    encode_value(&canon)
}

/// Decode deterministic CBOR bytes into a serde value.
pub fn decode_cbor<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, CanonError> {
    let val = decode_value(bytes)?;
    let sv = cv_to_sv(val)?;
    T::deserialize(sv).map_err(|e| CanonError::Decode(e.to_string()))
}

fn sv_to_cv(val: serde_value::Value) -> Result<ciborium::value::Value, CanonError> {
    use ciborium::value::Value as CV;
    use serde_value::Value::*;
    Ok(match val {
        Bool(b) => CV::Bool(b),
        I8(n) => CV::Integer((n as i64).into()),
        I16(n) => CV::Integer((n as i64).into()),
        I32(n) => CV::Integer((n as i64).into()),
        I64(n) => CV::Integer(n.into()),
        U8(n) => CV::Integer((n as u64).into()),
        U16(n) => CV::Integer((n as u64).into()),
        U32(n) => CV::Integer((n as u64).into()),
        U64(n) => CV::Integer(n.into()),
        F32(f) => CV::Float(f as f64),
        F64(f) => CV::Float(f),
        Char(c) => CV::Text(c.to_string()),
        String(s) => CV::Text(s),
        Bytes(b) => CV::Bytes(b),
        Unit => CV::Null,
        Option(None) => CV::Null,
        Option(Some(v)) => sv_to_cv(*v)?,
        Newtype(v) => sv_to_cv(*v)?,
        Seq(vs) => {
            let mut out = Vec::with_capacity(vs.len());
            for v in vs {
                out.push(sv_to_cv(v)?);
            }
            CV::Array(out)
        }
        Map(m) => {
            let mut out = Vec::with_capacity(m.len());
            for (k, v) in m {
                out.push((sv_to_cv(k)?, sv_to_cv(v)?));
            }
            CV::Map(out)
        }
    })
}

fn cv_to_sv(val: ciborium::value::Value) -> Result<serde_value::Value, CanonError> {
    use ciborium::value::Value as CV;
    use serde_value::Value as SV;
    Ok(match val {
        CV::Bool(b) => SV::Bool(b),
        CV::Null => SV::Unit,
        CV::Integer(i) => {
            let n: i128 = i.into();
            if n >= 0 {
                if let Ok(v) = u64::try_from(n) {
                    return Ok(SV::U64(v));
                }
            }
            if let Ok(v) = i64::try_from(n) {
                SV::I64(v)
            } else {
                return Err(CanonError::Decode("integer out of range".into()));
            }
        }
        CV::Float(f) => SV::F64(f),
        CV::Text(s) => SV::String(s),
        CV::Bytes(b) => SV::Bytes(b),
        CV::Array(vs) => {
            let mut out = Vec::with_capacity(vs.len());
            for v in vs {
                out.push(cv_to_sv(v)?);
            }
            SV::Seq(out)
        }
        CV::Map(entries) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in entries {
                map.insert(cv_to_sv(k)?, cv_to_sv(v)?);
            }
            SV::Map(map)
        }
        CV::Tag(_, _) => return Err(CanonError::Decode("tags not supported".into())),
        _ => return Err(CanonError::Decode("unsupported value".into())),
    })
}

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
/// - `id` should be unique within a [`WarpGraph`] (not enforced by the type).
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

/// Minimal WARP graph view for the WASM demo.
///
/// This is the “teaching graph” representation used by Spec-000 and friends, not the canonical
/// engine graph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WarpGraph {
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
/// - `subject` is an optional secondary identifier (e.g., field name for `Set`).
/// - `old_value` / `new_value` are intentionally generic to keep the DTO small; their meaning is
///   operation-dependent (see below).
///
/// Operation field semantics (Spec-000 demo conventions):
///
/// - [`SemanticOp::AddNode`]: `target = node_id`, values are `None`.
/// - [`SemanticOp::DeleteNode`]: `target = node_id`, values are `None`.
/// - [`SemanticOp::Set`]: `target = node_id`, `subject = Some(field_name)`,
///   `old_value = Some(prior_value)` (or `None`), and `new_value = Some(new_value)`.
/// - [`SemanticOp::Connect`]: `target = from_id`, `new_value = Some(Value::Str(to_id))`.
/// - [`SemanticOp::Disconnect`]: same encoding as `Connect`, but interpreted as removal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rewrite {
    /// Monotonic rewrite id within history.
    pub id: u64,
    /// Operation kind.
    pub op: SemanticOp,
    /// Target node id.
    pub target: NodeId,
    /// Optional secondary identifier for the operation.
    ///
    /// For [`SemanticOp::Set`] this is the field name.
    pub subject: Option<String>,
    /// Prior value (if any).
    pub old_value: Option<Value>,
    /// New value (if any).
    pub new_value: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_cbor_roundtrip_rewrite() {
        let rw = Rewrite {
            id: 42,
            op: SemanticOp::Set,
            target: "A".into(),
            subject: Some("title".into()),
            old_value: Some(Value::Str("Old".into())),
            new_value: Some(Value::Str("New".into())),
        };
        let bytes = encode_cbor(&rw).expect("encode");
        let back: Rewrite = decode_cbor(&bytes).expect("decode");
        assert_eq!(rw, back);
    }

    #[test]
    fn serialize_rewrite_round_trips_across_ops() {
        let cases = [
            Rewrite {
                id: 1,
                op: SemanticOp::AddNode,
                target: "A".into(),
                subject: None,
                old_value: None,
                new_value: None,
            },
            Rewrite {
                id: 2,
                op: SemanticOp::Set,
                target: "A".into(),
                subject: Some("name".into()),
                old_value: Some(Value::Str("Prior".into())),
                new_value: Some(Value::Str("Server".into())),
            },
            Rewrite {
                id: 3,
                op: SemanticOp::Connect,
                target: "A".into(),
                subject: None,
                old_value: None,
                new_value: Some(Value::Str("B".into())),
            },
            Rewrite {
                id: 4,
                op: SemanticOp::Disconnect,
                target: "A".into(),
                subject: None,
                old_value: None,
                new_value: Some(Value::Str("B".into())),
            },
            Rewrite {
                id: 5,
                op: SemanticOp::DeleteNode,
                target: "A".into(),
                subject: None,
                old_value: None,
                new_value: None,
            },
        ];

        for rw in cases {
            let json = serde_json::to_string(&rw).expect("serialize");
            let back: Rewrite = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(rw, back);
        }
    }
}
