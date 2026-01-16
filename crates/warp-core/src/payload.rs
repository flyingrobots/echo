// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical payload encoding for the motion demo.

use crate::attachment::AtomPayload;
use crate::ident::TypeId;
use bytes::Bytes;

const MOTION_PAYLOAD_V0_BYTES: usize = 24;
const MOTION_PAYLOAD_V2_BYTES: usize = 48;
const MOTION_V0_TYPE_ID: TypeId = TypeId([
    0xfe, 0xc2, 0x37, 0x94, 0x35, 0xc3, 0x5e, 0x68, 0xd8, 0xf0, 0xb6, 0xa8, 0x1c, 0x51, 0xa9, 0xb5,
    0x89, 0xf3, 0xdb, 0x3d, 0x1f, 0x56, 0x9a, 0x9b, 0x44, 0xa5, 0xcf, 0xeb, 0x65, 0x61, 0xbf, 0x3f,
]);
const MOTION_V2_TYPE_ID: TypeId = TypeId([
    0xf8, 0xab, 0xe5, 0xd1, 0x03, 0x69, 0xa2, 0xbb, 0x01, 0x1c, 0xb4, 0x8f, 0x3b, 0xa3, 0x62, 0x72,
    0xf9, 0x12, 0x6f, 0xca, 0x66, 0x35, 0x90, 0xb8, 0x8e, 0x8e, 0x1f, 0xd7, 0x8e, 0xc1, 0x0a, 0xa5,
]);

/// Returns the legacy motion payload `TypeId` (`payload/motion/v0`).
///
/// This format stores six little-endian `f32` values (position + velocity).
#[must_use]
pub fn motion_payload_type_id_v0() -> TypeId {
    MOTION_V0_TYPE_ID
}

/// Returns the canonical payload `TypeId` for the motion demo atom payload (`payload/motion/v2`).
///
/// This is used as the attachment-plane `type_id` for motion component bytes.
#[must_use]
pub fn motion_payload_type_id() -> TypeId {
    MOTION_V2_TYPE_ID
}

/// Serialises a 3D position + velocity pair into the canonical motion payload.
///
/// **Breaking change from v0:** This function now produces the v2 encoding (48 bytes Q32.32).
/// Legacy v0 payloads (24 bytes f32) remain readable via [`decode_motion_payload`], but new
/// writes always use v2.
///
/// The canonical format is Q32.32 fixed-point stored as six `i64` values (little-endian).
/// This provides a stable, cross-platform, cross-language wire encoding even when callers
/// originate values as `f32`.
///
/// Layout (little‑endian):
/// - bytes 0..24: position [x, y, z] as 3 × i64 (Q32.32)
/// - bytes 24..48: velocity [vx, vy, vz] as 3 × i64 (Q32.32)
///   Always 48 bytes.
///
/// Non-finite inputs are mapped deterministically:
/// - `NaN` → `0`
/// - `+∞`/`-∞` → saturated extrema
#[inline]
#[must_use]
pub fn encode_motion_payload(position: [f32; 3], velocity: [f32; 3]) -> Bytes {
    let mut buf = Vec::with_capacity(MOTION_PAYLOAD_V2_BYTES);
    for value in position.into_iter().chain(velocity.into_iter()) {
        let raw = crate::math::fixed_q32_32::from_f32(value);
        buf.extend_from_slice(&raw.to_le_bytes());
    }
    Bytes::from(buf)
}

/// Serialises a 3D position + velocity pair into the legacy v0 motion payload encoding.
///
/// This is retained for compatibility testing and migration tooling. New writes inside
/// the deterministic runtime should prefer the canonical v2 encoder ([`encode_motion_payload`]).
///
/// Layout (little-endian): 6 × `f32` = 24 bytes.
#[inline]
#[must_use]
pub fn encode_motion_payload_v0(position: [f32; 3], velocity: [f32; 3]) -> Bytes {
    let mut buf = Vec::with_capacity(MOTION_PAYLOAD_V0_BYTES);
    for value in position.into_iter().chain(velocity.into_iter()) {
        buf.extend_from_slice(&value.to_le_bytes());
    }
    Bytes::from(buf)
}

/// Serialises a Q32.32 raw position + velocity pair into the canonical motion payload.
///
/// Layout is identical to [`encode_motion_payload`], but callers supply pre-scaled
/// Q32.32 raw integers directly.
#[inline]
#[must_use]
pub fn encode_motion_payload_q32_32(position_raw: [i64; 3], velocity_raw: [i64; 3]) -> Bytes {
    let mut buf = Vec::with_capacity(MOTION_PAYLOAD_V2_BYTES);
    for raw in position_raw.into_iter().chain(velocity_raw.into_iter()) {
        buf.extend_from_slice(&raw.to_le_bytes());
    }
    Bytes::from(buf)
}

/// Serialises motion data into a typed atom payload (`AtomPayload`).
///
/// Equivalent to `AtomPayload { type_id: motion_payload_type_id(), bytes: encode_motion_payload(...) }`.
#[must_use]
pub fn encode_motion_atom_payload(position: [f32; 3], velocity: [f32; 3]) -> AtomPayload {
    AtomPayload::new(
        motion_payload_type_id(),
        encode_motion_payload(position, velocity),
    )
}

/// Serialises motion data into a typed legacy v0 atom payload (`AtomPayload`).
///
/// This produces the legacy 24-byte 6×f32 encoding (`payload/motion/v0`). New writes in the
/// deterministic runtime should prefer the canonical v2 encoder ([`encode_motion_atom_payload`]).
#[must_use]
pub fn encode_motion_atom_payload_v0(position: [f32; 3], velocity: [f32; 3]) -> AtomPayload {
    AtomPayload::new(
        motion_payload_type_id_v0(),
        encode_motion_payload_v0(position, velocity),
    )
}

fn decode_motion_payload_v0(bytes: &Bytes) -> Option<([f32; 3], [f32; 3])> {
    if bytes.len() != MOTION_PAYLOAD_V0_BYTES {
        return None;
    }
    let mut floats = [0f32; 6];
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        floats[index] = f32::from_le_bytes(chunk.try_into().ok()?);
    }
    let position = [floats[0], floats[1], floats[2]];
    let velocity = [floats[3], floats[4], floats[5]];
    Some((position, velocity))
}

fn decode_motion_payload_v2(bytes: &Bytes) -> Option<([f32; 3], [f32; 3])> {
    if bytes.len() != MOTION_PAYLOAD_V2_BYTES {
        return None;
    }
    let mut floats = [0f32; 6];
    for (index, chunk) in bytes.chunks_exact(8).enumerate() {
        let raw = i64::from_le_bytes(chunk.try_into().ok()?);
        floats[index] = crate::math::fixed_q32_32::to_f32(raw);
    }
    let position = [floats[0], floats[1], floats[2]];
    let velocity = [floats[3], floats[4], floats[5]];
    Some((position, velocity))
}

fn decode_motion_payload_q32_32_v2(bytes: &Bytes) -> Option<([i64; 3], [i64; 3])> {
    if bytes.len() != MOTION_PAYLOAD_V2_BYTES {
        return None;
    }
    let mut raw = [0_i64; 6];
    for (index, chunk) in bytes.chunks_exact(8).enumerate() {
        raw[index] = i64::from_le_bytes(chunk.try_into().ok()?);
    }
    let position = [raw[0], raw[1], raw[2]];
    let velocity = [raw[3], raw[4], raw[5]];
    Some((position, velocity))
}

fn decode_motion_payload_q32_32_v0(bytes: &Bytes) -> Option<([i64; 3], [i64; 3])> {
    let (pos, vel) = decode_motion_payload_v0(bytes)?;
    let position = [
        crate::math::fixed_q32_32::from_f32(pos[0]),
        crate::math::fixed_q32_32::from_f32(pos[1]),
        crate::math::fixed_q32_32::from_f32(pos[2]),
    ];
    let velocity = [
        crate::math::fixed_q32_32::from_f32(vel[0]),
        crate::math::fixed_q32_32::from_f32(vel[1]),
        crate::math::fixed_q32_32::from_f32(vel[2]),
    ];
    Some((position, velocity))
}

/// Deserialises a canonical motion payload into `(position, velocity)` arrays.
///
/// Supports two encodings:
/// - v0: 6 × `f32` little-endian (24 bytes)
/// - v2: 6 × `i64` Q32.32 little-endian (48 bytes)
///
/// **Note:** This function dispatches by byte length alone. When `type_id` is
/// available, prefer [`decode_motion_atom_payload`] for unambiguous routing.
///
/// Returns `None` if the payload does not match either canonical encoding or if any
/// chunk cannot be converted (invalid input).
#[must_use]
pub fn decode_motion_payload(bytes: &Bytes) -> Option<([f32; 3], [f32; 3])> {
    if bytes.len() == MOTION_PAYLOAD_V2_BYTES {
        return decode_motion_payload_v2(bytes);
    }
    if bytes.len() == MOTION_PAYLOAD_V0_BYTES {
        return decode_motion_payload_v0(bytes);
    }
    None
}

/// Deserialises a typed atom payload into `(position, velocity)` arrays.
///
/// Returns `None` if the payload `type_id` is not a supported motion payload type id
/// or if the underlying bytes do not match the canonical motion encoding.
#[must_use]
pub fn decode_motion_atom_payload(payload: &AtomPayload) -> Option<([f32; 3], [f32; 3])> {
    if payload.type_id == motion_payload_type_id() {
        return decode_motion_payload_v2(&payload.bytes);
    }
    if payload.type_id == motion_payload_type_id_v0() {
        return decode_motion_payload_v0(&payload.bytes);
    }
    None
}

/// Deserialises a typed atom payload into Q32.32 raw `(position, velocity)` arrays.
///
/// This is the canonical form used by deterministic motion logic:
/// - v2 payloads decode directly to raw integers.
/// - v0 payloads decode through f32 and are deterministically quantized to Q32.32.
#[must_use]
pub fn decode_motion_atom_payload_q32_32(payload: &AtomPayload) -> Option<([i64; 3], [i64; 3])> {
    if payload.type_id == motion_payload_type_id() {
        return decode_motion_payload_q32_32_v2(&payload.bytes);
    }
    if payload.type_id == motion_payload_type_id_v0() {
        return decode_motion_payload_q32_32_v0(&payload.bytes);
    }
    None
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn q32_32_decoder_accepts_v0_and_quantizes_deterministically() {
        let pos = [1.0, 0.5, -1.0];
        let vel = [2.0, -0.25, 0.0];

        let payload = encode_motion_atom_payload_v0(pos, vel);
        let (p_raw, v_raw) =
            decode_motion_atom_payload_q32_32(&payload).expect("v0 atom payload should decode");

        for i in 0..3 {
            assert_eq!(p_raw[i], crate::math::fixed_q32_32::from_f32(pos[i]));
            assert_eq!(v_raw[i], crate::math::fixed_q32_32::from_f32(vel[i]));
        }
    }

    #[test]
    fn round_trip_v0_ok() {
        let pos = [1.0, 2.0, 3.0];
        let vel = [0.5, -1.0, 0.25];
        let bytes = encode_motion_payload_v0(pos, vel);
        let (p, v) = decode_motion_payload(&bytes).expect("v0 payload");
        for i in 0..3 {
            assert_eq!(p[i].to_bits(), pos[i].to_bits());
            assert_eq!(v[i].to_bits(), vel[i].to_bits());
        }
    }

    #[test]
    fn round_trip_v2_ok_for_exact_values() {
        let pos = [1.0, 2.0, 3.0];
        let vel = [0.5, -1.0, 0.25];
        let bytes = encode_motion_payload(pos, vel);
        assert_eq!(bytes.len(), MOTION_PAYLOAD_V2_BYTES);
        let (p, v) = decode_motion_payload(&bytes).expect("v2 payload");
        assert_eq!(p, pos);
        assert_eq!(v, vel);
    }

    #[test]
    fn q32_32_round_trip_matches_v2_bytes() {
        let pos = [1.0, 2.0, 3.0];
        let vel = [0.5, -1.0, 0.25];
        let raw_pos = [
            crate::math::fixed_q32_32::from_f32(pos[0]),
            crate::math::fixed_q32_32::from_f32(pos[1]),
            crate::math::fixed_q32_32::from_f32(pos[2]),
        ];
        let raw_vel = [
            crate::math::fixed_q32_32::from_f32(vel[0]),
            crate::math::fixed_q32_32::from_f32(vel[1]),
            crate::math::fixed_q32_32::from_f32(vel[2]),
        ];
        let a = encode_motion_payload(pos, vel);
        let b = encode_motion_payload_q32_32(raw_pos, raw_vel);
        assert_eq!(a, b);
    }

    #[test]
    fn reject_wrong_len() {
        let b = Bytes::from_static(&[0u8; 23]);
        assert!(decode_motion_payload(&b).is_none());
        let b = Bytes::from_static(&[0u8; 25]);
        assert!(decode_motion_payload(&b).is_none());
        let b = Bytes::from_static(&[0u8; 47]);
        assert!(decode_motion_payload(&b).is_none());
        let b = Bytes::from_static(&[0u8; 49]);
        assert!(decode_motion_payload(&b).is_none());
    }
}
