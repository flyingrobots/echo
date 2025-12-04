// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! CBOR framing helpers for session messages.

use crate::Message;
use serde::{de::Error as DeError, Deserialize, Serialize};

/// Serialize a message to CBOR bytes.
pub fn to_cbor(msg: &Message) -> Result<Vec<u8>, serde_cbor::Error> {
    serde_cbor::to_vec(msg)
}

/// Deserialize a message from CBOR bytes.
pub fn from_cbor(bytes: &[u8]) -> Result<Message, serde_cbor::Error> {
    serde_cbor::from_slice(bytes)
}

/// A framed packet: len (u32, BE) + CBOR payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    /// Raw CBOR payload.
    pub bytes: Vec<u8>,
}

impl Packet {
    /// Encode a `Message` into a length-prefixed CBOR packet.
    pub fn encode(msg: &Message) -> Result<Vec<u8>, serde_cbor::Error> {
        let body = to_cbor(msg)?;
        let mut out = Vec::with_capacity(4 + body.len());
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Decode a packet from the provided buffer, returning the message and bytes consumed.
    pub fn decode(stream: &[u8]) -> Result<(Message, usize), serde_cbor::Error> {
        if stream.len() < 4 {
            return Err(<serde_cbor::Error as DeError>::custom("incomplete length"));
        }
        let len = u32::from_be_bytes([stream[0], stream[1], stream[2], stream[3]]) as usize;
        if stream.len() < 4 + len {
            return Err(<serde_cbor::Error as DeError>::custom("incomplete frame"));
        }
        let msg = from_cbor(&stream[4..4 + len])?;
        Ok((msg, 4 + len))
    }
}
