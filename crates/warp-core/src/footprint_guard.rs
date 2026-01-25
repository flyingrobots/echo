// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Footprint enforcement guard for BOAW Phase 6B.
//!
//! This module provides runtime validation that execute functions stay within
//! their declared footprints. Violations are reported via [`std::panic::panic_any`]
//! with a typed [`FootprintViolation`] payload, matchable via `downcast_ref` in tests.
//!
//! # Scope
//!
//! This is **graph footprint enforcement**: it validates that executors only read/write
//! graph resources (nodes, edges, attachments) they declared in their [`Footprint`].
//! Non-graph side effects (telemetry, caching, counters) are out of scope.
//!
//! # Cfg Gating
//!
//! The guard is active when `debug_assertions` is set (debug builds) or when the
//! `footprint_enforce_release` feature is enabled. The `unsafe_graph` feature
//! disables all enforcement regardless.
//!
//! # Panic Semantics
//!
//! Footprint violations panic with `panic_any(FootprintViolation)` because:
//!
//! - Violations are **programmer errors** (incorrect footprint declarations), not
//!   recoverable runtime conditions.
//! - Detection must be immediate and unambiguous to catch bugs early.
//! - Workers catch panics via `catch_unwind` in `execute_item_enforced`.
//!
//! On violation: the violating item's execution is aborted, its delta becomes a
//! `PoisonedDelta`, and the worker returns immediately (fail-fast). Poisoned
//! deltas abort the tick at merge time via `MergeError::PoisonedDelta`.
//!
//! This is NOT a recoverable runtime error; fix your footprint declarations.
//!
//! # Cross-Warp Write Policy
//!
//! Cross-warp writes are **forbidden**. Each rule executes within a single warp
//! scope and may only emit ops targeting that warp. Attempting to emit an op
//! targeting a different warp triggers [`ViolationKind::CrossWarpEmission`].
//!
//! This is a fundamental invariant of BOAW, not a temporary restriction. Inter-warp
//! communication flows through portals (attachment-based descent), not direct writes.

use std::any::Any;
use std::collections::BTreeSet;

use crate::attachment::{AttachmentKey, AttachmentOwner};
use crate::footprint::Footprint;
use crate::ident::{EdgeId, NodeId, WarpId};
use crate::tick_patch::WarpOp;

// ─────────────────────────────────────────────────────────────────────────────
// Violation types (public: integration tests + future sandboxes need these)
// ─────────────────────────────────────────────────────────────────────────────

/// Classification of a footprint violation.
///
/// Each variant identifies the specific access that was attempted outside
/// the declared footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationKind {
    /// Executor read a node not declared in `n_read`.
    NodeReadNotDeclared(NodeId),
    /// Executor read an edge not declared in `e_read`.
    EdgeReadNotDeclared(EdgeId),
    /// Executor read an attachment not declared in `a_read`.
    AttachmentReadNotDeclared(AttachmentKey),
    /// Executor emitted a node write not declared in `n_write`.
    NodeWriteNotDeclared(NodeId),
    /// Executor emitted an edge write not declared in `e_write`.
    EdgeWriteNotDeclared(EdgeId),
    /// Executor emitted an attachment write not declared in `a_write`.
    AttachmentWriteNotDeclared(AttachmentKey),
    /// Executor emitted an op targeting a different warp than the guard's scope.
    CrossWarpEmission {
        /// The warp the op was targeting.
        op_warp: WarpId,
    },
    /// A non-system rule emitted a warp-instance-level op.
    UnauthorizedInstanceOp,
    /// Safety net: an op was emitted with no warp scope and it's not
    /// an instance op. This is always a programmer error (system or user).
    /// Catches future match-arm omissions in `op_write_targets`.
    OpWarpUnknown,
}

/// Violation payload for [`std::panic::panic_any`].
///
/// Matchable via `downcast_ref::<FootprintViolation>()` in tests and
/// future sandboxes (Rhai/WASM/FFI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootprintViolation {
    /// Name of the rule that violated its footprint.
    pub rule_name: &'static str,
    /// Warp scope in which the violation occurred.
    pub warp_id: WarpId,
    /// Classification of the violation.
    pub kind: ViolationKind,
    /// The op variant or access type that triggered the violation.
    /// e.g. `"UpsertNode"`, `"node_read"`, `"edge_attachment_read"`.
    pub op_kind: &'static str,
}

/// Composite payload when an executor panic coincides with a write violation.
///
/// The violation remains primary, but the original executor panic is preserved
/// for inspection or rethrow by higher-level callers.
pub struct FootprintViolationWithPanic {
    /// The footprint violation that occurred during post-hoc validation.
    pub violation: FootprintViolation,
    /// The original executor panic payload.
    pub exec_panic: Box<dyn Any + Send + 'static>,
}

impl std::fmt::Debug for FootprintViolationWithPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Attempt to downcast exec_panic to common string types for readability
        let type_id_str: String;
        let panic_desc: &dyn std::fmt::Debug =
            if let Some(s) = self.exec_panic.downcast_ref::<&str>() {
                s
            } else if let Some(s) = self.exec_panic.downcast_ref::<String>() {
                s
            } else {
                // Fallback: show the actual TypeId for non-string payloads
                let type_id = (*self.exec_panic).type_id();
                type_id_str = format!("panic TypeId({type_id:?})");
                &type_id_str
            };

        f.debug_struct("FootprintViolationWithPanic")
            .field("violation", &self.violation)
            .field("exec_panic", panic_desc)
            .finish()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OpTargets: canonical write-target extraction from WarpOp
// ─────────────────────────────────────────────────────────────────────────────

/// Targets that a [`WarpOp`] writes to, as local ids within a specific warp.
///
/// This is the output of [`op_write_targets`] — the single source of truth for
/// what a `WarpOp` mutates. Used by enforcement. Available as a shared primitive
/// for future scheduling linting (but the scheduler does NOT currently use it).
pub(crate) struct OpTargets {
    /// Node ids that the op writes/mutates.
    pub nodes: Vec<NodeId>,
    /// Edge ids that the op writes/mutates.
    pub edges: Vec<EdgeId>,
    /// Attachment keys that the op writes/mutates.
    pub attachments: Vec<AttachmentKey>,
    /// Whether this is an instance-level op (`UpsertWarpInstance`/`DeleteWarpInstance`).
    pub is_instance_op: bool,
    /// The warp the op targets (for cross-warp check). `None` for instance-level ops
    /// without a specific target warp.
    pub op_warp: Option<WarpId>,
    /// Static string naming the op variant (e.g. `"UpsertNode"`).
    pub kind_str: &'static str,
}

/// Returns a static string naming the [`WarpOp`] variant.
///
/// Single source of truth — never manually type these strings elsewhere.
pub(crate) fn op_kind_str(op: &WarpOp) -> &'static str {
    match op {
        WarpOp::UpsertNode { .. } => "UpsertNode",
        WarpOp::DeleteNode { .. } => "DeleteNode",
        WarpOp::UpsertEdge { .. } => "UpsertEdge",
        WarpOp::DeleteEdge { .. } => "DeleteEdge",
        WarpOp::SetAttachment { .. } => "SetAttachment",
        WarpOp::OpenPortal { .. } => "OpenPortal",
        WarpOp::UpsertWarpInstance { .. } => "UpsertWarpInstance",
        WarpOp::DeleteWarpInstance { .. } => "DeleteWarpInstance",
    }
}

/// Canonical extraction of write targets from a [`WarpOp`].
///
/// This is the SINGLE SOURCE OF TRUTH for what a `WarpOp` mutates.
///
/// # Adjacency Model
///
/// `UpsertEdge`/`DeleteEdge` produce BOTH an edge write target (`edge_id`) AND a
/// node write target (`from`). This means any rule that inserts/removes edges MUST
/// declare `from` in `n_write` in its footprint.
///
/// **Why only `from`, not `to`?** Although `GraphStore` maintains reverse indexes
/// (`edge_to_index`, `edges_to`) internally, the execution API ([`GraphView`]) only
/// exposes `edges_from()` — rules cannot observe incoming edges. Since `to` adjacency
/// is not observable during execution, it doesn't require footprint declaration.
pub(crate) fn op_write_targets(op: &WarpOp) -> OpTargets {
    let kind_str = op_kind_str(op);

    match op {
        WarpOp::UpsertNode { node, .. } => OpTargets {
            nodes: vec![node.local_id],
            edges: Vec::new(),
            attachments: Vec::new(),
            is_instance_op: false,
            op_warp: Some(node.warp_id),
            kind_str,
        },
        WarpOp::DeleteNode { node } => OpTargets {
            // DeleteNode deletes node + its alpha attachment (allowed mini-cascade).
            // Footprint must declare both n_write(node) and a_write(node_alpha).
            nodes: vec![node.local_id],
            edges: Vec::new(),
            attachments: vec![AttachmentKey::node_alpha(*node)],
            is_instance_op: false,
            op_warp: Some(node.warp_id),
            kind_str,
        },
        WarpOp::UpsertEdge { warp_id, record } => OpTargets {
            // Adjacency write: edge mutation implies node adjacency mutation on `from`
            nodes: vec![record.from],
            edges: vec![record.id],
            attachments: Vec::new(),
            is_instance_op: false,
            op_warp: Some(*warp_id),
            kind_str,
        },
        WarpOp::DeleteEdge {
            warp_id,
            from,
            edge_id,
        } => OpTargets {
            // Adjacency write: edge deletion implies node adjacency mutation on `from`
            // DeleteEdge also removes the edge's attachment (allowed mini-cascade).
            nodes: vec![*from],
            edges: vec![*edge_id],
            attachments: vec![AttachmentKey::edge_beta(crate::ident::EdgeKey {
                warp_id: *warp_id,
                local_id: *edge_id,
            })],
            is_instance_op: false,
            op_warp: Some(*warp_id),
            kind_str,
        },
        WarpOp::SetAttachment { key, .. } => OpTargets {
            nodes: Vec::new(),
            edges: Vec::new(),
            attachments: vec![*key],
            is_instance_op: false,
            op_warp: Some(key.owner.warp_id()),
            kind_str,
        },
        WarpOp::OpenPortal { key, .. } => OpTargets {
            nodes: Vec::new(),
            edges: Vec::new(),
            attachments: vec![*key],
            is_instance_op: true,
            op_warp: Some(key.owner.warp_id()),
            kind_str,
        },
        WarpOp::UpsertWarpInstance { .. } => OpTargets {
            nodes: Vec::new(),
            edges: Vec::new(),
            attachments: Vec::new(),
            is_instance_op: true,
            op_warp: None,
            kind_str,
        },
        WarpOp::DeleteWarpInstance { warp_id } => OpTargets {
            nodes: Vec::new(),
            edges: Vec::new(),
            attachments: Vec::new(),
            is_instance_op: true,
            op_warp: Some(*warp_id),
            kind_str,
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FootprintGuard: runtime enforcement of declared footprints
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime guard enforcing declared footprints on executor read/write access.
///
/// Constructed from a [`Footprint`] by pre-filtering to local ids within the
/// guard's warp. Read checks are called by [`GraphView`](crate::GraphView) methods;
/// write checks are called post-hoc on emitted [`WarpOp`]s.
///
/// # Key Type Invariant
///
/// `nodes_read`/`nodes_write` store `NodeId` (bare local id).
/// `edges_read`/`edges_write` store `EdgeId` (bare local id).
/// These match EXACTLY what `GraphView` methods receive as parameters.
///
/// # Why `BTreeSet`?
///
/// `BTreeSet` is chosen for deterministic debug output and iteration order, aiding
/// reproducibility when violations are logged. Footprints are typically small
/// (< 100 items), so the O(log n) lookup cost is negligible.
#[derive(Debug)]
pub(crate) struct FootprintGuard {
    warp_id: WarpId,
    // BTreeSet for deterministic iteration/debug output; see doc above.
    nodes_read: BTreeSet<NodeId>,
    nodes_write: BTreeSet<NodeId>,
    edges_read: BTreeSet<EdgeId>,
    edges_write: BTreeSet<EdgeId>,
    attachments_read: BTreeSet<AttachmentKey>,
    attachments_write: BTreeSet<AttachmentKey>,
    rule_name: &'static str,
    is_system: bool,
}

#[allow(clippy::panic)]
impl FootprintGuard {
    /// Constructs a guard from a footprint, pre-filtering to local ids within `warp_id`.
    pub(crate) fn new(
        footprint: &Footprint,
        warp_id: WarpId,
        rule_name: &'static str,
        is_system: bool,
    ) -> Self {
        let nodes_read = footprint
            .n_read
            .iter()
            .filter(|k| k.warp_id == warp_id)
            .map(|k| k.local_id)
            .collect();
        let nodes_write = footprint
            .n_write
            .iter()
            .filter(|k| k.warp_id == warp_id)
            .map(|k| k.local_id)
            .collect();
        let edges_read = footprint
            .e_read
            .iter()
            .filter(|k| k.warp_id == warp_id)
            .map(|k| k.local_id)
            .collect();
        let edges_write = footprint
            .e_write
            .iter()
            .filter(|k| k.warp_id == warp_id)
            .map(|k| k.local_id)
            .collect();
        let attachments_read = footprint
            .a_read
            .iter()
            .filter(|k| k.owner.warp_id() == warp_id)
            .copied()
            .collect();
        let attachments_write = footprint
            .a_write
            .iter()
            .filter(|k| k.owner.warp_id() == warp_id)
            .copied()
            .collect();

        Self {
            warp_id,
            nodes_read,
            nodes_write,
            edges_read,
            edges_write,
            attachments_read,
            attachments_write,
            rule_name,
            is_system,
        }
    }

    /// Panics if the node is not declared in the read set.
    pub(crate) fn check_node_read(&self, id: &NodeId) {
        if !self.nodes_read.contains(id) {
            std::panic::panic_any(FootprintViolation {
                rule_name: self.rule_name,
                warp_id: self.warp_id,
                kind: ViolationKind::NodeReadNotDeclared(*id),
                op_kind: "node_read",
            });
        }
    }

    /// Panics if the edge is not declared in the read set.
    pub(crate) fn check_edge_read(&self, id: &EdgeId) {
        if !self.edges_read.contains(id) {
            std::panic::panic_any(FootprintViolation {
                rule_name: self.rule_name,
                warp_id: self.warp_id,
                kind: ViolationKind::EdgeReadNotDeclared(*id),
                op_kind: "edge_read",
            });
        }
    }

    /// Panics if the attachment is not declared in the read set.
    pub(crate) fn check_attachment_read(&self, key: &AttachmentKey) {
        if !self.attachments_read.contains(key) {
            std::panic::panic_any(FootprintViolation {
                rule_name: self.rule_name,
                warp_id: self.warp_id,
                kind: ViolationKind::AttachmentReadNotDeclared(*key),
                op_kind: match key.owner {
                    AttachmentOwner::Node(_) => "node_attachment_read",
                    AttachmentOwner::Edge(_) => "edge_attachment_read",
                },
            });
        }
    }

    /// Validates a single emitted op against the write footprint.
    ///
    /// Checks (in order):
    /// 1. Instance-level ops require `is_system`
    /// 2. Op warp must match guard's warp (cross-warp rejection)
    /// 3. Missing `op_warp` on non-instance ops is always an error
    /// 4. Node/edge/attachment targets must be in the write sets
    pub(crate) fn check_op(&self, op: &WarpOp) {
        let targets = op_write_targets(op);

        // 1. Instance-level ops blocked for user rules
        if targets.is_instance_op && !self.is_system {
            std::panic::panic_any(FootprintViolation {
                rule_name: self.rule_name,
                warp_id: self.warp_id,
                kind: ViolationKind::UnauthorizedInstanceOp,
                op_kind: targets.kind_str,
            });
        }

        // 2. Cross-warp check
        if let Some(op_warp) = targets.op_warp {
            if op_warp != self.warp_id {
                std::panic::panic_any(FootprintViolation {
                    rule_name: self.rule_name,
                    warp_id: self.warp_id,
                    kind: ViolationKind::CrossWarpEmission { op_warp },
                    op_kind: targets.kind_str,
                });
            }
        } else if !targets.is_instance_op {
            // 3. Missing op_warp on non-instance op: always a programmer error
            std::panic::panic_any(FootprintViolation {
                rule_name: self.rule_name,
                warp_id: self.warp_id,
                kind: ViolationKind::OpWarpUnknown,
                op_kind: targets.kind_str,
            });
        }

        // 4. Write-set checks
        for n in &targets.nodes {
            if !self.nodes_write.contains(n) {
                std::panic::panic_any(FootprintViolation {
                    rule_name: self.rule_name,
                    warp_id: self.warp_id,
                    kind: ViolationKind::NodeWriteNotDeclared(*n),
                    op_kind: targets.kind_str,
                });
            }
        }
        for e in &targets.edges {
            if !self.edges_write.contains(e) {
                std::panic::panic_any(FootprintViolation {
                    rule_name: self.rule_name,
                    warp_id: self.warp_id,
                    kind: ViolationKind::EdgeWriteNotDeclared(*e),
                    op_kind: targets.kind_str,
                });
            }
        }
        for a in &targets.attachments {
            if !self.attachments_write.contains(a) {
                std::panic::panic_any(FootprintViolation {
                    rule_name: self.rule_name,
                    warp_id: self.warp_id,
                    kind: ViolationKind::AttachmentWriteNotDeclared(*a),
                    op_kind: targets.kind_str,
                });
            }
        }
    }
}
