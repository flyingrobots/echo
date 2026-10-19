// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Rebound native judgments; public compiler evidence is retained separately.
use super::*;

fn add(left: CanonicalValueV1, right: CanonicalValueV1) -> CanonicalValueV1 {
    let mut value = call("core.integer.add", vec![left, right]);
    *map_field_mut(&mut value, "typeArgs") = list(vec![text("U64")]);
    value
}

fn addition_fixture(
    guarded: bool,
    unsafe_helper: bool,
) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    called_fixture(|core, target| {
        let a = local("arg.0", "U64");
        let body = map_field_mut(
            map_field_mut(map_field_mut(core, "functions"), "passNext"),
            "body",
        );
        *map_field_mut(body, "result") =
            add(reference(&a), literal("U64", i128::from(unsafe_helper)));
        edit_sum_intent(core, target, guarded);
    })
}

fn edit_sum_intent(core: &mut CanonicalValueV1, target: &mut CanonicalValueV1, guarded: bool) {
    let constraints = map_field(intent(core), "inputConstraints");
    let CanonicalValueV1::Array(items) = constraints else {
        panic!("constraints")
    };
    let original = items.first().expect("original input guard");
    let predicate = map_field(original, "predicate");
    let left = map_field(predicate, "left").clone();
    let right = map_field(predicate, "right").clone();
    let mut appended = items.clone();
    if guarded {
        for (index, operand) in [left.clone(), right.clone()].into_iter().enumerate() {
            let mut constraint = original.clone();
            *map_field_mut(&mut constraint, "coordinate") = text(format!("where.addition.{index}"));
            *map_field_mut(&mut constraint, "predicate") = map([
                ("kind", text("compare")),
                ("op", text("<=")),
                ("left", operand),
                ("right", literal("U64", 100)),
            ]);
            appended.push(constraint);
        }
    }
    for (artifact, key) in [(&mut *core, "nodes"), (&mut *target, "pureBindings")] {
        *map_field_mut(intent(artifact), "inputConstraints") = list(appended.clone());
        let body = if key == "nodes" {
            map_field_mut(intent(artifact), "body")
        } else {
            intent(artifact)
        };
        let CanonicalValueV1::Array(bindings) = map_field_mut(body, key) else {
            panic!("bindings")
        };
        *map_field_mut(&mut bindings[0], "value") = add(left.clone(), right.clone());
    }
}

#[test]
fn native_judgments_accept_proven_addition_and_zero_identity_helpers() {
    let (names, fixture, package) = addition_fixture(true, false);
    let lower = lowerer::lower(lowering_request(names, &fixture));
    let verify = verifier_accepts(names, &fixture, package.clone());
    assert_eq!(
        (lower.is_ok(), verify),
        (true, true),
        "both native judgments must independently support the call"
    );
    assert_eq!(
        lower.expect("proven native lowering").outputs[0]
            .artifact
            .bytes,
        package
    );
}

#[test]
fn native_judgments_refuse_missing_sum_proofs_and_unsafe_unused_helpers() {
    for (guarded, unsafe_helper) in [(false, false), (true, true)] {
        let (names, fixture, package) = addition_fixture(guarded, unsafe_helper);
        both_refuse(names, &fixture, package, "missing addition proof");
    }
}

#[test]
fn legacy_core_without_a_function_table_still_requires_the_addition_proof() {
    for guarded in [true, false] {
        let (names, fixture, package) =
            edited_fixture(|core, target| edit_sum_intent(core, target, guarded));
        let lower = lowerer::lower(lowering_request(names, &fixture));
        let verify = verifier_accepts(names, &fixture, package);
        assert_eq!(
            (lower.is_ok(), verify),
            (guarded, guarded),
            "absence of a source-function table cannot skip new arithmetic validation"
        );
    }
}

#[test]
fn addition_predicate_proofs_follow_prefix_order_and_do_not_escape_negation() {
    for mode in ["prefix", "late", "negated"] {
        let (names, fixture, package) = edited_fixture(|core, target| {
            edit_sum_intent(core, target, true);
            let CanonicalValueV1::Array(clauses) = map_field(intent(core), "inputConstraints")
            else {
                panic!("constraints")
            };
            let bounds: Vec<_> = clauses
                .iter()
                .skip(1)
                .map(|clause| map_field(clause, "predicate").clone())
                .collect();
            let mut clause = clauses[0].clone();
            let CanonicalValueV1::Array(nodes) =
                map_field(map_field(intent(core), "body"), "nodes")
            else {
                panic!("nodes")
            };
            let sum = map_field(&nodes[0], "value").clone();
            let check = map([
                ("kind", text("compare")),
                ("op", text("<=")),
                ("left", sum),
                ("right", literal("U64", 200)),
            ]);
            let items = if mode == "late" {
                vec![check, bounds[0].clone(), bounds[1].clone()]
            } else {
                vec![bounds[0].clone(), bounds[1].clone(), check]
            };
            let predicate = if mode == "negated" {
                map([
                    ("kind", text("not")),
                    (
                        "value",
                        map([("kind", text("all")), ("values", list(bounds))]),
                    ),
                ])
            } else {
                map([("kind", text("all")), ("values", list(items))])
            };
            *map_field_mut(&mut clause, "predicate") = predicate;
            for artifact in [&mut *core, &mut *target] {
                *map_field_mut(intent(artifact), "inputConstraints") = list(vec![clause.clone()]);
            }
        });
        let lower = lowerer::lower(lowering_request(names, &fixture)).is_ok();
        let verify = verifier_accepts(names, &fixture, package);
        assert_eq!(
            (lower, verify),
            (mode == "prefix", mode == "prefix"),
            "{mode}: no self-justifying or negation-leaked arithmetic proof"
        );
    }
}
