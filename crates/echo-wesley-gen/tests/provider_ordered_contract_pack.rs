// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Exact publication selection is separate from runtime capability admission.

use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_for_publication_v1, admit_provider_contract_pack_v1,
    AdmittedProviderContractPackV1, ProviderContractPackError, ProviderContractPackErrorKind,
    ProviderContractPublicationV1, ProviderContractValidationErrorKind,
};

use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};

use sha2::{Digest, Sha256};

const LEGACY_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const LEGACY_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
const ORDERED_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/ordered/edict-provider-contracts.cddl");
const ORDERED_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/ordered/manifest.json");

fn admit_ordered(
    schema: &[u8],
    manifest: &[u8],
) -> Result<AdmittedProviderContractPackV1, ProviderContractPackError> {
    admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::OrderedInstructions,
        schema,
        manifest,
    )
}

#[test]
fn ordered_publication_admits_exact_schema_and_manifest() {
    let pack = admit_ordered(ORDERED_CDDL, ORDERED_MANIFEST)
        .expect("the explicitly selected ordered publication must admit");
    assert_eq!(pack.schema_bytes(), ORDERED_CDDL);
    assert_eq!(pack.manifest_bytes(), ORDERED_MANIFEST);
    assert_eq!(pack.contract_count(), 11);
    assert_eq!(pack.domain_count(), 7);
    assert_eq!(pack.resource_count(), 5);
}

#[test]
fn admitted_identity_and_selector_match_each_exact_publication() {
    let ordered = admit_ordered(ORDERED_CDDL, ORDERED_MANIFEST).expect("ordered admission");
    assert_eq!(
        ordered.publication(),
        ProviderContractPublicationV1::OrderedInstructions
    );
    assert_eq!(
        ordered.schema_sha256(),
        "82273f3ea016a421c881f15b0fd451802205903ac9177bac8accbf3173f66d2c"
    );
    assert_eq!(
        ordered.manifest_sha256(),
        "6303668861667a30418870ef25e5f169017905ae1f9d261451ba298120afdd9d"
    );
    let legacy = admit_provider_contract_pack_v1(LEGACY_CDDL, LEGACY_MANIFEST)
        .expect("original entry point preserves the legacy publication");
    assert_eq!(
        legacy.publication(),
        ProviderContractPublicationV1::PureBindings
    );
    assert_ne!(legacy, ordered);
}

#[test]
fn neither_entry_point_silently_selects_the_other_publication() {
    assert_eq!(
        admit_provider_contract_pack_v1(ORDERED_CDDL, ORDERED_MANIFEST)
            .expect_err("legacy admission must retain its size bound")
            .kind(),
        ProviderContractPackErrorKind::ManifestSizeExceeded,
    );
    assert_eq!(
        admit_ordered(LEGACY_CDDL, LEGACY_MANIFEST)
            .expect_err("explicit ordered selection must reject legacy bytes")
            .kind(),
        ProviderContractPackErrorKind::SchemaDigestMismatch,
    );
}

#[test]
fn crossed_publication_pairs_reject() {
    for (schema, manifest) in [
        (ORDERED_CDDL, LEGACY_MANIFEST),
        (LEGACY_CDDL, ORDERED_MANIFEST),
    ] {
        assert_eq!(
            admit_ordered(schema, manifest)
                .expect_err("mixed pair must reject")
                .kind(),
            ProviderContractPackErrorKind::SchemaBytesMismatch,
        );
    }
}

#[test]
fn tampered_schema_and_self_consistent_republication_reject() {
    let mut schema = ORDERED_CDDL.to_vec();
    schema[0] ^= 1;
    assert_eq!(
        admit_ordered(&schema, ORDERED_MANIFEST)
            .expect_err("schema tampering")
            .kind(),
        ProviderContractPackErrorKind::SchemaBytesMismatch,
    );
    let mut manifest: serde_json::Value = serde_json::from_slice(ORDERED_MANIFEST).expect("JSON");
    manifest["schema"]["bytesHex"] = hex::encode(&schema).into();
    manifest["schema"]["rawSha256"] = hex::encode(Sha256::digest(&schema)).into();
    let bytes = serde_json::to_vec(&manifest).expect("JSON");
    assert_eq!(
        admit_ordered(&schema, &bytes)
            .expect_err("a self-consistent unpinned publication")
            .kind(),
        ProviderContractPackErrorKind::SchemaDigestMismatch,
    );
}

#[test]
fn manifest_bytes_remain_exact_and_size_bounded() {
    let manifest: serde_json::Value = serde_json::from_slice(ORDERED_MANIFEST).expect("JSON");
    let reformatted = serde_json::to_vec(&manifest).expect("JSON");
    assert_eq!(
        admit_ordered(ORDERED_CDDL, &reformatted)
            .expect_err("reformatted publication")
            .kind(),
        ProviderContractPackErrorKind::ManifestDigestMismatch,
    );
    let mut oversized = ORDERED_MANIFEST.to_vec();
    oversized.push(b' ');
    assert_eq!(
        admit_ordered(ORDERED_CDDL, &oversized)
            .expect_err("oversized publication")
            .kind(),
        ProviderContractPackErrorKind::ManifestSizeExceeded,
    );
}

#[test]
fn unsupported_manifest_api_and_mutated_resources_reject() {
    for (field, expected) in [
        ("api", ProviderContractPackErrorKind::UnsupportedApiVersion),
        (
            "resource",
            ProviderContractPackErrorKind::ResourceRawDigestMismatch,
        ),
    ] {
        let mut manifest: serde_json::Value =
            serde_json::from_slice(ORDERED_MANIFEST).expect("JSON");
        if field == "api" {
            manifest["apiVersion"] = "edict.provider-contract-pack/v99".into();
        } else {
            manifest["resources"][0]["rawSha256"] = "0".repeat(64).into();
        }
        assert_eq!(
            admit_ordered(ORDERED_CDDL, &serde_json::to_vec(&manifest).expect("JSON"))
                .expect_err("invalid manifest")
                .kind(),
            expected,
        );
    }
}

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

// A structural CDDL specimen, deliberately not a compiler or runtime witness.
fn ordered_envelope(order: Option<Value>) -> Value {
    let mut intent = vec![
        ("operationProfile", text("continuum.profile.read/v1")),
        ("inputConstraints", Value::Array(vec![])),
        (
            "coreEvaluationBudget",
            map([
                ("maxSteps", Value::Integer(1)),
                ("maxAllocatedBytes", Value::Integer(1)),
                ("maxOutputBytes", Value::Integer(1)),
            ]),
        ),
        ("requirements", Value::Array(vec![])),
        ("steps", Value::Array(vec![])),
        (
            "result",
            map([("kind", text("record")), ("fields", map([]))]),
        ),
    ];
    if let Some(order) = order {
        intent.push(("executionOrder", order));
    }
    map([
        ("kind", text("orderedTargetIrArtifact")),
        ("domain", text("echo.span-ir/v2")),
        (
            "targetProfile",
            map([
                ("id", text("echo.dpo@1")),
                (
                    "digest",
                    Value::Array(vec![text("sha256"), Value::Bytes(vec![0; 32])]),
                ),
            ]),
        ),
        ("sourceCoreCoordinate", text("conformance.ordered@1")),
        ("intents", map([("read", map(intent))])),
    ])
}

#[test]
fn ordered_schema_validates_shape_and_requires_the_order_table() {
    let ordered = admit_ordered(ORDERED_CDDL, ORDERED_MANIFEST).expect("ordered admission");
    let valid = encode_canonical_cbor_v1(&ordered_envelope(Some(Value::Array(vec![]))))
        .expect("canonical specimen");
    ordered
        .validate_contract_bytes("target-ir-artifact", &valid)
        .expect("ordered shape");
    let legacy = admit_provider_contract_pack_v1(LEGACY_CDDL, LEGACY_MANIFEST).expect("legacy");
    assert!(legacy
        .validate_contract_bytes("target-ir-artifact", &valid)
        .is_err());
    for order in [None, Some(text("not an order array"))] {
        let malformed = encode_canonical_cbor_v1(&ordered_envelope(order)).expect("canonical");
        let error = ordered
            .validate_contract_bytes("target-ir-artifact", &malformed)
            .expect_err("malformed order cannot pass the selected schema");
        assert_eq!(
            error.kind(),
            ProviderContractValidationErrorKind::SchemaMismatch
        );
    }
}
