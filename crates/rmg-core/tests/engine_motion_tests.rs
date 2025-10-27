#![allow(missing_docs)]
use rmg_core::{
    decode_motion_payload, encode_motion_payload, make_node_id, make_type_id, ApplyResult, Engine,
    EngineError, GraphStore, NodeRecord, MOTION_RULE_NAME,
};

#[test]
fn motion_rule_updates_position_deterministically() {
    let entity = make_node_id("entity-1");
    let entity_type = make_type_id("entity");
    let payload = encode_motion_payload([1.0, 2.0, 3.0], [0.5, -1.0, 0.25]);

    let mut store = GraphStore::default();
    store.insert_node(
        entity,
        NodeRecord {
            ty: entity_type,
            payload: Some(payload),
        },
    );

    let mut engine = Engine::new(store, entity);
    engine.register_rule(rmg_core::motion_rule());

    let tx = engine.begin();
    let apply = engine.apply(tx, MOTION_RULE_NAME, &entity).unwrap();
    assert!(matches!(apply, ApplyResult::Applied));

    let snap = engine.commit(tx).expect("commit");
    let hash_after_first_apply = snap.hash;

    // Run a second engine with identical initial state and ensure hashes match.
    let mut store_b = GraphStore::default();
    let payload_b = encode_motion_payload([1.0, 2.0, 3.0], [0.5, -1.0, 0.25]);
    store_b.insert_node(
        entity,
        NodeRecord {
            ty: entity_type,
            payload: Some(payload_b),
        },
    );

    let mut engine_b = Engine::new(store_b, entity);
    engine_b.register_rule(rmg_core::motion_rule());
    let tx_b = engine_b.begin();
    let apply_b = engine_b.apply(tx_b, MOTION_RULE_NAME, &entity).unwrap();
    assert!(matches!(apply_b, ApplyResult::Applied));
    let snap_b = engine_b.commit(tx_b).expect("commit B");

    assert_eq!(hash_after_first_apply, snap_b.hash);

    // Ensure the position actually moved.
    let node = engine
        .node(&entity)
        .expect("entity exists")
        .payload
        .as_ref()
        .and_then(decode_motion_payload)
        .expect("payload decode");
    assert_eq!(node.0, [1.5, 1.0, 3.25]);
}

#[test]
fn motion_rule_no_match_on_missing_payload() {
    let entity = make_node_id("entity-2");
    let entity_type = make_type_id("entity");

    let mut store = GraphStore::default();
    store.insert_node(
        entity,
        NodeRecord {
            ty: entity_type,
            payload: None,
        },
    );

    let mut engine = Engine::new(store, entity);
    engine.register_rule(rmg_core::motion_rule());

    let tx = engine.begin();
    let apply = engine.apply(tx, MOTION_RULE_NAME, &entity).unwrap();
    assert!(matches!(apply, ApplyResult::NoMatch));
}

#[test]
fn apply_unknown_rule_returns_error() {
    let entity = make_node_id("entity-unknown-rule");
    let entity_type = make_type_id("entity");

    let mut store = GraphStore::default();
    store.insert_node(
        entity,
        NodeRecord {
            ty: entity_type,
            payload: Some(encode_motion_payload([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])),
        },
    );

    let mut engine = Engine::new(store, entity);
    let tx = engine.begin();
    let result = engine.apply(tx, "missing-rule", &entity);
    assert!(matches!(result, Err(EngineError::UnknownRule(rule)) if rule == "missing-rule"));
}
