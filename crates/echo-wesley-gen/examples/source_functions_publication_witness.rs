// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generates an explicitly selected source-function publication candidate.
//! Supports public compiler witnesses; generation grants no runtime authority.

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_corpus::checked_provider_generator_source_bundle_v1;
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
use echo_wesley_gen::provider_package::{
    admit_provider_package_v1, assemble_provider_package_v1, ProviderPackageComponentMaterialV1,
};
use echo_wesley_gen::provider_provenance::generate_provider_generation_provenance_v1;
use echo_wesley_gen::provider_review::generate_provider_generation_review_v1;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Arguments {
    /// New directory for the admitted candidate package.
    output: PathBuf,
    /// Explicit candidate lowerer; requires an explicit verifier too.
    #[arg(long, requires = "verifier_component")]
    lowerer_component: Option<PathBuf>,
    /// Explicit candidate verifier; requires an explicit lowerer too.
    #[arg(long, requires = "lowerer_component")]
    verifier_component: Option<PathBuf>,
}

fn component_bytes(candidate: Option<PathBuf>, checked: &[u8]) -> Result<Vec<u8>> {
    match candidate {
        Some(path) => {
            // Match provider-package admission's component ceiling, including
            // files that grow after inspection. Do not allocate an unbounded input.
            const MAX_BYTES: u64 = 16 * 1024 * 1024;
            let metadata = path.symlink_metadata()?;
            ensure!(
                metadata.is_file(),
                "component candidate must be a regular file"
            );
            ensure!(
                metadata.len() <= MAX_BYTES,
                "component candidate exceeds 16 MiB"
            );
            let mut bytes = Vec::new();
            std::fs::File::open(&path)
                .with_context(|| format!("read explicit component candidate {}", path.display()))?
                .take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() as u64 <= MAX_BYTES,
                "component candidate exceeds 16 MiB"
            );
            Ok(bytes)
        }
        None => Ok(checked.to_vec()),
    }
}

fn main() -> Result<()> {
    if !std::path::Path::new("/.dockerenv").is_file() {
        bail!("run this consumer witness in a guarded Docker worker with copied sources");
    }
    let arguments = Arguments::parse();
    let output = arguments.output;
    if output.exists() {
        bail!("expected a new output directory");
    }
    let contracts = admit_provider_contract_pack_v1(
        include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl"),
        include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json"),
    )?;
    let source = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
    let input = build_provider_generation_input_v1(
        source,
        &contracts,
        include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json"),
    )?;
    let primary = generate_provider_primary_artifacts_v1(&input, &contracts)?;
    let generator = checked_provider_generator_source_bundle_v1()?.generator_material()?;
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator)?;
    let review = generate_provider_generation_review_v1(&input, &provenance)?;
    let lowerer = component_bytes(
        arguments.lowerer_component,
        include_bytes!(
            "../assets/v1/edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm"
        ),
    )?;
    let verifier = component_bytes(
        arguments.verifier_component,
        include_bytes!(
            "../assets/v1/edict-provider/package/v1/components/verifier.echo-dpo.component.wasm"
        ),
    )?;
    let candidate = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        vec![
            ProviderPackageComponentMaterialV1::new("lowerer.echo-dpo", &lowerer)?,
            ProviderPackageComponentMaterialV1::new("verifier.echo-dpo", &verifier)?,
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
