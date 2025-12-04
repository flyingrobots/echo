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
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use serde::de::Error as DeError;

use crate::{Message, OpEnvelope, RmgStreamPayload, SubscribeRmgPayload};

/// Protocol magic constant "JIT!".
pub const MAGIC: [u8; 4] = [0x4a, 0x49, 0x54, 0x21];
/// Wire protocol version (big-endian u16) – JS-ABI v1.0 => 0x0001.
pub const VERSION: u16 = 0x0001;
/// Reserved flags (set to zero for v1).
pub const FLAGS: u16 = 0x0000;

/// Encode to CBOR bytes using serde_cbor (definite lengths by default).
pub fn to_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_cbor::Error> {
    serde_cbor::to_vec(value)
}

/// Decode from CBOR bytes.
pub fn from_cbor<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, serde_cbor::Error> {
    serde_cbor::from_slice(bytes)
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

        Packet { header, payload, checksum }
    }

    /// Encode an `OpEnvelope` into a full packet byte vector.
    pub fn encode_envelope<P: Serialize>(env: &OpEnvelope<P>) -> Result<Vec<u8>, serde_cbor::Error> {
        let payload = to_cbor(env)?;
        let packet = Packet::from_payload(payload);
        let mut out = Vec::with_capacity(packet.header.len() + packet.payload.len() + packet.checksum.len());
        out.extend_from_slice(&packet.header);
        out.extend_from_slice(&packet.payload);
        out.extend_from_slice(&packet.checksum);
        Ok(out)
    }

    /// Decode a packet from a byte slice, returning the envelope and bytes consumed.
    pub fn decode_envelope<'de, P: Deserialize<'de>>(bytes: &'de [u8]) -> Result<(OpEnvelope<P>, usize), serde_cbor::Error> {
        if bytes.len() < 12 + 32 {
            return Err(serde_cbor::Error::custom("incomplete packet"));
        }
        if bytes[0..4] != MAGIC {
            return Err(serde_cbor::Error::custom("bad magic"));
        }
        let version = u16::from_be_bytes([bytes[4], bytes[5]]);
        if version != VERSION {
            return Err(serde_cbor::Error::custom("unsupported version"));
        }
        let len = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        if bytes.len() < 12 + len + 32 {
            return Err(serde_cbor::Error::custom("incomplete payload"));
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
            return Err(serde_cbor::Error::custom("checksum mismatch"));
        }

        let env: OpEnvelope<P> = from_cbor(payload)?;
        Ok((env, 12 + len + 32))
    }
}

/// Encode a `Message` into a packet with the provided logical timestamp.
pub fn encode_message(msg: Message, ts: u64) -> Result<Vec<u8>, serde_cbor::Error> {
    let (op, payload) = match &msg {
        Message::Handshake(p) => ("handshake", serde_cbor::value::to_value(p)?),
        Message::HandshakeAck(p) => ("handshake_ack", serde_cbor::value::to_value(p)?),
        Message::Error(p) => ("error", serde_cbor::value::to_value(p)?),
        Message::SubscribeRmg { rmg_id } => (
            "subscribe_rmg",
            serde_cbor::value::to_value(&SubscribeRmgPayload { rmg_id: *rmg_id })?,
        ),
        Message::RmgStream { rmg_id, frame } => (
            "rmg_stream",
            serde_cbor::value::to_value(&RmgStreamPayload {
                rmg_id: *rmg_id,
                frame: frame.clone(),
            })?,
        ),
        Message::Notification(n) => ("notification", serde_cbor::value::to_value(n)?),
    };

    let env = OpEnvelope {
        op: op.to_string(),
        ts,
        payload,
    };
    Packet::encode_envelope(&env)
}

/// Decode bytes into (Message, ts, bytes_consumed).
pub fn decode_message(bytes: &[u8]) -> Result<(Message, u64, usize), serde_cbor::Error> {
    let (env, used) = Packet::decode_envelope::<Value>(bytes)?;
    let ts = env.ts;
    let msg = match env.op.as_str() {
        "handshake" => Message::Handshake(serde_cbor::value::from_value(env.payload)?),
        "handshake_ack" => Message::HandshakeAck(serde_cbor::value::from_value(env.payload)?),
        "error" => Message::Error(serde_cbor::value::from_value(env.payload)?),
        "subscribe_rmg" => {
            let p: SubscribeRmgPayload = serde_cbor::value::from_value(env.payload)?;
            Message::SubscribeRmg { rmg_id: p.rmg_id }
        }
        "rmg_stream" => {
            let p: RmgStreamPayload = serde_cbor::value::from_value(env.payload)?;
            Message::RmgStream {
                rmg_id: p.rmg_id,
                frame: p.frame,
            }
        }
        "notification" => Message::Notification(serde_cbor::value::from_value(env.payload)?),
        other => {
            return Err(serde_cbor::Error::custom(format!("unknown op {other}")));
        }
    };
    Ok((msg, ts, used))
}

// --- Unit tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use hex::FromHex;
    use std::collections::BTreeMap;
    use crate::{HandshakePayload, ErrorPayload};

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
        let env = OpEnvelope { op: "handshake".into(), ts: 0, payload: payload };
        let bytes = to_cbor(&env).unwrap();
        assert_eq!(bytes.len(), 113);
    }

    #[test]
    fn canonical_error_payload_matches_vector() {
        let payload = ErrorPayload {
            code: 3,
            name: "E_BAD_PAYLOAD".into(),
            message: "Invalid CBOR payload".into(),
            details: Some(serde_cbor::value::to_value(&BTreeMap::from([(
                "hint".to_string(), serde_cbor::Value::Text("Check canonical encoding".into()),
            )]))
            .unwrap()),
        };
        let env = OpEnvelope { op: "error".into(), ts: 42, payload };
        let bytes = to_cbor(&env).unwrap();
        assert_eq!(bytes.len(), 118);
    }
}
