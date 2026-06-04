// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Contract tests for the Echo-owned file aperture.

use echo_file_aperture::{
    ContentAdmissionPosture, FileApertureError, FileContentProposal, HostFileIdentity,
    HostFileSnapshot, HostObservationPosture, InMemoryFileAperture,
    MaterializationVerificationPosture,
};

fn snapshot(path: &str, bytes: &[u8]) -> Result<HostFileSnapshot, FileApertureError> {
    HostFileSnapshot::new(
        HostFileIdentity::new(path.as_bytes(), None)?,
        bytes.to_vec(),
    )
}

#[test]
fn unknown_host_file_admits_initial_import_before_projection() -> Result<(), FileApertureError> {
    let mut aperture = InMemoryFileAperture::default();

    let receipt = aperture.observe(snapshot("/tmp/demo.txt", b"one")?)?;

    assert_eq!(receipt.observation_id, 0);
    assert_eq!(receipt.posture, HostObservationPosture::InitialImport);
    assert_eq!(receipt.projection.bytes, b"one");
    assert_eq!(
        receipt.projection.content_digest,
        receipt.fingerprint.content_digest
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
