// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-owned host file aperture contract.
//!
//! A host file is not Echo state. It is an observed boundary artifact and a
//! materialization target. This crate owns the deterministic file-aperture
//! contract used by Echo consumers such as editors and projection adapters.
//!
//! The first implementation slice is in-memory: callers supply host bytes and
//! path evidence, while the aperture records file-site basis, admitted content,
//! observation posture, and materialization verification posture. Later slices
//! should bind these records to WAL/WSC retention and Echo scheduler receipts.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

/// Number of bytes in Echo file-aperture digests.
const DIGEST_LEN: usize = 32;

/// Domain prefix for file-site identity.
const FILE_SITE_DOMAIN: &[u8] = b"echo:file-aperture:site:v1";
/// Domain prefix for host metadata fingerprints.
const HOST_METADATA_DOMAIN: &[u8] = b"echo:file-aperture:metadata:v1";
/// Domain prefix for basis tokens.
const BASIS_DOMAIN: &[u8] = b"echo:file-aperture:basis:v1";

/// Stable Echo identity for a file-like host artifact.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSiteId([u8; DIGEST_LEN]);

impl FileSiteId {
    /// Derives a file site id from host identity evidence.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::LengthOverflow`] if identity evidence is
    /// too large to length-prefix deterministically.
    pub fn for_identity(identity: &HostFileIdentity) -> Result<Self, FileApertureError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(FILE_SITE_DOMAIN);
        update_len_prefixed(&mut hasher, &identity.path_evidence)?;
        match &identity.platform_identity {
            Some(platform_identity) => {
                hasher.update(&[1]);
                update_len_prefixed(&mut hasher, platform_identity)?;
            }
            None => {
                hasher.update(&[0]);
            }
        }
        Ok(Self(*hasher.finalize().as_bytes()))
    }

    /// Constructs a file site id from raw digest bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_LEN]) -> Self {
        Self(bytes)
    }

    /// Returns raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_LEN] {
        &self.0
    }
}

impl fmt::Display for FileSiteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(f, &self.0)
    }
}

/// Content digest for a file projection.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileContentDigest([u8; DIGEST_LEN]);

impl FileContentDigest {
    /// Computes a content digest from exact file bytes.
    #[must_use]
    pub fn for_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    /// Constructs a content digest from raw digest bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_LEN]) -> Self {
        Self(bytes)
    }

    /// Returns raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_LEN] {
        &self.0
    }
}

impl fmt::Display for FileContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(f, &self.0)
    }
}

/// Echo-owned token naming the causal basis for a file projection.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileBasisToken([u8; DIGEST_LEN]);

impl FileBasisToken {
    /// Derives a basis token from a site id, generation, and content digest.
    #[must_use]
    pub fn derive(site_id: FileSiteId, generation: u64, digest: FileContentDigest) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(BASIS_DOMAIN);
        hasher.update(site_id.as_bytes());
        hasher.update(&generation.to_le_bytes());
        hasher.update(digest.as_bytes());
        Self(*hasher.finalize().as_bytes())
    }

    /// Constructs a basis token from raw digest bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_LEN]) -> Self {
        Self(bytes)
    }

    /// Returns raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_LEN] {
        &self.0
    }
}

impl fmt::Display for FileBasisToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(f, &self.0)
    }
}

/// Host-supplied evidence used to locate and identify file material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFileIdentity {
    /// Host path bytes or path-like evidence supplied by the caller.
    pub path_evidence: Vec<u8>,
    /// Optional platform file identity, such as device/inode or file id bytes.
    pub platform_identity: Option<Vec<u8>>,
}

impl HostFileIdentity {
    /// Builds host file identity evidence.
    ///
    /// Path evidence is required because it is the user's visible coordinate
    /// even when a platform identity is available.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::EmptyPathEvidence`] when `path_evidence`
    /// is empty, or [`FileApertureError::LengthOverflow`] when evidence is too
    /// large to length-prefix deterministically.
    pub fn new(
        path_evidence: impl Into<Vec<u8>>,
        platform_identity: Option<Vec<u8>>,
    ) -> Result<Self, FileApertureError> {
        let path_evidence = path_evidence.into();
        if path_evidence.is_empty() {
            return Err(FileApertureError::EmptyPathEvidence);
        }
        ensure_len_fits(path_evidence.len())?;
        if let Some(platform_identity) = &platform_identity {
            ensure_len_fits(platform_identity.len())?;
        }
        Ok(Self {
            path_evidence,
            platform_identity,
        })
    }

    /// Derives the Echo file site id for this host identity.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::LengthOverflow`] if identity evidence is
    /// too large to length-prefix deterministically.
    pub fn site_id(&self) -> Result<FileSiteId, FileApertureError> {
        FileSiteId::for_identity(self)
    }
}

/// Deterministic metadata retained with a host observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostFileMetadata {
    /// Exact observed byte length.
    pub byte_len: u64,
}

impl HostFileMetadata {
    /// Builds metadata for exact file bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::LengthOverflow`] if the byte length cannot
    /// be represented in Echo's deterministic metadata encoding.
    pub fn for_bytes(bytes: &[u8]) -> Result<Self, FileApertureError> {
        Ok(Self {
            byte_len: ensure_len_fits(bytes.len())?,
        })
    }

    /// Computes a deterministic metadata digest.
    #[must_use]
    pub fn digest(&self) -> [u8; DIGEST_LEN] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(HOST_METADATA_DOMAIN);
        hasher.update(&self.byte_len.to_le_bytes());
        *hasher.finalize().as_bytes()
    }
}

/// Fingerprint of observed host file content and metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostFileFingerprint {
    /// Digest of exact file bytes.
    pub content_digest: FileContentDigest,
    /// Exact observed byte length.
    pub byte_len: u64,
    /// Digest of deterministic host metadata retained by this slice.
    pub metadata_digest: [u8; DIGEST_LEN],
}

impl HostFileFingerprint {
    /// Builds a host fingerprint from exact bytes and metadata.
    #[must_use]
    pub fn from_parts(bytes: &[u8], metadata: HostFileMetadata) -> Self {
        Self {
            content_digest: FileContentDigest::for_bytes(bytes),
            byte_len: metadata.byte_len,
            metadata_digest: metadata.digest(),
        }
    }
}

/// Exact host file material supplied to the aperture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFileSnapshot {
    /// Host identity evidence for the observed material.
    pub identity: HostFileIdentity,
    /// Exact bytes observed from the host.
    pub bytes: Vec<u8>,
    /// Deterministic metadata for the observed material.
    pub metadata: HostFileMetadata,
    /// Content and metadata fingerprint for the observation.
    pub fingerprint: HostFileFingerprint,
}

impl HostFileSnapshot {
    /// Builds a host file snapshot from identity evidence and exact bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::LengthOverflow`] if the bytes are too long
    /// for deterministic metadata encoding.
    pub fn new(
        identity: HostFileIdentity,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<Self, FileApertureError> {
        let bytes = bytes.into();
        let metadata = HostFileMetadata::for_bytes(&bytes)?;
        let fingerprint = HostFileFingerprint::from_parts(&bytes, metadata);
        Ok(Self {
            identity,
            bytes,
            metadata,
            fingerprint,
        })
    }
}

/// Bounded Echo projection of file content at a basis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileContentProjection {
    /// Echo file site being projected.
    pub site_id: FileSiteId,
    /// Basis token for this exact projection.
    pub basis: FileBasisToken,
    /// Content digest for the projected bytes.
    pub content_digest: FileContentDigest,
    /// Exact projected byte length.
    pub byte_len: u64,
    /// Exact projected bytes.
    pub bytes: Vec<u8>,
}

/// Posture assigned to a host observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostObservationPosture {
    /// The file site had no prior causal basis and this observation imported it.
    InitialImport,
    /// The observed bytes matched the current Echo basis.
    Unchanged,
    /// The host bytes differed from the current Echo basis and were admitted
    /// as external drift.
    ExternalChange,
}

/// Receipt returned after accepting host file material into the aperture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFileObservationReceipt {
    /// Deterministic per-aperture observation sequence.
    pub observation_id: u64,
    /// File site named by the observation.
    pub site_id: FileSiteId,
    /// Observation posture.
    pub posture: HostObservationPosture,
    /// Host fingerprint observed by this receipt.
    pub fingerprint: HostFileFingerprint,
    /// Projection returned to the caller after reconciliation.
    pub projection: FileContentProjection,
}

/// Proposed file content against an explicit Echo basis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileContentProposal {
    /// Echo file site the proposal targets.
    pub site_id: FileSiteId,
    /// Basis token the caller claims to have edited from.
    pub basis: FileBasisToken,
    /// Desired file bytes.
    pub bytes: Vec<u8>,
}

impl FileContentProposal {
    /// Builds a file content proposal.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::LengthOverflow`] if the desired bytes are
    /// too long for deterministic metadata encoding.
    pub fn new(
        site_id: FileSiteId,
        basis: FileBasisToken,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<Self, FileApertureError> {
        let bytes = bytes.into();
        ensure_len_fits(bytes.len())?;
        Ok(Self {
            site_id,
            basis,
            bytes,
        })
    }
}

/// Posture assigned to an admitted content proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentAdmissionPosture {
    /// Proposal matched the current basis and did not advance content.
    Unchanged,
    /// Proposal changed content and advanced the file basis.
    AdmittedChange,
}

/// Receipt returned after admitting desired file content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileContentIntentReceipt {
    /// File site targeted by the content intent.
    pub site_id: FileSiteId,
    /// Content admission posture.
    pub posture: ContentAdmissionPosture,
    /// Basis token supplied by the caller.
    pub previous_basis: FileBasisToken,
    /// Projection after admission.
    pub projection: FileContentProjection,
}

/// Posture assigned to host materialization verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterializationVerificationPosture {
    /// Host material matched the admitted projection digest.
    Verified,
    /// Host material did not match the admitted projection digest.
    DigestMismatch,
}

/// Receipt returned after checking materialized host bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMaterializationReceipt {
    /// File site whose materialization was checked.
    pub site_id: FileSiteId,
    /// Basis token expected by the verification.
    pub basis: FileBasisToken,
    /// Expected content digest from Echo's admitted projection.
    pub expected_digest: FileContentDigest,
    /// Observed host content digest.
    pub observed_digest: FileContentDigest,
    /// Materialization verification posture.
    pub posture: MaterializationVerificationPosture,
}

/// Errors returned by the file aperture contract.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FileApertureError {
    /// Host identity did not include path evidence.
    #[error("host file identity path evidence is empty")]
    EmptyPathEvidence,
    /// Input length cannot be represented in deterministic file aperture
    /// encodings.
    #[error("file aperture input length overflows deterministic encoding")]
    LengthOverflow,
    /// The requested file site has no admitted basis.
    #[error("unknown file site {site_id}")]
    UnknownFileSite {
        /// Unknown file site id.
        site_id: FileSiteId,
    },
    /// A caller supplied an old or unrelated basis token.
    #[error("stale file basis for {site_id}: expected {expected}, got {actual}")]
    StaleBasis {
        /// File site whose basis check failed.
        site_id: FileSiteId,
        /// Current Echo basis token.
        expected: FileBasisToken,
        /// Caller-supplied basis token.
        actual: FileBasisToken,
    },
    /// Host materialization evidence named a different file site.
    #[error("host file identity mapped to {observed_site_id}, expected {expected_site_id}")]
    SiteIdentityMismatch {
        /// Expected file site id.
        expected_site_id: FileSiteId,
        /// Observed file site id.
        observed_site_id: FileSiteId,
    },
}

/// In-memory implementation of the Echo file aperture contract.
#[derive(Default, Debug)]
pub struct InMemoryFileAperture {
    next_observation_id: u64,
    sites: BTreeMap<FileSiteId, FileState>,
}

impl InMemoryFileAperture {
    /// Accepts host file material and returns a reconciled Echo projection.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::LengthOverflow`] if host identity evidence
    /// cannot be encoded deterministically.
    pub fn observe(
        &mut self,
        snapshot: HostFileSnapshot,
    ) -> Result<HostFileObservationReceipt, FileApertureError> {
        let site_id = snapshot.identity.site_id()?;
        let observation_id = self.next_observation_id;
        self.next_observation_id = self
            .next_observation_id
            .checked_add(1)
            .ok_or(FileApertureError::LengthOverflow)?;

        let fingerprint = snapshot.fingerprint;
        let posture = match self.sites.get_mut(&site_id) {
            Some(state) if state.content_digest == fingerprint.content_digest => {
                HostObservationPosture::Unchanged
            }
            Some(state) => {
                state.advance(fingerprint.content_digest, snapshot.bytes)?;
                HostObservationPosture::ExternalChange
            }
            None => {
                self.sites.insert(
                    site_id,
                    FileState::new(fingerprint.content_digest, snapshot.bytes),
                );
                HostObservationPosture::InitialImport
            }
        };

        let projection = self.projection(site_id)?;
        Ok(HostFileObservationReceipt {
            observation_id,
            site_id,
            posture,
            fingerprint,
            projection,
        })
    }

    /// Returns the current projection for a file site.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::UnknownFileSite`] when `site_id` has no
    /// admitted basis.
    pub fn projection(
        &self,
        site_id: FileSiteId,
    ) -> Result<FileContentProjection, FileApertureError> {
        let state = self
            .sites
            .get(&site_id)
            .ok_or(FileApertureError::UnknownFileSite { site_id })?;
        state.projection(site_id)
    }

    /// Admits desired content against the current file basis.
    ///
    /// This is the causal content-intent step before any host filesystem write.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::UnknownFileSite`] when the file site is not
    /// known, or [`FileApertureError::StaleBasis`] when the proposal basis does
    /// not match the current Echo basis.
    pub fn propose_content(
        &mut self,
        proposal: FileContentProposal,
    ) -> Result<FileContentIntentReceipt, FileApertureError> {
        let state =
            self.sites
                .get_mut(&proposal.site_id)
                .ok_or(FileApertureError::UnknownFileSite {
                    site_id: proposal.site_id,
                })?;
        let current_basis = state.basis(proposal.site_id);
        if current_basis != proposal.basis {
            return Err(FileApertureError::StaleBasis {
                site_id: proposal.site_id,
                expected: current_basis,
                actual: proposal.basis,
            });
        }

        let proposed_digest = FileContentDigest::for_bytes(&proposal.bytes);
        let posture = if proposed_digest == state.content_digest {
            ContentAdmissionPosture::Unchanged
        } else {
            state.advance(proposed_digest, proposal.bytes)?;
            ContentAdmissionPosture::AdmittedChange
        };
        let projection = state.projection(proposal.site_id)?;
        Ok(FileContentIntentReceipt {
            site_id: proposal.site_id,
            posture,
            previous_basis: current_basis,
            projection,
        })
    }

    /// Verifies that host material matches an admitted Echo projection.
    ///
    /// # Errors
    ///
    /// Returns [`FileApertureError::UnknownFileSite`] when the site is not
    /// known, [`FileApertureError::StaleBasis`] when `basis` is not current, or
    /// [`FileApertureError::SiteIdentityMismatch`] when `snapshot` names a
    /// different file site.
    pub fn verify_materialization(
        &self,
        site_id: FileSiteId,
        basis: FileBasisToken,
        snapshot: HostFileSnapshot,
    ) -> Result<FileMaterializationReceipt, FileApertureError> {
        let observed_site_id = snapshot.identity.site_id()?;
        if observed_site_id != site_id {
            return Err(FileApertureError::SiteIdentityMismatch {
                expected_site_id: site_id,
                observed_site_id,
            });
        }

        let state = self
            .sites
            .get(&site_id)
            .ok_or(FileApertureError::UnknownFileSite { site_id })?;
        let current_basis = state.basis(site_id);
        if current_basis != basis {
            return Err(FileApertureError::StaleBasis {
                site_id,
                expected: current_basis,
                actual: basis,
            });
        }

        let expected_digest = state.content_digest;
        let observed_digest = snapshot.fingerprint.content_digest;
        let posture = if expected_digest == observed_digest {
            MaterializationVerificationPosture::Verified
        } else {
            MaterializationVerificationPosture::DigestMismatch
        };
        Ok(FileMaterializationReceipt {
            site_id,
            basis,
            expected_digest,
            observed_digest,
            posture,
        })
    }
}

#[derive(Clone, Debug)]
struct FileState {
    generation: u64,
    content_digest: FileContentDigest,
    bytes: Vec<u8>,
}

impl FileState {
    fn new(content_digest: FileContentDigest, bytes: Vec<u8>) -> Self {
        Self {
            generation: 0,
            content_digest,
            bytes,
        }
    }

    fn advance(
        &mut self,
        content_digest: FileContentDigest,
        bytes: Vec<u8>,
    ) -> Result<(), FileApertureError> {
        ensure_len_fits(bytes.len())?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(FileApertureError::LengthOverflow)?;
        self.content_digest = content_digest;
        self.bytes = bytes;
        Ok(())
    }

    fn basis(&self, site_id: FileSiteId) -> FileBasisToken {
        FileBasisToken::derive(site_id, self.generation, self.content_digest)
    }

    fn projection(&self, site_id: FileSiteId) -> Result<FileContentProjection, FileApertureError> {
        Ok(FileContentProjection {
            site_id,
            basis: self.basis(site_id),
            content_digest: self.content_digest,
            byte_len: ensure_len_fits(self.bytes.len())?,
            bytes: self.bytes.clone(),
        })
    }
}

fn ensure_len_fits(len: usize) -> Result<u64, FileApertureError> {
    u64::try_from(len).map_err(|_error| FileApertureError::LengthOverflow)
}

fn update_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) -> Result<(), FileApertureError> {
    let len = ensure_len_fits(bytes.len())?;
    hasher.update(&len.to_le_bytes());
    hasher.update(bytes);
    Ok(())
}

fn write_hex(f: &mut fmt::Formatter<'_>, bytes: &[u8; DIGEST_LEN]) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}
