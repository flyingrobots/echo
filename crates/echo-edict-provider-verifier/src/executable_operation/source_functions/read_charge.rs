// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned worst-case charge for one bounded read, as the runtime pays.
//!
//! Node-atom bytes cost their bound. A typed edge read pays its declared scan
//! allowance, then its layout. A guided closure pays its scans and, for each
//! node it may resolve, the atom twice (precharged, then in the row); for each
//! occurrence, one step, the guide and the row; and for each edge row it may
//! hold, the row, the candidate's parts, the candidate again as the selector's
//! argument with its validation, the selector and the guide it returns.
use super::types::{Charge, Schema};
use super::{Check, Summary};
use std::collections::BTreeMap;

/// Bytes the runtime reserves per edge of a scanned bucket.
const EDGE_SCAN_STORAGE: u128 = 128;
/// The closure result's own four value cells.
const RESULT_CELL_STORAGE: u128 = 256;

pub(super) fn read_charge(
    delivered: &Schema,
    summaries: &BTreeMap<String, Summary>,
    edge_scans: u64,
) -> Check<Charge> {
    if let Schema::Blob { high, .. } = delivered.representation() {
        return Ok(Charge {
            ticks: u128::from(*high) + 1,
            storage: u128::from(*high) + 64,
        });
    }
    let layout = delivered.measure()?;
    let mut total = Charge {
        ticks: u128::from(edge_scans) + layout.checks,
        storage: u128::from(edge_scans) * EDGE_SCAN_STORAGE,
    };
    match Walk::of(delivered)? {
        None => total.append(layout.copy)?,
        Some(walk) => total.append(walk.charge(summaries)?)?,
    }
    Ok(total)
}

/// Row schemas and capacities of a closure result.
struct Walk<'s> {
    node: &'s Schema,
    node_bound: u64,
    atom_high: u64,
    edge: &'s Schema,
    edge_bound: u64,
    occurrence: &'s Schema,
    occurrence_bound: u64,
    guide: &'s Schema,
}

fn scaled(charge: Charge, count: u64) -> Check<Charge> {
    Ok(Charge {
        ticks: charge.ticks.checked_mul(u128::from(count)).ok_or(())?,
        storage: charge.storage.checked_mul(u128::from(count)).ok_or(())?,
    })
}

impl<'s> Walk<'s> {
    fn of(delivered: &'s Schema) -> Check<Option<Self>> {
        let Schema::Fields(fields) = delivered.representation() else {
            return Ok(None);
        };
        let rows = |name: &str| match fields.get(name).map(Schema::representation) {
            Some(Schema::Sequence {
                element: Some(element),
                bound,
            }) => Some((element.as_ref(), *bound)),
            _ => None,
        };
        let (
            Some((node, node_bound)),
            Some((edge, edge_bound)),
            Some((occurrence, occurrence_bound)),
        ) = (rows("nodes"), rows("edges"), rows("occurrences"))
        else {
            return Ok(None);
        };
        let (Schema::Fields(node_fields), Schema::Fields(position)) =
            (node.representation(), occurrence.representation())
        else {
            return Err(());
        };
        let Some(Schema::Blob {
            high: atom_high, ..
        }) = node_fields.get("atomBytes").map(Schema::representation)
        else {
            return Err(());
        };
        Ok(Some(Self {
            node,
            node_bound,
            atom_high: *atom_high,
            edge,
            edge_bound,
            occurrence,
            occurrence_bound,
            guide: position.get("guide").ok_or(())?,
        }))
    }

    fn charge(&self, summaries: &BTreeMap<String, Summary>) -> Check<Charge> {
        let guide = self.guide.measure()?;
        let node = self.node.measure()?.copy;
        let argument = Schema::Fields(BTreeMap::from([
            ("guide".to_owned(), self.guide.clone()),
            ("depth".to_owned(), Schema::Word(u64::MAX)),
            ("slot".to_owned(), Schema::Word(u64::MAX)),
            ("parent".to_owned(), self.node.clone()),
            ("child".to_owned(), self.node.clone()),
        ]))
        .measure()?;
        // The declared selector is one of these helpers; the dearest bounds it.
        let selector = summaries
            .values()
            .fold(Charge::default(), |dearest, summary| {
                Charge::choice(dearest, summary.charge)
            });
        let atom = u128::from(self.atom_high);
        let mut each_node = Charge {
            ticks: atom,
            storage: atom + 64,
        };
        each_node.append(node)?;
        let mut each_occurrence = Charge {
            ticks: 1,
            storage: 0,
        };
        each_occurrence.append(guide.copy)?;
        each_occurrence.append(self.occurrence.measure()?.copy)?;
        let mut each_edge = self.edge.measure()?.copy;
        for part in [guide.copy, node, node, argument.copy, selector, guide.copy] {
            each_edge.append(part)?;
        }
        each_edge.append(Charge {
            ticks: argument.checks + 2,
            storage: 0,
        })?;
        let mut total = Charge {
            ticks: guide.checks,
            storage: RESULT_CELL_STORAGE,
        };
        total.append(guide.copy)?;
        total.append(scaled(each_node, self.node_bound)?)?;
        total.append(scaled(each_occurrence, self.occurrence_bound)?)?;
        total.append(scaled(each_edge, self.edge_bound)?)?;
        Ok(total)
    }
}

#[cfg(test)]
mod tests;
