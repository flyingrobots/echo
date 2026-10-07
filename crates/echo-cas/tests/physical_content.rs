// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Complete-object conformance and quarantine faults.
#[path = "common/physical_content.rs"]
mod common;
use echo_cas::{
    blob_hash,
    physical_content::{
        reconstruct_quarantined, ContentError, ContentTarget, MemoryContentDestination,
        PhysicalContentBackend, PhysicalContentView,
    },
    DiskTier, MemoryTier,
};
use std::fs;
use std::io;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn memory_complete_object_conformance() -> TestResult {
    common::conformance(&mut MemoryTier::new())
}
#[test]
fn disk_complete_object_conformance_and_corruption_quarantine() -> TestResult {
    let cleanup = fresh_disk_fixture()?;
    let root = &cleanup.0;
    let mut backend = DiskTier::open(root)?;
    common::conformance(&mut backend)?;
    let bytes = b"corrupt me";
    let hash = backend.put(bytes)?;
    let hex = hash.to_string();
    let path = root.join("blobs").join(&hex[..2]).join(hex);
    fs::write(&path, b"corrupt xx")?;
    let target = ContentTarget { hash, length: 10 };
    let mut destination = MemoryContentDestination::new(b"prior".to_vec(), 16)?;
    assert!(matches!(
        backend.content_view().reconstruct(target, &mut destination),
        Err(ContentError::Mismatch)
    ));
    assert_eq!(destination.visible(), b"prior");
    // Publication I/O failure cannot produce a publication receipt.
    fs::remove_file(&path)?;
    fs::create_dir(&path)?;
    let staged = echo_cas::physical_content::StagedContent::read_expected(
        target,
        &mut io::Cursor::new(bytes),
        10,
    )?;
    assert!(matches!(
        backend.publish_content(staged),
        Err(ContentError::Disk(_))
    ));
    Ok(())
}
fn fresh_disk_fixture() -> io::Result<Cleanup> {
    for slot in 0..1024 {
        let root = std::env::temp_dir().join(format!(
            "echo-physical-content-{}-{slot}",
            std::process::id()
        ));
        match fs::create_dir(&root) {
            Ok(()) => return Ok(Cleanup(root)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no fresh disk fixture slot",
    ))
}

struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn prefix_and_ignored_writer_failure_never_reach_visible_output() -> TestResult {
    let target = ContentTarget {
        hash: blob_hash(b"good"),
        length: 4,
    };
    let mut destination = MemoryContentDestination::new(b"old".to_vec(), 4)?;
    let error = reconstruct_quarantined(target, &mut destination, |output| {
        output.write_all(b"go")?;
        Err(io::Error::from(io::ErrorKind::BrokenPipe).into())
    });
    assert!(matches!(error, Err(ContentError::Io(e)) if e.kind() == io::ErrorKind::BrokenPipe));
    assert_eq!(destination.visible(), b"old");
    let error = reconstruct_quarantined(target, &mut destination, |output| {
        output.write_all(b"good")?;
        let _ = output.write_all(b"overflow");
        Ok(())
    });
    assert!(matches!(error, Err(ContentError::Mismatch)));
    assert_eq!(destination.visible(), b"old");
    Ok(())
}

#[test]
fn impossible_staging_budget_does_not_read_source() {
    struct UnreadSource(bool);
    impl std::io::Read for UnreadSource {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            self.0 = true;
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }
    let mut source = UnreadSource(false);
    let target = ContentTarget {
        hash: blob_hash(b"good"),
        length: 4,
    };
    assert!(matches!(
        echo_cas::physical_content::StagedContent::read_expected(target, &mut source, 3),
        Err(ContentError::ResourceLimit)
    ));
    assert!(!source.0);
}

#[test]
fn overlong_reconstruction_is_mismatch_at_any_sufficient_budget() -> TestResult {
    let target = ContentTarget {
        hash: blob_hash(b"good"),
        length: 4,
    };
    for limit in [4, 8] {
        let mut destination = MemoryContentDestination::new(b"old".to_vec(), limit)?;
        let result = reconstruct_quarantined(target, &mut destination, |output| {
            output.write_all(b"goodx")?;
            Ok(())
        });
        assert!(matches!(result, Err(ContentError::Mismatch)));
        assert_eq!(destination.visible(), b"old");
    }
    Ok(())
}
