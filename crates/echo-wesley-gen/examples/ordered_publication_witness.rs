// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generates a schema-publication candidate for the Docker consumer witness.
//! Supports public compiler witnesses; generation grants no runtime authority.

use anyhow::{bail, Context, Result};
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_for_publication_v1, ProviderContractPublicationV1,
};
use echo_wesley_gen::provider_corpus::checked_provider_generator_source_bundle_v1;
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
use echo_wesley_gen::provider_package::{
    admit_provider_package_v1, assemble_provider_package_v1, ProviderPackageComponentMaterialV1,
};
use echo_wesley_gen::provider_provenance::generate_provider_generation_provenance_v1;
use echo_wesley_gen::provider_review::generate_provider_generation_review_v1;
use std::path::PathBuf;

fn main() -> Result<()> {
    if !std::path::Path::new("/.dockerenv").is_file() {
        bail!("run this consumer witness in its COPY-based Docker image");
    }
    let mut arguments = std::env::args_os().skip(1);
    let output = PathBuf::from(
        arguments
            .next()
            .context("expected a new output directory")?,
    );
    if arguments.next().is_some() || output.exists() {
        bail!("expected exactly one new output directory");
    }
    let contracts = admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::OrderedInstructions,
        include_bytes!(
            "../assets/v1/edict-provider/contracts/ordered/edict-provider-contracts.cddl"
        ),
        include_bytes!("../assets/v1/edict-provider/contracts/ordered/manifest.json"),
    )?;
    let input = build_provider_generation_input_v1(
        include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json"),
        &contracts,
        include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json"),
    )?;
    let primary = generate_provider_primary_artifacts_v1(&input, &contracts)?;
    let generator = checked_provider_generator_source_bundle_v1()?.generator_material()?;
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator)?;
    let review = generate_provider_generation_review_v1(&input, &provenance)?;
    let candidate = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        vec![
            ProviderPackageComponentMaterialV1::new(
                "lowerer.echo-dpo",
                include_bytes!("../assets/v1/edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm"),
            )?,
            ProviderPackageComponentMaterialV1::new(
                "verifier.echo-dpo",
                include_bytes!("../assets/v1/edict-provider/package/v1/components/verifier.echo-dpo.component.wasm"),
            )?,
        ],
    )?;
    let admitted =
        admit_provider_package_v1(candidate.files().to_vec(), candidate.provider_reference())?;
    // Publish only after the complete generated package has passed digest admission.
    std::fs::create_dir(&output)?;
    for file in admitted.files() {
        let path = output.join(file.relative_path());
        std::fs::create_dir_all(path.parent().context("package file parent")?)?;
        std::fs::write(path, file.bytes())?;
    }
    Ok(())
}
