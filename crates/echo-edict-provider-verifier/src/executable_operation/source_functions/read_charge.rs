// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned worst-case charge for the value one bounded read delivers.
//!
//! Node-atom bytes cost their bound. A graph result costs its declared
//! layout. A guided closure additionally evaluates its selector once per edge
//! row it can hold, building a candidate from two node rows, the guide, a
//! depth and a slot each time.
use super::types::{Charge, Schema};
use super::{Check, Summary};
use std::collections::BTreeMap;

pub(super) fn read_charge(
    delivered: &Schema,
    summaries: &BTreeMap<String, Summary>,
) -> Check<Charge> {
    if let Schema::Blob { high, .. } = delivered.representation() {
        return Ok(Charge {
            ticks: u128::from(*high) + 1,
            storage: u128::from(*high) + 64,
        });
    }
    let layout = delivered.measure()?;
    let mut total = layout.copy;
    total.append(Charge {
        ticks: layout.checks,
        storage: 0,
    })?;
    let Schema::Fields(fields) = delivered.representation() else {
        return Ok(total);
    };
    let rows = |name: &str| match fields.get(name).map(Schema::representation) {
        Some(Schema::Sequence {
            element: Some(element),
            bound,
        }) => Some((element.as_ref(), *bound)),
        _ => None,
    };
    let (Some((node, _)), Some((_, edge_rows)), Some((occurrence, _))) =
        (rows("nodes"), rows("edges"), rows("occurrences"))
    else {
        return Ok(total);
    };
    let Schema::Fields(occurrence) = occurrence.representation() else {
        return Err(());
    };
    let guide = occurrence.get("guide").ok_or(())?;
    // The declared selector is one of these helpers; the dearest bounds it.
    let selector = summaries
        .values()
        .fold(Charge::default(), |dearest, summary| {
            Charge::choice(dearest, summary.charge)
        });
    let mut offer = selector;
    for part in [
        node,
        node,
        guide,
        &Schema::Word(u64::MAX),
        &Schema::Word(u64::MAX),
    ] {
        offer.append(part.measure()?.copy)?;
    }
    total.append(Charge {
        ticks: offer.ticks.checked_mul(u128::from(edge_rows)).ok_or(())?,
        storage: offer.storage.checked_mul(u128::from(edge_rows)).ok_or(())?,
    })?;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(entries: &[(&str, Schema)]) -> Schema {
        Schema::Fields(
            entries
                .iter()
                .map(|(name, schema)| ((*name).to_owned(), schema.clone()))
                .collect(),
        )
    }
    fn rows(element: Schema, bound: u64) -> Schema {
        Schema::Sequence {
            element: Some(Box::new(element)),
            bound,
        }
    }
    fn walk(edge_rows: u64) -> (Schema, Schema, Schema) {
        let node = fields(&[("atomBytes", Schema::Blob { low: 0, high: 8 })]);
        let guide = fields(&[("reach", Schema::Word(u64::MAX))]);
        let delivered = fields(&[
            ("nodes", rows(node.clone(), 2)),
            (
                "edges",
                rows(fields(&[("slot", Schema::Word(u64::MAX))]), edge_rows),
            ),
            ("occurrences", rows(fields(&[("guide", guide.clone())]), 3)),
        ]);
        (delivered, node, guide)
    }
    fn selector(ticks: u128, storage: u128) -> BTreeMap<String, Summary> {
        BTreeMap::from([(
            "app@1.choose".to_owned(),
            Summary {
                charge: Charge { ticks, storage },
                height: 1,
            },
        )])
    }

    #[test]
    fn atom_bytes_keep_their_existing_charge() -> Check<()> {
        let charge = read_charge(&Schema::Blob { low: 0, high: 10 }, &selector(9, 9))?;
        assert_eq!((charge.ticks, charge.storage), (11, 74));
        Ok(())
    }

    /// Oracle: the delivered layout, plus per edge row one selector charge
    /// and one candidate of two node rows, the guide and two words.
    #[test]
    fn a_closure_pays_its_selector_once_per_edge_row() -> Check<()> {
        for edge_rows in [4_u64, 5] {
            let (delivered, node, guide) = walk(edge_rows);
            let layout = delivered.measure()?;
            let word = Schema::Word(u64::MAX).measure()?.copy;
            let node = node.measure()?.copy;
            let guide = guide.measure()?.copy;
            let candidate_ticks = 2 * node.ticks + guide.ticks + 2 * word.ticks;
            let candidate_storage = 2 * node.storage + guide.storage + 2 * word.storage;
            let charge = read_charge(&delivered, &selector(7, 11))?;
            let n = u128::from(edge_rows);
            assert_eq!(
                charge.ticks,
                layout.copy.ticks + layout.checks + n * (7 + candidate_ticks)
            );
            assert_eq!(
                charge.storage,
                layout.copy.storage + n * (11 + candidate_storage)
            );
        }
        Ok(())
    }

    #[test]
    fn an_edge_list_pays_only_its_layout() -> Check<()> {
        let delivered = rows(fields(&[("toId", Schema::Blob { low: 32, high: 32 })]), 4);
        let layout = delivered.measure()?;
        let charge = read_charge(&delivered, &selector(1000, 1000))?;
        assert_eq!(
            (charge.ticks, charge.storage),
            (layout.copy.ticks + layout.checks, layout.copy.storage)
        );
        Ok(())
    }
}
