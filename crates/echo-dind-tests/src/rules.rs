// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Rewrite rules for the DIND test kernel.

use crate::codecs::{ops, MotionV2Builder, MotionV2View};
use crate::type_ids::*;
use echo_wasm_abi::unpack_intent_v1;
use warp_core::{
    make_edge_id, make_node_id, make_type_id, AtomPayload, AtomView, AttachmentKey, AttachmentSet,
    AttachmentValue, ConflictPolicy, EdgeRecord, EdgeSet, Footprint, GraphStore, GraphView, Hash,
    NodeId, NodeKey, NodeRecord, NodeSet, PatternGraph, RewriteRule, TickDelta, TypeId, WarpId,
    WarpOp,
};

const TYPE_VIEW_OP: &str = "sys/view/op";

// -----------------------------------------------------------------------------
// Fixed-point physics constants (Q32.32 format)
// -----------------------------------------------------------------------------

/// Q32.32 scale factor: 1 unit = 2^32 fixed-point units.
const FIXED_POINT_SCALE: i64 = 1 << 32;

/// Initial ball height in world units (400 units above ground).
const BALL_INITIAL_HEIGHT: i64 = 400;

/// Initial downward velocity magnitude in world units per tick.
const BALL_INITIAL_VELOCITY: i64 = 5;

/// Gravity acceleration in world units per tick (applied each physics step).
const GRAVITY_ACCEL: i64 = 1;

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
        executor: |s, scope, delta| {
            if let Some(args) =
                decode_op_args::<ops::route_push::Args>(s, scope, ops::route_push::decode_vars)
            {
                emit_route_push(s.warp_id(), delta, args.path);
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
        executor: |s, scope, delta| {
            if let Some(args) =
                decode_op_args::<ops::set_theme::Args>(s, scope, ops::set_theme::decode_vars)
            {
                emit_set_theme(s.warp_id(), delta, args.mode);
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
        executor: |s, _scope, delta| {
            emit_toggle_nav(s, delta);
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
        executor: |s, scope, delta| {
            if let Some(args) =
                decode_op_args::<ops::toast::Args>(s, scope, ops::toast::decode_vars)
            {
                // Use intent scope (NodeId) for deterministic view op sequencing.
                // This ensures the same intent always produces the same view op ID,
                // regardless of parallel execution order.
                emit_view_op_delta_scoped(
                    s.warp_id(),
                    delta,
                    TYPEID_VIEW_OP_SHOWTOAST,
                    args.message.as_bytes(),
                    scope,
                );
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
        executor: |view, _scope, delta| {
            let warp_id = view.warp_id();
            let ball_id = make_node_id("ball");
            // Q32.32 fixed-point: 1 unit = 1 << 32
            // Initial: y=400 units, downward velocity 5 units/tick
            let pos = [0, BALL_INITIAL_HEIGHT * FIXED_POINT_SCALE, 0];
            let vel = [0, -BALL_INITIAL_VELOCITY * FIXED_POINT_SCALE, 0];
            let payload = MotionV2Builder::new(pos, vel).into_bytes();
            let atom = AtomPayload::new(TYPEID_PAYLOAD_MOTION_V2, payload);
            delta.push(WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: ball_id,
                },
                record: NodeRecord {
                    ty: make_type_id("entity"),
                },
            });
            delta.push(WarpOp::SetAttachment {
                key: AttachmentKey::node_alpha(NodeKey {
                    warp_id,
                    local_id: ball_id,
                }),
                value: Some(AttachmentValue::Atom(atom)),
            });
        },
        compute_footprint: |s, scope| {
            let mut fp = footprint_for_state_node(s, scope, "ball");
            fp.n_write.insert(NodeKey {
                warp_id: s.warp_id(),
                local_id: make_node_id("ball"),
            });
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
        matcher: |view, scope| {
            if *scope != make_node_id("ball") {
                return false;
            }
            if let Some(m) = MotionV2View::try_from_node(&view, scope) {
                let has_velocity = m.vel_raw().iter().any(|&v| v != 0);
                let not_at_rest = m.pos_raw()[1] != 0;
                return has_velocity || not_at_rest;
            }
            false
        },
        executor: |view, scope, delta| {
            if let Some(m) = MotionV2View::try_from_node(&view, scope) {
                let mut pos = m.pos_raw();
                let mut vel = m.vel_raw();

                // Apply gravity (semi-implicit Euler) while airborne
                if pos[1] > 0 {
                    vel[1] -= GRAVITY_ACCEL * FIXED_POINT_SCALE;
                }
                pos[1] += vel[1];
                if pos[1] <= 0 {
                    pos[1] = 0;
                    vel = [0, 0, 0];
                }

                let out = MotionV2Builder::new(pos, vel).into_bytes();
                delta.push(WarpOp::SetAttachment {
                    key: AttachmentKey::node_alpha(NodeKey {
                        warp_id: view.warp_id(),
                        local_id: *scope,
                    }),
                    value: Some(AttachmentValue::Atom(AtomPayload::new(
                        TYPEID_PAYLOAD_MOTION_V2,
                        out,
                    ))),
                });
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
// TODO: Double-decoding desync risk - decode_op_args is called in both executor
// and compute_footprint. If the decoder has side effects or if the attachment
// changes between calls, this could lead to inconsistent behavior.
#[cfg(feature = "dind_ops")]
#[must_use]
pub fn put_kv_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/put_kv").0;
    RewriteRule {
        id,
        name: PUT_KV_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| matcher_for_op(s, scope, ops::put_kv::OP_ID),
        executor: |s, scope, delta| {
            if let Some(args) =
                decode_op_args::<ops::put_kv::Args>(s, scope, ops::put_kv::decode_vars)
            {
                emit_put_kv(s.warp_id(), delta, args.key, args.value);
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

fn matcher_for_op(view: GraphView<'_>, scope: &NodeId, expected_op_id: u32) -> bool {
    let Some(AttachmentValue::Atom(a)) = view.node_attachment(scope) else {
        return false;
    };
    if let Ok((op_id, _)) = unpack_intent_v1(&a.bytes) {
        return op_id == expected_op_id;
    }
    false
}

fn decode_op_args<T>(
    view: GraphView<'_>,
    scope: &NodeId,
    decode_fn: fn(&[u8]) -> Option<T>,
) -> Option<T> {
    let AttachmentValue::Atom(a) = view.node_attachment(scope)? else {
        return None;
    };
    let (_, vars) = unpack_intent_v1(&a.bytes).ok()?;
    decode_fn(vars)
}

impl<'a> MotionV2View<'a> {
    /// Attempt to construct a motion v2 view from a node's attachment.
    pub fn try_from_node(view: &'a GraphView<'a>, node: &NodeId) -> Option<Self> {
        let AttachmentValue::Atom(p) = view.node_attachment(node)? else {
            return None;
        };
        Self::try_from_payload(p)
    }
}

/// Compute the footprint for a state node operation.
pub fn footprint_for_state_node(
    view: GraphView<'_>,
    scope: &NodeId,
    state_node_path: &str,
) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut n_write = NodeSet::default();
    let mut e_write = EdgeSet::default();
    let mut a_read = AttachmentSet::default();
    let mut a_write = AttachmentSet::default();

    n_read.insert_with_warp(warp_id, *scope);
    a_read.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: *scope,
    }));

    let sim_id = make_node_id("sim");
    let sim_state_id = make_node_id("sim/state");
    let target_id = make_node_id(state_node_path);

    n_write.insert_with_warp(warp_id, sim_id);
    n_write.insert_with_warp(warp_id, sim_state_id);
    n_write.insert_with_warp(warp_id, target_id);

    e_write.insert_with_warp(warp_id, make_edge_id("edge:sim/state"));
    e_write.insert_with_warp(warp_id, make_edge_id(&format!("edge:{state_node_path}")));

    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
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

// =============================================================================
// Phase 5 BOAW emit functions (emit deltas instead of mutating store)
// =============================================================================

/// Emit ops to ensure sim and sim/state base nodes exist.
fn emit_state_base(warp_id: WarpId, delta: &mut TickDelta) -> (NodeId, NodeId) {
    let sim_id = make_node_id("sim");
    let sim_state_id = make_node_id("sim/state");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: sim_id,
        },
        record: NodeRecord {
            ty: make_type_id("sim"),
        },
    });
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: sim_state_id,
        },
        record: NodeRecord {
            ty: make_type_id("sim/state"),
        },
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: make_edge_id("edge:sim/state"),
            from: sim_id,
            to: sim_state_id,
            ty: make_type_id("edge:state"),
        },
    });
    (sim_id, sim_state_id)
}

/// Emit ops for a route push operation.
fn emit_route_push(warp_id: WarpId, delta: &mut TickDelta, path: String) {
    let (_, sim_state_id) = emit_state_base(warp_id, delta);
    let id = make_node_id("sim/state/routePath");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: id,
        },
        record: NodeRecord {
            ty: make_type_id("sim/state/routePath"),
        },
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: make_edge_id("edge:sim/state/routePath"),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:routePath"),
        },
    });

    let b = path.as_bytes();
    let mut out = Vec::with_capacity(4 + b.len());
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: id,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_ROUTE_PATH,
            out.into(),
        ))),
    });
}

/// Emit ops for a set theme operation.
fn emit_set_theme(warp_id: WarpId, delta: &mut TickDelta, mode: crate::codecs::Theme) {
    let (_, sim_state_id) = emit_state_base(warp_id, delta);
    let id = make_node_id("sim/state/theme");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: id,
        },
        record: NodeRecord {
            ty: make_type_id("sim/state/theme"),
        },
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: make_edge_id("edge:sim/state/theme"),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:theme"),
        },
    });
    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: id,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_THEME,
            (mode as u16).to_le_bytes().to_vec().into(),
        ))),
    });
}

/// Emit ops for a toggle nav operation.
fn emit_toggle_nav(view: GraphView<'_>, delta: &mut TickDelta) {
    let warp_id = view.warp_id();
    let (_, sim_state_id) = emit_state_base(warp_id, delta);
    let id = make_node_id("sim/state/navOpen");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: id,
        },
        record: NodeRecord {
            ty: make_type_id("sim/state/navOpen"),
        },
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: make_edge_id("edge:sim/state/navOpen"),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:navOpen"),
        },
    });

    let current_val = match view.node_attachment(&id) {
        Some(AttachmentValue::Atom(a)) if !a.bytes.is_empty() && a.bytes[0] == 1 => 1u8,
        _ => 0u8,
    };
    let next_val = if current_val == 1 { 0u8 } else { 1u8 };

    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: id,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_NAV_OPEN,
            bytes::Bytes::copy_from_slice(&[next_val]),
        ))),
    });
}

/// Emit ops for a view operation with scope-derived deterministic sequencing.
///
/// Uses the triggering intent's scope (NodeId) to derive a unique view op ID.
/// This ensures determinism under parallel execution since the same intent
/// always produces the same view op ID regardless of worker assignment.
fn emit_view_op_delta_scoped(
    warp_id: WarpId,
    delta: &mut TickDelta,
    type_id: TypeId,
    payload: &[u8],
    scope: &NodeId,
) {
    let view_id = make_node_id("sim/view");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: view_id,
        },
        record: NodeRecord {
            ty: make_type_id("sim/view"),
        },
    });
    // Derive view op ID from the intent's scope (NodeId) for deterministic sequencing.
    // The scope is content-addressed and unique per intent, ensuring no collisions.
    // Use first 16 bytes of scope as hex for a compact, collision-resistant identifier.
    let scope_hex: String = scope.0[..16].iter().map(|b| format!("{:02x}", b)).collect();
    let op_id = make_node_id(&format!("sim/view/op:{}", scope_hex));
    let edge_id = make_edge_id(&format!("edge:view/op:{}", scope_hex));
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: op_id,
        },
        record: NodeRecord {
            ty: make_type_id(TYPE_VIEW_OP),
        },
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: edge_id,
            from: view_id,
            to: op_id,
            ty: make_type_id("edge:view/op"),
        },
    });
    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: op_id,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            type_id,
            bytes::Bytes::copy_from_slice(payload),
        ))),
    });
}

/// Emit ops for a view operation.
///
/// The `op_ix` parameter provides a deterministic per-op sequence to avoid ID collisions.
/// Callers should pass `delta.len()` to get a unique index for each op in the tick.
///
/// **DEPRECATED**: Use [`emit_view_op_delta_scoped`] instead for parallel-safe determinism.
#[allow(dead_code)]
fn emit_view_op_delta(
    warp_id: WarpId,
    delta: &mut TickDelta,
    type_id: TypeId,
    payload: &[u8],
    op_ix: usize,
) {
    let view_id = make_node_id("sim/view");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: view_id,
        },
        record: NodeRecord {
            ty: make_type_id("sim/view"),
        },
    });
    // Use op_ix from caller (typically delta.len() before this call) for unique sequencing
    let seq = op_ix as u64;
    let op_id = make_node_id(&format!("sim/view/op:{:016}", seq));
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: op_id,
        },
        record: NodeRecord {
            ty: make_type_id(TYPE_VIEW_OP),
        },
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: make_edge_id(&format!("edge:view/op:{:016}", seq)),
            from: view_id,
            to: op_id,
            ty: make_type_id("edge:view/op"),
        },
    });
    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: op_id,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            type_id,
            bytes::Bytes::copy_from_slice(payload),
        ))),
    });
}

/// Emit ops for a put KV operation.
#[cfg(feature = "dind_ops")]
fn emit_put_kv(warp_id: WarpId, delta: &mut TickDelta, key: String, value: String) {
    let (_, sim_state_id) = emit_state_base(warp_id, delta);
    let node_label = format!("sim/state/kv/{}", key);
    let id = make_node_id(&node_label);

    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: id,
        },
        record: NodeRecord {
            ty: make_type_id("sim/state/kv"),
        },
    });

    let edge_label = format!("edge:sim/state/kv/{}", key);
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: make_edge_id(&edge_label),
            from: sim_state_id,
            to: id,
            ty: make_type_id("edge:kv"),
        },
    });

    let b = value.as_bytes();
    let mut out = Vec::with_capacity(4 + b.len());
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: id,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_KV,
            out.into(),
        ))),
    });
}

// =============================================================================
// Legacy apply functions (for backwards compatibility with tests that need
// direct store mutation; these will be deprecated as Phase 5 BOAW completes)
// =============================================================================

/// Ensure the sim and sim/state base nodes exist, returning their IDs.
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

/// Apply a route push operation to update the current route path.
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

/// Apply a set theme operation to update the current theme.
pub fn apply_set_theme(store: &mut GraphStore, mode: crate::codecs::Theme) {
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

/// Apply a toggle nav operation to toggle the navigation state.
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

    let current_val = match store.node_attachment(&id) {
        Some(AttachmentValue::Atom(a)) if !a.bytes.is_empty() && a.bytes[0] == 1 => 1u8,
        _ => 0u8,
    };
    let next_val = if current_val == 1 { 0u8 } else { 1u8 };

    store.set_node_attachment(
        id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            TYPEID_STATE_NAV_OPEN,
            bytes::Bytes::copy_from_slice(&[next_val]),
        ))),
    );
}

/// Apply a put KV operation to store a key-value pair.
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

/// Emit a view operation with the given type and payload.
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
                if a.bytes.len() < 8 {
                    return None;
                }
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

/// Helper to extract attachment bytes from a state node without cloning.
fn get_state_bytes(store: &GraphStore, state_path: &str) -> Option<bytes::Bytes> {
    match store.node_attachment(&make_node_id(state_path))? {
        AttachmentValue::Atom(a) => Some(a.bytes.clone()),
        _ => None,
    }
}

/// Project the current state to view operations.
///
/// This function reads the stored attachment bytes from state nodes and emits
/// them as view operations. The payload format is consistent between live
/// execution (e.g., `apply_route_push`, `apply_set_theme`) and replay:
///
/// - Live execution stores processed values as attachments
/// - `project_state` reads those same bytes and emits them as view ops
///
/// This ensures time-travel sync delivers the same payload shape as the UI expects.
///
/// We gather all bytes first (borrowing store immutably), then emit view ops
/// (requiring mutable store). bytes::Bytes clone is O(1) ref-count increment.
pub fn project_state(store: &mut GraphStore) {
    // Borrow bytes directly - gather all state before mutating
    let theme_bytes = get_state_bytes(store, "sim/state/theme");
    let nav_bytes = get_state_bytes(store, "sim/state/navOpen");
    let route_bytes = get_state_bytes(store, "sim/state/routePath");

    // Now emit view ops with mutable store access
    if let Some(b) = theme_bytes {
        emit_view_op(store, TYPEID_VIEW_OP_SETTHEME, &b);
    }
    if let Some(b) = nav_bytes {
        emit_view_op(store, TYPEID_VIEW_OP_TOGGLENAV, &b);
    }
    if let Some(b) = route_bytes {
        emit_view_op(store, TYPEID_VIEW_OP_ROUTEPUSH, &b);
    }
}
