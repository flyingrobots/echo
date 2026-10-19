// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generates an explicitly selected source-function publication candidate.
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
        bail!("run this consumer witness in a guarded Docker worker with copied sources");
    }
    let mut arguments = std::env::args_os().skip(1);
    let output = PathBuf::from(
        arguments
            .next()
            .context("expected a new output directory")?,
    );
    let source_modules = match arguments.next() {
        None => false,
        Some(option) if option == "--source-modules" => true,
        Some(_) => bail!("expected a new output directory and optional --source-modules"),
    };
    if arguments.next().is_some() || output.exists() {
        bail!("expected a new output directory and optional --source-modules");
    }
    let (publication, schema, manifest, source): (_, &[u8], &[u8], &[u8]) = if source_modules {
        (
            ProviderContractPublicationV1::SourceModules,
            include_bytes!("../assets/v1/edict-provider/contracts/source-modules-v1/edict-provider-contracts.cddl"),
            include_bytes!("../assets/v1/edict-provider/contracts/source-modules-v1/manifest.json"),
            include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-source-modules-v1.json"),
        )
    } else {
        (
            ProviderContractPublicationV1::SourceFunctions,
            include_bytes!("../assets/v1/edict-provider/contracts/source-functions-v1/edict-provider-contracts.cddl"),
            include_bytes!("../assets/v1/edict-provider/contracts/source-functions-v1/manifest.json"),
            include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json"),
        )
    };
    let contracts = admit_provider_contract_pack_for_publication_v1(publication, schema, manifest)?;
    let input = build_provider_generation_input_v1(
        source,
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
