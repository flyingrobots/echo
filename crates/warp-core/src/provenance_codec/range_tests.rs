// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)] // Test assertions may abort the witness.
use super::*;
use crate::OutgoingEdgeRange;

fn range() -> OutgoingEdgeRange {
    OutgoingEdgeRange {
        source: NodeKey {
            warp_id: WarpId([1; 32]),
            local_id: NodeId([2; 32]),
        },
        edge_type: TypeId([3; 32]),
    }
}

#[test]
fn retained_range_slot_has_exact_bytes_and_refuses_every_truncated_prefix() {
    let slot = SlotId::OutgoingEdges(range());
    let mut bytes = Vec::new();
    push_slot(&mut bytes, slot);
    let expected = [vec![5], vec![1; 32], vec![2; 32], vec![3; 32]].concat();
    assert_eq!(bytes, expected);
    let mut cursor = RetainedProvenanceCursor::new(&bytes);
    assert_eq!(cursor.read_slot(), Ok(slot));
    assert_eq!(cursor.finish(), Ok(()));
    for end in 0..bytes.len() {
        let mut cursor = RetainedProvenanceCursor::new(&bytes[..end]);
        assert_eq!(
            cursor.read_slot(),
            Err(RetainedProvenanceError::UnexpectedEof)
        );
    }
}

#[test]
fn legacy_point_slot_bytes_remain_unchanged() {
    let mut bytes = Vec::new();
    push_slot(&mut bytes, SlotId::Node(range().source));
    assert_eq!(bytes, [vec![1], vec![1; 32], vec![2; 32]].concat());
    bytes.clear();
    push_slot(&mut bytes, SlotId::Port((range().source.warp_id, 7)));
    assert_eq!(
        bytes,
        [vec![4], vec![1; 32], 7_u64.to_le_bytes().to_vec()].concat()
    );
}

#[test]
fn patch_identity_binds_each_range_component_and_read_write_role() {
    let patch = |read, key| {
        let slot = SlotId::OutgoingEdges(key);
        WarpTickPatchV1::new(
            1,
            [0; 32],
            TickCommitStatus::Committed,
            if read { vec![slot] } else { vec![] },
            if read { vec![] } else { vec![slot] },
            vec![],
        )
    };
    let base = patch(true, range());
    for key in [
        OutgoingEdgeRange {
            source: NodeKey {
                warp_id: WarpId([4; 32]),
                ..range().source
            },
            ..range()
        },
        OutgoingEdgeRange {
            source: NodeKey {
                local_id: NodeId([4; 32]),
                ..range().source
            },
            ..range()
        },
        OutgoingEdgeRange {
            edge_type: TypeId([4; 32]),
            ..range()
        },
    ] {
        assert_ne!(base.digest(), patch(true, key).digest());
    }
    assert_ne!(base.digest(), patch(false, range()).digest());
}

#[test]
fn retained_worldline_patch_preserves_mixed_point_and_range_slots() {
    let slot = SlotId::OutgoingEdges(range());
    let patch = WarpTickPatchV1::new(
        1,
        [7; 32],
        TickCommitStatus::Committed,
        vec![slot, SlotId::Node(range().source)],
        vec![slot],
        vec![],
    );
    let retained = WorldlineTickPatchV1 {
        header: WorldlineTickHeaderV1 {
            commit_global_tick: GlobalTick::from_raw(1),
            policy_id: 1,
            rule_pack_id: [7; 32],
            plan_digest: [8; 32],
            decision_digest: [9; 32],
            rewrites_digest: [10; 32],
        },
        warp_id: range().source.warp_id,
        ops: patch.ops().to_vec(),
        in_slots: patch.in_slots().to_vec(),
        out_slots: patch.out_slots().to_vec(),
        patch_digest: patch.digest(),
    };
    let mut bytes = Vec::new();
    push_worldline_patch(&mut bytes, &retained);
    let mut cursor = RetainedProvenanceCursor::new(&bytes);
    let decoded = cursor.read_worldline_patch().expect("retained patch");
    assert_eq!(cursor.finish(), Ok(()));
    assert_eq!(decoded, retained);
    let rebuilt = WarpTickPatchV1::new(
        decoded.header.policy_id,
        decoded.header.rule_pack_id,
        TickCommitStatus::Committed,
        decoded.in_slots,
        decoded.out_slots,
        decoded.ops,
    );
    assert_eq!(rebuilt.digest(), retained.patch_digest);
}
