//! Demo motion rule: advances position by velocity stored in payload.

use crate::engine_impl::Engine;
use crate::footprint::{Footprint, IdSet};
use crate::graph::GraphStore;
use crate::ident::{make_node_id, make_type_id, Hash, NodeId};
use crate::payload::{decode_motion_payload, encode_motion_payload};
use crate::record::NodeRecord;
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};
// Build-time generated canonical ids (domain-separated).
include!(concat!(env!("OUT_DIR"), "/rule_ids.rs"));

/// Rule name constant for the built-in motion update rule.
///
/// Pass this name to [`Engine::apply`] to execute the motion update rule,
/// which advances an entity's position by its velocity. Operates on nodes
/// whose payload is a valid 24-byte motion encoding (position + velocity as
/// 6 × f32 little-endian).
///
/// Example usage (in tests):
/// `engine.apply(MOTION_RULE_NAME, &entity_id)?;`
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

/// Deterministic rule id bytes for `rule:motion/update`.
const MOTION_RULE_ID: Hash = MOTION_UPDATE_FAMILY_ID;

/// Returns a rewrite rule that updates entity positions based on velocity.
///
/// This rule matches any node containing a valid 24-byte motion payload
/// (position + velocity encoded as 6 × f32 little-endian) and updates the
/// position by adding the velocity component-wise.
///
/// Register this rule with [`Engine::register_rule`], then apply it with
/// [`Engine::apply`] using [`MOTION_RULE_NAME`].
///
/// Returns a [`RewriteRule`] with deterministic id, empty pattern (relies on
/// the matcher), and the motion update executor.
#[must_use]
pub fn motion_rule() -> RewriteRule {
    RewriteRule {
        id: MOTION_RULE_ID,
        name: MOTION_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: motion_matcher,
        executor: motion_executor,
        compute_footprint: compute_motion_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn compute_motion_footprint(store: &GraphStore, scope: &NodeId) -> Footprint {
    // Motion updates the payload on the scoped node only (write), no edges/ports.
    let mut n_write = IdSet::default();
    if store.node(scope).is_some() {
        n_write.insert_node(scope);
    }
    Footprint {
        n_read: IdSet::default(),
        n_write,
        e_read: IdSet::default(),
        e_write: IdSet::default(),
        b_in: crate::footprint::PortSet::default(),
        b_out: crate::footprint::PortSet::default(),
        factor_mask: 0,
    }
}

/// Constructs a demo [`Engine`] with a world-root node and motion rule pre-registered.
///
/// Creates a [`GraphStore`] with a single root node (id: "world-root", type:
/// "world"), initializes an [`Engine`] with that root, and registers the
/// [`motion_rule`]. Ready for immediate use in tests and demos.
///
/// Returns an [`Engine`] with the motion rule registered and an empty
/// world‑root node.
///
/// # Panics
/// Panics if rule registration fails (should not happen in a fresh engine).
#[must_use]
#[allow(clippy::expect_used)]
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
    engine
        .register_rule(motion_rule())
        .expect("motion rule should register successfully in fresh engine");
    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_rule_id_matches_domain_separated_name() {
        // Our build.rs generates the family id using a domain separator:
        // blake3("rule:" ++ MOTION_RULE_NAME)
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"rule:");
        hasher.update(MOTION_RULE_NAME.as_bytes());
        let expected: Hash = hasher.finalize().into();
        assert_eq!(
            MOTION_RULE_ID, expected,
            "MOTION_RULE_ID must equal blake3(\"rule:\" ++ MOTION_RULE_NAME)"
        );
    }
}
