// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Recording reads for the future authored-effect interpreter, not admission.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used)]
use warp_core::edict_effects::{EffectRangeReadError, EffectRangeReadSession};
use warp_core::{
    make_edge_id, make_node_id, make_type_id, EchoOperationBudgetV1, EdgeRecord, GraphStore,
    NodeKey, NodeRecord, OutgoingEdgeRange, SlotId, WorldlineFrontier, WorldlineId, WorldlineState,
};

fn fixture(edges: bool) -> (WorldlineFrontier, OutgoingEdgeRange) {
    let mut store = GraphStore::default();
    let source = make_node_id("source");
    let target = make_node_id("target");
    for id in [source, target] {
        store.insert_node(
            id,
            NodeRecord {
                ty: make_type_id("node"),
            },
        );
    }
    let key = OutgoingEdgeRange {
        source: NodeKey {
            warp_id: store.warp_id(),
            local_id: source,
        },
        edge_type: make_type_id("selected"),
    };
    if edges {
        for (name, ty) in [
            ("selected", key.edge_type),
            ("other", make_type_id("other")),
        ] {
            store.insert_edge(
                source,
                EdgeRecord {
                    id: make_edge_id(name),
                    from: source,
                    to: target,
                    ty,
                },
            );
        }
    }
    let state = WorldlineState::from_root_store(store, source).expect("fixture state");
    (
        WorldlineFrontier::new(WorldlineId::from_bytes([7; 32]), state),
        key,
    )
}

#[test]
fn forbidden_type_is_refused_before_read_or_charge() {
    let (frontier, key) = fixture(true);
    let mut session =
        EffectRangeReadSession::new(&frontier, &[], 8, EchoOperationBudgetV1::new(10, 1024, 0));
    assert_eq!(
        session.read(key),
        Err(EffectRangeReadError::FootprintLawViolation)
    );
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(0, 0, 0));
    assert!(session.readings().is_empty());
    assert_eq!(session.allocated_bytes(), 0);
}

#[test]
fn nonmatching_edges_count_toward_the_preinspection_budget() {
    let (frontier, key) = fixture(true);
    let allowed = [key.edge_type];
    let mut session = EffectRangeReadSession::new(
        &frontier,
        &allowed,
        8,
        EchoOperationBudgetV1::new(2, 128, 0),
    );
    assert_eq!(session.read(key), Err(EffectRangeReadError::BudgetExceeded));
    // Source presence was resolved, but neither edge was inspected or recorded.
    assert_eq!(session.allocated_bytes(), 128);
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(1, 64, 0));
    assert_eq!(session.readings().len(), 1);
    assert!(session.readings().contains_key(&SlotId::Node(key.source)));
    assert!(!session.readings().contains_key(&SlotId::OutgoingEdges(key)));
}

#[test]
fn empty_range_records_the_node_and_range_dependencies() {
    let (frontier, key) = fixture(false);
    let allowed = [key.edge_type];
    let mut session =
        EffectRangeReadSession::new(&frontier, &allowed, 8, EchoOperationBudgetV1::new(1, 64, 0));
    assert_eq!(session.read(key).expect("empty range"), Some(Vec::new()));
    assert_eq!(session.readings().len(), 2);
    assert!(session
        .footprint()
        .n_read
        .iter()
        .any(|node| *node == key.source));
    assert_eq!(session.footprint().r_read.len(), 1);
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(1, 64, 0));
}

#[test]
fn a_successful_scan_filters_values_but_charges_every_edge() {
    let (frontier, key) = fixture(true);
    let allowed = [key.edge_type];
    let mut session = EffectRangeReadSession::new(
        &frontier,
        &allowed,
        2,
        EchoOperationBudgetV1::new(3, 192, 0),
    );
    let expected = EdgeRecord {
        id: make_edge_id("selected"),
        from: key.source.local_id,
        to: make_node_id("target"),
        ty: key.edge_type,
    };
    let mut delivered = session
        .read(key)
        .expect("bounded scan")
        .expect("source present");
    assert_eq!(delivered, vec![expected.clone()]);
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(3, 192, 0));
    // A host result copy cannot change the runtime-owned observation.
    delivered.clear();
    assert_eq!(
        session.readings().get(&SlotId::OutgoingEdges(key)),
        Some(&warp_core::SlotReading::OutgoingEdges(Some(vec![expected])))
    );
}

#[test]
fn absent_source_and_absent_warp_are_recorded_as_absence() {
    let (frontier, mut key) = fixture(false);
    let allowed = [key.edge_type];
    key.source.local_id = make_node_id("missing");
    for range in [
        key,
        OutgoingEdgeRange {
            source: NodeKey {
                warp_id: warp_core::WarpId([99; 32]),
                local_id: key.source.local_id,
            },
            edge_type: key.edge_type,
        },
    ] {
        let mut session = EffectRangeReadSession::new(
            &frontier,
            &allowed,
            0,
            EchoOperationBudgetV1::new(1, 64, 0),
        );
        assert_eq!(session.read(range).expect("absent source"), None);
        assert_eq!(
            session.readings().get(&SlotId::Node(range.source)),
            Some(&warp_core::SlotReading::Node(None))
        );
        assert_eq!(
            session.readings().get(&SlotId::OutgoingEdges(range)),
            Some(&warp_core::SlotReading::OutgoingEdges(None))
        );
        assert_eq!(session.readings().len(), 2);
        assert_eq!(session.consumed(), EchoOperationBudgetV1::new(1, 64, 0));
    }
}

#[test]
fn the_bucket_limit_includes_nonmatching_edges_even_with_spare_budget() {
    let (frontier, key) = fixture(true);
    let allowed = [key.edge_type];
    let mut session = EffectRangeReadSession::new(
        &frontier,
        &allowed,
        1,
        EchoOperationBudgetV1::new(10, 1024, 0),
    );
    assert_eq!(session.read(key), Err(EffectRangeReadError::BudgetExceeded));
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(1, 64, 0));
    assert_eq!(session.readings().len(), 1);
    assert!(session.footprint().r_read.is_empty());
}

#[test]
fn exhausted_node_budget_records_nothing_and_failed_charges_are_atomic() {
    let (frontier, key) = fixture(false);
    let allowed = [key.edge_type];
    for budget in [
        EchoOperationBudgetV1::new(0, 64, 0),
        EchoOperationBudgetV1::new(1, 63, 0),
    ] {
        let mut session = EffectRangeReadSession::new(&frontier, &allowed, 0, budget);
        assert_eq!(session.read(key), Err(EffectRangeReadError::BudgetExceeded));
        assert_eq!(session.consumed(), EchoOperationBudgetV1::new(0, 0, 0));
        assert!(session.readings().is_empty());
        assert!(session.footprint().n_read.is_empty());
        assert_eq!(session.allocated_bytes(), 0);
    }
}

#[test]
fn matching_rows_are_canonical_despite_bucket_insertion_order() {
    let mut store = GraphStore::default();
    let source = make_node_id("source");
    let target = make_node_id("target");
    let ty = make_type_id("selected");
    for id in [source, target] {
        store.insert_node(
            id,
            NodeRecord {
                ty: make_type_id("node"),
            },
        );
    }
    for (id, edge_type) in [(2, ty), (3, make_type_id("other")), (1, ty)] {
        store.insert_edge(
            source,
            EdgeRecord {
                id: warp_core::EdgeId([id; 32]),
                from: source,
                to: target,
                ty: edge_type,
            },
        );
    }
    let key = OutgoingEdgeRange {
        source: NodeKey {
            warp_id: store.warp_id(),
            local_id: source,
        },
        edge_type: ty,
    };
    let state = WorldlineState::from_root_store(store, source).expect("state");
    let frontier = WorldlineFrontier::new(WorldlineId::from_bytes([7; 32]), state);
    let allowed = [ty];
    let mut session = EffectRangeReadSession::new(
        &frontier,
        &allowed,
        3,
        EchoOperationBudgetV1::new(4, 256, 0),
    );
    let rows = session.read(key).expect("read").expect("source");
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![warp_core::EdgeId([1; 32]), warp_core::EdgeId([2; 32])]
    );
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(4, 256, 0));
}

#[test]
fn repeated_reads_charge_each_attempt_without_duplicating_observations() {
    let (frontier, key) = fixture(false);
    let allowed = [key.edge_type];
    let mut session = EffectRangeReadSession::new(
        &frontier,
        &allowed,
        0,
        EchoOperationBudgetV1::new(2, 128, 0),
    );
    for _ in 0..2 {
        assert_eq!(session.read(key).expect("empty range"), Some(Vec::new()));
        assert_eq!(session.readings().len(), 2);
    }
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(2, 128, 0));
    assert_eq!(session.read(key), Err(EffectRangeReadError::BudgetExceeded));
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(2, 128, 0));
    assert_eq!(session.readings().len(), 2);
}

#[test]
fn allocation_refuses_before_node_resolution_and_failed_reservations_are_atomic() {
    let (frontier, key) = fixture(false);
    let allowed = [key.edge_type];
    let mut session = EffectRangeReadSession::with_allocation_limit(
        &frontier,
        &allowed,
        0,
        EchoOperationBudgetV1::new(10, 1024, 0),
        127,
    )
    .expect("valid declared limit");
    assert_eq!(session.read(key), Err(EffectRangeReadError::BudgetExceeded));
    assert_eq!(session.allocated_bytes(), 0);
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(0, 0, 0));
    assert!(session.readings().is_empty());
}

#[test]
fn empty_range_storage_has_an_exact_success_boundary() {
    let (frontier, key) = fixture(false);
    let allowed = [key.edge_type];
    for (limit, succeeds, allocated, observations) in [(255, false, 128, 1), (256, true, 256, 2)] {
        let mut session = EffectRangeReadSession::with_allocation_limit(
            &frontier,
            &allowed,
            0,
            EchoOperationBudgetV1::new(1, 64, 0),
            limit,
        )
        .expect("valid limit");
        assert_eq!(session.read(key).is_ok(), succeeds);
        assert_eq!(session.allocated_bytes(), allocated);
        assert_eq!(session.readings().len(), observations);
        assert_eq!(session.consumed(), EchoOperationBudgetV1::new(1, 64, 0));
    }
}

#[test]
fn scan_reserves_both_row_vectors_for_the_whole_bucket_before_filtering() {
    let (frontier, key) = fixture(true);
    let allowed = [key.edge_type];
    // 128 node + 128 range + 2 copies * 2 bucket edges * 128 row bytes.
    for (limit, succeeds, allocated, steps, bytes) in
        [(767, false, 128, 1, 64), (768, true, 768, 3, 192)]
    {
        let mut session = EffectRangeReadSession::with_allocation_limit(
            &frontier,
            &allowed,
            8,
            EchoOperationBudgetV1::new(10, 1024, 0),
            limit,
        )
        .expect("valid limit");
        assert_eq!(session.read(key).is_ok(), succeeds);
        assert_eq!(session.allocated_bytes(), allocated);
        assert_eq!(
            session.consumed(),
            EchoOperationBudgetV1::new(steps, bytes, 0)
        );
    }
}

#[test]
fn allocation_declarations_above_the_fixed_ceiling_are_refused() {
    let (frontier, key) = fixture(false);
    let allowed = [key.edge_type];
    assert!(matches!(
        EffectRangeReadSession::with_allocation_limit(
            &frontier,
            &allowed,
            0,
            EchoOperationBudgetV1::new(1, 64, 0),
            16_777_217,
        ),
        Err(warp_core::edict_effects::EffectReadSetupError::AllocationLimitTooLarge)
    ));
    let mut zero = EffectRangeReadSession::with_allocation_limit(
        &frontier,
        &allowed,
        0,
        EchoOperationBudgetV1::new(1, 64, 0),
        0,
    )
    .expect("zero is a valid declaration");
    assert_eq!(zero.read(key), Err(EffectRangeReadError::BudgetExceeded));
    assert_eq!(zero.allocated_bytes(), 0);
    assert!(zero.readings().is_empty());
}

#[test]
fn repeated_reads_do_not_refund_cumulative_storage() {
    let (frontier, key) = fixture(false);
    let allowed = [key.edge_type];
    let mut session = EffectRangeReadSession::with_allocation_limit(
        &frontier,
        &allowed,
        0,
        EchoOperationBudgetV1::new(10, 1024, 0),
        511,
    )
    .expect("valid declaration");
    assert_eq!(
        session.read(key).expect("first empty range"),
        Some(Vec::new())
    );
    assert_eq!(session.allocated_bytes(), 256);
    assert_eq!(session.read(key), Err(EffectRangeReadError::BudgetExceeded));
    assert_eq!(session.allocated_bytes(), 384);
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(2, 128, 0));
    assert_eq!(session.readings().len(), 2);
    assert_eq!(session.read(key), Err(EffectRangeReadError::BudgetExceeded));
    assert_eq!(session.allocated_bytes(), 384);
    assert_eq!(session.consumed(), EchoOperationBudgetV1::new(2, 128, 0));
}
