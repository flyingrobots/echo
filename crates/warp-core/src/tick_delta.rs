// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tick delta accumulator for rule-engine op collection.
//!
//! [`TickDelta`] collects [`WarpOp`]s emitted during rule execution,
//! optionally tracking their origin for future tie-breaking, then
//! finalizes them into canonical replay order.

use crate::tick_patch::WarpOp;

/// Origin metadata for a collected operation.
///
/// This metadata supports future canonical tie-breaking when multiple
/// rules produce semantically equivalent operations in the same tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpOrigin {
    /// Intent ID (for future canonical ordering).
    pub intent_id: u64,
    /// Rule ID (compact form).
    pub rule_id: u32,
    /// Match index within rule.
    pub match_ix: u32,
    /// Operation index within this scoped emission (auto-assigned by `ScopedDelta`).
    pub op_ix: u32,
}

/// Accumulates [`WarpOp`]s during rule execution.
///
/// The delta collects operations as they are produced, then finalizes
/// them into canonical order for patch construction.
///
/// # Origin Tracking
///
/// When the `delta_validate` feature is enabled (or in tests), the delta
/// also tracks [`OpOrigin`] metadata for each operation. This metadata
/// is stored in a separate vector and is not included in the finalized
/// output.
pub struct TickDelta {
    ops: Vec<WarpOp>,
    #[cfg(any(test, feature = "delta_validate"))]
    origins: Vec<OpOrigin>,
}

impl TickDelta {
    /// Creates a new empty delta.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            #[cfg(any(test, feature = "delta_validate"))]
            origins: Vec::new(),
        }
    }

    /// Emits an operation with no origin metadata.
    ///
    /// This is the primary method for adding operations to the delta.
    /// When origin tracking is enabled, a default [`OpOrigin`] is recorded.
    pub fn emit(&mut self, op: WarpOp) {
        self.ops.push(op);
        #[cfg(any(test, feature = "delta_validate"))]
        self.origins.push(OpOrigin::default());
    }

    /// Emits an operation with explicit origin metadata.
    ///
    /// Use this when the origin of an operation needs to be tracked
    /// for debugging, validation, or future tie-breaking logic.
    pub fn emit_with_origin(&mut self, op: WarpOp, origin: OpOrigin) {
        self.ops.push(op);
        #[cfg(any(test, feature = "delta_validate"))]
        self.origins.push(origin);
        #[cfg(not(any(test, feature = "delta_validate")))]
        let _ = origin; // Suppress unused warning in release builds
    }

    /// Pushes an operation with no origin metadata.
    ///
    /// This is an alias for [`emit()`](Self::emit) for backward compatibility.
    #[inline]
    pub fn push(&mut self, op: WarpOp) {
        self.emit(op);
    }

    /// Pushes an operation with origin metadata for tie-breaking.
    ///
    /// This is an alias for [`emit_with_origin()`](Self::emit_with_origin)
    /// for backward compatibility.
    #[inline]
    pub fn push_with_origin(&mut self, op: WarpOp, origin: OpOrigin) {
        self.emit_with_origin(op, origin);
    }

    /// Returns the number of collected operations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Returns `true` if no operations have been collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Finalizes the delta into canonically sorted operations.
    ///
    /// Operations are sorted by [`WarpOp::sort_key()`] to ensure
    /// deterministic replay order. Origin metadata is discarded.
    #[must_use]
    pub fn finalize(mut self) -> Vec<WarpOp> {
        self.ops.sort_by_key(WarpOp::sort_key);
        self.ops
    }

    /// Returns the operations without sorting (for testing).
    ///
    /// This preserves insertion order and is useful for verifying
    /// that operations were collected in the expected sequence.
    #[must_use]
    pub fn into_ops_unsorted(self) -> Vec<WarpOp> {
        self.ops
    }

    /// Returns both ops and origins without sorting (for testing/validation).
    ///
    /// This preserves insertion order and is useful for verifying
    /// that operations and their origins were collected correctly.
    #[cfg(any(test, feature = "delta_validate"))]
    #[must_use]
    pub fn into_parts_unsorted(self) -> (Vec<WarpOp>, Vec<OpOrigin>) {
        (self.ops, self.origins)
    }

    /// Creates a [`ScopedDelta`] with the given origin.
    ///
    /// This is a convenience method for creating a scoped delta that
    /// applies the same origin metadata to all emitted operations.
    pub fn scoped(&mut self, origin: OpOrigin) -> ScopedDelta<'_> {
        ScopedDelta::new(self, origin)
    }

    /// Computes statistics about the collected operations.
    #[must_use]
    pub fn stats(&self) -> DeltaStats {
        let mut stats = DeltaStats::default();
        for op in &self.ops {
            match op {
                WarpOp::UpsertNode { .. } => stats.upsert_node += 1,
                WarpOp::DeleteNode { .. } => stats.delete_node += 1,
                WarpOp::UpsertEdge { .. } => stats.upsert_edge += 1,
                WarpOp::DeleteEdge { .. } => stats.delete_edge += 1,
                WarpOp::SetAttachment { .. } => stats.set_attachment += 1,
                WarpOp::OpenPortal { .. } => stats.open_portal += 1,
                WarpOp::UpsertWarpInstance { .. } => stats.upsert_warp_instance += 1,
                WarpOp::DeleteWarpInstance { .. } => stats.delete_warp_instance += 1,
            }
        }
        stats
    }

    /// Returns the collected origins (only available with `delta_validate` feature).
    ///
    /// This is useful for debugging and validation to see which rule/intent
    /// produced each operation.
    #[cfg(any(test, feature = "delta_validate"))]
    #[must_use]
    pub fn origins(&self) -> &[OpOrigin] {
        &self.origins
    }
}

impl Default for TickDelta {
    fn default() -> Self {
        Self::new()
    }
}

/// A scoped wrapper around [`TickDelta`] that applies a default origin to all emitted ops.
///
/// This is useful when a rule executor wants to emit multiple operations with the
/// same origin metadata without repeating the origin on each call.
///
/// # Example
///
/// ```ignore
/// let mut delta = TickDelta::new();
/// let origin = OpOrigin { intent_id: 42, rule_id: 1, match_ix: 0 };
/// let mut scoped = ScopedDelta::new(&mut delta, origin);
///
/// // All operations emitted through `scoped` will have the same origin
/// scoped.emit(WarpOp::UpsertNode { ... });
/// scoped.emit(WarpOp::SetAttachment { ... });
/// ```
pub struct ScopedDelta<'a> {
    inner: &'a mut TickDelta,
    origin: OpOrigin,
    next_op_ix: u32,
}

impl<'a> ScopedDelta<'a> {
    /// Creates a new scoped delta with the given default origin.
    pub fn new(delta: &'a mut TickDelta, origin: OpOrigin) -> Self {
        Self {
            inner: delta,
            origin,
            next_op_ix: 0,
        }
    }

    /// Emits an operation with the scoped origin (auto-assigns `op_ix`).
    pub fn emit(&mut self, op: WarpOp) {
        let mut origin = self.origin;
        origin.op_ix = self.next_op_ix;
        self.next_op_ix += 1;
        self.inner.emit_with_origin(op, origin);
    }

    /// Returns a reference to the underlying [`TickDelta`].
    pub fn inner(&self) -> &TickDelta {
        self.inner
    }

    /// Returns a mutable reference to the underlying [`TickDelta`].
    pub fn inner_mut(&mut self) -> &mut TickDelta {
        self.inner
    }
}

/// Statistics about operations in a [`TickDelta`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeltaStats {
    /// Count of `UpsertNode` operations.
    pub upsert_node: usize,
    /// Count of `DeleteNode` operations.
    pub delete_node: usize,
    /// Count of `UpsertEdge` operations.
    pub upsert_edge: usize,
    /// Count of `DeleteEdge` operations.
    pub delete_edge: usize,
    /// Count of `SetAttachment` operations.
    pub set_attachment: usize,
    /// Count of `OpenPortal` operations.
    pub open_portal: usize,
    /// Count of `UpsertWarpInstance` operations.
    pub upsert_warp_instance: usize,
    /// Count of `DeleteWarpInstance` operations.
    pub delete_warp_instance: usize,
}

// ============================================================================
// Delta validation helpers
// ============================================================================

/// Captures details about a mismatch between delta ops and diff ops.
///
/// This struct provides programmatic access to mismatch details for testing
/// and debugging, as an alternative to the panicking [`assert_delta_matches_diff`].
#[derive(Debug)]
#[cfg(any(test, feature = "delta_validate"))]
pub struct DeltaMismatch {
    /// Number of operations in the delta (executor-emitted) ops.
    pub delta_len: usize,
    /// Number of operations in the diff (state-diffed) ops.
    pub diff_len: usize,
    /// Index of the first mismatching operation, if lengths match.
    pub first_mismatch_index: Option<usize>,
    /// Statistics for delta ops.
    pub delta_stats: DeltaStats,
    /// Statistics for diff ops.
    pub diff_stats: DeltaStats,
}

#[cfg(any(test, feature = "delta_validate"))]
impl std::fmt::Display for DeltaMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Delta mismatch detected:")?;
        writeln!(f, "  delta_len: {}", self.delta_len)?;
        writeln!(f, "  diff_len:  {}", self.diff_len)?;
        if let Some(idx) = self.first_mismatch_index {
            writeln!(f, "  first_mismatch_index: {idx}")?;
        }
        writeln!(f, "  delta_stats: {:?}", self.delta_stats)?;
        writeln!(f, "  diff_stats:  {:?}", self.diff_stats)?;
        Ok(())
    }
}

#[cfg(any(test, feature = "delta_validate"))]
impl std::error::Error for DeltaMismatch {}

/// Formats a [`WarpOp`] compactly for error messages.
///
/// Produces output like:
/// - `"UpsertNode(warp:abc123, node:def456)"`
/// - `"SetAttachment(node_alpha:xyz789)"`
/// - `"DeleteEdge(warp:abc, from:def, id:ghi)"`
#[cfg(any(test, feature = "delta_validate"))]
pub fn format_op_compact(op: &WarpOp) -> String {
    use crate::attachment::AttachmentOwner;

    /// Formats a hash as a short hex prefix (first 6 chars).
    fn short_hash(h: &[u8; 32]) -> String {
        hex::encode(&h[..3])
    }

    match op {
        WarpOp::OpenPortal {
            key,
            child_warp,
            child_root,
            ..
        } => {
            let owner_str = match key.owner {
                AttachmentOwner::Node(node) => {
                    format!(
                        "node_alpha(warp:{}, node:{})",
                        short_hash(&node.warp_id.0),
                        short_hash(&node.local_id.0)
                    )
                }
                AttachmentOwner::Edge(edge) => {
                    format!(
                        "edge_beta(warp:{}, edge:{})",
                        short_hash(&edge.warp_id.0),
                        short_hash(&edge.local_id.0)
                    )
                }
            };
            format!(
                "OpenPortal({owner_str}, child_warp:{}, child_root:{})",
                short_hash(&child_warp.0),
                short_hash(&child_root.0)
            )
        }
        WarpOp::UpsertWarpInstance { instance } => {
            format!(
                "UpsertWarpInstance(warp:{}, root:{})",
                short_hash(&instance.warp_id.0),
                short_hash(&instance.root_node.0)
            )
        }
        WarpOp::DeleteWarpInstance { warp_id } => {
            format!("DeleteWarpInstance(warp:{})", short_hash(&warp_id.0))
        }
        WarpOp::UpsertNode { node, record } => {
            format!(
                "UpsertNode(warp:{}, node:{}, ty:{})",
                short_hash(&node.warp_id.0),
                short_hash(&node.local_id.0),
                short_hash(&record.ty.0)
            )
        }
        WarpOp::DeleteNode { node } => {
            format!(
                "DeleteNode(warp:{}, node:{})",
                short_hash(&node.warp_id.0),
                short_hash(&node.local_id.0)
            )
        }
        WarpOp::UpsertEdge { warp_id, record } => {
            format!(
                "UpsertEdge(warp:{}, from:{}, id:{}, to:{})",
                short_hash(&warp_id.0),
                short_hash(&record.from.0),
                short_hash(&record.id.0),
                short_hash(&record.to.0)
            )
        }
        WarpOp::DeleteEdge {
            warp_id,
            from,
            edge_id,
        } => {
            format!(
                "DeleteEdge(warp:{}, from:{}, id:{})",
                short_hash(&warp_id.0),
                short_hash(&from.0),
                short_hash(&edge_id.0)
            )
        }
        WarpOp::SetAttachment { key, value } => {
            let owner_str = match key.owner {
                AttachmentOwner::Node(node) => {
                    format!("node_alpha:{}", short_hash(&node.local_id.0))
                }
                AttachmentOwner::Edge(edge) => {
                    format!("edge_beta:{}", short_hash(&edge.local_id.0))
                }
            };
            let value_str = match value {
                None => "None".to_string(),
                Some(crate::attachment::AttachmentValue::Atom(_)) => "Atom(...)".to_string(),
                Some(crate::attachment::AttachmentValue::Descend(warp_id)) => {
                    format!("Descend({})", short_hash(&warp_id.0))
                }
            };
            format!("SetAttachment({owner_str}, {value_str})")
        }
    }
}

/// Computes statistics for a slice of [`WarpOp`]s.
#[cfg(any(test, feature = "delta_validate"))]
fn compute_stats(ops: &[WarpOp]) -> DeltaStats {
    let mut stats = DeltaStats::default();
    for op in ops {
        match op {
            WarpOp::UpsertNode { .. } => stats.upsert_node += 1,
            WarpOp::DeleteNode { .. } => stats.delete_node += 1,
            WarpOp::UpsertEdge { .. } => stats.upsert_edge += 1,
            WarpOp::DeleteEdge { .. } => stats.delete_edge += 1,
            WarpOp::SetAttachment { .. } => stats.set_attachment += 1,
            WarpOp::OpenPortal { .. } => stats.open_portal += 1,
            WarpOp::UpsertWarpInstance { .. } => stats.upsert_warp_instance += 1,
            WarpOp::DeleteWarpInstance { .. } => stats.delete_warp_instance += 1,
        }
    }
    stats
}

/// Canonicalizes ops by sorting by `sort_key`.
#[cfg(any(test, feature = "delta_validate"))]
fn canonicalize_ops(ops: &[WarpOp]) -> Vec<WarpOp> {
    let mut sorted = ops.to_vec();
    sorted.sort_by_key(WarpOp::sort_key);
    sorted
}

/// Validates that ops emitted by executors match ops from `diff_state()`.
///
/// This is the non-panicking version that returns a [`DeltaMismatch`] on failure.
///
/// # Arguments
/// * `delta_ops` - Operations emitted by rule executors (will be canonicalized).
/// * `diff_ops` - Operations from `diff_state()` (will be canonicalized).
///
/// # Returns
/// `Ok(())` if the canonicalized ops match exactly, or `Err(DeltaMismatch)` with
/// detailed information about the first difference.
#[cfg(any(test, feature = "delta_validate"))]
pub fn validate_delta_matches_diff(
    delta_ops: &[WarpOp],
    diff_ops: &[WarpOp],
) -> Result<(), Box<DeltaMismatch>> {
    let delta_sorted = canonicalize_ops(delta_ops);
    let diff_sorted = canonicalize_ops(diff_ops);

    let delta_stats = compute_stats(&delta_sorted);
    let diff_stats = compute_stats(&diff_sorted);

    // Check lengths first
    if delta_sorted.len() != diff_sorted.len() {
        return Err(Box::new(DeltaMismatch {
            delta_len: delta_sorted.len(),
            diff_len: diff_sorted.len(),
            first_mismatch_index: None,
            delta_stats,
            diff_stats,
        }));
    }

    // Find first mismatch
    for (i, (delta_op, diff_op)) in delta_sorted.iter().zip(diff_sorted.iter()).enumerate() {
        if delta_op.sort_key() != diff_op.sort_key() {
            return Err(Box::new(DeltaMismatch {
                delta_len: delta_sorted.len(),
                diff_len: diff_sorted.len(),
                first_mismatch_index: Some(i),
                delta_stats,
                diff_stats,
            }));
        }
    }

    Ok(())
}

/// Validates that ops emitted by executors match ops from `diff_state()`.
///
/// Panics with a detailed diff on mismatch, useful for test assertions.
///
/// # Arguments
/// * `delta_ops` - Operations emitted by rule executors (will be canonicalized).
/// * `diff_ops` - Operations from `diff_state()` (will be canonicalized).
///
/// # Panics
/// Panics if the canonicalized ops do not match, printing:
/// - Length comparison if different
/// - First mismatching operation with compact formatting
/// - Statistics for both op sets
#[cfg(any(test, feature = "delta_validate"))]
#[allow(clippy::panic)]
pub fn assert_delta_matches_diff(delta_ops: &[WarpOp], diff_ops: &[WarpOp]) {
    use std::fmt::Write;

    if let Err(mismatch) = validate_delta_matches_diff(delta_ops, diff_ops) {
        let delta_sorted = canonicalize_ops(delta_ops);
        let diff_sorted = canonicalize_ops(diff_ops);

        let mut msg = String::new();
        msg.push_str("\n========== DELTA MISMATCH ==========\n");

        // Length info
        let _ = writeln!(
            msg,
            "Lengths: delta={}, diff={}",
            mismatch.delta_len, mismatch.diff_len
        );

        // Stats comparison
        msg.push_str("\nOperation statistics:\n");
        let _ = writeln!(msg, "  delta_stats: {:?}", mismatch.delta_stats);
        let _ = writeln!(msg, "  diff_stats:  {:?}", mismatch.diff_stats);

        // First mismatch details
        if let Some(idx) = mismatch.first_mismatch_index {
            let _ = writeln!(msg, "\nFirst mismatch at index {idx}:");
            if idx < delta_sorted.len() {
                let _ = writeln!(
                    msg,
                    "  delta[{idx}]: {}",
                    format_op_compact(&delta_sorted[idx])
                );
            }
            if idx < diff_sorted.len() {
                let _ = writeln!(
                    msg,
                    "  diff[{idx}]:  {}",
                    format_op_compact(&diff_sorted[idx])
                );
            }
        } else if mismatch.delta_len != mismatch.diff_len {
            // Length mismatch - show the extra ops
            let min_len = mismatch.delta_len.min(mismatch.diff_len);
            if mismatch.delta_len > min_len {
                msg.push_str("\nExtra ops in delta:\n");
                for (i, op) in delta_sorted.iter().enumerate().skip(min_len).take(5) {
                    let _ = writeln!(msg, "  [{i}]: {}", format_op_compact(op));
                }
                if mismatch.delta_len - min_len > 5 {
                    let _ = writeln!(msg, "  ... and {} more", mismatch.delta_len - min_len - 5);
                }
            }
            if mismatch.diff_len > min_len {
                msg.push_str("\nExtra ops in diff:\n");
                for (i, op) in diff_sorted.iter().enumerate().skip(min_len).take(5) {
                    let _ = writeln!(msg, "  [{i}]: {}", format_op_compact(op));
                }
                if mismatch.diff_len - min_len > 5 {
                    let _ = writeln!(msg, "  ... and {} more", mismatch.diff_len - min_len - 5);
                }
            }
        }

        msg.push_str("\n=====================================\n");
        panic!("{msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::{AttachmentKey, AttachmentValue};
    use crate::ident::{make_node_id, make_type_id, make_warp_id, NodeKey};
    use crate::record::NodeRecord;
    use crate::tick_patch::PortalInit;
    use crate::warp_state::WarpInstance;

    #[test]
    fn finalize_produces_canonically_sorted_ops() {
        let warp_id = make_warp_id("test-warp");
        let node_a = make_node_id("node-a");
        let node_b = make_node_id("node-b");

        let mut delta = TickDelta::new();

        // Push in non-canonical order: SetAttachment should sort after UpsertNode
        delta.push(WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(NodeKey {
                warp_id,
                local_id: node_a,
            }),
            value: Some(AttachmentValue::Atom(crate::attachment::AtomPayload {
                type_id: make_type_id("test"),
                bytes: vec![1, 2, 3].into(),
            })),
        });

        delta.push(WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_b,
            },
            record: NodeRecord {
                ty: make_type_id("TestNode"),
            },
        });

        delta.push(WarpOp::DeleteNode {
            node: NodeKey {
                warp_id,
                local_id: node_a,
            },
        });

        let ops = delta.finalize();
        assert_eq!(ops.len(), 3);

        // Canonical order: DeleteNode (kind 5), UpsertNode (kind 6), SetAttachment (kind 8)
        assert!(matches!(ops[0], WarpOp::DeleteNode { .. }));
        assert!(matches!(ops[1], WarpOp::UpsertNode { .. }));
        assert!(matches!(ops[2], WarpOp::SetAttachment { .. }));
    }

    #[test]
    fn stats_counts_correctly() {
        let warp_id = make_warp_id("stats-warp");
        let node_id = make_node_id("stats-node");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };
        let child_warp = make_warp_id("child-warp");
        let child_root = make_node_id("child-root");

        let mut delta = TickDelta::new();

        // Add various ops
        delta.push(WarpOp::UpsertNode {
            node: node_key,
            record: NodeRecord {
                ty: make_type_id("TestNode"),
            },
        });
        delta.push(WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: make_node_id("node-2"),
            },
            record: NodeRecord {
                ty: make_type_id("TestNode"),
            },
        });
        delta.push(WarpOp::DeleteNode {
            node: NodeKey {
                warp_id,
                local_id: make_node_id("deleted-node"),
            },
        });
        delta.push(WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(node_key),
            value: None,
        });
        delta.push(WarpOp::OpenPortal {
            key: AttachmentKey::node_alpha(node_key),
            child_warp,
            child_root,
            init: PortalInit::Empty {
                root_record: NodeRecord {
                    ty: make_type_id("ChildRoot"),
                },
            },
        });
        delta.push(WarpOp::UpsertWarpInstance {
            instance: WarpInstance {
                warp_id,
                root_node: node_id,
                parent: None,
            },
        });
        delta.push(WarpOp::DeleteWarpInstance {
            warp_id: child_warp,
        });

        let stats = delta.stats();
        assert_eq!(stats.upsert_node, 2);
        assert_eq!(stats.delete_node, 1);
        assert_eq!(stats.upsert_edge, 0);
        assert_eq!(stats.delete_edge, 0);
        assert_eq!(stats.set_attachment, 1);
        assert_eq!(stats.open_portal, 1);
        assert_eq!(stats.upsert_warp_instance, 1);
        assert_eq!(stats.delete_warp_instance, 1);
    }

    #[test]
    fn new_creates_empty_delta() {
        let delta = TickDelta::new();
        assert!(delta.is_empty());
        assert_eq!(delta.len(), 0);
    }

    #[test]
    fn push_with_origin_increments_len() {
        let warp_id = make_warp_id("origin-warp");
        let node_id = make_node_id("origin-node");

        let mut delta = TickDelta::new();
        delta.push_with_origin(
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_id,
                },
                record: NodeRecord {
                    ty: make_type_id("TestNode"),
                },
            },
            OpOrigin {
                intent_id: 42,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );

        assert_eq!(delta.len(), 1);
        assert!(!delta.is_empty());
    }

    #[test]
    fn into_ops_unsorted_preserves_insertion_order() {
        let warp_id = make_warp_id("unsorted-warp");
        let node_a = make_node_id("node-a");
        let node_b = make_node_id("node-b");

        let mut delta = TickDelta::new();

        // Push SetAttachment first (would sort last in canonical order)
        delta.push(WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(NodeKey {
                warp_id,
                local_id: node_a,
            }),
            value: None,
        });

        // Push UpsertNode second (would sort before SetAttachment)
        delta.push(WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_b,
            },
            record: NodeRecord {
                ty: make_type_id("TestNode"),
            },
        });

        let ops = delta.into_ops_unsorted();
        assert_eq!(ops.len(), 2);

        // Should preserve insertion order, not canonical order
        assert!(matches!(ops[0], WarpOp::SetAttachment { .. }));
        assert!(matches!(ops[1], WarpOp::UpsertNode { .. }));
    }

    #[test]
    fn emit_records_default_origin() {
        let warp_id = make_warp_id("emit-warp");
        let node_id = make_node_id("emit-node");

        let mut delta = TickDelta::new();
        delta.emit(WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_id,
            },
            record: NodeRecord {
                ty: make_type_id("TestNode"),
            },
        });

        assert_eq!(delta.len(), 1);
        assert_eq!(delta.origins().len(), 1);
        assert_eq!(delta.origins()[0], OpOrigin::default());
    }

    #[test]
    fn emit_with_origin_records_explicit_origin() {
        let warp_id = make_warp_id("emit-origin-warp");
        let node_id = make_node_id("emit-origin-node");
        let origin = OpOrigin {
            intent_id: 123,
            rule_id: 456,
            match_ix: 789,
            op_ix: 42,
        };

        let mut delta = TickDelta::new();
        delta.emit_with_origin(
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_id,
                },
                record: NodeRecord {
                    ty: make_type_id("TestNode"),
                },
            },
            origin,
        );

        assert_eq!(delta.len(), 1);
        assert_eq!(delta.origins().len(), 1);
        assert_eq!(delta.origins()[0], origin);
    }

    #[test]
    fn scoped_delta_applies_origin_to_all_ops() {
        let warp_id = make_warp_id("scoped-warp");
        let node_a = make_node_id("scoped-node-a");
        let node_b = make_node_id("scoped-node-b");
        let origin = OpOrigin {
            intent_id: 100,
            rule_id: 200,
            match_ix: 300,
            op_ix: 0, // Will be overwritten by ScopedDelta
        };

        let mut delta = TickDelta::new();
        {
            let mut scoped = ScopedDelta::new(&mut delta, origin);
            scoped.emit(WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_a,
                },
                record: NodeRecord {
                    ty: make_type_id("TestNode"),
                },
            });
            scoped.emit(WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_b,
                },
                record: NodeRecord {
                    ty: make_type_id("TestNode"),
                },
            });
        }

        assert_eq!(delta.len(), 2);
        assert_eq!(delta.origins().len(), 2);
        // ScopedDelta auto-assigns op_ix: 0 for first, 1 for second
        assert_eq!(
            delta.origins()[0],
            OpOrigin {
                intent_id: 100,
                rule_id: 200,
                match_ix: 300,
                op_ix: 0,
            }
        );
        assert_eq!(
            delta.origins()[1],
            OpOrigin {
                intent_id: 100,
                rule_id: 200,
                match_ix: 300,
                op_ix: 1,
            }
        );
    }

    #[test]
    fn scoped_delta_inner_accessors() {
        let mut delta = TickDelta::new();
        let origin = OpOrigin::default();
        let mut scoped = ScopedDelta::new(&mut delta, origin);

        // Test inner() returns reference
        assert!(scoped.inner().is_empty());

        // Test inner_mut() allows modification
        let warp_id = make_warp_id("inner-warp");
        let node_id = make_node_id("inner-node");
        scoped.inner_mut().emit(WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_id,
            },
            record: NodeRecord {
                ty: make_type_id("TestNode"),
            },
        });

        assert_eq!(scoped.inner().len(), 1);
    }

    #[test]
    fn format_op_compact_produces_readable_output() {
        let warp_id = make_warp_id("test-warp");
        let node_id = make_node_id("test-node");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };

        let op = WarpOp::UpsertNode {
            node: node_key,
            record: NodeRecord {
                ty: make_type_id("TestType"),
            },
        };

        let formatted = format_op_compact(&op);
        assert!(formatted.starts_with("UpsertNode("));
        assert!(formatted.contains("warp:"));
        assert!(formatted.contains("node:"));
        assert!(formatted.contains("ty:"));
    }

    #[test]
    fn format_op_compact_handles_all_op_types() {
        use crate::attachment::AtomPayload;
        use crate::ident::{make_edge_id, EdgeKey};
        use crate::record::EdgeRecord;

        let warp_id = make_warp_id("test-warp");
        let node_id = make_node_id("test-node");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };
        let edge_id = make_edge_id("test-edge");
        let child_warp = make_warp_id("child-warp");
        let child_root = make_node_id("child-root");

        // Test all op types produce non-empty strings
        let ops = vec![
            WarpOp::OpenPortal {
                key: AttachmentKey::node_alpha(node_key),
                child_warp,
                child_root,
                init: PortalInit::Empty {
                    root_record: NodeRecord {
                        ty: make_type_id("ChildRoot"),
                    },
                },
            },
            WarpOp::UpsertWarpInstance {
                instance: WarpInstance {
                    warp_id,
                    root_node: node_id,
                    parent: None,
                },
            },
            WarpOp::DeleteWarpInstance { warp_id },
            WarpOp::UpsertNode {
                node: node_key,
                record: NodeRecord {
                    ty: make_type_id("TestType"),
                },
            },
            WarpOp::DeleteNode { node: node_key },
            WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: crate::ident::EdgeId(edge_id.0),
                    from: node_id,
                    to: node_id,
                    ty: make_type_id("EdgeType"),
                },
            },
            WarpOp::DeleteEdge {
                warp_id,
                from: node_id,
                edge_id: crate::ident::EdgeId(edge_id.0),
            },
            WarpOp::SetAttachment {
                key: AttachmentKey::node_alpha(node_key),
                value: None,
            },
            WarpOp::SetAttachment {
                key: AttachmentKey::edge_beta(EdgeKey {
                    warp_id,
                    local_id: crate::ident::EdgeId(edge_id.0),
                }),
                value: Some(AttachmentValue::Atom(AtomPayload {
                    type_id: make_type_id("AtomType"),
                    bytes: vec![1, 2, 3].into(),
                })),
            },
            WarpOp::SetAttachment {
                key: AttachmentKey::node_alpha(node_key),
                value: Some(AttachmentValue::Descend(child_warp)),
            },
        ];

        for op in ops {
            let formatted = format_op_compact(&op);
            assert!(
                !formatted.is_empty(),
                "format_op_compact returned empty string"
            );
            // Verify the format contains parentheses (properly structured)
            assert!(formatted.contains('('), "format missing opening paren");
            assert!(formatted.contains(')'), "format missing closing paren");
        }
    }

    #[test]
    fn validate_delta_matches_diff_succeeds_for_identical_ops() {
        let warp_id = make_warp_id("test-warp");
        let node_id = make_node_id("test-node");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };

        let ops = vec![
            WarpOp::UpsertNode {
                node: node_key,
                record: NodeRecord {
                    ty: make_type_id("TestType"),
                },
            },
            WarpOp::SetAttachment {
                key: AttachmentKey::node_alpha(node_key),
                value: None,
            },
        ];

        // Same ops should match
        let result = validate_delta_matches_diff(&ops, &ops);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_delta_matches_diff_succeeds_regardless_of_order() {
        let warp_id = make_warp_id("test-warp");
        let node_a = make_node_id("node-a");
        let node_b = make_node_id("node-b");

        let ops_order_1 = vec![
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_a,
                },
                record: NodeRecord {
                    ty: make_type_id("TypeA"),
                },
            },
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_b,
                },
                record: NodeRecord {
                    ty: make_type_id("TypeB"),
                },
            },
        ];

        let ops_order_2 = vec![
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_b,
                },
                record: NodeRecord {
                    ty: make_type_id("TypeB"),
                },
            },
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_a,
                },
                record: NodeRecord {
                    ty: make_type_id("TypeA"),
                },
            },
        ];

        // Different order but same ops should match after canonicalization
        let result = validate_delta_matches_diff(&ops_order_1, &ops_order_2);
        assert!(result.is_ok());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_delta_matches_diff_detects_length_mismatch() {
        let warp_id = make_warp_id("test-warp");
        let node_a = make_node_id("node-a");
        let node_b = make_node_id("node-b");

        let ops_one = vec![WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_a,
            },
            record: NodeRecord {
                ty: make_type_id("TypeA"),
            },
        }];

        let ops_two = vec![
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_a,
                },
                record: NodeRecord {
                    ty: make_type_id("TypeA"),
                },
            },
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: node_b,
                },
                record: NodeRecord {
                    ty: make_type_id("TypeB"),
                },
            },
        ];

        let result = validate_delta_matches_diff(&ops_one, &ops_two);
        assert!(result.is_err());
        let mismatch = result.unwrap_err();
        assert_eq!(mismatch.delta_len, 1);
        assert_eq!(mismatch.diff_len, 2);
        assert!(mismatch.first_mismatch_index.is_none());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_delta_matches_diff_detects_content_mismatch() {
        let warp_id = make_warp_id("test-warp");
        let node_a = make_node_id("node-a");
        let node_b = make_node_id("node-b");

        let ops_delta = vec![WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_a,
            },
            record: NodeRecord {
                ty: make_type_id("TypeA"),
            },
        }];

        let ops_diff = vec![WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_b, // Different node
            },
            record: NodeRecord {
                ty: make_type_id("TypeB"),
            },
        }];

        let result = validate_delta_matches_diff(&ops_delta, &ops_diff);
        assert!(result.is_err());
        let mismatch = result.unwrap_err();
        assert_eq!(mismatch.delta_len, 1);
        assert_eq!(mismatch.diff_len, 1);
        assert_eq!(mismatch.first_mismatch_index, Some(0));
    }

    #[test]
    fn delta_mismatch_display_format() {
        let mismatch = DeltaMismatch {
            delta_len: 5,
            diff_len: 3,
            first_mismatch_index: Some(2),
            delta_stats: DeltaStats {
                upsert_node: 2,
                delete_node: 1,
                ..Default::default()
            },
            diff_stats: DeltaStats {
                upsert_node: 1,
                ..Default::default()
            },
        };

        let display = format!("{mismatch}");
        assert!(display.contains("delta_len: 5"));
        assert!(display.contains("diff_len:  3"));
        assert!(display.contains("first_mismatch_index: 2"));
    }

    #[test]
    fn assert_delta_matches_diff_does_not_panic_on_match() {
        let warp_id = make_warp_id("test-warp");
        let node_id = make_node_id("test-node");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };

        let ops = vec![WarpOp::UpsertNode {
            node: node_key,
            record: NodeRecord {
                ty: make_type_id("TestType"),
            },
        }];

        // Should not panic
        assert_delta_matches_diff(&ops, &ops);
    }

    #[test]
    #[should_panic(expected = "DELTA MISMATCH")]
    fn assert_delta_matches_diff_panics_on_mismatch() {
        let warp_id = make_warp_id("test-warp");
        let node_a = make_node_id("node-a");
        let node_b = make_node_id("node-b");

        let ops_delta = vec![WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_a,
            },
            record: NodeRecord {
                ty: make_type_id("TypeA"),
            },
        }];

        let ops_diff = vec![WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: node_b,
            },
            record: NodeRecord {
                ty: make_type_id("TypeB"),
            },
        }];

        // Should panic with detailed error message
        assert_delta_matches_diff(&ops_delta, &ops_diff);
    }
}
