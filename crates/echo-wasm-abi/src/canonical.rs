// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic CBOR encoder/decoder (subset, canonical) for WASM ABI payloads.
//! Copied verbatim in spirit from `echo-session-proto` canonical encoder to avoid
//! divergence; supports definite lengths only, no tags, sorted map keys, and the
//! smallest-width integers/floats that round-trip. Limits integers to i64/u64 as
//! supported by `ciborium::value::Integer`.

use ciborium::value::{Integer, Value};
use half::f16;

/// Errors produced by the canonical CBOR encoder/decoder.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CanonError {
    /// Input exhausted before value completed.
    #[error("incomplete input")]
    Incomplete,
    /// Extra bytes present after decoding a single value.
    #[error("trailing bytes after value")]
    Trailing,
    /// CBOR tags are disallowed in this canonical subset.
    #[error("tags not allowed")]
    Tag,
    /// Indefinite lengths are disallowed.
    #[error("indefinite length not allowed")]
    Indefinite,
    /// Integer encoded with non-minimal width.
    #[error("non-canonical integer width")]
    NonCanonicalInt,
    /// Float encoded with non-minimal width.
    #[error("non-canonical float width")]
    NonCanonicalFloat,
    /// Float used where an integer canonical form is required.
    #[error("float encodes integral value; must be integer")]
    FloatShouldBeInt,
    /// Map keys not strictly increasing by bytewise order.
    #[error("map keys not strictly increasing")]
    MapKeyOrder,
    /// Duplicate map key encountered.
    #[error("duplicate map key")]
    MapKeyDuplicate,
    /// Generic decode error with detail.
    #[error("decode error: {0}")]
    Decode(String),
}

type Result<T> = std::result::Result<T, CanonError>;

/// Encode a `ciborium::value::Value` to deterministic CBOR bytes.
pub fn encode_value(val: &Value) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    enc_value(val, &mut out)?;
    Ok(out)
}

/// Decode deterministic CBOR bytes into a `ciborium::value::Value`.
pub fn decode_value(bytes: &[u8]) -> Result<Value> {
    let mut idx = 0usize;
    let v = dec_value(bytes, &mut idx)?;
    if idx != bytes.len() {
        return Err(CanonError::Trailing);
    }
    Ok(v)
}

fn enc_value(v: &Value, out: &mut Vec<u8>) -> Result<()> {
    match v {
        Value::Bool(b) => {
            out.push(if *b { 0xf5 } else { 0xf4 });
        }
        Value::Null => out.push(0xf6),
        Value::Integer(n) => enc_int(i128::from(*n), out),
        Value::Float(f) => enc_float(*f, out),
        Value::Text(s) => enc_text(s, out)?,
        Value::Bytes(b) => enc_bytes(b, out)?,
        Value::Array(items) => {
            enc_len(4, items.len() as u64, out);
            for it in items {
                enc_value(it, out)?;
            }
        }
        Value::Map(entries) => {
            let mut buf: Vec<(Value, Value, Vec<u8>)> = Vec::with_capacity(entries.len());
            for (k, v) in entries {
                let mut kb = Vec::new();
                enc_value(k, &mut kb)?;
                buf.push((k.clone(), v.clone(), kb));
            }

            buf.sort_by(|a, b| a.2.cmp(&b.2));

            for win in buf.windows(2) {
                if win[0].2 == win[1].2 {
                    return Err(CanonError::MapKeyDuplicate);
                }
            }

            enc_len(5, buf.len() as u64, out);
            for (_k, v, kb) in buf {
                out.extend_from_slice(&kb);
                enc_value(&v, out)?;
            }
        }
        Value::Tag(_, _) => return Err(CanonError::Tag),
        _ => return Err(CanonError::Decode("unsupported simple value".into())),
    }
    Ok(())
}

fn enc_len(major: u8, len: u64, out: &mut Vec<u8>) {
    write_major(major, len as u128, out);
}

fn enc_int(n: i128, out: &mut Vec<u8>) {
    if n >= 0 {
        write_major(0, n as u128, out);
    } else {
        let m = (-1 - n) as u128;
        write_major(1, m, out);
    }
}

fn enc_float(f: f64, out: &mut Vec<u8>) {
    if f.is_nan() {
        let h = f16::NAN;
        write_half(h, out);
        return;
    }
    if f.is_infinite() {
        let h = if f.is_sign_positive() {
            f16::INFINITY
        } else {
            f16::NEG_INFINITY
        };
        write_half(h, out);
        return;
    }
    if f.fract() == 0.0 {
        // i128 range: approximately ±1.7e38; f64 can represent up to ±1.8e308
        // Check range before casting to avoid overflow/UB
        const I128_MAX_F: f64 = i128::MAX as f64;
        const I128_MIN_F: f64 = i128::MIN as f64;
        if f >= I128_MIN_F && f <= I128_MAX_F {
            let i = f as i128;
            if i as f64 == f {
                enc_int(i, out);
                return;
            }
        }
    }
    let h = f16::from_f64(f);
    if h.to_f64() == f {
        write_half(h, out);
        return;
    }
    let f32v = f as f32;
    if f32v as f64 == f {
        write_f32(f32v, out);
    } else {
        write_f64(f, out);
    }
}

fn write_half(h: f16, out: &mut Vec<u8>) {
    out.push(0xf9);
    out.extend_from_slice(&h.to_bits().to_be_bytes());
}

fn write_f32(fv: f32, out: &mut Vec<u8>) {
    out.push(0xfa);
    out.extend_from_slice(&fv.to_be_bytes());
}

fn write_f64(fv: f64, out: &mut Vec<u8>) {
    out.push(0xfb);
    out.extend_from_slice(&fv.to_be_bytes());
}

fn enc_bytes(b: &[u8], out: &mut Vec<u8>) -> Result<()> {
    enc_len(2, b.len() as u64, out);
    out.extend_from_slice(b);
    Ok(())
}

fn enc_text(s: &str, out: &mut Vec<u8>) -> Result<()> {
    enc_len(3, s.len() as u64, out);
    out.extend_from_slice(s.as_bytes());
    Ok(())
}

fn write_major(major: u8, n: u128, out: &mut Vec<u8>) {
    debug_assert!(major <= 7);
    match n {
        0..=23 => out.push((major << 5) | n as u8),
        24..=0xff => {
            out.push((major << 5) | 24);
            out.push(n as u8);
        }
        0x100..=0xffff => {
            out.push((major << 5) | 25);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push((major << 5) | 26);
            out.extend_from_slice(&(n as u32).to_be_bytes());
        }
        _ => {
            out.push((major << 5) | 27);
            out.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
}

fn dec_value(bytes: &[u8], idx: &mut usize) -> Result<Value> {
    fn need(bytes: &[u8], idx: usize, n: usize) -> Result<()> {
        if bytes.len().saturating_sub(idx) < n {
            Err(CanonError::Incomplete)
        } else {
            Ok(())
        }
    }

    need(bytes, *idx, 1)?;
    let b0 = bytes[*idx];
    *idx += 1;
    let major = b0 >> 5;
    let info = b0 & 0x1f;

    fn read_uint(bytes: &[u8], idx: &mut usize, nbytes: usize) -> Result<u64> {
        need(bytes, *idx, nbytes)?;
        let mut val = 0u64;
        for _ in 0..nbytes {
            val = (val << 8) | (bytes[*idx] as u64);
            *idx += 1;
        }
        Ok(val)
    }

    fn read_f(bytes: &[u8], idx: &mut usize, nbytes: usize) -> Result<f64> {
        need(bytes, *idx, nbytes)?;
        let slice = &bytes[*idx..*idx + nbytes];
        *idx += nbytes;
        Ok(match nbytes {
            2 => f16::from_bits(u16::from_be_bytes(
                slice
                    .try_into()
                    .map_err(|_| CanonError::Decode("invalid f16 bytes".into()))?,
            ))
            .to_f64(),
            4 => f32::from_be_bytes(
                slice
                    .try_into()
                    .map_err(|_| CanonError::Decode("invalid f32 bytes".into()))?,
            ) as f64,
            8 => f64::from_be_bytes(
                slice
                    .try_into()
                    .map_err(|_| CanonError::Decode("invalid f64 bytes".into()))?,
            ),
            _ => unreachable!(),
        })
    }

    fn read_len(bytes: &[u8], idx: &mut usize, info: u8) -> Result<u64> {
        match info {
            0..=23 => Ok(info as u64),
            24 => read_uint(bytes, idx, 1),
            25 => read_uint(bytes, idx, 2),
            26 => read_uint(bytes, idx, 4),
            27 => read_uint(bytes, idx, 8),
            31 => Err(CanonError::Indefinite),
            _ => Err(CanonError::Decode("invalid length info".into())),
        }
    }

    match major {
        0 | 1 => {
            let n = read_len(bytes, idx, info)?;
            let i = if major == 0 {
                Integer::from(n)
            } else {
                let neg = -(1i128 + n as i128);
                let signed = i64::try_from(neg)
                    .map_err(|_| CanonError::Decode("integer out of range".into()))?;
                Integer::from(signed)
            };
            // verify canonical width
            match info {
                24 if n <= 23 => return Err(CanonError::NonCanonicalInt),
                25 if n <= 0xff => return Err(CanonError::NonCanonicalInt),
                26 if n <= 0xffff => return Err(CanonError::NonCanonicalInt),
                27 if n <= 0xffff_ffff => return Err(CanonError::NonCanonicalInt),
                _ => {}
            }
            Ok(Value::Integer(i))
        }
        2 | 3 => {
            let len = read_len(bytes, idx, info)?;
            let len = len as usize;
            need(bytes, *idx, len)?;
            let data = &bytes[*idx..*idx + len];
            *idx += len;
            if major == 2 {
                Ok(Value::Bytes(data.to_vec()))
            } else {
                let s = std::str::from_utf8(data)
                    .map_err(|e| CanonError::Decode(format!("utf8: {}", e)))?;
                Ok(Value::Text(s.to_string()))
            }
        }
        4 => {
            let len = read_len(bytes, idx, info)? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(dec_value(bytes, idx)?);
            }
            Ok(Value::Array(items))
        }
        5 => {
            let len = read_len(bytes, idx, info)? as usize;
            let mut entries = Vec::with_capacity(len);
            let mut last_key: Option<Vec<u8>> = None;
            for _ in 0..len {
                let key_start = *idx;
                let k = dec_value(bytes, idx)?;
                let key_end = *idx;
                let kb = &bytes[key_start..key_end];
                if let Some(prev) = &last_key {
                    if kb == prev.as_slice() {
                        return Err(CanonError::MapKeyDuplicate);
                    } else if kb < prev.as_slice() {
                        return Err(CanonError::MapKeyOrder);
                    }
                }
                last_key = Some(kb.to_vec());
                let v = dec_value(bytes, idx)?;
                entries.push((k, v));
            }
            Ok(Value::Map(entries))
        }
        7 => match info {
            20 => Ok(Value::Bool(false)),
            21 => Ok(Value::Bool(true)),
            22 => Ok(Value::Null),
            25 => {
                let f = read_f(bytes, idx, 2)?;
                if is_exact_int(f) {
                    return Err(CanonError::FloatShouldBeInt);
                }
                Ok(Value::Float(f))
            }
            26 => {
                let f = read_f(bytes, idx, 4)?;
                if is_exact_int(f) {
                    return Err(CanonError::FloatShouldBeInt);
                }
                if can_fit_f16(f) {
                    return Err(CanonError::NonCanonicalFloat);
                }
                Ok(Value::Float(f))
            }
            27 => {
                let f = read_f(bytes, idx, 8)?;
                if is_exact_int(f) {
                    return Err(CanonError::FloatShouldBeInt);
                }
                if can_fit_f16(f) {
                    return Err(CanonError::NonCanonicalFloat);
                }
                if can_fit_f32(f) {
                    return Err(CanonError::NonCanonicalFloat);
                }
                Ok(Value::Float(f))
            }
            31 => Err(CanonError::Indefinite),
            _ => Err(CanonError::Decode("simple value not supported".into())),
        },
        6 => Err(CanonError::Tag),
        _ => Err(CanonError::Decode("unknown major type".into())),
    }
}

fn is_exact_int(f: f64) -> bool {
    if f.is_infinite() || f.is_nan() {
        return false;
    }
    if f.fract() != 0.0 {
        return false;
    }
    let i = f as i128;
    i as f64 == f
}

fn can_fit_f16(f: f64) -> bool {
    if f.is_nan() {
        return true;
    }
    let h = f16::from_f64(f);
    h.to_f64() == f
}

fn can_fit_f32(f: f64) -> bool {
    if f.is_nan() {
        return true;
    }
    (f as f32) as f64 == f
}
