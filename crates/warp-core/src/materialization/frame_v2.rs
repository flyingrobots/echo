// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! MBUS v2 frame encoding for cursor-addressed truth delivery.
//!
//! MBUS v2 extends v1 with cursor context so clients receive authoritative
//! truth frames stamped with session, cursor, worldline, warp, tick, and
//! commit hash. Clients are "dumb": they replace render state, never diff.
//!
//! # Frame Format
//!
//! ```text
//! V2Packet (variable length):
//!   magic[4]          = "MBUS" (0x4D, 0x42, 0x55, 0x53)
//!   version[2]        = 0x0002 (little-endian)
//!   reserved[2]       = 0x0000 (little-endian, future use)
//!   payload_len[4]    = payload byte length (little-endian)
//!   payload[len]:
//!     session_id[32]
//!     cursor_id[32]
//!     worldline_id[32]
//!     warp_id[32]
//!     tick[8]         = u64 little-endian
//!     commit_hash[32]
//!     entry_count[4]  = u32 little-endian
//!     entries[entry_count]:
//!       channel_id[32]
//!       value_hash[32]  = blake3(value)
//!       value_len[4]    = u32 little-endian
//!       value[value_len]
//! ```
//!
//! All multi-byte integers are **little-endian**.
//!
//! # Compatibility
//!
//! - V1 decoders reject V2 packets (version mismatch)
//! - V2 decoders reject V1 packets (version mismatch)
//! - Packets can be concatenated; use [`decode_v2_packets`] for multi-packet streams

use super::channel::ChannelId;
use crate::ident::{Hash, TypeId, WarpId};
use core::fmt;

/// Error returned when encoding a v2 packet fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// The computed payload size exceeds the maximum encodable size (`u32::MAX`).
    PayloadTooLarge {
        /// The actual payload size in bytes.
        actual: usize,
        /// The maximum allowed payload size.
        max: usize,
    },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayloadTooLarge { actual, max } => {
                write!(
                    f,
                    "payload size ({actual} bytes) exceeds maximum encodable size ({max} bytes)"
                )
            }
        }
    }
}

impl std::error::Error for EncodeError {}

/// Error returned when decoding a v2 packet fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The input buffer is shorter than the minimum header size.
    TruncatedHeader,
    /// The magic bytes do not match "MBUS".
    BadMagic,
    /// The version field is not `0x0002`.
    UnsupportedVersion {
        /// The version found in the packet.
        actual: u16,
    },
    /// The declared payload length is smaller than the minimum payload size.
    PayloadTooSmall {
        /// The declared payload length.
        declared: usize,
    },
    /// The input buffer is shorter than header + declared payload length.
    TruncatedPacket {
        /// Bytes required (header + payload).
        expected: usize,
        /// Bytes actually available.
        actual: usize,
    },
    /// The entry count or entry data is inconsistent with the payload size.
    InvalidEntryCount,
    /// An individual entry is truncated (missing channel/hash/len/value bytes).
    TruncatedEntry {
        /// Zero-based index of the entry that failed to decode.
        index: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHeader => write!(
                f,
                "input shorter than v2 header size ({HEADER_SIZE_V2} bytes)"
            ),
            Self::BadMagic => write!(f, "magic bytes do not match \"MBUS\""),
            Self::UnsupportedVersion { actual } => {
                write!(
                    f,
                    "unsupported version 0x{actual:04X} (expected 0x{FRAME_VERSION_V2:04X})"
                )
            }
            Self::PayloadTooSmall { declared } => {
                write!(f, "declared payload length ({declared}) is less than minimum ({MIN_PAYLOAD_SIZE_V2})")
            }
            Self::TruncatedPacket { expected, actual } => {
                write!(
                    f,
                    "input too short: need {expected} bytes but only {actual} available"
                )
            }
            Self::InvalidEntryCount => write!(f, "entry count inconsistent with payload"),
            Self::TruncatedEntry { index } => write!(f, "entry {index} is truncated"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Frame magic bytes: "MBUS" in ASCII (shared with v1).
pub const FRAME_MAGIC: [u8; 4] = [0x4D, 0x42, 0x55, 0x53];

/// Frame version for v2.
pub const FRAME_VERSION_V2: u16 = 0x0002;

/// V2 header size: magic(4) + version(2) + reserved(2) + length(4) = 12.
pub const HEADER_SIZE_V2: usize = 12;

/// V2 receipt size: session(32) + cursor(32) + worldline(32) + warp(32) + tick(8) + commit(32) = 168.
pub const RECEIPT_SIZE_V2: usize = 168;

/// Minimum payload size: receipt(168) + `entry_count`(4) = 172.
pub const MIN_PAYLOAD_SIZE_V2: usize = RECEIPT_SIZE_V2 + 4;

/// Cursor receipt header for v2 packets.
///
/// Stamps every truth frame with the cursor context so clients know exactly
/// which point in time/space the values represent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2PacketHeader {
    /// Session that requested this truth.
    pub session_id: Hash,
    /// Cursor providing the viewpoint.
    pub cursor_id: Hash,
    /// Worldline being viewed.
    pub worldline_id: Hash,
    /// Warp instance within the worldline.
    pub warp_id: WarpId,
    /// Tick number within the worldline.
    pub tick: u64,
    /// Commit hash at this tick (for verification).
    pub commit_hash: Hash,
}

/// A single channel entry within a v2 packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2Entry {
    /// Channel this value belongs to.
    pub channel: ChannelId,
    /// Blake3 hash of the value bytes.
    pub value_hash: Hash,
    /// Finalized channel value.
    pub value: Vec<u8>,
}

/// A complete MBUS v2 packet with header and entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2Packet {
    /// Cursor context for this packet.
    pub header: V2PacketHeader,
    /// Channel entries in this packet.
    pub entries: Vec<V2Entry>,
}

impl V2Packet {
    /// Creates a new v2 packet.
    #[inline]
    pub fn new(header: V2PacketHeader, entries: Vec<V2Entry>) -> Self {
        Self { header, entries }
    }
}

/// Encodes a v2 packet to bytes.
///
/// # Wire-Format Limitation
///
/// The `payload_len` field is encoded as a [`u32`], so total payload must fit.
/// Returns [`EncodeError::PayloadTooLarge`] if payload exceeds [`u32::MAX`].
///
/// # Errors
///
/// Returns [`EncodeError::PayloadTooLarge`] if the computed payload size exceeds
/// `u32::MAX` bytes.
// SAFETY: All `as u32` casts below are guarded by the `u32::try_from(payload_len)` check
// at the top of the function body, which ensures the total payload (and therefore each
// individual component length) fits in a u32.
#[allow(clippy::cast_possible_truncation)]
pub fn encode_v2_packet(
    header: &V2PacketHeader,
    entries: &[V2Entry],
) -> Result<Vec<u8>, EncodeError> {
    // Calculate payload size
    let entries_size: usize = entries
        .iter()
        .map(|e| 32 + 32 + 4 + e.value.len()) // channel + hash + len + value
        .sum();
    let payload_len = RECEIPT_SIZE_V2 + 4 + entries_size; // receipt + entry_count + entries

    if u32::try_from(payload_len).is_err() {
        return Err(EncodeError::PayloadTooLarge {
            actual: payload_len,
            max: u32::MAX as usize,
        });
    }

    let total_len = HEADER_SIZE_V2 + payload_len;
    let mut buf = Vec::with_capacity(total_len);

    // Header
    buf.extend_from_slice(&FRAME_MAGIC);
    buf.extend_from_slice(&FRAME_VERSION_V2.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes()); // reserved
    buf.extend_from_slice(&(payload_len as u32).to_le_bytes());

    // Receipt
    buf.extend_from_slice(&header.session_id);
    buf.extend_from_slice(&header.cursor_id);
    buf.extend_from_slice(&header.worldline_id);
    buf.extend_from_slice(&header.warp_id.0);
    buf.extend_from_slice(&header.tick.to_le_bytes());
    buf.extend_from_slice(&header.commit_hash);

    // Entry count
    buf.extend_from_slice(&(entries.len() as u32).to_le_bytes());

    // Entries
    for entry in entries {
        buf.extend_from_slice(&entry.channel.0);
        buf.extend_from_slice(&entry.value_hash);
        buf.extend_from_slice(&(entry.value.len() as u32).to_le_bytes());
        buf.extend_from_slice(&entry.value);
    }

    Ok(buf)
}

/// Decodes a single v2 packet from bytes.
///
/// # Errors
///
/// Returns [`DecodeError`] describing the specific failure:
/// - [`DecodeError::TruncatedHeader`] — input shorter than header
/// - [`DecodeError::BadMagic`] — magic bytes mismatch
/// - [`DecodeError::UnsupportedVersion`] — version is not 0x0002
/// - [`DecodeError::PayloadTooSmall`] — declared payload smaller than minimum
/// - [`DecodeError::TruncatedPacket`] — input shorter than header + payload
/// - [`DecodeError::InvalidEntryCount`] — entry count field unreadable
/// - [`DecodeError::TruncatedEntry`] — an entry is incomplete
// NOTE: Kept monolithic — match arms parse sequential fields from a single byte slice;
// splitting would require threading offset state through sub-functions.
#[allow(clippy::too_many_lines)]
#[allow(clippy::cast_possible_truncation)]
pub fn decode_v2_packet(bytes: &[u8]) -> Result<V2Packet, DecodeError> {
    if bytes.len() < HEADER_SIZE_V2 {
        return Err(DecodeError::TruncatedHeader);
    }

    // Check magic
    if bytes[0..4] != FRAME_MAGIC {
        return Err(DecodeError::BadMagic);
    }

    // Check version (must be v2)
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != FRAME_VERSION_V2 {
        return Err(DecodeError::UnsupportedVersion { actual: version });
    }

    // Skip reserved (bytes 6..8)

    // Read payload length
    let payload_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;

    if payload_len < MIN_PAYLOAD_SIZE_V2 {
        return Err(DecodeError::PayloadTooSmall {
            declared: payload_len,
        });
    }

    let expected_total = HEADER_SIZE_V2 + payload_len;
    if bytes.len() < expected_total {
        return Err(DecodeError::TruncatedPacket {
            expected: expected_total,
            actual: bytes.len(),
        });
    }

    let payload = &bytes[HEADER_SIZE_V2..expected_total];
    let mut offset = 0;

    // Read receipt fields
    let session_id: Hash =
        payload[offset..offset + 32]
            .try_into()
            .map_err(|_| DecodeError::TruncatedPacket {
                expected: expected_total,
                actual: bytes.len(),
            })?;
    offset += 32;

    let cursor_id: Hash =
        payload[offset..offset + 32]
            .try_into()
            .map_err(|_| DecodeError::TruncatedPacket {
                expected: expected_total,
                actual: bytes.len(),
            })?;
    offset += 32;

    let worldline_id: Hash =
        payload[offset..offset + 32]
            .try_into()
            .map_err(|_| DecodeError::TruncatedPacket {
                expected: expected_total,
                actual: bytes.len(),
            })?;
    offset += 32;

    let warp_id_bytes: Hash =
        payload[offset..offset + 32]
            .try_into()
            .map_err(|_| DecodeError::TruncatedPacket {
                expected: expected_total,
                actual: bytes.len(),
            })?;
    offset += 32;

    let tick = u64::from_le_bytes(payload[offset..offset + 8].try_into().map_err(|_| {
        DecodeError::TruncatedPacket {
            expected: expected_total,
            actual: bytes.len(),
        }
    })?);
    offset += 8;

    let commit_hash: Hash =
        payload[offset..offset + 32]
            .try_into()
            .map_err(|_| DecodeError::TruncatedPacket {
                expected: expected_total,
                actual: bytes.len(),
            })?;
    offset += 32;

    // Read entry count
    let entry_count = u32::from_le_bytes(
        payload[offset..offset + 4]
            .try_into()
            .map_err(|_| DecodeError::InvalidEntryCount)?,
    ) as usize;
    offset += 4;

    // Validate entry_count against remaining payload to prevent OOM from malicious packets.
    // Each entry needs at minimum: channel(32) + hash(32) + len(4) = 68 bytes.
    let remaining = payload.len().saturating_sub(offset);
    let max_possible_entries = remaining / 68;
    if entry_count > max_possible_entries {
        return Err(DecodeError::InvalidEntryCount);
    }

    // Read entries
    let mut entries = Vec::with_capacity(entry_count);
    for i in 0..entry_count {
        if offset + 68 > payload.len() {
            // Need at least channel(32) + hash(32) + len(4)
            return Err(DecodeError::TruncatedEntry { index: i });
        }

        let channel_bytes: Hash = payload[offset..offset + 32]
            .try_into()
            .map_err(|_| DecodeError::TruncatedEntry { index: i })?;
        offset += 32;

        let value_hash: Hash = payload[offset..offset + 32]
            .try_into()
            .map_err(|_| DecodeError::TruncatedEntry { index: i })?;
        offset += 32;

        let value_len = u32::from_le_bytes(
            payload[offset..offset + 4]
                .try_into()
                .map_err(|_| DecodeError::TruncatedEntry { index: i })?,
        ) as usize;
        offset += 4;

        if offset + value_len > payload.len() {
            return Err(DecodeError::TruncatedEntry { index: i });
        }

        let value = payload[offset..offset + value_len].to_vec();
        offset += value_len;

        entries.push(V2Entry {
            channel: TypeId(channel_bytes),
            value_hash,
            value,
        });
    }

    Ok(V2Packet {
        header: V2PacketHeader {
            session_id,
            cursor_id,
            worldline_id,
            warp_id: WarpId(warp_id_bytes),
            tick,
            commit_hash,
        },
        entries,
    })
}

/// Decodes multiple v2 packets from a concatenated byte buffer.
///
/// # Errors
///
/// Returns the [`DecodeError`] from the first malformed packet encountered.
#[allow(clippy::cast_possible_truncation)]
pub fn decode_v2_packets(mut bytes: &[u8]) -> Result<Vec<V2Packet>, DecodeError> {
    let mut packets = Vec::new();

    while !bytes.is_empty() {
        if bytes.len() < HEADER_SIZE_V2 {
            return Err(DecodeError::TruncatedHeader);
        }

        // Read payload length to determine packet size
        let payload_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let packet_size = HEADER_SIZE_V2 + payload_len;

        if bytes.len() < packet_size {
            return Err(DecodeError::TruncatedPacket {
                expected: packet_size,
                actual: bytes.len(),
            });
        }

        // NOTE: decode_v2_packet re-validates magic/version/length on the subslice.
        // This is intentional (defense in depth) — the outer loop only peeks at the
        // payload_len field to find packet boundaries, while the inner decode performs
        // full validation. This keeps decode_v2_packet safe to call standalone.
        let packet = decode_v2_packet(&bytes[..packet_size])?;
        packets.push(packet);
        bytes = &bytes[packet_size..];
    }

    Ok(packets)
}

/// Computes the blake3 hash of a value for use in `V2Entry`.
#[inline]
pub fn compute_value_hash(value: &[u8]) -> Hash {
    blake3::hash(value).into()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::materialization::channel::make_channel_id;
    use crate::materialization::frame::{MaterializationFrame, FRAME_VERSION};

    fn fixed_hash(byte: u8) -> Hash {
        [byte; 32]
    }

    fn test_header() -> V2PacketHeader {
        V2PacketHeader {
            session_id: fixed_hash(0x01),
            cursor_id: fixed_hash(0x02),
            worldline_id: fixed_hash(0x03),
            warp_id: WarpId(fixed_hash(0x04)),
            tick: 42,
            commit_hash: fixed_hash(0x05),
        }
    }

    fn test_entries() -> Vec<V2Entry> {
        vec![
            V2Entry {
                channel: make_channel_id("a"),
                value_hash: compute_value_hash(&[1, 2, 3]),
                value: vec![1, 2, 3],
            },
            V2Entry {
                channel: make_channel_id("b"),
                value_hash: compute_value_hash(&[9]),
                value: vec![9],
            },
        ]
    }

    // T19: mbus_v2_roundtrip_single_packet
    #[test]
    fn mbus_v2_roundtrip_single_packet() {
        // Arrange
        let header = test_header();
        let entries = test_entries();

        // Act
        let encoded = encode_v2_packet(&header, &entries).expect("encode should succeed");
        let decoded = decode_v2_packet(&encoded).expect("decode should succeed");

        // Assert: decoded receipt fields equal original
        assert_eq!(decoded.header.session_id, header.session_id);
        assert_eq!(decoded.header.cursor_id, header.cursor_id);
        assert_eq!(decoded.header.worldline_id, header.worldline_id);
        assert_eq!(decoded.header.warp_id, header.warp_id);
        assert_eq!(decoded.header.tick, header.tick);
        assert_eq!(decoded.header.commit_hash, header.commit_hash);

        // Assert: decoded entries count == 2
        assert_eq!(decoded.entries.len(), 2);

        // Assert: each entry channel/value equals original
        assert_eq!(decoded.entries[0].channel, entries[0].channel);
        assert_eq!(decoded.entries[0].value, entries[0].value);
        assert_eq!(decoded.entries[1].channel, entries[1].channel);
        assert_eq!(decoded.entries[1].value, entries[1].value);

        // Assert: each entry value_hash equals blake3(value)
        assert_eq!(
            decoded.entries[0].value_hash,
            compute_value_hash(&[1, 2, 3])
        );
        assert_eq!(decoded.entries[1].value_hash, compute_value_hash(&[9]));
    }

    // T20: mbus_v1_rejects_v2
    #[test]
    fn mbus_v1_rejects_v2() {
        // Arrange: build a valid v2 packet
        let v2_bytes =
            encode_v2_packet(&test_header(), &test_entries()).expect("encode should succeed");

        // Act: call v1 decoder on v2 bytes
        let result = MaterializationFrame::decode(&v2_bytes);

        // Assert: rejected due to version mismatch
        assert!(result.is_none(), "v1 decoder should reject v2 packet");
    }

    // T21: mbus_v2_rejects_v1
    #[test]
    fn mbus_v2_rejects_v1() {
        // Arrange: build a valid v1 frame
        let channel = make_channel_id("test:channel");
        let v1_frame = MaterializationFrame::new(channel, vec![1, 2, 3, 4]);
        let v1_bytes = v1_frame.encode();

        // Sanity check: v1 should decode with v1 decoder
        assert!(
            MaterializationFrame::decode(&v1_bytes).is_some(),
            "v1 frame should decode with v1 decoder"
        );

        // Act: call v2 decoder on v1 bytes
        let result = decode_v2_packet(&v1_bytes);

        // Assert: rejected due to version mismatch
        assert_eq!(
            result.unwrap_err(),
            DecodeError::UnsupportedVersion {
                actual: FRAME_VERSION
            },
            "v2 decoder should reject v1 packet with UnsupportedVersion"
        );
    }

    // T22: mbus_v2_multi_packet_roundtrip
    #[test]
    fn mbus_v2_multi_packet_roundtrip() {
        // Arrange
        let ch_a = make_channel_id("chA");
        let ch_b = make_channel_id("chB");

        let p1_header = V2PacketHeader {
            session_id: fixed_hash(0x10),
            cursor_id: fixed_hash(0x11),
            worldline_id: fixed_hash(0x12),
            warp_id: WarpId(fixed_hash(0x13)),
            tick: 1,
            commit_hash: fixed_hash(0x14),
        };
        let p1_entries = vec![V2Entry {
            channel: ch_a,
            value_hash: compute_value_hash(&[1]),
            value: vec![1],
        }];

        let p2_header = V2PacketHeader {
            session_id: fixed_hash(0x20),
            cursor_id: fixed_hash(0x21),
            worldline_id: fixed_hash(0x22),
            warp_id: WarpId(fixed_hash(0x23)),
            tick: 2,
            commit_hash: fixed_hash(0x24),
        };
        let p2_entries = vec![
            V2Entry {
                channel: ch_a,
                value_hash: compute_value_hash(&[2]),
                value: vec![2],
            },
            V2Entry {
                channel: ch_b,
                value_hash: compute_value_hash(&[7, 7]),
                value: vec![7, 7],
            },
        ];

        // Act: concatenate encoded packets
        let mut concat_bytes =
            encode_v2_packet(&p1_header, &p1_entries).expect("encode p1 should succeed");
        concat_bytes.extend_from_slice(
            &encode_v2_packet(&p2_header, &p2_entries).expect("encode p2 should succeed"),
        );

        let decoded = decode_v2_packets(&concat_bytes).expect("multi-packet decode should succeed");

        // Assert: returns Vec with len=2
        assert_eq!(decoded.len(), 2);

        // Assert: packet[0] matches P1 exactly
        assert_eq!(decoded[0].header.tick, 1);
        assert_eq!(decoded[0].entries.len(), 1);
        assert_eq!(decoded[0].entries[0].value, vec![1]);

        // Assert: packet[1] matches P2 exactly
        assert_eq!(decoded[1].header.tick, 2);
        assert_eq!(decoded[1].entries.len(), 2);
        assert_eq!(decoded[1].entries[0].value, vec![2]);
        assert_eq!(decoded[1].entries[1].value, vec![7, 7]);
    }

    #[test]
    fn encode_decode_empty_entries() {
        let header = test_header();
        let entries: Vec<V2Entry> = vec![];

        let encoded = encode_v2_packet(&header, &entries).expect("encode should succeed");
        let decoded = decode_v2_packet(&encoded).expect("empty entries should decode");

        assert_eq!(decoded.header, header);
        assert!(decoded.entries.is_empty());
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut bad =
            encode_v2_packet(&test_header(), &test_entries()).expect("encode should succeed");
        bad[0] = 0xFF; // corrupt magic
        assert_eq!(decode_v2_packet(&bad).unwrap_err(), DecodeError::BadMagic);
    }

    #[test]
    fn decode_rejects_truncated() {
        let encoded =
            encode_v2_packet(&test_header(), &test_entries()).expect("encode should succeed");
        let truncated = &encoded[..encoded.len() - 1];
        assert!(decode_v2_packet(truncated).is_err());
    }

    #[test]
    fn decode_rejects_too_short() {
        let short = vec![0u8; HEADER_SIZE_V2 - 1];
        assert_eq!(
            decode_v2_packet(&short).unwrap_err(),
            DecodeError::TruncatedHeader
        );
    }

    #[test]
    fn header_size_correct() {
        let encoded = encode_v2_packet(&test_header(), &[]).expect("encode should succeed");
        // Header(12) + receipt(168) + entry_count(4) = 184
        assert_eq!(encoded.len(), HEADER_SIZE_V2 + MIN_PAYLOAD_SIZE_V2);
    }

    #[test]
    fn value_hash_is_blake3() {
        let value = vec![1, 2, 3, 4, 5];
        let expected: Hash = blake3::hash(&value).into();
        assert_eq!(compute_value_hash(&value), expected);
    }

    #[test]
    fn version_constants_differ() {
        assert_ne!(
            FRAME_VERSION, FRAME_VERSION_V2,
            "v1 and v2 versions must differ"
        );
    }

    #[test]
    fn decode_v2_rejects_oversized_entry_count() {
        // Build a minimal valid header with entry_count = u32::MAX but tiny payload
        let mut buf = Vec::new();
        // Header: magic + version + reserved + payload_len
        buf.extend_from_slice(&FRAME_MAGIC);
        buf.extend_from_slice(&FRAME_VERSION_V2.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // reserved

        // Payload = receipt (168 bytes) + entry_count (4) = 172 bytes
        #[allow(clippy::cast_possible_truncation)] // test constant, always fits u32
        let payload_len = (RECEIPT_SIZE_V2 + 4) as u32;
        buf.extend_from_slice(&payload_len.to_le_bytes());

        // Receipt fields (168 bytes of zeros)
        buf.extend_from_slice(&[0u8; 32]); // session_id
        buf.extend_from_slice(&[0u8; 32]); // cursor_id
        buf.extend_from_slice(&[0u8; 32]); // worldline_id
        buf.extend_from_slice(&[0u8; 32]); // warp_id
        buf.extend_from_slice(&0u64.to_le_bytes()); // tick
        buf.extend_from_slice(&[0u8; 32]); // commit_hash

        // Entry count: u32::MAX (malicious)
        buf.extend_from_slice(&u32::MAX.to_le_bytes());

        let result = decode_v2_packet(&buf);
        assert!(
            matches!(result, Err(DecodeError::InvalidEntryCount)),
            "expected InvalidEntryCount, got: {result:?}",
        );
    }
}
