// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Golden vectors and rejection cases for canonical CBOR encoding/decoding.

use echo_wasm_abi::{decode_cbor, encode_cbor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Sample {
    a: u8,
    b: bool,
}

#[test]
fn golden_sample_map() {
    let sample = Sample { a: 1, b: true };
    let bytes = encode_cbor(&sample).expect("encode");
    // Expected canonical CBOR: { "a": 1, "b": true }
    // a2 61 61 01 61 62 f5
    let expected: &[u8] = &[0xa2, 0x61, 0x61, 0x01, 0x61, 0x62, 0xf5];
    assert_eq!(bytes, expected);
    let back: Sample = decode_cbor(&bytes).expect("decode");
    assert_eq!(sample, back);
}

#[test]
fn reject_non_canonical_int_width() {
    // Map { "x": 1 } but 1 is encoded as 0x18 0x01 (non-minimal)
    let bytes: &[u8] = &[0xa1, 0x61, 0x78, 0x18, 0x01];
    let err = decode_cbor::<serde_json::Value>(bytes).unwrap_err();
    assert!(
        format!("{err:?}").contains("NonCanonicalInt"),
        "expected NonCanonicalInt, got {err:?}"
    );
}

#[test]
fn reject_map_key_order() {
    // Map { "b":1, "a":2 } (keys not sorted)
    let bytes: &[u8] = &[0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02];
    let err = decode_cbor::<serde_json::Value>(bytes).unwrap_err();
    assert!(
        format!("{err:?}").contains("MapKeyOrder"),
        "expected MapKeyOrder, got {err:?}"
    );
}
