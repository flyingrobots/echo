// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Public contract witnesses for the independently publishable canonical leaf.

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, digest_canonical_value_v1,
    encode_canonical_cbor_v1, CanonicalValueErrorKind, CanonicalValueV1,
    MAX_CANONICAL_DECODE_NODES_V1, MAX_CANONICAL_NESTING_DEPTH_V1,
};

fn text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn map(
    entries: impl IntoIterator<Item = (CanonicalValueV1, CanonicalValueV1)>,
) -> CanonicalValueV1 {
    CanonicalValueV1::Map(entries.into_iter().collect())
}

fn string_map(
    entries: impl IntoIterator<Item = (&'static str, CanonicalValueV1)>,
) -> CanonicalValueV1 {
    map(entries.into_iter().map(|(key, value)| (text(key), value)))
}

fn nested_array(depth: usize) -> CanonicalValueV1 {
    (0..depth).fold(CanonicalValueV1::Null, |value, _| {
        CanonicalValueV1::Array(vec![value])
    })
}

fn nested_array_bytes(depth: usize) -> Vec<u8> {
    let mut bytes = vec![0x81; depth];
    bytes.push(0xf6);
    bytes
}

fn nested_empty_array(container_count: usize) -> CanonicalValueV1 {
    assert!(container_count > 0);
    (1..container_count).fold(CanonicalValueV1::Array(Vec::new()), |value, _| {
        CanonicalValueV1::Array(vec![value])
    })
}

fn nested_empty_array_bytes(container_count: usize) -> Vec<u8> {
    assert!(container_count > 0);
    let mut bytes = vec![0x81; container_count - 1];
    bytes.push(0x80);
    bytes
}

fn nested_empty_map(container_count: usize) -> CanonicalValueV1 {
    assert!(container_count > 0);
    (1..container_count).fold(CanonicalValueV1::Map(Vec::new()), |value, _| {
        CanonicalValueV1::Map(vec![(text("k"), value)])
    })
}

fn nested_empty_map_bytes(container_count: usize) -> Vec<u8> {
    assert!(container_count > 0);
    let mut bytes = Vec::with_capacity((container_count - 1) * 3 + 1);
    for _ in 1..container_count {
        bytes.extend([0xa1, 0x61, b'k']);
    }
    bytes.push(0xa0);
    bytes
}

#[test]
fn typed_digest_bytes_and_review_rendering_are_the_same_proposition() {
    let ordinary = string_map([
        ("apiVersion", text("fixture/v1")),
        ("payload", CanonicalValueV1::Bytes(vec![0, 1, 2, 255])),
    ]);
    let maximum_depth = nested_empty_array(MAX_CANONICAL_NESTING_DEPTH_V1);

    for value in [&ordinary, &maximum_depth] {
        let bytes = digest_canonical_value_bytes_v1("fixture.digest/v1", value)
            .expect("typed digest bytes are computed");
        let review = digest_canonical_value_v1("fixture.digest/v1", value)
            .expect("digest review rendering is computed");
        assert_eq!(review, format!("sha256:{}", hex::encode(bytes)));
    }

    for value in [&ordinary, &maximum_depth] {
        assert_eq!(
            digest_canonical_value_bytes_v1("", value)
                .expect_err("empty byte-digest domain refuses")
                .kind(),
            CanonicalValueErrorKind::UnsupportedValue
        );
        assert_eq!(
            digest_canonical_value_v1("", value)
                .expect_err("empty review-digest domain refuses")
                .kind(),
            CanonicalValueErrorKind::UnsupportedValue
        );
    }

    let too_deep = nested_empty_array(MAX_CANONICAL_NESTING_DEPTH_V1 + 1);
    assert_eq!(
        digest_canonical_value_bytes_v1("fixture.digest/v1", &too_deep)
            .expect_err("unencodable byte digest refuses")
            .kind(),
        CanonicalValueErrorKind::NestingLimitExceeded
    );
    assert_eq!(
        digest_canonical_value_v1("fixture.digest/v1", &too_deep)
            .expect_err("unencodable review digest refuses")
            .kind(),
        CanonicalValueErrorKind::NestingLimitExceeded
    );
}

#[test]
fn canonical_maps_are_order_independent_and_fail_closed() {
    let forward = string_map([("z", text("last")), ("a", text("first"))]);
    let reversed = string_map([("a", text("first")), ("z", text("last"))]);
    assert_eq!(
        encode_canonical_cbor_v1(&forward).expect("forward map encodes"),
        encode_canonical_cbor_v1(&reversed).expect("reversed map encodes")
    );
    let heterogeneous = map([
        (text(""), CanonicalValueV1::Null),
        (CanonicalValueV1::Integer(24), CanonicalValueV1::Null),
    ]);
    assert_eq!(
        encode_canonical_cbor_v1(&heterogeneous).expect("heterogeneous map encodes"),
        [0xa2, 0x18, 0x18, 0xf6, 0x60, 0xf6]
    );

    let duplicate = map([(text("same"), text("one")), (text("same"), text("two"))]);
    assert_eq!(
        encode_canonical_cbor_v1(&duplicate)
            .expect_err("duplicate canonical keys reject")
            .kind(),
        CanonicalValueErrorKind::DuplicateMapKey
    );
    assert_eq!(
        decode_canonical_cbor_v1(&[0x18, 0x00])
            .expect_err("non-minimal integer rejects")
            .kind(),
        CanonicalValueErrorKind::NonCanonical
    );
    assert_eq!(
        decode_canonical_cbor_v1(&[0xf6, 0xf6])
            .expect_err("trailing canonical value rejects")
            .kind(),
        CanonicalValueErrorKind::TrailingData
    );
    assert_eq!(
        decode_canonical_cbor_v1(&[0xa2, 0x61, b'a', 0x01, 0x61, b'a', 0x02])
            .expect_err("duplicate decoded map keys reject")
            .kind(),
        CanonicalValueErrorKind::DuplicateMapKey
    );
    assert_eq!(
        decode_canonical_cbor_v1(&[0xa2, 0x61, b'b', 0x00, 0x61, b'a', 0x00])
            .expect_err("unsorted decoded map keys reject")
            .kind(),
        CanonicalValueErrorKind::NonCanonical
    );
}

#[test]
fn canonical_integer_ranges_and_widths_are_exact() {
    let cases: &[(i128, &[u8])] = &[
        (23, &[0x17]),
        (24, &[0x18, 0x18]),
        (255, &[0x18, 0xff]),
        (256, &[0x19, 0x01, 0x00]),
        (65_535, &[0x19, 0xff, 0xff]),
        (65_536, &[0x1a, 0x00, 0x01, 0x00, 0x00]),
        (4_294_967_295, &[0x1a, 0xff, 0xff, 0xff, 0xff]),
        (
            4_294_967_296,
            &[0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
        ),
        (
            i128::from(u64::MAX),
            &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (
            -1 - i128::from(u64::MAX),
            &[0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
    ];
    for (integer, expected) in cases {
        let value = CanonicalValueV1::Integer(*integer);
        let bytes = encode_canonical_cbor_v1(&value).expect("boundary integer encodes");
        assert_eq!(bytes, *expected);
        assert_eq!(
            decode_canonical_cbor_v1(expected).expect("boundary integer decodes"),
            value
        );
    }

    for integer in [i128::from(u64::MAX) + 1, -2 - i128::from(u64::MAX)] {
        assert_eq!(
            encode_canonical_cbor_v1(&CanonicalValueV1::Integer(integer))
                .expect_err("out-of-range integer rejects")
                .kind(),
            CanonicalValueErrorKind::InvalidInteger
        );
    }
}

#[test]
fn canonical_nesting_bound_includes_empty_containers_but_not_digest_framing() {
    let at_limit = nested_array(MAX_CANONICAL_NESTING_DEPTH_V1);
    let at_limit_bytes = nested_array_bytes(MAX_CANONICAL_NESTING_DEPTH_V1);
    assert_eq!(
        encode_canonical_cbor_v1(&at_limit).expect("maximum-depth value encodes"),
        at_limit_bytes
    );
    assert_eq!(
        decode_canonical_cbor_v1(&at_limit_bytes).expect("maximum-depth bytes decode"),
        at_limit
    );
    digest_canonical_value_v1("test.maximum-depth/v1", &at_limit)
        .expect("digest frame preserves the full artifact depth budget");

    let over_limit = MAX_CANONICAL_NESTING_DEPTH_V1 + 1;
    assert_eq!(
        encode_canonical_cbor_v1(&nested_array(over_limit))
            .expect_err("over-depth value rejects")
            .kind(),
        CanonicalValueErrorKind::NestingLimitExceeded
    );
    assert_eq!(
        decode_canonical_cbor_v1(&nested_array_bytes(over_limit))
            .expect_err("over-depth bytes reject")
            .kind(),
        CanonicalValueErrorKind::NestingLimitExceeded
    );

    let empty_cases = [
        (
            nested_empty_array(MAX_CANONICAL_NESTING_DEPTH_V1),
            nested_empty_array(over_limit),
            nested_empty_array_bytes(MAX_CANONICAL_NESTING_DEPTH_V1),
            nested_empty_array_bytes(over_limit),
        ),
        (
            nested_empty_map(MAX_CANONICAL_NESTING_DEPTH_V1),
            nested_empty_map(over_limit),
            nested_empty_map_bytes(MAX_CANONICAL_NESTING_DEPTH_V1),
            nested_empty_map_bytes(over_limit),
        ),
    ];
    for (at_limit, over_limit_value, at_limit_bytes, over_limit_bytes) in empty_cases {
        assert_eq!(
            encode_canonical_cbor_v1(&at_limit).expect("128 empty containers encode"),
            at_limit_bytes
        );
        assert_eq!(
            decode_canonical_cbor_v1(&at_limit_bytes).expect("128 empty containers decode"),
            at_limit
        );
        assert_eq!(
            encode_canonical_cbor_v1(&over_limit_value)
                .expect_err("the 129th empty container rejects on encode")
                .kind(),
            CanonicalValueErrorKind::NestingLimitExceeded
        );
        assert_eq!(
            decode_canonical_cbor_v1(&over_limit_bytes)
                .expect_err("the 129th empty container rejects on decode")
                .kind(),
            CanonicalValueErrorKind::NestingLimitExceeded
        );
    }
}

#[test]
fn unsupported_cbor_and_digest_domains_fail_closed() {
    for bytes in [
        &[0xc0, 0xf6][..],
        &[0xf9, 0x00, 0x00],
        &[0xf7],
        &[0x9f, 0xff],
        &[0x1c],
    ] {
        assert_eq!(
            decode_canonical_cbor_v1(bytes)
                .expect_err("unsupported CBOR form rejects")
                .kind(),
            CanonicalValueErrorKind::UnsupportedCbor
        );
    }
    assert_eq!(
        decode_canonical_cbor_v1(&[0x61, 0xff])
            .expect_err("invalid UTF-8 rejects")
            .kind(),
        CanonicalValueErrorKind::InvalidUtf8
    );
    assert_eq!(
        decode_canonical_cbor_v1(&[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
            .expect_err("oversized declared collection rejects before allocation")
            .kind(),
        CanonicalValueErrorKind::UnexpectedEof
    );
    let value = text("domain-separated");
    assert_eq!(
        digest_canonical_value_v1("", &value)
            .expect_err("empty domain rejects")
            .kind(),
        CanonicalValueErrorKind::UnsupportedValue
    );
    assert_ne!(
        digest_canonical_value_v1("test.first/v1", &value).expect("first digest computes"),
        digest_canonical_value_v1("test.second/v1", &value).expect("second digest computes")
    );
}

#[test]
fn decoded_node_budget_rejects_compact_container_amplification_before_reserve() {
    let node_limit = u32::try_from(MAX_CANONICAL_DECODE_NODES_V1)
        .expect("canonical decode-node limit fits the CBOR u32 length form");

    let mut array = vec![0x9a];
    array.extend_from_slice(&node_limit.to_be_bytes());
    array.resize(array.len() + MAX_CANONICAL_DECODE_NODES_V1, 0xf6);
    let array_error = decode_canonical_cbor_v1(&array)
        .expect_err("an array that exhausts the child-node budget rejects before reserve");
    assert_eq!(
        array_error.kind(),
        CanonicalValueErrorKind::UnsupportedValue
    );
    assert_eq!(
        array_error.detail(),
        "canonical CBOR decoded node budget exceeded"
    );

    let map_entries = node_limit / 2;
    let mut map = vec![0xb9];
    map.extend_from_slice(
        &u16::try_from(map_entries)
            .expect("half the canonical decode-node limit fits the CBOR u16 length form")
            .to_be_bytes(),
    );
    map.resize(map.len() + MAX_CANONICAL_DECODE_NODES_V1, 0xf6);
    let map_error = decode_canonical_cbor_v1(&map)
        .expect_err("a map that exhausts the key/value-node budget rejects before reserve");
    assert_eq!(map_error.kind(), CanonicalValueErrorKind::UnsupportedValue);
    assert_eq!(
        map_error.detail(),
        "canonical CBOR decoded node budget exceeded"
    );
}
