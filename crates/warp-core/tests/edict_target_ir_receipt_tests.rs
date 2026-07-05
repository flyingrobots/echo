// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Edict Target IR obstruction receipt bridge tests.

use warp_core::{
    accept_edict_echo_target_ir, execute_accepted_edict_echo_target_ir, ContractObstructionKind,
    EdictEchoAttemptInput, EdictEchoAttemptOutcomeKind, EdictEchoObstructionReason,
    EdictEchoTargetIrAcceptanceErrorKind, EdictEchoTargetIrArtifact, EdictEchoTargetIrDigestField,
    EdictEchoTargetIrIntent, EdictEchoTargetIrPredicate, EdictEchoTargetIrRequirement,
    EdictEchoTargetIrRequirementFailure, EDICT_ECHO_ATTEMPT_RECEIPT_SCHEMA,
    EDICT_ECHO_TARGET_IR_DOMAIN,
};

const TARGET_PROFILE_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TARGET_IR_DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn edict_echo_target_ir_obstructed_attempt_binds_digest() -> Result<(), String> {
    let accepted = accept_edict_echo_target_ir(&supported_artifact())
        .map_err(|error| format!("supported fixture rejected: {error:?}"))?;

    let receipt_a = execute_accepted_edict_echo_target_ir(&accepted, stale_basis_input());
    let receipt_b = execute_accepted_edict_echo_target_ir(&accepted, stale_basis_input());

    assert_eq!(receipt_a, receipt_b);
    assert_eq!(receipt_a.receipt_schema, EDICT_ECHO_ATTEMPT_RECEIPT_SCHEMA);
    assert_eq!(receipt_a.target_ir_digest, digest(0xbb));
    assert_eq!(
        receipt_a.outcome_kind,
        EdictEchoAttemptOutcomeKind::ObstructedAttempt
    );
    assert_ne!(
        receipt_a.outcome_kind,
        EdictEchoAttemptOutcomeKind::InvalidProposal
    );
    assert_ne!(
        receipt_a.outcome_kind,
        EdictEchoAttemptOutcomeKind::LegalUnselectedCounterfactual
    );

    let Some(obstruction) = receipt_a.obstruction.as_ref() else {
        return Err("stale basis produced no obstruction evidence".to_owned());
    };
    assert_eq!(
        obstruction.contract.kind,
        ContractObstructionKind::StaleBasis
    );
    assert_eq!(obstruction.reason.kind, "jim.EditObstruction.StaleBase");
    assert_eq!(
        obstruction.reason.payload.get("provided"),
        Some(&"input.basis".to_owned())
    );
    assert_eq!(receipt_a.input_basis_digest, digest(0x11));
    assert_eq!(receipt_a.observed_basis_digest, digest(0x22));
    Ok(())
}

#[test]
fn accepted_artifact_and_execution_receipt_are_separate_steps() -> Result<(), String> {
    let accepted = accept_edict_echo_target_ir(&supported_artifact())
        .map_err(|error| format!("supported fixture rejected: {error:?}"))?;

    assert_eq!(accepted.target_ir_digest(), digest(0xbb));
    assert_eq!(accepted.requirement_count(), 1);

    let receipt = execute_accepted_edict_echo_target_ir(&accepted, fresh_basis_input());
    assert_eq!(
        receipt.outcome_kind,
        EdictEchoAttemptOutcomeKind::CommittedSuccess
    );
    assert!(receipt.obstruction.is_none());
    assert_eq!(receipt.target_ir_digest, accepted.target_ir_digest());
    Ok(())
}

#[test]
fn unsupported_fixture_shapes_reject_with_stable_errors() -> Result<(), String> {
    let mut wrong_domain = supported_artifact();
    wrong_domain.domain = "gitwarp.commit-reducer-ir/v1".to_owned();
    assert_acceptance_error(
        &wrong_domain,
        EdictEchoTargetIrAcceptanceErrorKind::WrongDomain,
        None,
    )?;

    let mut missing_requirement = supported_artifact();
    missing_requirement.intents[0].requirements.clear();
    assert_acceptance_error(
        &missing_requirement,
        EdictEchoTargetIrAcceptanceErrorKind::MissingRequirement,
        None,
    )?;

    let mut extra_requirement = supported_artifact();
    let duplicate_requirement = extra_requirement.intents[0].requirements[0].clone();
    extra_requirement.intents[0]
        .requirements
        .push(duplicate_requirement);
    assert_acceptance_error(
        &extra_requirement,
        EdictEchoTargetIrAcceptanceErrorKind::UnsupportedRequirementCount,
        None,
    )?;

    let mut unsupported_disposition = supported_artifact();
    unsupported_disposition.intents[0].requirements[0].on_failure =
        EdictEchoTargetIrRequirementFailure::Terminal {
            reason: stale_reason(),
        };
    assert_acceptance_error(
        &unsupported_disposition,
        EdictEchoTargetIrAcceptanceErrorKind::UnsupportedRequirementDisposition,
        None,
    )?;

    let mut malformed_target_ir_digest = supported_artifact();
    malformed_target_ir_digest.target_ir_digest =
        "sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_owned();
    assert_acceptance_error(
        &malformed_target_ir_digest,
        EdictEchoTargetIrAcceptanceErrorKind::MalformedDigest,
        Some(EdictEchoTargetIrDigestField::TargetIrDigest),
    )?;

    let mut malformed_profile_digest = supported_artifact();
    malformed_profile_digest.target_profile_digest =
        "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_owned();
    assert_acceptance_error(
        &malformed_profile_digest,
        EdictEchoTargetIrAcceptanceErrorKind::MalformedDigest,
        Some(EdictEchoTargetIrDigestField::TargetProfileDigest),
    )?;
    Ok(())
}

fn assert_acceptance_error(
    artifact: &EdictEchoTargetIrArtifact,
    expected_kind: EdictEchoTargetIrAcceptanceErrorKind,
    expected_field: Option<EdictEchoTargetIrDigestField>,
) -> Result<(), String> {
    let Err(error) = accept_edict_echo_target_ir(artifact) else {
        return Err("unsupported fixture unexpectedly accepted".to_owned());
    };
    assert_eq!(error.kind, expected_kind);
    assert_eq!(error.digest_field, expected_field);
    assert_eq!(
        error.outcome_kind(),
        EdictEchoAttemptOutcomeKind::InvalidProposal
    );
    Ok(())
}

fn supported_artifact() -> EdictEchoTargetIrArtifact {
    EdictEchoTargetIrArtifact {
        domain: EDICT_ECHO_TARGET_IR_DOMAIN.to_owned(),
        target_profile_coordinate: "echo.dpo@1".to_owned(),
        target_profile_digest: TARGET_PROFILE_DIGEST.to_owned(),
        target_ir_digest: TARGET_IR_DIGEST.to_owned(),
        source_core_coordinate: "core://obstruction-strands/stale-basis".to_owned(),
        intents: vec![EdictEchoTargetIrIntent {
            name: "edit".to_owned(),
            requirements: vec![EdictEchoTargetIrRequirement {
                id: "edit.require.0".to_owned(),
                predicate: EdictEchoTargetIrPredicate::BasisFresh,
                on_failure: EdictEchoTargetIrRequirementFailure::ContinueObstructed {
                    reason: stale_reason(),
                },
            }],
        }],
    }
}

fn stale_reason() -> EdictEchoObstructionReason {
    EdictEchoObstructionReason {
        kind: "jim.EditObstruction.StaleBase".to_owned(),
        payload: std::iter::once(("provided".to_owned(), "input.basis".to_owned())).collect(),
    }
}

fn stale_basis_input() -> EdictEchoAttemptInput {
    EdictEchoAttemptInput {
        basis_is_fresh: false,
        input_basis_digest: digest(0x11),
        observed_basis_digest: digest(0x22),
    }
}

fn fresh_basis_input() -> EdictEchoAttemptInput {
    EdictEchoAttemptInput {
        basis_is_fresh: true,
        input_basis_digest: digest(0x33),
        observed_basis_digest: digest(0x33),
    }
}

fn digest(byte: u8) -> [u8; 32] {
    [byte; 32]
}
