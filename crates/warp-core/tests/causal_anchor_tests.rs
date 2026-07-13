// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Causal anchor public API tests.

use warp_core::{
    CausalAnchorAppRootRole, CausalAnchorCasRole, CausalAnchorError, CausalAnchorFact,
    CausalAnchorGraphRole, CausalAnchorPurpose, CausalAnchorRequest, CausalAnchorRoot,
    CausalAnchorSubject, CausalFrontierRef, CAUSAL_ANCHOR_SCHEMA_VERSION,
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
) -> CausalAnchorRequest {
    CausalAnchorRequest {
        schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION,
        subject: subject(),
        basis_frontier: frontier(1),
        retained_roots,
        materialization_roots,
        purpose: CausalAnchorPurpose::UserSave,
        admitted_by_receipt_id: hash(9),
    }
}

fn graph_index_root(seed: u8) -> CausalAnchorRoot {
    CausalAnchorRoot::GraphFact {
        id: hash(seed),
        role: CausalAnchorGraphRole::Index,
    }
}

#[test]
fn causal_anchor_digest_is_order_insensitive_for_root_sets() -> Result<(), CausalAnchorError> {
    let retained_a = app_authority_root("head:42");
    let retained_b = graph_evidence_root(2);
    let materialized_a = materialization_root(3);
    let materialized_b = materialization_root(4);

    let first = CausalAnchorFact::from_request(request_with_roots(
        vec![retained_a.clone(), retained_b.clone()],
        vec![materialized_a.clone(), materialized_b.clone()],
    ))?;
    let second = CausalAnchorFact::from_request(request_with_roots(
        vec![retained_b, retained_a],
        vec![materialized_b, materialized_a],
    ))?;

    assert_eq!(first.anchor_digest, second.anchor_digest);
    assert_eq!(first.anchor_id, second.anchor_id);
    assert_eq!(first.retained_roots, second.retained_roots);
    assert_eq!(first.materialization_roots, second.materialization_roots);
    Ok(())
}

#[test]
fn causal_anchor_rejects_empty_retained_roots() {
    assert!(matches!(
        CausalAnchorFact::from_request(request_with_roots(
            Vec::new(),
            vec![materialization_root(1)],
        )),
        Err(CausalAnchorError::EmptyRetainedRoots)
    ));
}

#[test]
fn causal_anchor_rejects_authority_materialization_roots() {
    assert!(matches!(
        CausalAnchorFact::from_request(request_with_roots(
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
        CausalAnchorFact::from_request(request_with_roots(
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
        CausalAnchorFact::from_request(request_with_roots(
            vec![ambiguous.clone()],
            vec![ambiguous],
        )),
        Err(CausalAnchorError::RootAppearsInRetainedAndMaterialization)
    ));
}

#[test]
fn causal_anchor_digest_binds_subject_frontier_purpose_and_receipt() -> Result<(), CausalAnchorError>
{
    let base = CausalAnchorFact::from_request(request_with_roots(
        vec![app_authority_root("head:42")],
        vec![materialization_root(3)],
    ))?;

    let different_subject = CausalAnchorFact::from_request(CausalAnchorRequest {
        subject: CausalAnchorSubject::new("mail", "Thread", "thread:42"),
        ..request_with_roots(
            vec![app_authority_root("head:42")],
            vec![materialization_root(3)],
        )
    })?;
    let different_frontier = CausalAnchorFact::from_request(CausalAnchorRequest {
        basis_frontier: frontier(8),
        ..request_with_roots(
            vec![app_authority_root("head:42")],
            vec![materialization_root(3)],
        )
    })?;
    let different_purpose = CausalAnchorFact::from_request(CausalAnchorRequest {
        purpose: CausalAnchorPurpose::Export,
        ..request_with_roots(
            vec![app_authority_root("head:42")],
            vec![materialization_root(3)],
        )
    })?;
    let different_receipt = CausalAnchorFact::from_request(CausalAnchorRequest {
        admitted_by_receipt_id: hash(7),
        ..request_with_roots(
            vec![app_authority_root("head:42")],
            vec![materialization_root(3)],
        )
    })?;

    assert_ne!(base.anchor_digest, different_subject.anchor_digest);
    assert_ne!(base.anchor_digest, different_frontier.anchor_digest);
    assert_ne!(base.anchor_digest, different_purpose.anchor_digest);
    assert_ne!(base.anchor_digest, different_receipt.anchor_digest);
    Ok(())
}

#[test]
fn causal_anchor_digest_binds_schema_version() -> Result<(), CausalAnchorError> {
    let base = CausalAnchorFact::from_request(request_with_roots(
        vec![app_authority_root("head:42")],
        Vec::new(),
    ))?;
    let next_schema = CausalAnchorFact::from_request(CausalAnchorRequest {
        schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION + 1,
        ..request_with_roots(vec![app_authority_root("head:42")], Vec::new())
    })?;

    assert_eq!(base.schema_version, CAUSAL_ANCHOR_SCHEMA_VERSION);
    assert_eq!(next_schema.schema_version, CAUSAL_ANCHOR_SCHEMA_VERSION + 1);
    assert_ne!(base.anchor_digest, next_schema.anchor_digest);
    assert_ne!(base.anchor_id, next_schema.anchor_id);
    Ok(())
}

#[test]
fn jim_rope_checkpoint_anchor_retains_head_as_authority_not_projection(
) -> Result<(), CausalAnchorError> {
    let anchor = CausalAnchorFact::from_request(request_with_roots(
        vec![app_authority_root("rope-head:42")],
        vec![materialization_root(5)],
    ))?;

    assert_eq!(anchor.subject, subject());
    assert_eq!(anchor.purpose, CausalAnchorPurpose::UserSave);
    assert_eq!(
        anchor.retained_roots,
        vec![app_authority_root("rope-head:42")]
    );
    assert_eq!(anchor.materialization_roots, vec![materialization_root(5)]);
    assert!(anchor.retained_roots[0].is_authority());
    assert!(!anchor.materialization_roots[0].is_authority());
    Ok(())
}

#[test]
fn causal_anchor_id_is_domain_separated_from_anchor_digest() -> Result<(), CausalAnchorError> {
    let anchor = CausalAnchorFact::from_request(request_with_roots(
        vec![app_authority_root("head:42")],
        Vec::new(),
    ))?;

    assert_ne!(anchor.anchor_id.as_bytes(), &anchor.anchor_digest);
    Ok(())
}
