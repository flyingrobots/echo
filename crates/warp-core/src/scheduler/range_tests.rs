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

/// The slots read back from the conflict sets come out sorted and free of
/// repeats on their own, before `WarpTickPatchV1::new` sorts them again:
/// node slots before range slots, keys ascending, and a key both read and
/// written listed once.
#[test]
fn reserved_slots_are_sorted_unions_without_a_sort() {
    let tx = TxId::from_raw(1);
    let mut scheduler = RadixScheduler::default();
    let mut reads = candidate(&[key(5), key(3)], &[]);
    reads.footprint.n_read.insert(key(1).source);
    assert!(scheduler.reserve(tx, &mut reads));
    assert!(scheduler.reserve(tx, &mut candidate(&[], &[key(4)])));
    assert!(scheduler.reserve(tx, &mut candidate(&[], &[key(6)])));
    assert!(scheduler.reserve(tx, &mut candidate(&[key(7)], &[key(7)])));
    let (in_slots, out_slots) = scheduler.take_reserved_slots(tx);
    assert_eq!(
        in_slots,
        vec![
            SlotId::Node(key(1).source),
            SlotId::OutgoingEdges(key(3)),
            SlotId::OutgoingEdges(key(4)),
            SlotId::OutgoingEdges(key(5)),
            SlotId::OutgoingEdges(key(6)),
            SlotId::OutgoingEdges(key(7)),
        ]
    );
    assert_eq!(
        out_slots,
        vec![
            SlotId::OutgoingEdges(key(4)),
            SlotId::OutgoingEdges(key(6)),
            SlotId::OutgoingEdges(key(7)),
        ]
    );
}
