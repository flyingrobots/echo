// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Worst-case static cost of one bounded read, as the runtime meter charges it.
//!
//! A node-atom read copies at most its byte bound. A typed edge read pays its
//! declared scan allowance and its result. A guided closure pays, on top of
//! its scans, every node it may materialize, every occurrence it may record,
//! and for every edge row it may hold: that row, the candidate the selector is
//! offered (built from copies, then copied again as the call argument and
//! validated), the selector itself and the guide it returns. Each copy costs
//! what `shape` says a maximal value of that type costs.
use super::{shape, Check, Cost, Fact, Ty};
use std::collections::BTreeMap;

/// Bytes the runtime reserves for each edge in a scanned bucket.
const SCANNED_EDGE_BYTES: u64 = 128;
/// The four value cells of the closure result record.
const RESULT_CELLS_BYTES: u64 = 4 * 64;

pub(super) fn read_cost(
    declared: &Ty,
    completed: &BTreeMap<String, Fact>,
    edge_scans: u64,
) -> Check<Cost> {
    if let Ty::Bytes(_, max) = declared.representation() {
        return Ok(Cost {
            steps: max.checked_add(1).ok_or(())?,
            bytes: max.checked_add(64).ok_or(())?,
        });
    }
    let (copy, validations, _, _) = shape(declared)?;
    let scans = Cost {
        steps: edge_scans,
        bytes: edge_scans.checked_mul(SCANNED_EDGE_BYTES).ok_or(())?,
    };
    let validation = Cost {
        steps: validations,
        bytes: 0,
    };
    let Some(parts) = closure_parts(declared)? else {
        // A typed edge read charges its scans, then its whole result.
        return scans.plus(copy)?.plus(validation);
    };
    closure_cost(&parts, completed)?
        .plus(scans)?
        .plus(validation)
}

/// The capacities and row types of a closure result.
struct Parts<'a> {
    node: &'a Ty,
    nodes: u64,
    atom: u64,
    edge: &'a Ty,
    edges: u64,
    occurrence: &'a Ty,
    occurrences: u64,
    guide: &'a Ty,
}

fn times(cost: Cost, count: u64) -> Check<Cost> {
    Ok(Cost {
        steps: cost.steps.checked_mul(count).ok_or(())?,
        bytes: cost.bytes.checked_mul(count).ok_or(())?,
    })
}

fn copy(ty: &Ty) -> Check<Cost> {
    Ok(shape(ty)?.0)
}

fn closure_cost(parts: &Parts<'_>, completed: &BTreeMap<String, Fact>) -> Check<Cost> {
    let guide = copy(parts.guide)?;
    let node = copy(parts.node)?;
    let candidate_ty = Ty::Record(BTreeMap::from([
        ("guide".to_owned(), parts.guide.clone()),
        ("depth".to_owned(), Ty::Word(u64::MAX)),
        ("slot".to_owned(), Ty::Word(u64::MAX)),
        ("parent".to_owned(), parts.node.clone()),
        ("child".to_owned(), parts.node.clone()),
    ]));
    let (candidate, candidate_validations, _, _) = shape(&candidate_ty)?;
    // Any source helper may be the declared selector; charge the costliest.
    let selector = completed
        .values()
        .fold(Cost::default(), |worst, fact| worst.maximum(fact.cost));
    // The start guide is validated, then copied into the root occurrence.
    let start = Cost {
        steps: shape(parts.guide)?.1,
        bytes: 0,
    }
    .plus(guide)?;
    // Atom bytes are charged before they are copied, then again in the row.
    let per_node = Cost {
        steps: parts.atom,
        bytes: parts.atom.checked_add(64).ok_or(())?,
    }
    .plus(node)?;
    let per_occurrence = Cost { steps: 1, bytes: 0 }
        .plus(guide)?
        .plus(copy(parts.occurrence)?)?;
    let per_edge = copy(parts.edge)?
        .plus(guide)?
        .plus(node)?
        .plus(node)?
        .plus(candidate)?
        .plus(Cost {
            steps: candidate_validations.checked_add(2).ok_or(())?,
            bytes: 0,
        })?
        .plus(selector)?
        .plus(guide)?;
    start
        .plus(times(per_node, parts.nodes)?)?
        .plus(times(per_occurrence, parts.occurrences)?)?
        .plus(times(per_edge, parts.edges)?)?
        .plus(Cost {
            steps: 0,
            bytes: RESULT_CELLS_BYTES,
        })
}

fn closure_parts(declared: &Ty) -> Check<Option<Parts<'_>>> {
    let Ty::Record(fields) = declared.representation() else {
        return Ok(None);
    };
    let list = |name: &str| match fields.get(name).map(Ty::representation) {
        Some(Ty::List(Some(item), max)) => Some((item.as_ref(), *max)),
        _ => None,
    };
    let (Some((node, nodes)), Some((edge, edges)), Some((occurrence, occurrences))) =
        (list("nodes"), list("edges"), list("occurrences"))
    else {
        return Ok(None);
    };
    let (Ty::Record(row), Ty::Record(position)) =
        (node.representation(), occurrence.representation())
    else {
        return Err(());
    };
    let Some(Ty::Bytes(_, atom)) = row.get("atomBytes").map(Ty::representation) else {
        return Err(());
    };
    Ok(Some(Parts {
        node,
        nodes,
        atom: *atom,
        edge,
        edges,
        occurrence,
        occurrences,
        guide: position.get("guide").ok_or(())?,
    }))
}

#[cfg(test)]
mod tests;
