// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! TTDR v2 — Tick Receipt Record wire codec.
//!
//! Wire format (Little-Endian):
//! ```text
//! offset size  field
//! 0      4     magic = ASCII "TTDR"
//! 4      2     receipt_version = u16 LE (2)
//! 6      2     flags = u16 LE
//!
//! 8      32    schema_hash = [u8;32]
//! 40     32    worldline_id = [u8;32]
//! 72     8     tick = u64 LE
//!
//! 80     32    commit_hash = [u8;32]
//! 112    32    patch_digest = [u8;32]
//! 144    32    state_root = [u8;32]         (zero if absent)
//! 176    32    emissions_digest = [u8;32]
//! 208    32    op_emission_index_digest     (zero if absent)
//!
//! 240    2     parent_count = u16 LE
//! 242    2     channel_count = u16 LE
//!
//! 244    32*P  parent_hashes [P][32]
//! ...    var   channel_digests (per channel)
//! ```
//!
//! Flags (u16):
//! - bit0: HAS_STATE_ROOT
//! - bit1: HAS_OP_EMISSION_INDEX_DIGEST
//! - bit2: HAS_CHANNEL_PAYLOAD_HASH
//! - bit3: HAS_ENTRY_HASHES
//! - bit4-5: RECEIPT_MODE (2 bits)

/// Protocol magic constant "TTDR".
pub const TTDR_MAGIC: [u8; 4] = [b'T', b'T', b'D', b'R'];

/// Current receipt version.
pub const TTDR_VERSION: u16 = 2;

/// Fixed header size in bytes (before variable sections).
pub const TTDR_FIXED_HEADER_SIZE: usize = 244;

/// Maximum parent count to prevent DoS.
pub const TTDR_MAX_PARENTS: u16 = 256;

/// Maximum channel count to prevent DoS.
pub const TTDR_MAX_CHANNELS: u16 = 1024;

/// Hash type (32-byte BLAKE3 hash).
pub type Hash32 = [u8; 32];

/// Zero hash constant.
pub const ZERO_HASH: Hash32 = [0u8; 32];

/// Receipt compression modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ReceiptMode {
    /// Full: all fields, all channel digests, all entry hashes.
    #[default]
    Full = 0,
    /// Proof: hashes + digests only (no payload bodies).
    Proof = 1,
    /// Light: commit_hash + emissions_digest + state_root only.
    Light = 2,
    /// Reserved for future use.
    Reserved = 3,
}

impl ReceiptMode {
    /// Decode from 2-bit value.
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0x03 {
            0 => Self::Full,
            1 => Self::Proof,
            2 => Self::Light,
            _ => Self::Reserved,
        }
    }

    /// Encode to 2-bit value.
    pub const fn to_bits(self) -> u8 {
        self as u8
    }
}

/// TTDR v2 flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TtdrFlags(pub u16);

impl TtdrFlags {
    /// Flag: state_root field is meaningful (not zero).
    pub const HAS_STATE_ROOT: u16 = 1 << 0;
    /// Flag: op_emission_index_digest field is meaningful (not zero).
    pub const HAS_OP_EMISSION_INDEX_DIGEST: u16 = 1 << 1;
    /// Flag: channel digests include payload hashes.
    pub const HAS_CHANNEL_PAYLOAD_HASH: u16 = 1 << 2;
    /// Flag: channel digests include per-entry hashes.
    pub const HAS_ENTRY_HASHES: u16 = 1 << 3;
    /// Mask for receipt mode bits (4-5).
    const RECEIPT_MODE_MASK: u16 = 0b11 << 4;
    /// Shift for receipt mode bits.
    const RECEIPT_MODE_SHIFT: u16 = 4;

    /// Create flags from raw u16.
    #[inline]
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// Check if HAS_STATE_ROOT is set.
    #[inline]
    pub const fn has_state_root(self) -> bool {
        self.0 & Self::HAS_STATE_ROOT != 0
    }

    /// Check if HAS_OP_EMISSION_INDEX_DIGEST is set.
    #[inline]
    pub const fn has_op_emission_index_digest(self) -> bool {
        self.0 & Self::HAS_OP_EMISSION_INDEX_DIGEST != 0
    }

    /// Check if HAS_CHANNEL_PAYLOAD_HASH is set.
    #[inline]
    pub const fn has_channel_payload_hash(self) -> bool {
        self.0 & Self::HAS_CHANNEL_PAYLOAD_HASH != 0
    }

    /// Check if HAS_ENTRY_HASHES is set.
    #[inline]
    pub const fn has_entry_hashes(self) -> bool {
        self.0 & Self::HAS_ENTRY_HASHES != 0
    }

    /// Get the receipt mode.
    #[inline]
    pub const fn receipt_mode(self) -> ReceiptMode {
        let bits = ((self.0 & Self::RECEIPT_MODE_MASK) >> Self::RECEIPT_MODE_SHIFT) as u8;
        ReceiptMode::from_bits(bits)
    }

    /// Set the receipt mode.
    #[inline]
    pub const fn with_receipt_mode(self, mode: ReceiptMode) -> Self {
        let cleared = self.0 & !Self::RECEIPT_MODE_MASK;
        let mode_bits = (mode.to_bits() as u16) << Self::RECEIPT_MODE_SHIFT;
        Self(cleared | mode_bits)
    }

    /// Create flags with specific settings.
    pub const fn new(
        has_state_root: bool,
        has_op_emission_index_digest: bool,
        has_channel_payload_hash: bool,
        has_entry_hashes: bool,
        mode: ReceiptMode,
    ) -> Self {
        let mut bits = 0u16;
        if has_state_root {
            bits |= Self::HAS_STATE_ROOT;
        }
        if has_op_emission_index_digest {
            bits |= Self::HAS_OP_EMISSION_INDEX_DIGEST;
        }
        if has_channel_payload_hash {
            bits |= Self::HAS_CHANNEL_PAYLOAD_HASH;
        }
        if has_entry_hashes {
            bits |= Self::HAS_ENTRY_HASHES;
        }
        bits |= (mode.to_bits() as u16) << Self::RECEIPT_MODE_SHIFT;
        Self(bits)
    }
}

/// TTDR v2 decode/encode errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TtdrError {
    /// Input too short to contain fixed header.
    #[error("incomplete header: need {TTDR_FIXED_HEADER_SIZE} bytes, got {0}")]
    IncompleteHeader(usize),

    /// Input too short to contain parent hashes.
    #[error("incomplete parent hashes: need {needed} bytes, got {got}")]
    IncompleteParents {
        /// Bytes needed.
        needed: usize,
        /// Bytes available.
        got: usize,
    },

    /// Input too short to contain channel digests.
    #[error("incomplete channel digests: need {needed} bytes, got {got}")]
    IncompleteChannels {
        /// Bytes needed.
        needed: usize,
        /// Bytes available.
        got: usize,
    },

    /// Magic bytes do not match "TTDR".
    #[error("bad magic: expected TTDR, got {0:?}")]
    BadMagic([u8; 4]),

    /// Unsupported receipt version.
    #[error("unsupported version: expected {TTDR_VERSION}, got {0}")]
    UnsupportedVersion(u16),

    /// Parent count exceeds maximum.
    #[error("too many parents: {0} exceeds max {TTDR_MAX_PARENTS}")]
    TooManyParents(u16),

    /// Channel count exceeds maximum.
    #[error("too many channels: {0} exceeds max {TTDR_MAX_CHANNELS}")]
    TooManyChannels(u16),

    /// Entry count exceeds maximum.
    #[error("too many entries in channel {channel_idx}: {count} exceeds max {max}")]
    TooManyEntries {
        /// Channel index.
        channel_idx: u16,
        /// Actual entry count.
        count: u32,
        /// Maximum allowed.
        max: u32,
    },

    /// Reserved receipt mode used.
    #[error("reserved receipt mode 3 is not allowed")]
    ReservedReceiptMode,
}

/// TTDR v2 fixed header (first 244 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtdrHeader {
    /// Receipt version (always 2 for v2).
    pub version: u16,
    /// Flags (see [`TtdrFlags`]).
    pub flags: TtdrFlags,
    /// Schema hash identifying the protocol schema.
    pub schema_hash: Hash32,
    /// Worldline identifier.
    pub worldline_id: Hash32,
    /// Tick number.
    pub tick: u64,
    /// Commit hash for this tick.
    pub commit_hash: Hash32,
    /// Patch digest.
    pub patch_digest: Hash32,
    /// State root (zero if HAS_STATE_ROOT is false).
    pub state_root: Hash32,
    /// Emissions digest.
    pub emissions_digest: Hash32,
    /// Op emission index digest (zero if HAS_OP_EMISSION_INDEX_DIGEST is false).
    pub op_emission_index_digest: Hash32,
    /// Number of parent hashes.
    pub parent_count: u16,
    /// Number of channel digest entries.
    pub channel_count: u16,
}

impl TtdrHeader {
    /// Encode header to a 244-byte array.
    pub fn to_bytes(&self) -> [u8; TTDR_FIXED_HEADER_SIZE] {
        let mut buf = [0u8; TTDR_FIXED_HEADER_SIZE];

        // offset 0-3: magic
        buf[0..4].copy_from_slice(&TTDR_MAGIC);
        // offset 4-5: version (LE)
        buf[4..6].copy_from_slice(&self.version.to_le_bytes());
        // offset 6-7: flags (LE)
        buf[6..8].copy_from_slice(&self.flags.0.to_le_bytes());
        // offset 8-39: schema_hash
        buf[8..40].copy_from_slice(&self.schema_hash);
        // offset 40-71: worldline_id
        buf[40..72].copy_from_slice(&self.worldline_id);
        // offset 72-79: tick (LE)
        buf[72..80].copy_from_slice(&self.tick.to_le_bytes());
        // offset 80-111: commit_hash
        buf[80..112].copy_from_slice(&self.commit_hash);
        // offset 112-143: patch_digest
        buf[112..144].copy_from_slice(&self.patch_digest);
        // offset 144-175: state_root
        buf[144..176].copy_from_slice(&self.state_root);
        // offset 176-207: emissions_digest
        buf[176..208].copy_from_slice(&self.emissions_digest);
        // offset 208-239: op_emission_index_digest
        buf[208..240].copy_from_slice(&self.op_emission_index_digest);
        // offset 240-241: parent_count (LE)
        buf[240..242].copy_from_slice(&self.parent_count.to_le_bytes());
        // offset 242-243: channel_count (LE)
        buf[242..244].copy_from_slice(&self.channel_count.to_le_bytes());

        buf
    }

    /// Parse header from bytes. Returns error if invalid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TtdrError> {
        if bytes.len() < TTDR_FIXED_HEADER_SIZE {
            return Err(TtdrError::IncompleteHeader(bytes.len()));
        }

        // offset 0-3: magic
        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if magic != TTDR_MAGIC {
            return Err(TtdrError::BadMagic(magic));
        }

        // offset 4-5: version
        let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        if version != TTDR_VERSION {
            return Err(TtdrError::UnsupportedVersion(version));
        }

        // offset 6-7: flags
        let flags = TtdrFlags::from_bits(u16::from_le_bytes(bytes[6..8].try_into().unwrap()));

        // Reject reserved receipt mode
        if flags.receipt_mode() == ReceiptMode::Reserved {
            return Err(TtdrError::ReservedReceiptMode);
        }

        // offset 8-39: schema_hash
        let schema_hash: Hash32 = bytes[8..40].try_into().unwrap();

        // offset 40-71: worldline_id
        let worldline_id: Hash32 = bytes[40..72].try_into().unwrap();

        // offset 72-79: tick
        let tick = u64::from_le_bytes(bytes[72..80].try_into().unwrap());

        // offset 80-111: commit_hash
        let commit_hash: Hash32 = bytes[80..112].try_into().unwrap();

        // offset 112-143: patch_digest
        let patch_digest: Hash32 = bytes[112..144].try_into().unwrap();

        // offset 144-175: state_root
        let state_root: Hash32 = bytes[144..176].try_into().unwrap();

        // offset 176-207: emissions_digest
        let emissions_digest: Hash32 = bytes[176..208].try_into().unwrap();

        // offset 208-239: op_emission_index_digest
        let op_emission_index_digest: Hash32 = bytes[208..240].try_into().unwrap();

        // offset 240-241: parent_count
        let parent_count = u16::from_le_bytes(bytes[240..242].try_into().unwrap());
        if parent_count > TTDR_MAX_PARENTS {
            return Err(TtdrError::TooManyParents(parent_count));
        }

        // offset 242-243: channel_count
        let channel_count = u16::from_le_bytes(bytes[242..244].try_into().unwrap());
        if channel_count > TTDR_MAX_CHANNELS {
            return Err(TtdrError::TooManyChannels(channel_count));
        }

        Ok(Self {
            version,
            flags,
            schema_hash,
            worldline_id,
            tick,
            commit_hash,
            patch_digest,
            state_root,
            emissions_digest,
            op_emission_index_digest,
            parent_count,
            channel_count,
        })
    }
}

/// Per-channel digest entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelDigest {
    /// Channel identifier (32 bytes).
    pub channel_id: Hash32,
    /// Channel version.
    pub channel_version: u16,
    /// Payload hash (present if HAS_CHANNEL_PAYLOAD_HASH flag is set).
    pub payload_hash: Option<Hash32>,
    /// Entry hashes (present if HAS_ENTRY_HASHES flag is set).
    pub entry_hashes: Vec<Hash32>,
}

impl ChannelDigest {
    /// Minimum size for a channel digest (channel_id + version).
    pub const MIN_SIZE: usize = 32 + 2;

    /// Encode to bytes.
    pub fn encode(&self, flags: TtdrFlags, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.channel_id);
        out.extend_from_slice(&self.channel_version.to_le_bytes());

        if flags.has_channel_payload_hash() {
            let hash = self.payload_hash.unwrap_or(ZERO_HASH);
            out.extend_from_slice(&hash);
        }

        if flags.has_entry_hashes() {
            let count = self.entry_hashes.len() as u32;
            out.extend_from_slice(&count.to_le_bytes());
            for hash in &self.entry_hashes {
                out.extend_from_slice(hash);
            }
        }
    }

    /// Decode from bytes. Returns (digest, bytes_consumed).
    pub fn decode(
        bytes: &[u8],
        flags: TtdrFlags,
        channel_idx: u16,
    ) -> Result<(Self, usize), TtdrError> {
        let mut offset = 0;

        // channel_id (32 bytes)
        if bytes.len() < offset + 32 {
            return Err(TtdrError::IncompleteChannels {
                needed: offset + 32,
                got: bytes.len(),
            });
        }
        let channel_id: Hash32 = bytes[offset..offset + 32].try_into().unwrap();
        offset += 32;

        // channel_version (2 bytes)
        if bytes.len() < offset + 2 {
            return Err(TtdrError::IncompleteChannels {
                needed: offset + 2,
                got: bytes.len(),
            });
        }
        let channel_version = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        offset += 2;

        // payload_hash (32 bytes, optional)
        let payload_hash = if flags.has_channel_payload_hash() {
            if bytes.len() < offset + 32 {
                return Err(TtdrError::IncompleteChannels {
                    needed: offset + 32,
                    got: bytes.len(),
                });
            }
            let hash: Hash32 = bytes[offset..offset + 32].try_into().unwrap();
            offset += 32;
            Some(hash)
        } else {
            None
        };

        // entry_hashes (variable, optional)
        let entry_hashes = if flags.has_entry_hashes() {
            // entry_count (4 bytes)
            if bytes.len() < offset + 4 {
                return Err(TtdrError::IncompleteChannels {
                    needed: offset + 4,
                    got: bytes.len(),
                });
            }
            let entry_count = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            offset += 4;

            // Limit entry count to prevent DoS
            const MAX_ENTRIES_PER_CHANNEL: u32 = 65536;
            if entry_count > MAX_ENTRIES_PER_CHANNEL {
                return Err(TtdrError::TooManyEntries {
                    channel_idx,
                    count: entry_count,
                    max: MAX_ENTRIES_PER_CHANNEL,
                });
            }

            let hashes_size = entry_count as usize * 32;
            if bytes.len() < offset + hashes_size {
                return Err(TtdrError::IncompleteChannels {
                    needed: offset + hashes_size,
                    got: bytes.len(),
                });
            }

            let mut hashes = Vec::with_capacity(entry_count as usize);
            for _ in 0..entry_count {
                let hash: Hash32 = bytes[offset..offset + 32].try_into().unwrap();
                hashes.push(hash);
                offset += 32;
            }
            hashes
        } else {
            Vec::new()
        };

        Ok((
            Self {
                channel_id,
                channel_version,
                payload_hash,
                entry_hashes,
            },
            offset,
        ))
    }
}

/// TTDR v2 frame with parsed contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtdrFrame {
    /// Parsed header.
    pub header: TtdrHeader,
    /// Parent hashes.
    pub parent_hashes: Vec<Hash32>,
    /// Channel digests.
    pub channel_digests: Vec<ChannelDigest>,
}

impl TtdrFrame {
    /// Convert to a lighter receipt mode (strips data as appropriate).
    pub fn to_mode(&self, mode: ReceiptMode) -> Self {
        match mode {
            ReceiptMode::Full => self.clone(),
            ReceiptMode::Proof => {
                // Keep hashes, strip entry bodies (entry_hashes are already hashes)
                let mut frame = self.clone();
                frame.header.flags = frame.header.flags.with_receipt_mode(ReceiptMode::Proof);
                frame
            }
            ReceiptMode::Light => {
                // Minimal: just commit_hash, emissions_digest, state_root
                let mut header = self.header;
                header.flags = TtdrFlags::new(
                    header.flags.has_state_root(),
                    false, // no op_emission_index_digest
                    false, // no channel payload hash
                    false, // no entry hashes
                    ReceiptMode::Light,
                );
                header.parent_count = 0;
                header.channel_count = 0;
                header.op_emission_index_digest = ZERO_HASH;

                TtdrFrame {
                    header,
                    parent_hashes: Vec::new(),
                    channel_digests: Vec::new(),
                }
            }
            ReceiptMode::Reserved => self.clone(), // No-op for reserved
        }
    }
}

/// Encode a TTDR v2 frame.
pub fn encode_ttdr_v2(frame: &TtdrFrame) -> Result<Vec<u8>, TtdrError> {
    // Validate counts
    if frame.parent_hashes.len() > TTDR_MAX_PARENTS as usize {
        return Err(TtdrError::TooManyParents(frame.parent_hashes.len() as u16));
    }
    if frame.channel_digests.len() > TTDR_MAX_CHANNELS as usize {
        return Err(TtdrError::TooManyChannels(
            frame.channel_digests.len() as u16
        ));
    }

    // Build header with correct counts
    let mut header = frame.header;
    header.parent_count = frame.parent_hashes.len() as u16;
    header.channel_count = frame.channel_digests.len() as u16;

    // Calculate size
    let parents_size = frame.parent_hashes.len() * 32;
    let mut channels_size = 0;
    for digest in &frame.channel_digests {
        channels_size += ChannelDigest::MIN_SIZE;
        if header.flags.has_channel_payload_hash() {
            channels_size += 32;
        }
        if header.flags.has_entry_hashes() {
            channels_size += 4 + digest.entry_hashes.len() * 32;
        }
    }

    let total_size = TTDR_FIXED_HEADER_SIZE + parents_size + channels_size;
    let mut out = Vec::with_capacity(total_size);

    // Write header
    out.extend_from_slice(&header.to_bytes());

    // Write parent hashes
    for hash in &frame.parent_hashes {
        out.extend_from_slice(hash);
    }

    // Write channel digests
    for digest in &frame.channel_digests {
        digest.encode(header.flags, &mut out);
    }

    Ok(out)
}

/// Decode a TTDR v2 frame from bytes.
///
/// # Returns
/// * `Ok((frame, consumed))` - Parsed frame and number of bytes consumed
/// * `Err(e)` - Parse error
pub fn decode_ttdr_v2(bytes: &[u8]) -> Result<(TtdrFrame, usize), TtdrError> {
    let header = TtdrHeader::from_bytes(bytes)?;

    let mut offset = TTDR_FIXED_HEADER_SIZE;

    // Read parent hashes
    let parents_size = header.parent_count as usize * 32;
    if bytes.len() < offset + parents_size {
        return Err(TtdrError::IncompleteParents {
            needed: offset + parents_size,
            got: bytes.len(),
        });
    }

    let mut parent_hashes = Vec::with_capacity(header.parent_count as usize);
    for _ in 0..header.parent_count {
        let hash: Hash32 = bytes[offset..offset + 32].try_into().unwrap();
        parent_hashes.push(hash);
        offset += 32;
    }

    // Read channel digests
    let mut channel_digests = Vec::with_capacity(header.channel_count as usize);
    for i in 0..header.channel_count {
        let (digest, consumed) = ChannelDigest::decode(&bytes[offset..], header.flags, i)?;
        channel_digests.push(digest);
        offset += consumed;
    }

    Ok((
        TtdrFrame {
            header,
            parent_hashes,
            channel_digests,
        },
        offset,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_schema_hash() -> Hash32 {
        let mut h = [0u8; 32];
        h[0] = 0xAB;
        h[31] = 0xCD;
        h
    }

    fn test_worldline_id() -> Hash32 {
        let mut h = [0u8; 32];
        h[0] = 0x11;
        h[31] = 0x22;
        h
    }

    fn test_commit_hash() -> Hash32 {
        let mut h = [0u8; 32];
        h[0] = 0x33;
        h[31] = 0x44;
        h
    }

    #[test]
    fn roundtrip_minimal_receipt() {
        // LIGHT mode: no parents, no channels
        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(true, false, false, false, ReceiptMode::Light),
                schema_hash: test_schema_hash(),
                worldline_id: test_worldline_id(),
                tick: 42,
                commit_hash: test_commit_hash(),
                patch_digest: ZERO_HASH,
                state_root: [0xFFu8; 32],
                emissions_digest: ZERO_HASH,
                op_emission_index_digest: ZERO_HASH,
                parent_count: 0,
                channel_count: 0,
            },
            parent_hashes: vec![],
            channel_digests: vec![],
        };

        let encoded = encode_ttdr_v2(&frame).unwrap();
        assert_eq!(encoded.len(), TTDR_FIXED_HEADER_SIZE);

        let (decoded, consumed) = decode_ttdr_v2(&encoded).unwrap();
        assert_eq!(consumed, TTDR_FIXED_HEADER_SIZE);
        assert_eq!(decoded.header.tick, 42);
        assert_eq!(decoded.header.flags.receipt_mode(), ReceiptMode::Light);
        assert!(decoded.header.flags.has_state_root());
        assert!(decoded.parent_hashes.is_empty());
        assert!(decoded.channel_digests.is_empty());
    }

    #[test]
    fn roundtrip_with_parents() {
        let parent1 = [0x11u8; 32];
        let parent2 = [0x22u8; 32];

        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(false, false, false, false, ReceiptMode::Full),
                schema_hash: test_schema_hash(),
                worldline_id: test_worldline_id(),
                tick: 100,
                commit_hash: test_commit_hash(),
                patch_digest: ZERO_HASH,
                state_root: ZERO_HASH,
                emissions_digest: ZERO_HASH,
                op_emission_index_digest: ZERO_HASH,
                parent_count: 2,
                channel_count: 0,
            },
            parent_hashes: vec![parent1, parent2],
            channel_digests: vec![],
        };

        let encoded = encode_ttdr_v2(&frame).unwrap();
        assert_eq!(encoded.len(), TTDR_FIXED_HEADER_SIZE + 64);

        let (decoded, consumed) = decode_ttdr_v2(&encoded).unwrap();
        assert_eq!(consumed, TTDR_FIXED_HEADER_SIZE + 64);
        assert_eq!(decoded.parent_hashes.len(), 2);
        assert_eq!(decoded.parent_hashes[0], parent1);
        assert_eq!(decoded.parent_hashes[1], parent2);
    }

    #[test]
    fn roundtrip_with_channels_minimal() {
        // Channels without payload hash or entry hashes
        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(false, false, false, false, ReceiptMode::Proof),
                schema_hash: test_schema_hash(),
                worldline_id: test_worldline_id(),
                tick: 1000,
                commit_hash: test_commit_hash(),
                patch_digest: ZERO_HASH,
                state_root: ZERO_HASH,
                emissions_digest: ZERO_HASH,
                op_emission_index_digest: ZERO_HASH,
                parent_count: 0,
                channel_count: 2,
            },
            parent_hashes: vec![],
            channel_digests: vec![
                ChannelDigest {
                    channel_id: [0xAAu8; 32],
                    channel_version: 1,
                    payload_hash: None,
                    entry_hashes: vec![],
                },
                ChannelDigest {
                    channel_id: [0xBBu8; 32],
                    channel_version: 2,
                    payload_hash: None,
                    entry_hashes: vec![],
                },
            ],
        };

        let encoded = encode_ttdr_v2(&frame).unwrap();
        // 244 + 2*(32+2) = 244 + 68 = 312
        assert_eq!(encoded.len(), TTDR_FIXED_HEADER_SIZE + 2 * 34);

        let (decoded, consumed) = decode_ttdr_v2(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.channel_digests.len(), 2);
        assert_eq!(decoded.channel_digests[0].channel_id, [0xAAu8; 32]);
        assert_eq!(decoded.channel_digests[0].channel_version, 1);
        assert_eq!(decoded.channel_digests[1].channel_id, [0xBBu8; 32]);
        assert_eq!(decoded.channel_digests[1].channel_version, 2);
    }

    #[test]
    fn roundtrip_with_channel_payload_hash() {
        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(false, false, true, false, ReceiptMode::Full),
                schema_hash: test_schema_hash(),
                worldline_id: test_worldline_id(),
                tick: 500,
                commit_hash: test_commit_hash(),
                patch_digest: ZERO_HASH,
                state_root: ZERO_HASH,
                emissions_digest: ZERO_HASH,
                op_emission_index_digest: ZERO_HASH,
                parent_count: 0,
                channel_count: 1,
            },
            parent_hashes: vec![],
            channel_digests: vec![ChannelDigest {
                channel_id: [0xCCu8; 32],
                channel_version: 3,
                payload_hash: Some([0xDDu8; 32]),
                entry_hashes: vec![],
            }],
        };

        let encoded = encode_ttdr_v2(&frame).unwrap();
        // 244 + (32+2+32) = 244 + 66 = 310
        assert_eq!(encoded.len(), TTDR_FIXED_HEADER_SIZE + 66);

        let (decoded, _) = decode_ttdr_v2(&encoded).unwrap();
        assert_eq!(decoded.channel_digests[0].payload_hash, Some([0xDDu8; 32]));
    }

    #[test]
    fn roundtrip_with_entry_hashes() {
        let entry1 = [0xE1u8; 32];
        let entry2 = [0xE2u8; 32];

        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(false, false, false, true, ReceiptMode::Full),
                schema_hash: test_schema_hash(),
                worldline_id: test_worldline_id(),
                tick: 999,
                commit_hash: test_commit_hash(),
                patch_digest: ZERO_HASH,
                state_root: ZERO_HASH,
                emissions_digest: ZERO_HASH,
                op_emission_index_digest: ZERO_HASH,
                parent_count: 0,
                channel_count: 1,
            },
            parent_hashes: vec![],
            channel_digests: vec![ChannelDigest {
                channel_id: [0xFFu8; 32],
                channel_version: 1,
                payload_hash: None,
                entry_hashes: vec![entry1, entry2],
            }],
        };

        let encoded = encode_ttdr_v2(&frame).unwrap();
        // 244 + (32+2+4+64) = 244 + 102 = 346
        assert_eq!(encoded.len(), TTDR_FIXED_HEADER_SIZE + 102);

        let (decoded, _) = decode_ttdr_v2(&encoded).unwrap();
        assert_eq!(decoded.channel_digests[0].entry_hashes.len(), 2);
        assert_eq!(decoded.channel_digests[0].entry_hashes[0], entry1);
        assert_eq!(decoded.channel_digests[0].entry_hashes[1], entry2);
    }

    #[test]
    fn roundtrip_full_featured() {
        // All flags set, multiple parents and channels with entries
        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(true, true, true, true, ReceiptMode::Full),
                schema_hash: test_schema_hash(),
                worldline_id: test_worldline_id(),
                tick: 12345,
                commit_hash: test_commit_hash(),
                patch_digest: [0x01u8; 32],
                state_root: [0x02u8; 32],
                emissions_digest: [0x03u8; 32],
                op_emission_index_digest: [0x04u8; 32],
                parent_count: 2,
                channel_count: 2,
            },
            parent_hashes: vec![[0xA1u8; 32], [0xA2u8; 32]],
            channel_digests: vec![
                ChannelDigest {
                    channel_id: [0xC1u8; 32],
                    channel_version: 1,
                    payload_hash: Some([0xD1u8; 32]),
                    entry_hashes: vec![[0xE1u8; 32]],
                },
                ChannelDigest {
                    channel_id: [0xC2u8; 32],
                    channel_version: 2,
                    payload_hash: Some([0xD2u8; 32]),
                    entry_hashes: vec![[0xE2u8; 32], [0xE3u8; 32]],
                },
            ],
        };

        let encoded = encode_ttdr_v2(&frame).unwrap();
        let (decoded, consumed) = decode_ttdr_v2(&encoded).unwrap();

        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.header.tick, 12345);
        assert!(decoded.header.flags.has_state_root());
        assert!(decoded.header.flags.has_op_emission_index_digest());
        assert!(decoded.header.flags.has_channel_payload_hash());
        assert!(decoded.header.flags.has_entry_hashes());
        assert_eq!(decoded.header.flags.receipt_mode(), ReceiptMode::Full);
        assert_eq!(decoded.parent_hashes.len(), 2);
        assert_eq!(decoded.channel_digests.len(), 2);
        assert_eq!(decoded.channel_digests[1].entry_hashes.len(), 2);
    }

    #[test]
    fn reject_bad_magic() {
        let mut bytes = [0u8; TTDR_FIXED_HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"NOPE");

        let err = decode_ttdr_v2(&bytes).unwrap_err();
        assert!(matches!(err, TtdrError::BadMagic([b'N', b'O', b'P', b'E'])));
    }

    #[test]
    fn reject_bad_version() {
        let mut bytes = [0u8; TTDR_FIXED_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&TTDR_MAGIC);
        bytes[4..6].copy_from_slice(&1u16.to_le_bytes()); // version 1

        let err = decode_ttdr_v2(&bytes).unwrap_err();
        assert!(matches!(err, TtdrError::UnsupportedVersion(1)));
    }

    #[test]
    fn reject_reserved_receipt_mode() {
        let mut bytes = [0u8; TTDR_FIXED_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&TTDR_MAGIC);
        bytes[4..6].copy_from_slice(&TTDR_VERSION.to_le_bytes());
        // flags with receipt mode = 3 (reserved)
        let flags: u16 = 0b11 << 4; // bits 4-5 = 11
        bytes[6..8].copy_from_slice(&flags.to_le_bytes());

        let err = decode_ttdr_v2(&bytes).unwrap_err();
        assert!(matches!(err, TtdrError::ReservedReceiptMode));
    }

    #[test]
    fn reject_truncated_header() {
        let bytes = [0u8; TTDR_FIXED_HEADER_SIZE - 1];
        let err = decode_ttdr_v2(&bytes).unwrap_err();
        assert!(matches!(err, TtdrError::IncompleteHeader(243)));
    }

    #[test]
    fn reject_truncated_parents() {
        let mut bytes = vec![0u8; TTDR_FIXED_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&TTDR_MAGIC);
        bytes[4..6].copy_from_slice(&TTDR_VERSION.to_le_bytes());
        bytes[240..242].copy_from_slice(&2u16.to_le_bytes()); // 2 parents

        // Only add 1 parent hash (32 bytes) instead of 2 (64 bytes)
        bytes.extend_from_slice(&[0u8; 32]);

        let err = decode_ttdr_v2(&bytes).unwrap_err();
        assert!(matches!(err, TtdrError::IncompleteParents { .. }));
    }

    #[test]
    fn reject_too_many_parents() {
        let mut bytes = [0u8; TTDR_FIXED_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&TTDR_MAGIC);
        bytes[4..6].copy_from_slice(&TTDR_VERSION.to_le_bytes());
        bytes[240..242].copy_from_slice(&(TTDR_MAX_PARENTS + 1).to_le_bytes());

        let err = decode_ttdr_v2(&bytes).unwrap_err();
        assert!(matches!(err, TtdrError::TooManyParents(_)));
    }

    #[test]
    fn reject_too_many_channels() {
        let mut bytes = [0u8; TTDR_FIXED_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&TTDR_MAGIC);
        bytes[4..6].copy_from_slice(&TTDR_VERSION.to_le_bytes());
        bytes[242..244].copy_from_slice(&(TTDR_MAX_CHANNELS + 1).to_le_bytes());

        let err = decode_ttdr_v2(&bytes).unwrap_err();
        assert!(matches!(err, TtdrError::TooManyChannels(_)));
    }

    #[test]
    fn header_byte_layout_matches_spec() {
        let header = TtdrHeader {
            version: TTDR_VERSION,
            flags: TtdrFlags::new(true, true, true, true, ReceiptMode::Proof),
            schema_hash: {
                let mut h = [0u8; 32];
                h[0] = 0x11;
                h[31] = 0x12;
                h
            },
            worldline_id: {
                let mut h = [0u8; 32];
                h[0] = 0x21;
                h[31] = 0x22;
                h
            },
            tick: 0x123456789ABCDEF0,
            commit_hash: {
                let mut h = [0u8; 32];
                h[0] = 0x31;
                h[31] = 0x32;
                h
            },
            patch_digest: {
                let mut h = [0u8; 32];
                h[0] = 0x41;
                h[31] = 0x42;
                h
            },
            state_root: {
                let mut h = [0u8; 32];
                h[0] = 0x51;
                h[31] = 0x52;
                h
            },
            emissions_digest: {
                let mut h = [0u8; 32];
                h[0] = 0x61;
                h[31] = 0x62;
                h
            },
            op_emission_index_digest: {
                let mut h = [0u8; 32];
                h[0] = 0x71;
                h[31] = 0x72;
                h
            },
            parent_count: 0x1234,
            channel_count: 0x5678,
        };

        let bytes = header.to_bytes();

        // offset 0-3: magic
        assert_eq!(&bytes[0..4], b"TTDR");
        // offset 4-5: version LE
        assert_eq!(&bytes[4..6], &[0x02, 0x00]);
        // offset 6-7: flags LE (all bits set + mode 1)
        // bits 0-3 = 0xF, bits 4-5 = 01 (Proof) => 0x1F
        assert_eq!(&bytes[6..8], &[0x1F, 0x00]);
        // offset 8: schema_hash[0]
        assert_eq!(bytes[8], 0x11);
        // offset 39: schema_hash[31]
        assert_eq!(bytes[39], 0x12);
        // offset 40: worldline_id[0]
        assert_eq!(bytes[40], 0x21);
        // offset 71: worldline_id[31]
        assert_eq!(bytes[71], 0x22);
        // offset 72-79: tick LE
        assert_eq!(
            &bytes[72..80],
            &[0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]
        );
        // offset 80: commit_hash[0]
        assert_eq!(bytes[80], 0x31);
        // offset 111: commit_hash[31]
        assert_eq!(bytes[111], 0x32);
        // offset 112: patch_digest[0]
        assert_eq!(bytes[112], 0x41);
        // offset 143: patch_digest[31]
        assert_eq!(bytes[143], 0x42);
        // offset 144: state_root[0]
        assert_eq!(bytes[144], 0x51);
        // offset 175: state_root[31]
        assert_eq!(bytes[175], 0x52);
        // offset 176: emissions_digest[0]
        assert_eq!(bytes[176], 0x61);
        // offset 207: emissions_digest[31]
        assert_eq!(bytes[207], 0x62);
        // offset 208: op_emission_index_digest[0]
        assert_eq!(bytes[208], 0x71);
        // offset 239: op_emission_index_digest[31]
        assert_eq!(bytes[239], 0x72);
        // offset 240-241: parent_count LE
        assert_eq!(&bytes[240..242], &[0x34, 0x12]);
        // offset 242-243: channel_count LE
        assert_eq!(&bytes[242..244], &[0x78, 0x56]);
    }

    #[test]
    fn receipt_mode_roundtrip() {
        for mode in [
            ReceiptMode::Full,
            ReceiptMode::Proof,
            ReceiptMode::Light,
            ReceiptMode::Reserved,
        ] {
            let bits = mode.to_bits();
            let decoded = ReceiptMode::from_bits(bits);
            assert_eq!(decoded, mode);
        }
    }

    #[test]
    fn flags_with_receipt_mode() {
        let flags = TtdrFlags::new(true, false, true, false, ReceiptMode::Full);
        assert_eq!(flags.receipt_mode(), ReceiptMode::Full);

        let flags2 = flags.with_receipt_mode(ReceiptMode::Light);
        assert_eq!(flags2.receipt_mode(), ReceiptMode::Light);
        // Other flags should be preserved
        assert!(flags2.has_state_root());
        assert!(flags2.has_channel_payload_hash());
        assert!(!flags2.has_op_emission_index_digest());
        assert!(!flags2.has_entry_hashes());
    }

    #[test]
    fn to_light_mode_strips_data() {
        let full_frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(true, true, true, true, ReceiptMode::Full),
                schema_hash: test_schema_hash(),
                worldline_id: test_worldline_id(),
                tick: 100,
                commit_hash: test_commit_hash(),
                patch_digest: [0x01u8; 32],
                state_root: [0x02u8; 32],
                emissions_digest: [0x03u8; 32],
                op_emission_index_digest: [0x04u8; 32],
                parent_count: 2,
                channel_count: 1,
            },
            parent_hashes: vec![[0xAAu8; 32], [0xBBu8; 32]],
            channel_digests: vec![ChannelDigest {
                channel_id: [0xCCu8; 32],
                channel_version: 1,
                payload_hash: Some([0xDDu8; 32]),
                entry_hashes: vec![[0xEEu8; 32]],
            }],
        };

        let light = full_frame.to_mode(ReceiptMode::Light);

        assert_eq!(light.header.flags.receipt_mode(), ReceiptMode::Light);
        assert!(light.parent_hashes.is_empty());
        assert!(light.channel_digests.is_empty());
        assert_eq!(light.header.parent_count, 0);
        assert_eq!(light.header.channel_count, 0);
        // Core hashes preserved
        assert_eq!(light.header.commit_hash, test_commit_hash());
        assert_eq!(light.header.state_root, [0x02u8; 32]);
        assert_eq!(light.header.emissions_digest, [0x03u8; 32]);
        // op_emission_index_digest zeroed
        assert_eq!(light.header.op_emission_index_digest, ZERO_HASH);
    }

    #[test]
    fn decode_with_trailing_bytes() {
        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::default(),
                schema_hash: ZERO_HASH,
                worldline_id: ZERO_HASH,
                tick: 1,
                commit_hash: ZERO_HASH,
                patch_digest: ZERO_HASH,
                state_root: ZERO_HASH,
                emissions_digest: ZERO_HASH,
                op_emission_index_digest: ZERO_HASH,
                parent_count: 0,
                channel_count: 0,
            },
            parent_hashes: vec![],
            channel_digests: vec![],
        };

        let mut encoded = encode_ttdr_v2(&frame).unwrap();
        encoded.extend_from_slice(b"TRAILING");

        let (decoded, consumed) = decode_ttdr_v2(&encoded).unwrap();
        assert_eq!(consumed, TTDR_FIXED_HEADER_SIZE);
        assert_eq!(decoded.header.tick, 1);
        assert_eq!(&encoded[consumed..], b"TRAILING");
    }

    /// Golden vector: minimal LIGHT receipt at tick 0.
    #[test]
    fn golden_vector_light_receipt_tick_zero() {
        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(false, false, false, false, ReceiptMode::Light),
                schema_hash: [0xFFu8; 32],
                worldline_id: [0x00u8; 32],
                tick: 0,
                commit_hash: [0xAAu8; 32],
                patch_digest: ZERO_HASH,
                state_root: ZERO_HASH,
                emissions_digest: [0xBBu8; 32],
                op_emission_index_digest: ZERO_HASH,
                parent_count: 0,
                channel_count: 0,
            },
            parent_hashes: vec![],
            channel_digests: vec![],
        };

        let encoded = encode_ttdr_v2(&frame).unwrap();

        // Verify structure
        assert_eq!(&encoded[0..4], b"TTDR");
        assert_eq!(u16::from_le_bytes([encoded[4], encoded[5]]), 2); // version
                                                                     // flags: receipt mode 2 in bits 4-5 => 0x20
        assert_eq!(u16::from_le_bytes([encoded[6], encoded[7]]), 0x20);
        assert_eq!(&encoded[8..40], &[0xFFu8; 32]); // schema_hash
        assert_eq!(&encoded[40..72], &[0x00u8; 32]); // worldline_id
        assert_eq!(u64::from_le_bytes(encoded[72..80].try_into().unwrap()), 0); // tick
        assert_eq!(&encoded[80..112], &[0xAAu8; 32]); // commit_hash
        assert_eq!(&encoded[176..208], &[0xBBu8; 32]); // emissions_digest

        // Roundtrip
        let (decoded, _) = decode_ttdr_v2(&encoded).unwrap();
        assert_eq!(decoded.header.tick, 0);
        assert_eq!(decoded.header.flags.receipt_mode(), ReceiptMode::Light);
        assert_eq!(decoded.header.commit_hash, [0xAAu8; 32]);
        assert_eq!(decoded.header.emissions_digest, [0xBBu8; 32]);
    }
}
