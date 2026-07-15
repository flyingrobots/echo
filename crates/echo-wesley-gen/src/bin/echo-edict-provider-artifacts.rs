// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Generates or checks Echo's deterministic Edict provider artifact corpus.

use anyhow::{anyhow, bail, Context, Result};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, File, OpenOptions},
};
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
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
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
    let Some(root_directory) = open_corpus_root(root, false)? else {
        return Ok(Vec::new());
    };

    read_actual_corpus_from_directory(&root_directory, expected)
}

fn read_actual_corpus_from_directory(
    root: &Dir,
    expected: &[ProviderArtifactCorpusFileV1],
) -> Result<Vec<ProviderArtifactCorpusFileV1>> {
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
    directory: &Dir,
    relative_directory: &str,
    expected_directories: &BTreeSet<String>,
    actual: &mut Vec<ProviderArtifactCorpusFileV1>,
) -> Result<()> {
    let mut entries = directory
        .entries()
        .with_context(|| format!("failed to read corpus directory {relative_directory}"))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to enumerate corpus directory {relative_directory}"))?;
    entries.sort_by_key(cap_std::fs::DirEntry::file_name);

    for entry in entries {
        let raw_file_name = entry.file_name();
        let diagnostic_path = Path::new(relative_directory).join(&raw_file_name);
        let file_name = utf8_file_name(raw_file_name.clone(), &diagnostic_path)?;
        let relative_path = if relative_directory.is_empty() {
            file_name
        } else {
            format!("{relative_directory}/{file_name}")
        };
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect corpus entry {relative_path}"))?;

        if file_type.is_file() {
            let file = open_file_nofollow(directory, &raw_file_name)
                .with_context(|| format!("failed to open corpus file {relative_path}"))?;
            let metadata = file
                .metadata()
                .with_context(|| format!("failed to inspect corpus file {relative_path}"))?;
            if !metadata.is_file() {
                actual.push(ProviderArtifactCorpusFileV1::new(
                    relative_path,
                    NON_REGULAR_ENTRY_BYTES,
                )?);
            } else if metadata.len() > MAX_ACTUAL_CORPUS_FILE_BYTES_U64 {
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
            let child = open_dir_nofollow(directory, &raw_file_name)
                .with_context(|| format!("failed to open corpus directory {relative_path}"))?;
            read_actual_directory(&child, &relative_path, expected_directories, actual)?;
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

fn open_corpus_root(root: &Path, create: bool) -> Result<Option<Dir>> {
    let absolute = std::path::absolute(root)
        .with_context(|| format!("failed to resolve corpus root {}", root.display()))?;
    let Some(name) = absolute.file_name() else {
        return Dir::open_ambient_dir(&absolute, ambient_authority())
            .map(Some)
            .with_context(|| format!("failed to open corpus root {}", absolute.display()));
    };
    let parent_path = absolute.parent().ok_or_else(|| {
        anyhow!(
            "corpus root has no parent directory: {}",
            absolute.display()
        )
    })?;

    if create {
        std::fs::create_dir_all(parent_path).with_context(|| {
            format!(
                "failed to create corpus root parent {}",
                parent_path.display()
            )
        })?;
    }
    let parent = match Dir::open_ambient_dir(parent_path, ambient_authority()) {
        Ok(parent) => parent,
        Err(error) if !create && error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to open corpus root parent {}",
                    parent_path.display()
                )
            });
        }
    };

    if create {
        ensure_corpus_directory(&parent, name, &absolute).map(Some)
    } else {
        open_existing_corpus_directory(&parent, name, &absolute)
    }
}

fn open_existing_corpus_directory(
    parent: &Dir,
    name: &OsStr,
    display_path: &Path,
) -> Result<Option<Dir>> {
    match parent.symlink_metadata(name) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing to traverse corpus directory symlink {}",
                display_path.display()
            );
        }
        Ok(metadata) if !metadata.is_dir() => {
            bail!(
                "corpus directory path is not a directory: {}",
                display_path.display()
            );
        }
        Ok(_) => open_dir_nofollow(parent, name)
            .map(Some)
            .with_context(|| format!("failed to open corpus directory {}", display_path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to inspect corpus directory {}",
                display_path.display()
            )
        }),
    }
}

fn ensure_corpus_directory(parent: &Dir, name: &OsStr, display_path: &Path) -> Result<Dir> {
    if let Some(directory) = open_existing_corpus_directory(parent, name, display_path)? {
        return Ok(directory);
    }

    match parent.create_dir(name) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to create corpus directory {}",
                    display_path.display()
                )
            });
        }
    }
    open_existing_corpus_directory(parent, name, display_path)?.ok_or_else(|| {
        anyhow!(
            "created corpus directory disappeared before it could be opened: {}",
            display_path.display()
        )
    })
}

fn open_dir_nofollow(parent: &Dir, name: &OsStr) -> std::io::Result<Dir> {
    let mut options = OpenOptions::new();
    options.read(true);
    options.follow(FollowSymlinks::No);
    options.maybe_dir(true);
    let file = parent.open_with(name, &options)?;
    if !file.metadata()?.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "corpus path component is not a directory",
        ));
    }
    Ok(Dir::from_std_file(file.into_std()))
}

fn open_file_nofollow(parent: &Dir, name: &OsStr) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    options.follow(FollowSymlinks::No);
    parent.open_with(name, &options)
}

fn write_corpus(root: &Path, files: &[ProviderArtifactCorpusFileV1]) -> Result<()> {
    let root_directory = open_corpus_root(root, true)?.ok_or_else(|| {
        anyhow!(
            "created corpus root could not be opened: {}",
            root.display()
        )
    })?;
    let actual = read_actual_corpus_from_directory(&root_directory, files)?;
    let expected_paths = files
        .iter()
        .map(ProviderArtifactCorpusFileV1::relative_path)
        .collect::<BTreeSet<_>>();
    if let Some(unexpected) = actual
        .iter()
        .find(|file| !expected_paths.contains(file.relative_path()))
    {
        bail!(
            "refusing to generate over unexpected corpus entry {}",
            unexpected.relative_path()
        );
    }

    let directories = prepare_corpus_directories(&root_directory, root, files)?;
    preflight_corpus_write(&directories, files)?;
    for file in files {
        let (parent, leaf) = corpus_file_parts(file.relative_path());
        let directory = directories.get(parent).ok_or_else(|| {
            anyhow!(
                "corpus file parent was not retained: {}",
                file.relative_path()
            )
        })?;
        let path = root.join(file.relative_path());
        replace_corpus_file(directory, OsStr::new(leaf), file.bytes(), &path)?;
        println!("  wrote {}", path.display());
    }
    Ok(())
}

fn prepare_corpus_directories(
    root: &Dir,
    root_display: &Path,
    files: &[ProviderArtifactCorpusFileV1],
) -> Result<BTreeMap<String, Dir>> {
    let mut directories = BTreeMap::new();
    directories.insert(
        String::new(),
        root.try_clone()
            .context("failed to retain the corpus root directory")?,
    );

    for relative_path in expected_directories(files) {
        let (parent, name) = corpus_file_parts(&relative_path);
        let parent_directory = directories
            .get(parent)
            .ok_or_else(|| anyhow!("corpus directory parent was not retained: {relative_path}"))?;
        let directory = ensure_corpus_directory(
            parent_directory,
            OsStr::new(name),
            &root_display.join(&relative_path),
        )?;
        directories.insert(relative_path, directory);
    }

    Ok(directories)
}

fn preflight_corpus_write(
    directories: &BTreeMap<String, Dir>,
    files: &[ProviderArtifactCorpusFileV1],
) -> Result<()> {
    for file in files {
        let (parent, leaf) = corpus_file_parts(file.relative_path());
        let directory = directories.get(parent).ok_or_else(|| {
            anyhow!(
                "corpus file parent was not retained: {}",
                file.relative_path()
            )
        })?;
        validate_existing_corpus_file(
            directory,
            OsStr::new(leaf),
            Path::new(file.relative_path()),
        )?;
    }
    Ok(())
}

fn corpus_file_parts(relative_path: &str) -> (&str, &str) {
    relative_path
        .rsplit_once('/')
        .unwrap_or(("", relative_path))
}

fn validate_existing_corpus_file(parent: &Dir, leaf: &OsStr, display_path: &Path) -> Result<()> {
    match parent.symlink_metadata(leaf) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing to replace corpus symlink {}",
                display_path.display()
            );
        }
        Ok(metadata) if !metadata.is_file() => {
            bail!(
                "refusing to replace non-file corpus entry {}",
                display_path.display()
            );
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect corpus file {}", display_path.display())),
    }
}

fn replace_corpus_file(
    parent: &Dir,
    leaf: &OsStr,
    bytes: &[u8],
    display_path: &Path,
) -> Result<()> {
    validate_existing_corpus_file(parent, leaf, display_path)?;
    let display_parent = display_path.parent().ok_or_else(|| {
        anyhow!(
            "corpus file has no display parent: {}",
            display_path.display()
        )
    })?;
    let (temporary_name, mut temporary_file) = create_temporary_file(parent, display_parent)?;

    let write_result = temporary_file
        .write_all(bytes)
        .with_context(|| {
            format!(
                "failed to write temporary corpus file {}",
                display_parent.join(&temporary_name).display()
            )
        })
        .and_then(|()| {
            temporary_file.sync_all().with_context(|| {
                format!(
                    "failed to sync temporary corpus file {}",
                    display_parent.join(&temporary_name).display()
                )
            })
        });
    drop(temporary_file);
    if let Err(error) = write_result {
        remove_temporary_file(parent, &temporary_name);
        return Err(error);
    }

    validate_existing_corpus_file(parent, leaf, display_path)?;
    if let Err(error) = replace_temporary_file(parent, &temporary_name, leaf, display_path) {
        remove_temporary_file(parent, &temporary_name);
        return Err(error);
    }
    Ok(())
}

fn create_temporary_file(parent: &Dir, display_parent: &Path) -> Result<(OsString, File)> {
    for _ in 0..MAX_TEMP_FILE_ATTEMPTS {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let name = OsString::from(format!(
            ".echo-provider-artifact-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        options.follow(FollowSymlinks::No);
        match parent.open_with(&name, &options) {
            Ok(file) => return Ok((name, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to create temporary corpus file {}",
                        display_parent.join(&name).display()
                    )
                });
            }
        }
    }
    bail!(
        "failed to reserve a temporary corpus file in {} after {} attempts",
        display_parent.display(),
        MAX_TEMP_FILE_ATTEMPTS
    );
}

fn replace_temporary_file(
    parent: &Dir,
    temporary_name: &OsStr,
    destination_name: &OsStr,
    destination_display: &Path,
) -> Result<()> {
    replace_temporary_file_with(
        temporary_name,
        destination_name,
        destination_display,
        |from, to| parent.rename(from, parent, to),
    )
}

fn replace_temporary_file_with(
    temporary_name: &OsStr,
    destination_name: &OsStr,
    destination_display: &Path,
    replace: impl FnOnce(&OsStr, &OsStr) -> std::io::Result<()>,
) -> Result<()> {
    replace(temporary_name, destination_name).with_context(|| {
        format!(
            "failed to replace corpus file {} from {}",
            destination_display.display(),
            temporary_name.to_string_lossy()
        )
    })
}

fn remove_temporary_file(parent: &Dir, name: &OsStr) {
    drop(parent.remove_file(name));
}

#[cfg(test)]
mod tests {
    use super::{
        open_dir_nofollow, replace_corpus_file, replace_temporary_file_with, NEXT_TEMP_FILE,
    };
    use anyhow::Result;
    use cap_std::{ambient_authority, fs::Dir};
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::Ordering;

    fn test_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "echo-provider-{label}-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn failed_replacement_preserves_existing_destination() -> Result<()> {
        let root = test_directory("replacement-test");
        std::fs::create_dir(&root)?;
        let temporary = root.join("temporary");
        let destination = root.join("destination");
        std::fs::write(&temporary, b"replacement")?;
        std::fs::write(&destination, b"admitted")?;

        let result = replace_temporary_file_with(
            OsStr::new("temporary"),
            OsStr::new("destination"),
            &destination,
            |_, _| Err(std::io::Error::other("injected replacement failure")),
        );

        assert!(result.is_err());
        assert_eq!(std::fs::read(&destination)?, b"admitted");
        assert_eq!(std::fs::read(&temporary)?, b"replacement");
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn retained_parent_handle_prevents_symlink_redirection() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = test_directory("parent-swap");
        let outside = test_directory("parent-swap-outside");
        let evidence = root.join("evidence");
        let parked = root.join("evidence-parked");
        std::fs::create_dir_all(&evidence)?;
        std::fs::create_dir_all(&outside)?;
        let root_directory = Dir::open_ambient_dir(&root, ambient_authority())?;
        let evidence_directory = open_dir_nofollow(&root_directory, OsStr::new("evidence"))?;

        std::fs::rename(&evidence, &parked)?;
        symlink(&outside, &evidence)?;

        replace_corpus_file(
            &evidence_directory,
            OsStr::new("artifact"),
            b"canonical",
            Path::new("evidence/artifact"),
        )?;

        assert!(!outside.join("artifact").exists());
        assert_eq!(std::fs::read(parked.join("artifact"))?, b"canonical");
        std::fs::remove_dir_all(root)?;
        std::fs::remove_dir_all(outside)?;
        Ok(())
    }
}
