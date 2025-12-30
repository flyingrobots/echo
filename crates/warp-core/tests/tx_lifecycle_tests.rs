// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use warp_core::{
    encode_motion_atom_payload, make_node_id, make_type_id, EngineError, GraphStore, NodeRecord,
    MOTION_RULE_NAME,
};

#[test]
fn tx_invalid_after_commit() {
    let entity = make_node_id("tx-lifecycle-entity");
    let entity_type = make_type_id("entity");
    let payload = encode_motion_atom_payload([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);

    let mut store = GraphStore::default();
    store.insert_node(
        entity,
        NodeRecord {
            ty: entity_type,
            payload: Some(payload),
        },
    );

    let mut engine = warp_core::Engine::new(store, entity);
    engine
        .register_rule(warp_core::motion_rule())
        .expect("duplicate rule name");

    let tx = engine.begin();
    // Valid apply then commit
    engine.apply(tx, MOTION_RULE_NAME, &entity).unwrap();
    engine.commit(tx).unwrap();

    // Reusing the same tx should be rejected
    let err = engine.apply(tx, MOTION_RULE_NAME, &entity).unwrap_err();
    match err {
        EngineError::UnknownTx => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
