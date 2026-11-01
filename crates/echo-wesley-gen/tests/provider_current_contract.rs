// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! The production admission boundary has one current publication.
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;

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
    for (schema, manifest) in [
        (include_bytes!("fixtures/historical-contract-packs/v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("fixtures/historical-contract-packs/v1/manifest.json").as_slice()),
        (include_bytes!("fixtures/historical-contract-packs/ordered/edict-provider-contracts.cddl").as_slice(), include_bytes!("fixtures/historical-contract-packs/ordered/manifest.json").as_slice()),
        (include_bytes!("fixtures/historical-contract-packs/source-functions-v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("fixtures/historical-contract-packs/source-functions-v1/manifest.json").as_slice()),
        (include_bytes!("fixtures/historical-contract-packs/source-modules-v1/edict-provider-contracts.cddl").as_slice(), include_bytes!("fixtures/historical-contract-packs/source-modules-v1/manifest.json").as_slice()),
    ] {
        assert_eq!(admit_provider_contract_pack_v1(schema, manifest).expect_err("historical publication must not be admitted").kind(), Kind::SchemaDigestMismatch);
        for (schema, manifest) in [(schema, current_manifest.as_slice()), (current_schema.as_slice(), manifest)] {
            assert_eq!(admit_provider_contract_pack_v1(schema, manifest).expect_err("crossed pair").kind(), Kind::SchemaBytesMismatch);
        }
    }
}
