// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Filesystem-backed retained blob tier tests.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use echo_cas::{BlobHash, CasError, DiskTier, DiskTierError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> io::Result<Self> {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "echo-cas-disk-tier-{}-{counter}",
            std::process::id()
        ));
        match fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn missing_blob(message: &'static str) -> io::Error {
    io::Error::other(message)
}

#[test]
fn disk_tier_persists_bytes_across_reopen() -> TestResult {
    let root = TestDir::new()?;
    let bytes = b"durable retained bytes";
    let hash = {
        let tier = DiskTier::open(root.path())?;
        tier.put(bytes)?
    };

    let reopened = DiskTier::open(root.path())?;
    let stored = reopened
        .get(&hash)?
        .ok_or_else(|| missing_blob("reopened disk tier did not contain retained bytes"))?;

    assert_eq!(stored.as_ref(), bytes);
    assert!(reopened.has(&hash)?);
    Ok(())
}

#[test]
fn disk_tier_put_verified_rejects_mismatch_without_mutating_store() -> TestResult {
    let root = TestDir::new()?;
    let tier = DiskTier::open(root.path())?;
    let original = b"original retained bytes";
    let original_hash = tier.put(original)?;
    let bad_hash = BlobHash::from_bytes([0xFF; 32]);

    let result = tier.put_verified(bad_hash, b"mismatched retained bytes");

    assert!(matches!(
        result,
        Err(DiskTierError::Cas(CasError::HashMismatch { expected, .. })) if expected == bad_hash
    ));
    assert!(!tier.has(&bad_hash)?);
    assert!(tier.get(&bad_hash)?.is_none());
    let stored = tier
        .get(&original_hash)?
        .ok_or_else(|| missing_blob("original blob disappeared after rejected write"))?;
    assert_eq!(stored.as_ref(), original);
    assert_eq!(tier.list()?, vec![original_hash]);
    Ok(())
}

#[test]
fn disk_tier_missing_blob_remains_absence() -> TestResult {
    let root = TestDir::new()?;
    let tier = DiskTier::open(root.path())?;
    let missing = BlobHash::from_bytes([0x55; 32]);

    assert!(!tier.has(&missing)?);
    assert!(tier.get(&missing)?.is_none());
    Ok(())
}

#[test]
fn disk_tier_list_returns_sorted_hashes() -> TestResult {
    let root = TestDir::new()?;
    let tier = DiskTier::open(root.path())?;
    let payloads: [&[u8]; 4] = [
        b"z-last".as_slice(),
        b"a-first".as_slice(),
        b"middle".as_slice(),
        b"another".as_slice(),
    ];
    let mut expected = Vec::new();
    for payload in payloads {
        expected.push(tier.put(payload)?);
    }
    expected.sort_unstable();
    expected.dedup();
    let first_hex = expected[0].to_string();
    let temp_shard = root.path().join("blobs").join(&first_hex[..2]);
    fs::write(temp_shard.join(".stale-write.tmp"), b"partial")?;

    assert_eq!(tier.list()?, expected);

    let reopened = DiskTier::open(root.path())?;
    assert_eq!(reopened.list()?, expected);
    Ok(())
}
