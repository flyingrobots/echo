// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Demo motion rule: advances position by velocity stored in payload.

use crate::attachment::AttachmentValue;
use crate::engine_impl::Engine;
use crate::footprint::{AttachmentSet, Footprint, IdSet, PortSet};
use crate::graph::GraphStore;
use crate::ident::{make_node_id, make_type_id, Hash, NodeId};
use crate::payload::{
    decode_motion_atom_payload, decode_motion_atom_payload_q32_32, encode_motion_payload_q32_32,
    motion_payload_type_id,
};
use crate::record::NodeRecord;
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};
// Build-time generated canonical ids (domain-separated).
include!(concat!(env!("OUT_DIR"), "/rule_ids.rs"));

/// Rule name constant for the built-in motion update rule.
///
/// Pass this name to [`Engine::apply`] to execute the motion update rule,
/// which advances an entity's position by its velocity. Operates on nodes
/// whose payload is a valid motion encoding.
///
/// Canonical payload encoding is v2:
/// - 6 × `i64` Q32.32 little-endian (48 bytes).
/// - Legacy v0 decoding is supported for compatibility:
///   6 × `f32` little-endian (24 bytes).
///
/// Example usage (in tests):
/// ```ignore
/// let mut engine = build_motion_demo_engine();
/// let entity_id = make_node_id("entity");
/// // ... insert entity and payload ...
/// let tx = engine.begin();
/// engine.apply(tx, MOTION_RULE_NAME, &entity_id)?;
/// ```
pub const MOTION_RULE_NAME: &str = "motion/update";

#[cfg(feature = "det_fixed")]
mod motion_scalar_backend {
    use crate::math::scalar::DFix64;

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
    use crate::math::fixed_q32_32;
    use crate::math::scalar::F32Scalar;
    use crate::math::Scalar;

    pub(super) type MotionScalar = F32Scalar;

    pub(super) fn scalar_from_raw(raw: i64) -> MotionScalar {
        MotionScalar::from_f32(fixed_q32_32::to_f32(raw))
    }

    pub(super) fn scalar_to_raw(value: MotionScalar) -> i64 {
        fixed_q32_32::from_f32(value.to_f32())
    }
}

use motion_scalar_backend::{scalar_from_raw, scalar_to_raw};

fn motion_executor(store: &mut GraphStore, scope: &NodeId) {
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

    // Always upgrade to the canonical v2 payload encoding on write.
    payload.type_id = motion_payload_type_id();
    payload.bytes = encode_motion_payload_q32_32(new_pos_raw, vel_out_raw);
}

fn motion_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    matches!(
        store.node_attachment(scope),
        Some(AttachmentValue::Atom(payload)) if decode_motion_atom_payload(payload).is_some()
    )
}

/// Deterministic rule id bytes for `rule:motion/update`.
const MOTION_RULE_ID: Hash = MOTION_UPDATE_FAMILY_ID;

/// Returns a rewrite rule that updates entity positions based on velocity.
///
/// This rule matches any node containing a valid motion payload and updates
/// the position by adding the velocity component-wise under deterministic
/// scalar semantics.
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
    // Motion updates only the node's attachment payload (α plane).
    let mut a_write = AttachmentSet::default();
    if store.node(scope).is_some() {
        a_write.insert(crate::AttachmentKey::node_alpha(crate::NodeKey {
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
    store.insert_node(root_id, NodeRecord { ty: root_type });

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

    #[test]
    fn motion_executor_updates_position_and_bytes() {
        let mut store = GraphStore::default();
        let ent = make_node_id("entity-motion-bytes");
        let ty = make_type_id("entity");
        let pos = [10.0, -2.0, 3.5];
        let vel = [0.125, 2.0, -1.5];
        let payload = crate::encode_motion_atom_payload(pos, vel);
        store.insert_node(ent, NodeRecord { ty });
        store.set_node_attachment(ent, Some(AttachmentValue::Atom(payload)));

        // Run executor directly and validate position math and encoded bytes.
        motion_executor(&mut store, &ent);
        let Some(_rec) = store.node(&ent) else {
            unreachable!("entity present");
        };
        let Some(AttachmentValue::Atom(bytes)) = store.node_attachment(&ent) else {
            unreachable!("payload present");
        };
        let Some((new_pos, new_vel)) = decode_motion_atom_payload(bytes) else {
            unreachable!("payload decode");
        };
        // Compare component-wise using exact bit equality for deterministic values.
        for i in 0..3 {
            assert_eq!(new_vel[i].to_bits(), vel[i].to_bits());
            let expected = (pos[i] + vel[i]).to_bits();
            assert_eq!(new_pos[i].to_bits(), expected);
        }
        // Encoding round-trip should match re-encoding of updated values exactly.
        assert_eq!(bytes.type_id, motion_payload_type_id());
        let expected_bytes = crate::encode_motion_payload(new_pos, new_vel);
        let Some(AttachmentValue::Atom(bytes)) = store.node_attachment(&ent) else {
            unreachable!("payload present after executor");
        };
        assert_eq!(bytes.bytes, expected_bytes);
    }
}
