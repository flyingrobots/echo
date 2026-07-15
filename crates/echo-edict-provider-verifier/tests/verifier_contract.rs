// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Executable contract for the first pure Echo Edict provider verifier.

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1, CanonicalValueV1,
};
use echo_edict_provider_verifier::{
    verify, Artifact, BoundArtifact, DiagnosticSeverity, Digest, DigestAlgorithm,
    ProtocolVersionV1, ProviderRefusalKind, ResourceRef, ResponseLimitsV1, SemanticInput,
    SemanticInputKind, VerificationOutputKind, VerificationOutputRequest, VerificationRequestV1,
    VerificationSuccessV1,
};
use sha2::{Digest as _, Sha256};

const TARGET_PROFILE: &[u8] = include_bytes!("../resources/target-profile.echo-dpo.cbor");
const LAWPACK: &[u8] = include_bytes!("../resources/lawpack.echo-dpo.cbor");
const TARGET_AUTHORITY: &[u8] = include_bytes!("../resources/authority-facts.echo-dpo.cbor");
const LAWPACK_AUTHORITY: &[u8] = include_bytes!("../resources/authority-facts.echo-lawpack.cbor");

const CORE_DOMAIN: &str = "edict.core.module/v1";
const TARGET_PROFILE_DOMAIN: &str = "edict.target-profile/v1";
const LAWPACK_DOMAIN: &str = "edict.lawpack/v1";
const AUTHORITY_DOMAIN: &str = "edict.authority-facts/v1";
const LOWERABILITY_DOMAIN: &str = "edict.lowering-requirements/v1";
const TARGET_IR_DOMAIN: &str = "edict.target-ir.artifact/v1";
const REPORT_DOMAIN: &str = "echo.verifier-report/v1";
const REPORT_ROLE: &str = "verifier-report.echo-dpo";
const DIAGNOSTIC_ABI_DIGEST: &str =
    "28fd72a98223153982ca084c29dbb1b2d430623967ab3b6db9d7fee668e614b9";

fn text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn map(entries: impl IntoIterator<Item = (&'static str, CanonicalValueV1)>) -> CanonicalValueV1 {
    let mut entries = entries
        .into_iter()
        .map(|(key, value)| (text(key), value))
        .collect::<Vec<_>>();
    entries.sort_by_cached_key(|(key, _)| canonical_bytes(key));
    CanonicalValueV1::Map(entries)
}

fn canonical_bytes(value: &CanonicalValueV1) -> Vec<u8> {
    encode_canonical_cbor_v1(value).expect("test value has a canonical encoding")
}

fn bound(coordinate: &str, domain: &str, bytes: impl Into<Vec<u8>>) -> BoundArtifact {
    let bytes = bytes.into();
    let value = decode_canonical_cbor_v1(&bytes).expect("fixture is canonical CBOR");
    let digest = digest_canonical_value_v1(domain, &value)
        .expect("fixture has a domain-framed digest")
        .strip_prefix("sha256:")
        .expect("fixture digest uses sha256")
        .to_owned();
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: hex::decode(digest).expect("fixture digest is hexadecimal"),
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

fn lowerability_value() -> CanonicalValueV1 {
    let guard_kinds = || CanonicalValueV1::Array(vec![text("precommit-atomic")]);
    let obstructions = || CanonicalValueV1::Array(vec![text("rejected")]);
    let footprints = || CanonicalValueV1::Array(vec![text("target.replace.footprint")]);
    let costs = || CanonicalValueV1::Array(vec![text("target.replace.cost")]);
    map([
        ("apiVersion", text(LOWERABILITY_DOMAIN)),
        ("operationProfile", text("continuum.profile.write/v1")),
        (
            "semanticEffects",
            CanonicalValueV1::Array(vec![map([
                ("coordinate", text("target.replace")),
                ("writeClass", text("replace")),
                ("guardKinds", guard_kinds()),
                ("obstructionCoordinates", obstructions()),
                ("footprintObligations", footprints()),
                ("costObligations", costs()),
            ])]),
        ),
        (
            "requiredWriteClasses",
            CanonicalValueV1::Array(vec![text("replace")]),
        ),
        ("guardKinds", guard_kinds()),
        ("atomicity", text("atomic")),
        ("postconditionSupport", CanonicalValueV1::Bool(true)),
        ("obstructionCoordinates", obstructions()),
        ("footprintObligations", footprints()),
        ("costObligations", costs()),
        ("opticContract", text("replace-point")),
    ])
}

fn limits(max_total_response_bytes: u64) -> ResponseLimitsV1 {
    ResponseLimitsV1 {
        max_output_count: 8,
        max_diagnostic_count: 8,
        max_total_response_bytes,
    }
}

fn request_with_target_ir(
    target_ir_bytes: impl Into<Vec<u8>>,
    response_limits: ResponseLimitsV1,
) -> VerificationRequestV1 {
    VerificationRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: bound(
            "a.b@1",
            CORE_DOMAIN,
            hex::decode(include_str!("fixtures/edict-core.hex").trim())
                .expect("Core fixture hex is valid"),
        ),
        target_profile: bound("echo.dpo@1", TARGET_PROFILE_DOMAIN, TARGET_PROFILE.to_vec()),
        target_ir: bound("echo.target-ir@1", TARGET_IR_DOMAIN, target_ir_bytes.into()),
        semantic_inputs: vec![
            SemanticInput {
                role: "authority-facts.echo-dpo".to_owned(),
                kind: SemanticInputKind::AuthorityFacts,
                artifact: bound(
                    "echo.dpo-authority-facts@1",
                    AUTHORITY_DOMAIN,
                    TARGET_AUTHORITY.to_vec(),
                ),
            },
            SemanticInput {
                role: "authority-facts.echo-lawpack".to_owned(),
                kind: SemanticInputKind::AuthorityFacts,
                artifact: bound(
                    "echo.dpo-lawpack-authority-facts@1",
                    AUTHORITY_DOMAIN,
                    LAWPACK_AUTHORITY.to_vec(),
                ),
            },
            SemanticInput {
                role: "lawpack.echo-dpo".to_owned(),
                kind: SemanticInputKind::Lawpack,
                artifact: bound("echo.dpo-lawpack@1", LAWPACK_DOMAIN, LAWPACK.to_vec()),
            },
            SemanticInput {
                role: "lowerability.echo-dpo".to_owned(),
                kind: SemanticInputKind::LowerabilityFacts,
                artifact: bound(
                    "echo.dpo-lowerability@1",
                    LOWERABILITY_DOMAIN,
                    canonical_bytes(&lowerability_value()),
                ),
            },
        ],
        requested_outputs: vec![VerificationOutputRequest {
            role: REPORT_ROLE.to_owned(),
            kind: VerificationOutputKind::VerifierReport,
            domain: REPORT_DOMAIN.to_owned(),
        }],
        limits: response_limits,
    }
}

fn request() -> VerificationRequestV1 {
    request_with_target_ir(
        hex::decode(include_str!("fixtures/edict-target-ir.hex").trim())
            .expect("Target IR fixture hex is valid"),
        limits(64 * 1024),
    )
}

fn core_value() -> CanonicalValueV1 {
    decode_canonical_cbor_v1(
        &hex::decode(include_str!("fixtures/edict-core.hex").trim())
            .expect("Core fixture hex is valid"),
    )
    .expect("Core fixture is canonical")
}

fn target_ir_value() -> CanonicalValueV1 {
    decode_canonical_cbor_v1(
        &hex::decode(include_str!("fixtures/edict-target-ir.hex").trim())
            .expect("Target IR fixture hex is valid"),
    )
    .expect("Target IR fixture is canonical")
}

fn bind_core(request: &mut VerificationRequestV1, core: &CanonicalValueV1) {
    request.core = bound("a.b@1", CORE_DOMAIN, canonical_bytes(core));
}

fn bind_target_ir(request: &mut VerificationRequestV1, target_ir: &CanonicalValueV1) {
    request.target_ir = bound(
        "echo.target-ir@1",
        TARGET_IR_DOMAIN,
        canonical_bytes(target_ir),
    );
}

fn map_field<'a>(value: &'a CanonicalValueV1, field: &str) -> &'a CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("value is not a map");
    };
    entries
        .iter()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .unwrap_or_else(|| panic!("map field {field} is absent"))
}

fn map_field_mut<'a>(value: &'a mut CanonicalValueV1, field: &str) -> &'a mut CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("value is not a map");
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .unwrap_or_else(|| panic!("map field {field} is absent"))
}

fn array_mut(value: &mut CanonicalValueV1) -> &mut Vec<CanonicalValueV1> {
    let CanonicalValueV1::Array(values) = value else {
        panic!("value is not an array");
    };
    values
}

fn map_entries_mut(value: &mut CanonicalValueV1) -> &mut Vec<(CanonicalValueV1, CanonicalValueV1)> {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("value is not a map");
    };
    entries
}

fn insert_map_field(value: &mut CanonicalValueV1, field: &str, inserted: CanonicalValueV1) {
    let entries = map_entries_mut(value);
    entries.push((text(field), inserted));
    entries.sort_by_cached_key(|(key, _)| canonical_bytes(key));
}

fn remove_map_field(value: &mut CanonicalValueV1, field: &str) -> CanonicalValueV1 {
    let entries = map_entries_mut(value);
    let index = entries
        .iter()
        .position(|(key, _)| key == &text(field))
        .unwrap_or_else(|| panic!("map field {field} is absent"));
    entries.remove(index).1
}

fn map_field_index(value: &CanonicalValueV1, field: &str) -> usize {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("value is not a map");
    };
    entries
        .iter()
        .position(|(key, _)| key == &text(field))
        .unwrap_or_else(|| panic!("map field {field} is absent"))
}

fn core_result_mut(core: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    let intent = map_field_mut(map_field_mut(core, "intents"), "t");
    map_field_mut(map_field_mut(intent, "body"), "result")
}

fn core_effect_mut(core: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    let intent = map_field_mut(map_field_mut(core, "intents"), "t");
    let nodes = array_mut(map_field_mut(map_field_mut(intent, "body"), "nodes"));
    &mut nodes[0]
}

fn target_intent_mut(target_ir: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    map_field_mut(map_field_mut(target_ir, "intents"), "t")
}

fn target_step_mut(target_ir: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    let steps = array_mut(map_field_mut(target_intent_mut(target_ir), "steps"));
    &mut steps[0]
}

fn assert_rejected(request: VerificationRequestV1, expected_code: &str) {
    let target_ir_reference = request.target_ir.reference.clone();
    let success = verify(request).expect("semantic disagreement produces a verifier report");
    assert_eq!(success.diagnostics.len(), 1);
    assert_eq!(success.diagnostics[0].severity, DiagnosticSeverity::Error);
    assert_eq!(success.diagnostics[0].code, expected_code);
    assert_report_common(&report(&success), &target_ir_reference, "rejected");
}

fn assert_accepted(success: &VerificationSuccessV1, target_ir_reference: &ResourceRef) {
    assert!(success.diagnostics.is_empty());
    assert_report_common(&report(success), target_ir_reference, "accepted");
}

fn text_value(value: &CanonicalValueV1) -> &str {
    let CanonicalValueV1::Text(value) = value else {
        panic!("value is not text");
    };
    value
}

fn report(success: &echo_edict_provider_verifier::VerificationSuccessV1) -> CanonicalValueV1 {
    assert_eq!(success.outputs.len(), 1);
    let output = &success.outputs[0];
    assert_eq!(output.role, REPORT_ROLE);
    assert_eq!(output.kind, VerificationOutputKind::VerifierReport);
    assert_eq!(output.artifact.domain, REPORT_DOMAIN);
    assert_eq!(output.logical_path, None);
    decode_canonical_cbor_v1(&output.artifact.bytes).expect("report is canonical CBOR")
}

fn assert_report_common(
    report: &CanonicalValueV1,
    target_ir_reference: &ResourceRef,
    expected_outcome: &str,
) {
    let CanonicalValueV1::Map(fields) = report else {
        panic!("report is not a map");
    };
    assert_eq!(fields.len(), 5);
    assert_eq!(
        text_value(map_field(report, "apiVersion")),
        "echo.verifier-report/v1"
    );
    assert_eq!(
        map_field(report, "targetIr"),
        &map([
            ("id", text(&target_ir_reference.coordinate)),
            (
                "digest",
                CanonicalValueV1::Array(vec![
                    text("sha256"),
                    CanonicalValueV1::Bytes(target_ir_reference.digest.bytes.clone()),
                ]),
            ),
        ])
    );
    assert_eq!(text_value(map_field(report, "outcome")), expected_outcome);
    let diagnostic_abi = map_field(report, "diagnosticAbi");
    assert_eq!(
        text_value(map_field(diagnostic_abi, "id")),
        "edict.diagnostics/v1"
    );
    assert_eq!(
        map_field(diagnostic_abi, "digest"),
        &CanonicalValueV1::Array(vec![
            text("sha256"),
            CanonicalValueV1::Bytes(
                hex::decode(DIAGNOSTIC_ABI_DIGEST).expect("diagnostic digest is valid")
            ),
        ])
    );
    assert_eq!(
        map_field(report, "diagnosticBytes"),
        &CanonicalValueV1::Bytes(Vec::new())
    );
}

#[test]
fn exact_one_operation_relation_is_accepted() {
    let request = request();
    let target_ir_reference = request.target_ir.reference.clone();
    let success = verify(request).expect("the exact reviewed relation verifies");

    assert!(success.diagnostics.is_empty());
    assert_report_common(&report(&success), &target_ir_reference, "accepted");
}

#[test]
fn packaged_semantic_resource_bytes_are_pinned() {
    let resources = [
        (
            TARGET_PROFILE,
            "95626e5be6e6b2c1c8aa1858277f1c67487ab6724b08408eb3c0054adce6b1eb",
        ),
        (
            LAWPACK,
            "df62a4ff2b56f9553c80cf400728cab3717f5f442c4c2fc415d2c89c21c41dad",
        ),
        (
            TARGET_AUTHORITY,
            "d17b03810ecc53f288aa1de457a5ba295c537c4f64046f8e3777b8f98ff3fc86",
        ),
        (
            LAWPACK_AUTHORITY,
            "3911de5075d3709a3ba40419e4b67f1226961f3520ba9dfbbad78278c9bb0e96",
        ),
    ];

    for (bytes, expected_sha256) in resources {
        assert_eq!(hex::encode(Sha256::digest(bytes)), expected_sha256);
    }
}

#[test]
fn changed_target_intrinsic_is_rejected() {
    let mut target_ir = decode_canonical_cbor_v1(
        &hex::decode(include_str!("fixtures/edict-target-ir.hex").trim())
            .expect("Target IR fixture hex is valid"),
    )
    .expect("Target IR fixture is canonical");
    let intent = map_field_mut(map_field_mut(&mut target_ir, "intents"), "t");
    let CanonicalValueV1::Array(steps) = map_field_mut(intent, "steps") else {
        panic!("steps is not an array");
    };
    *map_field_mut(&mut steps[0], "targetIntrinsic") = text("echo.dpo@1.unreviewed");

    let request = request_with_target_ir(canonical_bytes(&target_ir), limits(64 * 1024));
    let target_ir_reference = request.target_ir.reference.clone();
    let success = verify(request).expect("semantic disagreement produces a verifier report");

    assert_eq!(success.diagnostics.len(), 1);
    assert_eq!(
        success.diagnostics[0].code,
        "echo.verifier.target-intrinsic-mismatch"
    );
    assert_eq!(success.diagnostics[0].severity, DiagnosticSeverity::Error);
    assert_report_common(&report(&success), &target_ir_reference, "rejected");
}

#[test]
fn output_overclaim_names_the_first_unsupported_role() {
    let mut case = request();
    case.requested_outputs.push(VerificationOutputRequest {
        role: "verifier-report.unreviewed".to_owned(),
        kind: VerificationOutputKind::VerifierReport,
        domain: REPORT_DOMAIN.to_owned(),
    });

    let refusal = verify(case).expect_err("an undeclared second output must refuse");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedOutputRole);
    assert_eq!(
        refusal.subject.as_deref(),
        Some("verifier-report.unreviewed")
    );
}

#[test]
fn repeated_verification_is_byte_identical() {
    let first = verify(request()).expect("first verification succeeds");
    let second = verify(request_with_target_ir(
        hex::decode(include_str!("fixtures/edict-target-ir.hex").trim())
            .expect("Target IR fixture hex is valid"),
        limits(1024 * 1024),
    ))
    .expect("second verification succeeds");

    assert_eq!(first, second);
    assert_eq!(
        first.outputs[0].artifact.bytes,
        second.outputs[0].artifact.bytes
    );
}

#[test]
fn target_domain_and_profile_bindings_are_exact() {
    let mut inner_domain = target_ir_value();
    *map_field_mut(&mut inner_domain, "domain") = text("echo.span-ir/v2");
    let mut case = request();
    bind_target_ir(&mut case, &inner_domain);
    assert_rejected(case, "echo.verifier.target-domain-mismatch");

    let mut profile_id = target_ir_value();
    *map_field_mut(map_field_mut(&mut profile_id, "targetProfile"), "id") =
        text("echo.dpo@1.rogue");
    let mut case = request();
    bind_target_ir(&mut case, &profile_id);
    assert_rejected(case, "echo.verifier.target-profile-mismatch");

    let mut profile_digest = target_ir_value();
    let digest = map_field_mut(
        map_field_mut(&mut profile_digest, "targetProfile"),
        "digest",
    );
    array_mut(digest)[1] = CanonicalValueV1::Bytes(vec![0; 32]);
    let mut case = request();
    bind_target_ir(&mut case, &profile_digest);
    assert_rejected(case, "echo.verifier.target-profile-mismatch");

    let mut rogue_request = request();
    rogue_request.target_profile.reference.coordinate = "echo.dpo@1.rogue".to_owned();
    let refusal = verify(rogue_request).expect_err("an unauthorized supplied profile must refuse");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedTargetProfile);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.verifier.unsupported-target-profile"
    );
}

#[test]
fn copied_budget_disagreement_is_rejected() {
    let mut target_ir = target_ir_value();
    let budget = map_field_mut(target_intent_mut(&mut target_ir), "coreEvaluationBudget");
    *map_field_mut(budget, "maxSteps") = CanonicalValueV1::Integer(9);
    let mut request = request();
    bind_target_ir(&mut request, &target_ir);

    assert_rejected(request, "echo.verifier.budget-mismatch");
}

#[test]
fn obstruction_relation_is_exact() {
    let mut failures_dropped = target_ir_value();
    *map_field_mut(
        target_step_mut(&mut failures_dropped),
        "obstructionFailures",
    ) = CanonicalValueV1::Array(Vec::new());
    let mut case = request();
    bind_target_ir(&mut case, &failures_dropped);
    assert_rejected(case, "echo.verifier.obstruction-mismatch");

    let mut arms_dropped = target_ir_value();
    *map_field_mut(target_step_mut(&mut arms_dropped), "obstructionArms") =
        CanonicalValueV1::Map(Vec::new());
    let mut case = request();
    bind_target_ir(&mut case, &arms_dropped);
    assert_rejected(case, "echo.verifier.obstruction-mismatch");

    let mut both_dropped = target_ir_value();
    let step = target_step_mut(&mut both_dropped);
    *map_field_mut(step, "obstructionFailures") = CanonicalValueV1::Array(Vec::new());
    *map_field_mut(step, "obstructionArms") = CanonicalValueV1::Map(Vec::new());
    let mut case = request();
    bind_target_ir(&mut case, &both_dropped);
    assert_rejected(case, "echo.verifier.obstruction-mismatch");

    let mut changed_arm = target_ir_value();
    let arms = map_field_mut(target_step_mut(&mut changed_arm), "obstructionArms");
    let arm = map_field_mut(arms, "rejected");
    *map_field_mut(map_field_mut(arm, "value"), "callee") = text("domain.Other");
    let mut case = request();
    bind_target_ir(&mut case, &changed_arm);
    assert_rejected(case, "echo.verifier.obstruction-mismatch");

    let mut extra_arm = target_ir_value();
    let step = target_step_mut(&mut extra_arm);
    array_mut(map_field_mut(step, "obstructionFailures")).push(text("other"));
    let existing_arm = map_field(map_field(step, "obstructionArms"), "rejected").clone();
    insert_map_field(
        map_field_mut(step, "obstructionArms"),
        "other",
        existing_arm,
    );
    let mut case = request();
    bind_target_ir(&mut case, &extra_arm);
    assert_rejected(case, "echo.verifier.obstruction-mismatch");
}

#[test]
fn target_only_claims_are_rejected() {
    let mut extra_intent = target_ir_value();
    let existing_intent = map_field(map_field(&extra_intent, "intents"), "t").clone();
    insert_map_field(
        map_field_mut(&mut extra_intent, "intents"),
        "u",
        existing_intent,
    );
    let mut case = request();
    bind_target_ir(&mut case, &extra_intent);
    assert_rejected(case, "echo.verifier.introduced-claim");

    let mut extra_step = target_ir_value();
    let steps = array_mut(map_field_mut(target_intent_mut(&mut extra_step), "steps"));
    steps.push(steps[0].clone());
    let mut case = request();
    bind_target_ir(&mut case, &extra_step);
    assert_rejected(case, "echo.verifier.introduced-claim");

    let mut extra_requirement = target_ir_value();
    let requirement = map([
        ("id", text("guard.0")),
        ("predicate", map([("kind", text("true"))])),
        (
            "onFailure",
            map([
                ("kind", text("terminal")),
                (
                    "reason",
                    map([
                        ("reasonKind", text("domain.GuardRejected")),
                        ("payload", CanonicalValueV1::Map(Vec::new())),
                    ]),
                ),
            ]),
        ),
    ]);
    array_mut(map_field_mut(
        target_intent_mut(&mut extra_requirement),
        "requirements",
    ))
    .push(requirement);
    let mut case = request();
    bind_target_ir(&mut case, &extra_requirement);
    assert_rejected(case, "echo.verifier.introduced-claim");
}

#[test]
fn silent_loss_is_rejected() {
    let mut missing_intent = target_ir_value();
    remove_map_field(map_field_mut(&mut missing_intent, "intents"), "t");
    let mut case = request();
    bind_target_ir(&mut case, &missing_intent);
    assert_rejected(case, "echo.verifier.silent-loss");

    let mut missing_step = target_ir_value();
    array_mut(map_field_mut(target_intent_mut(&mut missing_step), "steps")).clear();
    let mut case = request();
    bind_target_ir(&mut case, &missing_step);
    assert_rejected(case, "echo.verifier.silent-loss");
}

#[test]
fn canonical_decoding_does_not_admit_a_malformed_target_ir() {
    let mut missing_kind = target_ir_value();
    remove_map_field(&mut missing_kind, "kind");
    let mut invalid_profile_ref = target_ir_value();
    *map_field_mut(
        map_field_mut(&mut invalid_profile_ref, "targetProfile"),
        "digest",
    ) = text("not-a-digest");
    let mut invalid_step = target_ir_value();
    *map_field_mut(target_intent_mut(&mut invalid_step), "steps") =
        CanonicalValueV1::Array(vec![CanonicalValueV1::Null]);
    let mut invalid_result = target_ir_value();
    *map_field_mut(target_intent_mut(&mut invalid_result), "result") =
        map([("kind", text("unknown"))]);

    for malformed in [
        CanonicalValueV1::Text("not-target-ir".to_owned()),
        missing_kind,
        invalid_profile_ref,
        invalid_step,
        invalid_result,
    ] {
        let mut case = request();
        bind_target_ir(&mut case, &malformed);
        let refusal = verify(case).expect_err("malformed Target IR must refuse");
        assert_eq!(refusal.kind, ProviderRefusalKind::InvalidSemanticArtifact);
        assert_eq!(
            refusal.diagnostics[0].code,
            "echo.verifier.invalid-semantic-artifact"
        );
    }
}

#[test]
fn semantic_guard_and_footprint_declarations_are_pinned() {
    let profile = decode_canonical_cbor_v1(TARGET_PROFILE).expect("target profile is canonical");
    assert_eq!(
        text_value(map_field(&profile, "guardEvaluation")),
        "precommit-atomic"
    );
    assert_eq!(
        text_value(map_field(map_field(&profile, "footprintAlgebra"), "id")),
        "echo.dpo.footprint/v1"
    );
    let lowerability = lowerability_value();
    assert_eq!(
        map_field(&lowerability, "guardKinds"),
        &CanonicalValueV1::Array(vec![text("precommit-atomic")])
    );
    assert_eq!(
        map_field(&lowerability, "footprintObligations"),
        &CanonicalValueV1::Array(vec![text("target.replace.footprint")])
    );

    let mut changed_guard = lowerability_value();
    *map_field_mut(&mut changed_guard, "guardKinds") =
        CanonicalValueV1::Array(vec![text("postcommit")]);
    let mut case = request();
    case.semantic_inputs[3].artifact = bound(
        "echo.dpo-lowerability@1",
        LOWERABILITY_DOMAIN,
        canonical_bytes(&changed_guard),
    );
    let refusal = verify(case).expect_err("changed guard obligations must refuse");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.verifier.unsupported-semantics"
    );

    let mut changed_footprint = lowerability_value();
    *map_field_mut(&mut changed_footprint, "footprintObligations") =
        CanonicalValueV1::Array(vec![text("target.replace.unreviewed-footprint")]);
    let mut case = request();
    case.semantic_inputs[3].artifact = bound(
        "echo.dpo-lowerability@1",
        LOWERABILITY_DOMAIN,
        canonical_bytes(&changed_footprint),
    );
    let refusal = verify(case).expect_err("changed footprint obligations must refuse");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.verifier.unsupported-semantics"
    );
}

#[test]
fn effect_input_relation_is_dynamic_and_exact() {
    let alternate_input = map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("string")), ("value", text("changed"))]),
        ),
    ]);
    let mut core = core_value();
    *map_field_mut(core_effect_mut(&mut core), "input") = alternate_input.clone();
    let mut correlated_target = target_ir_value();
    *map_field_mut(target_step_mut(&mut correlated_target), "input") = alternate_input;
    let mut correlated = request();
    bind_core(&mut correlated, &core);
    bind_target_ir(&mut correlated, &correlated_target);
    let target_ir_reference = correlated.target_ir.reference.clone();
    let success = verify(correlated).expect("a correlated effect input change must verify");
    assert_accepted(&success, &target_ir_reference);

    let mut target_only = request();
    bind_target_ir(&mut target_only, &correlated_target);
    assert_rejected(target_only, "echo.verifier.effect-input-mismatch");
}

#[test]
fn string_bound_counts_unicode_scalar_values() {
    let authored = "🦀".repeat(16);
    let bounded_input = map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("string")), ("value", text(&authored))]),
        ),
    ]);
    let mut core = core_value();
    *map_field_mut(core_effect_mut(&mut core), "input") = bounded_input.clone();
    let mut target_ir = target_ir_value();
    *map_field_mut(target_step_mut(&mut target_ir), "input") = bounded_input;
    let mut case = request();
    bind_core(&mut case, &core);
    bind_target_ir(&mut case, &target_ir);
    let target_ir_reference = case.target_ir.reference.clone();

    let success = verify(case).expect("sixteen Unicode scalar values remain within the bound");
    assert_accepted(&success, &target_ir_reference);
}

#[test]
fn type_wrong_correlated_artifacts_refuse_instead_of_agreeing() {
    let mut wrong_effect_core = core_value();
    let input_local = {
        let intent = map_field(map_field(&wrong_effect_core, "intents"), "t");
        let body = map_field(intent, "body");
        let CanonicalValueV1::Array(locals) = map_field(body, "locals") else {
            panic!("Core locals are not an array");
        };
        locals[0].clone()
    };
    let record_typed_input = map([("kind", text("local")), ("ref", input_local)]);
    *map_field_mut(core_effect_mut(&mut wrong_effect_core), "input") = record_typed_input.clone();
    let mut wrong_effect_target = target_ir_value();
    *map_field_mut(target_step_mut(&mut wrong_effect_target), "input") = record_typed_input;
    let mut wrong_effect = request();
    bind_core(&mut wrong_effect, &wrong_effect_core);
    bind_target_ir(&mut wrong_effect, &wrong_effect_target);
    let refusal = verify(wrong_effect).expect_err("a record cannot become an effect scalar");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);

    let scalar_result = map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("string")), ("value", text("wrong-shape"))]),
        ),
    ]);
    let mut wrong_result_core = core_value();
    *core_result_mut(&mut wrong_result_core) = scalar_result.clone();
    let mut wrong_result_target = target_ir_value();
    *map_field_mut(target_intent_mut(&mut wrong_result_target), "result") = scalar_result;
    let mut wrong_result = request();
    bind_core(&mut wrong_result, &wrong_result_core);
    bind_target_ir(&mut wrong_result, &wrong_result_target);
    let refusal = verify(wrong_result).expect_err("a scalar cannot become the output record");
    assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);

    for local_index in [0, 1] {
        let mut wrong_nominal_core = core_value();
        let local = {
            let intent = map_field(map_field(&wrong_nominal_core, "intents"), "t");
            let body = map_field(intent, "body");
            let CanonicalValueV1::Array(locals) = map_field(body, "locals") else {
                panic!("Core locals are not an array");
            };
            locals[local_index].clone()
        };
        let nominally_wrong = map([("kind", text("local")), ("ref", local)]);
        *core_result_mut(&mut wrong_nominal_core) = nominally_wrong.clone();
        let mut wrong_nominal_target = target_ir_value();
        *map_field_mut(target_intent_mut(&mut wrong_nominal_target), "result") = nominally_wrong;
        let mut case = request();
        bind_core(&mut case, &wrong_nominal_core);
        bind_target_ir(&mut case, &wrong_nominal_target);
        let refusal = verify(case).expect_err("Input and Receipt are not Output records");
        assert_eq!(refusal.kind, ProviderRefusalKind::UnsupportedSemantics);
    }
}

#[test]
fn relation_is_derived_from_the_supplied_core_result() {
    let mut core = core_value();
    let intent = map_field_mut(map_field_mut(&mut core, "intents"), "t");
    let body = map_field_mut(intent, "body");
    let (input_local, receipt_local) = {
        let CanonicalValueV1::Array(locals) = map_field(body, "locals") else {
            panic!("Core locals are not an array");
        };
        (locals[0].clone(), locals[1].clone())
    };
    let result_from = |local| {
        map([
            ("kind", text("record")),
            (
                "fields",
                map([(
                    "id",
                    map([
                        ("kind", text("field")),
                        ("base", map([("kind", text("local")), ("ref", local)])),
                        ("field", text("id")),
                    ]),
                )]),
            ),
        ])
    };
    let original_result = map_field(body, "result").clone();
    let input_result = result_from(input_local);
    let alternate_result = if input_result == original_result {
        result_from(receipt_local)
    } else {
        input_result
    };
    *map_field_mut(body, "result") = alternate_result.clone();

    let mut correlated_target = target_ir_value();
    *map_field_mut(target_intent_mut(&mut correlated_target), "result") = alternate_result;
    let mut correlated_request = request();
    bind_core(&mut correlated_request, &core);
    bind_target_ir(&mut correlated_request, &correlated_target);
    let target_ir_reference = correlated_request.target_ir.reference.clone();
    let success = verify(correlated_request).expect("a correlated result change must verify");
    assert_accepted(&success, &target_ir_reference);

    let mut one_sided_request = request();
    bind_target_ir(&mut one_sided_request, &correlated_target);
    assert_rejected(one_sided_request, "echo.verifier.result-mismatch");
}

#[test]
fn recursive_expression_dispatch_is_bounded_and_order_independent() {
    let core = core_value();
    let result = map_field(
        map_field(map_field(map_field(&core, "intents"), "t"), "body"),
        "result",
    );
    assert!(map_field_index(result, "kind") < map_field_index(result, "fields"));
    let CanonicalValueV1::Map(fields) = map_field(result, "fields") else {
        panic!("result fields are not a map");
    };
    let nested_field = &fields[0].1;
    assert!(map_field_index(nested_field, "base") < map_field_index(nested_field, "kind"));
    verify(request()).expect("both canonical map orders must verify");

    for (malformed, expected_code) in [
        (
            map([("value", text("missing discriminator"))]),
            "echo.verifier.invalid-expression-discriminator",
        ),
        (
            map([("kind", text("unknown"))]),
            "echo.verifier.invalid-expression-discriminator",
        ),
        (
            map([
                ("kind", text("local")),
                ("fields", CanonicalValueV1::Map(Vec::new())),
            ]),
            "echo.verifier.invalid-expression-shape",
        ),
    ] {
        let mut malformed_core = core_value();
        *core_result_mut(&mut malformed_core) = malformed;
        let mut request = request();
        bind_core(&mut request, &malformed_core);
        let refusal = verify(request).expect_err("a malformed expression must refuse");
        assert_eq!(refusal.kind, ProviderRefusalKind::InvalidSemanticArtifact);
        assert_eq!(refusal.diagnostics[0].code, expected_code);
    }

    let mut deep_child = map([("kind", text("unknown"))]);
    for _ in 0..72 {
        deep_child = map([
            ("kind", text("field")),
            ("base", deep_child),
            ("field", text("x")),
        ]);
    }
    let malicious_outer = map([("kind", text("unknown")), ("base", deep_child)]);
    let mut malicious_core = core_value();
    *core_result_mut(&mut malicious_core) = malicious_outer;
    let mut request = request();
    bind_core(&mut request, &malicious_core);
    let refusal = verify(request).expect_err("the outer discriminator must fail closed");
    assert_eq!(refusal.kind, ProviderRefusalKind::InvalidSemanticArtifact);
    assert_eq!(
        refusal.diagnostics[0].code,
        "echo.verifier.invalid-expression-discriminator"
    );
}

#[test]
fn native_result_is_invariant_under_all_host_limits() {
    let expected = verify(request()).expect("baseline verification succeeds");
    let mut zero_limits = request();
    zero_limits.limits = ResponseLimitsV1 {
        max_output_count: 0,
        max_diagnostic_count: 0,
        max_total_response_bytes: 0,
    };

    assert_eq!(
        verify(zero_limits).expect("limits are enforced by the host"),
        expected
    );
}
