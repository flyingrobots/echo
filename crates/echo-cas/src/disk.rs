// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Filesystem-backed content-addressed blob tier.

use std::collections::HashSet;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use thiserror::Error;

use crate::{blob_hash, BlobHash, CasError};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Filesystem-backed content-addressed blob tier.
///
/// `DiskTier` persists blob bytes under their content-only BLAKE3 hash. It is
/// intentionally fallible instead of implementing [`crate::BlobStore`], whose
/// current `put`/`get` surface has no way to report filesystem failures.
pub struct DiskTier {
    root: PathBuf,
    blobs_dir: PathBuf,
    pins: HashSet<BlobHash>,
}

impl DiskTier {
    /// Opens or creates a disk tier rooted at `root`.
    ///
    /// # Errors
    ///
    /// Returns [`DiskTierError::Io`] when the tier directories cannot be
    /// created.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, DiskTierError> {
        let root = root.as_ref().to_path_buf();
        let blobs_dir = root.join("blobs");
        fs::create_dir_all(&blobs_dir)
            .map_err(|source| DiskTierError::io("create_dir_all", &blobs_dir, source))?;
        Ok(Self {
            root,
            blobs_dir,
            pins: HashSet::new(),
        })
    }

    /// Returns the tier root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Computes the content hash and stores `bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`DiskTierError::Io`] when the blob cannot be written.
    pub fn put(&self, bytes: &[u8]) -> Result<BlobHash, DiskTierError> {
        let hash = blob_hash(bytes);
        self.put_verified(hash, bytes)?;
        Ok(hash)
    }

    /// Stores `bytes` only if they match `expected`.
    ///
    /// # Errors
    ///
    /// Returns [`DiskTierError::Cas`] when `bytes` hash to a different
    /// [`BlobHash`], without mutating the tier. Returns [`DiskTierError::Io`]
    /// when verified bytes cannot be written.
    pub fn put_verified(&self, expected: BlobHash, bytes: &[u8]) -> Result<(), DiskTierError> {
        let computed = blob_hash(bytes);
        if computed != expected {
            return Err(CasError::HashMismatch { expected, computed }.into());
        }
        let path = self.blob_path(&expected);
        let parent = path
            .parent()
            .ok_or_else(|| DiskTierError::invalid_blob_path(path.clone()))?;
        fs::create_dir_all(parent)
            .map_err(|source| DiskTierError::io("create_dir_all", parent, source))?;
        let temp_path = Self::temp_path(parent, &expected);
        fs::write(&temp_path, bytes)
            .map_err(|source| DiskTierError::io("write", &temp_path, source))?;
        fs::rename(&temp_path, &path).map_err(|source| {
            let _ = fs::remove_file(&temp_path);
            DiskTierError::io("rename", &path, source)
        })?;
        Ok(())
    }

    /// Retrieves blob bytes by content hash.
    ///
    /// Missing blobs return `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns [`DiskTierError::Io`] for filesystem errors other than absence
    /// and [`DiskTierError::Cas`] when stored bytes no longer match their path
    /// hash.
    pub fn get(&self, hash: &BlobHash) -> Result<Option<Arc<[u8]>>, DiskTierError> {
        let path = self.blob_path(hash);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(DiskTierError::io("read", &path, source)),
        };
        let computed = blob_hash(&bytes);
        if computed != *hash {
            return Err(CasError::HashMismatch {
                expected: *hash,
                computed,
            }
            .into());
        }
        Ok(Some(Arc::from(bytes)))
    }

    /// Returns whether a blob exists without reading its bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DiskTierError::Io`] when metadata lookup fails for reasons
    /// other than absence.
    pub fn has(&self, hash: &BlobHash) -> Result<bool, DiskTierError> {
        let path = self.blob_path(hash);
        match fs::metadata(&path) {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(source) => Err(DiskTierError::io("metadata", &path, source)),
        }
    }

    /// Returns all stored blob hashes in sorted order.
    ///
    /// # Errors
    ///
    /// Returns [`DiskTierError::Io`] when the blob directory cannot be read.
    pub fn list(&self) -> Result<Vec<BlobHash>, DiskTierError> {
        let mut hashes = Vec::new();
        let entries = match fs::read_dir(&self.blobs_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(hashes),
            Err(source) => return Err(DiskTierError::io("read_dir", &self.blobs_dir, source)),
        };
        for shard in entries {
            let shard = shard
                .map_err(|source| DiskTierError::io("read_dir_entry", &self.blobs_dir, source))?;
            let shard_path = shard.path();
            if !shard_path.is_dir() {
                continue;
            }
            let blob_entries = fs::read_dir(&shard_path)
                .map_err(|source| DiskTierError::io("read_dir", &shard_path, source))?;
            for blob in blob_entries {
                let blob = blob
                    .map_err(|source| DiskTierError::io("read_dir_entry", &shard_path, source))?;
                let path = blob.path();
                if !path.is_file() {
                    continue;
                }
                let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                    return Err(DiskTierError::invalid_blob_path(path));
                };
                if file_name.starts_with('.') {
                    continue;
                }
                let Some(hash) = blob_hash_from_hex(file_name) else {
                    return Err(DiskTierError::invalid_blob_path(path));
                };
                hashes.push(hash);
            }
        }
        hashes.sort_unstable();
        Ok(hashes)
    }

    /// Marks a hash as pinned in this process.
    pub fn pin(&mut self, hash: &BlobHash) {
        self.pins.insert(*hash);
    }

    /// Removes a process-local pin.
    pub fn unpin(&mut self, hash: &BlobHash) {
        self.pins.remove(hash);
    }

    /// Returns whether the hash is pinned in this process.
    #[must_use]
    pub fn is_pinned(&self, hash: &BlobHash) -> bool {
        self.pins.contains(hash)
    }

    /// Number of process-local pins.
    #[must_use]
    pub fn pinned_count(&self) -> usize {
        self.pins.len()
    }

    fn blob_path(&self, hash: &BlobHash) -> PathBuf {
        let hex = blob_hash_hex(hash);
        self.blobs_dir.join(&hex[..2]).join(hex)
    }

    fn temp_path(parent: &Path, hash: &BlobHash) -> PathBuf {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        parent.join(format!(".{}.{}.tmp", blob_hash_hex(hash), counter))
    }
}

/// Errors returned by [`DiskTier`].
#[derive(Debug, Error)]
pub enum DiskTierError {
    /// Content hash verification failed.
    #[error(transparent)]
    Cas(#[from] CasError),
    /// Filesystem operation failed.
    #[error("[CAS_IO] {operation} {path}: {source}")]
    Io {
        /// Filesystem operation being performed.
        operation: &'static str,
        /// Path associated with the failure.
        path: PathBuf,
        /// Source I/O error.
        #[source]
        source: io::Error,
    },
    /// Blob path did not match the tier layout.
    #[error("[CAS_INVALID_BLOB_PATH] {path}")]
    InvalidBlobPath {
        /// Invalid path.
        path: PathBuf,
    },
}

impl DiskTierError {
    fn io(operation: &'static str, path: &Path, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.to_path_buf(),
            source,
        }
    }

    fn invalid_blob_path(path: PathBuf) -> Self {
        Self::InvalidBlobPath { path }
    }
}

fn blob_hash_hex(hash: &BlobHash) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash.as_bytes() {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn blob_hash_from_hex(input: &str) -> Option<BlobHash> {
    if input.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in input.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(chunk[0])?;
        let low = hex_nibble(chunk[1])?;
        bytes[index] = (high << 4) | low;
    }
    Some(BlobHash::from_bytes(bytes))
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
