// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Contract tests for the Echo-owned file aperture.

use echo_file_aperture::{
    ContentAdmissionPosture, FileApertureError, FileContentDigest, FileContentProposal, FileSiteId,
    FileSiteIdentityPosture, HostFileFingerprint, HostFileIdentity, HostFileSnapshot,
    HostObservationPosture, InMemoryFileAperture, MaterializationVerificationPosture,
};

fn snapshot(path: &str, bytes: &[u8]) -> Result<HostFileSnapshot, FileApertureError> {
    HostFileSnapshot::new(
        HostFileIdentity::new(path.as_bytes(), None)?,
        bytes.to_vec(),
    )
}

fn identity_with_platform(
    path: &str,
    platform_identity: &[u8],
) -> Result<HostFileIdentity, FileApertureError> {
    HostFileIdentity::new(path.as_bytes(), Some(platform_identity.to_vec()))
}

#[test]
fn platform_identity_keeps_file_site_stable_across_path_move() -> Result<(), FileApertureError> {
    let before_move = identity_with_platform("/tmp/a/demo.txt", b"host-a:dev-1:inode-42")?;
    let after_move = identity_with_platform("/tmp/b/demo.txt", b"host-a:dev-1:inode-42")?;
    let before_resolution = before_move.site_resolution()?;
    let after_resolution = after_move.site_resolution()?;

    assert_eq!(
        before_resolution.file_site_id,
        after_resolution.file_site_id
    );
    assert_eq!(
        before_resolution.posture,
        FileSiteIdentityPosture::PlatformStable
    );
    assert_eq!(
        after_resolution.posture,
        FileSiteIdentityPosture::PlatformStable
    );
    Ok(())
}

#[test]
fn different_platform_identity_wins_over_same_path() -> Result<(), FileApertureError> {
    let first = identity_with_platform("/tmp/demo.txt", b"host-a:dev-1:inode-42")?;
    let second = identity_with_platform("/tmp/demo.txt", b"host-a:dev-1:inode-43")?;

    assert_ne!(first.site_id()?, second.site_id()?);
    assert_eq!(
        first.site_resolution()?.posture,
        FileSiteIdentityPosture::PlatformStable
    );
    Ok(())
}

#[test]
fn path_only_identity_is_explicitly_path_bound() -> Result<(), FileApertureError> {
    let first = HostFileIdentity::new(b"/tmp/demo.txt", None)?;
    let second = HostFileIdentity::new(b"/tmp/demo.txt", None)?;
    let resolution = first.site_resolution()?;

    assert_eq!(resolution.file_site_id, second.site_id()?);
    assert_eq!(resolution.posture, FileSiteIdentityPosture::PathBound);
    Ok(())
}

#[test]
fn path_only_move_creates_distinct_path_bound_sites() -> Result<(), FileApertureError> {
    let before_move = HostFileIdentity::new(b"/tmp/a/demo.txt", None)?;
    let after_move = HostFileIdentity::new(b"/tmp/b/demo.txt", None)?;

    assert_ne!(before_move.site_id()?, after_move.site_id()?);
    assert_eq!(
        before_move.site_resolution()?.posture,
        FileSiteIdentityPosture::PathBound
    );
    assert_eq!(
        after_move.site_resolution()?.posture,
        FileSiteIdentityPosture::PathBound
    );
    Ok(())
}

#[test]
fn platform_site_derivation_does_not_include_path_bytes() -> Result<(), FileApertureError> {
    let identity = identity_with_platform("/tmp/demo.txt", b"host-a:dev-1:inode-42")?;
    let platform_site = FileSiteId::from_platform_identity(b"host-a:dev-1:inode-42")?;
    let path_bound_site = FileSiteId::from_path_bound_evidence(b"/tmp/demo.txt")?;

    assert_eq!(identity.site_id()?, platform_site);
    assert_ne!(identity.site_id()?, path_bound_site);
    Ok(())
}

#[test]
fn host_observation_receipt_exposes_identity_posture() -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();
    let snapshot = HostFileSnapshot::new(
        identity_with_platform("/tmp/demo.txt", b"host-a:dev-1:inode-42")?,
        b"one".to_vec(),
    )?;

    let receipt = aperture.observe(snapshot)?;

    assert_eq!(
        receipt.site_identity_posture,
        FileSiteIdentityPosture::PlatformStable
    );
    assert_eq!(
        receipt.projection.site_identity_posture,
        FileSiteIdentityPosture::PlatformStable
    );
    Ok(())
}

#[test]
fn empty_platform_identity_is_invalid() {
    let error = HostFileIdentity::new(b"/tmp/demo.txt", Some(Vec::new()));

    assert!(matches!(
        error,
        Err(FileApertureError::EmptyPlatformIdentity)
    ));
}

#[test]
fn unknown_host_file_admits_initial_import_before_projection() -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();

    let receipt = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;

    assert_eq!(receipt.observation_id, 0);
    assert_eq!(receipt.posture, HostObservationPosture::InitialImport);
    assert_eq!(
        receipt.site_identity_posture,
        FileSiteIdentityPosture::PathBound
    );
    assert_eq!(
        receipt.projection.site_identity_posture,
        FileSiteIdentityPosture::PathBound
    );
    assert_eq!(receipt.projection.bytes, b"one");
    assert_eq!(
        receipt.projection.content_digest,
        receipt.fingerprint.content_digest
    );
    Ok(())
}

#[test]
fn forged_snapshot_fingerprint_cannot_override_observed_bytes() -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();
    let identity = HostFileIdentity::new(b"/tmp/demo.txt", None)?;
    let forged_metadata = echo_file_aperture::HostFileMetadata { byte_len: 3 };
    let forged_snapshot = HostFileSnapshot {
        identity,
        bytes: b"one".to_vec(),
        metadata: forged_metadata,
        fingerprint: HostFileFingerprint::from_parts(b"two", forged_metadata),
    };

    let receipt = aperture.observe(forged_snapshot)?;

    assert_eq!(receipt.projection.bytes, b"one");
    assert_eq!(
        receipt.projection.content_digest,
        FileContentDigest::for_bytes(b"one")
    );
    assert_eq!(
        receipt.fingerprint.content_digest,
        FileContentDigest::for_bytes(b"one")
    );
    Ok(())
}

#[test]
fn unchanged_host_file_records_no_change_projection() -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();
    let first = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;

    let second = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;

    assert_eq!(second.observation_id, 1);
    assert_eq!(second.posture, HostObservationPosture::Unchanged);
    assert_eq!(second.projection.basis, first.projection.basis);
    assert_eq!(second.projection.bytes, b"one");
    Ok(())
}

#[test]
fn changed_host_file_admits_external_change_transition() -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();
    let first = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;

    let second = aperture.observe(snapshot("/tmp/demo.txt", b"two")?)?;

    assert_eq!(second.posture, HostObservationPosture::ExternalChange);
    assert_ne!(second.projection.basis, first.projection.basis);
    assert_eq!(second.projection.bytes, b"two");
    Ok(())
}

#[test]
fn save_current_basis_admits_content_and_verifies_materialization() -> Result<(), FileApertureError>
{
    let mut aperture = InMemoryFileAperture::default();
    let initial = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;
    let proposal =
        FileContentProposal::new(initial.site_id, initial.projection.basis, b"three".to_vec())?;

    let admitted = aperture.propose_content(proposal)?;
    let materialized = aperture.verify_materialization(
        admitted.site_id,
        admitted.projection.basis,
        snapshot("/tmp/demo.txt", b"three")?,
    )?;

    assert_eq!(admitted.posture, ContentAdmissionPosture::AdmittedChange);
    assert_eq!(admitted.projection.bytes, b"three");
    assert_eq!(
        materialized.posture,
        MaterializationVerificationPosture::Verified
    );
    assert_eq!(materialized.expected_digest, materialized.observed_digest);
    Ok(())
}

#[test]
fn stale_basis_obstructs_save_without_changing_projection() -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();
    let initial = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;
    let changed = aperture.observe(snapshot("/tmp/demo.txt", b"two")?)?;
    let proposal =
        FileContentProposal::new(initial.site_id, initial.projection.basis, b"three".to_vec())?;

    let error = aperture.propose_content(proposal);
    let projection = aperture.projection(changed.site_id)?;

    assert!(matches!(error, Err(FileApertureError::StaleBasis { .. })));
    assert_eq!(projection.basis, changed.projection.basis);
    assert_eq!(projection.bytes, b"two");
    Ok(())
}

#[test]
fn verification_digest_mismatch_returns_materialization_obstruction(
) -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();
    let initial = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;
    let proposal =
        FileContentProposal::new(initial.site_id, initial.projection.basis, b"three".to_vec())?;
    let admitted = aperture.propose_content(proposal)?;

    let materialized = aperture.verify_materialization(
        admitted.site_id,
        admitted.projection.basis,
        snapshot("/tmp/demo.txt", b"not-three")?,
    )?;

    assert_eq!(
        materialized.posture,
        MaterializationVerificationPosture::DigestMismatch
    );
    assert_ne!(materialized.expected_digest, materialized.observed_digest);
    Ok(())
}
