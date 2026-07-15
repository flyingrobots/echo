// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Generates or checks Echo's deterministic Edict provider artifact corpus.

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use echo_wesley_gen::provider_artifacts::generate_provider_primary_artifacts_v1;
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_corpus::{
    checked_provider_generator_source_bundle_v1, diff_provider_artifact_corpus_v1,
    render_provider_artifact_corpus_v1, ProviderArtifactCorpusFileV1,
};
use echo_wesley_gen::provider_generation::build_provider_generation_input_v1;
use echo_wesley_gen::provider_provenance::generate_provider_generation_provenance_v1;
use echo_wesley_gen::provider_review::generate_provider_generation_review_v1;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SOURCE: &[u8] =
    include_bytes!("../../../../schemas/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] =
    include_bytes!("../../../../schemas/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../../../../schemas/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../../../../schemas/edict-provider/contracts/v1/manifest.json");
const NON_REGULAR_ENTRY_BYTES: &[u8] = b"echo.provider-artifact-corpus.non-regular/v1";
const OVERSIZED_ENTRY_BYTES: &[u8] = b"echo.provider-artifact-corpus.oversized/v1";
const MAX_ACTUAL_CORPUS_FILE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ACTUAL_CORPUS_FILE_BYTES_U64: u64 = 32 * 1024 * 1024;
const MAX_TEMP_FILE_ATTEMPTS: usize = 1_024;

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

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

fn read_actual_corpus(
    root: &Path,
    expected: &[ProviderArtifactCorpusFileV1],
) -> Result<Vec<ProviderArtifactCorpusFileV1>> {
    let metadata = match std::fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect corpus root {}", root.display()));
        }
    };
    if metadata.file_type().is_symlink() {
        bail!("corpus root must not be a symlink: {}", root.display());
    }
    if !metadata.is_dir() {
        bail!("corpus root is not a directory: {}", root.display());
    }

    let expected_directories = expected_directories(expected);
    let mut actual = Vec::new();
    read_actual_directory(root, "", &expected_directories, &mut actual)?;
    Ok(actual)
}

fn expected_directories(expected: &[ProviderArtifactCorpusFileV1]) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    for file in expected {
        let components = file.relative_path().split('/').collect::<Vec<_>>();
        for end in 1..components.len() {
            directories.insert(components[..end].join("/"));
        }
    }
    directories
}

fn read_actual_directory(
    directory: &Path,
    relative_directory: &str,
    expected_directories: &BTreeSet<String>,
    actual: &mut Vec<ProviderArtifactCorpusFileV1>,
) -> Result<()> {
    let mut entries = std::fs::read_dir(directory)
        .with_context(|| format!("failed to read corpus directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| {
            format!(
                "failed to enumerate corpus directory {}",
                directory.display()
            )
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let file_name = utf8_file_name(entry.file_name(), &entry.path())?;
        let relative_path = if relative_directory.is_empty() {
            file_name
        } else {
            format!("{relative_directory}/{file_name}")
        };
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect corpus entry {relative_path}"))?;

        if file_type.is_file() {
            let file = File::open(entry.path())
                .with_context(|| format!("failed to open corpus file {relative_path}"))?;
            let metadata = file
                .metadata()
                .with_context(|| format!("failed to inspect corpus file {relative_path}"))?;
            if metadata.len() > MAX_ACTUAL_CORPUS_FILE_BYTES_U64 {
                actual.push(ProviderArtifactCorpusFileV1::new(
                    relative_path,
                    OVERSIZED_ENTRY_BYTES,
                )?);
            } else {
                let mut bytes = Vec::new();
                file.take(MAX_ACTUAL_CORPUS_FILE_BYTES_U64 + 1)
                    .read_to_end(&mut bytes)
                    .with_context(|| format!("failed to read corpus file {relative_path}"))?;
                let exact_bytes = if bytes.len() > MAX_ACTUAL_CORPUS_FILE_BYTES {
                    OVERSIZED_ENTRY_BYTES
                } else {
                    &bytes
                };
                actual.push(ProviderArtifactCorpusFileV1::new(
                    relative_path,
                    exact_bytes,
                )?);
            }
        } else if file_type.is_dir() && expected_directories.contains(&relative_path) {
            read_actual_directory(&entry.path(), &relative_path, expected_directories, actual)?;
        } else {
            actual.push(ProviderArtifactCorpusFileV1::new(
                relative_path,
                NON_REGULAR_ENTRY_BYTES,
            )?);
        }
    }
    Ok(())
}

fn utf8_file_name(file_name: OsString, path: &Path) -> Result<String> {
    file_name.into_string().map_err(|name| {
        anyhow!(
            "corpus entry name is not UTF-8 at {}: {}",
            path.display(),
            name.to_string_lossy()
        )
    })
}

fn write_corpus(root: &Path, files: &[ProviderArtifactCorpusFileV1]) -> Result<()> {
    ensure_corpus_root(root)?;
    let root = std::fs::canonicalize(root)
        .with_context(|| format!("failed to resolve corpus root {}", root.display()))?;
    preflight_corpus_write(&root, files)?;
    for file in files {
        let mut directory = root.clone();
        if let Some(parent) = Path::new(file.relative_path()).parent() {
            for component in parent.components() {
                directory.push(component.as_os_str());
                ensure_corpus_directory(&directory)?;
            }
        }
        let path = root.join(file.relative_path());
        replace_corpus_file(&path, file.bytes())?;
        println!("  wrote {}", path.display());
    }
    Ok(())
}

fn preflight_corpus_write(root: &Path, files: &[ProviderArtifactCorpusFileV1]) -> Result<()> {
    match std::fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("corpus root must not be a symlink: {}", root.display());
        }
        Ok(metadata) if !metadata.is_dir() => {
            bail!("corpus root is not a directory: {}", root.display());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect corpus root {}", root.display()));
        }
    }

    for file in files {
        let mut directory = root.to_path_buf();
        let mut parents_exist = true;
        if let Some(parent) = Path::new(file.relative_path()).parent() {
            for component in parent.components() {
                directory.push(component.as_os_str());
                match std::fs::symlink_metadata(&directory) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        bail!(
                            "refusing to traverse corpus directory symlink {}",
                            directory.display()
                        );
                    }
                    Ok(metadata) if !metadata.is_dir() => {
                        bail!(
                            "corpus directory path is not a directory: {}",
                            directory.display()
                        );
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        parents_exist = false;
                        break;
                    }
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("failed to inspect corpus directory {}", directory.display())
                        });
                    }
                }
            }
        }
        if parents_exist {
            validate_existing_corpus_file(&root.join(file.relative_path()))?;
        }
    }
    Ok(())
}

fn validate_existing_corpus_file(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("refusing to replace corpus symlink {}", path.display());
        }
        Ok(metadata) if !metadata.is_file() => {
            bail!(
                "refusing to replace non-file corpus entry {}",
                path.display()
            );
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to inspect corpus file {}", path.display()))
        }
    }
}

fn replace_corpus_file(path: &Path, bytes: &[u8]) -> Result<()> {
    validate_existing_corpus_file(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("corpus file has no parent: {}", path.display()))?;
    let (temporary_path, mut temporary_file) = create_temporary_file(parent)?;

    let write_result = temporary_file
        .write_all(bytes)
        .with_context(|| {
            format!(
                "failed to write temporary corpus file {}",
                temporary_path.display()
            )
        })
        .and_then(|()| {
            temporary_file.sync_all().with_context(|| {
                format!(
                    "failed to sync temporary corpus file {}",
                    temporary_path.display()
                )
            })
        });
    drop(temporary_file);
    if let Err(error) = write_result {
        remove_temporary_file(&temporary_path);
        return Err(error);
    }

    validate_existing_corpus_file(path)?;
    if let Err(error) = replace_temporary_file(&temporary_path, path) {
        remove_temporary_file(&temporary_path);
        return Err(error);
    }
    Ok(())
}

fn create_temporary_file(parent: &Path) -> Result<(PathBuf, File)> {
    for _ in 0..MAX_TEMP_FILE_ATTEMPTS {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".echo-provider-artifact-{}-{sequence}.tmp",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to create temporary corpus file {}", path.display())
                });
            }
        }
    }
    bail!(
        "failed to reserve a temporary corpus file in {} after {} attempts",
        parent.display(),
        MAX_TEMP_FILE_ATTEMPTS
    );
}

#[cfg(not(windows))]
fn replace_temporary_file(temporary_path: &Path, destination: &Path) -> Result<()> {
    std::fs::rename(temporary_path, destination).with_context(|| {
        format!(
            "failed to replace corpus file {} from {}",
            destination.display(),
            temporary_path.display()
        )
    })
}

#[cfg(windows)]
fn replace_temporary_file(temporary_path: &Path, destination: &Path) -> Result<()> {
    match std::fs::symlink_metadata(destination) {
        Ok(_) => {
            validate_existing_corpus_file(destination)?;
            std::fs::remove_file(destination).with_context(|| {
                format!(
                    "failed to unlink existing corpus file {}",
                    destination.display()
                )
            })?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to inspect corpus file {}", destination.display())
            });
        }
    }
    std::fs::rename(temporary_path, destination).with_context(|| {
        format!(
            "failed to replace corpus file {} from {}",
            destination.display(),
            temporary_path.display()
        )
    })
}

fn remove_temporary_file(path: &Path) {
    drop(std::fs::remove_file(path));
}

fn ensure_corpus_root(root: &Path) -> Result<()> {
    match std::fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("corpus root must not be a symlink: {}", root.display());
        }
        Ok(metadata) if !metadata.is_dir() => {
            bail!("corpus root is not a directory: {}", root.display());
        }
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect corpus root {}", root.display()));
        }
    }
    std::fs::create_dir_all(root)
        .with_context(|| format!("failed to create corpus root {}", root.display()))?;
    let metadata = std::fs::symlink_metadata(root)
        .with_context(|| format!("failed to inspect created corpus root {}", root.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("created corpus root is not a directory: {}", root.display());
    }
    Ok(())
}

fn ensure_corpus_directory(directory: &Path) -> Result<()> {
    match std::fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing to traverse corpus directory symlink {}",
                directory.display()
            );
        }
        Ok(metadata) if !metadata.is_dir() => {
            bail!(
                "corpus directory path is not a directory: {}",
                directory.display()
            );
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(directory).with_context(|| {
                format!("failed to create corpus directory {}", directory.display())
            })
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect corpus directory {}", directory.display())),
    }
}
