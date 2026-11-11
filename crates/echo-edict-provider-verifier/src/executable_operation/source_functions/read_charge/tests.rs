// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Structural laws of the verifier's read charge. Soundness against the
//! runtime meter is witnessed by evaluating worst-case closures.
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
fn walk(nodes: u64, edge_rows: u64, occurrences: u64) -> Schema {
    let node = fields(&[("atomBytes", Schema::Blob { low: 0, high: 8 })]);
    let guide = fields(&[("reach", Schema::Word(u64::MAX))]);
    fields(&[
        ("nodes", rows(node, nodes)),
        (
            "edges",
            rows(fields(&[("slot", Schema::Word(u64::MAX))]), edge_rows),
        ),
        (
            "occurrences",
            rows(fields(&[("guide", guide)]), occurrences),
        ),
    ])
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
    let charge = read_charge(&Schema::Blob { low: 0, high: 10 }, &selector(9, 9), 64)?;
    assert_eq!((charge.ticks, charge.storage), (11, 74));
    Ok(())
}

#[test]
fn an_edge_list_pays_its_scan_allowance_and_its_layout() -> Check<()> {
    let delivered = rows(fields(&[("toId", Schema::Blob { low: 32, high: 32 })]), 4);
    let layout = delivered.measure()?;
    let charge = read_charge(&delivered, &selector(1000, 1000), 10)?;
    assert_eq!(
        (charge.ticks, charge.storage),
        (
            10 + layout.copy.ticks + layout.checks,
            10 * 128 + layout.copy.storage
        )
    );
    Ok(())
}

#[test]
fn every_scan_and_every_edge_row_adds_its_own_charge() -> Check<()> {
    let base = read_charge(&walk(2, 3, 4), &selector(7, 11), 5)?;
    let scan = read_charge(&walk(2, 3, 4), &selector(7, 11), 6)?;
    assert_eq!(
        (scan.ticks - base.ticks, scan.storage - base.storage),
        (1, 128)
    );
    let dearer = read_charge(&walk(2, 3, 4), &selector(8, 11), 5)?;
    assert_eq!(
        dearer.ticks - base.ticks,
        3,
        "one selector call per edge row"
    );
    let wider = read_charge(&walk(2, 4, 4), &selector(7, 11), 5)?;
    assert!(wider.ticks - base.ticks > 7 && wider.storage - base.storage > 11);
    Ok(())
}
