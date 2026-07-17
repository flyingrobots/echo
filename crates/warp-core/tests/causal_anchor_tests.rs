// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Causal anchor public API tests.

use warp_core::{
    CausalAnchorAdmissionRequest, CausalAnchorAppRootRole, CausalAnchorCasRole, CausalAnchorClaim,
    CausalAnchorError, CausalAnchorGraphRole, CausalAnchorPurpose, CausalAnchorRoot,
    CausalAnchorRootSupportGrant, CausalAnchorRootSupportPolicy, CausalAnchorSubject,
    CausalAnchorSupportError, CausalAnchorSupportSet, CausalFrontierRef,
    CAUSAL_ANCHOR_SCHEMA_VERSION,
};

fn hash(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn subject() -> CausalAnchorSubject {
    CausalAnchorSubject::new("jedit", "BufferWorldline", "worldline:main")
}

fn frontier(seed: u8) -> CausalFrontierRef {
    CausalFrontierRef::from_digest(hash(seed))
}

fn app_authority_root(id: &str) -> CausalAnchorRoot {
    CausalAnchorRoot::AppSubjectRoot {
        app_id: "jedit".to_owned(),
        subject_kind: "RopeHead".to_owned(),
        id: id.to_owned(),
        role: CausalAnchorAppRootRole::Authority,
    }
}

fn graph_evidence_root(seed: u8) -> CausalAnchorRoot {
    CausalAnchorRoot::GraphFact {
        id: hash(seed),
        role: CausalAnchorGraphRole::Evidence,
    }
}

fn materialization_root(seed: u8) -> CausalAnchorRoot {
    CausalAnchorRoot::CasObject {
        id: hash(seed),
        role: CausalAnchorCasRole::Materialization,
    }
}

fn request_with_roots(
    retained_roots: Vec<CausalAnchorRoot>,
    materialization_roots: Vec<CausalAnchorRoot>,
) -> CausalAnchorAdmissionRequest {
    CausalAnchorAdmissionRequest {
        schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION,
        subject: subject(),
        basis_frontier: frontier(1),
        retained_roots,
        materialization_roots,
        purpose: CausalAnchorPurpose::UserSave,
    }
}

fn graph_index_root(seed: u8) -> CausalAnchorRoot {
    CausalAnchorRoot::GraphFact {
        id: hash(seed),
        role: CausalAnchorGraphRole::Index,
    }
}

#[test]
fn application_request_builds_a_claim_without_conferring_echo_admission(
) -> Result<(), CausalAnchorError> {
    let claim = CausalAnchorClaim::from_admission_request(CausalAnchorAdmissionRequest {
        schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION,
        subject: subject(),
        basis_frontier: frontier(1),
        retained_roots: vec![app_authority_root("head:42")],
        materialization_roots: vec![materialization_root(3)],
        purpose: CausalAnchorPurpose::UserSave,
    })?;

    assert_eq!(claim.subject(), &subject());
    assert_eq!(claim.basis_frontier(), &frontier(1));
    assert_eq!(claim.retained_roots(), [app_authority_root("head:42")]);
    assert_eq!(claim.materialization_roots(), [materialization_root(3)]);
    assert_eq!(claim.purpose(), CausalAnchorPurpose::UserSave);
    Ok(())
}

#[test]
fn causal_anchor_claim_digest_is_order_insensitive_for_root_sets() -> Result<(), CausalAnchorError>
{
    let retained_a = app_authority_root("head:42");
    let retained_b = graph_evidence_root(2);
    let materialized_a = materialization_root(3);
    let materialized_b = materialization_root(4);

    let first = CausalAnchorClaim::from_admission_request(request_with_roots(
        vec![retained_a.clone(), retained_b.clone()],
        vec![materialized_a.clone(), materialized_b.clone()],
    ))?;
    let second = CausalAnchorClaim::from_admission_request(request_with_roots(
        vec![retained_b, retained_a],
        vec![materialized_b, materialized_a],
    ))?;

    assert_eq!(first.claim_digest(), second.claim_digest());
    assert_eq!(first.retained_roots(), second.retained_roots());
    assert_eq!(
        first.materialization_roots(),
        second.materialization_roots()
    );
    Ok(())
}

#[test]
fn causal_anchor_rejects_empty_retained_roots() {
    assert!(matches!(
        CausalAnchorClaim::from_admission_request(request_with_roots(
            Vec::new(),
            vec![materialization_root(1)],
        )),
        Err(CausalAnchorError::EmptyRetainedRoots)
    ));
}

#[test]
fn causal_anchor_rejects_empty_application_root_fields() {
    assert!(matches!(
        CausalAnchorClaim::from_admission_request(request_with_roots(
            vec![CausalAnchorRoot::AppSubjectRoot {
                app_id: "jedit".to_owned(),
                subject_kind: "RopeHead".to_owned(),
                id: String::new(),
                role: CausalAnchorAppRootRole::Authority,
            }],
            Vec::new(),
        )),
        Err(CausalAnchorError::EmptyRootField {
            root_kind: "app-subject",
            field: "id",
        })
    ));
}

#[test]
fn causal_anchor_support_policy_is_canonical_and_exact() -> Result<(), CausalAnchorError> {
    let claim = CausalAnchorClaim::from_admission_request(request_with_roots(
        vec![app_authority_root("head:policy")],
        vec![materialization_root(9)],
    ))?;
    let retained = CausalAnchorRootSupportGrant::retained(
        claim.subject().clone(),
        claim.retained_roots()[0].clone(),
    );
    let materialization = CausalAnchorRootSupportGrant::materialization(
        claim.subject().clone(),
        claim.materialization_roots()[0].clone(),
    );
    let materialization_grant_digest = materialization.grant_digest();
    let first = CausalAnchorRootSupportPolicy::new([retained.clone(), materialization.clone()]);
    let reversed = CausalAnchorRootSupportPolicy::new([materialization, retained.clone()]);

    assert_eq!(first.policy_digest(), reversed.policy_digest());
    assert_eq!(first.validate_claim(&claim), Ok(()));
    assert_eq!(
        CausalAnchorRootSupportPolicy::new([retained]).validate_claim(&claim),
        Err(CausalAnchorSupportError::UnsupportedRoot {
            grant_digest: materialization_grant_digest,
            support_set: CausalAnchorSupportSet::Materialization,
        })
    );
    Ok(())
}

#[test]
fn causal_anchor_rejects_authority_materialization_roots() {
    assert!(matches!(
        CausalAnchorClaim::from_admission_request(request_with_roots(
            vec![app_authority_root("head:42")],
            vec![app_authority_root("head:42-flat-text")],
        )),
        Err(CausalAnchorError::AuthorityMaterializationRoot)
    ));
}

#[test]
fn causal_anchor_rejects_duplicate_roots_after_canonicalization() {
    let duplicated = app_authority_root("head:42");
    assert!(matches!(
        CausalAnchorClaim::from_admission_request(request_with_roots(
            vec![duplicated.clone(), duplicated],
            Vec::new(),
        )),
        Err(CausalAnchorError::DuplicateRetainedRoot)
    ));
}

#[test]
fn causal_anchor_rejects_roots_that_are_both_retained_and_materialized() {
    let ambiguous = graph_index_root(42);
    assert!(matches!(
        CausalAnchorClaim::from_admission_request(request_with_roots(
            vec![ambiguous.clone()],
            vec![ambiguous],
        )),
        Err(CausalAnchorError::RootAppearsInRetainedAndMaterialization)
    ));
}

#[test]
fn causal_anchor_claim_digest_binds_subject_frontier_and_purpose() -> Result<(), CausalAnchorError>
{
    let base = CausalAnchorClaim::from_admission_request(request_with_roots(
        vec![app_authority_root("head:42")],
        vec![materialization_root(3)],
    ))?;

    let different_subject =
        CausalAnchorClaim::from_admission_request(CausalAnchorAdmissionRequest {
            subject: CausalAnchorSubject::new("mail", "Thread", "thread:42"),
            ..request_with_roots(
                vec![app_authority_root("head:42")],
                vec![materialization_root(3)],
            )
        })?;
    let different_frontier =
        CausalAnchorClaim::from_admission_request(CausalAnchorAdmissionRequest {
            basis_frontier: frontier(8),
            ..request_with_roots(
                vec![app_authority_root("head:42")],
                vec![materialization_root(3)],
            )
        })?;
    let different_purpose =
        CausalAnchorClaim::from_admission_request(CausalAnchorAdmissionRequest {
            purpose: CausalAnchorPurpose::Export,
            ..request_with_roots(
                vec![app_authority_root("head:42")],
                vec![materialization_root(3)],
            )
        })?;
    assert_ne!(base.claim_digest(), different_subject.claim_digest());
    assert_ne!(base.claim_digest(), different_frontier.claim_digest());
    assert_ne!(base.claim_digest(), different_purpose.claim_digest());
    Ok(())
}

#[test]
fn causal_anchor_rejects_unsupported_schema_version() {
    let result = CausalAnchorClaim::from_admission_request(CausalAnchorAdmissionRequest {
        schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION + 1,
        ..request_with_roots(vec![app_authority_root("head:42")], Vec::new())
    });

    assert_eq!(
        result,
        Err(CausalAnchorError::UnsupportedSchemaVersion {
            expected: CAUSAL_ANCHOR_SCHEMA_VERSION,
            actual: CAUSAL_ANCHOR_SCHEMA_VERSION + 1,
        })
    );
}

#[test]
fn jim_rope_checkpoint_anchor_retains_head_as_authority_not_projection(
) -> Result<(), CausalAnchorError> {
    let claim = CausalAnchorClaim::from_admission_request(request_with_roots(
        vec![app_authority_root("rope-head:42")],
        vec![materialization_root(5)],
    ))?;

    assert_eq!(claim.subject(), &subject());
    assert_eq!(claim.purpose(), CausalAnchorPurpose::UserSave);
    assert_eq!(claim.retained_roots(), [app_authority_root("rope-head:42")]);
    assert_eq!(claim.materialization_roots(), [materialization_root(5)]);
    assert!(claim.retained_roots()[0].is_authority());
    assert!(!claim.materialization_roots()[0].is_authority());
    Ok(())
}
