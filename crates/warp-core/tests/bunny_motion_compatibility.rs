// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Literal wire witnesses for Bunny-backed motion payload conversion.

use bytes::Bytes;
use warp_core::math::fixed_q32_32::FixedQ32_32;
use warp_core::{
    decode_motion_atom_payload_q32_32, encode_motion_payload, encode_motion_payload_q32_32,
    motion_payload_type_id, motion_payload_type_id_v0, AtomPayload,
};

const EXPECTED_V2: [u8; 48] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // zero
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // +2 raw
    0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // -2 raw
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f, // saturated maximum
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, // saturated minimum
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // 1.0
];
const EXPECTED_RAW: ([i64; 3], [i64; 3]) = ([0, 2, -2], [i64::MAX, i64::MIN, 4_294_967_296]);

#[test]
fn bunny_backed_conversion_preserves_literal_motion_v2_bytes() {
    let position = [
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x2fc0_0000),
        f32::from_bits(0xafc0_0000),
    ];
    let velocity = [
        f32::from_bits(0x7f80_0000),
        f32::from_bits(0xff80_0000),
        f32::from_bits(0x3f80_0000),
    ];
    assert_eq!(
        encode_motion_payload(position, velocity).as_ref(),
        EXPECTED_V2
    );
    assert_eq!(
        encode_motion_payload_q32_32(EXPECTED_RAW.0, EXPECTED_RAW.1).as_ref(),
        EXPECTED_V2
    );
    let payload = AtomPayload::new(motion_payload_type_id(), Bytes::from_static(&EXPECTED_V2));
    assert_eq!(
        decode_motion_atom_payload_q32_32(&payload),
        Some(EXPECTED_RAW)
    );
    assert_eq!(FixedQ32_32::from_raw(EXPECTED_RAW.0[1]).raw(), 2);
}

#[test]
fn legacy_motion_v0_keeps_its_canonical_conversion() {
    // Literal IEEE f32 little-endian bytes: NaN, +1.5 raw, -1.5 raw, +inf, -inf, 1.
    let bytes = Bytes::from_static(&[
        0x00, 0x00, 0xc0, 0x7f, 0x00, 0x00, 0xc0, 0x2f, 0x00, 0x00, 0xc0, 0xaf, 0x00, 0x00, 0x80,
        0x7f, 0x00, 0x00, 0x80, 0xff, 0x00, 0x00, 0x80, 0x3f,
    ]);
    let payload = AtomPayload::new(motion_payload_type_id_v0(), bytes);
    assert_eq!(
        decode_motion_atom_payload_q32_32(&payload),
        Some(EXPECTED_RAW)
    );
    let wrong_type = AtomPayload::new(
        motion_payload_type_id_v0(),
        Bytes::from_static(&EXPECTED_V2),
    );
    assert_eq!(decode_motion_atom_payload_q32_32(&wrong_type), None);
}
