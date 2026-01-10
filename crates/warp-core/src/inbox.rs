// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Inbox handling primitives for the website kernel spike.
//!
//! The inbox lives at `sim/inbox` under the current root instance and contains
//! deterministic event nodes produced during ingest. This module provides the
//! `dispatch_inbox` rewrite rule that drains those events so downstream command
//! rules can route them.

use crate::attachment::{AtomPayload, AttachmentKey, AttachmentOwner, AttachmentValue};
use crate::footprint::{AttachmentSet, Footprint, IdSet, PortSet};
use crate::graph::GraphStore;
use crate::ident::{make_edge_id, make_node_id, make_type_id, Hash, NodeId, NodeKey};
use crate::record::EdgeRecord;
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};

/// Human-readable name for the dispatch rule.
pub const DISPATCH_INBOX_RULE_NAME: &str = "sys/dispatch_inbox";

/// Constructs the `sys/dispatch_inbox` rewrite rule.
#[must_use]
pub fn dispatch_inbox_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:sys/dispatch_inbox").0;
    RewriteRule {
        id,
        name: DISPATCH_INBOX_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: inbox_matcher,
        executor: inbox_executor,
        compute_footprint: inbox_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn inbox_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    store
        .node(scope)
        .is_some_and(|n| n.ty == make_type_id("sim/inbox"))
}

fn inbox_executor(store: &mut GraphStore, scope: &NodeId) {
    // Drain events: route recognized intents, then delete every child node reachable via outgoing edges.
    let event_ids: Vec<NodeId> = store.edges_from(scope).map(|e| e.to).collect();
    for event_id in event_ids {
        route_event(store, &event_id);
        let _ = store.delete_node_cascade(event_id);
    }
    // Clear the inbox breadcrumb attachment if present.
    store.set_node_attachment(*scope, None);
}

fn inbox_footprint(store: &GraphStore, scope: &NodeId) -> Footprint {
    let mut n_read = IdSet::default();
    let mut n_write = IdSet::default();
    let mut e_write = IdSet::default();
    let mut a_write = AttachmentSet::default();

    n_read.insert_node(scope);
    n_write.insert_node(scope); // we mutate its attachment
    a_write.insert(AttachmentKey {
        owner: AttachmentOwner::Node(NodeKey {
            warp_id: store.warp_id(),
            local_id: *scope,
        }),
        plane: crate::attachment::AttachmentPlane::Alpha,
    });

    for e in store.edges_from(scope) {
        e_write.insert_edge(&e.id);
        n_write.insert_node(&e.to);
        a_write.insert(AttachmentKey {
            owner: AttachmentOwner::Node(NodeKey {
                warp_id: store.warp_id(),
                local_id: e.to,
            }),
            plane: crate::attachment::AttachmentPlane::Alpha,
        });
    }

    Footprint {
        n_read,
        n_write,
        e_read: IdSet::default(),
        e_write,
        a_read: AttachmentSet::default(),
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

fn route_event(store: &mut GraphStore, event_id: &NodeId) {
    let Some(atom) = store
        .node_attachment(event_id)
        .cloned()
        .and_then(|v| match v {
            AttachmentValue::Atom(a) => Some(a),
            AttachmentValue::Descend(_) => None,
        })
    else {
        return;
    };

    // Only handle route_push for now.
    if atom.type_id != make_type_id("intent:route_push") {
        return;
    }

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
    let route_payload = AtomPayload::new(make_type_id("state:route_path"), atom.bytes);
    store.set_node_attachment(route_id, Some(AttachmentValue::Atom(route_payload)));
}
