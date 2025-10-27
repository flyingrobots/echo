//! Demo motion rule: advances position by velocity stored in payload.
use blake3::Hasher;

use crate::engine_impl::Engine;
use crate::graph::GraphStore;
use crate::ident::{make_node_id, make_type_id, Hash, NodeId};
use crate::payload::{decode_motion_payload, encode_motion_payload};
use crate::record::NodeRecord;
use crate::rule::{PatternGraph, RewriteRule};

/// Public identifier for the built-in motion update rule.
pub const MOTION_RULE_NAME: &str = "motion/update";

fn motion_executor(store: &mut GraphStore, scope: &NodeId) {
    if let Some(node) = store.node_mut(scope) {
        if let Some(payload) = &mut node.payload {
            if let Some((mut pos, vel)) = decode_motion_payload(payload) {
                pos[0] += vel[0];
                pos[1] += vel[1];
                pos[2] += vel[2];
                *payload = encode_motion_payload(pos, vel);
            }
        }
    }
}

fn motion_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    store
        .node(scope)
        .and_then(|n| n.payload.as_ref())
        .and_then(decode_motion_payload)
        .is_some()
}

/// Demo rule used by tests: move an entity by its velocity.
#[must_use]
pub fn motion_rule() -> RewriteRule {
    let mut hasher = Hasher::new();
    hasher.update(MOTION_RULE_NAME.as_bytes());
    let id: Hash = hasher.finalize().into();
    RewriteRule {
        id,
        name: MOTION_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: motion_matcher,
        executor: motion_executor,
    }
}

/// Builds an engine with the default world root and the motion rule registered.
#[must_use]
pub fn build_motion_demo_engine() -> Engine {
    let mut store = GraphStore::default();
    let root_id = make_node_id("world-root");
    let root_type = make_type_id("world");
    store.insert_node(
        root_id,
        NodeRecord {
            ty: root_type,
            payload: None,
        },
    );

    let mut engine = Engine::new(store, root_id);
    engine.register_rule(motion_rule());
    engine
}
