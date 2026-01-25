// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Demo rules for testing: motion update and port reservation.
//!
//! These rules were previously in warp-demo-kit but are now test-only utilities.

use warp_core::{
    decode_motion_atom_payload_q32_32, decode_motion_payload, encode_motion_atom_payload,
    encode_motion_payload, encode_motion_payload_q32_32, make_node_id, make_type_id,
    motion_payload_type_id, pack_port_key, AtomPayload, AttachmentKey, AttachmentSet,
    AttachmentValue, ConflictPolicy, EdgeSet, Engine, Footprint, GraphStore, GraphView, Hash,
    NodeId, NodeKey, NodeRecord, NodeSet, PatternGraph, PortSet, RewriteRule, TickDelta, WarpId,
    WarpOp,
};

// =============================================================================
// Motion Rule
// =============================================================================

/// Rule name constant for the built-in motion update rule.
pub const MOTION_RULE_NAME: &str = "motion/update";

#[cfg(feature = "det_fixed")]
mod motion_scalar_backend {
    use warp_core::math::scalar::DFix64;

    pub(super) type MotionScalar = DFix64;

    pub(super) fn scalar_from_raw(raw: i64) -> MotionScalar {
        MotionScalar::from_raw(raw)
    }

    pub(super) fn scalar_to_raw(value: MotionScalar) -> i64 {
        value.raw()
    }
}

#[cfg(not(feature = "det_fixed"))]
mod motion_scalar_backend {
    use warp_core::math::fixed_q32_32;
    use warp_core::math::scalar::F32Scalar;
    use warp_core::math::Scalar;

    pub(super) type MotionScalar = F32Scalar;

    pub(super) fn scalar_from_raw(raw: i64) -> MotionScalar {
        MotionScalar::from_f32(fixed_q32_32::to_f32(raw))
    }

    pub(super) fn scalar_to_raw(value: MotionScalar) -> i64 {
        fixed_q32_32::from_f32(value.to_f32())
    }
}

use motion_scalar_backend::{scalar_from_raw, scalar_to_raw};

fn motion_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    if view.node(scope).is_none() {
        return;
    }

    let warp_id = view.warp_id();

    let Some(AttachmentValue::Atom(payload)) = view.node_attachment(scope) else {
        return;
    };

    let Some((pos_raw, vel_raw)) = decode_motion_atom_payload_q32_32(payload) else {
        return;
    };

    let mut pos = [
        scalar_from_raw(pos_raw[0]),
        scalar_from_raw(pos_raw[1]),
        scalar_from_raw(pos_raw[2]),
    ];
    let vel = [
        scalar_from_raw(vel_raw[0]),
        scalar_from_raw(vel_raw[1]),
        scalar_from_raw(vel_raw[2]),
    ];

    for i in 0..3 {
        pos[i] = pos[i] + vel[i];
    }

    let new_pos_raw = [
        scalar_to_raw(pos[0]),
        scalar_to_raw(pos[1]),
        scalar_to_raw(pos[2]),
    ];
    let vel_out_raw = [
        scalar_to_raw(vel[0]),
        scalar_to_raw(vel[1]),
        scalar_to_raw(vel[2]),
    ];

    // Build new bytes
    let new_bytes = encode_motion_payload_q32_32(new_pos_raw, vel_out_raw);

    // Phase 5 BOAW: only emit delta ops, no direct mutation
    if payload.bytes != new_bytes {
        let key = AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: *scope,
        });
        delta.push(WarpOp::SetAttachment {
            key,
            value: Some(AttachmentValue::Atom(AtomPayload {
                type_id: motion_payload_type_id(),
                bytes: new_bytes,
            })),
        });
    }
}

fn motion_matcher(view: GraphView<'_>, scope: &NodeId) -> bool {
    matches!(
        view.node_attachment(scope),
        Some(AttachmentValue::Atom(payload)) if decode_motion_atom_payload_q32_32(payload).is_some()
    )
}

fn motion_rule_id() -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(MOTION_RULE_NAME.as_bytes());
    hasher.finalize().into()
}

fn base_scope_footprint(
    view: GraphView<'_>,
    scope: &NodeId,
) -> (
    WarpId,
    NodeSet,
    AttachmentSet,
    AttachmentSet,
    Option<AttachmentKey>,
) {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_read = AttachmentSet::default();
    let mut a_write = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, *scope);
    let mut attachment_key = None;
    if view.node(scope).is_some() {
        let key = AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: *scope,
        });
        a_read.insert(key);
        a_write.insert(key);
        attachment_key = Some(key);
    }
    (warp_id, n_read, a_read, a_write, attachment_key)
}

fn compute_motion_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let (_warp_id, n_read, a_read, a_write, _key) = base_scope_footprint(view, scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read,
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

/// Returns a rewrite rule that updates entity positions based on velocity.
#[must_use]
pub fn motion_rule() -> RewriteRule {
    RewriteRule {
        id: motion_rule_id(),
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

/// Constructs a demo Engine with a world-root node and motion rule pre-registered.
#[must_use]
#[allow(clippy::expect_used)]
pub fn build_motion_demo_engine() -> Engine {
    let mut store = GraphStore::default();
    let root_id = make_node_id("world-root");
    let root_type = make_type_id("world");
    store.insert_node(root_id, NodeRecord { ty: root_type });

    let mut engine = Engine::new(store, root_id);
    engine
        .register_rule(motion_rule())
        .expect("motion rule should register successfully in fresh engine");
    engine
}

// =============================================================================
// Port Rule
// =============================================================================

/// Rule name constant for the demo port reservation rule.
pub const PORT_RULE_NAME: &str = "demo/port_nop";

fn port_matcher(_: GraphView<'_>, _: &NodeId) -> bool {
    true
}

fn port_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    if view.node(scope).is_none() {
        return;
    }

    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: *scope,
    });

    let Some(attachment) = view.node_attachment(scope) else {
        let pos = [1.0, 0.0, 0.0];
        let vel = [0.0, 0.0, 0.0];
        let new_value = Some(AttachmentValue::Atom(encode_motion_atom_payload(pos, vel)));

        // Phase 5 BOAW: only emit delta ops, no direct mutation
        delta.push(WarpOp::SetAttachment {
            key,
            value: new_value,
        });

        return;
    };

    let AttachmentValue::Atom(payload) = attachment else {
        return;
    };
    if payload.type_id != motion_payload_type_id() {
        return;
    }
    if let Some((mut pos, vel)) = decode_motion_payload(&payload.bytes) {
        pos[0] += 1.0;
        let new_bytes = encode_motion_payload(pos, vel);

        // Phase 5 BOAW: only emit delta ops, no direct mutation.
        //
        // Guard emission by byte equality so no-op rewrites don't bloat the delta
        // stream (e.g. f32 increments that quantize to the same Q32.32 value).
        if payload.bytes != new_bytes {
            let new_value = Some(AttachmentValue::Atom(AtomPayload {
                type_id: motion_payload_type_id(),
                bytes: new_bytes,
            }));
            delta.push(WarpOp::SetAttachment {
                key,
                value: new_value,
            });
        }
    }
}

fn compute_port_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let (warp_id, n_read, a_read, a_write, attachment_key) = base_scope_footprint(view, scope);
    let mut n_write = NodeSet::default();
    let mut b_in = PortSet::default();
    if attachment_key.is_some() {
        n_write.insert_with_warp(warp_id, *scope);
        b_in.insert(warp_id, pack_port_key(scope, 0, true));
    }
    Footprint {
        n_read,
        n_write,
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read,
        a_write,
        b_in,
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

/// Returns a demo rewrite rule that reserves a boundary input port.
#[must_use]
pub fn port_rule() -> RewriteRule {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(PORT_RULE_NAME.as_bytes());
    let id: Hash = hasher.finalize().into();
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
#[allow(clippy::expect_used)]
pub fn build_port_demo_engine() -> Engine {
    let mut store = GraphStore::default();
    let root_id = make_node_id("world-root-ports");
    let root_type = make_type_id("world");
    store.insert_node(root_id, NodeRecord { ty: root_type });
    let mut engine = Engine::new(store, root_id);
    engine
        .register_rule(port_rule())
        .expect("port rule should register successfully in fresh engine");
    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_executor_skips_emitting_when_quantized_bytes_do_not_change() {
        // Choose a value where `pos[0] += 1.0` is a no-op in f32 (ulp >= 2),
        // so the canonical Q32.32 encoding remains unchanged.
        let pos = [16_777_216.0, 0.0, 0.0];
        let vel = [0.0, 0.0, 0.0];

        let mut store = GraphStore::default();
        let node_id = make_node_id("port/noop");
        store.insert_node(
            node_id,
            NodeRecord {
                ty: make_type_id("test"),
            },
        );
        store.set_node_attachment(
            node_id,
            Some(AttachmentValue::Atom(encode_motion_atom_payload(pos, vel))),
        );

        let view = GraphView::new(&store);
        let mut delta = TickDelta::new();
        port_executor(view, &node_id, &mut delta);

        assert!(delta.is_empty(), "no-op update should not emit a delta op");
    }

    #[test]
    fn compute_port_footprint_always_reads_scope_node() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = make_node_id("port/missing");
        let footprint = compute_port_footprint(view, &scope);
        let expected = NodeKey {
            warp_id: view.warp_id(),
            local_id: scope,
        };

        assert!(
            footprint.n_read.iter().any(|key| *key == expected),
            "scope node read must be declared even when node is missing"
        );
        assert!(
            footprint.n_write.is_empty(),
            "missing node should not be written"
        );
        assert!(
            footprint.a_read.is_empty(),
            "missing node should not declare attachment read"
        );
        assert!(
            footprint.a_write.is_empty(),
            "missing node should not declare attachment write"
        );
        assert!(
            footprint.b_in.is_empty(),
            "missing node should not declare boundary input"
        );
    }

    #[test]
    fn compute_motion_footprint_always_reads_scope_node() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = make_node_id("motion/missing");
        let footprint = compute_motion_footprint(view, &scope);
        let expected = NodeKey {
            warp_id: view.warp_id(),
            local_id: scope,
        };

        assert!(
            footprint.n_read.iter().any(|key| *key == expected),
            "scope node read must be declared even when node is missing"
        );
        assert!(
            footprint.n_write.is_empty(),
            "missing node should not be written"
        );
        assert!(
            footprint.a_read.is_empty(),
            "missing node should not declare attachment read"
        );
        assert!(
            footprint.a_write.is_empty(),
            "missing node should not declare attachment write"
        );
        assert!(
            footprint.b_in.is_empty(),
            "missing node should not declare boundary input"
        );
        assert!(
            footprint.b_out.is_empty(),
            "missing node should not declare boundary output"
        );
    }
}
