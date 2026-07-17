// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Synchronizes exact package-local assets for the Echo Edict provider tools.

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use echo_wesley_gen::provider_corpus::{diff_exact_corpus_files_v1, ProviderArtifactCorpusFileV1};
use echo_wesley_gen::provider_corpus_fs::{read_actual_corpus, write_corpus};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_OWNER_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_OWNER_TOTAL_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
struct AssetSpec {
    carrier: &'static str,
    owner: &'static str,
    corroborating_owner: Option<&'static str>,
}

impl AssetSpec {
    const fn new(carrier: &'static str, owner: &'static str) -> Self {
        Self {
            carrier,
            owner,
            corroborating_owner: None,
        }
    }

    const fn corroborated(
        carrier: &'static str,
        package_copy: &'static str,
        authoritative_owner: &'static str,
    ) -> Self {
        Self {
            carrier,
            owner: authoritative_owner,
            corroborating_owner: Some(package_copy),
        }
    }
}

const ASSETS: [AssetSpec; 37] = [
    AssetSpec::new(
        "edict-provider/contracts/v1/edict-provider-contracts.cddl",
        "schemas/edict-provider/contracts/v1/edict-provider-contracts.cddl",
    ),
    AssetSpec::new(
        "edict-provider/contracts/v1/manifest.json",
        "schemas/edict-provider/contracts/v1/manifest.json",
    ),
    AssetSpec::new(
        "edict-provider/echo-provider-semantics-v1.json",
        "schemas/edict-provider/echo-provider-semantics-v1.json",
    ),
    AssetSpec::new(
        "edict-provider/generation-settings-v1.json",
        "schemas/edict-provider/generation-settings-v1.json",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm",
        "schemas/edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm",
        "schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/components/verifier.echo-dpo.component.wasm",
        "schemas/edict-provider/package/v1/components/verifier.echo-dpo.component.wasm",
        "schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/evidence/provenance.provider-generation.json",
        "schemas/edict-provider/package/v1/generated/evidence/provenance.provider-generation.json",
        "schemas/edict-provider/generated/v1/evidence/provenance.provider-generation.json",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/evidence/review.provider-generation.json",
        "schemas/edict-provider/package/v1/generated/evidence/review.provider-generation.json",
        "schemas/edict-provider/generated/v1/evidence/review.provider-generation.json",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/primary/authority-facts.echo-dpo.cbor",
        "schemas/edict-provider/package/v1/generated/primary/authority-facts.echo-dpo.cbor",
        "schemas/edict-provider/generated/v1/primary/authority-facts.echo-dpo.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/primary/authority-facts.echo-lawpack.cbor",
        "schemas/edict-provider/package/v1/generated/primary/authority-facts.echo-lawpack.cbor",
        "schemas/edict-provider/generated/v1/primary/authority-facts.echo-lawpack.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/primary/generated-artifact-profile.echo-dpo-registration.cbor",
        "schemas/edict-provider/package/v1/generated/primary/generated-artifact-profile.echo-dpo-registration.cbor",
        "schemas/edict-provider/generated/v1/primary/generated-artifact-profile.echo-dpo-registration.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/primary/lawpack.echo-dpo.cbor",
        "schemas/edict-provider/package/v1/generated/primary/lawpack.echo-dpo.cbor",
        "schemas/edict-provider/generated/v1/primary/lawpack.echo-dpo.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/primary/schema.echo-provider-artifacts.cddl",
        "schemas/edict-provider/package/v1/generated/primary/schema.echo-provider-artifacts.cddl",
        "schemas/edict-provider/generated/v1/primary/schema.echo-provider-artifacts.cddl",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/primary/target-profile.echo-dpo.cbor",
        "schemas/edict-provider/package/v1/generated/primary/target-profile.echo-dpo.cbor",
        "schemas/edict-provider/generated/v1/primary/target-profile.echo-dpo.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.conformance-corpus.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.conformance-corpus.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.conformance-corpus.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.lawpack-compatibility.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.lawpack-compatibility.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.lawpack-compatibility.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.lawpack-exports.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.lawpack-exports.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.lawpack-exports.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.lawpack-target-adapter.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.lawpack-target-adapter.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.lawpack-target-adapter.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.lawpack-verifier.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.lawpack-verifier.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.lawpack-verifier.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-bundle-profile.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-bundle-profile.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-bundle-profile.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-cost-algebra.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-cost-algebra.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-cost-algebra.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-footprint-algebra.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-footprint-algebra.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-footprint-algebra.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-intrinsics.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-intrinsics.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-intrinsics.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-ir.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-ir.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-ir.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-lowerer-contract.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-lowerer-contract.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-lowerer-contract.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-obstruction-taxonomy.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-obstruction-taxonomy.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-obstruction-taxonomy.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-operation-profiles.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-operation-profiles.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-operation-profiles.cbor",
    ),
    AssetSpec::corroborated(
        "edict-provider/package/v1/generated/resources/resource.target-verifier-contract.cbor",
        "schemas/edict-provider/package/v1/generated/resources/resource.target-verifier-contract.cbor",
        "schemas/edict-provider/generated/v1/resources/resource.target-verifier-contract.cbor",
    ),
    AssetSpec::new(
        "edict-provider/package/v1/provider-manifest.echo.json",
        "schemas/edict-provider/package/v1/provider-manifest.echo.json",
    ),
    AssetSpec::new("repository/Cargo.lock.source", "Cargo.lock"),
    AssetSpec::new("repository/Cargo.toml.source", "Cargo.toml"),
    AssetSpec::new(
        "repository/crates/echo-edict-canonical/Cargo.toml.source",
        "crates/echo-edict-canonical/Cargo.toml",
    ),
    AssetSpec::new(
        "repository/crates/echo-edict-canonical/src/lib.rs.source",
        "crates/echo-edict-canonical/src/lib.rs",
    ),
    AssetSpec::new(
        "repository/crates/echo-registry-api/Cargo.toml.source",
        "crates/echo-registry-api/Cargo.toml",
    ),
    AssetSpec::new(
        "repository/crates/echo-registry-api/src/lib.rs.source",
        "crates/echo-registry-api/src/lib.rs",
    ),
    AssetSpec::new(
        "repository/crates/echo-wesley-gen/Cargo.toml.source",
        "crates/echo-wesley-gen/Cargo.toml",
    ),
    AssetSpec::new(
        "repository/rust-toolchain.toml.source",
        "rust-toolchain.toml",
    ),
];

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Synchronizes exact package-local Echo Edict provider assets"
)]
struct Args {
    /// Replace the exact package-local carrier tree from its fixed owners.
    #[arg(long)]
    write: bool,

    /// Prove Cargo selects exactly the fixed carrier tree for its archive.
    #[arg(long)]
    check_package_list: bool,

    /// Repository root containing the fixed owner paths.
    #[arg(long)]
    workspace_root: Option<PathBuf>,

    /// Override the carrier root, primarily for isolated check/write witnesses.
    #[arg(long)]
    assets_root: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = match args.workspace_root {
        Some(root) => root,
        None => crate_root.join("../.."),
    };
    let assets_root = args
        .assets_root
        .unwrap_or_else(|| crate_root.join("assets/v1"));
    // Write mode intentionally permits a stale package copy so authoritative
    // generated/component owners can break the regeneration cycle. Read-only
    // check mode still requires every distribution copy to corroborate them.
    let expected = load_expected_assets(&workspace_root, !args.write)?;

    if args.write {
        write_corpus(&assets_root, &expected)?;
        println!("Package-local provider assets synchronized");
    } else {
        check_assets(&assets_root, &expected)?;
        println!("Package-local provider assets are current");
    }

    if args.check_package_list {
        check_package_list(&workspace_root, &expected)?;
        println!("Cargo package selects the exact provider asset tree");
    }
    Ok(())
}

fn load_expected_assets(
    root: &Path,
    require_corroboration: bool,
) -> Result<Vec<ProviderArtifactCorpusFileV1>> {
    let mut files = Vec::with_capacity(ASSETS.len());
    let mut total_bytes = 0usize;
    for spec in ASSETS {
        let bytes = read_owner(root, spec.owner)?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| anyhow!("provider asset owner total byte length overflow"))?;
        if total_bytes > MAX_OWNER_TOTAL_BYTES {
            bail!("provider asset owners exceed total byte limit {MAX_OWNER_TOTAL_BYTES}");
        }
        if require_corroboration {
            let Some(corroborating_owner) = spec.corroborating_owner else {
                files.push(ProviderArtifactCorpusFileV1::new(spec.carrier, &bytes)?);
                continue;
            };
            let corroborating_bytes = read_owner(root, corroborating_owner)?;
            if bytes != corroborating_bytes {
                bail!(
                    "provider asset owner {} differs from corroborating owner {}",
                    spec.owner,
                    corroborating_owner
                );
            }
        }
        files.push(ProviderArtifactCorpusFileV1::new(spec.carrier, &bytes)?);
    }
    Ok(files)
}

fn read_owner(root: &Path, relative_path: &str) -> Result<Vec<u8>> {
    let path = root.join(relative_path);
    let metadata = std::fs::symlink_metadata(&path)
        .with_context(|| format!("failed to inspect provider asset owner {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "provider asset owner is not a regular non-symlink file: {}",
            path.display()
        );
    }
    if metadata.len() > MAX_OWNER_FILE_BYTES {
        bail!(
            "provider asset owner {} exceeds byte limit {MAX_OWNER_FILE_BYTES}",
            path.display()
        );
    }
    let bytes = std::fs::read(&path)
        .with_context(|| format!("failed to read provider asset owner {}", path.display()))?;
    if bytes.len() as u64 != metadata.len() {
        bail!(
            "provider asset owner changed length while being read: {}",
            path.display()
        );
    }
    Ok(bytes)
}

fn check_assets(root: &Path, expected: &[ProviderArtifactCorpusFileV1]) -> Result<()> {
    let actual = read_actual_corpus(root, expected)?;
    let drift = diff_exact_corpus_files_v1(expected, &actual)?;
    if drift.is_empty() {
        return Ok(());
    }

    eprintln!("Package-local provider asset drift:");
    for entry in &drift {
        eprintln!("  {}: {}", entry.kind().as_str(), entry.relative_path());
    }
    bail!("package-local provider assets are not current")
}

fn check_package_list(
    workspace_root: &Path,
    expected: &[ProviderArtifactCorpusFileV1],
) -> Result<()> {
    let cargo: OsString = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let manifest = workspace_root.join("crates/echo-wesley-gen/Cargo.toml");
    let output = Command::new(cargo)
        .current_dir(workspace_root)
        .arg("package")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--allow-dirty")
        .arg("--list")
        .output()
        .context("failed to execute cargo package --list for echo-wesley-gen")?;
    if !output.status.success() {
        bail!(
            "cargo package --list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout = String::from_utf8(output.stdout).context("cargo package list is not UTF-8")?;
    let actual_assets = stdout
        .lines()
        .filter(|path| path.starts_with("assets/v1/"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let expected_assets = expected
        .iter()
        .map(|file| format!("assets/v1/{}", file.relative_path()))
        .collect::<Vec<_>>();
    check_package_asset_inventory(actual_assets, expected_assets)
}

fn check_package_asset_inventory(
    mut actual_assets: Vec<String>,
    mut expected_assets: Vec<String>,
) -> Result<()> {
    actual_assets.sort_unstable();
    expected_assets.sort_unstable();
    if actual_assets != expected_assets {
        bail!(
            "cargo package provider asset inventory differs: expected {} entries, found {}\n\
             expected paths:\n  {}\n\
             actual paths:\n  {}",
            expected_assets.len(),
            actual_assets.len(),
            expected_assets.join("\n  "),
            actual_assets.join("\n  ")
        );
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{check_package_asset_inventory, load_expected_assets, ASSETS};
    use anyhow::Result;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn package_asset_inventory_ignores_listing_order_and_names_exact_drift() {
        let expected = vec!["assets/v1/alpha".to_owned(), "assets/v1/bravo".to_owned()];
        check_package_asset_inventory(
            vec!["assets/v1/bravo".to_owned(), "assets/v1/alpha".to_owned()],
            expected.clone(),
        )
        .expect("Cargo listing order does not change the selected asset inventory");

        let error = check_package_asset_inventory(
            vec!["assets/v1/alpha".to_owned(), "assets/v1/charlie".to_owned()],
            expected,
        )
        .expect_err("same-length asset substitution reports exact paths");
        let detail = error.to_string();
        assert!(detail.contains("assets/v1/bravo"));
        assert!(detail.contains("assets/v1/charlie"));
    }

    #[test]
    fn staged_write_uses_authority_while_strict_check_requires_package_corroboration() -> Result<()>
    {
        let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "echo-provider-asset-authority-{}-{sequence}",
            std::process::id()
        ));
        materialize_owner_fixture(&root)?;

        let generated = ASSETS
            .iter()
            .find(|spec| spec.carrier.ends_with("resource.target-ir.cbor"))
            .copied()
            .expect("fixed generated asset exists");
        let package_copy = root.join(
            generated
                .corroborating_owner
                .expect("generated asset has a package corroboration"),
        );
        std::fs::write(&package_copy, b"stale package copy")?;

        let strict = load_expected_assets(&root, true);
        assert!(strict.is_err());
        let staged = load_expected_assets(&root, false)?;
        let staged_file = staged
            .iter()
            .find(|file| file.relative_path() == generated.carrier)
            .expect("staged carrier is present");
        assert_eq!(
            staged_file.bytes(),
            std::fs::read(root.join(generated.owner))?
        );

        std::fs::copy(root.join(generated.owner), &package_copy)?;
        assert!(load_expected_assets(&root, true).is_ok());
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    fn materialize_owner_fixture(root: &Path) -> Result<()> {
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for spec in ASSETS {
            copy_owner(&source_root, root, spec.owner)?;
            if let Some(corroborating_owner) = spec.corroborating_owner {
                copy_owner(&source_root, root, corroborating_owner)?;
            }
        }
        Ok(())
    }

    fn copy_owner(source_root: &Path, target_root: &Path, relative: &str) -> Result<()> {
        let target = target_root.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source_root.join(relative), target)?;
        Ok(())
    }
}
