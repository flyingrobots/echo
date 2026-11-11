// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Worst-case static cost of one bounded read's delivered result.
//!
//! A node-atom read copies at most its byte bound. A graph read materializes
//! at most its declared result. A closure read also offers every edge row it
//! may hold to its selector once, each time with a fresh candidate record.
use super::{shape, Check, Cost, Fact, Ty};
use std::collections::BTreeMap;

pub(super) fn read_cost(declared: &Ty, completed: &BTreeMap<String, Fact>) -> Check<Cost> {
    if let Ty::Bytes(_, max) = declared.representation() {
        return Ok(Cost {
            steps: max.checked_add(1).ok_or(())?,
            bytes: max.checked_add(64).ok_or(())?,
        });
    }
    let (copy, validations, _, _) = shape(declared)?;
    let result = copy.plus(Cost {
        steps: validations,
        bytes: 0,
    })?;
    let Some((node, edges, guide)) = closure_parts(declared)? else {
        return Ok(result);
    };
    // Any source helper may be the declared selector; charge the costliest.
    let selector = completed
        .values()
        .fold(Cost::default(), |worst, fact| worst.maximum(fact.cost));
    let word = shape(&Ty::Word(u64::MAX))?.0;
    let node = shape(node)?.0;
    let candidate = node
        .plus(node)?
        .plus(shape(guide)?.0)?
        .plus(word)?
        .plus(word)?;
    let offer = selector.plus(candidate)?;
    result.plus(Cost {
        steps: offer.steps.checked_mul(edges).ok_or(())?,
        bytes: offer.bytes.checked_mul(edges).ok_or(())?,
    })
}

/// The node row, edge-row capacity and guide of a closure result, if
/// `declared` is one.
fn closure_parts(declared: &Ty) -> Check<Option<(&Ty, u64, &Ty)>> {
    let Ty::Record(fields) = declared.representation() else {
        return Ok(None);
    };
    let list = |name: &str| match fields.get(name).map(Ty::representation) {
        Some(Ty::List(Some(item), max)) => Some((item.as_ref(), *max)),
        _ => None,
    };
    let (Some((node, _)), Some((_, edges)), Some((occurrence, _))) =
        (list("nodes"), list("edges"), list("occurrences"))
    else {
        return Ok(None);
    };
    let Ty::Record(occurrence) = occurrence.representation() else {
        return Err(());
    };
    Ok(Some((node, edges, occurrence.get("guide").ok_or(())?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(fields: &[(&str, Ty)]) -> Ty {
        Ty::Record(
            fields
                .iter()
                .map(|(name, ty)| ((*name).to_owned(), ty.clone()))
                .collect(),
        )
    }
    fn list(item: Ty, max: u64) -> Ty {
        Ty::List(Some(Box::new(item)), max)
    }
    fn word() -> Ty {
        Ty::Word(u64::MAX)
    }
    fn closure(edges: u64) -> (Ty, Ty, Ty) {
        let node = record(&[
            ("nodeId", Ty::Bytes(32, 32)),
            ("atomBytes", Ty::Bytes(0, 8)),
        ]);
        let guide = record(&[("reach", word())]);
        let result = record(&[
            ("nodes", list(node.clone(), 2)),
            ("edges", list(record(&[("from", word())]), edges)),
            (
                "occurrences",
                list(record(&[("guide", guide.clone()), ("node", word())]), 3),
            ),
        ]);
        (result, node, guide)
    }
    fn helper(steps: u64, bytes: u64) -> BTreeMap<String, Fact> {
        BTreeMap::from([(
            "app@1.select".to_owned(),
            Fact {
                ty: Ty::Bool,
                cost: Cost { steps, bytes },
                depth: 1,
            },
        )])
    }

    #[test]
    fn a_node_atom_read_still_costs_its_byte_bound() -> Check<()> {
        let cost = read_cost(&Ty::Bytes(0, 10), &helper(1000, 1000))?;
        assert_eq!((cost.steps, cost.bytes), (11, 74));
        Ok(())
    }

    #[test]
    fn an_edge_read_costs_its_declared_result_only() -> Check<()> {
        let edges = list(record(&[("edgeId", Ty::Bytes(32, 32))]), 4);
        let (copy, validations, _, _) = shape(&edges)?;
        let cost = read_cost(&edges, &helper(1000, 1000))?;
        assert_eq!(
            (cost.steps, cost.bytes),
            (copy.steps + validations, copy.bytes)
        );
        Ok(())
    }

    /// Oracle: result shape, plus one selector evaluation and one fresh
    /// candidate (two node rows, the guide, depth and slot) per edge row.
    #[test]
    fn a_closure_read_charges_its_selector_once_per_possible_edge_row() -> Check<()> {
        for edges in [4, 5] {
            let (result, node, guide) = closure(edges);
            let (copy, validations, _, _) = shape(&result)?;
            let node = shape(&node)?.0;
            let guide = shape(&guide)?.0;
            let word = shape(&word())?.0;
            let candidate_steps = 2 * node.steps + guide.steps + 2 * word.steps;
            let candidate_bytes = 2 * node.bytes + guide.bytes + 2 * word.bytes;
            let cost = read_cost(&result, &helper(7, 11))?;
            assert_eq!(
                cost.steps,
                copy.steps + validations + edges * (7 + candidate_steps),
                "{edges}"
            );
            assert_eq!(
                cost.bytes,
                copy.bytes + edges * (11 + candidate_bytes),
                "{edges}"
            );
        }
        Ok(())
    }

    #[test]
    fn an_overflowing_closure_charge_is_refused() {
        let (result, _, _) = closure(u64::MAX);
        assert!(read_cost(&result, &helper(u64::MAX / 2, 1)).is_err());
    }
}
