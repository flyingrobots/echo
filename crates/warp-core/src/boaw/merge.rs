// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical delta merge for BOAW Phase 6A.

use crate::tick_delta::OpOrigin;
#[cfg(any(test, feature = "delta_validate"))]
use crate::tick_delta::TickDelta;
#[cfg(any(test, feature = "delta_validate"))]
use crate::tick_patch::WarpOp;

/// Conflict detected during merge (indicates missing footprint target).
#[derive(Debug)]
pub struct MergeConflict {
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
/// 3. Dedupe identical ops, explode on divergent ops
///
/// # Errors
///
/// Returns [`MergeConflict`] if multiple writers produced different values
/// for the same logical key, indicating a footprint model violation.
#[cfg(any(test, feature = "delta_validate"))]
pub fn merge_deltas(deltas: Vec<TickDelta>) -> Result<Vec<WarpOp>, MergeConflict> {
    let mut flat: Vec<(crate::tick_patch::WarpOpKey, OpOrigin, WarpOp)> = Vec::new();

    for d in deltas {
        let (ops, origins) = d.into_parts_unsorted();
        debug_assert_eq!(ops.len(), origins.len(), "ops/origins length mismatch");
        for (op, origin) in ops.into_iter().zip(origins) {
            flat.push((op.sort_key(), origin, op));
        }
    }

    // Sort by (WarpOpKey, OpOrigin) - both are Ord
    flat.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

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
            return Err(MergeConflict { writers });
        }
    }

    Ok(out)
}
