#![allow(missing_docs)]
use bytes::Bytes;
use once_cell::sync::Lazy;
use serde::Deserialize;

use rmg_core::{
    build_motion_demo_engine, decode_motion_payload, encode_motion_payload, make_node_id,
    make_type_id, ApplyResult, Engine, NodeRecord, MOTION_RULE_NAME,
};

static RAW: &str = include_str!("fixtures/motion-fixtures.json");

#[derive(Debug, Deserialize)]
struct MotionCase {
    label: String,
    pos: [f32; 3],
    vel: [f32; 3],
    expected_pos: [f32; 3],
}

#[derive(Debug, Deserialize)]
struct MotionFixtures {
    cases: Vec<MotionCase>,
}

static FIXTURES: Lazy<MotionFixtures> =
    Lazy::new(|| serde_json::from_str(RAW).expect("parse motion fixtures"));

#[test]
fn motion_golden_fixtures_apply_as_expected() {
    let entity_ty = make_type_id("entity");
    let mut engine: Engine = build_motion_demo_engine();

    for case in &FIXTURES.cases {
        let ent = make_node_id(&case.label);
        let payload = encode_motion_payload(case.pos, case.vel);
        engine.insert_node(
            ent,
            NodeRecord {
                ty: entity_ty,
                payload: Some(payload),
            },
        );

        let tx = engine.begin();
        let res = engine
            .apply(tx, MOTION_RULE_NAME, &ent)
            .unwrap_or_else(|_| panic!("apply motion rule failed for case: {}", case.label));
        assert!(matches!(res, ApplyResult::Applied));
        engine.commit(tx).expect("commit");

        let node = engine.node(&ent).expect("node exists");
        let (pos, vel) =
            decode_motion_payload(node.payload.as_ref().expect("payload")).expect("decode");
        for (i, v) in vel.iter().enumerate() {
            assert_eq!(
                v.to_bits(),
                case.vel[i].to_bits(),
                "[{}] velocity[{}] mismatch: got {:?}, expected {:?}",
                case.label,
                i,
                v,
                case.vel[i]
            );
        }
        for (i, p) in pos.iter().enumerate() {
            assert_eq!(
                p.to_bits(),
                case.expected_pos[i].to_bits(),
                "[{}] position[{}] mismatch: got {:?}, expected {:?}",
                case.label,
                i,
                p,
                case.expected_pos[i]
            );
        }
    }
}

#[test]
fn motion_apply_no_payload_returns_nomatch() {
    let entity_ty = make_type_id("entity");
    let ent = make_node_id("no-payload");
    let mut engine = build_motion_demo_engine();
    engine.insert_node(
        ent,
        NodeRecord {
            ty: entity_ty,
            payload: None,
        },
    );
    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::NoMatch));
}

#[test]
fn motion_apply_invalid_payload_size_returns_nomatch() {
    let entity_ty = make_type_id("entity");
    let ent = make_node_id("bad-payload");
    let mut engine = build_motion_demo_engine();
    let bad = Bytes::from(vec![0u8; 10]);
    engine.insert_node(
        ent,
        NodeRecord {
            ty: entity_ty,
            payload: Some(bad),
        },
    );
    let tx = engine.begin();
    let res = engine.apply(tx, MOTION_RULE_NAME, &ent).expect("apply");
    assert!(matches!(res, ApplyResult::NoMatch));
}
