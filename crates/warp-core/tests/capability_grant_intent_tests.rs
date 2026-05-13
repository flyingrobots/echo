// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for Echo-owned capability grant intent obstruction.

use warp_core::{
    AuthorityContext, AuthorityPolicy, CapabilityGrantIntent, CapabilityGrantIntentGate,
    CapabilityGrantIntentObstruction, CapabilityGrantIntentOutcome, CapabilityGrantIntentPosture,
    PrincipalRef,
};

fn principal(id: &str) -> PrincipalRef {
    PrincipalRef { id: id.to_owned() }
}

fn fixture_intent(intent_id: &str) -> CapabilityGrantIntent {
    CapabilityGrantIntent {
        intent_id: intent_id.to_owned(),
        proposed_by: principal("principal:issuer"),
        subject: principal("principal:jedit-session"),
        artifact_hash: "artifact-hash:stack-witness-0001".to_owned(),
        operation_id: "operation:textWindow:v0".to_owned(),
        requirements_digest: "requirements-digest:stack-witness-0001".to_owned(),
        rights: vec!["optic.invoke".to_owned()],
        scope_bytes: b"scope:fixture".to_vec(),
        expiry_bytes: Some(b"expiry:fixture".to_vec()),
        delegation_basis_bytes: Some(b"delegation-basis:fixture".to_vec()),
    }
}

fn fixture_authority_context() -> AuthorityContext {
    AuthorityContext {
        issuer: Some(principal("principal:issuer")),
        policy: Some(AuthorityPolicy {
            policy_id: "authority-policy:fixture".to_owned(),
        }),
    }
}

fn expected_obstructed_posture(
    intent: &CapabilityGrantIntent,
    obstruction: CapabilityGrantIntentObstruction,
) -> CapabilityGrantIntentOutcome {
    CapabilityGrantIntentOutcome::Obstructed(CapabilityGrantIntentPosture {
        kind: "capability-grant-intent-posture".to_owned(),
        intent_id: intent.intent_id.clone(),
        proposed_by: intent.proposed_by.clone(),
        subject: intent.subject.clone(),
        obstruction,
    })
}

fn obstruction_for(outcome: &CapabilityGrantIntentOutcome) -> CapabilityGrantIntentObstruction {
    match outcome {
        CapabilityGrantIntentOutcome::Obstructed(posture) => posture.obstruction,
    }
}

#[test]
fn capability_grant_intent_obstructs_malformed_grant_intent() {
    let mut registry = CapabilityGrantIntentGate::new();
    let mut intent = fixture_intent("intent:malformed");
    intent.artifact_hash.clear();

    let outcome = registry.submit_grant_intent(intent.clone(), fixture_authority_context());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            CapabilityGrantIntentObstruction::MalformedGrantIntent
        )
    );
}

#[test]
fn capability_grant_intent_obstructs_missing_required_identity_as_malformed() {
    let malformed_intents = {
        let mut missing_subject = fixture_intent("intent:missing-subject");
        missing_subject.subject.id.clear();

        let mut missing_artifact = fixture_intent("intent:missing-artifact");
        missing_artifact.artifact_hash.clear();

        let mut missing_operation = fixture_intent("intent:missing-operation");
        missing_operation.operation_id.clear();

        let mut missing_requirements = fixture_intent("intent:missing-requirements");
        missing_requirements.requirements_digest.clear();

        [
            missing_subject,
            missing_artifact,
            missing_operation,
            missing_requirements,
        ]
    };

    for intent in malformed_intents {
        let mut registry = CapabilityGrantIntentGate::new();
        let outcome = registry.submit_grant_intent(intent.clone(), fixture_authority_context());

        assert_eq!(
            outcome,
            expected_obstructed_posture(
                &intent,
                CapabilityGrantIntentObstruction::MalformedGrantIntent
            )
        );
    }
}

#[test]
fn capability_grant_intent_obstructs_duplicate_grant_intent() {
    let mut registry = CapabilityGrantIntentGate::new();
    let first_intent = fixture_intent("intent:duplicate");
    let duplicate_intent = fixture_intent("intent:duplicate");

    let first_outcome = registry.submit_grant_intent(first_intent, fixture_authority_context());
    let duplicate_outcome =
        registry.submit_grant_intent(duplicate_intent.clone(), fixture_authority_context());

    assert_eq!(
        obstruction_for(&first_outcome),
        CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
    );
    assert_eq!(
        duplicate_outcome,
        expected_obstructed_posture(
            &duplicate_intent,
            CapabilityGrantIntentObstruction::DuplicateGrantIntent
        )
    );
}

#[test]
fn capability_grant_intent_obstructs_missing_issuer_authority() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intent = fixture_intent("intent:missing-issuer");
    let authority_context = AuthorityContext {
        issuer: None,
        policy: Some(AuthorityPolicy {
            policy_id: "authority-policy:fixture".to_owned(),
        }),
    };

    let outcome = registry.submit_grant_intent(intent.clone(), authority_context);

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            CapabilityGrantIntentObstruction::MissingIssuerAuthority
        )
    );
}

#[test]
fn capability_grant_intent_obstructs_unsupported_authority_policy() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intent = fixture_intent("intent:unsupported-policy");

    let outcome = registry.submit_grant_intent(intent.clone(), fixture_authority_context());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
        )
    );
}

#[test]
fn capability_grant_intent_never_makes_grant_authority() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intents = [
        fixture_intent("intent:malformed-empty-rights"),
        fixture_intent("intent:missing-issuer"),
        fixture_intent("intent:unsupported-policy"),
    ];

    let mut malformed = intents[0].clone();
    malformed.rights.clear();
    let outcomes = [
        registry.submit_grant_intent(malformed, fixture_authority_context()),
        registry.submit_grant_intent(
            intents[1].clone(),
            AuthorityContext {
                issuer: None,
                policy: Some(AuthorityPolicy {
                    policy_id: "authority-policy:fixture".to_owned(),
                }),
            },
        ),
        registry.submit_grant_intent(intents[2].clone(), fixture_authority_context()),
    ];

    assert_eq!(
        obstruction_for(&outcomes[0]),
        CapabilityGrantIntentObstruction::MalformedGrantIntent
    );
    assert_eq!(
        obstruction_for(&outcomes[1]),
        CapabilityGrantIntentObstruction::MissingIssuerAuthority
    );
    assert_eq!(
        obstruction_for(&outcomes[2]),
        CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
    );
}
