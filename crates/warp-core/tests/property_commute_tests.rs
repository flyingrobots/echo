// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use echo_dry_tests::{motion_rule, MOTION_RULE_NAME};
use warp_core::{
    encode_motion_atom_payload, make_edge_id, make_node_id, make_type_id, AttachmentValue,
    EdgeRecord, Engine, GraphStore, NodeRecord,
};

#[test]
fn independent_motion_rewrites_commute_on_distinct_nodes() {
    // Build initial store with root and two entities that each have motion payloads.
    let root = make_node_id("world-root-commute");
    let world_ty = make_type_id("world");
    let ent_ty = make_type_id("entity");
    let a = make_node_id("entity-a");
    let b = make_node_id("entity-b");

    let mut store1 = GraphStore::default();
    store1.insert_node(root, NodeRecord { ty: world_ty });
    store1.insert_node(a, NodeRecord { ty: ent_ty });
    store1.set_node_attachment(
        a,
        Some(AttachmentValue::Atom(encode_motion_atom_payload(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ))),
    );
    store1.insert_node(b, NodeRecord { ty: ent_ty });
    store1.set_node_attachment(
        b,
        Some(AttachmentValue::Atom(encode_motion_atom_payload(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ))),
    );
    // Make entities reachable from root via edges so snapshots include them.
    let edge_ty = make_type_id("edge");
    store1.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("root->a"),
            from: root,
            to: a,
            ty: edge_ty,
        },
    );
    store1.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("root->b"),
            from: root,
            to: b,
            ty: edge_ty,
        },
    );
    let store2 = store1.clone();

    // Order 1: apply to A then B
    let mut engine1 = Engine::new(store1, root);
    engine1.register_rule(motion_rule()).unwrap();
    let tx1 = engine1.begin();
    engine1.apply(tx1, MOTION_RULE_NAME, &a).unwrap();
    engine1.apply(tx1, MOTION_RULE_NAME, &b).unwrap();
    let snapshot1 = engine1.commit(tx1).unwrap();
    let h1 = snapshot1.hash;

    // Order 2: apply to B then A
    let mut engine2 = Engine::new(store2, root);
    engine2.register_rule(motion_rule()).unwrap();
    let tx2 = engine2.begin();
    engine2.apply(tx2, MOTION_RULE_NAME, &b).unwrap();
    engine2.apply(tx2, MOTION_RULE_NAME, &a).unwrap();
    let snapshot2 = engine2.commit(tx2).unwrap();
    let h2 = snapshot2.hash;

    assert_eq!(h1, h2, "independent rewrites must commute");
}
