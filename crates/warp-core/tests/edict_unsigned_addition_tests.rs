// SPDX-License-Identifier: Apache-2.0
//! Execute an unmodified public compiler package containing proven unsigned sums.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode,
    CanonicalValueV1 as Value,
};
use warp_core::edict_pure::{evaluate, EvaluationError, EvaluationLimits};

fn package() -> Vec<u8> {
    hex::decode(
        include_str!(
            "fixtures/edict-unsigned-addition-control/executable-operation-package.cbor.hex"
        )
        .trim(),
    )
    .unwrap()
}
fn pin() -> [u8; 32] {
    hex::decode("75381ef9f3421a74f2fa21e256705e9737b94eda1f7549b5b23c254799254086")
        .unwrap()
        .try_into()
        .unwrap()
}
fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 32_768,
        max_input_bytes: 4096,
        max_steps: 10_000,
        max_allocated_bytes: 16_777_216,
        max_output_bytes: 4096,
    }
}
fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(name, value)| (Value::Text(name.into()), value))
            .collect(),
    )
}
fn input(left: u64, right: u64, counter: u64) -> Value {
    record([
        ("bufferId", Value::Bytes(vec![1; 32])),
        ("basisHeadId", Value::Bytes(vec![2; 32])),
        ("startByte", Value::Integer(left.into())),
        ("endByte", Value::Integer(right.into())),
        ("counter32", Value::Integer(counter.into())),
        ("replacement", Value::Bytes(vec![0x80, 0xff])),
    ])
}

#[test]
fn public_compiler_unsigned_sums_execute_at_both_width_boundaries() {
    for (left, right, counter, want64, want32, empty) in [
        (0, 0, 0, 0, 1, 1),
        (0, 1, 7, 1, 8, 0),
        (
            u64::MAX - 1,
            1,
            u64::from(u32::MAX) - 1,
            u64::MAX,
            u64::from(u32::MAX),
            0,
        ),
        (u64::MAX - 1, 0, 0, u64::MAX - 1, 1, 0),
    ] {
        let result = evaluate(
            &package(),
            pin(),
            &encode(&input(left, right, counter)).unwrap(),
            limits(),
        )
        .unwrap();
        let want = record([
            ("bufferId", Value::Bytes(vec![1; 32])),
            ("basisHeadId", Value::Bytes(vec![2; 32])),
            ("startByte", Value::Integer(left.into())),
            ("endByte", Value::Integer(right.into())),
            ("replacement", Value::Bytes(vec![0x80, 0xff])),
            ("rangeIsEmpty", Value::Integer(empty)),
            ("createdLeafCeiling", Value::Integer(4096)),
            ("sum64", Value::Integer(want64.into())),
            ("next32", Value::Integer(want32.into())),
        ]);
        assert_eq!(
            decode(&result.output).unwrap(),
            decode(&encode(&want).unwrap()).unwrap()
        );
    }
}

#[test]
fn public_addition_guards_refuse_inputs_outside_the_proven_domain() {
    for (left, right, counter) in [(u64::MAX, 1, 0), (0, 2, 0), (0, 0, u64::from(u32::MAX))] {
        assert!(matches!(
            evaluate(
                &package(),
                pin(),
                &encode(&input(left, right, counter)).unwrap(),
                limits()
            ),
            Err(EvaluationError::InputConstraintFailed(_))
        ));
    }
}
