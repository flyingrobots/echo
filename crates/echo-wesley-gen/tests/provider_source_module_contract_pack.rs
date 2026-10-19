// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Exact source-module schema publication selection, separate from execution.
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_for_publication_v1, ProviderContractPublicationV1,
};
use sha2::{Digest, Sha256};
const SCHEMA: &[u8] = include_bytes!(
    "../assets/v1/edict-provider/contracts/source-modules-v1/edict-provider-contracts.cddl"
);
const MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/source-modules-v1/manifest.json");

#[test]
fn source_module_pair_has_one_explicit_admission_route() {
    assert_eq!(
        hex::encode(Sha256::digest(SCHEMA)),
        "8f8db4052e1bb85c4f728a0a453c4eb20904bcf2ce81530ffdbfce9351e209bc"
    );
    assert_eq!(
        hex::encode(Sha256::digest(MANIFEST)),
        "29927f6db5e0fe1e5e7475bef3fb2c6e7f0f84ea56304fd97bd1299f1c2bb4be"
    );
    let publications = [
        ProviderContractPublicationV1::PureBindings,
        ProviderContractPublicationV1::OrderedInstructions,
        ProviderContractPublicationV1::SourceFunctions,
        ProviderContractPublicationV1::SourceModules,
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

fn selected() -> echo_wesley_gen::provider_contract_pack::AdmittedProviderContractPackV1 {
    admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::SourceModules,
        SCHEMA,
        MANIFEST,
    )
    .expect("exact module publication")
}

#[test]
fn source_module_selection_refuses_older_crossed_and_tampered_publications() {
    use echo_wesley_gen::provider_contract_pack::ProviderContractPackErrorKind as Kind;
    for (publication, schema, manifest) in [
        (ProviderContractPublicationV1::PureBindings, include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json").as_slice()),
        (ProviderContractPublicationV1::OrderedInstructions, include_bytes!("../assets/v1/edict-provider/contracts/ordered/edict-provider-contracts.cddl").as_slice(), include_bytes!("../assets/v1/edict-provider/contracts/ordered/manifest.json").as_slice()),
        (ProviderContractPublicationV1::SourceFunctions, include_bytes!("../assets/v1/edict-provider/contracts/source-functions-v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("../assets/v1/edict-provider/contracts/source-functions-v1/manifest.json").as_slice()),
    ] {
        assert_eq!(admit_provider_contract_pack_for_publication_v1(publication, SCHEMA, MANIFEST).expect_err("old selector must not upgrade").kind(), Kind::ManifestSizeExceeded);
        assert_eq!(admit_provider_contract_pack_for_publication_v1(ProviderContractPublicationV1::SourceModules, schema, manifest).expect_err("new selector must not downgrade").kind(), Kind::SchemaDigestMismatch);
        for (schema, manifest) in [(schema, MANIFEST), (SCHEMA, manifest)] {
            assert_eq!(admit_provider_contract_pack_for_publication_v1(ProviderContractPublicationV1::SourceModules, schema, manifest).expect_err("crossed pair").kind(), Kind::SchemaBytesMismatch);
        }
    }
    let mut schema = SCHEMA.to_vec();
    schema[0] ^= 1;
    assert_eq!(
        admit_provider_contract_pack_for_publication_v1(
            ProviderContractPublicationV1::SourceModules,
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
            ProviderContractPublicationV1::SourceModules,
            SCHEMA,
            &manifest
        )
        .expect_err("manifest byte substitution")
        .kind(),
        Kind::ManifestDigestMismatch
    );
    assert_eq!(
        admit_provider_contract_pack_for_publication_v1(
            ProviderContractPublicationV1::SourceModules,
            SCHEMA,
            &vec![b' '; MANIFEST.len() + 1]
        )
        .expect_err("oversized manifest")
        .kind(),
        Kind::ManifestSizeExceeded
    );
}

fn closure_value(
    count: usize,
    algorithm: &str,
    digest_bytes: usize,
) -> echo_wesley_gen::provider_canonical::CanonicalValueV1 {
    use echo_wesley_gen::provider_canonical::CanonicalValueV1 as Value;
    Value::Map(vec![
        (
            Value::Text("apiVersion".into()),
            Value::Text("edict.source-closure/v1".into()),
        ),
        (
            Value::Text("root".into()),
            Value::Text("library.m0@1".into()),
        ),
        (
            Value::Text("modules".into()),
            Value::Map(
                (0..count)
                    .map(|i| {
                        (
                            Value::Text(format!("library.m{i}@1")),
                            Value::Map(vec![
                                (
                                    Value::Text("source".into()),
                                    Value::Bytes(format!("package library.m{i}@1;").into_bytes()),
                                ),
                                (
                                    Value::Text("digest".into()),
                                    Value::Array(vec![
                                        Value::Text(algorithm.into()),
                                        Value::Bytes(vec![0; digest_bytes]),
                                    ]),
                                ),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}

#[test]
fn source_module_schema_enforces_closed_envelope_module_counts_and_typed_digests() {
    use echo_wesley_gen::provider_canonical::{
        encode_canonical_cbor_v1, CanonicalValueV1 as Value,
    };
    use echo_wesley_gen::provider_contract_pack::ProviderContractValidationErrorKind;
    let pack = selected();
    assert_eq!(
        pack.root_for_domain("edict.source-closure/v1"),
        Some("source-closure")
    );
    // Structural admission cannot authenticate these placeholder digest values.
    for count in [1, 64] {
        let value = closure_value(count, "sha256", 32);
        let bytes = encode_canonical_cbor_v1(&value).expect("canonical envelope");
        pack.validate_contract_bytes("source-closure", &bytes)
            .expect("bounded structural closure");
    }
    let mut extra_field = closure_value(1, "sha256", 32);
    let Value::Map(ref mut fields) = extra_field else {
        unreachable!()
    };
    fields.push((Value::Text("ignored".into()), Value::Bool(true)));
    for value in [
        Value::Bytes(b"package library.m0@1;".to_vec()),
        closure_value(0, "sha256", 32),
        closure_value(65, "sha256", 32),
        closure_value(1, "sha256", 31),
        closure_value(1, "sha256", 33),
        closure_value(1, "blake3", 32),
        extra_field,
    ] {
        let bytes = encode_canonical_cbor_v1(&value).expect("canonical invalid closure");
        assert_eq!(
            pack.validate_contract_bytes("source-closure", &bytes)
                .expect_err("must reject schema violation")
                .kind(),
            ProviderContractValidationErrorKind::SchemaMismatch
        );
    }
}
