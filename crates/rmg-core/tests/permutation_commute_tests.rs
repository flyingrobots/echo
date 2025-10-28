#![allow(missing_docs)]
use rmg_core::{encode_motion_payload, make_node_id, make_type_id, GraphStore, NodeRecord};
mod common;
use common::snapshot_hash_of;

#[test]
fn n_permutation_commute_n3_and_n4() {
    for &n in &[3usize, 4usize] {
        // Build initial graph: root + n entities with unique velocities.
        let root = make_node_id("world-root-perm");
        let world_ty = make_type_id("world");
        let ent_ty = make_type_id("entity");
        let mut store = GraphStore::default();
        store.insert_node(
            root,
            NodeRecord {
                ty: world_ty,
                payload: None,
            },
        );
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
            store.insert_node(
                id,
                NodeRecord {
                    ty: ent_ty,
                    payload: Some(encode_motion_payload([0.0, 0.0, 0.0], v)),
                },
            );
            scopes.push(id);
        }
        let rule = rmg_core::motion_rule();

        // Enumerate a few permutations deterministically (not all for n=4 to keep runtime low).
        let perms: Vec<Vec<usize>> = match n {
            3 => vec![vec![0, 1, 2], vec![2, 1, 0], vec![1, 2, 0], vec![0, 2, 1]],
            4 => vec![vec![0, 1, 2, 3], vec![3, 2, 1, 0], vec![1, 3, 0, 2]],
            _ => unreachable!(),
        };

        let mut baseline: Option<[u8; 32]> = None;
        for p in perms {
            let mut s = store.clone();
            for &idx in &p {
                (rule.executor)(&mut s, &scopes[idx]);
            }
            let h = snapshot_hash_of(s, root);
            if let Some(b) = baseline {
                assert_eq!(b, h, "commutation failed for n={n} perm={p:?}");
            } else {
                baseline = Some(h);
            }
        }
    }
}
