// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Semantic retention tests above the content-only CAS layer.

use echo_cas::{
    blob_hash, MemoryTier, RetainedBlobIndex, RetainedBlobRole, RetentionError,
    SemanticBlobCoordinate,
};

fn coordinate(role: RetainedBlobRole, semantic_seed: u8) -> SemanticBlobCoordinate {
    SemanticBlobCoordinate {
        namespace: "contract:toy-counter".to_owned(),
        schema_hash_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_owned(),
        artifact_hash_hex: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_owned(),
        role,
        semantic_digest: [semantic_seed; 32],
    }
}

#[test]
fn retained_contract_artifact_loads_by_hash_and_semantic_coordinate() -> Result<(), String> {
    let mut blobs = MemoryTier::new();
    let mut index = RetainedBlobIndex::default();
    let bytes = b"generated contract artifact bytes";
    let coord = coordinate(RetainedBlobRole::ContractArtifact, 1);

    let descriptor = index.retain(&mut blobs, coord.clone(), bytes);

    assert_eq!(descriptor.content_hash, blob_hash(bytes));
    assert_eq!(descriptor.byte_len, bytes.len() as u64);
    let by_hash = index
        .load_by_hash(&blobs, descriptor.content_hash)
        .map_err(|err| format!("load by hash failed: {err:?}"))?;
    assert_eq!(by_hash.as_ref(), bytes);
    let retained = index
        .load(&blobs, &coord)
        .map_err(|err| format!("load by semantic coordinate failed: {err:?}"))?;
    assert_eq!(retained.descriptor, descriptor);
    assert_eq!(retained.bytes.as_ref(), bytes);
    Ok(())
}

#[test]
fn same_bytes_under_different_semantic_coordinates_do_not_alias() -> Result<(), String> {
    let mut blobs = MemoryTier::new();
    let mut index = RetainedBlobIndex::default();
    let bytes = b"same retained bytes";
    let artifact = coordinate(RetainedBlobRole::ContractArtifact, 2);
    let reading = coordinate(RetainedBlobRole::ReadingPayload, 3);

    let artifact_descriptor = index.retain(&mut blobs, artifact.clone(), bytes);
    let reading_descriptor = index.retain(&mut blobs, reading.clone(), bytes);

    assert_eq!(
        artifact_descriptor.content_hash,
        reading_descriptor.content_hash
    );
    assert_ne!(
        artifact_descriptor.coordinate,
        reading_descriptor.coordinate
    );
    let loaded_artifact = index
        .load(&blobs, &artifact)
        .map_err(|err| format!("artifact semantic load failed: {err:?}"))?;
    let loaded_reading = index
        .load(&blobs, &reading)
        .map_err(|err| format!("reading semantic load failed: {err:?}"))?;
    assert_eq!(loaded_artifact.descriptor, artifact_descriptor);
    assert_eq!(loaded_reading.descriptor, reading_descriptor);
    Ok(())
}

#[test]
fn missing_semantic_coordinate_returns_typed_obstruction() -> Result<(), String> {
    let blobs = MemoryTier::new();
    let index = RetainedBlobIndex::default();
    let coord = coordinate(RetainedBlobRole::ReadingEnvelope, 4);

    let err = index
        .load(&blobs, &coord)
        .err()
        .ok_or_else(|| "missing semantic coordinate unexpectedly loaded".to_owned())?;

    assert_eq!(
        err,
        RetentionError::MissingSemanticCoordinate { coordinate: coord }
    );
    Ok(())
}

#[test]
fn semantic_lookup_reads_bounded_byte_range_under_budget() -> Result<(), String> {
    let mut blobs = MemoryTier::new();
    let mut index = RetainedBlobIndex::default();
    let coord = coordinate(RetainedBlobRole::ReadingPayload, 5);
    index.retain(&mut blobs, coord.clone(), b"abcdefghijklmnopqrstuvwxyz");

    let range = index
        .load_range(&blobs, &coord, 4, 6, 6)
        .map_err(|err| format!("bounded range lookup failed: {err:?}"))?;

    assert_eq!(range.offset, 4);
    assert_eq!(range.bytes.as_ref(), b"efghij");
    assert_eq!(range.descriptor.coordinate, coord);
    Ok(())
}

#[test]
fn semantic_range_lookup_returns_budget_obstruction() -> Result<(), String> {
    let mut blobs = MemoryTier::new();
    let mut index = RetainedBlobIndex::default();
    let coord = coordinate(RetainedBlobRole::ReadingPayload, 6);
    index.retain(&mut blobs, coord.clone(), b"bounded payload");

    let err = index
        .load_range(&blobs, &coord, 0, 8, 4)
        .err()
        .ok_or_else(|| "over-budget range unexpectedly loaded".to_owned())?;

    assert_eq!(
        err,
        RetentionError::RangeExceedsBudget {
            requested_bytes: 8,
            max_bytes: 4,
        }
    );
    Ok(())
}

#[test]
fn semantic_lookup_requires_exact_coordinate_even_when_content_hash_matches() -> Result<(), String>
{
    let mut blobs = MemoryTier::new();
    let mut index = RetainedBlobIndex::default();
    let coord = coordinate(RetainedBlobRole::ReadingPayload, 7);
    let wrong = coordinate(RetainedBlobRole::ReadingPayload, 8);
    let descriptor = index.retain(&mut blobs, coord, b"same content");

    assert_eq!(
        index
            .load_by_hash(&blobs, descriptor.content_hash)
            .map_err(|err| format!("load by content hash failed: {err:?}"))?
            .as_ref(),
        b"same content"
    );
    let err = index
        .load(&blobs, &wrong)
        .err()
        .ok_or_else(|| "wrong semantic coordinate unexpectedly loaded".to_owned())?;

    assert_eq!(
        err,
        RetentionError::MissingSemanticCoordinate { coordinate: wrong }
    );
    Ok(())
}
