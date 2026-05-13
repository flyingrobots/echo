// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for Echo-owned capability grant intent obstruction.

use warp_core::{
    AuthorityContext, AuthorityPolicy, AuthorityPolicyEvaluation, CapabilityGrantIntent,
    CapabilityGrantIntentGate, CapabilityGrantIntentObstruction,
    CapabilityGrantIntentObstructionReceipt, CapabilityGrantIntentOutcome,
    CapabilityGrantIntentPosture, PrincipalRef,
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
        policy_evaluation: AuthorityPolicyEvaluation::Unsupported,
    }
}

fn expected_obstructed_posture(
    intent: &CapabilityGrantIntent,
    authority_context: &AuthorityContext,
    obstruction: CapabilityGrantIntentObstruction,
) -> CapabilityGrantIntentOutcome {
    CapabilityGrantIntentOutcome::Obstructed(CapabilityGrantIntentPosture {
        kind: "capability-grant-intent-posture".to_owned(),
        intent_id: intent.intent_id.clone(),
        proposed_by: intent.proposed_by.clone(),
        subject: intent.subject.clone(),
        obstruction,
        receipt: expected_obstruction_receipt(intent, authority_context, obstruction),
    })
}

fn expected_obstruction_receipt(
    intent: &CapabilityGrantIntent,
    authority_context: &AuthorityContext,
    obstruction: CapabilityGrantIntentObstruction,
) -> CapabilityGrantIntentObstructionReceipt {
    let policy_id = authority_context
        .policy
        .as_ref()
        .map(|policy| policy.policy_id.clone());
    let policy_posture = format!("{:?}", authority_context.policy_evaluation);
    let receipt_input_bytes =
        expected_receipt_input_bytes(intent, obstruction, policy_id.as_deref(), &policy_posture);
    let receipt_digest = blake3::hash(&receipt_input_bytes).as_bytes().to_vec();

    CapabilityGrantIntentObstructionReceipt {
        kind: "capability-grant-intent-obstruction-receipt".to_owned(),
        intent_id: intent.intent_id.clone(),
        proposed_by: intent.proposed_by.clone(),
        subject: intent.subject.clone(),
        obstruction,
        policy_id,
        policy_posture,
        receipt_input_bytes,
        receipt_digest,
    }
}

fn expected_receipt_input_bytes(
    intent: &CapabilityGrantIntent,
    obstruction: CapabilityGrantIntentObstruction,
    policy_id: Option<&str>,
    policy_posture: &str,
) -> Vec<u8> {
    let obstruction = format!("{obstruction:?}");
    let mut input = Vec::new();
    append_expected_receipt_field(
        &mut input,
        "domain",
        b"capability-grant-intent-obstruction-receipt/v0",
    );
    append_expected_receipt_field(&mut input, "intent_id", intent.intent_id.as_bytes());
    append_expected_receipt_field(&mut input, "proposed_by", intent.proposed_by.id.as_bytes());
    append_expected_receipt_field(&mut input, "subject", intent.subject.id.as_bytes());
    append_expected_receipt_field(&mut input, "obstruction", obstruction.as_bytes());
    append_expected_receipt_field(&mut input, "policy_id", policy_id.unwrap_or("").as_bytes());
    append_expected_receipt_field(&mut input, "policy_posture", policy_posture.as_bytes());
    input
}

fn append_expected_receipt_field(input: &mut Vec<u8>, field_name: &str, value: &[u8]) {
    input.extend_from_slice(field_name.as_bytes());
    input.push(0);
    input.extend_from_slice(&(value.len() as u64).to_be_bytes());
    input.extend_from_slice(value);
}

fn obstruction_for(outcome: &CapabilityGrantIntentOutcome) -> CapabilityGrantIntentObstruction {
    match outcome {
        CapabilityGrantIntentOutcome::Obstructed(posture) => posture.obstruction,
    }
}

#[test]
fn capability_grant_intent_obstruction_receipt_echoes_refusal_context() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intent = fixture_intent("intent:obstruction-receipt");
    let authority_context = fixture_authority_context();

    let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

    assert_eq!(
        outcome,
        CapabilityGrantIntentOutcome::Obstructed(CapabilityGrantIntentPosture {
            kind: "capability-grant-intent-posture".to_owned(),
            intent_id: intent.intent_id.clone(),
            proposed_by: intent.proposed_by.clone(),
            subject: intent.subject.clone(),
            obstruction: CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy,
            receipt: expected_obstruction_receipt(
                &intent,
                &authority_context,
                CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy,
            ),
        })
    );
}

#[test]
fn capability_grant_intent_obstruction_receipt_is_deterministic() {
    let intent = fixture_intent("intent:deterministic-obstruction-receipt");
    let authority_context = fixture_authority_context();
    let mut first_registry = CapabilityGrantIntentGate::new();
    let mut second_registry = CapabilityGrantIntentGate::new();

    let first = first_registry.submit_grant_intent(intent.clone(), authority_context.clone());
    let second = second_registry.submit_grant_intent(intent, authority_context);

    assert_eq!(first, second);
}

#[test]
fn capability_grant_intent_obstructs_malformed_grant_intent() {
    let mut registry = CapabilityGrantIntentGate::new();
    let mut intent = fixture_intent("intent:malformed");
    intent.artifact_hash.clear();

    let authority_context = fixture_authority_context();
    let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            &authority_context,
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
        let authority_context = fixture_authority_context();
        let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

        assert_eq!(
            outcome,
            expected_obstructed_posture(
                &intent,
                &authority_context,
                CapabilityGrantIntentObstruction::MalformedGrantIntent
            )
        );
    }
}

#[test]
fn capability_grant_intent_obstructs_replay_or_duplicate_grant_intent() {
    let mut registry = CapabilityGrantIntentGate::new();
    let first_intent = fixture_intent("intent:replay");
    let replay_intent = fixture_intent("intent:replay");

    let first_outcome = registry.submit_grant_intent(first_intent, fixture_authority_context());
    let replay_outcome =
        registry.submit_grant_intent(replay_intent.clone(), fixture_authority_context());

    assert_eq!(
        obstruction_for(&first_outcome),
        CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
    );
    assert_eq!(
        replay_outcome,
        expected_obstructed_posture(
            &replay_intent,
            &fixture_authority_context(),
            CapabilityGrantIntentObstruction::ReplayOrDuplicateIntent
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
        policy_evaluation: AuthorityPolicyEvaluation::Unsupported,
    };

    let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            &authority_context,
            CapabilityGrantIntentObstruction::MissingIssuerAuthority
        )
    );
}

#[test]
fn capability_grant_intent_obstructs_invalid_delegation() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intent = fixture_intent("intent:invalid-delegation");
    let authority_context = AuthorityContext {
        issuer: Some(principal("principal:issuer")),
        policy: Some(AuthorityPolicy {
            policy_id: "authority-policy:fixture".to_owned(),
        }),
        policy_evaluation: AuthorityPolicyEvaluation::InvalidDelegation,
    };

    let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            &authority_context,
            CapabilityGrantIntentObstruction::InvalidDelegation
        )
    );
}

#[test]
fn capability_grant_intent_obstructs_scope_escalation() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intent = fixture_intent("intent:scope-escalation");
    let authority_context = AuthorityContext {
        issuer: Some(principal("principal:issuer")),
        policy: Some(AuthorityPolicy {
            policy_id: "authority-policy:fixture".to_owned(),
        }),
        policy_evaluation: AuthorityPolicyEvaluation::ScopeEscalation,
    };

    let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            &authority_context,
            CapabilityGrantIntentObstruction::ScopeEscalation
        )
    );
}

#[test]
fn capability_grant_intent_obstructs_missing_policy_identity_as_unsupported_policy() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intent = fixture_intent("intent:missing-policy-identity");
    let authority_context = AuthorityContext {
        issuer: Some(principal("principal:issuer")),
        policy: Some(AuthorityPolicy {
            policy_id: String::new(),
        }),
        policy_evaluation: AuthorityPolicyEvaluation::InvalidDelegation,
    };

    let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            &authority_context,
            CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
        )
    );
}

#[test]
fn capability_grant_intent_obstructs_unsupported_authority_policy() {
    let mut registry = CapabilityGrantIntentGate::new();
    let intent = fixture_intent("intent:unsupported-policy");

    let authority_context = fixture_authority_context();
    let outcome = registry.submit_grant_intent(intent.clone(), authority_context.clone());

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &intent,
            &authority_context,
            CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
        )
    );
}

#[test]
fn capability_grant_intent_never_makes_grant_authority() {
    let mut malformed = fixture_intent("intent:malformed-empty-rights");
    malformed.rights.clear();

    let mut malformed_registry = CapabilityGrantIntentGate::new();
    let mut missing_issuer_registry = CapabilityGrantIntentGate::new();
    let mut invalid_delegation_registry = CapabilityGrantIntentGate::new();
    let mut scope_escalation_registry = CapabilityGrantIntentGate::new();
    let mut replay_registry = CapabilityGrantIntentGate::new();
    let mut unsupported_registry = CapabilityGrantIntentGate::new();

    let missing_issuer = fixture_intent("intent:missing-issuer");
    let invalid_delegation = fixture_intent("intent:invalid-delegation-never-authority");
    let scope_escalation = fixture_intent("intent:scope-escalation-never-authority");
    let replay = fixture_intent("intent:replay-never-authority");
    let unsupported = fixture_intent("intent:unsupported-policy");

    let replay_first_outcome =
        replay_registry.submit_grant_intent(replay.clone(), fixture_authority_context());
    let outcomes = [
        malformed_registry.submit_grant_intent(malformed, fixture_authority_context()),
        missing_issuer_registry.submit_grant_intent(
            missing_issuer,
            AuthorityContext {
                issuer: None,
                policy: Some(AuthorityPolicy {
                    policy_id: "authority-policy:fixture".to_owned(),
                }),
                policy_evaluation: AuthorityPolicyEvaluation::Unsupported,
            },
        ),
        invalid_delegation_registry.submit_grant_intent(
            invalid_delegation,
            AuthorityContext {
                issuer: Some(principal("principal:issuer")),
                policy: Some(AuthorityPolicy {
                    policy_id: "authority-policy:fixture".to_owned(),
                }),
                policy_evaluation: AuthorityPolicyEvaluation::InvalidDelegation,
            },
        ),
        scope_escalation_registry.submit_grant_intent(
            scope_escalation,
            AuthorityContext {
                issuer: Some(principal("principal:issuer")),
                policy: Some(AuthorityPolicy {
                    policy_id: "authority-policy:fixture".to_owned(),
                }),
                policy_evaluation: AuthorityPolicyEvaluation::ScopeEscalation,
            },
        ),
        replay_registry.submit_grant_intent(replay, fixture_authority_context()),
        unsupported_registry.submit_grant_intent(unsupported, fixture_authority_context()),
    ];

    assert_eq!(
        obstruction_for(&replay_first_outcome),
        CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
    );
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
        CapabilityGrantIntentObstruction::InvalidDelegation
    );
    assert_eq!(
        obstruction_for(&outcomes[3]),
        CapabilityGrantIntentObstruction::ScopeEscalation
    );
    assert_eq!(
        obstruction_for(&outcomes[4]),
        CapabilityGrantIntentObstruction::ReplayOrDuplicateIntent
    );
    assert_eq!(
        obstruction_for(&outcomes[5]),
        CapabilityGrantIntentObstruction::UnsupportedAuthorityPolicy
    );
}
