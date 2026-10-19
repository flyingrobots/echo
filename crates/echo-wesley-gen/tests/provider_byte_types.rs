// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Byte aliases needed by authored graph catalog declarations.
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_canonical::CanonicalValueV1 as V;
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
use echo_wesley_gen::provider_semantics::{
    parse_provider_semantic_source_v1, ProviderSemanticSourceError,
};
use serde_json::{json, Value};
const SOURCE: &[u8] = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
fn source() -> Value {
    let mut source: Value = serde_json::from_slice(SOURCE).expect("source");
    for (name, min, max) in [
        ("Digest32", 32, 32),
        ("AtomBytes", 0, 1024),
        ("EmptyBytes", 0, 0),
        ("Window", 2, 8),
    ] {
        source["types"].as_array_mut().expect("types").push(json!({"identity":{"coordinate":format!("graph@1.{name}"),"domain":"echo.edict-provider/value/v1","authority":"echo.provider-semantic-declaration@1"},"shape":{"kind":"coreBytesAlias","minBytes":min,"maxBytes":max}}));
    }
    source["types"].as_array_mut().expect("types").push(json!({"identity":{"coordinate":"graph@1.ReadAddress","domain":"echo.edict-provider/value/v1","authority":"echo.provider-semantic-declaration@1"},"shape":{"kind":"record","fields":[{"name":"nodeId","type":"graph@1.Digest32"},{"name":"typeId","type":"graph@1.Digest32"},{"name":"warpId","type":"graph@1.Digest32"}]}}));
    source
}
fn source_error(value: &Value) -> ProviderSemanticSourceError {
    match parse_provider_semantic_source_v1(&serde_json::to_string(value).expect("JSON")) {
        Err(error) => error,
        Ok(_) => panic!("invalid source was admitted"),
    }
}
fn field<'a>(v: &'a V, key: &str) -> &'a V {
    let V::Map(fields) = v else { panic!("map") };
    &fields
        .iter()
        .find(|(k, _)| k == &V::Text(key.into()))
        .expect("field")
        .1
}
#[test]
fn byte_aliases_form_bounded_record_fields_and_emit_exact_lawpack_definitions() {
    let source = serde_json::to_vec(&source()).expect("JSON");
    let parsed = parse_provider_semantic_source_v1(std::str::from_utf8(&source).expect("UTF8"))
        .expect("bounded bytes are legal provider type leaves");
    let pack = admit_provider_contract_pack_v1(
        include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl"),
        include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json"),
    )
    .expect("pack");
    let input = build_provider_generation_input_v1(
        &source,
        &pack,
        include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json"),
    )
    .expect("input");
    let artifacts = generate_provider_primary_artifacts_v1(&input, &pack).expect("generation");
    let exports = artifacts
        .resource("resource.lawpack-exports")
        .expect("exports")
        .canonical_value();
    let V::Array(types) = field(exports, "types") else {
        panic!("types")
    };
    for (name, definition) in [
        ("Digest32", "Bytes<exact=32>"),
        ("AtomBytes", "Bytes<max=1024>"),
        ("EmptyBytes", "Bytes<exact=0>"),
        ("Window", "Bytes<min=2,max=8>"),
    ] {
        let coordinate = format!("graph@1.{name}");
        let exported = types
            .iter()
            .find(|v| field(v, "coordinate") == &V::Text(coordinate.clone()))
            .expect("byte alias exported");
        assert_eq!(field(exported, "definition"), &V::Text(definition.into()));
        let declared = parsed
            .source()
            .types
            .iter()
            .find(|v| v.identity.coordinate == coordinate)
            .expect("source type");
        assert_eq!(
            declared.shape.core_type_coordinate().as_deref(),
            Some(definition)
        );
        assert_eq!(declared.shape.accepts_raw_string("abc"), None);
    }
}
#[test]
fn inverted_byte_ranges_are_rejected_before_generation() {
    for (min, max) in [(33, 32), (1, 0), (u64::MAX, u64::MAX - 1)] {
        let mut value = source();
        let alias = value["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|v| v["identity"]["coordinate"] == "graph@1.Digest32")
            .expect("alias");
        alias["shape"]["minBytes"] = json!(min);
        alias["shape"]["maxBytes"] = json!(max);
        let error = source_error(&value);
        assert_eq!(
            error.kind(),
            echo_wesley_gen::provider_semantics::ProviderSemanticSourceErrorKind::InvalidByteBounds
        );
        assert_eq!(error.subject(), "graph@1.Digest32");
    }
}
#[test]
fn byte_alias_fields_are_explicit_unsigned_and_closed() {
    use echo_wesley_gen::provider_semantics::ProviderSemanticSourceErrorKind as K;
    for shape in [
        json!({"kind":"coreBytesAlias","minBytes":0}),
        json!({"kind":"coreBytesAlias","maxBytes":32}),
        json!({"kind":"coreBytesAlias","minBytes":-1,"maxBytes":32}),
        json!({"kind":"coreBytesAlias","minBytes":0,"maxBytes":"32"}),
        json!({"kind":"coreBytesAlias","minBytes":0,"maxBytes":32,"canonical":"raw-utf8"}),
    ] {
        let mut value = source();
        value["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|v| v["identity"]["coordinate"] == "graph@1.Digest32")
            .expect("alias")["shape"] = shape;
        assert_eq!(source_error(&value).kind(), K::MalformedDocument);
    }
}
#[test]
fn byte_leaves_do_not_make_recursive_records_or_core_identity_claims_valid() {
    use echo_wesley_gen::provider_semantics::ProviderSemanticSourceErrorKind as K;
    let mut value = source();
    value["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|v| v["identity"]["coordinate"] == "graph@1.ReadAddress")
        .expect("record")["shape"]["fields"]
        .as_array_mut()
        .expect("fields")
        .push(json!({"name":"next","type":"graph@1.ReadAddress"}));
    assert_eq!(source_error(&value).kind(), K::UnboundedTypeGraph);
    for coordinate in ["Bytes", "Bytes<exact=32>"] {
        let mut value = source();
        value["types"].as_array_mut().expect("types").push(json!({"identity":{"coordinate":coordinate,"domain":"echo.edict-provider/value/v1","authority":"echo.provider-semantic-declaration@1"},"shape":{"kind":"coreBytesAlias","minBytes":32,"maxBytes":32}}));
        assert_eq!(source_error(&value).kind(), K::CoreTypeAuthorityMismatch);
    }
}
