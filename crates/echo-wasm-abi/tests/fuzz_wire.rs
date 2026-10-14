// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Fuzz tests for wire protocol parsing robustness.

use ciborium::value::Value;
use echo_wasm_abi::{decode_value, encode_value, unpack_intent_v1};
use proptest::prelude::*;

/// Canonical values of bounded size and depth: integers, text, bytes,
/// booleans, null, arrays, and maps with distinct integer keys.
fn canonical_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        any::<i64>().prop_map(|n| Value::Integer(n.into())),
        any::<bool>().prop_map(Value::Bool),
        Just(Value::Null),
        "[a-z]{0,8}".prop_map(Value::Text),
        prop::collection::vec(any::<u8>(), 0..8).prop_map(Value::Bytes),
    ];
    leaf.prop_recursive(6, 64, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
            prop::collection::btree_map(any::<u16>(), inner, 0..6).prop_map(|entries| {
                Value::Map(
                    entries
                        .into_iter()
                        .map(|(key, value)| (Value::Integer(key.into()), value))
                        .collect(),
                )
            }),
        ]
    })
}

proptest! {
    #[test]
    fn fuzz_decode_value_no_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        // Arbitrary bytes reach the canonical decoder from every WASM export.
        let _ = decode_value(&bytes);
    }

    #[test]
    fn fuzz_decode_value_accepts_what_the_encoder_wrote(value in canonical_value()) {
        let bytes = encode_value(&value).unwrap();
        let decoded = decode_value(&bytes).unwrap();
        prop_assert_eq!(encode_value(&decoded).unwrap(), bytes);
    }

    #[test]
    fn fuzz_decode_value_survives_damage_to_valid_input(
        value in canonical_value(),
        position in any::<prop::sample::Index>(),
        replacement in any::<u8>(),
        cut in any::<prop::sample::Index>(),
    ) {
        // Damaged valid input reaches far deeper into the decoder than
        // arbitrary bytes do. One byte is replaced, then the input is cut.
        let mut bytes = encode_value(&value).unwrap();
        let at = position.index(bytes.len());
        bytes[at] = replacement;
        let _ = decode_value(&bytes);
        bytes.truncate(cut.index(bytes.len() + 1));
        let _ = decode_value(&bytes);
    }

    #[test]
    fn fuzz_unpack_intent_v1_no_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        // The goal is simply to ensure this does not panic.
        let _ = unpack_intent_v1(&bytes);
    }

    #[test]
    fn fuzz_unpack_valid_structure_garbage_payload(
        op_id in any::<u32>(),
        len in 0u32..1000,
        payload in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let mut data = Vec::new();
        data.extend_from_slice(b"EINT");
        data.extend_from_slice(&op_id.to_le_bytes());
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&payload);

        let res = unpack_intent_v1(&data);

        if payload.len() == len as usize {
            // Should succeed
             prop_assert!(res.is_ok());
             let (out_op, out_vars) = res.unwrap();
             prop_assert_eq!(out_op, op_id);
             prop_assert_eq!(out_vars, &payload[..]);
        } else {
             // Should fail cleanly
             prop_assert!(res.is_err());
        }
    }
}
