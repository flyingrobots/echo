// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Demo rules for testing: motion update and port reservation.
//!
//! These rules were previously in warp-demo-kit but are now test-only utilities.

use warp_core::{
    decode_motion_atom_payload_q32_32, decode_motion_payload, encode_motion_atom_payload,
    encode_motion_payload, encode_motion_payload_q32_32, make_node_id, make_type_id,
    motion_payload_type_id, pack_port_key, AttachmentKey, AttachmentSet, AttachmentValue,
    ConflictPolicy, Engine, Footprint, GraphStore, Hash, IdSet, NodeId, NodeKey, NodeRecord,
    PatternGraph, PortSet, RewriteRule, TickDelta,
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

fn motion_executor(store: &mut GraphStore, scope: &NodeId, _delta: &mut TickDelta) {
    if store.node(scope).is_none() {
        return;
    }
    let Some(AttachmentValue::Atom(payload)) = store.node_attachment_mut(scope) else {
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

    payload.type_id = motion_payload_type_id();
    payload.bytes = encode_motion_payload_q32_32(new_pos_raw, vel_out_raw);
}

fn motion_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    matches!(
        store.node_attachment(scope),
        Some(AttachmentValue::Atom(payload)) if decode_motion_atom_payload_q32_32(payload).is_some()
    )
}

fn motion_rule_id() -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(MOTION_RULE_NAME.as_bytes());
    hasher.finalize().into()
}

fn compute_motion_footprint(store: &GraphStore, scope: &NodeId) -> Footprint {
    let mut a_write = AttachmentSet::default();
    if store.node(scope).is_some() {
        a_write.insert(AttachmentKey::node_alpha(NodeKey {
            warp_id: store.warp_id(),
            local_id: *scope,
        }));
    }
    Footprint {
        n_read: IdSet::default(),
        n_write: IdSet::default(),
        e_read: IdSet::default(),
        e_write: IdSet::default(),
        a_read: AttachmentSet::default(),
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

/// Public identifier for the port demo rule.
pub const PORT_RULE_NAME: &str = "demo/port_nop";

fn port_matcher(_: &GraphStore, _: &NodeId) -> bool {
    true
}

fn port_executor(store: &mut GraphStore, scope: &NodeId, _delta: &mut TickDelta) {
    if store.node(scope).is_none() {
        return;
    }

    let Some(attachment) = store.node_attachment_mut(scope) else {
        let pos = [1.0, 0.0, 0.0];
        let vel = [0.0, 0.0, 0.0];
        store.set_node_attachment(
            *scope,
            Some(AttachmentValue::Atom(encode_motion_atom_payload(pos, vel))),
        );
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
        payload.bytes = encode_motion_payload(pos, vel);
    }
}

fn compute_port_footprint(store: &GraphStore, scope: &NodeId) -> Footprint {
    let mut n_write = IdSet::default();
    let mut a_write = AttachmentSet::default();
    let mut b_in = PortSet::default();
    if store.node(scope).is_some() {
        n_write.insert_node(scope);
        a_write.insert(AttachmentKey::node_alpha(NodeKey {
            warp_id: store.warp_id(),
            local_id: *scope,
        }));
        b_in.insert(pack_port_key(scope, 0, true));
    }
    Footprint {
        n_read: IdSet::default(),
        n_write,
        e_read: IdSet::default(),
        e_write: IdSet::default(),
        a_read: AttachmentSet::default(),
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
