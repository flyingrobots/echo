// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Coherently rebound native list authority, independent of lowerer endorsement.
use super::*;

fn expression(values: Vec<CanonicalValueV1>) -> CanonicalValueV1 {
    map([("kind", text("list")), ("values", list(values))])
}

fn transitive(result: &str, missing: bool) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    edited_fixture(|core, _| {
        for name in ["A", "B"] {
            add_field(
                map_field_mut(core, "types"),
                name,
                map([
                    ("kind", text("Nominal")),
                    ("contract", text(name)),
                    ("representation", text("U64")),
                ]),
            );
        }
        add_field(
            map_field_mut(core, "types"),
            "Items",
            map([
                ("kind", text("List")),
                ("item", text("A")),
                ("max", integer(2)),
            ]),
        );
        let coordinate = text_field(core, "coordinate").expect("coordinate");
        let parameter = local("arg.0", "A");
        let inner = call(
            &format!("{coordinate}.identity"),
            vec![reference(&parameter)],
        );
        let outer = call(
            &format!("{coordinate}.collect"),
            vec![reference(&parameter)],
        );
        let mut functions = vec![
            (
                text("collect"),
                function(
                    vec![parameter.clone()],
                    result,
                    vec![],
                    vec![],
                    expression(vec![inner.clone(), inner]),
                ),
            ),
            (
                text("relay"),
                function(vec![parameter.clone()], result, vec![], vec![], outer),
            ),
        ];
        if !missing {
            functions.push((
                text("identity"),
                function(
                    vec![parameter.clone()],
                    "A",
                    vec![],
                    vec![],
                    reference(&parameter),
                ),
            ));
        }
        add_field(core, "functions", CanonicalValueV1::Map(functions));
    })
}

#[test]
fn bounded_lists_preserve_nominal_identity_through_transitive_calls() {
    for result in ["Items", "List<A,max=2>"] {
        let (names, fixture, package) = transitive(result, false);
        let lowered =
            lowerer::lower(lowering_request(names, &fixture)).expect("nominal list calls");
        assert_eq!(lowered.outputs[0].artifact.bytes, package);
        assert!(verifier_accepts(names, &fixture, package));
    }
    for (result, missing) in [
        ("List<B,max=2>", false),
        ("List<U64,max=2>", false),
        ("List<A,max=1>", false),
        ("Items", true),
    ] {
        let (names, fixture, package) = transitive(result, missing);
        both_refuse(names, &fixture, package, result);
    }
}

#[test]
fn bounded_lists_cannot_join_records_with_different_field_names() {
    let (names, fixture, package) = edited_fixture(|core, _| {
        add_field(
            map_field_mut(core, "types"),
            "Row",
            map([
                ("kind", text("Record")),
                ("fields", map([("first", text("U64"))])),
            ]),
        );
        let row = |name| {
            map([
                ("kind", text("record")),
                ("fields", map([(name, literal("U64", 1))])),
            ])
        };
        add_field(
            core,
            "functions",
            map([(
                "rows",
                function(
                    vec![],
                    "List<Row,max=2>",
                    vec![],
                    vec![],
                    expression(vec![row("first"), row("second")]),
                ),
            )]),
        );
    });
    both_refuse(names, &fixture, package, "record field identity");
}
