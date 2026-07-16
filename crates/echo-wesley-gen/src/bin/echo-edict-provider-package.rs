// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Generates or checks Echo's deterministic digest-locked Edict provider package.

use anyhow::{bail, Result};
use clap::Parser;
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_corpus::{
    checked_provider_artifact_corpus_v1, checked_provider_generator_source_bundle_v1,
    diff_exact_corpus_files_v1, ProviderArtifactCorpusFileV1,
};
use echo_wesley_gen::provider_corpus_fs::{read_actual_corpus, write_corpus};
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
use echo_wesley_gen::provider_package::{
    assemble_provider_package_v1, ProviderPackageComponentMaterialV1, ProviderPackageV1,
};
use echo_wesley_gen::provider_provenance::generate_provider_generation_provenance_v1;
use echo_wesley_gen::provider_review::generate_provider_generation_review_v1;
use std::path::PathBuf;

const SOURCE: &[u8] =
    include_bytes!("../../assets/v1/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] =
    include_bytes!("../../assets/v1/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../../assets/v1/edict-provider/contracts/v1/manifest.json");
const LOWERER: &[u8] = include_bytes!(
    "../../assets/v1/edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm"
);
const VERIFIER: &[u8] = include_bytes!(
    "../../assets/v1/edict-provider/package/v1/components/verifier.echo-dpo.component.wasm"
);

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Generates or checks Echo's deterministic digest-locked Edict provider package"
)]
struct Args {
    /// Output directory for the checked provider package.
    #[arg(long, default_value = "schemas/edict-provider/package/v1")]
    out: PathBuf,

    /// Report package drift without creating, deleting, or rewriting files.
    #[arg(long)]
    check: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let contract_pack = admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)?;
    let input = build_provider_generation_input_v1(SOURCE, &contract_pack, SETTINGS)?;
    let primary = generate_provider_primary_artifacts_v1(&input, &contract_pack)?;
    let generator = checked_provider_generator_source_bundle_v1()?.generator_material()?;
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator)?;
    let review = generate_provider_generation_review_v1(&input, &provenance)?;
    let package = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        vec![
            ProviderPackageComponentMaterialV1::new("lowerer.echo-dpo", LOWERER)?,
            ProviderPackageComponentMaterialV1::new("verifier.echo-dpo", VERIFIER)?,
        ],
    )?;

    // The package crossing consumes the checked #652 publication. It must not
    // silently publish a freshly rendered but unchecked generation occurrence.
    verify_checked_generated_members(&package)?;
    let files = corpus_files(&package)?;

    if args.check {
        let actual = read_actual_corpus(&args.out, &files)?;
        let drift = diff_exact_corpus_files_v1(&files, &actual)?;
        if drift.is_empty() {
            println!("Provider package is current in {}", args.out.display());
            return Ok(());
        }

        eprintln!("Provider package drift:");
        for entry in &drift {
            eprintln!("  {}: {}", entry.kind().as_str(), entry.relative_path());
        }
        bail!("provider package is not current");
    }

    write_corpus(&args.out, &files)?;
    println!("Provider package generated in {}", args.out.display());
    Ok(())
}

fn verify_checked_generated_members(package: &ProviderPackageV1) -> Result<()> {
    let checked = checked_provider_artifact_corpus_v1()?;
    let packaged = package
        .files()
        .iter()
        .filter(|file| file.relative_path().starts_with("generated/"))
        .collect::<Vec<_>>();

    if packaged.len() != checked.files().len() {
        bail!(
            "provider package generated-member count {} does not match checked corpus count {}",
            packaged.len(),
            checked.files().len()
        );
    }

    for (packaged_file, checked_file) in packaged.iter().zip(checked.files()) {
        let expected_path = format!("generated/{}", checked_file.relative_path());
        if packaged_file.relative_path() != expected_path {
            bail!(
                "provider package generated member {} does not match checked path {}",
                packaged_file.relative_path(),
                expected_path
            );
        }
        if packaged_file.bytes() != checked_file.bytes() {
            bail!(
                "provider package generated member {} does not match the checked exact bytes",
                packaged_file.relative_path()
            );
        }
    }

    Ok(())
}

fn corpus_files(package: &ProviderPackageV1) -> Result<Vec<ProviderArtifactCorpusFileV1>> {
    let mut files = Vec::with_capacity(package.files().len());
    for file in package.files() {
        files.push(ProviderArtifactCorpusFileV1::new(
            file.relative_path(),
            file.bytes(),
        )?);
    }
    Ok(files)
}
