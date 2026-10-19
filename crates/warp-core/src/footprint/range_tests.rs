// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::*;
use crate::TypeId;

fn range(warp: u8, source: u8, ty: u8) -> OutgoingEdgeRange {
    OutgoingEdgeRange {
        source: NodeKey {
            warp_id: WarpId([warp; 32]),
            local_id: NodeId([source; 32]),
        },
        edge_type: TypeId([ty; 32]),
    }
}

#[test]
fn an_empty_range_observation_conflicts_with_a_fresh_edge_write() {
    let key = range(1, 2, 3);
    let mut observation = Footprint::default();
    observation.r_read.insert(key);
    let mut insertion = Footprint::default();
    insertion.r_write.insert(key);
    insertion.e_write.insert(EdgeKey {
        warp_id: key.source.warp_id,
        local_id: EdgeId([99; 32]),
    });
    assert!(!observation.independent(&insertion));
    assert!(!insertion.independent(&observation));
}

#[test]
fn range_claims_keep_read_sharing_and_full_key_separation() {
    let mut left = Footprint::default();
    left.r_read.insert(range(1, 2, 3));
    assert!(left.independent(&left));
    for key in [range(9, 2, 3), range(1, 9, 3), range(1, 2, 9)] {
        let mut right = Footprint::default();
        right.r_write.insert(key);
        assert!(left.independent(&right));
        assert!(right.independent(&left));
    }
    left.r_write.insert(range(1, 2, 3));
    assert!(!left.independent(&left));
}

#[test]
fn union_preserves_range_claims_and_the_unpopulated_mask() {
    let mut ranges = Footprint::default();
    ranges.r_read.insert(range(1, 2, 3));
    ranges.r_write.insert(range(1, 4, 3));
    let original = ranges.clone();
    let mut populated = Footprint {
        factor_mask: 1,
        ..Footprint::default()
    };
    populated.n_read.insert(range(1, 2, 3).source);
    ranges.union_assign(&populated);
    assert_eq!(ranges.factor_mask, 0);
    assert_eq!(ranges.r_read, original.r_read);
    assert_eq!(ranges.r_write, original.r_write);
    populated.union_assign(&original);
    assert_eq!(populated.factor_mask, 0);
    assert_eq!(populated.r_read, original.r_read);
    assert_eq!(populated.r_write, original.r_write);
    assert_eq!(ranges.bounded_site().claim_footprint, ranges);
}

#[test]
fn admission_region_preserves_the_range_write_wake() {
    let mut footprint = Footprint::default();
    footprint.r_write.insert(range(1, 2, 3));
    assert_eq!(
        footprint
            .bounded_site()
            .affected_region
            .outgoing_edge_ranges,
        footprint.r_write
    );
}
