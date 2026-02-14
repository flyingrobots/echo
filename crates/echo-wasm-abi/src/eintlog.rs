// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Binary logging format for Echo intent logs (.eintlog).
//!
//! Format Spec (v1):
//! Header (48 bytes):
//! - Magic: "ELOG" (4 bytes)
//! - Version: u16 LE (2 bytes) = 1
//! - Flags: u16 LE (2 bytes) = 0
//! - Schema Hash: [u8; 32] (32 bytes)
//! - Reserved: [u8; 8] (8 bytes) = 0
//!
//! Frames (Repeated):
//! - Length: u32 LE (4 bytes)
//! - Payload: [u8; Length] (Must be valid EINT envelope)

#[cfg(feature = "std")]
use std::io::{self, Read, Write};

extern crate alloc;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

/// Magic bytes identifying an ELOG file: "ELOG".
pub const ELOG_MAGIC: [u8; 4] = *b"ELOG";
/// Current ELOG format version.
pub const ELOG_VERSION: u16 = 1;
/// Maximum allowed frame length (10 MiB).
pub const MAX_FRAME_LEN: usize = 10 * 1024 * 1024;

/// Header for an ELOG binary log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElogHeader {
    /// BLAKE3 hash of the schema used for this log.
    pub schema_hash: [u8; 32],
    /// Reserved flags (currently unused, should be 0).
    pub flags: u16,
}

#[cfg(feature = "std")]
fn read_exact_arr<const N: usize, R: Read>(r: &mut R) -> io::Result<[u8; N]> {
    let mut buf = [0u8; N];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// Reads and validates an ELOG header from a reader.
///
/// # Errors
/// Returns an error if the magic bytes are invalid or the version is unsupported.
#[cfg(feature = "std")]
pub fn read_elog_header<R: Read>(r: &mut R) -> io::Result<ElogHeader> {
    let magic = read_exact_arr::<4, _>(r)?;
    if magic != ELOG_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad ELOG magic"));
    }

    let version_bytes = read_exact_arr::<2, _>(r)?;
    let version = u16::from_le_bytes(version_bytes);
    if version != ELOG_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported ELOG version: {}", version),
        ));
    }

    let flags_bytes = read_exact_arr::<2, _>(r)?;
    let flags = u16::from_le_bytes(flags_bytes);
    if flags != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ELOG flags must be zero",
        ));
    }

    let schema_hash = read_exact_arr::<32, _>(r)?;
    let reserved = read_exact_arr::<8, _>(r)?;
    if reserved != [0u8; 8] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ELOG reserved bytes must be zero",
        ));
    }

    Ok(ElogHeader { schema_hash, flags })
}

/// Writes an ELOG header to a writer.
///
/// # Errors
/// Returns an error if writing fails.
#[cfg(feature = "std")]
pub fn write_elog_header<W: Write>(w: &mut W, hdr: &ElogHeader) -> io::Result<()> {
    w.write_all(&ELOG_MAGIC)?;
    w.write_all(&ELOG_VERSION.to_le_bytes())?;
    w.write_all(&hdr.flags.to_le_bytes())?;
    w.write_all(&hdr.schema_hash)?;
    w.write_all(&[0u8; 8])?; // reserved
    Ok(())
}

/// Reads a single frame from an ELOG file.
///
/// Returns `Ok(None)` only if EOF occurs while reading the 4-byte length prefix.
/// If EOF happens mid-payload, this returns `Err(UnexpectedEof)`.
///
/// Returns `Ok(Some(data))` for a valid frame.
///
/// # Errors
/// Returns an error if the frame is too large, reading fails, or the payload is truncated.
#[cfg(feature = "std")]
pub fn read_elog_frame<R: Read>(r: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut len_bytes = [0u8; 4];
    match r.read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > MAX_FRAME_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }

    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(Some(buf))
}

/// Writes a single frame to an ELOG file.
///
/// # Errors
/// Returns an error if the frame exceeds [`MAX_FRAME_LEN`] or writing fails.
#[cfg(feature = "std")]
pub fn write_elog_frame<W: Write>(w: &mut W, frame: &[u8]) -> io::Result<()> {
    if frame.len() > MAX_FRAME_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame too large",
        ));
    }
    let len = frame.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(frame)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    #[cfg(feature = "std")]
    fn test_elog_round_trip() {
        let hdr = ElogHeader {
            schema_hash: [0xAA; 32],
            flags: 0, // Must be zero per spec
        };
        let frame1 = b"frame1".to_vec();
        let frame2 = vec![0u8; 1000];

        let mut buf = Vec::new();
        write_elog_header(&mut buf, &hdr).unwrap();
        write_elog_frame(&mut buf, &frame1).unwrap();
        write_elog_frame(&mut buf, &frame2).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let read_hdr = read_elog_header(&mut cursor).unwrap();
        assert_eq!(read_hdr, hdr);

        let f1 = read_elog_frame(&mut cursor).unwrap().unwrap();
        assert_eq!(f1, frame1);

        let f2 = read_elog_frame(&mut cursor).unwrap().unwrap();
        assert_eq!(f2, frame2);

        assert!(read_elog_frame(&mut cursor).unwrap().is_none());
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_elog_rejects_large_frame() {
        let mut buf = Vec::new();
        let huge_len = (MAX_FRAME_LEN + 1) as u32;
        buf.extend_from_slice(&huge_len.to_le_bytes());

        let mut cursor = std::io::Cursor::new(buf);
        let res = read_elog_frame(&mut cursor);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "frame too large");
    }
}
