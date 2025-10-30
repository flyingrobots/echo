#![allow(missing_docs)]
use once_cell::sync::Lazy;
use serde::Deserialize;

use rmg_core::{
    decode_motion_payload, encode_motion_payload, make_node_id, make_type_id, ApplyResult, Engine,
    GraphStore, NodeRecord, MOTION_RULE_NAME,
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
    for case in &FIXTURES.cases {
        let ent = make_node_id(&case.label);
        // Create a fresh engine and insert entity with payload
        let mut store = GraphStore::default();
        let payload = encode_motion_payload(case.pos, case.vel);
        store.insert_node(
            ent,
            NodeRecord {
                ty: entity_ty,
                payload: Some(payload),
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

        // Verify payload bytes decode to expected values
        let node = engine.node(&ent).expect("node exists");
        let (pos, vel) =
            decode_motion_payload(node.payload.as_ref().expect("payload")).expect("decode");
        for i in 0..3 {
            assert_eq!(
                vel[i].to_bits(),
                case.vel[i].to_bits(),
                "vel component {}",
                i
            );
            assert_eq!(
                pos[i].to_bits(),
                case.expected_pos[i].to_bits(),
                "pos component {}",
                i
            );
        }
    }
}
