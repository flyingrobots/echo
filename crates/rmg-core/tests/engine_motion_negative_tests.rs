// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
//! Negative/edge-case tests for the motion rule.
//!
//! These tests document behavior when payloads contain non-finite values
//! (NaN/Infinity) and when payload length is invalid. The runtime does not
//! sanitize non-finite inputs; NaN propagates and Infinity is preserved. An
//! invalid payload size results in `ApplyResult::NoMatch` at the apply boundary.
use bytes::Bytes;
use rmg_core::{
    decode_motion_payload, encode_motion_payload, make_node_id, make_type_id, ApplyResult, Engine,
    GraphStore, NodeRecord, MOTION_RULE_NAME,
};

fn run_motion_once(pos: [f32; 3], vel: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let ent = make_node_id("case");
    let ty = make_type_id("entity");
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
    let _ = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    engine.commit(tx).expect("commit");
    let node = engine.node(&ent).expect("node exists");
    decode_motion_payload(node.payload.as_ref().expect("payload")).expect("decode")
}

#[test]
fn motion_nan_propagates_and_rule_applies() {
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
    assert!(new_pos[0].is_nan(), "pos.x should be NaN after update");
    assert!(new_pos[1].is_nan(), "pos.y should be NaN after update");
    assert_eq!(new_pos[2].to_bits(), (1.0f32 + 2.0f32).to_bits());

    // Velocity preserved; NaN stays NaN; finite components equal bitwise.
    assert!(new_vel[1].is_nan(), "vel.y should remain NaN");
    assert_eq!(new_vel[0].to_bits(), 0.0f32.to_bits());
    assert_eq!(new_vel[2].to_bits(), 2.0f32.to_bits());
}

#[test]
fn motion_infinity_preserves_infinite_values() {
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
fn motion_invalid_payload_size_returns_nomatch() {
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

#[test]
fn motion_all_position_components_nan_stay_nan() {
    let (new_pos, new_vel) = run_motion_once([f32::NAN, f32::NAN, f32::NAN], [0.0, 0.0, 0.0]);
    assert!(new_pos[0].is_nan());
    assert!(new_pos[1].is_nan());
    assert!(new_pos[2].is_nan());
    // Velocity preserved
    assert_eq!(new_vel, [0.0, 0.0, 0.0]);
}

#[test]
fn motion_all_velocity_components_nan_propagate_to_position_nan() {
    let (new_pos, new_vel) = run_motion_once([1.0, 2.0, 3.0], [f32::NAN, f32::NAN, f32::NAN]);
    assert!(new_pos[0].is_nan());
    assert!(new_pos[1].is_nan());
    assert!(new_pos[2].is_nan());
    assert!(new_vel[0].is_nan());
    assert!(new_vel[1].is_nan());
    assert!(new_vel[2].is_nan());
}

#[test]
fn motion_infinity_plus_infinity_remains_infinite() {
    let (new_pos, new_vel) = run_motion_once(
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
    );
    assert!(new_pos[0].is_infinite() && new_pos[0].is_sign_positive());
    assert!(new_pos[1].is_infinite() && new_pos[1].is_sign_negative());
    assert_eq!(new_pos[2].to_bits(), 0.0f32.to_bits());
    for i in 0..3 {
        assert_eq!(
            new_vel[i].to_bits(),
            [f32::INFINITY, f32::NEG_INFINITY, 0.0][i].to_bits()
        );
    }
}

#[test]
fn motion_infinity_minus_infinity_results_nan() {
    // +inf + (-inf) → NaN, and -inf + (+inf) → NaN
    let (new_pos, _) = run_motion_once(
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::NEG_INFINITY, f32::INFINITY, 0.0],
    );
    assert!(new_pos[0].is_nan());
    assert!(new_pos[1].is_nan());
}

#[test]
fn motion_mixed_nan_and_infinity_behaves_as_expected() {
    // NaN dominates arithmetic; Infinity preserves sign where finite partner exists;
    // Infinity + (-Infinity) becomes NaN per IEEE-754.
    let (new_pos, new_vel) = run_motion_once(
        [f32::NAN, f32::INFINITY, 1.0],
        [2.0, f32::NEG_INFINITY, f32::NAN],
    );
    assert!(new_pos[0].is_nan());
    assert!(new_pos[2].is_nan());
    assert!(new_pos[1].is_nan());
    assert_eq!(new_vel[0].to_bits(), 2.0f32.to_bits());
    assert!(new_vel[1].is_infinite() && new_vel[1].is_sign_negative());
    assert!(new_vel[2].is_nan());
}

#[test]
fn motion_signed_zero_preservation_against_expected_math() {
    // Compare to direct arithmetic to avoid making assumptions about zero sign rules.
    let pos = [0.0f32, -0.0, 0.0];
    let vel = [-0.0f32, 0.0, -0.0];
    let (new_pos, new_vel) = run_motion_once(pos, vel);
    for i in 0..3 {
        assert_eq!(new_pos[i].to_bits(), (pos[i] + vel[i]).to_bits());
        assert_eq!(new_vel[i].to_bits(), vel[i].to_bits());
    }
}

#[test]
fn motion_subnormal_and_extreme_values_follow_ieee_math() {
    let sub = f32::from_bits(1); // smallest positive subnormal
    let pos = [f32::MAX, -f32::MAX, sub];
    let vel = [sub, sub, sub];
    let (new_pos, new_vel) = run_motion_once(pos, vel);
    for i in 0..3 {
        assert_eq!(new_pos[i].to_bits(), (pos[i] + vel[i]).to_bits());
        assert_eq!(new_vel[i].to_bits(), vel[i].to_bits());
    }
}

#[test]
fn motion_zero_length_payload_returns_nomatch() {
    let ent = make_node_id("bad-size-0");
    let ty = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(
        ent,
        NodeRecord {
            ty,
            payload: Some(Bytes::from(vec![])),
        },
    );
    let mut engine = Engine::new(store, ent);
    engine.register_rule(rmg_core::motion_rule()).unwrap();
    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::NoMatch));
}

#[test]
fn motion_boundary_payload_sizes() {
    for &len in &[1usize, 23, 25, 32, 4096] {
        let ent = make_node_id(&format!("bad-size-{}", len));
        let ty = make_type_id("entity");
        let mut store = GraphStore::default();
        store.insert_node(
            ent,
            NodeRecord {
                ty,
                payload: Some(Bytes::from(vec![0u8; len])),
            },
        );
        let mut engine = Engine::new(store, ent);
        engine.register_rule(rmg_core::motion_rule()).unwrap();
        let tx = engine.begin();
        let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
        assert!(
            matches!(res, ApplyResult::NoMatch),
            "len={} should be NoMatch",
            len
        );
    }
}

#[test]
fn motion_exact_24_bytes_with_weird_bits_is_accepted_and_propagates() {
    // 24 bytes of 0xFF -> three NaNs for pos, three NaNs for vel
    let weird = Bytes::from(vec![0xFFu8; 24]);
    let ent = make_node_id("weird-24");
    let ty = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(
        ent,
        NodeRecord {
            ty,
            payload: Some(weird),
        },
    );
    let mut engine = Engine::new(store, ent);
    engine.register_rule(rmg_core::motion_rule()).unwrap();
    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::Applied));
    engine.commit(tx).unwrap();
    let (pos, vel) = {
        let node = engine.node(&ent).unwrap();
        decode_motion_payload(node.payload.as_ref().unwrap()).unwrap()
    };
    assert!(pos.iter().all(|v| v.is_nan()));
    assert!(vel.iter().all(|v| v.is_nan()));
}

#[test]
fn motion_nan_idempotency_applies_twice_stays_nan() {
    let ent = make_node_id("nan-twice");
    let ty = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(
        ent,
        NodeRecord {
            ty,
            payload: Some(encode_motion_payload(
                [f32::NAN, f32::NAN, f32::NAN],
                [0.0, 0.0, 0.0],
            )),
        },
    );
    let mut engine = Engine::new(store, ent);
    engine.register_rule(rmg_core::motion_rule()).unwrap();
    for _ in 0..2 {
        let tx = engine.begin();
        let res = engine.apply(tx, MOTION_RULE_NAME, &ent).unwrap();
        assert!(matches!(res, ApplyResult::Applied));
        engine.commit(tx).unwrap();
    }
    let (pos, vel) = {
        let node = engine.node(&ent).unwrap();
        decode_motion_payload(node.payload.as_ref().unwrap()).unwrap()
    };
    assert!(pos.iter().all(|v| v.is_nan()));
    assert_eq!(vel, [0.0, 0.0, 0.0]);
}
