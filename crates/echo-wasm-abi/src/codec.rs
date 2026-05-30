// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal deterministic codec helpers (length-prefixed, LE scalars).

extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::string::String;
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
    /// Bool tag byte was not 0x00 or 0x01.
    #[error("invalid bool tag")]
    InvalidBoolTag,
    /// Trailing bytes remain after a complete decode.
    #[error("trailing bytes after decode")]
    Trailing,
}

/// Trait for deterministic encoding to bytes.
pub trait Encode {
    /// Encode into the provided writer.
    fn encode(&self, writer: &mut Writer) -> Result<(), CodecError>;
}

/// Trait for deterministic decoding from bytes.
pub trait Decode: Sized {
    /// Decode from the provided reader.
    fn decode(reader: &mut Reader<'_>) -> Result<Self, CodecError>;
}

/// Encode a value into a fresh Vec.
pub fn encode_to_vec<T: Encode>(value: &T) -> Result<Vec<u8>, CodecError> {
    let mut writer = Writer::default();
    value.encode(&mut writer)?;
    Ok(writer.into_vec())
}

/// Decode a value from a byte slice, failing if any trailing bytes remain.
pub fn decode_from_bytes<T: Decode>(bytes: &[u8]) -> Result<T, CodecError> {
    let mut reader = Reader::new(bytes);
    let value = T::decode(&mut reader)?;
    if reader.remaining() > 0 {
        return Err(CodecError::Trailing);
    }
    Ok(value)
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

    /// Write a little-endian i32.
    pub fn write_i32_le(&mut self, value: i32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a canonicalized little-endian f32.
    ///
    /// Applies the same canonicalization as `F32Scalar::new()` before writing:
    /// - NaN → `0x7fc00000` (positive quiet NaN)
    /// - subnormal → `+0.0` (`0x00000000`)
    /// - `-0.0` → `+0.0`
    /// - all other values pass through unchanged
    pub fn write_f32_le(&mut self, value: f32) {
        let canonical = canonicalize_f32(value);
        self.buf.extend_from_slice(&canonical.to_le_bytes());
    }

    /// Write a bool as a single byte: `0x00` = false, `0x01` = true.
    pub fn write_bool(&mut self, value: bool) {
        self.buf.push(u8::from(value));
    }

    /// Write an optional value: `0x00` = null, `0x01` + encoded payload = present.
    pub fn write_option<T, F>(&mut self, value: Option<T>, encode: F) -> Result<(), CodecError>
    where
        F: FnOnce(&mut Writer, T) -> Result<(), CodecError>,
    {
        match value {
            None => self.write_u8(0x00),
            Some(v) => {
                self.write_u8(0x01);
                encode(self, v)?;
            }
        }
        Ok(())
    }

    /// Write a list: `u32 LE` element count, then each element encoded inline.
    pub fn write_list<T, F>(&mut self, values: &[T], encode: F) -> Result<(), CodecError>
    where
        F: Fn(&mut Writer, &T) -> Result<(), CodecError>,
    {
        let count: u32 = values
            .len()
            .try_into()
            .map_err(|_| CodecError::LengthTooLarge)?;
        self.write_u32_le(count);
        for v in values {
            encode(self, v)?;
        }
        Ok(())
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

    /// Return the number of unread bytes remaining in the buffer.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
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

    /// Read a fixed-size byte array. Used for `no_std` ID fields where
    /// the GraphQL `ID` scalar maps to `[u8; 32]` instead of `String`.
    pub fn read_byte_array<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let chunk = self.take(N)?;
        chunk.try_into().map_err(|_| CodecError::OutOfBounds)
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
            .map(ToOwned::to_owned)
            .map_err(|_| CodecError::InvalidUtf8)
    }

    /// Read a little-endian i32.
    pub fn read_i32_le(&mut self) -> Result<i32, CodecError> {
        let chunk = self.take(4)?;
        let raw: [u8; 4] = chunk.try_into().map_err(|_| CodecError::OutOfBounds)?;
        Ok(i32::from_le_bytes(raw))
    }

    /// Read a little-endian f32 and canonicalize the result so identical
    /// values cannot have two distinct wire encodings.
    ///
    /// The writer canonicalizes (NaN -> 0x7fc00000, subnormal -> +0.0,
    /// -0.0 -> +0.0), so honest senders cannot produce non-canonical bytes.
    /// But untrusted EINT or query var payloads can: without the same
    /// canonicalization on decode, two distinct byte strings can represent
    /// the same intended float value and break the determinism contract.
    /// `canonicalize_f32` is idempotent on already-canonical inputs.
    pub fn read_f32_le(&mut self) -> Result<f32, CodecError> {
        let chunk = self.take(4)?;
        let raw: [u8; 4] = chunk.try_into().map_err(|_| CodecError::OutOfBounds)?;
        Ok(canonicalize_f32(f32::from_le_bytes(raw)))
    }

    /// Read a bool from a single byte (`0x00` = false, `0x01` = true).
    pub fn read_bool(&mut self) -> Result<bool, CodecError> {
        match self.read_u8()? {
            0x00 => Ok(false),
            0x01 => Ok(true),
            _ => Err(CodecError::InvalidBoolTag),
        }
    }

    /// Read an optional value: `0x00` = `None`, `0x01` = `Some(decode(r))`.
    pub fn read_option<T, F>(&mut self, decode: F) -> Result<Option<T>, CodecError>
    where
        F: FnOnce(&mut Reader<'_>) -> Result<T, CodecError>,
    {
        match self.read_u8()? {
            0x00 => Ok(None),
            0x01 => Ok(Some(decode(self)?)),
            _ => Err(CodecError::InvalidBoolTag),
        }
    }

    /// Read a list: `u32 LE` element count, then decode each element.
    ///
    /// Capacity allocation is bounded by the remaining buffer length so a
    /// malformed payload claiming `count = 0xFFFF_FFFF` followed by zero
    /// bytes cannot force a multi-gigabyte `Vec::with_capacity` (DoS) before
    /// element validation runs. Honest decoders are unaffected: the cap is
    /// looser than any real workload would need.
    pub fn read_list<T, F>(&mut self, decode: F) -> Result<Vec<T>, CodecError>
    where
        F: Fn(&mut Reader<'_>) -> Result<T, CodecError>,
    {
        let count = self.read_u32_le()? as usize;
        let remaining = self.bytes.len().saturating_sub(self.offset);
        // A list element occupies at least 1 byte (e.g. a u8 tag); cap the
        // initial allocation at the byte budget so we never pre-reserve more
        // entries than the payload could possibly contain.
        let initial_capacity = core::cmp::min(count, remaining);
        let mut out = Vec::with_capacity(initial_capacity);
        for _ in 0..count {
            out.push(decode(self)?);
        }
        Ok(out)
    }
}

/// Canonicalize an `f32` to a deterministic bit pattern.
///
/// Mirrors `F32Scalar::new()` from `warp-math` without taking that crate as a dependency:
/// - NaN (any variant) → `0x7fc00000` (positive quiet NaN)
/// - subnormal → `+0.0` (`0x00000000`)
/// - `-0.0` → `+0.0`
/// - all other values pass through unchanged
#[inline]
#[must_use]
pub fn canonicalize_f32(v: f32) -> f32 {
    if v.is_nan() {
        f32::from_bits(0x7fc0_0000)
    } else if v.is_subnormal() {
        0.0_f32
    } else {
        // Maps -0.0 → +0.0; all other normal values are unchanged.
        v + 0.0_f32
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
    let scaled = f64::from(value) * ((1u64 << 32) as f64);
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_closure_for_method_calls
)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn roundtrip<T, W, R>(write: W, read: R) -> T
    where
        W: FnOnce(&mut Writer),
        R: FnOnce(&mut Reader<'_>) -> Result<T, CodecError>,
    {
        let mut w = Writer::default();
        write(&mut w);
        let buf = w.into_vec();
        let mut r = Reader::new(&buf);
        read(&mut r).expect("decode failed")
    }

    // ── i32 ──────────────────────────────────────────────────────────────────

    #[test]
    fn i32_roundtrip_zero() {
        let v = roundtrip(|w| w.write_i32_le(0), |r| r.read_i32_le());
        assert_eq!(v, 0);
    }

    #[test]
    fn i32_roundtrip_positive() {
        let v = roundtrip(|w| w.write_i32_le(42), |r| r.read_i32_le());
        assert_eq!(v, 42);
    }

    #[test]
    fn i32_roundtrip_negative() {
        let v = roundtrip(|w| w.write_i32_le(-1), |r| r.read_i32_le());
        assert_eq!(v, -1);
    }

    #[test]
    fn i32_roundtrip_min() {
        let v = roundtrip(|w| w.write_i32_le(i32::MIN), |r| r.read_i32_le());
        assert_eq!(v, i32::MIN);
    }

    #[test]
    fn i32_roundtrip_max() {
        let v = roundtrip(|w| w.write_i32_le(i32::MAX), |r| r.read_i32_le());
        assert_eq!(v, i32::MAX);
    }

    #[test]
    fn i32_wire_bytes() {
        // 0x01020304 in LE → [0x04, 0x03, 0x02, 0x01]
        let mut w = Writer::default();
        w.write_i32_le(0x0102_0304);
        assert_eq!(w.into_vec(), [0x04, 0x03, 0x02, 0x01]);
    }

    // ── f32 canonicalization ─────────────────────────────────────────────────

    #[test]
    fn f32_canonicalize_nan_any_becomes_quiet_nan() {
        // All NaN variants must produce the same canonical bit pattern 0x7fc00000.
        let quiet_nan = f32::from_bits(0x7fc0_0000);
        let signaling_nan = f32::from_bits(0x7f80_0001);
        let neg_nan = f32::from_bits(0xffc0_0000);
        assert_eq!(canonicalize_f32(quiet_nan).to_bits(), 0x7fc0_0000);
        assert_eq!(canonicalize_f32(signaling_nan).to_bits(), 0x7fc0_0000);
        assert_eq!(canonicalize_f32(neg_nan).to_bits(), 0x7fc0_0000);
    }

    #[test]
    fn f32_canonicalize_subnormal_becomes_positive_zero() {
        // Smallest positive subnormal: 0x00000001.
        let subnormal = f32::from_bits(0x0000_0001);
        assert!(subnormal.is_subnormal());
        assert_eq!(canonicalize_f32(subnormal).to_bits(), 0x0000_0000);
    }

    #[test]
    fn f32_canonicalize_negative_zero_becomes_positive_zero() {
        let neg_zero = -0.0_f32;
        assert_eq!(neg_zero.to_bits(), 0x8000_0000);
        assert_eq!(canonicalize_f32(neg_zero).to_bits(), 0x0000_0000);
    }

    #[test]
    fn f32_canonicalize_positive_infinity_unchanged() {
        let inf = f32::INFINITY;
        assert_eq!(canonicalize_f32(inf).to_bits(), inf.to_bits());
    }

    #[test]
    fn f32_canonicalize_normal_values_unchanged() {
        for v in [1.0_f32, -1.0, 42.5, f32::MAX, f32::MIN] {
            assert_eq!(canonicalize_f32(v).to_bits(), v.to_bits(), "value: {v}");
        }
    }

    #[test]
    fn f32_roundtrip_normal() {
        let v = roundtrip(|w| w.write_f32_le(1.5), |r| r.read_f32_le());
        assert_eq!(v.to_bits(), 1.5_f32.to_bits());
    }

    #[test]
    fn f32_roundtrip_nan_canonicalizes() {
        // write_f32_le must canonicalize; the decoded bits must be 0x7fc00000.
        let v = roundtrip(|w| w.write_f32_le(f32::NAN), |r| r.read_f32_le());
        assert_eq!(v.to_bits(), 0x7fc0_0000);
    }

    #[test]
    fn f32_roundtrip_subnormal_canonicalizes_to_zero() {
        let subnormal = f32::from_bits(0x0000_0001);
        let v = roundtrip(|w| w.write_f32_le(subnormal), |r| r.read_f32_le());
        assert_eq!(v.to_bits(), 0x0000_0000);
    }

    #[test]
    fn f32_roundtrip_negative_zero_canonicalizes_to_positive_zero() {
        let v = roundtrip(|w| w.write_f32_le(-0.0_f32), |r| r.read_f32_le());
        assert_eq!(v.to_bits(), 0x0000_0000);
    }

    #[test]
    fn f32_roundtrip_positive_infinity() {
        let v = roundtrip(|w| w.write_f32_le(f32::INFINITY), |r| r.read_f32_le());
        assert!(v.is_infinite() && v.is_sign_positive());
    }

    // ── bool ─────────────────────────────────────────────────────────────────

    #[test]
    fn bool_roundtrip_false() {
        let v = roundtrip(|w| w.write_bool(false), |r| r.read_bool());
        assert!(!v);
    }

    #[test]
    fn bool_roundtrip_true() {
        let v = roundtrip(|w| w.write_bool(true), |r| r.read_bool());
        assert!(v);
    }

    #[test]
    fn bool_wire_bytes() {
        let mut w = Writer::default();
        w.write_bool(false);
        w.write_bool(true);
        assert_eq!(w.into_vec(), [0x00, 0x01]);
    }

    #[test]
    fn bool_invalid_tag_returns_error() {
        let buf = [0x02_u8];
        let mut r = Reader::new(&buf);
        assert_eq!(r.read_bool(), Err(CodecError::InvalidBoolTag));
    }

    // ── option ───────────────────────────────────────────────────────────────

    #[test]
    fn option_roundtrip_none() {
        let mut w = Writer::default();
        w.write_option::<u32, _>(None, |w, v| {
            w.write_u32_le(v);
            Ok(())
        })
        .unwrap();
        let buf = w.into_vec();
        assert_eq!(buf, [0x00]);
        let mut r = Reader::new(&buf);
        let v: Option<u32> = r.read_option(|r| r.read_u32_le()).unwrap();
        assert_eq!(v, None);
    }

    #[test]
    fn option_roundtrip_some() {
        let mut w = Writer::default();
        w.write_option(Some(999_u32), |w, v| {
            w.write_u32_le(v);
            Ok(())
        })
        .unwrap();
        let buf = w.into_vec();
        // presence tag 0x01, then 999u32 LE = [0xe7, 0x03, 0x00, 0x00]
        assert_eq!(buf[0], 0x01);
        let mut r = Reader::new(&buf);
        let v: Option<u32> = r.read_option(|r| r.read_u32_le()).unwrap();
        assert_eq!(v, Some(999));
    }

    #[test]
    fn option_nested_string() {
        let input: Option<&str> = Some("hello");
        let mut w = Writer::default();
        w.write_option(input, |w, v| w.write_string(v, usize::MAX))
            .unwrap();
        let buf = w.into_vec();
        let mut r = Reader::new(&buf);
        let v = r.read_option(|r| r.read_string(usize::MAX)).unwrap();
        assert_eq!(v, Some("hello".to_owned()));
    }

    // ── list ─────────────────────────────────────────────────────────────────

    #[test]
    fn list_roundtrip_empty() {
        let input: &[u32] = &[];
        let mut w = Writer::default();
        w.write_list(input, |w, v| {
            w.write_u32_le(*v);
            Ok(())
        })
        .unwrap();
        let buf = w.into_vec();
        // count = 0 → 4 zero bytes
        assert_eq!(buf, [0x00, 0x00, 0x00, 0x00]);
        let mut r = Reader::new(&buf);
        let v: Vec<u32> = r.read_list(|r| r.read_u32_le()).unwrap();
        assert_eq!(v, [] as [u32; 0]);
    }

    #[test]
    fn list_roundtrip_three_elements() {
        let input = [1_u32, 2, 3];
        let mut w = Writer::default();
        w.write_list(&input, |w, v| {
            w.write_u32_le(*v);
            Ok(())
        })
        .unwrap();
        let buf = w.into_vec();
        let mut r = Reader::new(&buf);
        let v: Vec<u32> = r.read_list(|r| r.read_u32_le()).unwrap();
        assert_eq!(v, [1, 2, 3]);
    }

    #[test]
    fn list_roundtrip_strings() {
        let input = ["alpha", "beta", "gamma"];
        let mut w = Writer::default();
        w.write_list(&input, |w, v| w.write_string(v, usize::MAX))
            .unwrap();
        let buf = w.into_vec();
        let mut r = Reader::new(&buf);
        let v: Vec<String> = r.read_list(|r| r.read_string(usize::MAX)).unwrap();
        assert_eq!(v, ["alpha", "beta", "gamma"]);
    }

    // ── canonicalize_f32 public function ────────────────────────────────────

    #[test]
    fn canonicalize_f32_all_nan_bits_map_to_canonical() {
        // Exhaustively sample a range of NaN bit patterns.
        for payload in [0u32, 1, 0x003f_ffff, 0x0040_0000, 0x007f_ffff] {
            // Quiet NaN: exponent all-1s, fraction MSB set, positive
            let bits = 0x7f80_0000 | 0x0040_0000 | payload;
            let nan = f32::from_bits(bits);
            assert!(nan.is_nan());
            assert_eq!(
                canonicalize_f32(nan).to_bits(),
                0x7fc0_0000,
                "bits={bits:#010x}"
            );
        }
    }
}
