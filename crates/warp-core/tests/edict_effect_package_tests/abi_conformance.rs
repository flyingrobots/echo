// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-ABI conformance for the six shared `echo.dpo@1` intrinsics.
//!
//! The provider's pinned catalog names Echo-owned ABI coordinates. An
//! application keeps its own nominal types; its declarations must conform to
//! the ABI representation of the intrinsic its adapter explicitly selects.
//! Expected coordinates are written out here rather than read from the
//! implementation, so the two can disagree.
use super::mutations::{fixture_with, Shape};
use super::*;

const PROFILE: &str = "continuum.profile.graph-effects/v1";
const MUTATIONS: [&str; 5] = [
    "ensure-node",
    "create-node",
    "replace-atom",
    "ensure-edge",
    "replace-edge",
];
const NODE_PAYLOAD_MUTATIONS: [&str; 3] = ["ensure-node", "create-node", "replace-atom"];

fn accepts(name: &str, shape: &Shape) {
    let (publication, _) = fixture_with(name, PROFILE, shape);
    prepare(&publication).unwrap_or_else(|error| panic!("{name}: {error:?}"));
}

fn refuses(name: &str, shape: &Shape, why: &str) {
    let (publication, _) = fixture_with(name, PROFILE, shape);
    assert!(prepare(&publication).is_err(), "{name}: {why} was accepted");
}

fn with_field(name: &str, field: &'static str, ty: &'static str) -> Shape {
    let mut shape = Shape::conforming(name);
    for entry in &mut shape.fields {
        if entry.0 == field {
            entry.1 = ty;
        }
    }
    shape
}

#[test]
fn differently_named_application_types_conform_to_the_same_abi() {
    for name in MUTATIONS {
        accepts(name, &Shape::conforming(name));
        accepts(
            name,
            &Shape {
                input_name: "JimRopeEdit",
                output_name: "JimRopeEditDone",
                ..Shape::conforming(name)
            },
        );
    }
}

#[test]
fn the_catalog_must_name_the_selected_intrinsics_abi_not_an_application_type() {
    for name in MUTATIONS {
        let mut shape = Shape::conforming(name);
        shape.catalog_argument = shape.input_name.to_owned();
        shape.catalog_return = shape.output_name.to_owned();
        refuses(name, &shape, "a catalog entry naming application types");
    }
    let mut crossed = Shape::conforming("replace-atom");
    crossed.catalog_argument = "echo.dpo@1.ensure-node.input".to_owned();
    refuses("replace-atom", &crossed, "another intrinsic's ABI input");
    let mut crossed = Shape::conforming("ensure-edge");
    crossed.catalog_return = "echo.dpo@1.node-atom-read.result".to_owned();
    refuses("ensure-edge", &crossed, "another intrinsic's ABI result");
}

#[test]
fn fixed_identity_fields_require_exactly_thirty_two_bytes() {
    for name in MUTATIONS {
        refuses(
            name,
            &with_field(name, "warpId", "Bytes<max=32>"),
            "a maximum-bounded identity",
        );
        refuses(
            name,
            &with_field(name, "warpId", "Bytes<exact=31>"),
            "a 31-byte identity",
        );
    }
}

#[test]
fn field_membership_is_exact() {
    for name in MUTATIONS {
        let mut missing = Shape::conforming(name);
        missing.fields.pop();
        refuses(name, &missing, "a missing field");

        let mut renamed = Shape::conforming(name);
        renamed.fields[0].0 = "warp";
        refuses(name, &renamed, "a renamed field");

        let mut extra = Shape::conforming(name);
        extra.fields.push(("note", "Bytes<exact=32>"));
        refuses(name, &extra, "an extra field");
    }
}

#[test]
fn payload_maxima_conform_within_the_runtime_ceiling() {
    for name in NODE_PAYLOAD_MUTATIONS {
        accepts(name, &with_field(name, "atomBytes", "Bytes<max=16>"));
        accepts(name, &with_field(name, "atomBytes", "Bytes<max=16777216>"));
        refuses(
            name,
            &with_field(name, "atomBytes", "Bytes<max=16777217>"),
            "a payload above the 16 MiB ceiling",
        );
        refuses(
            name,
            &with_field(name, "atomBytes", "Bytes<exact=32>"),
            "a payload with a nonzero minimum",
        );
    }
}

#[test]
fn the_successful_result_must_be_the_empty_record() {
    for name in MUTATIONS {
        let mut shape = Shape::conforming(name);
        shape.output = map([
            ("kind", text("Record")),
            ("fields", map([("done", text("Bytes<exact=32>"))])),
        ]);
        refuses(name, &shape, "a non-empty successful result");
    }
}

fn read_catalog_with(edit: impl Fn(&mut V)) -> V {
    let mut catalog = catalog_fixture();
    edit(value_field_mut(
        value_field_mut(&mut catalog, "intrinsics"),
        "echo.dpo@1.node-atom-read",
    ));
    catalog
}

#[test]
fn the_read_catalog_names_its_abi_and_its_failures_carry_the_abi_input() {
    let intrinsic = get(
        get(&catalog_fixture(), "intrinsics"),
        "echo.dpo@1.node-atom-read",
    )
    .clone();
    assert_eq!(
        get(&intrinsic, "argumentTypes"),
        &V::Array(vec![text("echo.dpo@1.node-atom-read.input")])
    );
    assert_eq!(
        get(&intrinsic, "returnType"),
        &text("echo.dpo@1.node-atom-read.result")
    );
    let V::Map(failures) = get(&intrinsic, "effectFailures") else {
        panic!("failures")
    };
    assert_eq!(failures.len(), 4);
    for (_, failure) in failures {
        assert_eq!(
            get(failure, "payloadType"),
            &text("echo.dpo@1.node-atom-read.input")
        );
    }
    assert!(prepare(&publication_for_lowering(&repinned_lowering_artifacts())).is_ok());
}

#[test]
fn a_read_catalog_naming_the_application_type_is_refused() {
    let catalog = read_catalog_with(|intrinsic| {
        set(
            intrinsic,
            "argumentTypes",
            V::Array(vec![text("jedit.text@1.FactAddress")]),
        );
    });
    refused(&publication_for_lowering(&lowering_fixture(Some(catalog))));
    let catalog = read_catalog_with(|intrinsic| {
        let failure = value_field_mut(value_field_mut(intrinsic, "effectFailures"), "atomMissing");
        set(failure, "payloadType", text("jedit.text@1.FactAddress"));
    });
    refused(&publication_for_lowering(&lowering_fixture(Some(catalog))));
}

#[test]
fn a_read_failure_must_carry_the_effects_own_input_type() {
    let (_, _, mut exports) = repinned_compiler_artifacts();
    let V::Array(effects) = value_field_mut(&mut exports, "effects") else {
        panic!("effects")
    };
    let V::Map(failures) = value_field_mut(&mut effects[0], "effectFailures") else {
        panic!("failures")
    };
    set(&mut failures[0].1, "payloadType", text("U32"));
    let lowering = lowering_fixture_inputs(None, Some(exports), None);
    refused(&publication_for_lowering(&lowering));
}

#[test]
fn each_mutation_declares_its_own_effect_kind() {
    for name in MUTATIONS {
        accepts(name, &Shape::conforming(name));
        for wrong in ["write", "read", "delete"] {
            let mut shape = Shape::conforming(name);
            shape.effect_kind = wrong.to_owned();
            refuses(name, &shape, "a mutation declaring another effect kind");
        }
    }
    let mut crossed = Shape::conforming("create-node");
    crossed.effect_kind = "ensure".to_owned();
    refuses("create-node", &crossed, "create-node declaring ensure");
}

#[test]
fn each_mutation_write_class_is_its_effect_kind() {
    for name in MUTATIONS {
        for wrong in ["write", "read", "delete"] {
            let mut shape = Shape::conforming(name);
            shape.write_class = wrong.to_owned();
            refuses(
                name,
                &shape,
                "a write class outside Edict's ABI or another kind",
            );
        }
    }
}

const READ: &str = "echo.dpo@1.node-atom-read";

pub(super) fn rename_read_failure(catalog: &mut V, from: &str, to: &str) {
    let V::Map(failures) = value_field_mut(
        value_field_mut(value_field_mut(catalog, "intrinsics"), READ),
        "effectFailures",
    ) else {
        panic!("failures")
    };
    let entry = failures
        .iter_mut()
        .find(|(name, _)| name == &text(from))
        .expect("failure");
    entry.0 = text(to);
}

#[test]
fn the_read_catalog_names_its_failures_by_identifier() {
    let catalog = catalog_fixture();
    let V::Map(failures) = get(get(get(&catalog, "intrinsics"), READ), "effectFailures") else {
        panic!("failures")
    };
    let mut names = failures
        .iter()
        .map(|(name, _)| match name {
            V::Text(name) => name.clone(),
            _ => panic!("name"),
        })
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "atomByteBoundExceeded",
            "atomMissing",
            "atomRequired",
            "atomTypeMismatch"
        ]
    );
    assert!(prepare(&publication_for_lowering(&repinned_lowering_artifacts())).is_ok());
}

#[test]
fn a_read_spelled_with_runtime_tags_is_refused_on_the_authored_route() {
    // The bounded-read route's adapter maps to runtime tags. The authored
    // route's catalog names failures by identifier, so no tag passes through.
    let mut catalog = catalog_fixture();
    for (tag, name) in READ_FAILURE_NAMES {
        rename_read_failure(&mut catalog, name, tag);
    }
    let lowering = lowering_fixture_inputs(Some(catalog), None, Some(captured_adapter()));
    refused(&publication_for_lowering(&lowering));
}

#[test]
fn unknown_missing_and_crossed_read_failure_names_are_refused() {
    // Unknown: a catalog identifier the read does not define, consistently
    // used by catalog and adapter.
    let mut catalog = catalog_fixture();
    rename_read_failure(&mut catalog, "atomMissing", "atomAbsent");
    let mut adapter = catalog_named_adapter(&captured_adapter());
    let V::Map(mappings) = value_field_mut(
        value_field_mut(
            value_field_mut(&mut adapter, "effectImplementations"),
            "jedit.text@1.readFact",
        ),
        "failureMappings",
    ) else {
        panic!("mappings")
    };
    for (_, target) in mappings.iter_mut() {
        if target == &text("atomMissing") {
            *target = text("atomAbsent");
        }
    }
    refused(&publication_for_lowering(&lowering_fixture_inputs(
        Some(catalog),
        None,
        Some(adapter),
    )));
    // Missing: a catalog without one of the read's four failures.
    let catalog = read_catalog_with(|intrinsic| {
        let V::Map(failures) = value_field_mut(intrinsic, "effectFailures") else {
            panic!("failures")
        };
        failures.retain(|(name, _)| name != &text("atomRequired"));
    });
    refused(&publication_for_lowering(&lowering_fixture(Some(catalog))));
    // Crossed: two identifiers swapping their authority classes.
    let catalog = read_catalog_with(|intrinsic| {
        let failures = value_field_mut(intrinsic, "effectFailures");
        set(
            value_field_mut(failures, "atomMissing"),
            "authorityClass",
            text("integrityFault"),
        );
        set(
            value_field_mut(failures, "atomRequired"),
            "authorityClass",
            text("resourceFault"),
        );
    });
    refused(&publication_for_lowering(&lowering_fixture(Some(catalog))));
}
