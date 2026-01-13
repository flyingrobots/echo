// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Inbox handling primitives for the website kernel spike.
//!
//! The inbox lives at `sim/inbox` under the current root instance and contains
//! deterministic event nodes produced during ingest. This module provides the
//! `dispatch_inbox` rewrite rule that drains those events so downstream command
//! rules can route them.

use crate::attachment::{AttachmentKey, AttachmentOwner};
use crate::footprint::{AttachmentSet, Footprint, IdSet, PortSet};
use crate::graph::GraphStore;
use crate::ident::{make_type_id, Hash, NodeId, NodeKey};
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
        && store.edges_from(scope).count() > 0
}

fn inbox_executor(store: &mut GraphStore, scope: &NodeId) {
    // Drain events: delete every child node reachable via outgoing edges.
    // Command execution is handled by separate rewrite rules registered with the engine
    // that match against the events in the inbox.
    let event_ids: Vec<NodeId> = store.edges_from(scope).map(|e| e.to).collect();
    for event_id in event_ids {
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

// NOTE: Intent routing logic lives in `crate::cmd` so it is shared between `sys/dispatch_inbox`
// and the standalone command rewrite rules (e.g. `cmd/route_push`).
