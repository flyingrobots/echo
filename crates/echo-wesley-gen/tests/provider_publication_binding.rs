// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! A generation input binds the sole admitted current schema.
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
#[test]
fn independently_admitted_current_pack_generates_the_bound_schema() {
    let schema =
        include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
    let manifest = include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
    let pack = admit_provider_contract_pack_v1(schema, manifest).expect("current pack");
    let independent =
        admit_provider_contract_pack_v1(schema, manifest).expect("independent admission");
    let input = build_provider_generation_input_v1(
        include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json"),
        &pack,
        include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json"),
    )
    .expect("current generation input");
    let primary =
        generate_provider_primary_artifacts_v1(&input, &independent).expect("bound schema");
    assert!(primary.schema().bytes().starts_with(schema));
    assert_eq!(primary.generation_input_digest(), input.digest());
}
