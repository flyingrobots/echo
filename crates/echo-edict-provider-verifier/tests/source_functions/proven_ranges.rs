// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Rebound native judgments of runtime slice and subtraction operands under
//! Edict's proven-range rules, not compiler acceptance evidence.
//!
//! A slice needs `start <= end <= len(bytes)` and a subtraction needs
//! `right <= left`, each from a dominating fact: equal terms, a zero right
//! side, ordered constants, an input constraint, or an earlier conjunct of
//! the same `all`. Conditional predicates, negations and later conjuncts
//! prove nothing.
use super::*;

const BYTES: &str = "Bytes<max=32>";

fn typed(name: &str, types: &[&str], args: Vec<CanonicalValueV1>) -> CanonicalValueV1 {
    let mut value = call(name, args);
    *map_field_mut(&mut value, "typeArgs") = list(types.iter().map(|ty| text(*ty)).collect());
    value
}
fn length(bytes: CanonicalValueV1) -> CanonicalValueV1 {
    typed("core.bytes.length", &[BYTES], vec![bytes])
}
fn slice(
    bytes: CanonicalValueV1,
    start: CanonicalValueV1,
    end: CanonicalValueV1,
) -> CanonicalValueV1 {
    typed("core.bytes.slice", &[BYTES], vec![bytes, start, end])
}
fn utf8(bytes: CanonicalValueV1) -> CanonicalValueV1 {
    typed("core.bytes.isUtf8", &[BYTES], vec![bytes])
}
fn subtract(left: CanonicalValueV1, right: CanonicalValueV1) -> CanonicalValueV1 {
    typed("core.integer.subtract", &["U64"], vec![left, right])
}
fn truth() -> CanonicalValueV1 {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bool")),
                ("value", CanonicalValueV1::Bool(true)),
            ]),
        ),
    ])
}
fn compare(op: &str, left: CanonicalValueV1, right: CanonicalValueV1) -> CanonicalValueV1 {
    map([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
fn all(values: Vec<CanonicalValueV1>) -> CanonicalValueV1 {
    map([("kind", text("all")), ("values", list(values))])
}
fn not(value: CanonicalValueV1) -> CanonicalValueV1 {
    map([("kind", text("not")), ("value", value)])
}
fn choose(
    predicate: CanonicalValueV1,
    then: CanonicalValueV1,
    otherwise: CanonicalValueV1,
) -> CanonicalValueV1 {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", then),
        ("else", otherwise),
    ])
}
fn flag(predicate: CanonicalValueV1) -> CanonicalValueV1 {
    choose(predicate, literal("U32", 1), literal("U32", 0))
}

fn probe(case: &str) -> CanonicalValueV1 {
    let bytes_local = local("arg.0", BYTES);
    let other_local = local("arg.1", BYTES);
    let at_local = local("arg.2", "U64");
    let elsewhere_local = local("arg.3", "U64");
    let bytes = reference(&bytes_local);
    let other = reference(&other_local);
    let at = reference(&at_local);
    let elsewhere = reference(&elsewhere_local);
    let prefix = |bytes: CanonicalValueV1| compare("<=", at.clone(), length(bytes));
    let use_prefix = || {
        compare(
            "==",
            utf8(slice(bytes.clone(), literal("U64", 0), at.clone())),
            truth(),
        )
    };
    let result = match case {
        "guarded prefix" => flag(all(vec![prefix(bytes.clone()), use_prefix()])),
        "zero offsets" => flag(compare(
            "==",
            utf8(slice(bytes.clone(), literal("U64", 0), literal("U64", 0))),
            truth(),
        )),
        "end offset" => flag(compare(
            "==",
            utf8(slice(
                bytes.clone(),
                literal("U64", 0),
                length(bytes.clone()),
            )),
            truth(),
        )),
        "guarded suffix" => flag(all(vec![
            prefix(bytes.clone()),
            compare(
                "==",
                utf8(slice(bytes.clone(), at.clone(), length(bytes.clone()))),
                truth(),
            ),
        ])),
        "unguarded" => flag(use_prefix()),
        "guard after use" => flag(all(vec![use_prefix(), prefix(bytes.clone())])),
        "guard on another operand" => flag(all(vec![prefix(other), use_prefix()])),
        "guard on another offset" => flag(all(vec![
            compare("<=", elsewhere, length(bytes.clone())),
            use_prefix(),
        ])),
        "guard by declared capacity" => flag(all(vec![
            compare("<=", at.clone(), literal("U64", 32)),
            use_prefix(),
        ])),
        "disjunctive guard" => flag(all(vec![
            not(all(vec![
                not(prefix(bytes.clone())),
                not(prefix(bytes.clone())),
            ])),
            use_prefix(),
        ])),
        "guard in a conditional" => {
            choose(prefix(bytes.clone()), flag(use_prefix()), literal("U32", 0))
        }
        "guard in a sibling branch" => {
            choose(prefix(bytes.clone()), literal("U32", 0), flag(use_prefix()))
        }
        "guarded subtraction" => flag(all(vec![
            compare("<=", elsewhere.clone(), at.clone()),
            compare("<=", subtract(at.clone(), elsewhere), literal("U64", 10)),
        ])),
        "unguarded subtraction" => flag(compare(
            "<=",
            subtract(at.clone(), elsewhere),
            literal("U64", 10),
        )),
        "reversed subtraction guard" => flag(all(vec![
            compare("<=", at.clone(), elsewhere.clone()),
            compare("<=", subtract(at.clone(), elsewhere), literal("U64", 10)),
        ])),
        _ => panic!("case {case}"),
    };
    function(
        vec![bytes_local, other_local, at_local, elsewhere_local],
        "U32",
        vec![],
        vec![],
        result,
    )
}
fn probe_fixture(case: &str) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    called_fixture(|core, _| add_field(map_field_mut(core, "functions"), "probe", probe(case)))
}

#[test]
fn dominating_range_facts_admit_runtime_slices_and_subtractions_in_both_judgments() {
    for case in [
        "guarded prefix",
        "zero offsets",
        "end offset",
        "guarded suffix",
        "guarded subtraction",
    ] {
        let (names, fixture, package) = probe_fixture(case);
        let lowered = lowerer::lower(lowering_request(names, &fixture))
            .unwrap_or_else(|_| panic!("{case} lowers"));
        assert_eq!(lowered.outputs[0].artifact.bytes, package, "{case}");
        assert!(
            verifier_accepts(names, &fixture, package),
            "{case}: verifier refused an independently proven range"
        );
    }
}

#[test]
fn missing_or_non_dominating_range_facts_refuse_in_both_judgments() {
    for case in [
        "unguarded",
        "guard after use",
        "guard on another operand",
        "guard on another offset",
        "guard by declared capacity",
        "disjunctive guard",
        "guard in a conditional",
        "guard in a sibling branch",
        "unguarded subtraction",
        "reversed subtraction guard",
    ] {
        let (names, fixture, package) = probe_fixture(case);
        both_refuse(names, &fixture, package, case);
    }
}

/// `endByte - startByte` in the intent body, proven by the fixture's own
/// input constraint `startByte <= endByte` when it is kept.
fn width_fixture(constrained: bool) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    called_fixture(|core, target| {
        let CanonicalValueV1::Array(constraints) = map_field(intent(core), "inputConstraints")
        else {
            panic!("constraints")
        };
        let predicate = map_field(&constraints[0], "predicate").clone();
        let start = map_field(&predicate, "left").clone();
        let end = map_field(&predicate, "right").clone();
        let kept = if constrained {
            constraints.clone()
        } else {
            vec![]
        };
        for (artifact, key) in [(&mut *core, "nodes"), (&mut *target, "pureBindings")] {
            *map_field_mut(intent(artifact), "inputConstraints") = list(kept.clone());
            let body = if key == "nodes" {
                map_field_mut(intent(artifact), "body")
            } else {
                intent(artifact)
            };
            let CanonicalValueV1::Array(bindings) = map_field_mut(body, key) else {
                panic!("bindings")
            };
            *map_field_mut(&mut bindings[0], "value") = subtract(end.clone(), start.clone());
        }
    })
}

#[test]
fn input_constraints_prove_intent_subtraction_in_both_judgments() {
    let (names, fixture, package) = width_fixture(true);
    let lowered = lowerer::lower(lowering_request(names, &fixture)).expect("proven width lowers");
    assert_eq!(lowered.outputs[0].artifact.bytes, package);
    assert!(verifier_accepts(names, &fixture, package));
    let (names, fixture, package) = width_fixture(false);
    both_refuse(names, &fixture, package, "unconstrained width");
}
