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
