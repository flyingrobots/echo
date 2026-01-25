// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical delta merge for BOAW Phase 6A.

use std::collections::BTreeSet;

#[cfg(any(test, feature = "delta_validate"))]
use super::exec::PoisonedDelta;
use crate::tick_delta::OpOrigin;
#[cfg(any(test, feature = "delta_validate"))]
use crate::tick_delta::TickDelta;
use crate::tick_patch::{PortalInit, WarpOp, WarpOpKey};
use crate::WarpId;

/// Errors produced during delta merge.
#[derive(Debug)]
#[cfg(any(test, feature = "delta_validate"))]
pub enum MergeError {
    /// Conflict detected (indicates missing footprint target).
    Conflict(Box<MergeConflict>),
    /// Executor produced a poisoned delta (panic or enforcement violation).
    PoisonedDelta(PoisonedDelta),
    /// Attempted to write to a newly created warp in the same tick.
    WriteToNewWarp {
        /// The newly created warp that was written to.
        warp_id: WarpId,
        /// The origin of the violating operation.
        op_origin: OpOrigin,
        /// Human-readable description of the operation kind.
        op_kind: &'static str,
    },
}

/// Conflict detected during merge (indicates missing footprint target).
#[derive(Debug)]
pub struct MergeConflict {
    /// The conflicting key (required by ADR-0007).
    pub key: WarpOpKey,
    /// The writers that produced conflicting ops.
    pub writers: Vec<OpOrigin>,
}

/// Merge per-worker deltas into canonical order.
///
/// # Phase 6A Policy
///
/// Conflicts are bugs. If multiple rewrites write different values to the
/// same logical key, the footprint model lied and we explode loudly.
///
/// # Algorithm
///
/// 1. Flatten all ops with origins (unsorted)
/// 2. Sort by ([`WarpOpKey`], [`OpOrigin`]) for canonical order
/// 3. Collect newly created warps from `OpenPortal { init: Empty { .. }, .. }` ops
/// 4. Validate no same-tick writes to newly created warps
/// 5. Dedupe identical ops, explode on divergent ops
///
/// # Errors
///
/// Returns [`MergeError::Conflict`] if multiple writers produced different values
/// for the same logical key, indicating a footprint model violation.
///
/// Returns [`MergeError::WriteToNewWarp`] if any operation targets a warp that
/// is being created in the same tick (via `OpenPortal` with `PortalInit::Empty`).
///
/// Returns [`MergeError::PoisonedDelta`] if any worker produced a poisoned delta
/// (executor panic or enforcement violation).
///
/// # Panics
///
/// Panics if any `TickDelta` has mismatched ops/origins lengths (internal invariant).
#[cfg(any(test, feature = "delta_validate"))]
pub fn merge_deltas(
    deltas: Vec<Result<TickDelta, PoisonedDelta>>,
) -> Result<Vec<WarpOp>, MergeError> {
    let mut flat: Vec<(crate::tick_patch::WarpOpKey, OpOrigin, WarpOp)> = Vec::new();

    for d in deltas {
        let d = match d {
            Ok(delta) => delta,
            Err(poisoned) => return Err(MergeError::PoisonedDelta(poisoned)),
        };
        let (ops, origins) = d.into_parts_unsorted();
        assert_eq!(ops.len(), origins.len(), "ops/origins length mismatch");
        for (op, origin) in ops.into_iter().zip(origins) {
            flat.push((op.sort_key(), origin, op));
        }
    }

    // Sort by (WarpOpKey, OpOrigin) - both are Ord
    flat.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    // Collect newly created warps and validate no same-tick writes to them.
    let new_warps = collect_new_warps(flat.iter().map(|(_, _, op)| op));
    for (_, origin, op) in &flat {
        if let Some((target_warp, op_kind)) = extract_target_warp(op) {
            if new_warps.contains(&target_warp) {
                return Err(MergeError::WriteToNewWarp {
                    warp_id: target_warp,
                    op_origin: *origin,
                    op_kind,
                });
            }
        }
    }

    let mut out = Vec::with_capacity(flat.len());
    let mut i = 0;

    while i < flat.len() {
        let key = flat[i].0;
        let start = i;

        // Find all ops with same WarpOpKey
        while i < flat.len() && flat[i].0 == key {
            i += 1;
        }

        // Check if all ops in group are identical
        let first = &flat[start].2;
        let all_same = flat[start + 1..i].iter().all(|(_, _, op)| op == first);

        if all_same {
            out.push(first.clone());
        } else {
            let writers = flat[start..i].iter().map(|(_, o, _)| *o).collect();
            return Err(MergeError::Conflict(Box::new(MergeConflict {
                key,
                writers,
            })));
        }
    }

    Ok(out)
}

/// Merge deltas when poison is impossible (e.g., deterministic tests).
///
/// # Errors
///
/// Forwards any error reported by [`merge_deltas`], such as merge conflicts or
/// illegal same-tick writes to newly created warps.
#[cfg(any(test, feature = "delta_validate"))]
pub fn merge_deltas_ok(deltas: Vec<TickDelta>) -> Result<Vec<WarpOp>, MergeError> {
    merge_deltas(deltas.into_iter().map(Ok).collect())
}

/// Collects warps being created in this tick via `OpenPortal` with `PortalInit::Empty`.
///
/// These warps must not receive any other writes during the same tick.
pub(crate) fn collect_new_warps<'a>(ops: impl IntoIterator<Item = &'a WarpOp>) -> BTreeSet<WarpId> {
    ops.into_iter()
        .filter_map(|op| match op {
            WarpOp::OpenPortal {
                init: PortalInit::Empty { .. },
                child_warp,
                ..
            } => Some(*child_warp),
            _ => None,
        })
        .collect()
}

/// Finds the first operation that writes to a newly created warp.
///
/// Returns `Some((warp_id, op_kind))` on the first violation found, `None` if valid.
#[cfg(not(any(test, feature = "delta_validate")))]
pub(crate) fn find_write_to_new_warp<'a>(
    ops: impl IntoIterator<Item = &'a WarpOp>,
    new_warps: &BTreeSet<WarpId>,
) -> Option<(WarpId, &'static str)> {
    for op in ops {
        if let Some((target_warp, op_kind)) = extract_target_warp(op) {
            if new_warps.contains(&target_warp) {
                return Some((target_warp, op_kind));
            }
        }
    }
    None
}

/// Extracts the target warp from an operation, if applicable.
///
/// Returns `None` for `OpenPortal` (which creates the warp, not writes to it).
/// Returns `Some((warp_id, op_kind))` for all other ops that target a warp.
pub(crate) fn extract_target_warp(op: &WarpOp) -> Option<(WarpId, &'static str)> {
    use crate::attachment::AttachmentOwner;

    match op {
        // OpenPortal creates the warp - exempt from the check
        WarpOp::OpenPortal { .. } => None,

        WarpOp::UpsertNode { node, .. } => Some((node.warp_id, "UpsertNode")),
        WarpOp::DeleteNode { node } => Some((node.warp_id, "DeleteNode")),
        WarpOp::UpsertEdge { warp_id, .. } => Some((*warp_id, "UpsertEdge")),
        WarpOp::DeleteEdge { warp_id, .. } => Some((*warp_id, "DeleteEdge")),
        WarpOp::SetAttachment { key, .. } => {
            let warp_id = match key.owner {
                AttachmentOwner::Node(node) => node.warp_id,
                AttachmentOwner::Edge(edge) => edge.warp_id,
            };
            Some((warp_id, "SetAttachment"))
        }
        WarpOp::UpsertWarpInstance { instance } => Some((instance.warp_id, "UpsertWarpInstance")),
        WarpOp::DeleteWarpInstance { warp_id } => Some((*warp_id, "DeleteWarpInstance")),
    }
}

/// Validates that no operation writes to a warp created in the same tick.
///
/// This function provides a lightweight, non-`delta_validate` check for the same-tick
/// write invariant that [`merge_deltas`] enforces in test/validation builds. Use this
/// when you have a finalized op slice and need to verify the new-warp write rule without
/// the full merge machinery.
///
/// # Preconditions
///
/// - `ops` should be a complete set of operations for a single tick.
/// - Operations are not required to be sorted; the function scans linearly.
///
/// # Postconditions
///
/// Returns `None` if all operations respect the new-warp write rule (i.e., no op
/// targets a warp that is being created via `OpenPortal` with `PortalInit::Empty`
/// in the same tick).
///
/// Returns `Some((warp_id, op_kind))` on the first violation found, identifying
/// the offending warp and operation type.
///
/// # When to use this vs [`merge_deltas`]
///
/// - Use `check_write_to_new_warp` for fast validation of a finalized op slice in
///   release builds where `delta_validate` is disabled.
/// - Use [`merge_deltas`] when you need full conflict detection, origin tracking,
///   and canonical merge ordering (test/validation builds).
///
/// # Panics
///
/// This function does not panic.
#[cfg(not(any(test, feature = "delta_validate")))]
pub(crate) fn check_write_to_new_warp(ops: &[WarpOp]) -> Option<(WarpId, &'static str)> {
    let new_warps = collect_new_warps(ops);
    find_write_to_new_warp(ops, &new_warps)
}
