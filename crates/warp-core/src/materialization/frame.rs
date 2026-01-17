// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Binary frame encoding for materialized channel data.
//!
//! All materialization data is transmitted as length-prefixed binary frames.
//! No JSON. No strings at the wire boundary.
//!
//! # Frame Format
//!
//! ```text
//! MaterializationFrame (variable length):
//!   magic[4]      = "MBUS" (0x4D, 0x42, 0x55, 0x53)
//!   version[2]    = 0x0001 (little-endian)
//!   reserved[2]   = 0x0000 (little-endian, future use)
//!   length[4]     = payload byte length (little-endian)
//!   payload[len]  = channel_id[32] || data[len-32]
//! ```
//!
//! All multi-byte integers are **little-endian**.

use super::channel::ChannelId;
use crate::ident::TypeId;

/// Frame magic bytes: "MBUS" in ASCII.
pub const FRAME_MAGIC: [u8; 4] = [0x4D, 0x42, 0x55, 0x53];

/// Frame version (v1).
pub const FRAME_VERSION: u16 = 0x0001;

/// Header size in bytes: magic(4) + version(2) + reserved(2) + length(4) = 12.
pub const HEADER_SIZE: usize = 12;

/// Minimum payload size: `channel_id` (32 bytes).
pub const MIN_PAYLOAD_SIZE: usize = 32;

/// A single materialization frame ready for transport.
///
/// # Wire-Format Limitations
///
/// The frame's `payload_len` field is stored as a [`u32`] in the wire format
/// (see [`HEADER_SIZE`] for header layout). This means the total payload
/// (`channel_id` + `data`) must fit in 32 bits:
///
/// - `channel_id` is fixed at 32 bytes
/// - `data.len()` must be ≤ `u32::MAX - 32` (approximately 4 GiB minus header overhead)
///
/// Frames exceeding this limit will have their length silently truncated during
/// encoding. In debug builds, [`encode()`](Self::encode) asserts this constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationFrame {
    /// Channel this data belongs to.
    pub channel: ChannelId,
    /// Finalized channel data (format depends on channel policy).
    pub data: Vec<u8>,
}

impl MaterializationFrame {
    /// Creates a new frame.
    #[inline]
    pub fn new(channel: ChannelId, data: Vec<u8>) -> Self {
        Self { channel, data }
    }

    /// Encodes this frame to bytes.
    ///
    /// # Format
    ///
    /// Returns a buffer with:
    /// - Header ([`HEADER_SIZE`] = 12 bytes): [`FRAME_MAGIC`] + version + reserved + length
    /// - `channel_id` (32 bytes)
    /// - `data` (variable length)
    ///
    /// # Wire-Format Limitation
    ///
    /// The `payload_len` field is encoded as a [`u32`], so `data.len()` must be
    /// ≤ `u32::MAX - 32` (~4 GiB). Larger payloads will have their length truncated.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `payload_len` exceeds [`u32::MAX`].
    #[allow(clippy::cast_possible_truncation)]
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = 32 + self.data.len();
        debug_assert!(
            u32::try_from(payload_len).is_ok(),
            "payload_len ({payload_len}) exceeds u32::MAX; frame would be truncated"
        );
        let total_len = HEADER_SIZE + payload_len;

        let mut buf = Vec::with_capacity(total_len);

        // Header
        buf.extend_from_slice(&FRAME_MAGIC);
        buf.extend_from_slice(&FRAME_VERSION.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // reserved
        buf.extend_from_slice(&(payload_len as u32).to_le_bytes());

        // Payload
        buf.extend_from_slice(&self.channel.0);
        buf.extend_from_slice(&self.data);

        buf
    }

    /// Decodes a frame from bytes.
    ///
    /// Returns `None` if the bytes are malformed or too short.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER_SIZE {
            return None;
        }

        // Check magic
        if bytes[0..4] != FRAME_MAGIC {
            return None;
        }

        // Check version
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != FRAME_VERSION {
            return None;
        }

        // Skip reserved (bytes 6..8)

        // Read payload length
        let payload_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;

        if payload_len < MIN_PAYLOAD_SIZE {
            return None;
        }

        let expected_total = HEADER_SIZE + payload_len;
        if bytes.len() < expected_total {
            return None;
        }

        // Extract channel_id
        let channel_bytes: [u8; 32] = bytes[HEADER_SIZE..HEADER_SIZE + 32].try_into().ok()?;
        let channel = TypeId(channel_bytes);

        // Extract data
        let data = bytes[HEADER_SIZE + 32..expected_total].to_vec();

        Some(Self { channel, data })
    }
}

/// Encodes multiple frames into a single byte buffer (concatenated).
pub fn encode_frames(frames: &[MaterializationFrame]) -> Vec<u8> {
    let total_size: usize = frames.iter().map(|f| HEADER_SIZE + 32 + f.data.len()).sum();
    let mut buf = Vec::with_capacity(total_size);
    for frame in frames {
        buf.extend_from_slice(&frame.encode());
    }
    buf
}

/// Decodes multiple frames from a concatenated byte buffer.
///
/// Returns `None` if any frame is malformed.
pub fn decode_frames(mut bytes: &[u8]) -> Option<Vec<MaterializationFrame>> {
    let mut frames = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < HEADER_SIZE {
            return None;
        }

        // Read payload length to determine frame size
        let payload_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let frame_size = HEADER_SIZE + payload_len;

        if bytes.len() < frame_size {
            return None;
        }

        let frame = MaterializationFrame::decode(&bytes[..frame_size])?;
        frames.push(frame);
        bytes = &bytes[frame_size..];
    }
    Some(frames)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_channel() -> ChannelId {
        super::super::channel::make_channel_id("test:channel")
    }

    #[test]
    fn encode_decode_roundtrip() {
        let frame = MaterializationFrame::new(test_channel(), vec![1, 2, 3, 4, 5]);
        let encoded = frame.encode();
        let decoded = MaterializationFrame::decode(&encoded).expect("decode should succeed");
        assert_eq!(frame, decoded);
    }

    #[test]
    fn encode_decode_empty_data() {
        let frame = MaterializationFrame::new(test_channel(), vec![]);
        let encoded = frame.encode();
        let decoded = MaterializationFrame::decode(&encoded).expect("decode empty data");
        assert_eq!(frame, decoded);
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut bad = MaterializationFrame::new(test_channel(), vec![1, 2, 3]).encode();
        bad[0] = 0xFF; // corrupt magic
        assert!(MaterializationFrame::decode(&bad).is_none());
    }

    #[test]
    fn decode_rejects_bad_version() {
        let mut bad = MaterializationFrame::new(test_channel(), vec![1, 2, 3]).encode();
        bad[4] = 0xFF; // corrupt version
        assert!(MaterializationFrame::decode(&bad).is_none());
    }

    #[test]
    fn decode_rejects_truncated() {
        let frame = MaterializationFrame::new(test_channel(), vec![1, 2, 3, 4, 5]);
        let encoded = frame.encode();
        let truncated = &encoded[..encoded.len() - 1];
        assert!(MaterializationFrame::decode(truncated).is_none());
    }

    #[test]
    fn multi_frame_roundtrip() {
        let ch1 = super::super::channel::make_channel_id("channel:one");
        let ch2 = super::super::channel::make_channel_id("channel:two");

        let frames = vec![
            MaterializationFrame::new(ch1, vec![1, 2, 3]),
            MaterializationFrame::new(ch2, vec![4, 5, 6, 7, 8]),
        ];

        let encoded = encode_frames(&frames);
        let decoded = decode_frames(&encoded).expect("decode multi-frame");

        assert_eq!(frames, decoded);
    }

    #[test]
    fn header_size_correct() {
        let frame = MaterializationFrame::new(test_channel(), vec![]);
        let encoded = frame.encode();
        // Header (12) + channel_id (32) + data (0) = 44
        assert_eq!(encoded.len(), HEADER_SIZE + 32);
    }
}
