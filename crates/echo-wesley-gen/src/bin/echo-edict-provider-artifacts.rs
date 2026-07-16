// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Generates or checks Echo's deterministic Edict provider artifact corpus.

use anyhow::{bail, Result};
use clap::Parser;
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_corpus::{
    checked_provider_generator_source_bundle_v1, diff_provider_artifact_corpus_v1,
    render_provider_artifact_corpus_v1,
};
use echo_wesley_gen::provider_corpus_fs::{read_actual_corpus, write_corpus};
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
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

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Generates or checks Echo's deterministic Edict provider artifact corpus"
)]
struct Args {
    /// Output directory for the checked provider artifact corpus.
    #[arg(long, default_value = "schemas/edict-provider/generated/v1")]
    out: PathBuf,

    /// Report corpus drift without creating, deleting, or rewriting files.
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
    let corpus =
        render_provider_artifact_corpus_v1(&input, &primary, &generator, &provenance, &review)?;

    if args.check {
        let actual = read_actual_corpus(&args.out, corpus.files())?;
        let drift = diff_provider_artifact_corpus_v1(&corpus, &actual)?;
        if drift.is_empty() {
            println!(
                "Provider artifact corpus is current in {}",
                args.out.display()
            );
            return Ok(());
        }

        eprintln!("Provider artifact corpus drift:");
        for entry in &drift {
            eprintln!("  {}: {}", entry.kind().as_str(), entry.relative_path());
        }
        bail!("provider artifact corpus is not current");
    }

    write_corpus(&args.out, corpus.files())?;
    println!(
        "Provider artifact corpus generated in {}",
        args.out.display()
    );
    Ok(())
}
