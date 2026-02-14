// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal deterministic codec helpers (length-prefixed, LE scalars).

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str;
use thiserror::Error;

/// Errors produced by codec readers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodecError {
    /// Attempted to read beyond the end of the buffer.
    #[error("buffer too short")]
    OutOfBounds,
    /// UTF-8 decoding failed.
    #[error("invalid utf-8")]
    InvalidUtf8,
    /// String length exceeded max bound.
    #[error("string too long")]
    StringTooLong,
    /// Length prefix exceeded max bound.
    #[error("length too large")]
    LengthTooLarge,
    /// Enum decoding failed.
    #[error("invalid enum value")]
    InvalidEnum,
}

/// Trait for deterministic encoding to bytes.
pub trait Encode {
    /// Encode into the provided writer.
    fn encode(&self, writer: &mut Writer) -> Result<(), CodecError>;
}

/// Trait for deterministic decoding from bytes.
pub trait Decode: Sized {
    /// Decode from the provided reader.
    fn decode(reader: &mut Reader) -> Result<Self, CodecError>;
}

/// Encode a value into a fresh Vec.
pub fn encode_to_vec<T: Encode>(value: &T) -> Result<Vec<u8>, CodecError> {
    let mut writer = Writer::default();
    value.encode(&mut writer)?;
    Ok(writer.into_vec())
}

/// Decode a value from a byte slice.
pub fn decode_from_bytes<T: Decode>(bytes: &[u8]) -> Result<T, CodecError> {
    let mut reader = Reader::new(bytes);
    T::decode(&mut reader)
}

/// Deterministic writer for little-endian scalars and length-prefixed bytes.
#[derive(Debug, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// Create a new writer with a pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
        }
    }

    /// Write raw bytes.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Write a single byte.
    pub fn write_u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    /// Write a little-endian u32.
    pub fn write_u32_le(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a little-endian u16.
    pub fn write_u16_le(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a little-endian i64.
    pub fn write_i64_le(&mut self, value: i64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write length-prefixed bytes (u32 LE length).
    pub fn write_len_prefixed_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        let len: u32 = bytes
            .len()
            .try_into()
            .map_err(|_| CodecError::LengthTooLarge)?;
        self.write_u32_le(len);
        self.write_bytes(bytes);
        Ok(())
    }

    /// Write a length-prefixed UTF-8 string with a max bound.
    pub fn write_string(&mut self, value: &str, max_len: usize) -> Result<(), CodecError> {
        let bytes = value.as_bytes();
        if bytes.len() > max_len {
            return Err(CodecError::StringTooLong);
        }
        self.write_len_prefixed_bytes(bytes)
    }

    /// Consume the writer and return the buffer.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

/// Deterministic reader for little-endian scalars and length-prefixed bytes.
#[derive(Debug)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    /// Create a reader over the provided byte slice.
    #[must_use]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(CodecError::OutOfBounds)?;
        if end > self.bytes.len() {
            return Err(CodecError::OutOfBounds);
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    /// Read a little-endian u32.
    pub fn read_u32_le(&mut self) -> Result<u32, CodecError> {
        let chunk = self.take(4)?;
        let raw: [u8; 4] = chunk.try_into().map_err(|_| CodecError::OutOfBounds)?;
        Ok(u32::from_le_bytes(raw))
    }

    /// Read a single byte.
    pub fn read_u8(&mut self) -> Result<u8, CodecError> {
        let chunk = self.take(1)?;
        Ok(chunk[0])
    }

    /// Read a little-endian u16.
    pub fn read_u16_le(&mut self) -> Result<u16, CodecError> {
        let chunk = self.take(2)?;
        let raw: [u8; 2] = chunk.try_into().map_err(|_| CodecError::OutOfBounds)?;
        Ok(u16::from_le_bytes(raw))
    }

    /// Read a little-endian i64.
    pub fn read_i64_le(&mut self) -> Result<i64, CodecError> {
        let chunk = self.take(8)?;
        let raw: [u8; 8] = chunk.try_into().map_err(|_| CodecError::OutOfBounds)?;
        Ok(i64::from_le_bytes(raw))
    }

    /// Read a length-prefixed byte slice with a max bound.
    pub fn read_len_prefixed_bytes(&mut self, max_len: usize) -> Result<&'a [u8], CodecError> {
        let len = self.read_u32_le()? as usize;
        if len > max_len {
            return Err(CodecError::LengthTooLarge);
        }
        self.take(len)
    }

    /// Read a length-prefixed UTF-8 string with a max bound.
    pub fn read_string(&mut self, max_len: usize) -> Result<String, CodecError> {
        let bytes = self.read_len_prefixed_bytes(max_len)?;
        str::from_utf8(bytes)
            .map(|s| s.to_string())
            .map_err(|_| CodecError::InvalidUtf8)
    }
}

/// Convert an integer to Q32.32 fixed-point representation.
///
/// Valid input range is `i32::MIN..=i32::MAX`. Values outside this range
/// saturate to `i64::MIN` or `i64::MAX` respectively.
#[inline]
#[must_use]
pub fn fx_from_i64(n: i64) -> i64 {
    // Q32.32 can only represent integers in i32 range without overflow
    if n > i64::from(i32::MAX) {
        i64::MAX
    } else if n < i64::from(i32::MIN) {
        i64::MIN
    } else {
        n << 32
    }
}

/// Convert f32 to Q32.32 using truncation toward zero.
#[inline]
#[must_use]
pub fn fx_from_f32(value: f32) -> i64 {
    let scaled = (value as f64) * ((1u64 << 32) as f64);
    if scaled.is_nan() {
        0
    } else if scaled.is_infinite() {
        if scaled.is_sign_positive() {
            i64::MAX
        } else {
            i64::MIN
        }
    } else {
        scaled.trunc() as i64
    }
}

/// Convert integer components to Q32.32 raw vector.
#[inline]
#[must_use]
pub fn vec3_fx_from_i64(x: i64, y: i64, z: i64) -> [i64; 3] {
    [fx_from_i64(x), fx_from_i64(y), fx_from_i64(z)]
}

/// Convert f32 components to Q32.32 raw vector using truncation toward zero.
#[inline]
#[must_use]
pub fn vec3_fx_from_f32(x: f32, y: f32, z: f32) -> [i64; 3] {
    [fx_from_f32(x), fx_from_f32(y), fx_from_f32(z)]
}
