// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! EINT v2 — Intent Envelope wire codec.
//!
//! Wire format (Little-Endian):
//! ```text
//! offset size  field
//! 0      4     magic = ASCII "EINT"
//! 4      2     envelope_version = u16 LE (2)
//! 6      2     flags = u16 LE
//! 8      32    schema_hash = [u8;32]
//!
//! 40     4     opcode = u32 LE
//! 44     2     op_version = u16 LE
//! 46     2     reserved = u16 LE (0)
//!
//! 48     4     payload_len = u32 LE
//! 52     32    payload_checksum = blake3(payload_bytes)
//!
//! 84     N     payload_bytes (canonical CBOR)
//! ```
//!
//! Flags (u16):
//! - bit0: HAS_RESPONSE_ID
//! - bit1: COMPRESSED
//! - others reserved

use blake3::Hasher;

/// Protocol magic constant "EINT".
pub const EINT_MAGIC: [u8; 4] = [b'E', b'I', b'N', b'T'];

/// Current envelope version.
pub const EINT_VERSION: u16 = 2;

/// Fixed header size in bytes (before payload).
pub const EINT_HEADER_SIZE: usize = 84;

/// Maximum payload size (256 MiB) to prevent DoS.
pub const EINT_MAX_PAYLOAD: u32 = 256 * 1024 * 1024;

/// EINT v2 flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EintFlags(pub u16);

impl EintFlags {
    /// Flag: payload includes a response ID suffix.
    pub const HAS_RESPONSE_ID: u16 = 1 << 0;
    /// Flag: payload is compressed (algorithm TBD).
    pub const COMPRESSED: u16 = 1 << 1;

    /// Create flags from raw u16.
    #[inline]
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// Check if HAS_RESPONSE_ID is set.
    #[inline]
    pub const fn has_response_id(self) -> bool {
        self.0 & Self::HAS_RESPONSE_ID != 0
    }

    /// Check if COMPRESSED is set.
    #[inline]
    pub const fn is_compressed(self) -> bool {
        self.0 & Self::COMPRESSED != 0
    }
}

/// EINT v2 decode/encode errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EintError {
    /// Input too short to contain header.
    #[error("incomplete header: need {EINT_HEADER_SIZE} bytes, got {0}")]
    IncompleteHeader(usize),

    /// Input too short to contain declared payload.
    #[error("incomplete payload: need {needed} bytes, got {got}")]
    IncompletePayload {
        /// Bytes needed.
        needed: usize,
        /// Bytes available.
        got: usize,
    },

    /// Magic bytes do not match "EINT".
    #[error("bad magic: expected EINT, got {0:?}")]
    BadMagic([u8; 4]),

    /// Unsupported envelope version.
    #[error("unsupported version: expected {EINT_VERSION}, got {0}")]
    UnsupportedVersion(u16),

    /// Reserved field is non-zero.
    #[error("reserved field must be zero, got {0}")]
    NonZeroReserved(u16),

    /// Payload length exceeds maximum allowed.
    #[error("payload too large: {0} bytes exceeds max {EINT_MAX_PAYLOAD}")]
    PayloadTooLarge(u32),

    /// Payload checksum mismatch.
    #[error("checksum mismatch: expected {expected:x?}, got {got:x?}")]
    ChecksumMismatch {
        /// Expected checksum from header.
        expected: [u8; 32],
        /// Computed checksum from payload.
        got: [u8; 32],
    },
}

/// Hash type (32-byte BLAKE3 hash).
pub type Hash32 = [u8; 32];

/// EINT v2 header (fixed 84-byte prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EintHeader {
    /// Envelope version (always 2 for v2).
    pub version: u16,
    /// Flags (see [`EintFlags`]).
    pub flags: EintFlags,
    /// Schema hash identifying the protocol schema.
    pub schema_hash: Hash32,
    /// Operation code.
    pub opcode: u32,
    /// Operation version.
    pub op_version: u16,
    /// Payload length in bytes.
    pub payload_len: u32,
    /// BLAKE3 checksum of payload bytes.
    pub payload_checksum: Hash32,
}

impl EintHeader {
    /// Encode header to a 84-byte array.
    pub fn to_bytes(&self) -> [u8; EINT_HEADER_SIZE] {
        let mut buf = [0u8; EINT_HEADER_SIZE];

        // Magic
        buf[0..4].copy_from_slice(&EINT_MAGIC);
        // Version (LE)
        buf[4..6].copy_from_slice(&self.version.to_le_bytes());
        // Flags (LE)
        buf[6..8].copy_from_slice(&self.flags.0.to_le_bytes());
        // Schema hash
        buf[8..40].copy_from_slice(&self.schema_hash);
        // Opcode (LE)
        buf[40..44].copy_from_slice(&self.opcode.to_le_bytes());
        // Op version (LE)
        buf[44..46].copy_from_slice(&self.op_version.to_le_bytes());
        // Reserved (LE, must be 0)
        buf[46..48].copy_from_slice(&0u16.to_le_bytes());
        // Payload length (LE)
        buf[48..52].copy_from_slice(&self.payload_len.to_le_bytes());
        // Payload checksum
        buf[52..84].copy_from_slice(&self.payload_checksum);

        buf
    }

    /// Parse header from bytes. Returns error if invalid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EintError> {
        if bytes.len() < EINT_HEADER_SIZE {
            return Err(EintError::IncompleteHeader(bytes.len()));
        }

        // Magic
        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if magic != EINT_MAGIC {
            return Err(EintError::BadMagic(magic));
        }

        // Version
        let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        if version != EINT_VERSION {
            return Err(EintError::UnsupportedVersion(version));
        }

        // Flags
        let flags = EintFlags::from_bits(u16::from_le_bytes(bytes[6..8].try_into().unwrap()));

        // Schema hash
        let schema_hash: Hash32 = bytes[8..40].try_into().unwrap();

        // Opcode
        let opcode = u32::from_le_bytes(bytes[40..44].try_into().unwrap());

        // Op version
        let op_version = u16::from_le_bytes(bytes[44..46].try_into().unwrap());

        // Reserved (must be 0)
        let reserved = u16::from_le_bytes(bytes[46..48].try_into().unwrap());
        if reserved != 0 {
            return Err(EintError::NonZeroReserved(reserved));
        }

        // Payload length
        let payload_len = u32::from_le_bytes(bytes[48..52].try_into().unwrap());
        if payload_len > EINT_MAX_PAYLOAD {
            return Err(EintError::PayloadTooLarge(payload_len));
        }

        // Payload checksum
        let payload_checksum: Hash32 = bytes[52..84].try_into().unwrap();

        Ok(Self {
            version,
            flags,
            schema_hash,
            opcode,
            op_version,
            payload_len,
            payload_checksum,
        })
    }
}

/// EINT v2 frame with borrowed payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EintFrame<'a> {
    /// Parsed header.
    pub header: EintHeader,
    /// Payload bytes (canonical CBOR).
    pub payload: &'a [u8],
}

/// Encode an EINT v2 frame.
///
/// Computes the BLAKE3 checksum of `payload_bytes` and builds the complete frame.
///
/// # Arguments
/// * `schema_hash` - Schema identifier (32 bytes)
/// * `opcode` - Operation code
/// * `op_version` - Operation version
/// * `flags` - Frame flags
/// * `payload_bytes` - Canonical CBOR payload
///
/// # Returns
/// Complete frame bytes (header + payload).
pub fn encode_eint_v2(
    schema_hash: Hash32,
    opcode: u32,
    op_version: u16,
    flags: EintFlags,
    payload_bytes: &[u8],
) -> Result<Vec<u8>, EintError> {
    let payload_len = payload_bytes.len() as u32;
    if payload_len > EINT_MAX_PAYLOAD {
        return Err(EintError::PayloadTooLarge(payload_len));
    }

    // Compute checksum
    let mut hasher = Hasher::new();
    hasher.update(payload_bytes);
    let payload_checksum: Hash32 = *hasher.finalize().as_bytes();

    let header = EintHeader {
        version: EINT_VERSION,
        flags,
        schema_hash,
        opcode,
        op_version,
        payload_len,
        payload_checksum,
    };

    let header_bytes = header.to_bytes();
    let mut out = Vec::with_capacity(EINT_HEADER_SIZE + payload_bytes.len());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(payload_bytes);
    Ok(out)
}

/// Decode an EINT v2 frame from bytes.
///
/// Performs strict validation:
/// - Verifies magic, version, reserved field
/// - Validates payload length bounds
/// - Verifies BLAKE3 checksum
///
/// # Arguments
/// * `bytes` - Input bytes (must contain complete frame)
///
/// # Returns
/// * `Ok((frame, consumed))` - Parsed frame and number of bytes consumed
/// * `Err(e)` - Parse error
pub fn decode_eint_v2(bytes: &[u8]) -> Result<(EintFrame<'_>, usize), EintError> {
    let header = EintHeader::from_bytes(bytes)?;

    let payload_start = EINT_HEADER_SIZE;
    let payload_end = payload_start + header.payload_len as usize;

    if bytes.len() < payload_end {
        return Err(EintError::IncompletePayload {
            needed: payload_end,
            got: bytes.len(),
        });
    }

    let payload = &bytes[payload_start..payload_end];

    // Verify checksum
    let mut hasher = Hasher::new();
    hasher.update(payload);
    let computed: Hash32 = *hasher.finalize().as_bytes();

    if computed != header.payload_checksum {
        return Err(EintError::ChecksumMismatch {
            expected: header.payload_checksum,
            got: computed,
        });
    }

    Ok((EintFrame { header, payload }, payload_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zero schema hash for testing.
    const ZERO_HASH: Hash32 = [0u8; 32];

    /// Test schema hash.
    fn test_schema_hash() -> Hash32 {
        let mut h = [0u8; 32];
        h[0] = 0xAB;
        h[31] = 0xCD;
        h
    }

    #[test]
    fn roundtrip_empty_payload() {
        let payload = b"";
        let encoded = encode_eint_v2(
            test_schema_hash(),
            0x0001, // TTD_SEEK
            1,
            EintFlags::default(),
            payload,
        )
        .unwrap();

        assert_eq!(encoded.len(), EINT_HEADER_SIZE);

        let (frame, consumed) = decode_eint_v2(&encoded).unwrap();
        assert_eq!(consumed, EINT_HEADER_SIZE);
        assert_eq!(frame.header.opcode, 0x0001);
        assert_eq!(frame.header.op_version, 1);
        assert_eq!(frame.header.schema_hash, test_schema_hash());
        assert_eq!(frame.payload, b"");
    }

    #[test]
    fn roundtrip_with_payload() {
        // Canonical CBOR for integer 42: 0x18 0x2a
        let payload = &[0x18, 0x2a];
        let encoded = encode_eint_v2(
            test_schema_hash(),
            0x0002, // TTD_STEP
            2,
            EintFlags::from_bits(EintFlags::HAS_RESPONSE_ID),
            payload,
        )
        .unwrap();

        assert_eq!(encoded.len(), EINT_HEADER_SIZE + 2);

        let (frame, consumed) = decode_eint_v2(&encoded).unwrap();
        assert_eq!(consumed, EINT_HEADER_SIZE + 2);
        assert_eq!(frame.header.opcode, 0x0002);
        assert_eq!(frame.header.op_version, 2);
        assert!(frame.header.flags.has_response_id());
        assert!(!frame.header.flags.is_compressed());
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn roundtrip_large_payload() {
        let payload = vec![0xAB; 1024];
        let encoded = encode_eint_v2(
            ZERO_HASH,
            0xFFFF_FFFF,
            0xFFFF,
            EintFlags::default(),
            &payload,
        )
        .unwrap();

        let (frame, consumed) = decode_eint_v2(&encoded).unwrap();
        assert_eq!(consumed, EINT_HEADER_SIZE + 1024);
        assert_eq!(frame.payload, payload.as_slice());
    }

    #[test]
    fn reject_bad_magic() {
        let mut bytes = [0u8; EINT_HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"NOPE");

        let err = decode_eint_v2(&bytes).unwrap_err();
        assert!(matches!(err, EintError::BadMagic([b'N', b'O', b'P', b'E'])));
    }

    #[test]
    fn reject_bad_version() {
        let mut bytes = [0u8; EINT_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&EINT_MAGIC);
        bytes[4..6].copy_from_slice(&1u16.to_le_bytes()); // version 1, not 2

        let err = decode_eint_v2(&bytes).unwrap_err();
        assert!(matches!(err, EintError::UnsupportedVersion(1)));
    }

    #[test]
    fn reject_nonzero_reserved() {
        let mut bytes = [0u8; EINT_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&EINT_MAGIC);
        bytes[4..6].copy_from_slice(&EINT_VERSION.to_le_bytes());
        bytes[46..48].copy_from_slice(&1u16.to_le_bytes()); // reserved != 0

        let err = decode_eint_v2(&bytes).unwrap_err();
        assert!(matches!(err, EintError::NonZeroReserved(1)));
    }

    #[test]
    fn reject_truncated_header() {
        let bytes = [0u8; EINT_HEADER_SIZE - 1];
        let err = decode_eint_v2(&bytes).unwrap_err();
        assert!(matches!(err, EintError::IncompleteHeader(83)));
    }

    #[test]
    fn reject_truncated_payload() {
        let payload = b"hello";
        let full = encode_eint_v2(ZERO_HASH, 1, 1, EintFlags::default(), payload).unwrap();

        // Truncate last byte
        let truncated = &full[..full.len() - 1];
        let err = decode_eint_v2(truncated).unwrap_err();
        assert!(matches!(err, EintError::IncompletePayload { .. }));
    }

    #[test]
    fn reject_bad_checksum() {
        let payload = b"hello";
        let mut encoded = encode_eint_v2(ZERO_HASH, 1, 1, EintFlags::default(), payload).unwrap();

        // Corrupt payload
        *encoded.last_mut().unwrap() ^= 0xFF;

        let err = decode_eint_v2(&encoded).unwrap_err();
        assert!(matches!(err, EintError::ChecksumMismatch { .. }));
    }

    #[test]
    fn reject_payload_too_large() {
        // Manually craft header with payload_len > max
        let mut bytes = [0u8; EINT_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&EINT_MAGIC);
        bytes[4..6].copy_from_slice(&EINT_VERSION.to_le_bytes());
        // payload_len = EINT_MAX_PAYLOAD + 1
        let bad_len = EINT_MAX_PAYLOAD + 1;
        bytes[48..52].copy_from_slice(&bad_len.to_le_bytes());

        let err = decode_eint_v2(&bytes).unwrap_err();
        assert!(matches!(err, EintError::PayloadTooLarge(_)));
    }

    #[test]
    fn header_byte_layout_matches_spec() {
        // Verify exact byte positions match spec
        let header = EintHeader {
            version: EINT_VERSION,
            flags: EintFlags::from_bits(0x0003), // HAS_RESPONSE_ID | COMPRESSED
            schema_hash: {
                let mut h = [0u8; 32];
                h[0] = 0x11;
                h[31] = 0x22;
                h
            },
            opcode: 0x12345678,
            op_version: 0xABCD,
            payload_len: 0xDEADBEEF,
            payload_checksum: {
                let mut h = [0u8; 32];
                h[0] = 0x33;
                h[31] = 0x44;
                h
            },
        };

        let bytes = header.to_bytes();

        // offset 0-3: magic
        assert_eq!(&bytes[0..4], b"EINT");
        // offset 4-5: version LE
        assert_eq!(&bytes[4..6], &[0x02, 0x00]);
        // offset 6-7: flags LE
        assert_eq!(&bytes[6..8], &[0x03, 0x00]);
        // offset 8: schema_hash[0]
        assert_eq!(bytes[8], 0x11);
        // offset 39: schema_hash[31]
        assert_eq!(bytes[39], 0x22);
        // offset 40-43: opcode LE
        assert_eq!(&bytes[40..44], &[0x78, 0x56, 0x34, 0x12]);
        // offset 44-45: op_version LE
        assert_eq!(&bytes[44..46], &[0xCD, 0xAB]);
        // offset 46-47: reserved (0)
        assert_eq!(&bytes[46..48], &[0x00, 0x00]);
        // offset 48-51: payload_len LE
        assert_eq!(&bytes[48..52], &[0xEF, 0xBE, 0xAD, 0xDE]);
        // offset 52: payload_checksum[0]
        assert_eq!(bytes[52], 0x33);
        // offset 83: payload_checksum[31]
        assert_eq!(bytes[83], 0x44);
    }

    #[test]
    fn flags_accessors() {
        let none = EintFlags::default();
        assert!(!none.has_response_id());
        assert!(!none.is_compressed());

        let resp = EintFlags::from_bits(EintFlags::HAS_RESPONSE_ID);
        assert!(resp.has_response_id());
        assert!(!resp.is_compressed());

        let comp = EintFlags::from_bits(EintFlags::COMPRESSED);
        assert!(!comp.has_response_id());
        assert!(comp.is_compressed());

        let both = EintFlags::from_bits(EintFlags::HAS_RESPONSE_ID | EintFlags::COMPRESSED);
        assert!(both.has_response_id());
        assert!(both.is_compressed());
    }

    #[test]
    fn decode_with_trailing_bytes() {
        // Encode a frame
        let payload = b"test";
        let encoded = encode_eint_v2(ZERO_HASH, 1, 1, EintFlags::default(), payload).unwrap();

        // Append trailing bytes
        let mut with_trailing = encoded.clone();
        with_trailing.extend_from_slice(b"EXTRA");

        // Decode should succeed and report correct consumed count
        let (frame, consumed) = decode_eint_v2(&with_trailing).unwrap();
        assert_eq!(consumed, EINT_HEADER_SIZE + 4);
        assert_eq!(frame.payload, b"test");
        assert_eq!(&with_trailing[consumed..], b"EXTRA");
    }

    /// Golden vector: TTD_SEEK opcode with empty payload.
    #[test]
    fn golden_vector_ttd_seek_empty() {
        // Spec: TTD_SEEK = 0x0001
        let schema_hash = [0xFFu8; 32]; // All 0xFF for reproducibility
        let payload = b"";

        let encoded =
            encode_eint_v2(schema_hash, 0x0001, 1, EintFlags::default(), payload).unwrap();

        // Verify structure
        assert_eq!(&encoded[0..4], b"EINT");
        assert_eq!(u16::from_le_bytes([encoded[4], encoded[5]]), 2); // version
        assert_eq!(u16::from_le_bytes([encoded[6], encoded[7]]), 0); // flags
        assert_eq!(&encoded[8..40], &schema_hash);
        assert_eq!(
            u32::from_le_bytes([encoded[40], encoded[41], encoded[42], encoded[43]]),
            0x0001
        ); // opcode
        assert_eq!(u16::from_le_bytes([encoded[44], encoded[45]]), 1); // op_version
        assert_eq!(u16::from_le_bytes([encoded[46], encoded[47]]), 0); // reserved
        assert_eq!(
            u32::from_le_bytes([encoded[48], encoded[49], encoded[50], encoded[51]]),
            0
        ); // payload_len

        // blake3("") = af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
        let empty_hash = blake3::hash(b"");
        assert_eq!(&encoded[52..84], empty_hash.as_bytes());

        // Roundtrip
        let (frame, _) = decode_eint_v2(&encoded).unwrap();
        assert_eq!(frame.header.opcode, 0x0001);
        assert_eq!(frame.payload, b"");
    }
}
