// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Synthetic serialization controls, not public compiler or execution evidence.
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as V};
use echo_edict_provider_lowerer::effect_publication::{
    encode_effect_publication, EffectPublicationInput,
};

fn text(value: &str) -> V {
    V::Text(value.into())
}
fn map<const N: usize>(fields: [(&str, V); N]) -> V {
    V::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn encoded(value: &V) -> Vec<u8> {
    echo_edict_canonical::encode_canonical_cbor_v1(value).expect("canonical fixture")
}
fn id(value: u8) -> V {
    V::Bytes(vec![value; 32])
}
fn get<'a>(value: &'a V, key: &str) -> &'a V {
    let V::Map(fields) = value else { panic!("map") };
    &fields
        .iter()
        .find(|(k, _)| k == &text(key))
        .expect("field")
        .1
}
fn set(value: &mut V, key: &str, replacement: V) {
    let V::Map(fields) = value else { panic!("map") };
    fields
        .iter_mut()
        .find(|(k, _)| k == &text(key))
        .expect("field")
        .1 = replacement;
}
fn decode_runtime(
    value: &V,
) -> Result<warp_core::ExecutableOperationPackage, warp_core::edict_effects::EffectPackageError> {
    let bytes = encoded(value);
    let expected = warp_core::echo_operation_package_id_v1(&bytes).as_hash();
    warp_core::ExecutableOperationPackage::prepare(&bytes, expected)
}
fn material() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        hex::decode(include_str!("fixtures/current-effect-wire/bundle.hex").trim())
            .expect("synthetic bundle"),
        hex::decode(include_str!("fixtures/current-effect-wire/source.hex").trim())
            .expect("synthetic source"),
        hex::decode(include_str!("fixtures/current-effect-wire/configuration.hex").trim())
            .expect("synthetic configuration"),
    )
}

fn emit(
    bundle: &[u8],
    source: &[u8],
    configuration: &[u8],
) -> Result<Vec<u8>, echo_edict_provider_lowerer::effect_publication::EffectPublicationError> {
    encode_effect_publication(EffectPublicationInput {
        compiler_program: bundle,
        application_schema_coordinate: "app.Exports",
        source_artifact: source,
        source_domain: "edict.source/v1",
        obstruction_coordinate: "app.Refusal",
        authority_profile_identity: [8; 32],
        configuration,
    })
}

#[test]
fn proposal_round_trips_the_current_runtime_decoder_without_rewriting_compiler_bytes() {
    let (bundle, source, config) = material();
    let bytes = emit(&bundle, &source, &config).expect("current effect proposal serialization");
    assert_eq!(bytes, emit(&bundle, &source, &config).expect("repeat"));
    let package = decode_runtime(&decode(&bytes).expect("current package"))
        .expect("independent current runtime decoder accepts the wire control");
    assert_eq!(
        package.to_canonical_bytes().expect("exact round trip"),
        bytes
    );
    let value = decode(&bytes).expect("package");
    let V::Bytes(program) = get(&value, "program") else {
        panic!("program bytes")
    };
    let program = decode(program).expect("program");
    assert_eq!(get(&program, "compiled_program"), &V::Bytes(bundle));
    let V::Map(closure) = get(&value, "semantic_closure") else {
        panic!("closure")
    };
    assert!(!closure.iter().any(|(k, _)| k == &text("lawpack_identity")));
    assert_eq!(get(&value, "authority_profile_identity"), &id(8));
}

#[test]
fn malformed_or_cross_bound_artifacts_refuse_before_serialization() {
    use echo_edict_provider_lowerer::effect_publication::EffectPublicationError as E;
    let (bundle, source, config) = material();
    for bytes in [vec![], vec![0xff], [bundle.as_slice(), &[0xf6]].concat()] {
        assert_eq!(emit(&bytes, &source, &config), Err(E::InvalidArtifact));
    }
    let mut changed = decode(&bundle).expect("bundle");
    let V::Bytes(raw) = get(&changed, "core_artifact") else {
        panic!("Core")
    };
    let mut core = decode(raw).expect("Core");
    set(&mut core, "coordinate", text("different.app"));
    set(&mut changed, "core_artifact", V::Bytes(encoded(&core)));
    assert_eq!(
        emit(&encoded(&changed), &source, &config),
        Err(E::InvalidArtifact)
    );
    let mut changed = decode(&bundle).expect("bundle");
    let V::Bytes(raw) = get(&changed, "target_ir_artifact") else {
        panic!("Target IR")
    };
    let mut target = decode(raw).expect("Target IR");
    let mut closure = get(&target, "semanticClosure").clone();
    let mut reference = get(&closure, "sourceCore").clone();
    set(
        &mut reference,
        "digest",
        V::Array(vec![text("sha256"), id(99)]),
    );
    set(&mut closure, "sourceCore", reference);
    set(&mut target, "semanticClosure", closure);
    set(
        &mut changed,
        "target_ir_artifact",
        V::Bytes(encoded(&target)),
    );
    assert_eq!(
        emit(&encoded(&changed), &source, &config),
        Err(E::InvalidArtifact)
    );
}

#[test]
fn resource_limits_and_current_envelopes_are_explicit_and_closed() {
    use echo_edict_provider_lowerer::effect_publication::EffectPublicationError as E;
    let (bundle, source, config) = material();
    for (key, value) in [
        ("identityLaw", V::Integer(2)),
        ("maxInputBytes", V::Integer(65537)),
        ("maxResultBytes", V::Integer(-1)),
        ("maxAllocatedBytes", V::Integer(16777217)),
        ("graphSchema", V::Bytes(vec![])),
        ("footprintLaw", V::Bytes(vec![0xff])),
        ("intrinsicCatalog", V::Bytes(encoded(&map([])))),
        (
            "apiVersion",
            text("echo.operation-lowering-configuration/v2"),
        ),
    ] {
        let mut changed = decode(&config).expect("config");
        set(&mut changed, key, value);
        assert_eq!(
            emit(&bundle, &source, &encoded(&changed)),
            Err(E::InvalidConfiguration)
        );
    }
    let mut changed = decode(&config).expect("config");
    set(
        &mut changed,
        "budgetCeiling",
        map([
            ("steps", V::Integer(0)),
            ("readBytes", V::Integer(0)),
            ("writeBytes", V::Integer(0)),
        ]),
    );
    assert_eq!(
        emit(&bundle, &source, &encoded(&changed)),
        Err(E::InvalidConfiguration)
    );
    let mut changed = decode(&config).expect("valid config for extra-field control");
    let V::Map(fields) = &mut changed else {
        panic!("config")
    };
    fields.push((text("legacy"), V::Bool(true)));
    assert_eq!(
        emit(&bundle, &source, &encoded(&changed)),
        Err(E::InvalidConfiguration)
    );
    let mut old = decode(&bundle).expect("bundle");
    set(
        &mut old,
        "schema",
        text("echo.compiler-produced-effect-program/v2"),
    );
    assert_eq!(
        emit(&encoded(&old), &source, &config),
        Err(E::InvalidArtifact)
    );
}

#[test]
fn explicit_zero_storage_limits_and_maximum_integer_budgets_remain_declarations() {
    let (bundle, source, config) = material();
    let mut config = decode(&config).expect("configuration");
    for key in ["maxInputBytes", "maxResultBytes", "maxAllocatedBytes"] {
        set(&mut config, key, V::Integer(0));
    }
    set(
        &mut config,
        "budgetCeiling",
        map([
            ("steps", V::Integer(i128::from(u64::MAX))),
            ("readBytes", V::Integer(i128::from(u64::MAX))),
            ("writeBytes", V::Integer(i128::from(u64::MAX))),
        ]),
    );
    let bytes = emit(&bundle, &source, &encoded(&config)).expect("bounded declarations");
    decode_runtime(&decode(&bytes).expect("package")).expect("wire-level preparation only");
}

#[test]
fn undeclared_obstruction_and_source_domain_refuse() {
    use echo_edict_provider_lowerer::effect_publication::EffectPublicationError as E;
    let (bundle, source, config) = material();
    let input = EffectPublicationInput {
        compiler_program: &bundle,
        application_schema_coordinate: "app.Exports",
        source_artifact: &source,
        source_domain: "edict.source/v1",
        obstruction_coordinate: "app.Refusal",
        authority_profile_identity: [8; 32],
        configuration: &config,
    };
    assert_eq!(
        encode_effect_publication(EffectPublicationInput {
            obstruction_coordinate: "app.Undeclared",
            ..input
        }),
        Err(E::InvalidCoordinate)
    );
    assert_eq!(
        encode_effect_publication(EffectPublicationInput {
            source_domain: "edict.source/v2",
            ..input
        }),
        Err(E::InvalidArtifact)
    );
}
