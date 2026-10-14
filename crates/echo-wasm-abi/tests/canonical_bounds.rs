// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The canonical decoder reads untrusted bytes. A length or a nesting depth
//! that the input claims must never decide how much memory or stack the
//! decoder uses before the input has paid for it.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use ciborium::value::Value;
use echo_wasm_abi::{CanonError, MAX_CANONICAL_NESTING_DEPTH, decode_value};
use proptest::prelude::*;

/// `depth` nested one-element arrays around the integer 0.
fn nested_arrays(depth: usize) -> Vec<u8> {
    let mut bytes = vec![0x81; depth];
    bytes.push(0x00);
    bytes
}

#[test]
fn array_header_claiming_more_elements_than_bytes_is_refused_before_allocation() {
    // Array of 2^64 - 1 elements, then nothing.
    let bytes = [0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(decode_value(&bytes), Err(CanonError::Incomplete));

    // Array of 2^32 elements followed by three one-byte elements.
    let bytes = [0x9b, 0, 0, 0, 1, 0, 0, 0, 0, 0x01, 0x02, 0x03];
    assert_eq!(decode_value(&bytes), Err(CanonError::Incomplete));

    // The smallest lie: four elements claimed, three present.
    assert_eq!(
        decode_value(&[0x84, 0x01, 0x02, 0x03]),
        Err(CanonError::Incomplete)
    );
}

#[test]
fn map_header_claiming_more_entries_than_bytes_is_refused_before_allocation() {
    let bytes = [0xbb, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(decode_value(&bytes), Err(CanonError::Incomplete));

    // Two entries need at least four bytes; three are present.
    assert_eq!(
        decode_value(&[0xa2, 0x01, 0x02, 0x03]),
        Err(CanonError::Incomplete)
    );
}

#[test]
fn byte_and_text_lengths_are_compared_before_they_are_narrowed() {
    // A length of 2^32 + 3 with three payload bytes. A 32-bit target that
    // narrowed the length first would read three bytes and accept.
    for major in [0x5b_u8, 0x7b] {
        let bytes = [major, 0, 0, 0, 1, 0, 0, 0, 3, b'a', b'b', b'c'];
        assert_eq!(decode_value(&bytes), Err(CanonError::Incomplete));
    }
}

#[test]
fn nesting_is_accepted_up_to_the_limit_and_refused_one_past_it() {
    let mut value = decode_value(&nested_arrays(MAX_CANONICAL_NESTING_DEPTH))
        .expect("nesting at the limit decodes");
    let mut depth = 0;
    while let Value::Array(mut items) = value {
        depth += 1;
        value = items.pop().expect("one element");
    }
    assert_eq!(depth, MAX_CANONICAL_NESTING_DEPTH);

    let error = decode_value(&nested_arrays(MAX_CANONICAL_NESTING_DEPTH + 1))
        .expect_err("one level past the limit is refused");
    assert!(matches!(error, CanonError::Decode(ref detail) if detail.contains("nesting")));
}

#[test]
fn deep_nesting_is_refused_without_exhausting_the_stack() {
    // Arrays and maps, far past any stack a recursive decoder could survive.
    let error = decode_value(&nested_arrays(2_000_000)).expect_err("refused");
    assert!(matches!(error, CanonError::Decode(ref detail) if detail.contains("nesting")));

    // {0: {0: {0: ...}}}
    let mut bytes = Vec::new();
    for _ in 0..2_000_000 {
        bytes.extend_from_slice(&[0xa1, 0x00]);
    }
    bytes.push(0x00);
    let error = decode_value(&bytes).expect_err("refused");
    assert!(matches!(error, CanonError::Decode(ref detail) if detail.contains("nesting")));
}

proptest! {
    #[test]
    fn arbitrary_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        let _ = decode_value(&bytes);
    }

    #[test]
    fn arbitrary_container_headers_never_panic(
        major in prop::sample::select(vec![0x80_u8, 0xa0, 0x40, 0x60]),
        info in 0_u8..28,
        tail in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let mut bytes = vec![major | info];
        bytes.extend_from_slice(&tail);
        let _ = decode_value(&bytes);
    }
}
