// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared WASM-friendly DTOs and Protocol Utilities for Echo.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod canonical;
pub use canonical::{CanonError, decode_value, encode_value};

pub mod eintlog;
pub use eintlog::*;

/// Errors produced by the Intent Envelope parser.
#[derive(Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    /// The 4-byte magic header "EINT" was missing or incorrect.
    InvalidMagic,
    /// The buffer was too short to contain the header or the declared payload.
    TooShort,
    /// The buffer length did not match the length declared in the header.
    LengthMismatch,
    /// Internal structure of the envelope was malformed (e.g. invalid integer encoding).
    Malformed,
}

/// Packs an application-blind intent envelope v1.
/// Layout: "EINT" (4 bytes) + op_id (u32 LE) + vars_len (u32 LE) + vars
pub fn pack_intent_v1(op_id: u32, vars: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + vars.len());
    out.extend_from_slice(b"EINT");
    out.extend_from_slice(&op_id.to_le_bytes());
    out.extend_from_slice(&(vars.len() as u32).to_le_bytes());
    out.extend_from_slice(vars);
    out
}

/// Unpacks an application-blind intent envelope v1.
/// Returns (op_id, vars_slice).
pub fn unpack_intent_v1(bytes: &[u8]) -> Result<(u32, &[u8]), EnvelopeError> {
    if bytes.len() < 12 {
        return Err(EnvelopeError::TooShort);
    }
    if &bytes[0..4] != b"EINT" {
        return Err(EnvelopeError::InvalidMagic);
    }
    
    let op_id_bytes: [u8; 4] = bytes[4..8].try_into().map_err(|_| EnvelopeError::Malformed)?;
    let op_id = u32::from_le_bytes(op_id_bytes);

    let vars_len_bytes: [u8; 4] = bytes[8..12].try_into().map_err(|_| EnvelopeError::Malformed)?;
    let vars_len = u32::from_le_bytes(vars_len_bytes) as usize;
    
    // Prevent integer overflow on 32-bit systems (though vars_len is u32, usize might be u32)
    let required_len = 12usize.checked_add(vars_len).ok_or(EnvelopeError::TooShort)?;

    if bytes.len() < required_len {
        return Err(EnvelopeError::TooShort);
    }
    if bytes.len() > required_len {
        return Err(EnvelopeError::LengthMismatch);
    }
    
    Ok((op_id, &bytes[12..]))
}

// -----------------------------------------------------------------------------
// Legacy DTOs (Retained for cross-repo compatibility, to be purged later)
// -----------------------------------------------------------------------------

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
            if n >= 0
                && let Ok(v) = u64::try_from(n)
            {
                return Ok(SV::U64(v));
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

pub type NodeId = String;
pub type FieldName = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    Str(String),
    Num(i64),
    Bool(bool),
    Null,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub fields: HashMap<FieldName, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WarpGraph {
    pub nodes: HashMap<NodeId, Node>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SemanticOp {
    Set,
    AddNode,
    DeleteNode,
    Connect,
    Disconnect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rewrite {
    pub id: u64,
    pub op: SemanticOp,
    pub target: NodeId,
    pub subject: Option<String>,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_round_trip() {
        let op_id = 12345;
        let vars = b"test payload";
        let packed = pack_intent_v1(op_id, vars);

        // Verify structure: "EINT" + op_id(4) + len(4) + payload
        assert_eq!(&packed[0..4], b"EINT");
        assert_eq!(&packed[4..8], &op_id.to_le_bytes());
        assert_eq!(&packed[8..12], &(vars.len() as u32).to_le_bytes());
        assert_eq!(&packed[12..], vars);

        // Round trip
        let (out_op, out_vars) = unpack_intent_v1(&packed).expect("unpack failed");
        assert_eq!(out_op, op_id);
        assert_eq!(out_vars, vars);
    }

    #[test]
    fn test_unpack_errors() {
        // Too short for header
        assert_eq!(unpack_intent_v1(b"EINT"), Err(EnvelopeError::TooShort));

        // Invalid magic
        assert_eq!(unpack_intent_v1(b"XXXX\x00\x00\x00\x00\x00\x00\x00\x00"), Err(EnvelopeError::InvalidMagic));

        // Payload shorter than declared length
        let mut short = Vec::new();
        short.extend_from_slice(b"EINT");
        short.extend_from_slice(&1u32.to_le_bytes()); // op_id
        short.extend_from_slice(&10u32.to_le_bytes()); // declared len 10
        short.extend_from_slice(b"123"); // actual len 3
        assert_eq!(unpack_intent_v1(&short), Err(EnvelopeError::TooShort));

        // Payload longer than declared length
        let mut long = Vec::new();
        long.extend_from_slice(b"EINT");
        long.extend_from_slice(&1u32.to_le_bytes()); // op_id
        long.extend_from_slice(&3u32.to_le_bytes()); // declared len 3
        long.extend_from_slice(b"12345"); // actual len 5
        assert_eq!(unpack_intent_v1(&long), Err(EnvelopeError::LengthMismatch));
    }

    #[test]
    fn test_empty_vars() {
        let packed = pack_intent_v1(99, &[]);
        let (op, vars) = unpack_intent_v1(&packed).unwrap();
        assert_eq!(op, 99);
        assert_eq!(vars, &[]);
    }
}
