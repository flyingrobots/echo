#![allow(missing_docs)]
use rmg_core::{
    encode_motion_payload, make_edge_id, make_node_id, make_type_id, GraphStore, NodeRecord,
};

fn snapshot_hash(store: GraphStore, root: rmg_core::NodeId) -> [u8; 32] {
    let engine = rmg_core::Engine::new(store, root);
    engine.snapshot().hash
}

#[test]
fn unreachable_nodes_do_not_affect_hash() {
    // Root world
    let root = make_node_id("root");
    let world_ty = make_type_id("world");

    let mut store_a = GraphStore::default();
    store_a.insert_node(
        root,
        NodeRecord {
            ty: world_ty,
            payload: None,
        },
    );

    let hash_a = snapshot_hash(store_a, root);

    // Add an unreachable entity elsewhere; hash should remain identical.
    let mut store_b = GraphStore::default();
    store_b.insert_node(
        root,
        NodeRecord {
            ty: world_ty,
            payload: None,
        },
    );
    let unreachable = make_node_id("ghost-entity");
    let ent_ty = make_type_id("entity");
    store_b.insert_node(
        unreachable,
        NodeRecord {
            ty: ent_ty,
            payload: Some(encode_motion_payload([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])),
        },
    );

    let hash_b = snapshot_hash(store_b, root);
    assert_eq!(hash_a, hash_b);
}

#[test]
fn reachable_edges_affect_hash() {
    let root = make_node_id("root2");
    let world_ty = make_type_id("world");
    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: world_ty,
            payload: None,
        },
    );

    // Initially only root is reachable; hash0
    let hash0 = snapshot_hash(store.clone(), root);

    // Add a reachable child entity and a typed edge from root -> child
    let child = make_node_id("child");
    let ent_ty = make_type_id("entity");
    let edge_ty = make_type_id("has");
    store.insert_node(
        child,
        NodeRecord {
            ty: ent_ty,
            payload: None,
        },
    );
    store.insert_edge(
        root,
        rmg_core::EdgeRecord {
            id: make_edge_id("root->child"),
            from: root,
            to: child,
            ty: edge_ty,
            payload: None,
        },
    );

    let hash1 = snapshot_hash(store, root);
    assert_ne!(hash0, hash1);
}
