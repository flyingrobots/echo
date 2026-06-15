// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! External-consumer braid public API checks.

use warp_core::{
    make_strand_id, AuthorityDomainId, AuthorityDomainRef, Braid, BraidError, BraidEvent,
    BraidMemberRef, BraidStatus, OriginId,
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
