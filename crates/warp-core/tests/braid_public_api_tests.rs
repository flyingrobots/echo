// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! External-consumer braid public API checks.

use warp_core::{
    make_strand_id, AuthorityDomainId, AuthorityDomainRef, Braid, BraidError, BraidEvent,
    BraidMemberRef, BraidMembershipCursor, BraidMembershipDiff, BraidMembershipEntry, BraidStatus,
    BraidTransitionKind, OriginId, ProofEnvelope, ProofError, ProofKind,
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
fn public_braid_transition_failures_are_typed() -> Result<(), BraidError> {
    let member_ref = BraidMemberRef::Revealed(make_strand_id("typed-transition-member"));
    let mut braid = Braid::new([0xAC; 32], authority_ref());

    braid.apply(BraidEvent::MemberWoven {
        member_ref,
        sequence_num: 0,
    })?;
    braid.apply(BraidEvent::SettlementFinalized {
        settlement_digest: [0x5E; 32],
    })?;

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
    Ok(())
}

#[test]
fn public_braid_transition_display_is_human_facing() {
    let err = BraidError::InvalidTransition {
        transition: BraidTransitionKind::WeaveMember,
        status: BraidStatus::Finalized,
    };

    assert_eq!(
        err.to_string(),
        "cannot transition braid state: cannot weave member in status Finalized"
    );
}

#[test]
fn public_braid_membership_history_is_append_only_event_history() -> Result<(), BraidError> {
    let first = BraidMemberRef::Revealed(make_strand_id("history-member-a"));
    let second = BraidMemberRef::Revealed(make_strand_id("history-member-b"));
    let late = BraidMemberRef::Revealed(make_strand_id("history-member-late"));
    let mut braid = Braid::new([0xAD; 32], authority_ref());

    braid.apply(BraidEvent::MemberWoven {
        member_ref: first,
        sequence_num: 0,
    })?;
    assert_eq!(
        braid.apply(BraidEvent::MemberWoven {
            member_ref: first,
            sequence_num: 1,
        }),
        Err(BraidError::DuplicateMember { member_ref: first })
    );
    braid.apply(BraidEvent::MemberWoven {
        member_ref: second,
        sequence_num: 1,
    })?;
    braid.apply(BraidEvent::SettlementFinalized {
        settlement_digest: [0x5E; 32],
    })?;
    assert_eq!(
        braid.apply(BraidEvent::MemberWoven {
            member_ref: late,
            sequence_num: 2,
        }),
        Err(BraidError::InvalidTransition {
            transition: BraidTransitionKind::WeaveMember,
            status: BraidStatus::Finalized,
        })
    );

    assert_eq!(
        braid.membership_history(),
        vec![
            BraidMembershipEntry {
                member_ref: first,
                sequence_num: 0,
            },
            BraidMembershipEntry {
                member_ref: second,
                sequence_num: 1,
            },
        ]
    );
    assert_eq!(braid.frontier(), &[first, second]);
    Ok(())
}

#[test]
fn public_braid_membership_views_are_historical_by_event_cursor() -> Result<(), BraidError> {
    let first = BraidMemberRef::Revealed(make_strand_id("cursor-member-a"));
    let second = BraidMemberRef::Revealed(make_strand_id("cursor-member-b"));
    let third = BraidMemberRef::Revealed(make_strand_id("cursor-member-c"));
    let mut braid = Braid::new([0xAE; 32], authority_ref());

    assert_eq!(
        braid.membership_at(BraidMembershipCursor::new(0)),
        Vec::<BraidMembershipEntry>::new()
    );

    braid.apply(BraidEvent::MemberWoven {
        member_ref: first,
        sequence_num: 0,
    })?;
    let after_first = braid.current_membership_cursor();

    braid.apply(BraidEvent::MemberWoven {
        member_ref: second,
        sequence_num: 1,
    })?;
    let before_third = braid.current_membership_cursor();

    braid.apply(BraidEvent::MemberWoven {
        member_ref: third,
        sequence_num: 2,
    })?;

    assert_eq!(
        braid.membership_at(after_first),
        vec![BraidMembershipEntry {
            member_ref: first,
            sequence_num: 0,
        }]
    );
    assert_eq!(
        braid.membership_at(before_third),
        vec![
            BraidMembershipEntry {
                member_ref: first,
                sequence_num: 0,
            },
            BraidMembershipEntry {
                member_ref: second,
                sequence_num: 1,
            },
        ]
    );
    assert_eq!(braid.frontier(), &[first, second, third]);
    assert_eq!(
        braid.membership_at(braid.current_membership_cursor()),
        braid.membership_history()
    );
    Ok(())
}

#[test]
fn public_braid_membership_diff_reports_projection_facts() -> Result<(), BraidError> {
    let first = BraidMemberRef::Revealed(make_strand_id("diff-member-a"));
    let second = BraidMemberRef::Revealed(make_strand_id("diff-member-b"));
    let third = BraidMemberRef::Revealed(make_strand_id("diff-member-c"));
    let mut braid = Braid::new([0xAF; 32], authority_ref());

    braid.apply(BraidEvent::MemberWoven {
        member_ref: first,
        sequence_num: 0,
    })?;
    let after_first = braid.current_membership_cursor();
    braid.apply(BraidEvent::MemberWoven {
        member_ref: second,
        sequence_num: 1,
    })?;
    braid.apply(BraidEvent::MemberWoven {
        member_ref: third,
        sequence_num: 2,
    })?;
    let after_third = braid.current_membership_cursor();

    assert_eq!(
        braid.diff_membership(after_first, after_third),
        BraidMembershipDiff {
            from: after_first,
            to: after_third,
            added: vec![
                BraidMembershipEntry {
                    member_ref: second,
                    sequence_num: 1,
                },
                BraidMembershipEntry {
                    member_ref: third,
                    sequence_num: 2,
                },
            ],
            ended: Vec::new(),
            revealed: Vec::new(),
            concealed: Vec::new(),
        }
    );

    assert_eq!(
        braid.diff_membership(after_third, after_first),
        BraidMembershipDiff {
            from: after_third,
            to: after_first,
            added: Vec::new(),
            ended: vec![
                BraidMembershipEntry {
                    member_ref: second,
                    sequence_num: 1,
                },
                BraidMembershipEntry {
                    member_ref: third,
                    sequence_num: 2,
                },
            ],
            revealed: Vec::new(),
            concealed: Vec::new(),
        }
    );
    assert_eq!(braid.frontier(), &[first, second, third]);
    Ok(())
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
