// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use warp_core::{decode_motion_payload, make_node_id, make_type_id, NodeRecord};

#[test]
fn reserve_gate_aborts_second_on_port_conflict() {
    // Engine with a single entity; register the port rule; apply it twice on same scope in one tx.
    let mut engine = warp_core::demo::ports::build_port_demo_engine();

    // Create an entity node under root that we’ll target.
    let entity = make_node_id("reserve-entity");
    let entity_ty = make_type_id("entity");
    engine.insert_node(
        entity,
        NodeRecord {
            ty: entity_ty,
            payload: None,
        },
    );

    let tx = engine.begin();
    let _ = engine.apply(tx, warp_core::demo::ports::PORT_RULE_NAME, &entity);
    let _ = engine.apply(tx, warp_core::demo::ports::PORT_RULE_NAME, &entity);
    let _snap = engine.commit(tx).expect("commit");

    // Exactly one executor should have run: pos.x == 1.0
    let (pos, _vel) = engine
        .node(&entity)
        .unwrap()
        .payload
        .as_ref()
        .and_then(decode_motion_payload)
        .expect("payload present");
    assert!(
        (pos[0] - 1.0).abs() < 1e-6,
        "expected exactly one reservation to succeed"
    );
}
