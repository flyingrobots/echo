// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC file reading and error types.
//!
//! This module provides low-level reading primitives for WSC files.
//! For zero-copy access, see the [`view`](super::view) module.

use std::io;

use bytemuck::Pod;
use thiserror::Error;

use super::types::WscHeader;

/// Errors that can occur when reading or validating WSC files.
#[derive(Debug, Error)]
pub enum ReadError {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// File is too small to contain a valid header.
    #[error("file too small: {size} bytes, minimum {minimum}")]
    FileTooSmall {
        /// Actual file size.
        size: usize,
        /// Minimum required size.
        minimum: usize,
    },

    /// Magic bytes don't match expected value.
    #[error("invalid magic: expected {expected:?}, got {actual:?}")]
    InvalidMagic {
        /// Expected magic bytes.
        expected: [u8; 8],
        /// Actual magic bytes found.
        actual: [u8; 8],
    },

    /// Section offset or length would extend past end of file.
    #[error(
        "section {name} out of bounds: offset {offset}, length {length}, file size {file_size}"
    )]
    SectionOutOfBounds {
        /// Section name for diagnostics.
        name: &'static str,
        /// Section offset.
        offset: u64,
        /// Section length in bytes.
        length: u64,
        /// Total file size.
        file_size: usize,
    },

    /// WARP index out of bounds.
    #[error("warp index {index} out of bounds, file contains {count} warps")]
    WarpIndexOutOfBounds {
        /// Requested index.
        index: usize,
        /// Number of WARPs in file.
        count: usize,
    },

    /// Section alignment violation.
    #[error("section {name} at offset {offset} is not {alignment}-byte aligned")]
    AlignmentViolation {
        /// Section name.
        name: &'static str,
        /// Section offset.
        offset: u64,
        /// Required alignment.
        alignment: usize,
    },

    /// Index/data table size mismatch.
    #[error("{index_name} has {index_len} entries but {data_name} expects {expected_len}")]
    IndexMismatch {
        /// Index table name.
        index_name: &'static str,
        /// Index table length.
        index_len: usize,
        /// Data table name.
        data_name: &'static str,
        /// Expected length based on data.
        expected_len: usize,
    },

    /// Attachment tag is invalid.
    #[error("invalid attachment tag {tag} at index {index}")]
    InvalidAttachmentTag {
        /// The invalid tag value.
        tag: u8,
        /// Index of the attachment.
        index: usize,
    },

    /// Blob reference out of bounds.
    #[error("blob reference out of bounds: offset {offset}, length {length}, blob section size {blob_size}")]
    BlobOutOfBounds {
        /// Blob offset.
        offset: u64,
        /// Blob length.
        length: u64,
        /// Total blob section size.
        blob_size: usize,
    },

    /// Data not properly aligned for the target type.
    #[error("alignment error: {0}")]
    Alignment(#[from] bytemuck::PodCastError),

    /// Index range extends past its data table.
    #[error(
        "{index_name}[{entry_index}] range ({start}..{end}) exceeds {data_name} length {data_len}"
    )]
    IndexRangeOutOfBounds {
        /// Name of the index table.
        index_name: &'static str,
        /// Entry position within the index table.
        entry_index: usize,
        /// Range start.
        start: u64,
        /// Range end (start + len).
        end: u64,
        /// Name of the data table.
        data_name: &'static str,
        /// Length of the data table.
        data_len: usize,
    },
}

/// Validates that a byte slice contains a valid WSC header.
///
/// # Errors
///
/// Returns [`ReadError::FileTooSmall`] if the data is shorter than the header size.
/// Returns [`ReadError::InvalidMagic`] if the magic bytes don't match.
pub fn validate_header(data: &[u8]) -> Result<&WscHeader, ReadError> {
    let header_size = std::mem::size_of::<WscHeader>();

    if data.len() < header_size {
        return Err(ReadError::FileTooSmall {
            size: data.len(),
            minimum: header_size,
        });
    }

    // Use bytemuck for safe transmutation
    let header: &WscHeader = bytemuck::from_bytes(&data[..header_size]);

    if header.magic != WscHeader::MAGIC_V1 {
        return Err(ReadError::InvalidMagic {
            expected: WscHeader::MAGIC_V1,
            actual: header.magic,
        });
    }

    Ok(header)
}

/// Reads a slice of `Pod` structs from a byte buffer.
///
/// # Arguments
///
/// * `data` - The full file data
/// * `offset` - Byte offset to start of section
/// * `count` - Number of elements to read
/// * `name` - Section name for error messages
///
/// # Errors
///
/// Returns [`ReadError::SectionOutOfBounds`] if the slice would extend past the buffer.
/// Returns [`ReadError::Alignment`] if the data is not properly aligned for type `T`.
#[allow(clippy::cast_possible_truncation)] // We bounds-check against data.len() first
pub fn read_slice<'a, T: Pod>(
    data: &'a [u8],
    offset: u64,
    count: u64,
    name: &'static str,
) -> Result<&'a [T], ReadError> {
    let elem_size = std::mem::size_of::<T>();
    let byte_len = count.saturating_mul(elem_size as u64);
    let end = offset.saturating_add(byte_len);

    if end > data.len() as u64 {
        return Err(ReadError::SectionOutOfBounds {
            name,
            offset,
            length: byte_len,
            file_size: data.len(),
        });
    }

    let slice = &data[offset as usize..end as usize];

    // Use bytemuck for safe transmutation
    let typed: &[T] = bytemuck::try_cast_slice(slice)?;

    Ok(typed)
}

/// Reads a byte slice from a buffer.
///
/// # Errors
///
/// Returns [`ReadError::SectionOutOfBounds`] if the slice would extend past the buffer.
#[allow(clippy::cast_possible_truncation)] // We bounds-check against data.len() first
pub fn read_bytes<'a>(
    data: &'a [u8],
    offset: u64,
    length: u64,
    name: &'static str,
) -> Result<&'a [u8], ReadError> {
    let end = offset.saturating_add(length);

    if end > data.len() as u64 {
        return Err(ReadError::SectionOutOfBounds {
            name,
            offset,
            length,
            file_size: data.len(),
        });
    }

    Ok(&data[offset as usize..end as usize])
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn validate_header_rejects_too_small() {
        let data = [0u8; 10];
        let err = validate_header(&data).unwrap_err();
        assert!(matches!(err, ReadError::FileTooSmall { .. }));
    }

    #[test]
    fn validate_header_rejects_bad_magic() {
        let mut data = [0u8; 128];
        data[0..8].copy_from_slice(b"NOTAWSC!");
        let err = validate_header(&data).unwrap_err();
        assert!(matches!(err, ReadError::InvalidMagic { .. }));
    }

    #[test]
    fn validate_header_accepts_valid() {
        let mut data = [0u8; 128];
        data[0..8].copy_from_slice(&WscHeader::MAGIC_V1);
        let header = validate_header(&data).unwrap();
        assert_eq!(header.magic, WscHeader::MAGIC_V1);
    }
}
