// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Integration tests for explicit admission of the Edict provider contract pack.

use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, ProviderContractPackErrorKind,
};
use serde_json::Value;

const CONTRACT_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
const DOMAIN_ROOTS: [(&str, &str); 6] = [
    ("edict.authority-facts/v1", "authority-facts"),
    ("edict.core.module/v1", "core-module"),
    ("edict.lawpack/v1", "lawpack-manifest"),
    ("edict.lowering-requirements/v1", "lowering-requirements"),
    ("edict.target-ir.artifact/v1", "target-ir-artifact"),
    ("edict.target-profile/v1", "target-profile-manifest"),
];

fn manifest_value() -> Value {
    serde_json::from_slice(CONTRACT_MANIFEST).expect("checked contract manifest is JSON")
}

fn mutated_manifest(mutate: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut manifest = manifest_value();
    mutate(&mut manifest);
    serde_json::to_vec(&manifest).expect("mutated contract manifest serializes")
}

fn assert_manifest_failure(manifest: &[u8], expected: ProviderContractPackErrorKind) {
    let error = admit_provider_contract_pack_v1(CONTRACT_CDDL, manifest)
        .expect_err("tampered contract manifest must fail admission");
    assert_eq!(error.kind(), expected);
}

#[test]
fn checked_edict_provider_contract_pack_is_admitted() {
    let pack = admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted");

    assert_eq!(pack.api_version(), "edict.provider-contract-pack/v1");
    assert_eq!(pack.coordinate(), "edict.provider-contract-pack.cddl@1");
    assert_eq!(pack.license(), "Apache-2.0");
    assert_eq!(
        pack.schema_sha256(),
        "92697bc9a5262c68258be9ee451ee8c144aeb363b92142915b8224430b85cf74"
    );
    assert_eq!(pack.contract_count(), 9);
    assert_eq!(pack.domain_count(), 6);
    assert_eq!(pack.resource_count(), 5);
    assert_eq!(pack.schema_bytes(), CONTRACT_CDDL);
    assert_eq!(pack.manifest_bytes(), CONTRACT_MANIFEST);
    for (domain, root) in DOMAIN_ROOTS {
        assert_eq!(pack.root_for_domain(domain), Some(root));
    }
    let canonical_cbor = pack
        .resource("edict.canonical-cbor/v1")
        .expect("canonical CBOR contract resource is admitted");
    assert_eq!(
        canonical_cbor.raw_sha256(),
        "8306e4f08c1e4e7d29ab22bcf55c324312712aac3eeeb675857ced57c3e48bdc"
    );
    assert_eq!(
        canonical_cbor.repository(),
        "https://github.com/flyingrobots/edict"
    );
}

#[test]
fn tampered_contract_cddl_has_stable_failure_kind() {
    let mut cddl = CONTRACT_CDDL.to_vec();
    cddl[0] ^= 1;

    let error = admit_provider_contract_pack_v1(&cddl, CONTRACT_MANIFEST)
        .expect_err("tampered contract CDDL must fail admission");
    assert_eq!(
        error.kind(),
        ProviderContractPackErrorKind::SchemaBytesMismatch
    );
    assert_eq!(
        error.to_string(),
        concat!(
            "provider contract pack schema-bytes-mismatch: schema.bytesHex -> ",
            "92697bc9a5262c68258be9ee451ee8c144aeb363b92142915b8224430b85cf74"
        )
    );
}

#[test]
fn malformed_contract_manifest_has_stable_failure_kind() {
    assert_manifest_failure(b"{", ProviderContractPackErrorKind::ManifestMalformed);
}

#[test]
fn oversized_contract_manifest_fails_before_json_admission() {
    let mut oversized = CONTRACT_MANIFEST.to_vec();
    oversized.extend(std::iter::repeat_n(b' ', CONTRACT_MANIFEST.len()));

    assert_manifest_failure(
        &oversized,
        ProviderContractPackErrorKind::ManifestSizeExceeded,
    );
}

#[test]
fn unknown_contract_manifest_field_is_rejected() {
    let manifest = mutated_manifest(|manifest| {
        manifest
            .as_object_mut()
            .expect("manifest root is an object")
            .insert("ambientRegistry".to_owned(), Value::Bool(true));
    });

    assert_manifest_failure(&manifest, ProviderContractPackErrorKind::ManifestMalformed);
}

#[test]
fn unsupported_contract_pack_api_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["apiVersion"] = Value::String("edict.provider-contract-pack/v2".to_owned());
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::UnsupportedApiVersion,
    );
}

#[test]
fn contract_pack_coordinate_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["coordinate"] = Value::String("edict.provider-contract-pack.cddl@2".to_owned());
    });

    assert_manifest_failure(&manifest, ProviderContractPackErrorKind::CoordinateMismatch);
}

#[test]
fn contract_pack_license_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["license"] = Value::String("substituted".to_owned());
    });

    assert_manifest_failure(&manifest, ProviderContractPackErrorKind::LicenseMismatch);
}

#[test]
fn noncanonical_contract_schema_hex_is_rejected() {
    let manifest = mutated_manifest(|manifest| {
        let bytes = manifest["schema"]["bytesHex"]
            .as_str()
            .expect("schema bytes are lowercase hex");
        manifest["schema"]["bytesHex"] = Value::String(bytes.replacen('a', "A", 1));
    });

    assert_manifest_failure(&manifest, ProviderContractPackErrorKind::SchemaHexInvalid);
}

#[test]
fn contract_schema_digest_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["schema"]["rawSha256"] = Value::String("0".repeat(64));
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::SchemaDigestMismatch,
    );
}

#[test]
fn contract_inventory_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["contracts"][0]["contract"] = Value::String("substitute-contract".to_owned());
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::ContractInventoryMismatch,
    );
}

#[test]
fn domain_inventory_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["domains"]
            .as_array_mut()
            .expect("domain inventory is an array")
            .swap(0, 1);
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::DomainInventoryMismatch,
    );
}

#[test]
fn resource_order_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["resources"]
            .as_array_mut()
            .expect("resource inventory is an array")
            .swap(0, 1);
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::ResourceInventoryMismatch,
    );
}

#[test]
fn resource_bytes_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        let bytes = manifest["resources"][0]["canonicalBytesHex"]
            .as_str()
            .expect("resource bytes are lowercase hex");
        let mut tampered = bytes.to_owned();
        let replacement = if tampered.ends_with('0') { '1' } else { '0' };
        tampered.pop();
        tampered.push(replacement);
        manifest["resources"][0]["canonicalBytesHex"] = Value::String(tampered);
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::ResourceRawDigestMismatch,
    );
}

#[test]
fn noncanonical_contract_resource_hex_is_rejected() {
    let manifest = mutated_manifest(|manifest| {
        let bytes = manifest["resources"][0]["canonicalBytesHex"]
            .as_str()
            .expect("resource bytes are lowercase hex");
        manifest["resources"][0]["canonicalBytesHex"] = Value::String(bytes.replacen('a', "A", 1));
    });

    assert_manifest_failure(&manifest, ProviderContractPackErrorKind::ResourceHexInvalid);
}

#[test]
fn resource_domain_digest_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["resources"][0]["domainFramedDigest"] =
            Value::String(format!("sha256:{}", "0".repeat(64)));
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::ResourceDomainDigestMismatch,
    );
}

#[test]
fn resource_provenance_tampering_has_stable_failure_kind() {
    let manifest = mutated_manifest(|manifest| {
        manifest["resources"][0]["provenance"]["sourcePath"] =
            Value::String("fixtures/substituted-resource.cbor".to_owned());
    });

    assert_manifest_failure(
        &manifest,
        ProviderContractPackErrorKind::ResourceProvenanceMismatch,
    );
}

#[test]
fn semantically_equivalent_manifest_bytes_do_not_replace_the_publication() {
    let reformatted = serde_json::to_vec(&manifest_value()).expect("checked manifest reformats");

    assert_manifest_failure(
        &reformatted,
        ProviderContractPackErrorKind::ManifestDigestMismatch,
    );
}
