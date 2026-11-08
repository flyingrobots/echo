// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native provider judgments over rebound single-byte counts, not compiler proof.
use super::*;
fn bytes(value: &[u8]) -> CanonicalValueV1 {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bytes")),
                ("value", CanonicalValueV1::Bytes(value.to_vec())),
            ]),
        ),
    ])
}
fn typed(name: &str, types: &[&str], args: Vec<CanonicalValueV1>) -> CanonicalValueV1 {
    let mut value = call(name, args);
    *map_field_mut(&mut value, "typeArgs") = list(types.iter().map(|ty| text(*ty)).collect());
    value
}
fn add(left: CanonicalValueV1, addend: u64) -> CanonicalValueV1 {
    typed(
        "core.integer.add",
        &["U64"],
        vec![left, literal("U64", i128::from(addend))],
    )
}
fn count_fixture(case: &str, addend: u64) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    called_fixture(|core, _| {
        let coordinate = text_field(core, "coordinate")
            .expect("coordinate")
            .to_owned();
        // A named operand type must give the same structural maximum.
        let named = format!("{coordinate}.Blob");
        let operand = if case.starts_with("named") {
            add_field(
                map_field_mut(core, "types"),
                "Blob",
                map([
                    ("kind", text("Bytes")),
                    ("max", CanonicalValueV1::Integer(32)),
                ]),
            );
            named.as_str()
        } else {
            "Bytes<max=32>"
        };
        let parameter = local("arg.0", operand);
        let mut types = vec![operand];
        let mut args = vec![reference(&parameter), bytes(b"\n")];
        match case {
            "empty-needle" => args[1] = bytes(b""),
            "two-byte-needle" => args[1] = bytes(b"\r\n"),
            "integer-needle" => args[1] = literal("U64", 10),
            "local-needle" => args[1] = reference(&parameter),
            "missing-needle" => args.truncate(1),
            "extra-needle" => args.push(bytes(b"\n")),
            "missing-type" => types.clear(),
            "wrong-type" => types = vec!["U64"],
            "wide-type" => types = vec!["Bytes<max=64>"],
            _ => {}
        }
        let scan = typed("core.bytes.count", &types, args);
        let breaks = local("local.0", "U64");
        let mut functions = Vec::new();
        let lines = match case {
            "let" | "named-let" => function(
                vec![parameter],
                "U64",
                vec![breaks.clone()],
                vec![map([
                    ("kind", text("let")),
                    ("binding", breaks.clone()),
                    ("value", scan),
                ])],
                add(reference(&breaks), addend),
            ),
            // The binding's own value is an addition, so its cap is the one
            // the addition proof already established: 32 + 1.
            "sum-let" => function(
                vec![parameter],
                "U64",
                vec![breaks.clone()],
                vec![map([
                    ("kind", text("let")),
                    ("binding", breaks.clone()),
                    ("value", add(scan, 1)),
                ])],
                add(reference(&breaks), addend),
            ),
            "helper-result" => {
                functions.push((
                    "countBreaks",
                    function(vec![parameter.clone()], "U64", vec![], vec![], scan),
                ));
                function(
                    vec![parameter.clone()],
                    "U64",
                    vec![],
                    vec![],
                    add(
                        call(
                            &format!("{coordinate}.countBreaks"),
                            vec![reference(&parameter)],
                        ),
                        addend,
                    ),
                )
            }
            "wrong-result" => function(vec![parameter], "Bool", vec![], vec![], scan),
            _ => function(vec![parameter], "U64", vec![], vec![], add(scan, addend)),
        };
        functions.push(("lines", lines));
        for (name, definition) in functions {
            add_field(map_field_mut(core, "functions"), name, definition);
        }
    })
}
#[test]
fn source_function_counts_prove_line_count_directly_and_through_let_bindings() {
    for case in ["direct", "let", "named", "named-let"] {
        for addend in [1, u64::MAX - 32] {
            let (names, fixture, package) = count_fixture(case, addend);
            let lowered = lowerer::lower(lowering_request(names, &fixture))
                .unwrap_or_else(|_| panic!("{case} + {addend} lowers"));
            assert_eq!(lowered.outputs[0].artifact.bytes, package);
            assert!(
                verifier_accepts(names, &fixture, package),
                "{case} + {addend}: verifier refused the independently proven count"
            );
        }
    }
}
#[test]
fn source_function_count_sums_that_overflow_at_the_operand_maximum_refuse() {
    for case in ["direct", "let", "named", "named-let"] {
        let (names, fixture, package) = count_fixture(case, u64::MAX - 31);
        both_refuse(names, &fixture, package, case);
    }
}
#[test]
fn source_function_count_bounds_do_not_cross_helper_results() {
    let (names, fixture, package) = count_fixture("helper-result", 1);
    both_refuse(names, &fixture, package, "helper-result");
}
#[test]
fn source_function_count_rejects_needle_signature_and_result_forgeries() {
    for case in [
        "empty-needle",
        "two-byte-needle",
        "integer-needle",
        "local-needle",
        "missing-needle",
        "extra-needle",
        "missing-type",
        "wrong-type",
        "wide-type",
        "wrong-result",
    ] {
        let (names, fixture, package) = count_fixture(case, 1);
        both_refuse(names, &fixture, package, case);
    }
}
#[test]
fn a_bound_addition_carries_its_proven_cap_to_later_sums() {
    let (names, fixture, package) = count_fixture("sum-let", u64::MAX - 33);
    let lowered = lowerer::lower(lowering_request(names, &fixture)).expect("sum-let lowers");
    assert_eq!(lowered.outputs[0].artifact.bytes, package);
    assert!(verifier_accepts(names, &fixture, package));
    let (names, fixture, package) = count_fixture("sum-let", u64::MAX - 32);
    both_refuse(names, &fixture, package, "sum-let one past the carried cap");
}
