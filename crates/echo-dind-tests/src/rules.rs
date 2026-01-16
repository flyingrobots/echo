// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Rewrite rules for the DIND test kernel.

use crate::generated::codecs::{ops, MotionV2Builder, MotionV2View};
use crate::generated::type_ids::*;
use echo_wasm_abi::unpack_intent_v1;
use warp_core::{
    make_edge_id, make_node_id, make_type_id, AtomPayload, AtomView, AttachmentKey, AttachmentSet,
    AttachmentValue, ConflictPolicy, EdgeRecord, Footprint, GraphStore, Hash, IdSet, NodeId,
    NodeKey, NodeRecord, PatternGraph, RewriteRule, TypeId,
};

const TYPE_VIEW_OP: &str = "sys/view/op";

/// Human-readable name for the route push command rule.
pub const ROUTE_PUSH_RULE_NAME: &str = "cmd/route_push";
/// Human-readable name for the set theme command rule.
pub const SET_THEME_RULE_NAME: &str = "cmd/set_theme";
/// Human-readable name for the toggle nav command rule.
pub const TOGGLE_NAV_RULE_NAME: &str = "cmd/toggle_nav";
/// Human-readable name for the toast command rule.
pub const TOAST_RULE_NAME: &str = "cmd/toast";
/// Human-readable name for the drop ball command rule.
pub const DROP_BALL_RULE_NAME: &str = "cmd/drop_ball";
/// Human-readable name for the combined ball physics rule.
pub const BALL_PHYSICS_RULE_NAME: &str = "physics/ball";
/// Human-readable name for the test-only KV rule.
#[cfg(feature = "dind_ops")]
pub const PUT_KV_RULE_NAME: &str = "cmd/put_kv";

/// Constructs the `cmd/route_push` rewrite rule.
#[must_use]
pub fn route_push_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/route_push").0;
    RewriteRule {
        id,
        name: ROUTE_PUSH_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| matcher_for_op(s, scope, ops::route_push::OP_ID),
        executor: |s, scope| {
            if let Some(args) =
                decode_op_args::<ops::route_push::Args>(s, scope, ops::route_push::decode_vars)
            {
                apply_route_push(s, args.path);
            }
        },
        compute_footprint: |s, scope| footprint_for_state_node(s, scope, "sim/state/routePath"),
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Constructs the `cmd/set_theme` rewrite rule.
#[must_use]
pub fn set_theme_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/set_theme").0;
    RewriteRule {
        id,
        name: SET_THEME_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| matcher_for_op(s, scope, ops::set_theme::OP_ID),
        executor: |s, scope| {
            if let Some(args) =
                decode_op_args::<ops::set_theme::Args>(s, scope, ops::set_theme::decode_vars)
            {
                apply_set_theme(s, args.mode);
            }
        },
        compute_footprint: |s, scope| footprint_for_state_node(s, scope, "sim/state/theme"),
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Constructs the `cmd/toggle_nav` rewrite rule.
#[must_use]
pub fn toggle_nav_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/toggle_nav").0;
    RewriteRule {
        id,
        name: TOGGLE_NAV_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| matcher_for_op(s, scope, ops::toggle_nav::OP_ID),
        executor: |s, _scope| {
            apply_toggle_nav(s);
        },
        compute_footprint: |s, scope| footprint_for_state_node(s, scope, "sim/state/navOpen"),
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Constructs the `cmd/toast` rewrite rule.
#[must_use]
pub fn toast_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/toast").0;
    RewriteRule {
        id,
        name: TOAST_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| matcher_for_op(s, scope, ops::toast::OP_ID),
        executor: |s, scope| {
            if let Some(args) =
                decode_op_args::<ops::toast::Args>(s, scope, ops::toast::decode_vars)
            {
                emit_view_op(s, TYPEID_VIEW_OP_SHOWTOAST, args.message.as_bytes());
            }
        },
        compute_footprint: |s, scope| footprint_for_state_node(s, scope, "sim/view"),
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Constructs the `cmd/drop_ball` rewrite rule.
#[must_use]
pub fn drop_ball_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/drop_ball").0;
    RewriteRule {
        id,
        name: DROP_BALL_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| matcher_for_op(s, scope, ops::drop_ball::OP_ID),
        executor: |s, _scope| {
            let ball_id = make_node_id("ball");
            let pos = [0, 400i64 << 32, 0];
            let vel = [0, -5i64 << 32, 0];
            let payload = MotionV2Builder::new(pos, vel).into_bytes();
            let atom = AtomPayload::new(TYPEID_PAYLOAD_MOTION_V2, payload);
            s.insert_node(
                ball_id,
                NodeRecord {
                    ty: make_type_id("entity"),
                },
            );
            s.set_node_attachment(ball_id, Some(AttachmentValue::Atom(atom)));
        },
        compute_footprint: |s, scope| {
            let mut fp = footprint_for_state_node(s, scope, "ball");
            fp.n_write.insert_node(&make_node_id("ball"));
            fp
        },
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Constructs the `physics/ball` rewrite rule (Semi-implicit Euler).
#[must_use]
pub fn ball_physics_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:physics/ball").0;
    RewriteRule {
        id,
        name: BALL_PHYSICS_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| {
            if *scope != make_node_id("ball") {
                return false;
            }
            if let Some(m) = MotionV2View::try_from_node(s, scope) {
                let moving = m.vel_raw().iter().any(|&v| v != 0);
                let below_ground = m.pos_raw()[1] < 0;
                let airborne = m.pos_raw()[1] > 0;
                return moving || below_ground || airborne;
            }
            false
        },
        executor: |s, scope| {
            if let Some(m) = MotionV2View::try_from_node(s, scope) {
                let mut pos = m.pos_raw();
                let mut vel = m.vel_raw();

                if pos[1] > 0 {
                    vel[1] -= 1i64 << 32;
                }
                pos[1] += vel[1];
                if pos[1] <= 0 {
                    pos[1] = 0;
                    vel = [0, 0, 0];
                }

                let out = MotionV2Builder::new(pos, vel).into_bytes();
                if let Some(AttachmentValue::Atom(atom)) = s.node_attachment_mut(scope) {
                    atom.bytes = out;
                }
            }
        },
        compute_footprint: |s, scope| {
            let mut a_write = AttachmentSet::default();
            a_write.insert(AttachmentKey::node_alpha(NodeKey {
                warp_id: s.warp_id(),
                local_id: *scope,
            }));
            Footprint {
                a_write,
                ..Default::default()
            }
        },
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Constructs the `cmd/put_kv` rewrite rule (Test only).
#[cfg(feature = "dind_ops")]
#[must_use]
pub fn put_kv_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/put_kv").0;
    RewriteRule {
        id,
        name: PUT_KV_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| matcher_for_op(s, scope, ops::put_kv::OP_ID),
        executor: |s, scope| {
            if let Some(args) =
                decode_op_args::<ops::put_kv::Args>(s, scope, ops::put_kv::decode_vars)
            {
                apply_put_kv(s, args.key, args.value);
            }
        },
        compute_footprint: |s, scope| {
            if let Some(args) =
                decode_op_args::<ops::put_kv::Args>(s, scope, ops::put_kv::decode_vars)
            {
                footprint_for_state_node(s, scope, &format!("sim/state/kv/{}", args.key))
            } else {
                Footprint::default()
            }
        },
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

// --- Helpers ---

fn matcher_for_op(store: &GraphStore, scope: &NodeId, expected_op_id: u32) -> bool {
    let Some(AttachmentValue::Atom(a)) = store.node_attachment(scope) else {
        return false;
    };
    if let Ok((op_id, _)) = unpack_intent_v1(&a.bytes) {
        return op_id == expected_op_id;
    }
    false
}

fn decode_op_args<T>(
    store: &GraphStore,
    scope: &NodeId,
    decode_fn: fn(&[u8]) -> Option<T>,
) -> Option<T> {
    let AttachmentValue::Atom(a) = store.node_attachment(scope)? else {
        return None;
    };
    let (_, vars) = unpack_intent_v1(&a.bytes).ok()?;
    decode_fn(vars)
}

impl<'a> MotionV2View<'a> {
    pub fn try_from_node(store: &'a GraphStore, node: &NodeId) -> Option<Self> {
        let AttachmentValue::Atom(p) = store.node_attachment(node)? else {
            return None;
        };
        Self::try_from_payload(p)
    }
}

pub fn footprint_for_state_node(
    store: &GraphStore,
    scope: &NodeId,
    state_node_path: &str,
) -> Footprint {
    let mut n_read = IdSet::default();
    let mut n_write = IdSet::default();
    let mut e_write = IdSet::default();
    let mut a_read = AttachmentSet::default();
    let mut a_write = AttachmentSet::default();

    n_read.insert_node(scope);
    a_read.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id: store.warp_id(),
        local_id: *scope,
    }));

    let sim_id = make_node_id("sim");
    let sim_state_id = make_node_id("sim/state");
    let target_id = make_node_id(state_node_path);

    n_write.insert_node(&sim_id);
    n_write.insert_node(&sim_state_id);
    n_write.insert_node(&target_id);

    e_write.insert_edge(&make_edge_id("edge:sim/state"));
    e_write.insert_edge(&make_edge_id(&format!("edge:{state_node_path}")));

    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id: store.warp_id(),
        local_id: target_id,
    }));

    Footprint {
        n_read,
        n_write,
        e_write,
        a_read,
        a_write,
        ..Default::default()
    }
}

pub fn ensure_state_base(store: &mut GraphStore) -> (NodeId, NodeId) {
    let sim_id = make_node_id("sim");
    let sim_state_id = make_node_id("sim/state");
    store.insert_node(
        sim_id,
        NodeRecord {
            ty: make_type_id("sim"),
        },
    );
    store.insert_node(
        sim_state_id,
        NodeRecord {
            ty: make_type_id("sim/state"),
        },
    );
    store.insert_edge(
        sim_id,
        EdgeRecord {
            id: make_edge_id("edge:sim/state"),
            from: sim_id,
            to: sim_state_id,
            ty: make_type_id("edge:state"),
        },
    );
    (sim_id, sim_state_id)
}

pub fn apply_route_push(store: &mut GraphStore, path: String) {
    let (_, sim_state_id) = ensure_state_base(store);
    let id = make_node_id("sim/state/routePath");
    store.insert_node(
        id,
        NodeRecord {
            ty: make_type_id("sim/state/routePath"),
        },
    );
    store.insert_edge(
        sim_state_id,
        EdgeRecord {
            id: make_edge_id("edge:sim/state/routePath"),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:routePath"),
        },
    );

    let b = path.as_bytes();
    let mut out = Vec::with_capacity(4 + b.len());
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
    store.set_node_attachment(
        id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_ROUTE_PATH,
            out.into(),
        ))),
    );
}

pub fn apply_set_theme(store: &mut GraphStore, mode: crate::generated::codecs::Theme) {
    let (_, sim_state_id) = ensure_state_base(store);
    let id = make_node_id("sim/state/theme");
    store.insert_node(
        id,
        NodeRecord {
            ty: make_type_id("sim/state/theme"),
        },
    );
    store.insert_edge(
        sim_state_id,
        EdgeRecord {
            id: make_edge_id("edge:sim/state/theme"),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:theme"),
        },
    );
    store.set_node_attachment(
        id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_THEME,
            (mode as u16).to_le_bytes().to_vec().into(),
        ))),
    );
}

pub fn apply_toggle_nav(store: &mut GraphStore) {
    let (_, sim_state_id) = ensure_state_base(store);
    let id = make_node_id("sim/state/navOpen");
    store.insert_node(
        id,
        NodeRecord {
            ty: make_type_id("sim/state/navOpen"),
        },
    );
    store.insert_edge(
        sim_state_id,
        EdgeRecord {
            id: make_edge_id("edge:sim/state/navOpen"),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:navOpen"),
        },
    );

    let current_bytes = store.node_attachment(&id).and_then(|v| match v {
        AttachmentValue::Atom(a) => Some(a.bytes.clone()),
        _ => None,
    });
    let current_val = current_bytes
        .as_ref()
        .map(|b| if !b.is_empty() && b[0] == 1 { 1 } else { 0 })
        .unwrap_or(0);
    let next_val = if current_val == 1 { 0u8 } else { 1u8 };

    store.set_node_attachment(
        id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_NAV_OPEN,
            bytes::Bytes::copy_from_slice(&[next_val]),
        ))),
    );
}

#[cfg(feature = "dind_ops")]
pub fn apply_put_kv(store: &mut GraphStore, key: String, value: String) {
    let (_, sim_state_id) = ensure_state_base(store);
    let node_label = format!("sim/state/kv/{}", key);
    let id = make_node_id(&node_label);

    store.insert_node(
        id,
        NodeRecord {
            ty: make_type_id("sim/state/kv"),
        },
    );

    let edge_label = format!("edge:sim/state/kv/{}", key);
    store.insert_edge(
        sim_state_id,
        EdgeRecord {
            id: make_edge_id(&edge_label),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:kv"),
        },
    );

    let b = value.as_bytes();
    let mut out = Vec::with_capacity(4 + b.len());
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
    store.set_node_attachment(
        id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_KV,
            out.into(),
        ))),
    );
}

pub fn emit_view_op(store: &mut GraphStore, type_id: TypeId, payload: &[u8]) {
    let view_id = make_node_id("sim/view");
    store.insert_node(
        view_id,
        NodeRecord {
            ty: make_type_id("sim/view"),
        },
    );
    let seq_key = make_type_id("sim/view/seq");
    let seq = store
        .node_attachment(&view_id)
        .and_then(|v| match v {
            AttachmentValue::Atom(a) if a.type_id == seq_key => {
                let mut b = [0u8; 8];
                b.copy_from_slice(&a.bytes[..8]);
                Some(u64::from_le_bytes(b))
            }
            _ => None,
        })
        .unwrap_or(0);
    let op_id = make_node_id(&format!("sim/view/op:{:016}", seq));
    store.insert_node(
        op_id,
        NodeRecord {
            ty: make_type_id(TYPE_VIEW_OP),
        },
    );
    store.insert_edge(
        view_id,
        EdgeRecord {
            id: make_edge_id(&format!("edge:view/op:{:016}", seq)),
            from: view_id,
            to: op_id,
            ty: make_type_id("edge:view/op"),
        },
    );
    store.set_node_attachment(
        op_id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            type_id,
            bytes::Bytes::copy_from_slice(payload),
        ))),
    );
    store.set_node_attachment(
        view_id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            seq_key,
            bytes::Bytes::copy_from_slice(&(seq + 1).to_le_bytes()),
        ))),
    );
}

pub fn project_state(store: &mut GraphStore) {
    let theme_bytes = store
        .node_attachment(&make_node_id("sim/state/theme"))
        .and_then(|v| {
            if let AttachmentValue::Atom(a) = v {
                Some(a.bytes.clone())
            } else {
                None
            }
        });
    if let Some(b) = theme_bytes {
        emit_view_op(store, TYPEID_VIEW_OP_SETTHEME, &b);
    }

    let nav_bytes = store
        .node_attachment(&make_node_id("sim/state/navOpen"))
        .and_then(|v| {
            if let AttachmentValue::Atom(a) = v {
                Some(a.bytes.clone())
            } else {
                None
            }
        });
    if let Some(b) = nav_bytes {
        emit_view_op(store, TYPEID_VIEW_OP_TOGGLENAV, &b);
    }

    let route_bytes = store
        .node_attachment(&make_node_id("sim/state/routePath"))
        .and_then(|v| {
            if let AttachmentValue::Atom(a) = v {
                Some(a.bytes.clone())
            } else {
                None
            }
        });
    if let Some(b) = route_bytes {
        emit_view_op(store, TYPEID_VIEW_OP_ROUTEPUSH, &b);
    }
}
