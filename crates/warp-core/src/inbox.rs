// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Inbox handling primitives for the website kernel spike.
//!
//! The inbox lives at `sim/inbox` under the current root instance and contains
//! deterministic event nodes produced during ingest.
//!
//! # Ledger vs. Queue Maintenance
//!
//! Inbox **event nodes** are treated as immutable, append-only ledger entries.
//! Pending vs. processed is tracked via **edges**, not by deleting ledger nodes.
//! In the minimal model:
//! - A pending intent is represented by a `edge:pending` edge from `sim/inbox`
//!   to the event node.
//! - When the intent is consumed, the pending edge is deleted as **queue
//!   maintenance**; the event node remains in the graph forever.
//!
//! This module provides:
//! - `sys/dispatch_inbox`: drains all pending edges from the inbox (legacy helper)
//! - `sys/ack_pending`: consumes exactly one pending edge for an event scope

use blake3::Hasher;

use crate::footprint::{AttachmentSet, Footprint, IdSet, PortSet};
use crate::graph_view::GraphView;
use crate::ident::{make_node_id, make_type_id, EdgeId, Hash, NodeId};
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};
use crate::tick_patch::WarpOp;
use crate::TickDelta;

/// Human-readable name for the dispatch rule.
pub const DISPATCH_INBOX_RULE_NAME: &str = "sys/dispatch_inbox";

/// Human-readable name for the pending-edge acknowledgment rule.
pub const ACK_PENDING_RULE_NAME: &str = "sys/ack_pending";

/// Node path for the simulation inbox.
pub const INBOX_PATH: &str = "sim/inbox";

/// Type identifier for inbox event nodes.
pub const INBOX_EVENT_TYPE: &str = "sim/inbox/event";

/// Type identifier for pending edges.
pub const PENDING_EDGE_TYPE: &str = "edge:pending";

/// Type identifier for intent attachments.
pub const INTENT_ATTACHMENT_TYPE: &str = "intent";

/// Hash domain prefix for pending edge IDs.
const PENDING_EDGE_HASH_DOMAIN: &[u8] = b"sim/inbox/pending:";

/// Constructs the `sys/dispatch_inbox` rewrite rule.
///
/// This rule drains all pending events from `sim/inbox` by deleting the
/// `edge:pending` edges that link the inbox to event nodes. Event nodes
/// themselves are preserved (ledger is append-only); only the pending
/// markers are removed as queue maintenance.
///
/// # Matching
/// The rule matches when the scope node has type `sim/inbox` and has at
/// least one outgoing `edge:pending` edge.
///
/// # Effects
/// - Deletes all `edge:pending` edges from the inbox node.
/// - Does NOT delete the event nodes (ledger entries remain).
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

/// Constructs the `sys/ack_pending` rewrite rule.
///
/// This rule deletes the `edge:pending` edge corresponding to the provided
/// `scope` event node, treating edge deletion as queue maintenance (not ledger deletion).
#[must_use]
pub fn ack_pending_rule() -> RewriteRule {
    let id: Hash = make_type_id("rule:sys/ack_pending").0;
    RewriteRule {
        id,
        name: ACK_PENDING_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: ack_pending_matcher,
        executor: ack_pending_executor,
        compute_footprint: ack_pending_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn inbox_matcher(view: GraphView<'_>, scope: &NodeId) -> bool {
    let pending_ty = make_type_id(PENDING_EDGE_TYPE);
    view.node(scope)
        .is_some_and(|n| n.ty == make_type_id(INBOX_PATH))
        && view.edges_from(scope).any(|e| e.ty == pending_ty)
}

fn inbox_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    // Drain the pending set by deleting `edge:pending` edges only.
    //
    // Ledger nodes are append-only; removing pending edges is queue maintenance.
    // Phase 5 BOAW: read from view, emit ops to delta (no direct mutation).
    let warp_id = view.warp_id();
    let pending_ty = make_type_id(PENDING_EDGE_TYPE);

    // Gather pending edges and emit delete ops
    for edge in view.edges_from(scope) {
        if edge.ty == pending_ty {
            delta.push(WarpOp::DeleteEdge {
                warp_id,
                from: *scope,
                edge_id: edge.id,
            });
        }
    }
}

fn inbox_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = IdSet::default();
    let mut e_read = IdSet::default();
    let mut e_write = IdSet::default();
    let pending_ty = make_type_id(PENDING_EDGE_TYPE);

    n_read.insert_node(scope);

    for e in view.edges_from(scope) {
        if e.ty != pending_ty {
            continue;
        }
        // Record edge read for conflict detection before writing
        e_read.insert_edge(&e.id);
        e_write.insert_edge(&e.id);
    }

    Footprint {
        n_read,
        n_write: IdSet::default(),
        e_read,
        e_write,
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

fn ack_pending_matcher(view: GraphView<'_>, scope: &NodeId) -> bool {
    let inbox_id = make_node_id(INBOX_PATH);
    let edge_id = pending_edge_id(&inbox_id, &scope.0);
    view.has_edge(&edge_id)
}

fn ack_pending_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    // Phase 5 BOAW: read from view, emit ops to delta (no direct mutation).
    let warp_id = view.warp_id();
    let inbox_id = make_node_id(INBOX_PATH);
    let edge_id = pending_edge_id(&inbox_id, &scope.0);

    delta.push(WarpOp::DeleteEdge {
        warp_id,
        from: inbox_id,
        edge_id,
    });
}

fn ack_pending_footprint(_view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = IdSet::default();
    let mut e_read = IdSet::default();
    let mut e_write = IdSet::default();

    let inbox_id = make_node_id(INBOX_PATH);
    n_read.insert_node(&inbox_id);
    n_read.insert_node(scope);

    let edge_id = pending_edge_id(&inbox_id, &scope.0);
    // Record edge read for conflict detection before writing
    e_read.insert_edge(&edge_id);
    e_write.insert_edge(&edge_id);

    Footprint {
        n_read,
        n_write: IdSet::default(),
        e_read,
        e_write,
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

pub(crate) fn compute_intent_id(intent_bytes: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(b"intent:");
    hasher.update(intent_bytes);
    hasher.finalize().into()
}

pub(crate) fn pending_edge_id(inbox_id: &NodeId, intent_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(b"edge:");
    hasher.update(PENDING_EDGE_HASH_DOMAIN);
    hasher.update(inbox_id.as_bytes());
    hasher.update(intent_id);
    EdgeId(hasher.finalize().into())
}

// NOTE: Intent routing logic lives in `crate::cmd` so it is shared between `sys/dispatch_inbox`
// and the standalone command rewrite rules (e.g. `cmd/route_push`).
