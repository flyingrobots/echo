// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use echo_dry_tests::{motion_rule, MOTION_RULE_NAME};
use warp_core::{
    encode_motion_atom_payload, make_edge_id, make_node_id, make_type_id, AttachmentValue,
    EdgeRecord, Engine, GraphStore, NodeRecord,
};

#[test]
fn n_permutation_commute_n3_and_n4() {
    for &n in &[3usize, 4usize] {
        // Build initial graph: root + n entities with unique velocities.
        let root = make_node_id("world-root-perm");
        let world_ty = make_type_id("world");
        let ent_ty = make_type_id("entity");
        let mut store = GraphStore::default();
        store.insert_node(root, NodeRecord { ty: world_ty });
        let mut scopes = Vec::new();
        for i in 0..n {
            let id = make_node_id(&format!("entity-{i}"));
            let v = match i {
                0 => [1.0, 0.0, 0.0],
                1 => [0.0, 1.0, 0.0],
                2 => [0.0, 0.0, 1.0],
                3 => [1.0, 1.0, 0.0],
                _ => unreachable!(),
            };
            store.insert_node(id, NodeRecord { ty: ent_ty });
            store.set_node_attachment(
                id,
                Some(AttachmentValue::Atom(encode_motion_atom_payload(
                    [0.0, 0.0, 0.0],
                    v,
                ))),
            );
            // Connect entity to root so snapshot reachability includes it.
            let edge = EdgeRecord {
                id: make_edge_id(&format!("root-to-entity-{i}")),
                from: root,
                to: id,
                ty: make_type_id("contains"),
            };
            store.insert_edge(root, edge);
            scopes.push(id);
        }

        // Enumerate a few permutations deterministically (not all for n=4 to keep runtime low).
        let perms: Vec<Vec<usize>> = match n {
            3 => vec![vec![0, 1, 2], vec![2, 1, 0], vec![1, 2, 0], vec![0, 2, 1]],
            4 => vec![vec![0, 1, 2, 3], vec![3, 2, 1, 0], vec![1, 3, 0, 2]],
            _ => unreachable!(),
        };

        let mut baseline: Option<[u8; 32]> = None;
        for p in perms {
            let mut engine = Engine::new(store.clone(), root);
            engine.register_rule(motion_rule()).unwrap();
            let tx = engine.begin();
            for &idx in &p {
                engine.apply(tx, MOTION_RULE_NAME, &scopes[idx]).unwrap();
            }
            let snapshot = engine.commit(tx).unwrap();
            let h = snapshot.hash;
            if let Some(b) = baseline {
                assert_eq!(b, h, "commutation failed for n={n} perm={p:?}");
            } else {
                baseline = Some(h);
            }
        }
    }
}
