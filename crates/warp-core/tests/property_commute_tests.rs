// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use echo_dry_tests::motion_rule;
use warp_core::{
    encode_motion_atom_payload, make_node_id, make_type_id, AttachmentValue, GraphStore,
    NodeRecord, TickDelta,
};
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
    use warp_core::{make_edge_id, EdgeRecord};
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
    let mut store2 = store1.clone();

    let rule = motion_rule();

    // Order 1: apply to A then B
    let mut delta = TickDelta::new();
    (rule.executor)(&mut store1, &a, &mut delta);
    (rule.executor)(&mut store1, &b, &mut delta);
    let h1 = snapshot_hash_of(store1, root);

    // Order 2: apply to B then A
    let mut delta = TickDelta::new();
    (rule.executor)(&mut store2, &b, &mut delta);
    (rule.executor)(&mut store2, &a, &mut delta);
    let h2 = snapshot_hash_of(store2, root);

    assert_eq!(h1, h2, "independent rewrites must commute");
}
