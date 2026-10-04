// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Generation must use the exact contract publication bound into its input.

use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderArtifactGenerationErrorKind,
};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_for_publication_v1, AdmittedProviderContractPackV1,
    ProviderContractPublicationV1,
};
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;

const SOURCE: &[u8] = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] = include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json");

fn pack(publication: ProviderContractPublicationV1) -> AdmittedProviderContractPackV1 {
    let (schema, manifest): (&[u8], &[u8]) = match publication {
        ProviderContractPublicationV1::PureBindings => (
            include_bytes!(
                "../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl"
            ),
            include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json"),
        ),
        ProviderContractPublicationV1::OrderedInstructions => (
            include_bytes!(
                "../assets/v1/edict-provider/contracts/ordered/edict-provider-contracts.cddl"
            ),
            include_bytes!("../assets/v1/edict-provider/contracts/ordered/manifest.json"),
        ),
    };
    admit_provider_contract_pack_for_publication_v1(publication, schema, manifest)
        .expect("exact publication admits")
}

fn rejects_crossed_publication(
    input_publication: ProviderContractPublicationV1,
    generation_publication: ProviderContractPublicationV1,
) {
    let bound = pack(input_publication);
    let other = pack(generation_publication);
    let input = build_provider_generation_input_v1(SOURCE, &bound, SETTINGS)
        .expect("valid generation input");
    let error = match generate_provider_primary_artifacts_v1(&input, &other) {
        Ok(_) => panic!("generation accepted a publication absent from the input provenance"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind(),
        ProviderArtifactGenerationErrorKind::ContractPackInputMismatch
    );
    assert_eq!(error.subject(), bound.coordinate());
}

#[test]
fn legacy_input_rejects_ordered_generation_pack() {
    rejects_crossed_publication(
        ProviderContractPublicationV1::PureBindings,
        ProviderContractPublicationV1::OrderedInstructions,
    );
}

#[test]
fn ordered_input_rejects_legacy_generation_pack() {
    rejects_crossed_publication(
        ProviderContractPublicationV1::OrderedInstructions,
        ProviderContractPublicationV1::PureBindings,
    );
}

#[test]
fn each_matching_publication_generates_its_bound_schema() {
    for publication in [
        ProviderContractPublicationV1::PureBindings,
        ProviderContractPublicationV1::OrderedInstructions,
    ] {
        let bound = pack(publication);
        let input = build_provider_generation_input_v1(SOURCE, &bound, SETTINGS)
            .expect("valid generation input");
        let primary = generate_provider_primary_artifacts_v1(&input, &pack(publication))
            .expect("independently admitted matching publication generates");
        assert!(primary.schema().bytes().starts_with(bound.schema_bytes()));
        assert_eq!(primary.generation_input_digest(), input.digest());
    }
}
