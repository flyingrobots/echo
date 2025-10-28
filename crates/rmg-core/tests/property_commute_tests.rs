#![allow(missing_docs)]
use rmg_core::{encode_motion_payload, make_node_id, make_type_id, GraphStore, NodeRecord};
mod common;
use common::snapshot_hash_of;

#[test]
fn independent_motion_rewrites_commute_on_distinct_nodes() {
    // Build initial store with root and two entities that each have motion payloads.
    let root = make_node_id("world-root-commute");
    let world_ty = make_type_id("world");
    let ent_ty = make_type_id("entity");
    let a = make_node_id("entity-a");
    let b = make_node_id("entity-b");

    let mut store1 = GraphStore::default();
    store1.insert_node(
        root,
        NodeRecord {
            ty: world_ty,
            payload: None,
        },
    );
    store1.insert_node(
        a,
        NodeRecord {
            ty: ent_ty,
            payload: Some(encode_motion_payload([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])),
        },
    );
    store1.insert_node(
        b,
        NodeRecord {
            ty: ent_ty,
            payload: Some(encode_motion_payload([0.0, 0.0, 0.0], [0.0, 1.0, 0.0])),
        },
    );
    let mut store2 = store1.clone();

    let rule = rmg_core::motion_rule();

    // Order 1: apply to A then B
    (rule.executor)(&mut store1, &a);
    (rule.executor)(&mut store1, &b);
    let h1 = snapshot_hash_of(store1, root);

    // Order 2: apply to B then A
    (rule.executor)(&mut store2, &b);
    (rule.executor)(&mut store2, &a);
    let h2 = snapshot_hash_of(store2, root);

    assert_eq!(h1, h2, "independent rewrites must commute");
}
