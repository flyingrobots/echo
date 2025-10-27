//! Canonical payload encoding for the motion demo.
use bytes::Bytes;

const POSITION_VELOCITY_BYTES: usize = 24;

/// Serialises a 3D position + velocity pair into the canonical payload.
///
/// Note: Values are encoded verbatim as `f32` little‑endian bytes; callers are
/// responsible for ensuring finiteness if deterministic behaviour is required
/// (NaN bit patterns compare unequal across some platforms).
///
/// Layout (little‑endian):
/// - bytes 0..12: position [x, y, z] as 3 × f32
/// - bytes 12..24: velocity [vx, vy, vz] as 3 × f32
///   Always 24 bytes.
#[inline]
pub fn encode_motion_payload(position: [f32; 3], velocity: [f32; 3]) -> Bytes {
    let mut buf = Vec::with_capacity(POSITION_VELOCITY_BYTES);
    for value in position.into_iter().chain(velocity.into_iter()) {
        buf.extend_from_slice(&value.to_le_bytes());
    }
    Bytes::from(buf)
}

/// Deserialises a canonical motion payload into (position, velocity) slices.
pub fn decode_motion_payload(bytes: &Bytes) -> Option<([f32; 3], [f32; 3])> {
    if bytes.len() != POSITION_VELOCITY_BYTES {
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
    fn round_trip_ok() {
        let pos = [1.0, 2.0, 3.0];
        let vel = [0.5, -1.0, 0.25];
        let bytes = encode_motion_payload(pos, vel);
        let (p, v) = decode_motion_payload(&bytes).expect("24-byte payload");
        for i in 0..3 {
            assert_eq!(p[i].to_bits(), pos[i].to_bits());
            assert_eq!(v[i].to_bits(), vel[i].to_bits());
        }
    }

    #[test]
    fn reject_wrong_len() {
        let b = Bytes::from_static(&[0u8; 23]);
        assert!(decode_motion_payload(&b).is_none());
        let b = Bytes::from_static(&[0u8; 25]);
        assert!(decode_motion_payload(&b).is_none());
    }
}
