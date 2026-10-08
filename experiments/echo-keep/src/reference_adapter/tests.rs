// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::*;
use echo_cas::{MemoryTier, blob_hash, physical_content::MemoryContentDestination};
use keep::{LayoutDecodePolicy, ReconstructionError};
use std::io::{self, Read};

// The shared source belongs to the root Rust 1.90 / edition 2021 formatter.
#[rustfmt::skip]
#[path = "../../../../crates/echo-cas/tests/common/physical_content.rs"]
mod common;
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn target(bytes: &[u8]) -> ContentTarget {
    ContentTarget {
        hash: blob_hash(bytes),
        length: bytes.len() as u64,
    }
}
fn staged(bytes: &[u8]) -> Result<StagedContent, ContentError> {
    StagedContent::read_expected(target(bytes), &mut Cursor::new(bytes), bytes.len())
}
fn adapter() -> Result<KeepReferenceAdapter, ContentError> {
    KeepReferenceAdapter::new(1_048_576, 1_048_576, 8, 64)
}
fn destination() -> Result<MemoryContentDestination, ContentError> {
    MemoryContentDestination::new(b"prior".to_vec(), 1_048_576)
}
#[test]
fn keep_and_existing_memory_share_the_exact_contract() -> TestResult {
    common::conformance(&mut adapter()?)?;
    common::conformance(&mut MemoryTier::new())?;
    Ok(())
}
#[test]
fn publication_limits_leave_existing_objects_readable() -> TestResult {
    for mut backend in [
        KeepReferenceAdapter::new(3, 8, 8, 64)?,
        KeepReferenceAdapter::new(8, 3, 8, 64)?,
        KeepReferenceAdapter::new(8, 8, 0, 64)?,
        KeepReferenceAdapter::new(8, 8, 8, 0)?,
    ] {
        assert!(matches!(
            backend.publish_content(staged(b"abcd")?),
            Err(ContentError::ResourceLimit)
        ));
        assert!(backend.bindings.is_empty());
        let mut output = destination()?;
        assert!(matches!(
            backend
                .content_view()
                .reconstruct(target(b"abcd"), &mut output),
            Err(ContentError::CapabilityUnavailable)
        ));
        assert_eq!(output.visible(), b"prior");
    }
    let mut backend = KeepReferenceAdapter::new(4, 4, 1, 64)?;
    backend.publish_content(staged(b"good")?)?;
    // Repeated publication at the object-count cap remains lawful.
    backend.publish_content(staged(b"good")?)?;
    assert!(matches!(
        backend.publish_content(staged(b"next")?),
        Err(ContentError::ResourceLimit)
    ));
    let mut output = destination()?;
    backend
        .content_view()
        .reconstruct(target(b"good"), &mut output)?;
    assert_eq!(output.visible(), b"good");
    let mut capacity = KeepReferenceAdapter::new(4, 8, 8, 64)?;
    capacity.publish_content(staged(b"good")?)?;
    assert!(matches!(
        capacity.publish_content(staged(b"next")?),
        Err(ContentError::ResourceLimit)
    ));
    capacity
        .content_view()
        .reconstruct(target(b"good"), &mut output)?;
    assert_eq!(output.visible(), b"good");
    assert_eq!(capacity.bindings.len(), 1);
    assert!(matches!(
        KeepReferenceAdapter::new(1, 1, 1, u32::MAX),
        Err(ContentError::ResourceLimit)
    ));
    Ok(())
}
#[test]
fn private_coordinate_substitutions_never_promote_output() -> TestResult {
    for mutation in 0..4 {
        let mut backend = adapter()?;
        backend.publish_content(staged(b"good")?)?;
        backend.publish_content(staged(b"next")?)?;
        let other = backend
            .bindings
            .get(&target(b"next").hash)
            .ok_or("missing next")?;
        let other_keep = other.identity.keep;
        let other_layout = other.layout;
        let binding = backend
            .bindings
            .get_mut(&target(b"good").hash)
            .ok_or("missing good")?;
        match mutation {
            0 => binding.identity.keep = other_keep,
            1 => binding.identity.echo = target(b"next").hash,
            2 => binding.identity.length += 1,
            _ => binding.layout = other_layout,
        }
        let mut output = destination()?;
        assert!(matches!(
            backend
                .content_view()
                .reconstruct(target(b"good"), &mut output),
            Err(ContentError::Mismatch)
        ));
        assert_eq!(output.visible(), b"prior");
    }
    Ok(())
}
#[test]
fn losing_the_store_or_recreating_the_adapter_refuses_without_a_receipt() -> TestResult {
    let mut backend = adapter()?;
    backend.publish_content(staged(b"good")?)?;
    backend.store = ReferenceStore::new(ReferenceStoreCapacity::new(1_048_576));
    let mut output = destination()?;
    let error = backend
        .content_view()
        .reconstruct(target(b"good"), &mut output)
        .err()
        .ok_or("missing store accepted")?;
    let keep_coordinate = format!(
        "{:?}",
        backend
            .bindings
            .get(&target(b"good").hash)
            .ok_or("missing binding")?
            .identity
            .keep
    );
    assert!(
        matches!(error, ContentError::Backend(ref cause) if cause.downcast_ref::<ReconstructionError>().is_none() && cause.source().is_none())
    );
    assert!(!format!("{error:?}").contains(&keep_coordinate));
    assert!(!format!("{error}").contains(&keep_coordinate));
    assert_eq!(output.visible(), b"prior");
    drop(backend);
    let fresh = adapter()?;
    assert!(matches!(
        fresh
            .content_view()
            .reconstruct(target(b"good"), &mut output),
        Err(ContentError::CapabilityUnavailable)
    ));
    assert_eq!(output.visible(), b"prior");
    Ok(())
}
#[test]
fn real_keep_missing_chunks_and_malformed_layouts_stay_quarantined() -> TestResult {
    let store = ReferenceStore::new(ReferenceStoreCapacity::new(1_048_576));
    let pending = store.stage(&mut Cursor::new(b"good"), LayoutEntryLimit::MAXIMUM)?;
    let mut output = destination()?;
    let error = reconstruct_quarantined(target(b"good"), &mut output, |private| {
        let _receipt = store
            .reconstruct_admitted_layout(pending.layout(), private)
            .map_err(|e| ContentError::Backend(Box::new(e)))?;
        Ok(())
    })
    .err()
    .ok_or("missing chunks accepted")?;
    assert!(
        matches!(error, ContentError::Backend(ref cause) if matches!(cause.downcast_ref::<ReconstructionError>(), Some(ReconstructionError::ChunkMissing { .. })))
    );
    assert_eq!(output.visible(), b"prior");
    let mut record = pending.layout().encode_record()?.bytes().to_vec();
    record[0] ^= 1;
    let error = reconstruct_quarantined(target(b"good"), &mut output, |private| {
        let _receipt = store
            .reconstruct_record(
                &record,
                LayoutDecodePolicy::new(LayoutEntryLimit::MAXIMUM),
                private,
            )
            .map_err(|e| ContentError::Backend(Box::new(e)))?;
        Ok(())
    })
    .err()
    .ok_or("malformed layout accepted")?;
    assert!(
        matches!(error, ContentError::Backend(ref cause) if matches!(cause.downcast_ref::<ReconstructionError>(), Some(ReconstructionError::LayoutDecode(_))))
    );
    assert_eq!(output.visible(), b"prior");
    // These use upstream public APIs and the same quarantine helper; they do
    // not mutate inaccessible private chunk tables inside the production view.
    Ok(())
}
#[test]
fn interrupted_ingress_retries_before_explicit_publication() -> TestResult {
    struct Interrupted(bool, Cursor<Vec<u8>>);
    impl Read for Interrupted {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if !self.0 {
                self.0 = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            self.1.read(bytes)
        }
    }
    let mut source = Interrupted(false, Cursor::new(b"good".to_vec()));
    let content = StagedContent::read_expected(target(b"good"), &mut source, 4)?;
    let mut backend = adapter()?;
    assert!(backend.bindings.is_empty());
    backend.publish_content(content)?;
    let mut output = destination()?;
    let receipt = backend
        .content_view()
        .reconstruct(target(b"good"), &mut output)?;
    assert_eq!(output.visible(), b"good");
    assert!(!receipt.establishes_durability());
    assert!(!receipt.establishes_complete_view());
    Ok(())
}

#[test]
fn upstream_error_types_and_coordinates_remain_private() -> TestResult {
    let store = ReferenceStore::new(ReferenceStoreCapacity::new(8));
    let pending = store.stage(&mut Cursor::new(b"good"), LayoutEntryLimit::MAXIMUM)?;
    let keep = pending.target();
    let layout = pending.layout_id();
    let other = IdentityBinding::from_source(&mut Cursor::new(b"next"), 4)?.keep;
    for error in [
        ingestion_error(IngestionError::BlobIdentityMismatch {
            expected: keep,
            observed: other,
        }),
        publication_error(PublishError::ConflictingLayout { identity: layout }),
        backend_failure(
            "reconstruction",
            ReconstructionError::BlobMissing { requested: keep },
        ),
    ] {
        let ContentError::Backend(ref cause) = error else {
            return Err("missing operational error".into());
        };
        assert!(cause.downcast_ref::<IngestionError>().is_none());
        assert!(cause.downcast_ref::<PublishError>().is_none());
        assert!(cause.downcast_ref::<ReconstructionError>().is_none());
        assert!(cause.source().is_none());
        for coordinate in [
            format!("{keep:?}"),
            keep.to_string(),
            format!("{layout:?}"),
            layout.to_string(),
        ] {
            assert!(!format!("{error:?}").contains(&coordinate));
            assert!(!format!("{error}").contains(&coordinate));
        }
    }
    Ok(())
}
