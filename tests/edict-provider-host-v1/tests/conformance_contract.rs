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

fn case_contract(
    crossing: &str,
    stimulus: &str,
    disposition: &str,
    contract: &str,
) -> CanonicalValue {
    CanonicalValue::Map(vec![
        (text("crossing"), text(crossing)),
        (text("stimulus"), text(stimulus)),
        (
            text("requiredOutcome"),
            CanonicalValue::Map(vec![
                (text("disposition"), text(disposition)),
                (text("contract"), text(contract)),
            ]),
        ),
    ])
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

#[test]
fn executable_registry_accepts_exact_laws_before_corpus_publication() {
    let mut value = checked_corpus_value();
    let cases = map_field_mut(&mut value, "cases");
    let CanonicalValue::Map(entries) = cases else {
        panic!("cases are a map");
    };
    entries.push((
        text("ambient-capability-denial"),
        case_contract(
            "component-preflight",
            "ambient-capabilities-denied",
            "rejected",
            "ambient-capability-preflight-denied",
        ),
    ));
    entries.push((
        text("noncanonical-output"),
        case_contract(
            "host-output-admission",
            "noncanonical-cbor-output",
            "rejected",
            "noncanonical-target-ir-output-denied",
        ),
    ));
    entries.push((
        text("unsupported-semantics"),
        case_contract(
            "lowering",
            "unsupported-core-semantics",
            "refused",
            "unsupported-core-semantics-refused",
        ),
    ));
    entries.push((
        text("output-overclaim"),
        case_contract(
            "verification",
            "unsupported-output-role-requested",
            "refused",
            "unsupported-verifier-output-role-refused",
        ),
    ));
    entries.push((
        text("wrong-intrinsic"),
        case_contract(
            "verification",
            "target-intrinsic-changed",
            "rejected",
            "target-intrinsic-mismatch-rejected",
        ),
    ));
    entries.push((
        text("dropped-obstruction"),
        case_contract(
            "verification",
            "obstruction-arm-removed",
            "rejected",
            "obstruction-relation-mismatch-rejected",
        ),
    ));
    entries.push((
        text("artifact-tamper"),
        case_contract(
            "request-admission",
            "artifact-bytes-changed",
            "rejected",
            "artifact-digest-mismatch-rejected",
        ),
    ));
    entries.push((
        text("schema-tamper"),
        case_contract(
            "schema-admission",
            "schema-bytes-changed",
            "rejected",
            "schema-artifact-digest-mismatch-rejected",
        ),
    ));
    entries.push((
        text("component-tamper"),
        case_contract(
            "component-preflight",
            "component-bytes-changed",
            "rejected",
            "component-digest-mismatch-rejected",
        ),
    ));
    let bytes = encode_canonical_cbor(&value).expect("synthetic registry corpus is canonical");
    let cases = decode_declared_cases(&bytes).expect("reviewed executable laws have exact owners");

    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::AmbientCapabilityPreflightDenied
            && case.owner() == ExecutorOwner::Host
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::NoncanonicalTargetIrOutputDenied
            && case.owner() == ExecutorOwner::Host
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::UnsupportedCoreSemanticsRefused
            && case.owner() == ExecutorOwner::Host
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::UnsupportedVerifierOutputRoleRefused
            && case.owner() == ExecutorOwner::Host
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::TargetIntrinsicMismatchRejected
            && case.owner() == ExecutorOwner::Host
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::ObstructionRelationMismatchRejected
            && case.owner() == ExecutorOwner::Host
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::ArtifactDigestMismatchRejected
            && case.owner() == ExecutorOwner::Package
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::SchemaArtifactDigestMismatchRejected
            && case.owner() == ExecutorOwner::Package
    }));
    assert!(cases.iter().any(|case| {
        case.contract() == ExecutableContract::ComponentDigestMismatchRejected
            && case.owner() == ExecutorOwner::Package
    }));
}

#[test]
fn duplicate_case_ids_cannot_reach_executable_dispatch() {
    let mut value = checked_corpus_value();
    let cases = map_field_mut(&mut value, "cases");
    let CanonicalValue::Map(entries) = cases else {
        panic!("cases are a map");
    };
    entries.push((
        text("package-parity"),
        case_contract(
            "component-preflight",
            "ambient-capabilities-denied",
            "rejected",
            "ambient-capability-preflight-denied",
        ),
    ));

    let Ok(bytes) = encode_canonical_cbor(&value) else {
        return;
    };
    let error = decode_declared_cases(&bytes)
        .expect_err("the decoder or executable parser rejects duplicate case IDs");
    assert!(matches!(
        error.kind(),
        CorpusContractErrorKind::CanonicalCborInvalid | CorpusContractErrorKind::DuplicateCaseId
    ));
}
