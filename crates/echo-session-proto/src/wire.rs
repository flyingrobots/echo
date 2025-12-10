// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! JS-ABI v1.0 deterministic framing and CBOR helpers (ADR/ARCH-0013).
//!
//! Packet layout (ARCH-0013):
//!
//! ``MAGIC(4) || VERSION(2) || FLAGS(2) || LENGTH(4) || PAYLOAD || CHECKSUM(32)``
//!
//! * PAYLOAD is a canonical CBOR `OpEnvelope` (ADR-0013)
//! * CHECKSUM = blake3-256 over HEADER (first 12 bytes) || PAYLOAD

use blake3::Hasher;
use ciborium::value::{Integer, Value};
use serde::Serialize;
use serde_value::{to_value, Value as SerdeValue};
use std::io;

use crate::canonical::{decode_value, encode_value};
use crate::{Message, OpEnvelope, RmgStreamPayload, SubscribeRmgPayload};

fn sv_to_cv(val: SerdeValue) -> Result<Value, String> {
    use serde_value::Value::*;
    match val {
        Bool(b) => Ok(Value::Bool(b)),
        I8(n) => Ok(Value::Integer(Integer::from(n))),
        I16(n) => Ok(Value::Integer(Integer::from(n))),
        I32(n) => Ok(Value::Integer(Integer::from(n))),
        I64(n) => Ok(Value::Integer(Integer::from(n))),
        U8(n) => Ok(Value::Integer(Integer::from(n))),
        U16(n) => Ok(Value::Integer(Integer::from(n))),
        U32(n) => Ok(Value::Integer(Integer::from(n))),
        U64(n) => Ok(Value::Integer(Integer::from(n))),
        F32(f) => Ok(Value::Float(f as f64)),
        F64(f) => Ok(Value::Float(f)),
        Char(c) => Ok(Value::Text(c.to_string())),
        String(s) => Ok(Value::Text(s)),
        Bytes(b) => Ok(Value::Bytes(b)),
        Unit => Ok(Value::Null),
        Option(None) => Ok(Value::Null),
        Option(Some(v)) => sv_to_cv(*v),
        Newtype(v) => sv_to_cv(*v),
        Seq(vs) => {
            let mut out = Vec::with_capacity(vs.len());
            for v in vs {
                out.push(sv_to_cv(v)?);
            }
            Ok(Value::Array(out))
        }
        Map(m) => {
            let mut out = Vec::with_capacity(m.len());
            for (k, v) in m {
                out.push((sv_to_cv(k)?, sv_to_cv(v)?));
            }
            Ok(Value::Map(out))
        }
    }
}

fn encode_payload<T: Serialize>(value: &T) -> Result<Value, ciborium::ser::Error<io::Error>> {
    let sv = to_value(value).map_err(|e| {
        ciborium::ser::Error::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    })?;
    sv_to_cv(sv)
        .map_err(|e| ciborium::ser::Error::Io(io::Error::new(io::ErrorKind::InvalidData, e)))
}

fn decode_payload<T: serde::de::DeserializeOwned>(
    value: Value,
) -> Result<T, ciborium::de::Error<io::Error>> {
    let sv = cv_to_sv(value).map_err(|e| ciborium::de::Error::Semantic(None, e.to_string()))?;
    T::deserialize(sv).map_err(|e| ciborium::de::Error::Semantic(None, e.to_string()))
}

fn cv_to_sv(val: Value) -> Result<SerdeValue, String> {
    match val {
        Value::Bool(b) => Ok(SerdeValue::Bool(b)),
        Value::Null => Ok(SerdeValue::Unit),
        Value::Integer(i) => {
            let n: i128 = i.into();
            if n >= 0 {
                if let Ok(v) = u64::try_from(n) {
                    return Ok(SerdeValue::U64(v));
                }
            }
            if let Ok(v) = i64::try_from(n) {
                return Ok(SerdeValue::I64(v));
            }
            Err("integer out of range for serde_value".into())
        }
        Value::Float(f) => Ok(SerdeValue::F64(f)),
        Value::Text(s) => Ok(SerdeValue::String(s)),
        Value::Bytes(b) => Ok(SerdeValue::Bytes(b)),
        Value::Array(vs) => {
            let mut out = Vec::with_capacity(vs.len());
            for v in vs {
                out.push(cv_to_sv(v)?);
            }
            Ok(SerdeValue::Seq(out))
        }
        Value::Map(entries) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in entries {
                map.insert(cv_to_sv(k)?, cv_to_sv(v)?);
            }
            Ok(SerdeValue::Map(map))
        }
        Value::Tag(_, _) => Err("tags not supported in serde conversion".into()),
        _ => Err("unsupported value".into()),
    }
}

/// Protocol magic constant "JIT!".
pub const MAGIC: [u8; 4] = [0x4a, 0x49, 0x54, 0x21];
/// Wire protocol version (big-endian u16) – JS-ABI v1.0 => 0x0001.
pub const VERSION: u16 = 0x0001;
/// Reserved flags (set to zero for v1).
pub const FLAGS: u16 = 0x0000;

/// Encode to canonical CBOR bytes.
pub fn to_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
    let val = encode_payload(value)?;
    encode_value(&val).map_err(|e| {
        ciborium::ser::Error::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    })
}

/// Decode from CBOR bytes with strict canonical validation.
pub fn from_cbor<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, ciborium::de::Error<std::io::Error>> {
    let val = decode_value(bytes).map_err(|e| {
        ciborium::de::Error::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    })?;
    decode_payload(val)
}

/// A full JS-ABI packet (header + payload + checksum).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    /// Raw header (12 bytes).
    pub header: [u8; 12],
    /// Canonical CBOR payload bytes.
    pub payload: Vec<u8>,
    /// blake3 checksum over header||payload.
    pub checksum: [u8; 32],
}

impl Packet {
    /// Build a packet from a canonical CBOR payload.
    pub fn from_payload(payload: Vec<u8>) -> Self {
        let mut header = [0u8; 12];
        header[0..4].copy_from_slice(&MAGIC);
        header[4..6].copy_from_slice(&VERSION.to_be_bytes());
        header[6..8].copy_from_slice(&FLAGS.to_be_bytes());
        header[8..12].copy_from_slice(&(payload.len() as u32).to_be_bytes());

        let mut hasher = Hasher::new();
        hasher.update(&header);
        hasher.update(&payload);
        let checksum = *hasher.finalize().as_bytes();

        Packet {
            header,
            payload,
            checksum,
        }
    }

    /// Encode an `OpEnvelope` into a full packet byte vector.
    pub fn encode_envelope<P: Serialize>(
        env: &OpEnvelope<P>,
    ) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
        let payload = to_cbor(env)?;
        let packet = Packet::from_payload(payload);
        let mut out =
            Vec::with_capacity(packet.header.len() + packet.payload.len() + packet.checksum.len());
        out.extend_from_slice(&packet.header);
        out.extend_from_slice(&packet.payload);
        out.extend_from_slice(&packet.checksum);
        Ok(out)
    }

    /// Decode a packet from a byte slice, returning the envelope and bytes consumed.
    pub fn decode_envelope<P: serde::de::DeserializeOwned>(
        bytes: &[u8],
    ) -> Result<(OpEnvelope<P>, usize), ciborium::de::Error<std::io::Error>> {
        if bytes.len() < 12 + 32 {
            return Err(ciborium::de::Error::Semantic(
                None,
                "incomplete packet".into(),
            ));
        }
        if bytes[0..4] != MAGIC {
            return Err(ciborium::de::Error::Semantic(None, "bad magic".into()));
        }
        let version = u16::from_be_bytes([bytes[4], bytes[5]]);
        if version != VERSION {
            return Err(ciborium::de::Error::Semantic(
                None,
                "unsupported version".into(),
            ));
        }
        let len = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        if bytes.len() < 12 + len + 32 {
            return Err(ciborium::de::Error::Semantic(
                None,
                "incomplete payload".into(),
            ));
        }
        let header = &bytes[0..12];
        let payload = &bytes[12..12 + len];
        let checksum = &bytes[12 + len..12 + len + 32];

        // Verify checksum
        let mut hasher = Hasher::new();
        hasher.update(header);
        hasher.update(payload);
        let expect = hasher.finalize();
        if expect.as_bytes() != checksum {
            return Err(ciborium::de::Error::Semantic(
                None,
                "checksum mismatch".into(),
            ));
        }

        let env: OpEnvelope<P> = from_cbor(payload)?;
        Ok((env, 12 + len + 32))
    }
}

/// Encode a `Message` into a packet with the provided logical timestamp.
pub fn encode_message(
    msg: Message,
    ts: u64,
) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
    let (op, payload) = match &msg {
        Message::Handshake(p) => ("handshake", encode_payload(p)?),
        Message::HandshakeAck(p) => ("handshake_ack", encode_payload(p)?),
        Message::Error(p) => ("error", encode_payload(p)?),
        Message::SubscribeRmg { rmg_id } => (
            "subscribe_rmg",
            encode_payload(&SubscribeRmgPayload { rmg_id: *rmg_id })?,
        ),
        Message::RmgStream { rmg_id, frame } => (
            "rmg_stream",
            encode_payload(&RmgStreamPayload {
                rmg_id: *rmg_id,
                frame: frame.clone(),
            })?,
        ),
        Message::Notification(n) => ("notification", encode_payload(n)?),
    };

    let env = OpEnvelope {
        op: op.to_string(),
        ts,
        payload,
    };
    Packet::encode_envelope(&env)
}

/// Decode bytes into (Message, ts, bytes_consumed).
pub fn decode_message(
    bytes: &[u8],
) -> Result<(Message, u64, usize), ciborium::de::Error<std::io::Error>> {
    let (env, used) = Packet::decode_envelope::<Value>(bytes)?;
    let ts = env.ts;
    let msg = match env.op.as_str() {
        "handshake" => Message::Handshake(decode_payload(env.payload)?),
        "handshake_ack" => Message::HandshakeAck(decode_payload(env.payload)?),
        "error" => Message::Error(decode_payload(env.payload)?),
        "subscribe_rmg" => {
            let p: SubscribeRmgPayload = decode_payload(env.payload)?;
            Message::SubscribeRmg { rmg_id: p.rmg_id }
        }
        "rmg_stream" => {
            let p: RmgStreamPayload = decode_payload(env.payload)?;
            Message::RmgStream {
                rmg_id: p.rmg_id,
                frame: p.frame,
            }
        }
        "notification" => Message::Notification(decode_payload(env.payload)?),
        other => {
            return Err(ciborium::de::Error::Semantic(
                None,
                format!("unknown op {other}"),
            ));
        }
    };
    Ok((msg, ts, used))
}

// --- Unit tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorPayload, HandshakePayload};
    use hex::FromHex;
    use std::collections::BTreeMap;

    fn hex_to_vec(s: &str) -> Vec<u8> {
        let s_clean: String = s.split_whitespace().collect();
        Vec::from_hex(s_clean).unwrap()
    }

    #[test]
    fn canonical_handshake_payload_matches_vector() {
        let payload = HandshakePayload {
            client_version: 1,
            capabilities: vec!["compression:zstd".into(), "stream:subgraph".into()],
            agent_id: Some("example-agent".into()),
            session_meta: None,
        };
        let env = OpEnvelope {
            op: "handshake".into(),
            ts: 0,
            payload,
        };
        let bytes = to_cbor(&env).unwrap();
        // Vector from ADR-0013 Appendix A (113 bytes)
        let expected_hex = "a3 62 6f 70 69 68 61 6e 64 73 68 61 6b 65 62 74 73 00 67 70 61 79 6c 6f 61 64 a4 68 61 67 65 6e 74 5f 69 64 6d 65 78 61 6d 70 6c 65 2d 61 67 65 6e 74 6c 63 61 70 61 62 69 6c 69 74 69 65 73 82 70 63 6f 6d 70 72 65 73 73 69 6f 6e 3a 7a 73 74 64 6f 73 74 72 65 61 6d 3a 73 75 62 67 72 61 70 68 6c 73 65 73 73 69 6f 6e 5f 6d 65 74 61 f6 6e 63 6c 69 65 6e 74 5f 76 65 72 73 69 6f 6e 01";
        let expected = hex_to_vec(expected_hex);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn canonical_error_payload_matches_vector() {
        let payload = ErrorPayload {
            code: 3,
            name: "E_BAD_PAYLOAD".into(),
            message: "Invalid CBOR payload".into(),
            details: Some(
                encode_payload(&BTreeMap::from([(
                    "hint".to_string(),
                    Value::Text("Check canonical encoding".into()),
                )]))
                .unwrap(),
            ),
        };
        let env = OpEnvelope {
            op: "error".into(),
            ts: 42,
            payload,
        };
        let bytes = to_cbor(&env).unwrap();
        let expected_hex = "a3 62 6f 70 65 65 72 72 6f 72 62 74 73 18 2a 67 70 61 79 6c 6f 61 64 a4 64 63 6f 64 65 03 64 6e 61 6d 65 6d 45 5f 42 41 44 5f 50 41 59 4c 4f 41 44 67 64 65 74 61 69 6c 73 a1 64 68 69 6e 74 78 18 43 68 65 63 6b 20 63 61 6e 6f 6e 69 63 61 6c 20 65 6e 63 6f 64 69 6e 67 67 6d 65 73 73 61 67 65 74 49 6e 76 61 6c 69 64 20 43 42 4f 52 20 70 61 79 6c 6f 61 64";
        let expected = hex_to_vec(expected_hex);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn broken_tv3_missing_value_length_is_rejected() {
        // Spec typo (missing 0x74 before the message value)
        let broken_hex = "a3 62 6f 70 65 65 72 72 6f 72 62 74 73 18 2a 67 70 61 79 6c 6f 61 64 a4 64 63 6f 64 65 03 64 6e 61 6d 65 6d 45 5f 42 41 44 5f 50 41 59 4c 4f 41 44 67 64 65 74 61 69 6c 73 a1 64 68 69 6e 74 78 18 43 68 65 63 6b 20 63 61 6e 6f 6e 69 63 61 6c 20 65 6e 63 6f 64 69 6e 67 67 6d 65 73 73 61 67 65 49 6e 76 61 6c 69 64 20 43 42 4f 52 20 70 61 79 6c 6f 61 64";
        let broken = hex_to_vec(broken_hex);
        let res = from_cbor::<Value>(&broken);
        assert!(res.is_err());
    }
}
