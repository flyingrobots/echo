// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical delta merge for parallel execution.

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
    let mut flat: Vec<(WarpOpKey, OpOrigin, WarpOp)> = Vec::new();

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

    merge_canonical(flat).map_err(|fault| match fault {
        MergeFault::Conflict { key, writers } => {
            MergeError::Conflict(Box::new(MergeConflict { key, writers }))
        }
        MergeFault::WriteToNewWarp {
            warp_id,
            origin,
            op_kind,
        } => MergeError::WriteToNewWarp {
            warp_id,
            op_origin: origin,
            op_kind,
        },
    })
}

/// Why [`merge_canonical`] refused a set of operations.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MergeFault<O> {
    /// Operations that share a key differ.
    Conflict {
        /// The shared key.
        key: WarpOpKey,
        /// Every writer of that key, in canonical order.
        writers: Vec<O>,
    },
    /// An operation targets a warp that an `OpenPortal` creates in the same tick.
    WriteToNewWarp {
        /// The newly created warp.
        warp_id: WarpId,
        /// The writer of the violating operation.
        origin: O,
        /// The kind of the violating operation.
        op_kind: &'static str,
    },
}

/// The canonical merge, shared by the engine and [`merge_deltas`].
///
/// `O` identifies an operation's writer. Builds that record origins
/// (`test` or `delta_validate`) pass [`OpOrigin`]; other builds pass `()`.
/// The origin orders operations that share a key and names the writers in a
/// refusal; it never changes which operations are accepted or their output
/// order, because only identical operations survive under one key.
///
/// 1. Sort by (key, origin).
/// 2. Refuse any write to a warp created in the same tick by
///    `OpenPortal { init: Empty { .. }, .. }`.
/// 3. Keep one operation per key when all are identical; refuse otherwise.
pub(crate) fn merge_canonical<O: Ord + Copy>(
    mut flat: Vec<(WarpOpKey, O, WarpOp)>,
) -> Result<Vec<WarpOp>, MergeFault<O>> {
    flat.sort_unstable_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    let new_warps = collect_new_warps(flat.iter().map(|(_, _, op)| op));
    for (_, origin, op) in &flat {
        if let Some((warp_id, op_kind)) = extract_target_warp(op) {
            if new_warps.contains(&warp_id) {
                return Err(MergeFault::WriteToNewWarp {
                    warp_id,
                    origin: *origin,
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
        while i < flat.len() && flat[i].0 == key {
            i += 1;
        }
        let first = &flat[start].2;
        if flat[start + 1..i].iter().all(|(_, _, op)| op == first) {
            out.push(first.clone());
        } else {
            return Err(MergeFault::Conflict {
                key,
                writers: flat[start..i].iter().map(|(_, o, _)| *o).collect(),
            });
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::{merge_canonical, MergeFault};
    use crate::attachment::{AtomPayload, AttachmentKey, AttachmentValue};
    use crate::ident::{make_node_id, make_type_id, make_warp_id, NodeKey};
    use crate::record::NodeRecord;
    use crate::tick_delta::OpOrigin;
    use crate::tick_patch::{PortalInit, WarpOp};

    fn origin(intent_id: u64) -> OpOrigin {
        OpOrigin {
            intent_id,
            rule_id: 0,
            match_ix: 0,
            op_ix: 0,
        }
    }

    fn set(node: &str, value: &[u8]) -> WarpOp {
        WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(NodeKey {
                warp_id: make_warp_id("merge/w"),
                local_id: make_node_id(node),
            }),
            value: Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id("merge/atom"),
                value.to_vec().into(),
            ))),
        }
    }

    fn open_child() -> WarpOp {
        WarpOp::OpenPortal {
            key: AttachmentKey::node_alpha(NodeKey {
                warp_id: make_warp_id("merge/w"),
                local_id: make_node_id("merge/parent"),
            }),
            child_warp: make_warp_id("merge/child"),
            child_root: make_node_id("merge/child-root"),
            init: PortalInit::Empty {
                root_record: NodeRecord {
                    ty: make_type_id("merge/root"),
                },
            },
        }
    }

    fn upsert_in_child() -> WarpOp {
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: make_warp_id("merge/child"),
                local_id: make_node_id("merge/new"),
            },
            record: NodeRecord {
                ty: make_type_id("merge/node"),
            },
        }
    }

    /// Runs the merge as the shipped build does (no origins) and as a
    /// validation build does (with origins), and checks they agree.
    fn both(ops: &[WarpOp]) -> Result<Vec<WarpOp>, MergeFault<()>> {
        let shipped = merge_canonical(
            ops.iter()
                .map(|op| (op.sort_key(), (), op.clone()))
                .collect(),
        );
        let validated = merge_canonical(
            ops.iter()
                .enumerate()
                .map(|(i, op)| (op.sort_key(), origin(i as u64), op.clone()))
                .collect(),
        );
        match (&shipped, validated) {
            (Ok(a), Ok(b)) => assert_eq!(a, &b, "shipped and validated outputs differ"),
            (
                Err(MergeFault::Conflict { key: a, .. }),
                Err(MergeFault::Conflict { key: b, .. }),
            ) => {
                assert_eq!(a, &b);
            }
            (
                Err(MergeFault::WriteToNewWarp {
                    warp_id: a,
                    op_kind: ka,
                    ..
                }),
                Err(MergeFault::WriteToNewWarp {
                    warp_id: b,
                    op_kind: kb,
                    ..
                }),
            ) => assert_eq!((a, ka), (&b, &kb)),
            (a, b) => panic!("shipped {a:?} and validated {b:?} disagree"),
        }
        shipped
    }

    #[test]
    fn identical_writes_collapse_in_canonical_order() {
        let ops = [
            set("b", b"2"),
            set("a", b"1"),
            set("b", b"2"),
            set("a", b"1"),
        ];
        let merged = both(&ops).expect("identical writes merge");
        let mut expected = vec![set("a", b"1"), set("b", b"2")];
        expected.sort_by_key(WarpOp::sort_key);
        assert_eq!(merged, expected);
    }

    #[test]
    fn divergent_writes_to_one_key_are_refused() {
        let ops = [set("a", b"1"), set("c", b"3"), set("a", b"other")];
        assert!(matches!(both(&ops), Err(MergeFault::Conflict { .. })));
    }

    #[test]
    fn a_write_to_a_warp_opened_in_the_same_tick_is_refused() {
        let ops = [upsert_in_child(), open_child()];
        assert!(matches!(
            both(&ops),
            Err(MergeFault::WriteToNewWarp {
                op_kind: "UpsertNode",
                ..
            })
        ));
    }

    #[test]
    fn opening_a_warp_alone_merges() {
        assert_eq!(both(&[open_child()]).expect("merge"), vec![open_child()]);
    }
}
