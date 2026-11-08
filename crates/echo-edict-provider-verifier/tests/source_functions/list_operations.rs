// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Rebound consumer authority tests; public compiler/runtime witnesses are separate.
use super::*;

fn operation(name: &str, types: &[&str], args: Vec<CanonicalValueV1>) -> CanonicalValueV1 {
    let mut value = call(name, args);
    *map_field_mut(&mut value, "typeArgs") = list(types.iter().map(|t| text(*t)).collect());
    value
}
fn less(left: CanonicalValueV1, right: CanonicalValueV1) -> CanonicalValueV1 {
    map([
        ("kind", text("compare")),
        ("op", text("<")),
        ("left", left),
        ("right", right),
    ])
}
fn choose(
    predicate: CanonicalValueV1,
    yes: CanonicalValueV1,
    no: CanonicalValueV1,
) -> CanonicalValueV1 {
    map([
        ("kind", text("if")),
        ("predicate", predicate),
        ("then", yes),
        ("else", no),
    ])
}
fn fixture(
    op: &str,
    edit: impl FnOnce(&mut CanonicalValueV1),
) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    edited_fixture(|core, _| {
        let xs = local("arg.0", "List<U8,max=2>");
        let ys = local("arg.1", "List<U8,max=2>");
        let i = local("arg.2", "U64");
        let length = operation(
            "core.list.length",
            &["List<U8,max=2>"],
            vec![reference(&xs)],
        );
        let (result, ty) = match op {
            "length" => (length, "U64"),
            "at" => (
                choose(
                    less(reference(&i), length),
                    operation(
                        "core.list.at",
                        &["List<U8,max=2>"],
                        vec![reference(&xs), reference(&i)],
                    ),
                    literal("U8", 0),
                ),
                "U8",
            ),
            "append" => (
                choose(
                    less(length, literal("U64", 2)),
                    operation(
                        "core.list.append",
                        &["List<U8,max=2>"],
                        vec![reference(&xs), literal("U8", 7)],
                    ),
                    reference(&xs),
                ),
                "List<U8,max=2>",
            ),
            "concat" => (
                operation(
                    "core.list.concat",
                    &["List<U8,max=2>", "List<U8,max=2>"],
                    vec![reference(&xs), reference(&ys)],
                ),
                "List<U8,max=4>",
            ),
            _ => panic!("unknown case"),
        };
        *map_field_mut(core, "requiredCoreCapabilities") = list(vec![text("core.list/v1")]);
        add_field(
            core,
            "functions",
            map([(
                "worklist",
                function(vec![xs, ys, i], ty, vec![], vec![], result),
            )]),
        );
        edit(core);
    })
}
fn result(core: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    map_field_mut(
        map_field_mut(
            map_field_mut(map_field_mut(core, "functions"), "worklist"),
            "body",
        ),
        "result",
    )
}

#[test]
fn list_operations_are_independently_admitted_with_strict_domains() {
    for op in ["length", "at", "append", "concat"] {
        let (names, fixture, package) = fixture(op, |_| {});
        let lowered = lowerer::lower(lowering_request(names, &fixture)).expect(op);
        assert_eq!(lowered.outputs[0].artifact.bytes, package, "{op} carriers");
        assert!(verifier_accepts(names, &fixture, package), "{op} verifier");
    }
}

#[test]
fn list_inline_record_operands_match_named_structural_types() {
    let mut refused = Vec::new();
    for item in ["U8", "List<Bytes<max=1>,max=2>", "Record<inner:U8>"] {
        let inline = format!("List<Record<long:{item},z:U8>,max=2>");
        let (names, candidate, package) = fixture("concat", |core| {
            add_field(
                map_field_mut(core, "types"),
                "Row",
                map([
                    ("kind", text("Record")),
                    ("fields", map([("long", text(item)), ("z", text("U8"))])),
                ]),
            );
            let left = local("arg.0", "List<Row,max=2>");
            let right = local("arg.1", "List<Row,max=2>");
            *map_field_mut(map_field_mut(core, "functions"), "worklist") = function(
                vec![left.clone(), right.clone()],
                "List<Row,max=4>",
                vec![],
                vec![],
                operation(
                    "core.list.concat",
                    &[&inline, &inline],
                    vec![reference(&left), reference(&right)],
                ),
            );
        });
        if lowerer::lower(lowering_request(names, &candidate)).is_err() {
            refused.push(format!("lowerer: {inline}"));
        }
        if !verifier_accepts(names, &candidate, package) {
            refused.push(format!("verifier: {inline}"));
        }
    }
    assert!(
        refused.is_empty(),
        "valid record operands refused: {refused:#?}"
    );
}

#[test]
fn list_inline_record_operands_reject_noncanonical_references() {
    for record in [
        "Record<x:U8,x:U8>",
        "Record<z:U8,a:U8>",
        "Record<x:U8,>",
        "Record<:U8>",
        "Record<0x:U8>",
        "Record<x:>",
        "Record<x:List<U8,max=1>>>",
        "Record<x:Unknown>",
    ] {
        let operand = format!("List<{record},max=2>");
        let output = format!("List<{record},max=4>");
        let (names, candidate, package) = fixture("concat", |core| {
            let left = local("arg.0", &operand);
            let right = local("arg.1", &operand);
            *map_field_mut(map_field_mut(core, "functions"), "worklist") = function(
                vec![left.clone(), right.clone()],
                &output,
                vec![],
                vec![],
                operation(
                    "core.list.concat",
                    &[&operand, &operand],
                    vec![reference(&left), reference(&right)],
                ),
            );
        });
        both_refuse(names, &candidate, package, record);
    }
}

#[test]
fn list_operations_refuse_missing_duplicate_and_unknown_capabilities() {
    for capabilities in [
        vec![],
        vec!["core.list/v2"],
        vec!["core.list/v1", "core.list/v1"],
    ] {
        let (names, fixture, package) = fixture("length", |core| {
            *map_field_mut(core, "requiredCoreCapabilities") =
                list(capabilities.iter().map(|c| text(*c)).collect());
        });
        both_refuse(names, &fixture, package, "capability inventory");
    }
}

#[test]
fn list_operations_refuse_weak_and_unrelated_guards() {
    for op in ["at", "append"] {
        for mutation in ["weak", "unrelated", "missing", "wrong-type"] {
            let (names, fixture, package) = fixture(op, |core| {
                let value = result(core);
                if mutation == "missing" {
                    *value = map_field(value, "then").clone();
                    return;
                }
                if mutation == "wrong-type" {
                    *map_field_mut(map_field_mut(value, "then"), "typeArgs") =
                        list(vec![text("List<U64,max=2>")]);
                    return;
                }
                let guard = map_field_mut(value, "predicate");
                if mutation == "weak" {
                    *map_field_mut(guard, "op") = text("<=");
                } else {
                    let side = if op == "at" { "right" } else { "left" };
                    *map_field_mut(map_field_mut(guard, side), "args") =
                        list(vec![reference(&local("arg.1", "List<U8,max=2>"))]);
                }
            });
            both_refuse(names, &fixture, package, mutation);
        }
    }
}

#[test]
fn list_operations_refuse_wrong_append_elements_and_concat_bounds() {
    let (names, candidate, package) = fixture("append", |core| {
        let CanonicalValueV1::Array(args) =
            map_field_mut(map_field_mut(result(core), "then"), "args")
        else {
            panic!("args")
        };
        args[1] = literal("U64", 7);
    });
    both_refuse(names, &candidate, package, "wrong append element");
    for output in ["List<U8,max=3>", "List<U64,max=4>"] {
        let (names, candidate, package) = fixture("concat", |core| {
            *map_field_mut(
                map_field_mut(map_field_mut(core, "functions"), "worklist"),
                "returnType",
            ) = text(output);
        });
        both_refuse(names, &candidate, package, output);
    }
}

#[test]
fn list_concat_refuses_coherent_operand_widening() {
    let mut admitted = Vec::new();
    for (operand, claimed, output) in [
        ("List<U8,max=2>", "List<U8,max=3>", "List<U8,max=6>"),
        (
            "List<Bytes<max=1>,max=2>",
            "List<Bytes<max=2>,max=2>",
            "List<Bytes<max=2>,max=4>",
        ),
    ] {
        for indices in [vec![], vec![0], vec![1], vec![0, 1]] {
            let (names, candidate, package) = fixture("concat", |core| {
                let left = local("arg.0", operand);
                let right = local("arg.1", operand);
                let mut coordinates = [operand, operand];
                for index in &indices {
                    coordinates[*index] = claimed;
                }
                *map_field_mut(map_field_mut(core, "functions"), "worklist") = function(
                    vec![left.clone(), right.clone()],
                    output,
                    vec![],
                    vec![],
                    operation(
                        "core.list.concat",
                        &coordinates,
                        vec![reference(&left), reference(&right)],
                    ),
                );
            });
            let lowered = lowerer::lower(lowering_request(names, &candidate));
            let verified = verifier_accepts(names, &candidate, package);
            if indices.is_empty() {
                assert!(lowered.is_ok(), "valid wider result: {operand}");
                assert!(verified, "valid wider result: {operand}");
            } else {
                if lowered.is_ok() {
                    admitted.push(format!("lowerer: {operand} -> {claimed}, {indices:?}"));
                }
                if verified {
                    admitted.push(format!("verifier: {operand} -> {claimed}, {indices:?}"));
                }
            }
        }
    }
    assert!(
        admitted.is_empty(),
        "forged operands admitted: {admitted:#?}"
    );
}

// Both independent judgments must account for nested operand validation inside
// helper calls, including otherwise unused helpers in the checked closure.
#[test]
fn list_operand_validation_depth_matches_runtime_across_calls() {
    fn wrapped(mut value: CanonicalValueV1, count: usize) -> CanonicalValueV1 {
        for _ in 0..count {
            value = choose(
                map([
                    ("kind", text("compare")),
                    ("op", text("==")),
                    ("left", literal("U64", 0)),
                    ("right", literal("U64", 0)),
                ]),
                value,
                literal("U64", 0),
            );
        }
        value
    }
    let deep = format!("{}U8{}", "List<".repeat(40), ",max=1>".repeat(40));
    let mut observations = Vec::new();
    for outer_wrappers in [0, 20] {
        let (names, candidate, package) = edited_fixture(|core, _| {
            let coordinate = text_field(core, "coordinate")
                .expect("fixture coordinate")
                .to_owned();
            let xs = local("arg.0", &deep);
            let inner = wrapped(
                operation("core.list.length", &[&deep], vec![reference(&xs)]),
                10,
            );
            let outer = wrapped(
                call(&format!("{coordinate}.inner"), vec![reference(&xs)]),
                outer_wrappers,
            );
            *map_field_mut(core, "requiredCoreCapabilities") = list(vec![text("core.list/v1")]);
            add_field(
                core,
                "functions",
                map([
                    (
                        "inner",
                        function(vec![xs.clone()], "U64", vec![], vec![], inner),
                    ),
                    ("outer", function(vec![xs], "U64", vec![], vec![], outer)),
                ]),
            );
        });
        let lowered = lowerer::lower(lowering_request(names, &candidate));
        // Invoke the verifier on independently rebound carriers even if lowering
        // refuses. A lowerer failure must not prevent the second observation.
        let verified = verifier_accepts(names, &candidate, package.clone());
        if outer_wrappers == 0 {
            let success = lowered.as_ref().expect("zero-wrapper admission control");
            assert_eq!(success.outputs[0].artifact.bytes, package);
            assert!(verified, "zero-wrapper verifier control");
        }
        observations.push((outer_wrappers, lowered.is_ok(), verified));
    }
    assert_eq!(
        observations,
        vec![(0, true, true), (20, false, false)],
        "providers must refuse the helper closure that runtime preparation rejects"
    );
}

#[test]
fn list_domain_proof_depth_does_not_add_disjoint_branches() {
    let ty = "List<U64,max=1>";
    for (nots, wrappers, op, accepted) in [
        (66, 0, "<", false),
        (0, 65, "<", false),
        (36, 0, "<=", false),
        (0, 0, "<", true),
        (36, 0, "<", true),
        (0, 35, "<", true),
        (36, 35, "<", true),
    ] {
        let (names, candidate, package) = edited_fixture(|core, _| {
            let xs = local("arg.0", ty);
            let mut guard = map([
                ("kind", text("compare")),
                ("op", text(op)),
                ("left", literal("U64", 0)),
                (
                    "right",
                    operation("core.list.length", &[ty], vec![reference(&xs)]),
                ),
            ]);
            for _ in 0..nots {
                guard = map([("kind", text("not")), ("value", guard)]);
            }
            let mut access = operation(
                "core.list.at",
                &[ty],
                vec![reference(&xs), literal("U64", 0)],
            );
            for _ in 0..wrappers {
                access = choose(
                    map([
                        ("kind", text("compare")),
                        ("op", text("==")),
                        ("left", literal("U64", 0)),
                        ("right", literal("U64", 0)),
                    ]),
                    access,
                    literal("U64", 0),
                );
            }
            let body = choose(guard, access, literal("U64", 0));
            *map_field_mut(core, "requiredCoreCapabilities") = list(vec![text("core.list/v1")]);
            add_field(
                core,
                "functions",
                map([("lookup", function(vec![xs], "U64", vec![], vec![], body))]),
            );
        });
        let lowered = lowerer::lower(lowering_request(names, &candidate));
        let verified = verifier_accepts(names, &candidate, package.clone());
        assert_eq!(
            (lowered.is_ok(), verified),
            (accepted, accepted),
            "nots={nots} wrappers={wrappers} op={op}",
        );
        if accepted {
            assert!(lowered.is_ok_and(|out| out.outputs[0].artifact.bytes == package));
        }
    }
}
