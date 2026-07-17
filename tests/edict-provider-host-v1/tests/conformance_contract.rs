// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Shared declarative provider-conformance contract witnesses.

mod support;

use edict_syntax::{decode_canonical_cbor, encode_canonical_cbor, CanonicalValue};
use support::conformance::{
    decode_declared_cases, CorpusContractErrorKind, ExecutableContract, ExecutorOwner,
    CONFORMANCE_CORPUS_BYTES,
};

fn text(value: &str) -> CanonicalValue {
    CanonicalValue::Text(value.to_owned())
}

fn map_field_mut<'a>(value: &'a mut CanonicalValue, field: &str) -> &'a mut CanonicalValue {
    let CanonicalValue::Map(entries) = value else {
        panic!("canonical value is not a map");
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .unwrap_or_else(|| panic!("canonical map field `{field}` is absent"))
}

fn checked_corpus_value() -> CanonicalValue {
    decode_canonical_cbor(CONFORMANCE_CORPUS_BYTES)
        .expect("checked conformance corpus is canonical CBOR")
}

fn parse_mutation(value: &CanonicalValue) -> CorpusContractErrorKind {
    let bytes = encode_canonical_cbor(value).expect("mutated corpus encodes canonically");
    decode_declared_cases(&bytes)
        .expect_err("mutated declaration must fail the executable-law boundary")
        .kind()
}

#[test]
fn checked_declaration_has_one_exact_executable_owner() {
    let cases = decode_declared_cases(CONFORMANCE_CORPUS_BYTES)
        .expect("checked declarations satisfy the executable-law inventory");
    let [case] = cases.as_slice() else {
        panic!("the current checked corpus has one declaration");
    };

    assert_eq!(case.id(), "package-parity");
    assert_eq!(case.crossing(), "pipeline");
    assert_eq!(case.stimulus(), "baseline");
    assert_eq!(case.required_disposition(), "accepted");
    assert_eq!(case.contract(), ExecutableContract::CompletedPackageParity);
    assert_eq!(case.owner(), ExecutorOwner::Package);
    assert_ne!(case.owner(), ExecutorOwner::Host);
}

#[test]
fn declaration_parser_rejects_unknown_or_mismatched_executable_laws() {
    let mut unknown = checked_corpus_value();
    *map_field_mut(
        map_field_mut(
            map_field_mut(map_field_mut(&mut unknown, "cases"), "package-parity"),
            "requiredOutcome",
        ),
        "contract",
    ) = text("unreviewed-contract");
    assert_eq!(
        parse_mutation(&unknown),
        CorpusContractErrorKind::UnknownContract
    );

    let mut mismatch = checked_corpus_value();
    let case = map_field_mut(map_field_mut(&mut mismatch, "cases"), "package-parity");
    *map_field_mut(case, "stimulus") = text("component-bytes-changed");
    assert_eq!(
        parse_mutation(&mismatch),
        CorpusContractErrorKind::DeclarationMismatch
    );
}

#[test]
fn declaration_parser_rejects_empty_duplicate_or_result_shaped_cases() {
    let mut empty = checked_corpus_value();
    *map_field_mut(&mut empty, "cases") = CanonicalValue::Map(Vec::new());
    assert_eq!(parse_mutation(&empty), CorpusContractErrorKind::EmptyCases);

    let mut duplicate = checked_corpus_value();
    let cases = map_field_mut(&mut duplicate, "cases");
    let CanonicalValue::Map(entries) = cases else {
        panic!("cases are a map");
    };
    let contract = entries[0].1.clone();
    entries.push((text("package-parity-copy"), contract));
    assert_eq!(
        parse_mutation(&duplicate),
        CorpusContractErrorKind::DuplicateContract
    );

    let mut result_shaped = checked_corpus_value();
    let case = map_field_mut(map_field_mut(&mut result_shaped, "cases"), "package-parity");
    let CanonicalValue::Map(fields) = case else {
        panic!("case declaration is a map");
    };
    fields.push((text("passed"), CanonicalValue::Bool(true)));
    assert_eq!(
        parse_mutation(&result_shaped),
        CorpusContractErrorKind::CaseClosureInvalid
    );
}
