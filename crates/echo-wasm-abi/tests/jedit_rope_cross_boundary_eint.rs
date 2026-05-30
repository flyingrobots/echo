// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Cross-boundary fixture proofs for the EINT envelope.
//!
//! These tests assert that `echo_wasm_abi::pack_intent_v1` produces byte
//! sequences that are bytewise identical to the literal hex vectors asserted
//! by `jedit/spec/eint.spec.mjs`. The hex literals here MUST stay in lockstep
//! with that TS spec — they are the cross-boundary contract for the EINT
//! envelope itself (the wrapper that carries LE-binary vars across the WASM
//! boundary).

use echo_wasm_abi::pack_intent_v1;

#[test]
fn pack_intent_v1_op1_three_byte_vars_matches_ts_spec() {
    let bytes = pack_intent_v1(1, &[0x01, 0x02, 0x03]).unwrap();
    let expected: Vec<u8> = vec![
        0x45, 0x49, 0x4e, 0x54, // "EINT"
        0x01, 0x00, 0x00, 0x00, // op_id = 1, u32 LE
        0x03, 0x00, 0x00, 0x00, // vars_len = 3
        0x01, 0x02, 0x03, // vars
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn pack_intent_v1_deadbeef_empty_vars_matches_ts_spec() {
    let bytes = pack_intent_v1(0xdead_beef, &[]).unwrap();
    let expected: Vec<u8> = vec![
        0x45, 0x49, 0x4e, 0x54, // "EINT"
        0xef, 0xbe, 0xad, 0xde, // op_id = 0xdeadbeef LE
        0x00, 0x00, 0x00, 0x00, // vars_len = 0
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn pack_intent_v1_op_one_empty_vars_matches_ts_spec() {
    let bytes = pack_intent_v1(1, &[]).unwrap();
    let expected: Vec<u8> = vec![
        0x45, 0x49, 0x4e, 0x54, // "EINT"
        0x01, 0x00, 0x00, 0x00, // op_id = 1
        0x00, 0x00, 0x00, 0x00, // vars_len = 0
    ];
    assert_eq!(bytes, expected);
}
