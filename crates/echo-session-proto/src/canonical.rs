// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal deterministic CBOR encoder/decoder for JS-ABI v1.0.
//!
//! Enforces:
//! - Definite lengths only (no break/indef)
//! - No tags
//! - Canonical integer widths (shortest)
//! - Floats encoded with smallest width that round-trips (and integers encoded as ints)
//! - Map keys sorted by their CBOR byte encoding; no duplicates
//! - Reject non-canonical float widths and int-as-float

use half::f16;
use serde_cbor::Value;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CanonError {
    #[error("incomplete input")]
    Incomplete,
    #[error("trailing bytes after value")]
    Trailing,
    #[error("tags not allowed")]
    Tag,
    #[error("indefinite length not allowed")]
    Indefinite,
    #[error("non-canonical integer width")]
    NonCanonicalInt,
    #[error("non-canonical float width")]
    NonCanonicalFloat,
    #[error("float encodes integral value; must be integer")]
    FloatShouldBeInt,
    #[error("map keys not strictly increasing")]
    MapKeyOrder,
    #[error("duplicate map key")]
    MapKeyDuplicate,
    #[error("decode error: {0}")]
    Decode(String),
}

type Result<T> = std::result::Result<T, CanonError>;

// Public API

pub fn encode_value(val: &Value) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    enc_value(val, &mut out)?;
    Ok(out)
}

pub fn decode_value(bytes: &[u8]) -> Result<Value> {
    let mut idx = 0usize;
    let v = dec_value(bytes, &mut idx, true)?;
    if idx != bytes.len() {
        return Err(CanonError::Trailing);
    }
    Ok(v)
}

// --- Encoder --------------------------------------------------------------

fn enc_value(v: &Value, out: &mut Vec<u8>) -> Result<()> {
    match v {
        Value::Bool(b) => {
            out.push(if *b { 0xf5 } else { 0xf4 });
        }
        Value::Null => out.push(0xf6),
        Value::Integer(n) => enc_int(*n, out),
        Value::Float(f) => enc_float(*f, out),
        Value::Text(s) => enc_text(s, out)?,
        Value::Bytes(b) => enc_bytes(b, out)?,
        Value::Array(items) => {
            enc_len(4, items.len() as u64, out);
            for it in items {
                enc_value(it, out)?;
            }
        }
        Value::Map(map) => {
            // collect entries
            let mut entries: Vec<(Value, Value, Vec<u8>)> = map
                .iter()
                .map(|(k, v)| {
                    let mut kb = Vec::new();
                    enc_value(k, &mut kb).expect("key encode");
                    (k.clone(), v.clone(), kb)
                })
                .collect();

            // canonical sort by encoded key bytes
            entries.sort_by(|a, b| a.2.cmp(&b.2));

            // dup check
            for win in entries.windows(2) {
                if win[0].2 == win[1].2 {
                    return Err(CanonError::MapKeyDuplicate);
                }
            }

            enc_len(5, entries.len() as u64, out);
            for (_k, v, kb) in entries {
                out.extend_from_slice(&kb);
                enc_value(&v, out)?;
            }
        }
        Value::Tag(_, _) => return Err(CanonError::Tag),
        Value::__Hidden => return Err(CanonError::Decode("hidden value".into())),
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
        // CBOR negative: value = -1 - n => major 1 with (-(n+1))
        let m = (-1 - n) as u128;
        write_major(1, m, out);
    }
}

fn enc_float(f: f64, out: &mut Vec<u8>) {
    if f.is_nan() {
        // canonical NaN: use half if possible, else f32, else f64
        let h = f16::NAN;
        write_half(h, out);
        return;
    }
    if f.is_infinite() {
        // prefer half if fits, else f32, else f64
        let h = if f.is_sign_positive() {
            f16::INFINITY
        } else {
            f16::NEG_INFINITY
        };
        write_half(h, out);
        return;
    }
    // If integral and fits i128, encode as integer per spec
    if f.fract() == 0.0 {
        let i = f as i128;
        if i as f64 == f {
            enc_int(i, out);
            return;
        }
    }
    // choose smallest width that round-trips
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

// --- Decoder --------------------------------------------------------------

fn dec_value(bytes: &[u8], idx: &mut usize, strict: bool) -> Result<Value> {
    if *idx >= bytes.len() {
        return Err(CanonError::Incomplete);
    }
    let b0 = bytes[*idx];
    *idx += 1;
    let major = b0 >> 5;
    let ai = b0 & 0x1f;

    // forbid tags
    if major == 6 {
        return Err(CanonError::Tag);
    }

    // forbid indefinite
    if ai == 31 {
        return Err(CanonError::Indefinite);
    }

    let n = match ai {
        0..=23 => ai as u64,
        24 => take_u(bytes, idx, 1),
        25 => take_u(bytes, idx, 2),
        26 => take_u(bytes, idx, 4),
        27 => take_u(bytes, idx, 8),
        _ => return Err(CanonError::Decode("invalid additional info".into())),
    };

    match major {
        0 => {
            // unsigned int
            check_min_int(ai, n, false, strict)?;
            Ok(int_to_value(n as u128, false))
        }
        1 => {
            // negative
            check_min_int(ai, n, true, strict)?;
            Ok(int_to_value(n as u128, true))
        }
        2 => {
            let len = n as usize;
            let end = *idx + len;
            if end > bytes.len() {
                return Err(CanonError::Incomplete);
            }
            let v = Value::Bytes(bytes[*idx..end].to_vec());
            *idx = end;
            Ok(v)
        }
        3 => {
            let len = n as usize;
            let end = *idx + len;
            if end > bytes.len() {
                return Err(CanonError::Incomplete);
            }
            let s = std::str::from_utf8(&bytes[*idx..end])
                .map_err(|e| CanonError::Decode(e.to_string()))?
                .to_string();
            *idx = end;
            Ok(Value::Text(s))
        }
        4 => {
            let len = n as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(dec_value(bytes, idx, strict)?);
            }
            Ok(Value::Array(items))
        }
        5 => {
            let len = n as usize;
            let mut entries = Vec::with_capacity(len);
            let mut prev_bytes: Option<Vec<u8>> = None;
            for _ in 0..len {
                let key_start = *idx;
                let key = dec_value(bytes, idx, strict)?;
                let key_end = *idx;
                let key_bytes = &bytes[key_start..key_end];
                let curr_bytes = key_bytes.to_vec();
                if let Some(pb) = &prev_bytes {
                    match pb.cmp(&curr_bytes) {
                        std::cmp::Ordering::Less => {}
                        std::cmp::Ordering::Equal => return Err(CanonError::MapKeyDuplicate),
                        std::cmp::Ordering::Greater => return Err(CanonError::MapKeyOrder),
                    }
                }
                prev_bytes = Some(curr_bytes);
                let val = dec_value(bytes, idx, strict)?;
                entries.push((key, val));
            }
            let map: BTreeMap<Value, Value> = entries.into_iter().collect();
            Ok(Value::Map(map))
        }
        6 => unreachable!(),
        7 => {
            match ai {
                20 => Ok(Value::Bool(false)),
                21 => Ok(Value::Bool(true)),
                22 | 23 => Ok(Value::Null),
                24 => Err(CanonError::Decode("simple value not supported".into())),
                25 => {
                    let bits = n as u16;
                    let f = f16::from_bits(bits).to_f64();
                    if strict && float_should_be_int(f) {
                        return Err(CanonError::FloatShouldBeInt);
                    }
                    if strict && !float_canonical_width(f, 16) {
                        return Err(CanonError::NonCanonicalFloat);
                    }
                    Ok(Value::Float(f))
                }
                26 => {
                    let bits = take_u(bytes, idx, 4) as u32;
                    let f = f32::from_bits(bits) as f64;
                    if strict && float_should_be_int(f) {
                        return Err(CanonError::FloatShouldBeInt);
                    }
                    if strict && float_canonical_width(f, 32) {
                        // ok
                    } else if strict {
                        return Err(CanonError::NonCanonicalFloat);
                    }
                    Ok(Value::Float(f))
                }
                27 => {
                    let bits = take_u(bytes, idx, 8);
                    let f = f64::from_bits(bits);
                    if strict && float_should_be_int(f) {
                        return Err(CanonError::FloatShouldBeInt);
                    }
                    if strict && float_canonical_width(f, 64) {
                        // ok
                    } else if strict {
                        return Err(CanonError::NonCanonicalFloat);
                    }
                    Ok(Value::Float(f))
                }
                _ => Err(CanonError::Decode("unknown simple/float".into())),
            }
        }
        _ => Err(CanonError::Decode("unknown major".into())),
    }
}

fn take_u(bytes: &[u8], idx: &mut usize, len: usize) -> u64 {
    let mut buf = [0u8; 8];
    let end = *idx + len;
    if end > bytes.len() {
        return 0; // will be caught as incomplete later
    }
    buf[8 - len..].copy_from_slice(&bytes[*idx..end]);
    *idx = end;
    u64::from_be_bytes(buf)
}

fn check_min_int(ai: u8, n: u64, _negative: bool, strict: bool) -> Result<()> {
    if !strict {
        return Ok(());
    }
    let min_ok = match ai {
        0..=23 => true,
        24 => n >= 24,
        25 => n > 0xff,
        26 => n > 0xffff,
        27 => n > 0xffff_ffff,
        _ => false,
    };
    if min_ok {
        Ok(())
    } else {
        Err(CanonError::NonCanonicalInt)
    }
}

fn int_to_value(n: u128, negative: bool) -> Value {
    if negative {
        // value = -1 - n
        let v = -1i128 - (n as i128);
        Value::Integer(v)
    } else {
        Value::Integer(n as i128)
    }
}

fn float_should_be_int(f: f64) -> bool {
    f.is_finite() && f.fract() == 0.0 && fits_i128(f)
}

fn fits_i128(f: f64) -> bool {
    const MAX: f64 = i128::MAX as f64;
    const MIN: f64 = i128::MIN as f64;
    (MIN..=MAX).contains(&f)
}

fn float_canonical_width(f: f64, width: u8) -> bool {
    // width: 16/32/64 is the encoding width encountered
    if f.is_nan() {
        return width == 16; // canonical NaN should be half if allowed
    }
    if f.is_infinite() {
        return width == 16; // fits in half
    }
    let h = f16::from_f64(f);
    if h.to_f64() == f {
        return width == 16;
    }
    let f32v = f as f32;
    if f32v as f64 == f {
        return width == 32;
    }
    true // otherwise needs f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ec03_minimal_int_widths() {
        assert_eq!(encode_value(&Value::Integer(23)).unwrap()[0], 0x17);
        assert_eq!(encode_value(&Value::Integer(24)).unwrap(), vec![0x18, 0x18]);
        assert_eq!(
            encode_value(&Value::Integer(255)).unwrap(),
            vec![0x18, 0xff]
        );
        assert_eq!(
            encode_value(&Value::Integer(256)).unwrap(),
            vec![0x19, 0x01, 0x00]
        );
    }

    #[test]
    fn ec04_ints_not_floats_and_smallest_float_width() {
        let one = encode_value(&Value::Float(1.0)).unwrap();
        assert_eq!(one[0], 0x01); // encoded as integer

        let half = encode_value(&Value::Float(0.5)).unwrap();
        assert_eq!(half, vec![0xf9, 0x38, 0x00]); // half-float
    }

    #[test]
    fn dc02_reject_indefinite() {
        let bytes = vec![0x9f, 0x01, 0x02, 0xff];
        let res = decode_value(&bytes);
        assert!(matches!(res, Err(CanonError::Indefinite)));
    }

    #[test]
    fn dc03_reject_non_canonical_int() {
        let bytes = vec![0x19, 0x00, 0x01];
        let res = decode_value(&bytes);
        assert!(matches!(res, Err(CanonError::NonCanonicalInt)));
    }

    #[test]
    fn dc04_reject_tag() {
        let bytes = vec![0xc0, 0x00];
        let res = decode_value(&bytes);
        assert!(matches!(res, Err(CanonError::Tag)));
    }

    #[test]
    fn dc05_reject_duplicate_keys() {
        let bytes = vec![0xa2, 0x61, 0x61, 0x01, 0x61, 0x61, 0x02];
        let res = decode_value(&bytes);
        assert!(matches!(res, Err(CanonError::MapKeyDuplicate)));
    }

    #[test]
    fn dc06_reject_wrong_order() {
        let bytes = vec![0xa2, 0x61, 0x7a, 0x01, 0x61, 0x61, 0x01];
        let res = decode_value(&bytes);
        assert!(matches!(res, Err(CanonError::MapKeyOrder)));
    }
}
