// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Command rewrite rules for the website kernel spike.
//!
//! Commands are deterministic actions derived from host intents.
//!
//! In Phase 1, intents are ingested into `sim/inbox` (see [`crate::Engine::ingest_inbox_event`]),
//! then `sys/dispatch_inbox` routes recognized intent payloads to these `cmd/*` rules.

use crate::attachment::{AtomPayload, AttachmentKey, AttachmentValue};
use crate::footprint::{AttachmentSet, Footprint, IdSet, PortSet};
use crate::graph::GraphStore;
use crate::ident::{make_edge_id, make_node_id, make_type_id, Hash, NodeId, NodeKey};
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};

/// Human-readable name for the route push command rule.
pub const ROUTE_PUSH_RULE_NAME: &str = "cmd/route_push";
/// Human-readable name for the set theme command rule.
pub const SET_THEME_RULE_NAME: &str = "cmd/set_theme";
/// Human-readable name for the toggle nav command rule.
pub const TOGGLE_NAV_RULE_NAME: &str = "cmd/toggle_nav";
/// Human-readable name for the toast command rule.
pub const TOAST_RULE_NAME: &str = "cmd/toast";

/// Constructs the `cmd/route_push` rewrite rule.
#[must_use]
pub fn route_push_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/route_push").0;
    RewriteRule {
        id,
        name: ROUTE_PUSH_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |s, scope| {
            matcher_for_intent(s, scope, "intent:route_push")
                || matcher_for_intent_numeric(s, scope, "routePush")
        },
        executor: |s, scope| {
            if matcher_for_intent(s, scope, "intent:route_push") {
                executor_for_intent(s, scope, "intent:route_push", apply_route_push);
            } else {
                executor_for_intent_numeric(s, scope, "routePush", apply_route_push);
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
        matcher: |s, scope| {
            matcher_for_intent(s, scope, "intent:set_theme")
                || matcher_for_intent_numeric(s, scope, "setTheme")
        },
        executor: |s, scope| {
            if matcher_for_intent(s, scope, "intent:set_theme") {
                executor_for_intent(s, scope, "intent:set_theme", apply_set_theme);
            } else {
                executor_for_intent_numeric(s, scope, "setTheme", apply_set_theme);
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
        matcher: |s, scope| {
            matcher_for_intent(s, scope, "intent:toggle_nav")
                || matcher_for_intent_numeric(s, scope, "toggleNav")
        },
        executor: |s, scope| {
            if matcher_for_intent(s, scope, "intent:toggle_nav") {
                executor_for_intent(s, scope, "intent:toggle_nav", apply_toggle_nav);
            } else {
                executor_for_intent_numeric(s, scope, "toggleNav", apply_toggle_nav);
            }
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
        matcher: |s, scope| {
            matcher_for_intent(s, scope, "intent:toast")
                || matcher_for_intent_numeric(s, scope, "toast")
        },
        executor: |s, scope| {
            if matcher_for_intent(s, scope, "intent:toast") {
                executor_for_intent(s, scope, "intent:toast", apply_toast);
            } else {
                executor_for_intent_numeric(s, scope, "toast", apply_toast);
            }
        },
        compute_footprint: |_, _| Footprint {
            factor_mask: u64::MAX,
            ..Default::default()
        },
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn matcher_for_intent_numeric(store: &GraphStore, scope: &NodeId, op_name: &str) -> bool {
    let Some(node) = store.node(scope) else {
        return false;
    };
    if node.ty != make_type_id("sim/inbox/event") {
        return false;
    }

    let Some(AttachmentValue::Atom(atom)) = store.node_attachment(scope) else {
        return false;
    };
    is_intent_for_op(store, atom.type_id, op_name)
}

fn executor_for_intent_numeric<F>(
    store: &mut GraphStore,
    scope: &NodeId,
    op_name: &str,
    apply_fn: F,
) where
    F: FnOnce(&mut GraphStore, bytes::Bytes),
{
    let Some(AttachmentValue::Atom(atom)) = store.node_attachment(scope) else {
        return;
    };
    if !is_intent_for_op(store, atom.type_id, op_name) {
        return;
    }
    apply_fn(store, atom.bytes.clone());
}

/// Attempt to route a single inbox event to known command rules.
pub fn route_inbox_event(store: &mut GraphStore, event_id: &NodeId) -> bool {
    let Some(atom) = store
        .node_attachment(event_id)
        .cloned()
        .and_then(|v| match v {
            AttachmentValue::Atom(a) => Some(a),
            AttachmentValue::Descend(_) => None,
        })
    else {
        return false;
    };

    let intent_id = atom.type_id;

    if intent_id == make_type_id("intent:route_push")
        || is_intent_for_op(store, intent_id, "routePush")
    {
        apply_route_push(store, atom.bytes);
        return true;
    }
    if intent_id == make_type_id("intent:set_theme")
        || is_intent_for_op(store, intent_id, "setTheme")
    {
        apply_set_theme(store, atom.bytes);
        return true;
    }
    if intent_id == make_type_id("intent:toggle_nav")
        || is_intent_for_op(store, intent_id, "toggleNav")
    {
        apply_toggle_nav(store, atom.bytes);
        return true;
    }
    if intent_id == make_type_id("intent:toast") || is_intent_for_op(store, intent_id, "toast") {
        apply_toast(store, atom.bytes);
        return true;
    }

    false
}

fn is_intent_for_op(_store: &GraphStore, intent_id: crate::ident::TypeId, op_name: &str) -> bool {
    let op_id: u32 = match op_name {
        "routePush" => 2216217860_u32,
        "setTheme" => 1822649880_u32,
        "toggleNav" => 3272403183_u32,
        "toast" => 4255241313_u32,
        _ => return false,
    };
    intent_id == make_type_id(&format!("intent:{op_id}"))
}

fn apply_toast(store: &mut GraphStore, payload_bytes: bytes::Bytes) {
    emit_view_op(store, "ShowToast", payload_bytes);
}

fn matcher_for_intent(store: &GraphStore, scope: &NodeId, intent_type: &str) -> bool {
    let Some(node) = store.node(scope) else {
        return false;
    };
    if node.ty != make_type_id("sim/inbox/event") {
        return false;
    }

    let Some(AttachmentValue::Atom(atom)) = store.node_attachment(scope) else {
        return false;
    };
    atom.type_id == make_type_id(intent_type)
}

fn executor_for_intent<F>(store: &mut GraphStore, scope: &NodeId, intent_type: &str, apply_fn: F)
where
    F: FnOnce(&mut GraphStore, bytes::Bytes),
{
    let Some(AttachmentValue::Atom(atom)) = store.node_attachment(scope) else {
        return;
    };
    if atom.type_id != make_type_id(intent_type) {
        return;
    }
    apply_fn(store, atom.bytes.clone());
}

fn footprint_for_state_node(
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
    let edge_id = make_edge_id(&format!("edge:{state_node_path}"));
    e_write.insert_edge(&edge_id);

    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id: store.warp_id(),
        local_id: target_id,
    }));

    Footprint {
        n_read,
        n_write,
        e_read: IdSet::default(),
        e_write,
        a_read,
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

fn ensure_state_base(store: &mut GraphStore) -> (NodeId, NodeId) {
    let sim_id = make_node_id("sim");
    let sim_state_id = make_node_id("sim/state");

    store.insert_node(
        sim_id,
        crate::record::NodeRecord {
            ty: make_type_id("sim"),
        },
    );
    store.insert_node(
        sim_state_id,
        crate::record::NodeRecord {
            ty: make_type_id("sim/state"),
        },
    );

    store.insert_edge(
        sim_id,
        crate::record::EdgeRecord {
            id: make_edge_id("edge:sim/state"),
            from: sim_id,
            to: sim_state_id,
            ty: make_type_id("edge:state"),
        },
    );

    (sim_id, sim_state_id)
}

fn apply_route_push(store: &mut GraphStore, payload_bytes: bytes::Bytes) {
    let (_, sim_state_id) = ensure_state_base(store);
    let route_id = make_node_id("sim/state/routePath");

    store.insert_node(
        route_id,
        crate::record::NodeRecord {
            ty: make_type_id("sim/state/routePath"),
        },
    );

    store.insert_edge(
        sim_state_id,
        crate::record::EdgeRecord {
            id: make_edge_id("edge:sim/state/routePath"),
            from: sim_state_id,
            to: route_id,
            ty: make_type_id("edge:routePath"),
        },
    );

    let route_payload = AtomPayload::new(make_type_id("state:route_path"), payload_bytes.clone());
    store.set_node_attachment(route_id, Some(AttachmentValue::Atom(route_payload)));

    emit_view_op(store, "RoutePush", payload_bytes);
}

fn apply_set_theme(store: &mut GraphStore, payload_bytes: bytes::Bytes) {
    let (_, sim_state_id) = ensure_state_base(store);
    let theme_id = make_node_id("sim/state/theme");

    store.insert_node(
        theme_id,
        crate::record::NodeRecord {
            ty: make_type_id("sim/state/theme"),
        },
    );

    store.insert_edge(
        sim_state_id,
        crate::record::EdgeRecord {
            id: make_edge_id("edge:sim/state/theme"),
            from: sim_state_id,
            to: theme_id,
            ty: make_type_id("edge:theme"),
        },
    );

    let theme_payload = AtomPayload::new(make_type_id("state:theme"), payload_bytes.clone());
    store.set_node_attachment(theme_id, Some(AttachmentValue::Atom(theme_payload)));

    emit_view_op(store, "SetTheme", payload_bytes);
}

fn apply_toggle_nav(store: &mut GraphStore, _payload_bytes: bytes::Bytes) {
    let (_, sim_state_id) = ensure_state_base(store);
    let nav_id = make_node_id("sim/state/navOpen");

    store.insert_node(
        nav_id,
        crate::record::NodeRecord {
            ty: make_type_id("sim/state/navOpen"),
        },
    );

    store.insert_edge(
        sim_state_id,
        crate::record::EdgeRecord {
            id: make_edge_id("edge:sim/state/navOpen"),
            from: sim_state_id,
            to: nav_id,
            ty: make_type_id("edge:navOpen"),
        },
    );

    let current_val = store.node_attachment(&nav_id).and_then(|v| match v {
        AttachmentValue::Atom(a) => Some(a.bytes.as_ref()),
        AttachmentValue::Descend(_) => None,
    });

    let new_val = if current_val == Some(b"true") {
        bytes::Bytes::from_static(b"false")
    } else {
        bytes::Bytes::from_static(b"true")
    };

    let nav_payload = AtomPayload::new(make_type_id("state:nav_open"), new_val.clone());
    store.set_node_attachment(nav_id, Some(AttachmentValue::Atom(nav_payload)));

    emit_view_op(store, "ToggleNav", new_val);
}

/// Emits a `ViewOp` into the `sim/view` inbox.
fn emit_view_op(store: &mut GraphStore, kind: &str, payload: bytes::Bytes) {
    let sim_id = make_node_id("sim");
    let view_id = make_node_id("sim/view");

    store.insert_node(
        view_id,
        crate::record::NodeRecord {
            ty: make_type_id("sim/view"),
        },
    );
    store.insert_edge(
        sim_id,
        crate::record::EdgeRecord {
            id: make_edge_id("edge:sim/view"),
            from: sim_id,
            to: view_id,
            ty: make_type_id("edge:view"),
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

    let op_label = format!("sim/view/op:{seq:016}");
    let op_id = make_node_id(&op_label);

    store.insert_node(
        op_id,
        crate::record::NodeRecord {
            ty: make_type_id("sys/view/op"),
        },
    );
    store.insert_edge(
        view_id,
        crate::record::EdgeRecord {
            id: make_edge_id(&format!("edge:view/op:{seq:016}")),
            from: view_id,
            to: op_id,
            ty: make_type_id("edge:view/op"),
        },
    );

    let atom = AtomPayload::new(make_type_id(&format!("view_op:{kind}")), payload);
    store.set_node_attachment(op_id, Some(AttachmentValue::Atom(atom)));

    let next_seq = seq + 1;
    store.set_node_attachment(
        view_id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            seq_key,
            bytes::Bytes::copy_from_slice(&next_seq.to_le_bytes()),
        ))),
    );
}
