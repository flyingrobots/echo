// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Safe exact-tree filesystem access for deterministic provider corpora.

use crate::provider_corpus::ProviderArtifactCorpusFileV1;
use anyhow::{anyhow, bail, Context, Result};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, File, OpenOptions},
};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const NON_REGULAR_ENTRY_BYTES: &[u8] = b"echo.provider-artifact-corpus.non-regular/v1";
const UNEXPECTED_ENTRY_BYTES: &[u8] = b"echo.provider-artifact-corpus.unexpected/v1";
const CHANGED_ENTRY_BYTES: &[u8] = b"echo.provider-artifact-corpus.changed/v1";
const ALTERNATE_CHANGED_ENTRY_BYTES: &[u8] = b"echo.provider-artifact-corpus.changed-alt/v1";
const MAX_ACTUAL_CORPUS_FILE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ACTUAL_CORPUS_FILE_BYTES_U64: u64 = 32 * 1024 * 1024;
const MAX_EXPECTED_CORPUS_FILE_COUNT: usize = 256;
const MAX_EXPECTED_CORPUS_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_ACTUAL_CORPUS_ENTRY_COUNT: usize = 1_024;
const MAX_ACTUAL_CORPUS_READ_BYTES: usize =
    MAX_EXPECTED_CORPUS_TOTAL_BYTES + MAX_EXPECTED_CORPUS_FILE_COUNT;
const MAX_TEMP_FILE_ATTEMPTS: usize = 1_024;

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

/// Reads the exact corpus tree without following symbolic links.
pub fn read_actual_corpus(
    root: &Path,
    expected: &[ProviderArtifactCorpusFileV1],
) -> Result<Vec<ProviderArtifactCorpusFileV1>> {
    let expected_by_path = validate_expected_corpus(expected)?;
    let Some(root_directory) = open_corpus_root(root, false)? else {
        return Ok(Vec::new());
    };

    read_actual_corpus_from_directory(&root_directory, expected, &expected_by_path)
}

fn read_actual_corpus_from_directory<'a>(
    root: &Dir,
    expected: &'a [ProviderArtifactCorpusFileV1],
    expected_by_path: &BTreeMap<&'a str, &'a [u8]>,
) -> Result<Vec<ProviderArtifactCorpusFileV1>> {
    let expected_directories = expected_directories(expected);
    let mut actual = Vec::new();
    let mut budget = ActualCorpusReadBudget::default();
    read_actual_directory(
        root,
        "",
        &expected_directories,
        expected_by_path,
        &mut budget,
        &mut actual,
    )?;
    Ok(actual)
}

fn validate_expected_corpus(
    expected: &[ProviderArtifactCorpusFileV1],
) -> Result<BTreeMap<&str, &[u8]>> {
    if expected.is_empty() {
        bail!("expected corpus inventory must not be empty");
    }
    if expected.len() > MAX_EXPECTED_CORPUS_FILE_COUNT {
        bail!("expected corpus inventory exceeds file limit {MAX_EXPECTED_CORPUS_FILE_COUNT}");
    }

    let mut total_bytes = 0usize;
    let mut inventory = BTreeMap::new();
    let mut previous_path: Option<&str> = None;
    for file in expected {
        if let Some(previous) = previous_path {
            if previous.as_bytes() >= file.relative_path().as_bytes() {
                bail!(
                    "expected corpus paths must be strictly sorted: {previous} then {}",
                    file.relative_path()
                );
            }
        }
        if file.bytes().len() > MAX_ACTUAL_CORPUS_FILE_BYTES {
            bail!(
                "expected corpus file {} exceeds byte limit {MAX_ACTUAL_CORPUS_FILE_BYTES}",
                file.relative_path()
            );
        }
        total_bytes = total_bytes
            .checked_add(file.bytes().len())
            .ok_or_else(|| anyhow!("expected corpus total byte length overflow"))?;
        if total_bytes > MAX_EXPECTED_CORPUS_TOTAL_BYTES {
            bail!("expected corpus exceeds total byte limit {MAX_EXPECTED_CORPUS_TOTAL_BYTES}");
        }
        inventory.insert(file.relative_path(), file.bytes());
        previous_path = Some(file.relative_path());
    }
    for file in expected {
        for (separator, _) in file.relative_path().match_indices('/') {
            let ancestor = &file.relative_path()[..separator];
            if inventory.contains_key(ancestor) {
                bail!("expected corpus path is both file and directory: {ancestor}");
            }
        }
    }
    let directory_count = expected_directories(expected).len();
    let expected_entry_count = expected
        .len()
        .checked_add(directory_count)
        .ok_or_else(|| anyhow!("expected corpus entry count overflow"))?;
    if expected_entry_count > MAX_ACTUAL_CORPUS_ENTRY_COUNT {
        bail!("expected corpus tree exceeds entry limit {MAX_ACTUAL_CORPUS_ENTRY_COUNT}");
    }
    Ok(inventory)
}

#[derive(Default)]
struct ActualCorpusReadBudget {
    entries: usize,
    read_bytes: usize,
}

impl ActualCorpusReadBudget {
    fn observe_entry(&mut self) -> Result<()> {
        self.entries = self
            .entries
            .checked_add(1)
            .ok_or_else(|| anyhow!("actual corpus entry count overflow"))?;
        if self.entries > MAX_ACTUAL_CORPUS_ENTRY_COUNT {
            bail!("actual corpus exceeds entry limit {MAX_ACTUAL_CORPUS_ENTRY_COUNT}");
        }
        Ok(())
    }

    fn reserve_read(&mut self, bytes: usize) -> Result<()> {
        self.read_bytes = self
            .read_bytes
            .checked_add(bytes)
            .ok_or_else(|| anyhow!("actual corpus read byte count overflow"))?;
        if self.read_bytes > MAX_ACTUAL_CORPUS_READ_BYTES {
            bail!("actual corpus exceeds read limit {MAX_ACTUAL_CORPUS_READ_BYTES}");
        }
        Ok(())
    }
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
    expected_by_path: &BTreeMap<&str, &[u8]>,
    budget: &mut ActualCorpusReadBudget,
    actual: &mut Vec<ProviderArtifactCorpusFileV1>,
) -> Result<()> {
    let mut entries = Vec::new();
    for entry in directory
        .entries()
        .with_context(|| format!("failed to read corpus directory {relative_directory}"))?
    {
        budget.observe_entry()?;
        entries.push(entry.with_context(|| {
            format!("failed to enumerate corpus directory {relative_directory}")
        })?);
    }
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
            let Some(expected_bytes) = expected_by_path.get(relative_path.as_str()).copied() else {
                actual.push(ProviderArtifactCorpusFileV1::new(
                    relative_path,
                    UNEXPECTED_ENTRY_BYTES,
                )?);
                continue;
            };
            let file = open_file_nofollow(directory, &raw_file_name)
                .with_context(|| format!("failed to open corpus file {relative_path}"))?;
            let metadata = file
                .metadata()
                .with_context(|| format!("failed to inspect corpus file {relative_path}"))?;
            if !metadata.is_file()
                || metadata.len() > MAX_ACTUAL_CORPUS_FILE_BYTES_U64
                || metadata.len() != expected_bytes.len() as u64
            {
                actual.push(ProviderArtifactCorpusFileV1::new(
                    relative_path,
                    changed_entry_bytes(expected_bytes),
                )?);
            } else {
                let read_limit = expected_bytes.len().checked_add(1).ok_or_else(|| {
                    anyhow!("actual corpus read limit overflow for {relative_path}")
                })?;
                budget.reserve_read(read_limit)?;
                let mut bytes = Vec::new();
                file.take(read_limit as u64)
                    .read_to_end(&mut bytes)
                    .with_context(|| format!("failed to read corpus file {relative_path}"))?;
                let exact_bytes = if bytes.len() == expected_bytes.len() {
                    &bytes
                } else {
                    changed_entry_bytes(expected_bytes)
                };
                actual.push(ProviderArtifactCorpusFileV1::new(
                    relative_path,
                    exact_bytes,
                )?);
            }
        } else if file_type.is_dir() && expected_directories.contains(&relative_path) {
            let child = open_dir_nofollow(directory, &raw_file_name)
                .with_context(|| format!("failed to open corpus directory {relative_path}"))?;
            read_actual_directory(
                &child,
                &relative_path,
                expected_directories,
                expected_by_path,
                budget,
                actual,
            )?;
        } else {
            let marker = expected_by_path
                .get(relative_path.as_str())
                .map_or(NON_REGULAR_ENTRY_BYTES, |expected| {
                    changed_entry_bytes(expected)
                });
            actual.push(ProviderArtifactCorpusFileV1::new(relative_path, marker)?);
        }
    }
    Ok(())
}

fn changed_entry_bytes(expected: &[u8]) -> &'static [u8] {
    if expected == CHANGED_ENTRY_BYTES {
        ALTERNATE_CHANGED_ENTRY_BYTES
    } else {
        CHANGED_ENTRY_BYTES
    }
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

/// Writes an exact corpus tree after refusing unsafe or unexpected entries.
#[allow(clippy::print_stdout)]
pub fn write_corpus(root: &Path, files: &[ProviderArtifactCorpusFileV1]) -> Result<()> {
    let expected_by_path = validate_expected_corpus(files)?;
    let root_directory = open_corpus_root(root, true)?.ok_or_else(|| {
        anyhow!(
            "created corpus root could not be opened: {}",
            root.display()
        )
    })?;
    let actual = read_actual_corpus_from_directory(&root_directory, files, &expected_by_path)?;
    if let Some(unexpected) = actual
        .iter()
        .find(|file| !expected_by_path.contains_key(file.relative_path()))
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
        open_dir_nofollow, read_actual_corpus, replace_corpus_file, replace_temporary_file_with,
        write_corpus, NEXT_TEMP_FILE,
    };
    use crate::provider_corpus::ProviderArtifactCorpusFileV1;
    use anyhow::Result;
    use cap_std::{ambient_authority, fs::Dir};
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::Ordering;

    fn test_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "echo-provider-{label}-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        drop(std::fs::remove_dir_all(&path));
        path
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

    #[test]
    fn invalid_expected_inventories_refuse_before_root_creation() -> Result<()> {
        let duplicate_root = test_directory("duplicate-inventory");
        let duplicate = vec![
            ProviderArtifactCorpusFileV1::new("same", b"first")?,
            ProviderArtifactCorpusFileV1::new("same", b"second")?,
        ];
        let Err(duplicate_error) = write_corpus(&duplicate_root, &duplicate) else {
            anyhow::bail!("duplicate expected paths did not fail closed");
        };
        assert!(duplicate_error.to_string().contains("strictly sorted"));
        assert!(!duplicate_root.exists());

        let unordered_root = test_directory("unordered-inventory");
        let unordered = vec![
            ProviderArtifactCorpusFileV1::new("z", b"last")?,
            ProviderArtifactCorpusFileV1::new("a", b"first")?,
        ];
        let Err(unordered_error) = write_corpus(&unordered_root, &unordered) else {
            anyhow::bail!("unordered expected paths did not fail closed");
        };
        assert!(unordered_error.to_string().contains("strictly sorted"));
        assert!(!unordered_root.exists());

        let empty_root = test_directory("empty-inventory");
        let Err(empty_error) = write_corpus(&empty_root, &[]) else {
            anyhow::bail!("an empty expected inventory did not fail closed");
        };
        assert!(empty_error.to_string().contains("must not be empty"));
        assert!(!empty_root.exists());

        let prefix_root = test_directory("prefix-inventory");
        let prefix_conflict = vec![
            ProviderArtifactCorpusFileV1::new("artifact", b"file")?,
            ProviderArtifactCorpusFileV1::new("artifact/child", b"child")?,
        ];
        let Err(prefix_error) = write_corpus(&prefix_root, &prefix_conflict) else {
            anyhow::bail!("a file/directory prefix conflict did not fail closed");
        };
        assert!(prefix_error.to_string().contains("both file and directory"));
        assert!(!prefix_root.exists());

        let interposed_prefix_root = test_directory("interposed-prefix-inventory");
        let interposed_prefix_conflict = vec![
            ProviderArtifactCorpusFileV1::new("artifact", b"file")?,
            ProviderArtifactCorpusFileV1::new("artifact.", b"interposed")?,
            ProviderArtifactCorpusFileV1::new("artifact/child", b"child")?,
        ];
        let Err(interposed_prefix_error) =
            write_corpus(&interposed_prefix_root, &interposed_prefix_conflict)
        else {
            anyhow::bail!("an interposed file/directory conflict did not fail closed");
        };
        assert!(interposed_prefix_error
            .to_string()
            .contains("both file and directory"));
        assert!(!interposed_prefix_root.exists());

        let overfull_root = test_directory("overfull-inventory");
        let overfull = (0..257)
            .map(|index| ProviderArtifactCorpusFileV1::new(format!("entry-{index:04}"), &[]))
            .collect::<Result<Vec<_>, _>>()?;
        let Err(overfull_error) = write_corpus(&overfull_root, &overfull) else {
            anyhow::bail!("an excessive expected inventory did not fail closed");
        };
        assert!(overfull_error.to_string().contains("file limit 256"));
        assert!(!overfull_root.exists());

        let deep_root = test_directory("deep-inventory");
        let deep = (0..5)
            .map(|branch| {
                let path = std::iter::once(format!("b{branch}"))
                    .chain(std::iter::repeat_n("s".to_owned(), 210))
                    .chain(std::iter::once("file".to_owned()))
                    .collect::<Vec<_>>()
                    .join("/");
                ProviderArtifactCorpusFileV1::new(path, &[])
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Err(deep_error) = write_corpus(&deep_root, &deep) else {
            anyhow::bail!("an excessive expected directory tree did not fail closed");
        };
        assert!(deep_error
            .to_string()
            .contains("tree exceeds entry limit 1024"));
        assert!(!deep_root.exists());
        Ok(())
    }

    #[test]
    fn actual_tree_scan_is_bounded_and_does_not_read_unexpected_files() -> Result<()> {
        const ADVERSARIAL_ENTRY_COUNT: usize = 1_025;
        const UNEXPECTED_MARKER: &[u8] = b"echo.provider-artifact-corpus.unexpected/v1";

        let unexpected_root = test_directory("unexpected-content");
        std::fs::create_dir(&unexpected_root)?;
        std::fs::write(
            unexpected_root.join("operator-owned"),
            b"operator-secret-bytes",
        )?;
        let expected = vec![ProviderArtifactCorpusFileV1::new("expected", b"expected")?];
        let actual = read_actual_corpus(&unexpected_root, &expected)?;
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].relative_path(), "operator-owned");
        assert_eq!(actual[0].bytes(), UNEXPECTED_MARKER);
        assert_eq!(
            std::fs::read(unexpected_root.join("operator-owned"))?,
            b"operator-secret-bytes"
        );
        std::fs::remove_dir_all(unexpected_root)?;

        let marker_root = test_directory("marker-collision");
        std::fs::create_dir_all(marker_root.join("expected"))?;
        let marker_expected = vec![ProviderArtifactCorpusFileV1::new(
            "expected",
            super::NON_REGULAR_ENTRY_BYTES,
        )?];
        let marker_actual = read_actual_corpus(&marker_root, &marker_expected)?;
        assert_ne!(marker_actual[0].bytes(), marker_expected[0].bytes());
        std::fs::remove_dir_all(marker_root)?;

        let crowded_root = test_directory("entry-budget");
        std::fs::create_dir(&crowded_root)?;
        for index in 0..ADVERSARIAL_ENTRY_COUNT {
            std::fs::write(crowded_root.join(format!("unexpected-{index:04}")), [])?;
        }
        let Err(error) = read_actual_corpus(&crowded_root, &expected) else {
            anyhow::bail!("an excessive actual inventory did not fail closed");
        };
        assert!(error.to_string().contains("entry limit 1024"));
        assert_eq!(
            std::fs::read_dir(&crowded_root)?.count(),
            ADVERSARIAL_ENTRY_COUNT
        );
        std::fs::remove_dir_all(crowded_root)?;
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
