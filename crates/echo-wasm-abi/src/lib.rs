// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared WASM-friendly DTOs and Protocol Utilities for Echo.

#![no_std]
#![allow(unsafe_code)]
// Low-level CBOR codec with intentional fixed-width casts and float ops.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::unnecessary_wraps,
    clippy::missing_errors_doc,
    clippy::match_same_arms,
    clippy::derive_partial_eq_without_eq
)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub mod canonical;
pub use canonical::{CanonError, decode_value, encode_value};

#[cfg(feature = "std")]
pub mod eintlog;
#[cfg(feature = "std")]
pub use eintlog::*;

pub mod kernel_port;

#[cfg(test)]
mod witnessed_suffix_tests;

pub mod ttd;
pub use ttd::*;

/// Deterministic binary codec for length-prefixed scalars and Q32.32 fixed-point helpers.
pub mod codec;

/// Reserved EINT op id for privileged control intents.
pub const CONTROL_INTENT_V1_OP_ID: u32 = u32::MAX;

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
    /// Payload length exceeds u32::MAX.
    PayloadTooLarge,
    /// Public application envelopes may not use the reserved control op id.
    ReservedOpId,
}

impl core::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidMagic => f.write_str("invalid EINT magic header"),
            Self::TooShort => f.write_str("envelope too short"),
            Self::LengthMismatch => f.write_str("envelope length mismatch"),
            Self::Malformed => f.write_str("malformed envelope"),
            Self::PayloadTooLarge => f.write_str("payload exceeds u32::MAX"),
            Self::ReservedOpId => f.write_str("reserved control op id is not allowed here"),
        }
    }
}

fn pack_envelope_v1_raw(op_id: u32, vars: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
    let vars_len: u32 = vars
        .len()
        .try_into()
        .map_err(|_| EnvelopeError::PayloadTooLarge)?;
    let mut out = Vec::with_capacity(12 + vars.len());
    out.extend_from_slice(b"EINT");
    out.extend_from_slice(&op_id.to_le_bytes());
    out.extend_from_slice(&vars_len.to_le_bytes());
    out.extend_from_slice(vars);
    Ok(out)
}

/// Packs an application-blind intent envelope v1.
/// Layout: "EINT" (4 bytes) + op_id (u32 LE) + vars_len (u32 LE) + vars
///
/// # Errors
/// Returns [`EnvelopeError::PayloadTooLarge`] if `vars.len()` exceeds
/// `u32::MAX`, or [`EnvelopeError::ReservedOpId`] if `op_id` is the reserved
/// control envelope id.
pub fn pack_intent_v1(op_id: u32, vars: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
    if op_id == CONTROL_INTENT_V1_OP_ID {
        return Err(EnvelopeError::ReservedOpId);
    }
    pack_envelope_v1_raw(op_id, vars)
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

    let op_id_bytes: [u8; 4] = bytes[4..8]
        .try_into()
        .map_err(|_| EnvelopeError::Malformed)?;
    let op_id = u32::from_le_bytes(op_id_bytes);

    let vars_len_bytes: [u8; 4] = bytes[8..12]
        .try_into()
        .map_err(|_| EnvelopeError::Malformed)?;
    let vars_len = u32::from_le_bytes(vars_len_bytes) as usize;

    // Prevent integer overflow on 32-bit systems (though vars_len is u32, usize might be u32)
    let required_len = 12usize
        .checked_add(vars_len)
        .ok_or(EnvelopeError::TooShort)?;

    if bytes.len() < required_len {
        return Err(EnvelopeError::TooShort);
    }
    if bytes.len() > required_len {
        return Err(EnvelopeError::LengthMismatch);
    }

    Ok((op_id, &bytes[12..]))
}

/// Packs a privileged control intent into an EINT envelope v1.
///
/// This helper always uses the protocol-reserved
/// [`CONTROL_INTENT_V1_OP_ID`]; callers do not provide or override the op id.
///
/// # Errors
///
/// Returns [`EnvelopeError::Malformed`] if
/// [`kernel_port::ControlIntentV1`] cannot be encoded as canonical CBOR.
/// Returns [`EnvelopeError::PayloadTooLarge`] if the encoded payload exceeds
/// the EINT v1 `u32` length field accepted by [`pack_envelope_v1_raw`].
pub fn pack_control_intent_v1(
    intent: &kernel_port::ControlIntentV1,
) -> Result<Vec<u8>, EnvelopeError> {
    let bytes = encode_cbor(intent).map_err(|_| EnvelopeError::Malformed)?;
    pack_envelope_v1_raw(CONTROL_INTENT_V1_OP_ID, &bytes)
}

/// Unpacks and decodes a privileged control intent from an EINT envelope v1.
///
/// The envelope must use the protocol-reserved [`CONTROL_INTENT_V1_OP_ID`].
///
/// # Errors
///
/// Returns [`EnvelopeError::InvalidMagic`], [`EnvelopeError::TooShort`], or
/// [`EnvelopeError::LengthMismatch`] if `bytes` is not a well-formed EINT v1
/// envelope as parsed by [`unpack_intent_v1`].
/// Returns [`EnvelopeError::Malformed`] if the envelope uses any other op id or
/// if the payload is not valid canonical CBOR for
/// [`kernel_port::ControlIntentV1`].
pub fn unpack_control_intent_v1(
    bytes: &[u8],
) -> Result<kernel_port::ControlIntentV1, EnvelopeError> {
    let (op_id, vars) = unpack_intent_v1(bytes)?;
    if op_id != CONTROL_INTENT_V1_OP_ID {
        return Err(EnvelopeError::Malformed);
    }
    decode_cbor(vars).map_err(|_| EnvelopeError::Malformed)
}

// -----------------------------------------------------------------------------
// Legacy DTOs (Retained for cross-repo compatibility, to be purged later)
// -----------------------------------------------------------------------------

/// Encode any serde value into deterministic CBOR bytes.
pub fn encode_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonError> {
    let val = serde_value::to_value(value).map_err(|e| CanonError::Encode(e.to_string()))?;
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
    use serde_value::Value::{
        Bool, Bytes, Char, F32, F64, I8, I16, I32, I64, Map, Newtype, Option, Seq, String, U8, U16,
        U32, U64, Unit,
    };
    Ok(match val {
        Bool(b) => CV::Bool(b),
        I8(n) => CV::Integer(i64::from(n).into()),
        I16(n) => CV::Integer(i64::from(n).into()),
        I32(n) => CV::Integer(i64::from(n).into()),
        I64(n) => CV::Integer(n.into()),
        U8(n) => CV::Integer(u64::from(n).into()),
        U16(n) => CV::Integer(u64::from(n).into()),
        U32(n) => CV::Integer(u64::from(n).into()),
        U64(n) => CV::Integer(n.into()),
        F32(f) => CV::Float(f64::from(f)),
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
            // Convert non-negative i128 to u64 if it fits
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
            let mut map = BTreeMap::new();
            for (k, v) in entries {
                map.insert(cv_to_sv(k)?, cv_to_sv(v)?);
            }
            SV::Map(map)
        }
        CV::Tag(_, _) => return Err(CanonError::Decode("tags not supported".into())),
        _ => return Err(CanonError::Decode("unsupported value".into())),
    })
}

/// Unique identifier for a graph node.
pub type NodeId = String;
/// Name of a field within a node.
pub type FieldName = String;

/// A typed value that can be stored in a node field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    /// String value.
    Str(String),
    /// Numeric value (64-bit signed integer).
    Num(i64),
    /// Boolean value.
    Bool(bool),
    /// Null/absent value.
    Null,
}

/// A node in the warp graph with an ID and field map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier for this node.
    pub id: NodeId,
    /// Map of field names to their values.
    pub fields: BTreeMap<FieldName, Value>,
}

/// A directed edge between two nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// Source node ID.
    pub from: NodeId,
    /// Target node ID.
    pub to: NodeId,
}

/// A graph structure containing nodes and edges.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WarpGraph {
    /// Map of node IDs to nodes.
    pub nodes: BTreeMap<NodeId, Node>,
    /// List of directed edges.
    pub edges: Vec<Edge>,
}

/// The type of semantic operation in a rewrite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticOp {
    /// Set a field value on a node.
    Set,
    /// Add a new node to the graph.
    AddNode,
    /// Delete an existing node from the graph.
    DeleteNode,
    /// Create an edge between two nodes.
    Connect,
    /// Remove an edge between two nodes.
    Disconnect,
}

/// A single rewrite operation describing a graph mutation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rewrite {
    /// Unique identifier for this rewrite operation.
    pub id: u64,
    /// The type of operation.
    pub op: SemanticOp,
    /// The target node ID.
    pub target: NodeId,
    /// Optional subject (e.g., field name or connected node).
    pub subject: Option<String>,
    /// Previous value before the operation (for Set operations).
    pub old_value: Option<Value>,
    /// New value after the operation (for Set operations).
    pub new_value: Option<Value>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::vec;

    fn hex_encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use core::fmt::Write as _;
            write!(&mut out, "{byte:02x}").unwrap();
        }
        out
    }

    #[test]
    fn test_pack_unpack_round_trip() {
        let op_id = 12345;
        let vars = b"test payload";
        let packed = pack_intent_v1(op_id, vars).expect("pack failed");

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
        assert_eq!(
            unpack_intent_v1(b"XXXX\x00\x00\x00\x00\x00\x00\x00\x00"),
            Err(EnvelopeError::InvalidMagic)
        );

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
        let packed = pack_intent_v1(99, &[]).unwrap();
        let (op, vars) = unpack_intent_v1(&packed).unwrap();
        assert_eq!(op, 99);
        assert_eq!(vars, &[] as &[u8]);
    }

    #[test]
    fn test_pack_intent_rejects_reserved_control_op_id() {
        assert_eq!(
            pack_intent_v1(CONTROL_INTENT_V1_OP_ID, b"reserved"),
            Err(EnvelopeError::ReservedOpId)
        );
    }

    #[test]
    fn test_control_intent_round_trip() {
        use crate::kernel_port::{
            ControlIntentV1, HeadId, SchedulerMode, WorldlineId, WriterHeadKey,
        };

        let packed = pack_control_intent_v1(&ControlIntentV1::Start {
            mode: SchedulerMode::UntilIdle {
                cycle_limit: Some(1),
            },
        })
        .unwrap();

        let unpacked = unpack_control_intent_v1(&packed).unwrap();
        assert_eq!(
            unpacked,
            ControlIntentV1::Start {
                mode: SchedulerMode::UntilIdle {
                    cycle_limit: Some(1),
                },
            }
        );

        let packed = pack_control_intent_v1(&ControlIntentV1::SetHeadEligibility {
            head: WriterHeadKey {
                worldline_id: WorldlineId::from_bytes([1u8; 32]),
                head_id: HeadId::from_bytes([2u8; 32]),
            },
            eligibility: crate::kernel_port::HeadEligibility::Dormant,
        })
        .unwrap();

        let unpacked = unpack_control_intent_v1(&packed).unwrap();
        assert_eq!(
            unpacked,
            ControlIntentV1::SetHeadEligibility {
                head: WriterHeadKey {
                    worldline_id: WorldlineId::from_bytes([1u8; 32]),
                    head_id: HeadId::from_bytes([2u8; 32]),
                },
                eligibility: crate::kernel_port::HeadEligibility::Dormant,
            }
        );
    }

    #[test]
    fn test_worldline_id_round_trip_uses_cbor_bytes() {
        use crate::kernel_port::WorldlineId;
        use ciborium::value::Value;

        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Wrapper {
            id: WorldlineId,
        }

        let bytes = encode_cbor(&Wrapper {
            id: WorldlineId::from_bytes([7u8; 32]),
        })
        .unwrap();
        let value = decode_value(&bytes).unwrap();
        assert!(matches!(value, Value::Map(_)));
        let Value::Map(entries) = value else {
            unreachable!();
        };
        let (_, encoded_id) = entries
            .into_iter()
            .find(|(key, _)| matches!(key, Value::Text(text) if text == "id"))
            .expect("id entry should exist");
        assert_eq!(encoded_id, Value::Bytes(vec![7u8; 32]));

        let decoded: Wrapper = decode_cbor(&bytes).unwrap();
        assert_eq!(
            decoded,
            Wrapper {
                id: WorldlineId::from_bytes([7u8; 32]),
            }
        );
    }

    #[test]
    fn test_worldline_id_rejects_non_32_byte_payloads() {
        use crate::kernel_port::WorldlineId;
        use ciborium::value::Value;

        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Wrapper {
            id: WorldlineId,
        }

        let bytes = encode_value(&Value::Map(vec![(
            Value::Text("id".into()),
            Value::Bytes(vec![9u8; 31]),
        )]))
        .unwrap();

        let err = decode_cbor::<Wrapper>(&bytes).unwrap_err();
        assert!(err.to_string().contains("32 bytes"));

        let bytes = encode_value(&Value::Map(vec![(
            Value::Text("id".into()),
            Value::Bytes(vec![9u8; 33]),
        )]))
        .unwrap();

        let err = decode_cbor::<Wrapper>(&bytes).unwrap_err();
        assert!(err.to_string().contains("32 bytes"));
    }

    #[test]
    fn test_worldline_id_rejects_integer_arrays() {
        use crate::kernel_port::WorldlineId;
        use ciborium::value::Value;

        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Wrapper {
            id: WorldlineId,
        }

        let bytes = encode_value(&Value::Map(vec![(
            Value::Text("id".into()),
            Value::Array(
                (0u8..32)
                    .map(|value| Value::Integer(value.into()))
                    .collect(),
            ),
        )]))
        .unwrap();

        let err = decode_cbor::<Wrapper>(&bytes).unwrap_err();
        assert!(err.to_string().contains("bytes"));
    }

    #[test]
    fn test_control_intent_wire_encoding_is_canonical() {
        use crate::kernel_port::{ControlIntentV1, SchedulerMode};

        let packed = pack_control_intent_v1(&ControlIntentV1::Start {
            mode: SchedulerMode::UntilIdle {
                cycle_limit: Some(1),
            },
        })
        .unwrap();

        assert_eq!(
            hex_encode(&packed),
            "45494e54ffffffff2f000000a2646b696e64657374617274646d6f6465a2646b696e646a756e74696c5f69646c656b6379636c655f6c696d697401"
        );
    }

    #[test]
    fn test_scheduler_status_wire_encoding_is_canonical() {
        use crate::kernel_port::{
            GlobalTick, RunCompletion, RunId, SchedulerState, SchedulerStatus, WorkState,
        };

        let status = SchedulerStatus {
            state: SchedulerState::Inactive,
            active_mode: None,
            work_state: WorkState::Quiescent,
            run_id: Some(RunId(7)),
            latest_cycle_global_tick: Some(GlobalTick(9)),
            latest_commit_global_tick: Some(GlobalTick(8)),
            last_quiescent_global_tick: Some(GlobalTick(9)),
            last_run_completion: Some(RunCompletion::Quiesced),
        };

        assert_eq!(
            hex_encode(&encode_cbor(&status).unwrap()),
            "a865737461746568696e6163746976656672756e5f6964076a776f726b5f737461746569717569657363656e746b6163746976655f6d6f6465f6736c6173745f72756e5f636f6d706c6574696f6e68717569657363656478186c61746573745f6379636c655f676c6f62616c5f7469636b0978196c61746573745f636f6d6d69745f676c6f62616c5f7469636b08781a6c6173745f717569657363656e745f676c6f62616c5f7469636b09"
        );
    }

    #[test]
    fn test_reading_residual_posture_wire_names_are_distinct() {
        use crate::kernel_port::ReadingResidualPosture;
        use ciborium::value::Value;

        let cases = [
            (
                ReadingResidualPosture::Complete,
                "complete",
                "68636f6d706c657465",
            ),
            (
                ReadingResidualPosture::Residual,
                "residual",
                "68726573696475616c",
            ),
            (
                ReadingResidualPosture::PluralityPreserved,
                "plurality_preserved",
                "73706c7572616c6974795f707265736572766564",
            ),
            (
                ReadingResidualPosture::Obstructed,
                "obstructed",
                "6a6f627374727563746564",
            ),
        ];

        for (posture, expected_text, expected_hex) in cases {
            let bytes = encode_cbor(&posture).unwrap();
            assert_eq!(hex_encode(&bytes), expected_hex);
            assert_eq!(
                decode_value(&bytes).unwrap(),
                Value::Text(expected_text.into())
            );
            let decoded: ReadingResidualPosture = decode_cbor(&bytes).unwrap();
            assert_eq!(decoded, posture);
        }
    }

    #[test]
    fn test_optic_core_dtos_round_trip() {
        use crate::kernel_port::{
            AttachmentDescentPolicy, BraidId, EchoCoordinate, EchoOptic, OpticAperture,
            OpticApertureShape, OpticCapabilityId, OpticFocus, OpticId, OpticReadBudget,
            ProjectionVersion, ReducerVersion, RetainedReadingKey, WorldlineId,
        };

        let optic = EchoOptic {
            optic_id: OpticId::from_bytes([1; 32]),
            focus: OpticFocus::Braid {
                braid_id: BraidId::from_bytes([2; 32]),
            },
            coordinate: EchoCoordinate::RetainedReading {
                key: RetainedReadingKey::from_bytes([3; 32]),
            },
            projection_version: ProjectionVersion(4),
            reducer_version: Some(ReducerVersion(5)),
            intent_family: crate::kernel_port::IntentFamilyId::from_bytes([6; 32]),
            capability: OpticCapabilityId::from_bytes([7; 32]),
        };

        let bytes = encode_cbor(&optic).unwrap();
        let decoded: EchoOptic = decode_cbor(&bytes).unwrap();
        assert_eq!(decoded, optic);

        let aperture = OpticAperture {
            shape: OpticApertureShape::QueryBytes {
                query_id: 42,
                vars_digest: vec![9; 32],
            },
            budget: OpticReadBudget {
                max_bytes: Some(1024),
                max_nodes: Some(64),
                max_ticks: Some(8),
                max_attachments: Some(0),
            },
            attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
        };
        let decoded: OpticAperture = decode_cbor(&encode_cbor(&aperture).unwrap()).unwrap();
        assert_eq!(decoded, aperture);

        let focus = OpticFocus::Worldline {
            worldline_id: WorldlineId::from_bytes([8; 32]),
        };
        let decoded: OpticFocus = decode_cbor(&encode_cbor(&focus).unwrap()).unwrap();
        assert_eq!(decoded, focus);
    }

    #[test]
    fn test_optic_read_identity_round_trip() {
        use crate::kernel_port::{
            BuiltinObserverPlan, EchoCoordinate, MissingWitnessBasisReason,
            ObservationBasisPosture, OpticId, OpticReadingEnvelope, ProjectionVersion,
            ReadIdentity, ReadingBudgetPosture, ReadingEnvelope, ReadingObserverBasis,
            ReadingObserverPlan, ReadingResidualPosture, ReadingRightsPosture, ReadingWitnessRef,
            WitnessBasis, WorldlineId, WorldlineTick,
        };

        let reference = crate::kernel_port::ProvenanceRef {
            worldline_id: WorldlineId::from_bytes([1; 32]),
            worldline_tick: WorldlineTick(7),
            commit_hash: vec![2; 32],
        };
        let identity = ReadIdentity {
            read_identity_hash: vec![3; 32],
            optic_id: OpticId::from_bytes([4; 32]),
            focus_digest: vec![5; 32],
            coordinate: EchoCoordinate::Worldline {
                worldline_id: WorldlineId::from_bytes([1; 32]),
                at: crate::kernel_port::CoordinateAt::Frontier,
            },
            aperture_digest: vec![6; 32],
            projection_version: ProjectionVersion(1),
            reducer_version: None,
            witness_basis: WitnessBasis::Missing {
                reason: MissingWitnessBasisReason::EvidenceUnavailable,
            },
            rights_posture: ReadingRightsPosture::KernelPublic,
            budget_posture: ReadingBudgetPosture::UnboundedOneShot,
            residual_posture: ReadingResidualPosture::Obstructed,
        };
        let envelope = OpticReadingEnvelope {
            reading: ReadingEnvelope {
                observer_plan: ReadingObserverPlan::Builtin {
                    plan: BuiltinObserverPlan::CommitBoundaryHead,
                },
                observer_basis: ReadingObserverBasis::CommitBoundary,
                witness_refs: vec![ReadingWitnessRef::ResolvedCommit { reference }],
                parent_basis_posture: ObservationBasisPosture::Worldline,
                budget_posture: ReadingBudgetPosture::UnboundedOneShot,
                rights_posture: ReadingRightsPosture::KernelPublic,
                residual_posture: ReadingResidualPosture::Obstructed,
            },
            read_identity: identity,
        };

        let decoded: OpticReadingEnvelope = decode_cbor(&encode_cbor(&envelope).unwrap()).unwrap();
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn test_optic_intent_dispatch_result_variants_round_trip() {
        use crate::kernel_port::{
            AdmittedIntent, CoordinateAt, EchoCoordinate, IntentConflict, IntentConflictReason,
            IntentDispatchResult, IntentFamilyId, MissingWitnessBasisReason, OpticFocus, OpticId,
            OpticObstruction, OpticObstructionKind, PluralIntent, ReadingResidualPosture,
            StagedIntent, StagedIntentReason, StrandId, WitnessBasis, WorldlineId, WorldlineTick,
        };

        fn classify(result: &IntentDispatchResult) -> &'static str {
            match result {
                IntentDispatchResult::Admitted(_) => "admitted",
                IntentDispatchResult::Staged(_) => "staged",
                IntentDispatchResult::Plural(_) => "plural",
                IntentDispatchResult::Conflict(_) => "conflict",
                IntentDispatchResult::Obstructed(_) => "obstructed",
            }
        }

        let optic_id = OpticId::from_bytes([1; 32]);
        let intent_family = IntentFamilyId::from_bytes([2; 32]);
        let worldline_id = WorldlineId::from_bytes([3; 32]);
        let base_coordinate = EchoCoordinate::Worldline {
            worldline_id,
            at: CoordinateAt::Frontier,
        };
        let admitted_ref = crate::kernel_port::ProvenanceRef {
            worldline_id,
            worldline_tick: WorldlineTick(4),
            commit_hash: vec![5; 32],
        };

        let outcomes = vec![
            IntentDispatchResult::Admitted(AdmittedIntent {
                optic_id,
                base_coordinate: base_coordinate.clone(),
                intent_family,
                admitted_ref: admitted_ref.clone(),
                receipt_hash: vec![6; 32],
            }),
            IntentDispatchResult::Staged(StagedIntent {
                optic_id,
                base_coordinate: base_coordinate.clone(),
                intent_family,
                stage_ref: vec![7; 32],
                reason: StagedIntentReason::AwaitingWitness,
            }),
            IntentDispatchResult::Plural(PluralIntent {
                optic_id,
                base_coordinate: base_coordinate.clone(),
                intent_family,
                candidate_refs: vec![admitted_ref.clone()],
                residual_posture: ReadingResidualPosture::PluralityPreserved,
            }),
            IntentDispatchResult::Conflict(IntentConflict {
                optic_id,
                base_coordinate: base_coordinate.clone(),
                intent_family,
                reason: IntentConflictReason::ConflictingFrontier,
                conflict_ref: Some(admitted_ref),
                evidence_digest: vec![8; 32],
                message: "frontier conflict".into(),
            }),
            IntentDispatchResult::Obstructed(OpticObstruction {
                kind: OpticObstructionKind::AttachmentDescentRequired,
                optic_id: Some(optic_id),
                focus: Some(OpticFocus::Strand {
                    strand_id: StrandId::from_bytes([9; 32]),
                }),
                coordinate: Some(base_coordinate),
                witness_basis: Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::EvidenceUnavailable,
                }),
                message: "explicit attachment descent required".into(),
            }),
        ];

        let decoded_labels = outcomes
            .iter()
            .map(|outcome| {
                let decoded: IntentDispatchResult =
                    decode_cbor(&encode_cbor(outcome).unwrap()).unwrap();
                assert_eq!(&decoded, outcome);
                classify(&decoded)
            })
            .collect::<Vec<_>>();

        assert_eq!(
            decoded_labels,
            vec!["admitted", "staged", "plural", "conflict", "obstructed"]
        );
    }

    #[test]
    fn test_optic_open_close_models_round_trip() {
        use crate::kernel_port::{
            CapabilityPosture, CloseOpticRequest, CloseOpticResult, CoordinateAt, EchoCoordinate,
            EchoOptic, IntentFamilyId, OpenOpticRequest, OpenOpticResult, OpticActorId,
            OpticCapability, OpticCapabilityId, OpticCause, OpticFocus, OpticId, OpticObstruction,
            OpticObstructionKind, OpticOpenError, OpticReadBudget, ProjectionVersion, WorldlineId,
            WorldlineTick,
        };

        let worldline_id = WorldlineId::from_bytes([1; 32]);
        let focus = OpticFocus::Worldline { worldline_id };
        let coordinate = EchoCoordinate::Worldline {
            worldline_id,
            at: CoordinateAt::Frontier,
        };
        let actor = OpticActorId::from_bytes([2; 32]);
        let capability_id = OpticCapabilityId::from_bytes([3; 32]);
        let intent_family = IntentFamilyId::from_bytes([4; 32]);
        let issuer_ref = crate::kernel_port::ProvenanceRef {
            worldline_id,
            worldline_tick: WorldlineTick(5),
            commit_hash: vec![6; 32],
        };
        let cause = OpticCause {
            actor,
            cause_hash: vec![7; 32],
            label: Some("test open".into()),
        };
        let capability = OpticCapability {
            capability_id,
            actor,
            issuer_ref: Some(issuer_ref.clone()),
            policy_hash: vec![8; 32],
            allowed_focus: focus.clone(),
            projection_version: ProjectionVersion(1),
            reducer_version: None,
            allowed_intent_family: intent_family,
            max_budget: OpticReadBudget {
                max_bytes: Some(4096),
                max_nodes: Some(64),
                max_ticks: Some(8),
                max_attachments: Some(0),
            },
        };
        let request = OpenOpticRequest {
            focus: focus.clone(),
            coordinate: coordinate.clone(),
            projection_version: ProjectionVersion(1),
            reducer_version: None,
            intent_family,
            capability,
            cause: cause.clone(),
        };
        let decoded: OpenOpticRequest = decode_cbor(&encode_cbor(&request).unwrap()).unwrap();
        assert_eq!(decoded, request);

        let result = OpenOpticResult {
            optic: EchoOptic {
                optic_id: OpticId::from_bytes([9; 32]),
                focus,
                coordinate: coordinate.clone(),
                projection_version: ProjectionVersion(1),
                reducer_version: None,
                intent_family,
                capability: capability_id,
            },
            capability_posture: CapabilityPosture::Granted {
                capability_id,
                actor,
                issuer_ref: Some(issuer_ref),
                policy_hash: vec![8; 32],
            },
        };
        let decoded: OpenOpticResult = decode_cbor(&encode_cbor(&result).unwrap()).unwrap();
        assert_eq!(decoded, result);

        let error = OpticOpenError::Obstructed(OpticObstruction {
            kind: OpticObstructionKind::CapabilityDenied,
            optic_id: None,
            focus: None,
            coordinate: Some(coordinate),
            witness_basis: None,
            message: "capability denied".into(),
        });
        let decoded: OpticOpenError = decode_cbor(&encode_cbor(&error).unwrap()).unwrap();
        assert_eq!(decoded, error);

        let close_request = CloseOpticRequest {
            optic_id: OpticId::from_bytes([9; 32]),
            cause,
        };
        let decoded: CloseOpticRequest =
            decode_cbor(&encode_cbor(&close_request).unwrap()).unwrap();
        assert_eq!(decoded, close_request);

        let close_result = CloseOpticResult {
            optic_id: OpticId::from_bytes([9; 32]),
        };
        let decoded: CloseOpticResult = decode_cbor(&encode_cbor(&close_result).unwrap()).unwrap();
        assert_eq!(decoded, close_result);
    }

    #[test]
    fn test_unpack_control_intent_rejects_wrong_op_id() {
        use crate::kernel_port::{ControlIntentV1, SchedulerMode};

        let payload = encode_cbor(&ControlIntentV1::Start {
            mode: SchedulerMode::UntilIdle { cycle_limit: None },
        })
        .unwrap();
        let packed = pack_intent_v1(99, &payload).unwrap();

        assert_eq!(
            unpack_control_intent_v1(&packed),
            Err(EnvelopeError::Malformed)
        );
    }

    #[test]
    fn test_unpack_control_intent_rejects_malformed_cbor() {
        let packed = pack_envelope_v1_raw(CONTROL_INTENT_V1_OP_ID, &[0xff]).unwrap();

        assert_eq!(
            unpack_control_intent_v1(&packed),
            Err(EnvelopeError::Malformed)
        );
    }
}
