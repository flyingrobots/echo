// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Explicit provider generation variant for the distinct source artifact.
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, AdmittedProviderContractPackV1,
};
use echo_wesley_gen::provider_corpus::checked_provider_generator_source_bundle_v1;
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
use echo_wesley_gen::provider_package::{
    admit_provider_package_v1, assemble_provider_package_v1, ProviderPackageComponentMaterialV1,
};
use echo_wesley_gen::provider_provenance::generate_provider_generation_provenance_v1;
use echo_wesley_gen::provider_review::generate_provider_generation_review_v1;
const SOURCE: &[u8] = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] = include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json");
fn pack() -> AdmittedProviderContractPackV1 {
    admit_provider_contract_pack_v1(
        include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl"),
        include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json"),
    )
    .expect("explicit module contract pack")
}
#[test]
fn source_module_generation_and_package_admission_bind_the_new_source_domain() {
    let pack = pack();
    let input =
        build_provider_generation_input_v1(SOURCE, &pack, SETTINGS).expect("module semantic input");
    let primary =
        generate_provider_primary_artifacts_v1(&input, &pack).expect("module-aware schema");
    let generator = checked_provider_generator_source_bundle_v1()
        .expect("generator sources")
        .generator_material()
        .expect("generator identity");
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator)
        .expect("provenance");
    let review = generate_provider_generation_review_v1(&input, &provenance).expect("review");
    let package = assemble_provider_package_v1(&input,&primary,&generator,&provenance,&review,vec![
        ProviderPackageComponentMaterialV1::new("lowerer.echo-dpo",include_bytes!("../assets/v1/edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm")).expect("lowerer bytes"),
        ProviderPackageComponentMaterialV1::new("verifier.echo-dpo",include_bytes!("../assets/v1/edict-provider/package/v1/components/verifier.echo-dpo.component.wasm")).expect("verifier bytes"),
    ]).expect("explicit module package");
    let admitted =
        admit_provider_package_v1(package.files().to_vec(), package.provider_reference())
            .expect("exact pinned raw package admission");
    assert_eq!(admitted.manifest().schema_bindings.len(), 32);
    let binding = admitted
        .manifest()
        .schema_bindings
        .iter()
        .find(|binding| binding.domain == "edict.source-closure/v1")
        .expect("new distinct domain");
    assert_eq!(binding.root_rule, "source-closure");
    assert!(admitted
        .manifest()
        .schema_bindings
        .iter()
        .any(|binding| binding.domain == "edict.source/v1"
            && binding.root_rule == "edict-source-bytes"));
}
