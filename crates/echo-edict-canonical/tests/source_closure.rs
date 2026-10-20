// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Source-closure wire checks do not claim parsing or import-graph authentication.
use echo_edict_canonical::source_closure::{
    source_closure_root_v1, SourceClosureEncodingError as E,
};
use echo_edict_canonical::{digest_canonical_value_bytes_v1 as digest, CanonicalValueV1 as V};

fn text(s: &str) -> V {
    V::Text(s.into())
}
fn object(fields: Vec<(&str, V)>) -> V {
    V::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn module(bytes: Vec<u8>) -> V {
    let source = V::Bytes(bytes);
    let hash = digest("edict.source/v1", &source).expect("module digest");
    object(vec![
        ("source", source),
        (
            "digest",
            V::Array(vec![text("sha256"), V::Bytes(hash.to_vec())]),
        ),
    ])
}
fn closure(count: usize) -> V {
    object(vec![
        ("apiVersion", text("edict.source-closure/v1")),
        ("root", text("library.m0@1")),
        (
            "modules",
            V::Map(
                (0..count)
                    .map(|i| {
                        (
                            text(&format!("library.m{i}@1")),
                            module(format!("package library.m{i}@1; // 🦀").into_bytes()),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}
fn field<'a>(v: &'a mut V, key: &str) -> &'a mut V {
    let V::Map(fields) = v else { panic!("map") };
    &mut fields
        .iter_mut()
        .find(|(k, _)| k == &text(key))
        .expect("field")
        .1
}
#[test]
fn bounded_closure_returns_original_root_after_checking_every_unit() {
    for count in [1, 64] {
        assert_eq!(
            source_closure_root_v1(&closure(count), "library.m0@1").expect("bounded envelope"),
            "package library.m0@1; // 🦀"
        );
    }
    let mut value = closure(2);
    let V::Map(units) = field(&mut value, "modules") else {
        panic!("units")
    };
    *field(&mut units[1].1, "source") = V::Bytes(b"substituted dependency".to_vec());
    assert_eq!(
        source_closure_root_v1(&value, "library.m0@1"),
        Err(E::DigestMismatch)
    );
}
#[test]
fn closure_encoding_refuses_wrong_identity_shape_and_utf8() {
    assert_eq!(
        source_closure_root_v1(&closure(1), "other@1"),
        Err(E::RootMismatch)
    );
    for count in [0, 65] {
        assert_eq!(
            source_closure_root_v1(&closure(count), "library.m0@1"),
            Err(E::ModuleLimit)
        );
    }
    let mut missing_root = closure(1);
    *field(&mut missing_root, "root") = text("library.absent@1");
    assert_eq!(
        source_closure_root_v1(&missing_root, "library.absent@1"),
        Err(E::RootMismatch)
    );
    for (algo, len) in [("sha256", 31), ("sha256", 33), ("blake3", 32)] {
        let mut value = closure(1);
        let V::Map(units) = field(&mut value, "modules") else {
            panic!("units")
        };
        *field(&mut units[0].1, "digest") = V::Array(vec![text(algo), V::Bytes(vec![0; len])]);
        assert_eq!(
            source_closure_root_v1(&value, "library.m0@1"),
            Err(E::InvalidDigest)
        );
    }
    let mut value = closure(1);
    let V::Map(units) = field(&mut value, "modules") else {
        panic!("units")
    };
    units[0].1 = module(vec![0xff]);
    assert_eq!(
        source_closure_root_v1(&value, "library.m0@1"),
        Err(E::InvalidUtf8)
    );
    let mut value = closure(1);
    let V::Map(fields) = &mut value else {
        panic!("fields")
    };
    fields.push((text("ignored"), V::Null));
    assert_eq!(
        source_closure_root_v1(&value, "library.m0@1"),
        Err(E::InvalidShape)
    );
    assert_eq!(
        source_closure_root_v1(&V::Bytes(b"legacy".to_vec()), "library.m0@1"),
        Err(E::InvalidShape)
    );
}
#[test]
fn aggregate_source_bytes_accept_the_exact_limit_and_refuse_one_more() {
    for (size, expected) in [(1024 * 1024, Ok(())), (1024 * 1024 + 1, Err(E::ByteLimit))] {
        let mut value = closure(1);
        let V::Map(units) = field(&mut value, "modules") else {
            panic!("units")
        };
        units[0].1 = module(vec![b'a'; size]);
        assert_eq!(
            source_closure_root_v1(&value, "library.m0@1").map(|_| ()),
            expected
        );
    }
}
