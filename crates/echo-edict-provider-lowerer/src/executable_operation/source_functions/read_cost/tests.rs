// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Structural laws of the static read cost. Soundness against the runtime
//! meter is witnessed separately by evaluating worst-case closures.
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
fn closure(nodes: u64, edges: u64, occurrences: u64) -> Ty {
    let node = record(&[
        ("nodeId", Ty::Bytes(32, 32)),
        ("atomBytes", Ty::Bytes(0, 8)),
    ]);
    let guide = record(&[("reach", word())]);
    record(&[
        ("nodes", list(node, nodes)),
        ("edges", list(record(&[("from", word())]), edges)),
        (
            "occurrences",
            list(record(&[("guide", guide), ("node", word())]), occurrences),
        ),
    ])
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
    let cost = read_cost(&Ty::Bytes(0, 10), &helper(1000, 1000), 64)?;
    assert_eq!((cost.steps, cost.bytes), (11, 74));
    Ok(())
}

#[test]
fn an_edge_read_pays_its_scan_allowance_and_its_whole_result() -> Check<()> {
    let edges = list(record(&[("edgeId", Ty::Bytes(32, 32))]), 4);
    let (copy, validations, _, _) = shape(&edges)?;
    let cost = read_cost(&edges, &helper(1000, 1000), 10)?;
    assert_eq!(
        (cost.steps, cost.bytes),
        (10 + copy.steps + validations, 10 * 128 + copy.bytes)
    );
    Ok(())
}

#[test]
fn each_scan_and_each_edge_row_adds_at_least_its_own_charge() -> Check<()> {
    let base = read_cost(&closure(2, 3, 4), &helper(7, 11), 5)?;
    let scan = read_cost(&closure(2, 3, 4), &helper(7, 11), 6)?;
    assert_eq!((scan.steps - base.steps, scan.bytes - base.bytes), (1, 128));
    let edge = read_cost(&closure(2, 4, 4), &helper(7, 11), 5)?;
    // One more edge row costs at least the selector and a candidate argument.
    assert!(edge.steps - base.steps > 7 && edge.bytes - base.bytes > 11);
    let dearer = read_cost(&closure(2, 3, 4), &helper(8, 11), 5)?;
    assert_eq!(
        dearer.steps - base.steps,
        3,
        "one selector call per edge row"
    );
    Ok(())
}

#[test]
fn an_overflowing_closure_charge_is_refused() {
    assert!(read_cost(&closure(1, u64::MAX, 1), &helper(u64::MAX / 2, 1), 1).is_err());
}
