// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for semantic retained-reading identity.

use warp_core::{
    AttachmentDescentPolicy, CoordinateAt, EchoCoordinate, EchoOptic, IntentFamilyId,
    OpticAperture, OpticApertureShape, OpticCapabilityId, OpticFocus, ProjectionVersion,
    ReadIdentity, ReadingBudgetPosture, ReadingResidualPosture, ReadingRightsPosture,
    RetainReadingRequest, RetainedReadingCache, RetainedReadingCodecId, RetainedReadingKey,
    RevealReadingRequest, WitnessBasis, WorldlineId, WorldlineTick,
};
use warp_core::{OpticObstructionKind, OpticReadBudget, ProvenanceRef};

fn worldline(seed: u8) -> WorldlineId {
    WorldlineId::from_bytes([seed; 32])
}

fn provenance(seed: u8, tick: u64) -> ProvenanceRef {
    ProvenanceRef {
        worldline_id: worldline(seed),
        worldline_tick: WorldlineTick::from_raw(tick),
        commit_hash: [seed.wrapping_add(1); 32],
    }
}

fn intent_family(seed: u8) -> IntentFamilyId {
    IntentFamilyId::from_bytes([seed; 32])
}

fn capability(seed: u8) -> OpticCapabilityId {
    OpticCapabilityId::from_bytes([seed; 32])
}

fn retained_codec(seed: u8) -> RetainedReadingCodecId {
    RetainedReadingCodecId::from_bytes([seed; 32])
}

fn coordinate(seed: u8, tick: u64) -> EchoCoordinate {
    EchoCoordinate::Worldline {
        worldline_id: worldline(seed),
        at: CoordinateAt::Tick(WorldlineTick::from_raw(tick)),
    }
}

fn aperture(shape: OpticApertureShape) -> OpticAperture {
    OpticAperture {
        shape,
        budget: OpticReadBudget {
            max_bytes: Some(256),
            max_nodes: Some(8),
            max_ticks: Some(1),
            max_attachments: Some(0),
        },
        attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
    }
}

fn witness_basis(seed: u8, tick: u64) -> WitnessBasis {
    let reference = provenance(seed, tick);
    WitnessBasis::ResolvedCommit {
        reference,
        state_root: [seed.wrapping_add(2); 32],
        commit_hash: reference.commit_hash,
    }
}

fn read_identity(seed: u8, coordinate: EchoCoordinate, aperture: OpticAperture) -> ReadIdentity {
    let focus = OpticFocus::Worldline {
        worldline_id: worldline(seed),
    };
    let optic = EchoOptic::new(
        focus.clone(),
        coordinate.clone(),
        ProjectionVersion::from_raw(1),
        None,
        intent_family(seed),
        capability(seed),
    );

    ReadIdentity::new(
        optic.optic_id,
        &focus,
        coordinate,
        &aperture,
        ProjectionVersion::from_raw(1),
        None,
        witness_basis(seed, 1),
        ReadingRightsPosture::KernelPublic,
        ReadingBudgetPosture::Bounded {
            max_payload_bytes: 256,
            payload_bytes: 12,
            max_witness_refs: 1,
            witness_refs: 1,
        },
        ReadingResidualPosture::Complete,
    )
}

#[test]
fn same_content_under_different_coordinate_gets_distinct_retained_keys() {
    let mut cache = RetainedReadingCache::default();
    let payload = b"same reading bytes".to_vec();
    let codec_id = retained_codec(7);
    let first_identity = read_identity(1, coordinate(1, 10), aperture(OpticApertureShape::Head));
    let second_identity = read_identity(1, coordinate(1, 11), aperture(OpticApertureShape::Head));

    let first = cache.retain_reading(RetainReadingRequest {
        read_identity: first_identity,
        codec_id,
        payload: payload.clone(),
    });
    let second = cache.retain_reading(RetainReadingRequest {
        read_identity: second_identity,
        codec_id,
        payload,
    });

    assert_eq!(
        first.descriptor.content_hash,
        second.descriptor.content_hash
    );
    assert_ne!(first.descriptor.key, second.descriptor.key);
    let same_content_keys = cache.keys_for_content_hash(first.descriptor.content_hash);
    assert_eq!(same_content_keys.len(), 2);
    assert!(same_content_keys.contains(&first.descriptor.key));
    assert!(same_content_keys.contains(&second.descriptor.key));
}

#[test]
fn same_content_under_different_aperture_gets_distinct_retained_keys() {
    let mut cache = RetainedReadingCache::default();
    let payload = b"same reading bytes".to_vec();
    let coordinate = coordinate(2, 10);
    let codec_id = retained_codec(8);
    let head_identity = read_identity(2, coordinate.clone(), aperture(OpticApertureShape::Head));
    let snapshot_identity = read_identity(
        2,
        coordinate,
        aperture(OpticApertureShape::SnapshotMetadata),
    );

    let head = cache.retain_reading(RetainReadingRequest {
        read_identity: head_identity,
        codec_id,
        payload: payload.clone(),
    });
    let snapshot = cache.retain_reading(RetainReadingRequest {
        read_identity: snapshot_identity,
        codec_id,
        payload,
    });

    assert_eq!(
        head.descriptor.content_hash,
        snapshot.descriptor.content_hash
    );
    assert_ne!(head.descriptor.key, snapshot.descriptor.key);
}

#[test]
fn content_hash_only_reveal_is_a_lookup_miss() -> Result<(), String> {
    let mut cache = RetainedReadingCache::default();
    let payload = b"retained payload".to_vec();
    let identity = read_identity(3, coordinate(3, 10), aperture(OpticApertureShape::Head));
    let retained = cache.retain_reading(RetainReadingRequest {
        read_identity: identity.clone(),
        codec_id: retained_codec(9),
        payload,
    });
    let content_hash_as_key = RetainedReadingKey::from_bytes(retained.descriptor.content_hash);

    let err = cache
        .reveal_reading(&RevealReadingRequest {
            key: content_hash_as_key,
            read_identity: identity,
        })
        .err()
        .ok_or_else(|| "content-hash-only reveal unexpectedly succeeded".to_owned())?;

    assert_eq!(err.kind, OpticObstructionKind::MissingRetainedReading);
    Ok(())
}

#[test]
fn reveal_requires_matching_read_identity() -> Result<(), String> {
    let mut cache = RetainedReadingCache::default();
    let payload = b"retained payload".to_vec();
    let identity = read_identity(4, coordinate(4, 10), aperture(OpticApertureShape::Head));
    let wrong_identity = read_identity(4, coordinate(4, 11), aperture(OpticApertureShape::Head));
    let retained = cache.retain_reading(RetainReadingRequest {
        read_identity: identity.clone(),
        codec_id: retained_codec(10),
        payload: payload.clone(),
    });

    let err = cache
        .reveal_reading(&RevealReadingRequest {
            key: retained.descriptor.key,
            read_identity: wrong_identity,
        })
        .err()
        .ok_or_else(|| "mismatched read identity unexpectedly revealed payload".to_owned())?;

    assert_eq!(err.kind, OpticObstructionKind::MissingRetainedReading);

    let revealed = cache
        .reveal_reading(&RevealReadingRequest {
            key: retained.descriptor.key,
            read_identity: identity,
        })
        .ok()
        .ok_or_else(|| "matching read identity failed to reveal payload".to_owned())?;

    assert_eq!(revealed.descriptor, retained.descriptor);
    assert_eq!(revealed.payload, payload);
    Ok(())
}
