// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Exact bounded-fold schema publication selection, separate from execution.
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_for_publication_v1, ProviderContractPublicationV1,
};
use sha2::{Digest, Sha256};
const SCHEMA: &[u8] = include_bytes!(
    "../assets/v1/edict-provider/contracts/bounded-state-fold-v1/edict-provider-contracts.cddl"
);
const MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/bounded-state-fold-v1/manifest.json");

#[test]
fn bounded_fold_pair_has_one_explicit_admission_route() {
    assert_eq!(
        hex::encode(Sha256::digest(SCHEMA)),
        "2210f75386be76ea4182b967d2c29d1ff24c1c575ca4ffa7bbc1735f6e2c0284"
    );
    assert_eq!(
        hex::encode(Sha256::digest(MANIFEST)),
        "48c08670e27bd722dc779488e5839f01252ff4ecc90c670043a509365bab6361"
    );
    let publications = [
        ProviderContractPublicationV1::PureBindings,
        ProviderContractPublicationV1::OrderedInstructions,
        ProviderContractPublicationV1::SourceFunctions,
        ProviderContractPublicationV1::SourceModules,
        ProviderContractPublicationV1::BoundedStateFold,
    ];
    let admitted = publications
        .into_iter()
        .filter_map(|publication| {
            admit_provider_contract_pack_for_publication_v1(publication, SCHEMA, MANIFEST).ok()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        admitted.len(),
        1,
        "one closed selector must admit the new pair"
    );
    assert_eq!(admitted[0].schema_bytes(), SCHEMA);
    assert_eq!(admitted[0].manifest_bytes(), MANIFEST);
    assert_eq!(admitted[0].contract_count(), 12);
    assert_eq!(admitted[0].domain_count(), 8);
}

#[test]
fn bounded_fold_selection_refuses_older_crossed_and_tampered_publications() {
    use echo_wesley_gen::provider_contract_pack::ProviderContractPackErrorKind as Kind;
    for (publication, schema, manifest) in [
        (ProviderContractPublicationV1::PureBindings, include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json").as_slice()),
        (ProviderContractPublicationV1::OrderedInstructions, include_bytes!("../assets/v1/edict-provider/contracts/ordered/edict-provider-contracts.cddl").as_slice(), include_bytes!("../assets/v1/edict-provider/contracts/ordered/manifest.json").as_slice()),
        (ProviderContractPublicationV1::SourceFunctions, include_bytes!("../assets/v1/edict-provider/contracts/source-functions-v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("../assets/v1/edict-provider/contracts/source-functions-v1/manifest.json").as_slice()),
        (ProviderContractPublicationV1::SourceModules, include_bytes!("../assets/v1/edict-provider/contracts/source-modules-v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("../assets/v1/edict-provider/contracts/source-modules-v1/manifest.json").as_slice()),
    ] {
        assert_eq!(admit_provider_contract_pack_for_publication_v1(publication, SCHEMA, MANIFEST).expect_err("old selector must not upgrade").kind(), Kind::ManifestSizeExceeded);
        assert_eq!(admit_provider_contract_pack_for_publication_v1(ProviderContractPublicationV1::BoundedStateFold, schema, manifest).expect_err("new selector must not downgrade").kind(), Kind::SchemaDigestMismatch);
        for (schema, manifest) in [(schema, MANIFEST), (SCHEMA, manifest)] {
            assert_eq!(admit_provider_contract_pack_for_publication_v1(ProviderContractPublicationV1::BoundedStateFold, schema, manifest).expect_err("crossed pair").kind(), Kind::SchemaBytesMismatch);
        }
    }
    let mut schema = SCHEMA.to_vec();
    schema[0] ^= 1;
    assert_eq!(
        admit_provider_contract_pack_for_publication_v1(
            ProviderContractPublicationV1::BoundedStateFold,
            &schema,
            MANIFEST
        )
        .expect_err("tampered schema")
        .kind(),
        Kind::SchemaBytesMismatch
    );
    let mut manifest = MANIFEST.to_vec();
    manifest.pop(); // Preserve valid JSON while changing its exact publication identity.
    assert_eq!(
        admit_provider_contract_pack_for_publication_v1(
            ProviderContractPublicationV1::BoundedStateFold,
            SCHEMA,
            &manifest
        )
        .expect_err("manifest byte substitution")
        .kind(),
        Kind::ManifestDigestMismatch
    );
    assert_eq!(
        admit_provider_contract_pack_for_publication_v1(
            ProviderContractPublicationV1::BoundedStateFold,
            SCHEMA,
            &vec![b' '; MANIFEST.len() + 1]
        )
        .expect_err("oversized manifest")
        .kind(),
        Kind::ManifestSizeExceeded
    );
}

use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn parameter() -> Value {
    map([
        ("id", text("arg.0")),
        ("alphaName", text("arg.0")),
        ("type", text("U64")),
    ])
}

fn identity_function(return_type: Option<Value>) -> Value {
    let mut fields = vec![
        ("params", Value::Array(vec![parameter()])),
        (
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                (
                    "result",
                    map([("kind", text("local")), ("ref", parameter())]),
                ),
            ]),
        ),
    ];
    if let Some(return_type) = return_type {
        fields.push(("returnType", return_type));
    }
    map(fields)
}

// Structural CDDL specimens only. Public compiler/provider/runtime witnesses
// independently establish source meaning and executable capability support.
fn core_module(functions: Option<Value>) -> Value {
    let mut fields = vec![
        ("apiVersion", text("edict.core/v1")),
        ("coordinate", text("conformance.source-functions@1")),
        ("imports", Value::Array(vec![])),
        ("types", map([])),
        (
            "intents",
            map([(
                "observe",
                map([
                    ("input", text("U64")),
                    ("output", text("U64")),
                    (
                        "requiredOperationProfile",
                        text("continuum.profile.read-only/v1"),
                    ),
                    ("inputConstraints", Value::Array(vec![])),
                    (
                        "coreEvaluationBudget",
                        map([
                            ("maxSteps", Value::Integer(100)),
                            ("maxAllocatedBytes", Value::Integer(1024)),
                            ("maxOutputBytes", Value::Integer(64)),
                        ]),
                    ),
                    (
                        "body",
                        map([
                            ("locals", Value::Array(vec![])),
                            ("nodes", Value::Array(vec![])),
                            (
                                "result",
                                map([
                                    ("kind", text("const")),
                                    (
                                        "value",
                                        map([
                                            ("kind", text("int")),
                                            ("width", text("U64")),
                                            ("value", Value::Integer(7)),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            )]),
        ),
        ("requiredCoreCapabilities", Value::Array(vec![])),
    ];
    if let Some(functions) = functions {
        fields.push(("functions", functions));
    }
    map(fields)
}

fn replace(value: &mut Value, key: &str, next: Value) {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    let (_, target) = entries
        .iter_mut()
        .find(|(k, _)| k == &text(key))
        .expect("field");
    *target = next;
}

#[test]
fn fold_shape_requires_the_new_schema_and_closed_fields() {
    use echo_wesley_gen::provider_contract_pack::ProviderContractValidationErrorKind as Kind;
    let old = admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::SourceModules,
        include_bytes!(
            "../assets/v1/edict-provider/contracts/source-modules-v1/edict-provider-contracts.cddl"
        ),
        include_bytes!("../assets/v1/edict-provider/contracts/source-modules-v1/manifest.json"),
    )
    .expect("old publication");
    let new = admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::BoundedStateFold,
        SCHEMA,
        MANIFEST,
    )
    .expect("fold publication");
    let function = identity_function(Some(text("U64")));
    let baseline = core_module(Some(map([("identity", function.clone())])));
    let baseline_bytes = encode_canonical_cbor_v1(&baseline).expect("baseline bytes");
    for pack in [&old, &new] {
        pack.validate_contract_bytes("core-module", &baseline_bytes)
            .expect("non-fold control");
    }
    // Structural specimens deliberately do not establish typing or execution.
    let initial = map([("kind", text("local")), ("ref", parameter())]);
    let fold = map([
        ("kind", text("fold")),
        ("initial", initial),
        ("step", text("conformance.source-functions@1.identity")),
        ("bound", Value::Integer(3)),
        ("stateType", text("U64")),
    ]);
    let specimen = |expression| {
        let mut run = function.clone();
        replace(
            &mut run,
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", expression),
            ]),
        );
        core_module(Some(map([("identity", function.clone()), ("run", run)])))
    };
    let bytes = encode_canonical_cbor_v1(&specimen(fold.clone())).expect("fold bytes");
    assert_eq!(
        old.validate_contract_bytes("core-module", &bytes)
            .expect_err("old schema refuses fold")
            .kind(),
        Kind::SchemaMismatch
    );
    new.validate_contract_bytes("core-module", &bytes)
        .expect("new schema admits fold shape");
    for key in ["initial", "step", "bound", "stateType"] {
        let mut invalid = fold.clone();
        let Value::Map(ref mut entries) = invalid else {
            panic!("map")
        };
        entries.retain(|(k, _)| k != &text(key));
        let bytes = encode_canonical_cbor_v1(&specimen(invalid)).expect("invalid bytes");
        assert_eq!(
            new.validate_contract_bytes("core-module", &bytes)
                .expect_err("missing field")
                .kind(),
            Kind::SchemaMismatch
        );
    }
    let mut invalid = fold;
    replace(&mut invalid, "bound", Value::Integer(-1));
    let bytes = encode_canonical_cbor_v1(&specimen(invalid)).expect("negative bound bytes");
    assert_eq!(
        new.validate_contract_bytes("core-module", &bytes)
            .expect_err("negative bound")
            .kind(),
        Kind::SchemaMismatch
    );
}
