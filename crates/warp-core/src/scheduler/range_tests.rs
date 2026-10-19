// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::*;
use crate::{footprint::OutgoingEdgeRange, NodeId, TypeId, WarpId};

fn key(ty: u8) -> OutgoingEdgeRange {
    OutgoingEdgeRange {
        source: NodeKey {
            warp_id: WarpId([1; 32]),
            local_id: NodeId([2; 32]),
        },
        edge_type: TypeId([ty; 32]),
    }
}

fn candidate(reads: &[OutgoingEdgeRange], writes: &[OutgoingEdgeRange]) -> PendingRewrite {
    let mut footprint = Footprint::default();
    for range in reads {
        footprint.r_read.insert(*range);
    }
    for range in writes {
        footprint.r_write.insert(*range);
    }
    PendingRewrite {
        rule_id: [1; 32],
        compact_rule: CompactRuleId(1),
        scope_hash: [2; 32],
        scope: key(1).source,
        footprint,
        phase: RewritePhase::Matched,
        origin: OpOrigin::default(),
    }
}

#[test]
fn range_reservations_conflict_in_both_orders_and_allow_shared_reads() {
    let tx = TxId::from_raw(1);
    for (left_writes, right_writes, expected) in [
        (false, false, true),
        (false, true, false),
        (true, false, false),
        (true, true, false),
    ] {
        let mut scheduler = RadixScheduler::default();
        let make = |writes| {
            if writes {
                candidate(&[], &[key(3)])
            } else {
                candidate(&[key(3)], &[])
            }
        };
        assert!(scheduler.reserve(tx, &mut make(left_writes)));
        let mut right = make(right_writes);
        assert_eq!(scheduler.reserve(tx, &mut right), expected);
        assert_eq!(right.phase == RewritePhase::Reserved, expected);
        assert!(scheduler.reserve(tx, &mut candidate(&[], &[key(4)])));
    }
}

#[test]
fn rejected_range_claims_do_not_leak_and_finalization_releases_them() {
    let tx = TxId::from_raw(1);
    let mut scheduler = RadixScheduler::default();
    assert!(scheduler.reserve(tx, &mut candidate(&[key(3)], &[])));
    assert!(!scheduler.reserve(tx, &mut candidate(&[], &[key(3), key(4)])));
    assert!(scheduler.reserve(tx, &mut candidate(&[key(4)], &[])));
    scheduler.finalize_tx(tx);
    assert!(scheduler.reserve(tx, &mut candidate(&[], &[key(3), key(4)])));
}
