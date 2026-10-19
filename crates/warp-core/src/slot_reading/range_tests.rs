// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)] // Test assertions may abort the witness.
use super::*;
use crate::{GraphStore, NodeId, OutgoingEdgeRange, TickCommitStatus, TypeId, WarpInstance};

fn edge(id: u8, source: u8, ty: u8) -> WarpOp {
    WarpOp::UpsertEdge {
        warp_id: WarpId([1; 32]),
        record: EdgeRecord {
            id: crate::EdgeId([id; 32]),
            from: NodeId([source; 32]),
            to: NodeId([3; 32]),
            ty: TypeId([ty; 32]),
        },
    }
}

#[test]
fn historical_range_reads_aggregate_insertions_relabels_moves_and_deletions() {
    let warp_id = WarpId([1; 32]);
    let mut store = GraphStore::new(warp_id);
    for node in [2, 3, 4] {
        store.insert_node(
            NodeId([node; 32]),
            NodeRecord {
                ty: TypeId([8; 32]),
            },
        );
    }
    let mut initial = WarpState::new();
    initial.upsert_instance(
        WarpInstance {
            warp_id,
            root_node: NodeId([2; 32]),
            parent: None,
        },
        store,
    );
    let slot = SlotId::OutgoingEdges(OutgoingEdgeRange {
        source: NodeKey {
            warp_id,
            local_id: NodeId([2; 32]),
        },
        edge_type: TypeId([9; 32]),
    });
    let ops = [
        vec![edge(22, 2, 9), edge(20, 2, 9), edge(21, 2, 10)],
        vec![edge(20, 2, 10)],
        vec![edge(22, 4, 9)],
        vec![edge(24, 2, 9)],
        vec![
            WarpOp::DeleteEdge {
                warp_id,
                from: NodeId([2; 32]),
                edge_id: crate::EdgeId([20; 32]),
            },
            WarpOp::DeleteEdge {
                warp_id,
                from: NodeId([2; 32]),
                edge_id: crate::EdgeId([21; 32]),
            },
            WarpOp::DeleteEdge {
                warp_id,
                from: NodeId([4; 32]),
                edge_id: crate::EdgeId([22; 32]),
            },
            WarpOp::DeleteEdge {
                warp_id,
                from: NodeId([2; 32]),
                edge_id: crate::EdgeId([24; 32]),
            },
            WarpOp::DeleteNode {
                node: NodeKey {
                    warp_id,
                    local_id: NodeId([3; 32]),
                },
            },
        ],
        vec![WarpOp::DeleteNode {
            node: NodeKey {
                warp_id,
                local_id: NodeId([2; 32]),
            },
        }],
    ];
    let patches: Vec<_> = ops
        .into_iter()
        .map(|ops| {
            WarpTickPatchV1::new(1, [0; 32], TickCommitStatus::Committed, vec![], vec![], ops)
        })
        .collect();
    let mut index = SlotWriteIndex::new();
    for patch in &patches {
        index.push_patch(patch);
    }
    let reader = index.reader(&patches, &initial).expect("bound history");
    let expected = [
        Some(vec![]),
        Some(vec![20, 22]),
        Some(vec![22]),
        Some(vec![]),
        Some(vec![24]),
        Some(vec![]),
        None,
    ];
    for (cursor, ids) in expected.into_iter().enumerate() {
        let SlotReading::OutgoingEdges(rows) = reader.read(slot, cursor).expect("range reading")
        else {
            panic!("range value")
        };
        assert_eq!(
            rows.map(|rows| rows
                .into_iter()
                .map(|row| {
                    assert_eq!(row.from, NodeId([2; 32]));
                    assert_eq!(row.to, NodeId([3; 32]));
                    assert_eq!(row.ty, TypeId([9; 32]));
                    row.id.0[0]
                })
                .collect::<Vec<_>>()),
            ids,
            "cursor {cursor}"
        );
    }
    assert_eq!(
        materialized_slot(&initial, slot),
        Some(SlotReading::OutgoingEdges(Some(vec![])))
    );
}

#[test]
fn range_reading_refuses_a_prefix_that_cannot_replay() {
    let warp_id = WarpId([1; 32]);
    let initial = WarpState::new();
    let slot = SlotId::OutgoingEdges(OutgoingEdgeRange {
        source: NodeKey {
            warp_id,
            local_id: NodeId([2; 32]),
        },
        edge_type: TypeId([9; 32]),
    });
    let patch = WarpTickPatchV1::new(
        1,
        [0; 32],
        TickCommitStatus::Committed,
        vec![],
        vec![],
        vec![edge(20, 2, 9)],
    );
    let patches = [patch];
    let mut index = SlotWriteIndex::new();
    index.push_patch(&patches[0]);
    let reader = index.reader(&patches, &initial).expect("bound history");
    assert_eq!(reader.read(slot, 0), Ok(SlotReading::OutgoingEdges(None)));
    assert_eq!(
        reader.read(slot, 1),
        Err(SlotReadingError::InvalidRangeHistory)
    );
    assert_eq!(
        materialized_slot(&initial, slot),
        Some(SlotReading::OutgoingEdges(None))
    );
}
