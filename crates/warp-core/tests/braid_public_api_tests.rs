// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! External-consumer braid public API checks.

use warp_core::{
    make_strand_id, AuthorityDomainId, AuthorityDomainRef, Braid, BraidError, BraidEvent,
    BraidMemberRef, BraidStatus, BraidTransitionKind, OriginId, ProofEnvelope, ProofError,
    ProofKind,
};

fn authority_ref() -> AuthorityDomainRef {
    AuthorityDomainRef::new(
        OriginId::from_bytes([0x10; 32]),
        AuthorityDomainId::from_bytes([0x20; 32]),
    )
}

#[test]
fn crate_root_exports_braid_lifecycle_error_and_member_types() -> Result<(), BraidError> {
    let member_ref = BraidMemberRef::Revealed(make_strand_id("public-member"));
    let mut braid = Braid::new([0xAB; 32], authority_ref());

    assert_eq!(braid.status(), BraidStatus::Active);
    braid.apply(BraidEvent::MemberWoven {
        member_ref,
        sequence_num: 0,
    })?;
    assert_eq!(braid.frontier(), &[member_ref]);

    assert_eq!(
        braid.apply(BraidEvent::MemberWoven {
            member_ref,
            sequence_num: 1,
        }),
        Err(BraidError::DuplicateMember { member_ref })
    );
    Ok(())
}

#[test]
fn public_braid_transition_failures_are_typed() {
    let member_ref = BraidMemberRef::Revealed(make_strand_id("typed-transition-member"));
    let mut braid = Braid::new([0xAC; 32], authority_ref());

    braid
        .apply(BraidEvent::MemberWoven {
            member_ref,
            sequence_num: 0,
        })
        .expect("first member weave");
    braid
        .apply(BraidEvent::SettlementFinalized {
            settlement_digest: [0x5E; 32],
        })
        .expect("settlement finalization");

    assert_eq!(
        braid.apply(BraidEvent::MemberWoven {
            member_ref: BraidMemberRef::Revealed(make_strand_id("late-member")),
            sequence_num: 1,
        }),
        Err(BraidError::InvalidTransition {
            transition: BraidTransitionKind::WeaveMember,
            status: BraidStatus::Finalized,
        })
    );
}

#[test]
fn public_proof_validation_failures_are_typed() {
    let expected = [0x42; 32];
    let actual = [0x24; 32];
    let proof = ProofEnvelope {
        kind: ProofKind::ReplayTrace,
        proof_bytes: vec![1, 2, 3],
        public_inputs_hash: actual,
    };

    assert_eq!(
        proof.validate_shape(expected),
        Err(ProofError::PublicInputsMismatch { expected, actual })
    );

    let unsupported = ProofEnvelope {
        kind: ProofKind::ZkSnark,
        proof_bytes: vec![1, 2, 3],
        public_inputs_hash: expected,
    };
    assert_eq!(
        unsupported.validate_shape(expected),
        Err(ProofError::UnsupportedKind {
            kind: ProofKind::ZkSnark,
        })
    );
}
