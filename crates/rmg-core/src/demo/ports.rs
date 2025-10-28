//! Demo rule that reserves a boundary input port, used to exercise the
//! reservation gate and independence checks.

use crate::engine_impl::Engine;
use crate::footprint::{pack_port_key, Footprint, IdSet, PortSet};
use crate::graph::GraphStore;
use crate::ident::{make_node_id, make_type_id, Hash, NodeId};
use crate::payload::{decode_motion_payload, encode_motion_payload};
use crate::record::NodeRecord;
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};

/// Public identifier for the port demo rule.
pub const PORT_RULE_NAME: &str = "demo/port_nop";

fn port_matcher(_: &GraphStore, _: &NodeId) -> bool {
    true
}

fn port_executor(store: &mut GraphStore, scope: &NodeId) {
    if let Some(node) = store.node_mut(scope) {
        // Use motion payload layout; increment pos.x by 1.0
        if let Some(bytes) = &mut node.payload {
            if let Some((mut pos, vel)) = decode_motion_payload(bytes) {
                pos[0] += 1.0;
                *bytes = encode_motion_payload(pos, vel);
            }
        } else {
            let pos = [1.0, 0.0, 0.0];
            let vel = [0.0, 0.0, 0.0];
            node.payload = Some(encode_motion_payload(pos, vel));
        }
    }
}

fn compute_port_footprint(_: &GraphStore, scope: &NodeId) -> Footprint {
    let mut n_write = IdSet::default();
    n_write.insert_node(scope);
    let mut b_in = PortSet::default();
    b_in.insert(pack_port_key(scope, 0, true));
    Footprint {
        n_read: IdSet::default(),
        n_write,
        e_read: IdSet::default(),
        e_write: IdSet::default(),
        b_in,
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

/// Returns a demo rewrite rule that reserves a boundary input port.
///
/// This rule always matches and increments the x component of the scoped
/// node's motion payload by 1.0 (or initializes to `[1.0, 0.0, 0.0]` if
/// absent). Its footprint reserves a single boundary input port (port 0,
/// direction=in) on the scoped node, used to test port-based independence
/// checks.
///
/// Register with [`Engine::register_rule`], then apply with [`Engine::apply`]
/// using [`PORT_RULE_NAME`]. Returns a [`RewriteRule`] with a runtime-computed
/// id (BLAKE3 of the name for the spike), empty pattern, and
/// [`ConflictPolicy::Abort`].
#[must_use]
pub fn port_rule() -> RewriteRule {
    // Family id will be generated later via build.rs when promoted to a stable demo.
    // For the spike, derive from the name at runtime (cost is irrelevant in tests).
    let id: Hash = blake3::hash(PORT_RULE_NAME.as_bytes()).into();
    RewriteRule {
        id,
        name: PORT_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: port_matcher,
        executor: port_executor,
        compute_footprint: compute_port_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Builds an engine with a world root for port-rule tests.
#[must_use]
pub fn build_port_demo_engine() -> Engine {
    let mut store = GraphStore::default();
    let root_id = make_node_id("world-root-ports");
    let root_type = make_type_id("world");
    store.insert_node(
        root_id,
        NodeRecord {
            ty: root_type,
            payload: None,
        },
    );
    Engine::new(store, root_id)
}
