#![allow(missing_docs)]
use bytes::Bytes;
use rmg_core::{
    decode_motion_payload, encode_motion_payload, make_node_id, make_type_id, ApplyResult, Engine,
    GraphStore, NodeRecord, MOTION_RULE_NAME,
};

#[test]
fn motion_apply_with_nan_propagates_nan_but_still_applies() {
    let ent = make_node_id("nan-case");
    let ty = make_type_id("entity");
    let pos = [f32::NAN, 0.0, 1.0];
    let vel = [0.0, f32::NAN, 2.0];

    let mut store = GraphStore::default();
    store.insert_node(
        ent,
        NodeRecord {
            ty,
            payload: Some(encode_motion_payload(pos, vel)),
        },
    );

    let mut engine = Engine::new(store, ent);
    engine
        .register_rule(rmg_core::motion_rule())
        .expect("register motion rule");

    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let node = engine.node(&ent).expect("node exists");
    let (new_pos, new_vel) =
        decode_motion_payload(node.payload.as_ref().expect("payload")).expect("decode");

    // NaN arithmetic propagates; check using is_nan rather than bitwise.
    assert!(new_pos[0].is_nan());
    assert!(new_pos[1].is_nan());
    assert_eq!(new_pos[2].to_bits(), (1.0f32 + 2.0f32).to_bits());

    // Velocity preserved; NaN stays NaN; finite components equal bitwise.
    assert!(new_vel[1].is_nan());
    assert_eq!(new_vel[0].to_bits(), 0.0f32.to_bits());
    assert_eq!(new_vel[2].to_bits(), 2.0f32.to_bits());
}

#[test]
fn motion_apply_with_infinity_preserves_infinite_values() {
    let ent = make_node_id("inf-case");
    let ty = make_type_id("entity");
    let pos = [f32::INFINITY, 1.0, f32::NEG_INFINITY];
    let vel = [1.0, 2.0, 3.0];

    let mut store = GraphStore::default();
    store.insert_node(
        ent,
        NodeRecord {
            ty,
            payload: Some(encode_motion_payload(pos, vel)),
        },
    );

    let mut engine = Engine::new(store, ent);
    engine
        .register_rule(rmg_core::motion_rule())
        .expect("register motion rule");

    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let node = engine.node(&ent).expect("node exists");
    let (new_pos, new_vel) =
        decode_motion_payload(node.payload.as_ref().expect("payload")).expect("decode");

    assert!(new_pos[0].is_infinite() && new_pos[0].is_sign_positive());
    assert_eq!(new_pos[1].to_bits(), 3.0f32.to_bits());
    assert!(new_pos[2].is_infinite() && new_pos[2].is_sign_negative());

    for i in 0..3 {
        assert_eq!(new_vel[i].to_bits(), vel[i].to_bits());
    }
}

#[test]
fn motion_apply_invalid_payload_size_returns_nomatch() {
    let ent = make_node_id("bad-payload-size");
    let ty = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(
        ent,
        NodeRecord {
            ty,
            payload: Some(Bytes::from(vec![0u8; 10])),
        },
    );
    let mut engine = Engine::new(store, ent);
    engine
        .register_rule(rmg_core::motion_rule())
        .expect("register motion rule");
    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::NoMatch));
}
