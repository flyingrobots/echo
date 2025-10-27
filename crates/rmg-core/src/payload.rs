//! Canonical payload encoding for the motion demo.
use bytes::Bytes;

const POSITION_VELOCITY_BYTES: usize = 24;

/// Serialises a 3D position + velocity vector pair into the canonical payload.
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

