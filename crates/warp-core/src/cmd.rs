// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Command rewrite rules for the website kernel spike.
//!
//! Commands are deterministic actions derived from host intents.
//!
//! In Phase 1, intents are ingested into `sim/inbox` (see [`crate::Engine::ingest_inbox_event`]),
//! then `sys/dispatch_inbox` routes recognized intent payloads to these `cmd/*` rules.

use crate::attachment::{AtomPayload, AttachmentKey, AttachmentOwner, AttachmentValue};
use crate::footprint::{AttachmentSet, Footprint, IdSet, PortSet};
use crate::graph::GraphStore;
use crate::ident::{make_edge_id, make_node_id, make_type_id, Hash, NodeId, NodeKey};
use crate::record::EdgeRecord;
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};

/// Human-readable name for the route push command rule.
pub const ROUTE_PUSH_RULE_NAME: &str = "cmd/route_push";

/// Constructs the `cmd/route_push` rewrite rule.
///
/// This rule is applied to an inbox event node (type `sim/inbox/event`) whose atom payload
/// has type id `intent:route_push`. The payload bytes are written into `sim/state/routePath`
/// under a `state:route_path` atom.
#[must_use]
pub fn route_push_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:cmd/route_push").0;
    RewriteRule {
        id,
        name: ROUTE_PUSH_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: route_push_matcher,
        executor: route_push_executor,
        compute_footprint: route_push_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Attempt to route a single inbox event to known command rules.
///
/// Returns `true` if a command rule handled the event, `false` otherwise.
///
/// This is used by the inbox dispatch rule to keep intent routing logic centralized
/// while the engine is still in a “single explicit apply” stage.
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

    if atom.type_id != make_type_id("intent:route_push") {
        return false;
    }

    apply_route_push(store, atom.bytes);
    true
}

fn route_push_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    let Some(node) = store.node(scope) else {
        return false;
    };
    if node.ty != make_type_id("sim/inbox/event") {
        return false;
    }

    let Some(AttachmentValue::Atom(atom)) = store.node_attachment(scope) else {
        return false;
    };
    atom.type_id == make_type_id("intent:route_push")
}

fn route_push_executor(store: &mut GraphStore, scope: &NodeId) {
    let Some(AttachmentValue::Atom(atom)) = store.node_attachment(scope) else {
        return;
    };
    if atom.type_id != make_type_id("intent:route_push") {
        return;
    }
    apply_route_push(store, atom.bytes.clone());
}

fn route_push_footprint(store: &GraphStore, scope: &NodeId) -> Footprint {
    let mut n_read = IdSet::default();
    let mut n_write = IdSet::default();
    let mut e_write = IdSet::default();
    let mut a_read = AttachmentSet::default();
    let mut a_write = AttachmentSet::default();

    n_read.insert_node(scope);
    a_read.insert(AttachmentKey {
        owner: AttachmentOwner::Node(NodeKey {
            warp_id: store.warp_id(),
            local_id: *scope,
        }),
        plane: crate::attachment::AttachmentPlane::Alpha,
    });

    // Target nodes/edges are fixed for this command.
    let sim_id = make_node_id("sim");
    let sim_state_id = make_node_id("sim/state");
    let route_id = make_node_id("sim/state/routePath");

    n_write.insert_node(&sim_id);
    n_write.insert_node(&sim_state_id);
    n_write.insert_node(&route_id);

    e_write.insert_edge(&make_edge_id("edge:sim/state"));
    e_write.insert_edge(&make_edge_id("edge:sim/state/routePath"));

    a_write.insert(AttachmentKey {
        owner: AttachmentOwner::Node(NodeKey {
            warp_id: store.warp_id(),
            local_id: route_id,
        }),
        plane: crate::attachment::AttachmentPlane::Alpha,
    });

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

fn apply_route_push(store: &mut GraphStore, payload_bytes: bytes::Bytes) {
    // Ensure sim/state/routePath structure exists.
    let sim_id = make_node_id("sim");
    let sim_state_id = make_node_id("sim/state");
    let route_id = make_node_id("sim/state/routePath");

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
    store.insert_node(
        route_id,
        crate::record::NodeRecord {
            ty: make_type_id("sim/state/routePath"),
        },
    );

    // Edges (idempotent inserts)
    store.insert_edge(
        sim_id,
        EdgeRecord {
            id: make_edge_id("edge:sim/state"),
            from: sim_id,
            to: sim_state_id,
            ty: make_type_id("edge:state"),
        },
    );
    store.insert_edge(
        sim_state_id,
        EdgeRecord {
            id: make_edge_id("edge:sim/state/routePath"),
            from: sim_state_id,
            to: route_id,
            ty: make_type_id("edge:routePath"),
        },
    );

    // Set the route payload as an atom on routePath.
    let route_payload = AtomPayload::new(make_type_id("state:route_path"), payload_bytes);
    store.set_node_attachment(route_id, Some(AttachmentValue::Atom(route_payload)));
}
