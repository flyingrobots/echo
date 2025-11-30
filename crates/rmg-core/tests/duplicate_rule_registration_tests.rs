// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(missing_docs)]
use blake3::Hasher;
use rmg_core::{
    make_node_id, make_type_id, ConflictPolicy, Engine, GraphStore, NodeRecord, PatternGraph,
    RewriteRule,
};

fn noop_match(_: &GraphStore, _: &rmg_core::NodeId) -> bool {
    true
}
fn noop_exec(_: &mut GraphStore, _: &rmg_core::NodeId) {}
fn noop_fp(_: &GraphStore, _: &rmg_core::NodeId) -> rmg_core::Footprint {
    rmg_core::Footprint::default()
}

#[test]
fn registering_duplicate_rule_name_is_rejected() {
    let mut store = GraphStore::default();
    let root = make_node_id("dup-root");
    let world_ty = make_type_id("world");
    store.insert_node(
        root,
        NodeRecord {
            ty: world_ty,
            payload: None,
        },
    );
    let mut engine = Engine::new(store, root);
    engine.register_rule(rmg_core::motion_rule()).unwrap();
    let err = engine.register_rule(rmg_core::motion_rule()).unwrap_err();
    match err {
        rmg_core::EngineError::DuplicateRuleName(name) => {
            assert_eq!(name, rmg_core::MOTION_RULE_NAME)
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn registering_duplicate_rule_id_is_rejected() {
    let mut store = GraphStore::default();
    let root = make_node_id("dup-root2");
    let world_ty = make_type_id("world");
    store.insert_node(
        root,
        NodeRecord {
            ty: world_ty,
            payload: None,
        },
    );
    let mut engine = Engine::new(store, root);
    engine.register_rule(rmg_core::motion_rule()).unwrap();

    // Compute the same family id used by the motion rule.
    let mut hasher = Hasher::new();
    hasher.update(b"rule:");
    hasher.update(rmg_core::MOTION_RULE_NAME.as_bytes());
    let same_id: rmg_core::Hash = hasher.finalize().into();

    let duplicate = RewriteRule {
        id: same_id,
        name: "motion/duplicate",
        left: PatternGraph { nodes: vec![] },
        matcher: noop_match,
        executor: noop_exec,
        compute_footprint: noop_fp,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let err = engine.register_rule(duplicate).unwrap_err();
    match err {
        rmg_core::EngineError::DuplicateRuleId(id) => assert_eq!(id, same_id),
        other => panic!("unexpected error: {other:?}"),
    }
}
