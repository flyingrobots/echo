// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for the Engine ledger/history API.

use warp_core::{make_node_id, make_type_id, Engine, GraphStore, NodeRecord};

#[test]
fn engine_ledger_records_commits() {
    let root = make_node_id("root");
    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("root"),
        },
    );
    let mut engine = Engine::new(store, root);

    // Ledger should be empty initially
    assert_eq!(engine.get_ledger().len(), 0);

    // Commit 1
    let tx1 = engine.begin();
    engine.commit(tx1).expect("commit 1");
    assert_eq!(engine.get_ledger().len(), 1);

    // Commit 2
    let tx2 = engine.begin();
    engine.commit(tx2).expect("commit 2");
    assert_eq!(engine.get_ledger().len(), 2);

    let (snapshot, receipt, patch) = &engine.get_ledger()[1];
    assert_eq!(snapshot.tx.value(), 2);
    assert_eq!(receipt.tx().value(), 2);
    assert_eq!(patch.policy_id(), warp_core::POLICY_ID_NO_POLICY_V0);
}
