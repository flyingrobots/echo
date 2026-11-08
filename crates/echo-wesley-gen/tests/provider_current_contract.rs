// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! The production admission boundary has one current publication.
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use sha2::{Digest as _, Sha256};

#[test]
fn current_admission_accepts_the_upstream_fold_publication() {
    let schema =
        include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
    let manifest = include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
    let pack = admit_provider_contract_pack_v1(schema, manifest)
        .expect("the sole production entry point must admit the current upstream publication");
    assert_eq!(pack.schema_bytes(), schema);
    assert_eq!(pack.contract_count(), 12);
    assert_eq!(pack.domain_count(), 8);
}

#[test]
fn current_admission_refuses_all_historical_and_crossed_pairs() {
    use echo_wesley_gen::provider_contract_pack::ProviderContractPackErrorKind as Kind;
    let current_schema =
        include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
    let current_manifest = include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
    for (schema, manifest, schema_hash, manifest_hash) in [
        (
            include_bytes!("fixtures/historical-contract-packs/v1/edict-provider-contracts.cddl").as_slice(),
            include_bytes!("fixtures/historical-contract-packs/v1/manifest.json").as_slice(),
            "8d77c84ecbd7d7e38496479b2f8346c15504a56f076170d79c0eca2a57fdb3a5",
            "bc222c4a669b888a60df6f398412bae232967c11841b9a27c7ba6c18eb41e362",
        ),
        (
            include_bytes!("fixtures/historical-contract-packs/ordered/edict-provider-contracts.cddl").as_slice(),
            include_bytes!("fixtures/historical-contract-packs/ordered/manifest.json").as_slice(),
            "82273f3ea016a421c881f15b0fd451802205903ac9177bac8accbf3173f66d2c",
            "6303668861667a30418870ef25e5f169017905ae1f9d261451ba298120afdd9d",
        ),
        (
            include_bytes!("fixtures/historical-contract-packs/source-functions-v1/edict-provider-contracts.cddl").as_slice(),
            include_bytes!("fixtures/historical-contract-packs/source-functions-v1/manifest.json").as_slice(),
            "e484eabd615584a38cb57454b52747786f2314c135c13f99da6f6bae619a709f",
            "aebc2e4133407ae433c18b55421a89bf15dc5d489368f82ce4874f948d4f9c79",
        ),
        (
            include_bytes!("fixtures/historical-contract-packs/source-modules-v1/edict-provider-contracts.cddl").as_slice(),
            include_bytes!("fixtures/historical-contract-packs/source-modules-v1/manifest.json").as_slice(),
            "8f8db4052e1bb85c4f728a0a453c4eb20904bcf2ce81530ffdbfce9351e209bc",
            "29927f6db5e0fe1e5e7475bef3fb2c6e7f0f84ea56304fd97bd1299f1c2bb4be",
        ),
    ] {
        assert_eq!(hex::encode(Sha256::digest(schema)), schema_hash, "historical schema provenance");
        assert_eq!(hex::encode(Sha256::digest(manifest)), manifest_hash, "historical manifest provenance");
        assert_eq!(admit_provider_contract_pack_v1(schema, manifest).expect_err("historical publication must not be admitted").kind(), Kind::SchemaDigestMismatch);
        for (schema, manifest) in [(schema, current_manifest.as_slice()), (current_schema.as_slice(), manifest)] {
            assert_eq!(admit_provider_contract_pack_v1(schema, manifest).expect_err("crossed pair").kind(), Kind::SchemaBytesMismatch);
        }
    }
}
