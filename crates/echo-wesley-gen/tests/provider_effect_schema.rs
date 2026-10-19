// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Public configuration selection for the current authored-effect runtime kind.
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as V};
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
fn text(value: &str) -> V {
    V::Text(value.into())
}
fn map<const N: usize>(fields: [(&str, V); N]) -> V {
    V::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn configuration() -> V {
    map([
        (
            "apiVersion",
            text("echo.operation-lowering-configuration/v1"),
        ),
        ("programKind", text("edict-effect-program/v1")),
        ("identityLaw", V::Integer(1)),
        ("intrinsicCatalog", V::Bytes(vec![0xa0])),
        (
            "graphSchema",
            V::Bytes(
                encode_canonical_cbor_v1(&map([
                    ("node_types", V::Map(vec![])),
                    ("edge_types", V::Map(vec![])),
                ]))
                .expect("empty executable schema"),
            ),
        ),
        (
            "footprintLaw",
            V::Bytes(
                encode_canonical_cbor_v1(&map([
                    ("read_node_types", V::Array(vec![])),
                    ("write_node_types", V::Array(vec![])),
                    ("atom_types", V::Array(vec![])),
                    ("range_edge_types", V::Array(vec![])),
                    ("write_edge_types", V::Array(vec![])),
                    ("portal_owner_types", V::Array(vec![])),
                    ("max_reads", V::Integer(0)),
                    ("max_writes", V::Integer(0)),
                    ("max_range_edges", V::Integer(0)),
                ]))
                .expect("empty executable footprint"),
            ),
        ),
        ("maxInputBytes", V::Integer(65536)),
        ("maxResultBytes", V::Integer(65536)),
        ("maxAllocatedBytes", V::Integer(16777216)),
        (
            "budgetCeiling",
            map([
                ("steps", V::Integer(10000)),
                ("readBytes", V::Integer(4096)),
                ("writeBytes", V::Integer(4096)),
            ]),
        ),
    ])
}
fn set(value: &mut V, name: &str, replacement: V) {
    let V::Map(fields) = value else { panic!("map") };
    fields
        .iter_mut()
        .find(|(key, _)| key == &text(name))
        .expect("field")
        .1 = replacement;
}
fn generated() -> echo_wesley_gen::provider_artifacts::ProviderPrimaryArtifactsV1 {
    let pack = admit_provider_contract_pack_v1(
        include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl"),
        include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json"),
    )
    .expect("pack");
    let input = build_provider_generation_input_v1(
        include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json"),
        &pack,
        include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json"),
    )
    .expect("input");
    generate_provider_primary_artifacts_v1(&input, &pack).expect("primary")
}

#[test]
fn authored_effect_configuration_is_selected_with_explicit_resource_and_law_fields() {
    let generated = generated();
    let schema = generated.schema();
    let accepts = |value: &V| {
        schema.validate_root_bytes(
            "echo-operation-lowering-configuration",
            &encode_canonical_cbor_v1(value).expect("canonical"),
        )
    };
    accepts(&configuration())
        .expect("current authored-effect configuration must reach the lowerer");
    for (field, value) in [
        ("maxInputBytes", 0),
        ("maxResultBytes", 0),
        ("maxAllocatedBytes", 0),
    ] {
        let mut config = configuration();
        set(&mut config, field, V::Integer(value));
        accepts(&config).expect("zero declaration is explicit");
    }
    for (field, value) in [
        ("maxInputBytes", 65537),
        ("maxResultBytes", 65537),
        ("maxAllocatedBytes", 16777217),
        ("identityLaw", 0),
    ] {
        let mut config = configuration();
        set(&mut config, field, V::Integer(value));
        assert!(accepts(&config).is_err(), "{field} outside current profile");
    }
    for field in ["graphSchema", "footprintLaw", "intrinsicCatalog"] {
        let mut config = configuration();
        set(&mut config, field, V::Bytes(vec![]));
        assert!(accepts(&config).is_err(), "{field} must carry bytes");
        let mut config = configuration();
        set(&mut config, field, map([]));
        assert!(
            accepts(&config).is_err(),
            "{field} is canonical encoded data"
        );
    }
    for (steps, read, write, valid) in [
        (1, 0, 0, true),
        (
            i128::from(u64::MAX),
            i128::from(u64::MAX),
            i128::from(u64::MAX),
            true,
        ),
        (0, 0, 0, false),
        (1, -1, 0, false),
        (1, 0, -1, false),
    ] {
        let mut config = configuration();
        set(
            &mut config,
            "budgetCeiling",
            map([
                ("steps", V::Integer(steps)),
                ("readBytes", V::Integer(read)),
                ("writeBytes", V::Integer(write)),
            ]),
        );
        assert_eq!(
            accepts(&config).is_ok(),
            valid,
            "budget {steps}/{read}/{write}"
        );
    }
    let V::Map(fields) = configuration() else {
        panic!("map")
    };
    for (name, _) in &fields {
        let config = V::Map(
            fields
                .iter()
                .filter(|(key, _)| key != name)
                .cloned()
                .collect(),
        );
        assert!(accepts(&config).is_err(), "missing {name:?}");
    }
    let mut extra = fields;
    extra.push((text("operation"), text("consumer-specific")));
    assert!(accepts(&V::Map(extra)).is_err());
    let mut config = configuration();
    set(
        &mut config,
        "apiVersion",
        text("echo.operation-lowering-configuration/v2"),
    );
    assert!(accepts(&config).is_err());
}

fn publication_control() -> Vec<u8> {
    hex::decode(include_str!("fixtures/current-effect-publication/package.hex").trim())
        .expect("synthetic current publication")
}
#[test]
fn current_runtime_effect_publication_reaches_the_owning_output_root() {
    let bytes = publication_control();
    let expected = warp_core::echo_operation_package_id_v1(&bytes).as_hash();
    let prepared = warp_core::ExecutableOperationPackage::prepare(&bytes, expected)
        .expect("the current runtime accepts the synthetic wire control");
    assert_eq!(
        prepared.to_canonical_bytes().expect("runtime round trip"),
        bytes
    );
    generated()
        .schema()
        .validate_root_bytes("echo-operation-package", &bytes)
        .expect("owning provider schema must admit the current runtime effect variant");
}

#[test]
fn current_effect_output_fields_are_required_typed_and_closed() {
    use echo_wesley_gen::provider_canonical::decode_canonical_cbor_v1 as decode;
    let generated = generated();
    let schema = generated.schema();
    let accepts = |value: &V| {
        schema
            .validate_root_bytes(
                "echo-operation-package",
                &encode_canonical_cbor_v1(value).expect("canonical control"),
            )
            .is_ok()
    };
    let value = decode(&publication_control()).expect("current proposal");
    let V::Map(fields) = &value else {
        panic!("package")
    };
    for (name, _) in fields {
        let missing = V::Map(fields.iter().filter(|(k, _)| k != name).cloned().collect());
        assert!(!accepts(&missing), "missing {name:?}");
    }
    for (key, _) in fields {
        let V::Text(key) = key else {
            panic!("text field")
        };
        if key.ends_with("_identity") {
            let mut malformed = value.clone();
            set(&mut malformed, key, V::Bytes(vec![0; 31]));
            assert!(!accepts(&malformed), "short {key}");
        }
    }
    for (name, replacement) in [
        ("schema", text("echo.operation-package/v2")),
        ("obstruction_coordinate", text("")),
        ("operation_coordinate", text("")),
        ("program", V::Null),
        (
            "budget_ceiling",
            map([
                ("steps", V::Integer(0)),
                ("read_bytes", V::Integer(0)),
                ("write_bytes", V::Integer(0)),
            ]),
        ),
    ] {
        let mut malformed = value.clone();
        set(&mut malformed, name, replacement);
        assert!(!accepts(&malformed), "invalid {name}");
    }
    let mut extra = fields.clone();
    extra.push((text("package_kind"), text("edict-effect-program/v1")));
    assert!(!accepts(&V::Map(extra)));
    let mut malformed = value.clone();
    let V::Map(fields) = &mut malformed else {
        panic!("package")
    };
    let (_, V::Map(closure)) = fields
        .iter_mut()
        .find(|(key, _)| key == &text("semantic_closure"))
        .expect("closure")
    else {
        panic!("closure")
    };
    closure.push((text("lawpack_coordinate"), text("unretained.law@1")));
    closure.push((text("lawpack_identity"), V::Bytes(vec![1; 32])));
    assert!(
        !accepts(&malformed),
        "effect closure does not duplicate a lawpack"
    );
}

#[test]
fn output_schema_admission_does_not_verify_the_opaque_program() {
    use echo_wesley_gen::provider_canonical::decode_canonical_cbor_v1 as decode;
    let mut value = decode(&publication_control()).expect("proposal");
    set(&mut value, "program", V::Bytes(vec![0xa0]));
    let bytes = encode_canonical_cbor_v1(&value).expect("canonical metadata");
    generated()
        .schema()
        .validate_root_bytes("echo-operation-package", &bytes)
        .expect("the metadata schema only declares an opaque program byte string");
    let expected = warp_core::echo_operation_package_id_v1(&bytes).as_hash();
    assert!(
        warp_core::ExecutableOperationPackage::prepare(&bytes, expected).is_err(),
        "a fresh package pin and schema admission do not authenticate a program"
    );
}
