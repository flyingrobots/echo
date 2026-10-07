// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Fallible complete-object physical-content operations.
//!
//! Private bounded staging precedes atomic destination promotion. Receipts bind
//! exact bytes and length; these adapters establish no authenticated absence,
//! pinned generation, retention, synchronization, or crash-durability evidence.

use crate::{blob_hash, BlobHash, BlobStore, DiskTier, DiskTierError, MemoryTier};
use std::io::{self, Read, Write};

/// An explicit Echo content identity and exact expected logical length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentTarget {
    /// Raw-content Echo BLAKE3 identity.
    pub hash: BlobHash,
    /// Exact logical byte count.
    pub length: u64,
}

/// Failure carries no complete content or absence claim.
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    /// Underlying I/O failed; the original cause is retained.
    #[error("physical content I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Existing filesystem CAS publication failed.
    #[error(transparent)]
    Disk(#[from] DiskTierError),
    /// Exact content hash or length did not match.
    #[error("physical content identity or length mismatch")]
    Mismatch,
    /// Explicit byte or allocation budget was exceeded.
    #[error("physical content resource limit")]
    ResourceLimit,
    /// The requested evidence or atomic output capability is unavailable.
    #[error("physical content capability unavailable")]
    CapabilityUnavailable,
    /// An adapter operation failed without a physical-content proposition.
    #[error("physical backend operation failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Bytes sealed only after exact Echo identity and length verification.
///
/// The sealed handle proves bytes, not storage or causal authority.
#[derive(Debug)]
pub struct VerifiedContent {
    target: ContentTarget,
    bytes: Vec<u8>,
}
impl VerifiedContent {
    /// Seals an owned bounded buffer after exact verification.
    ///
    /// # Errors
    /// Returns `Mismatch` for wrong length or hash.
    pub fn seal(target: ContentTarget, bytes: Vec<u8>) -> Result<Self, ContentError> {
        if u64::try_from(bytes.len()).ok() != Some(target.length)
            || blob_hash(&bytes) != target.hash
        {
            return Err(ContentError::Mismatch);
        }
        Ok(Self { target, bytes })
    }
    /// Returns the exact verified target.
    pub const fn target(&self) -> ContentTarget {
        self.target
    }
    /// Borrows the complete authenticated bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Transfers the sealed bytes to an atomic destination implementation.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A destination capability with all-or-nothing visible publication.
///
/// Implementations MUST leave prior visible output unchanged on promotion
/// failure and MUST publish the complete sealed handle atomically on success.
/// Ordinary prefix-writing `Write` sinks do not implement this contract.
pub trait TransactionalContentDestination {
    /// True only when atomic promotion is supported.
    fn supports_atomic_promotion(&self) -> bool;
    /// Maximum bytes allowed in private staging and visible output.
    fn byte_limit(&self) -> usize;
    /// Atomically replaces visible output with verified complete bytes.
    ///
    /// # Errors
    /// Returns an operational failure, preserving prior visible output.
    fn promote(&mut self, content: VerifiedContent) -> Result<(), ContentError>;
}

/// A bounded memory destination whose promotion replaces one owned buffer.
#[derive(Debug)]
pub struct MemoryContentDestination {
    visible: Vec<u8>,
    limit: usize,
}
impl MemoryContentDestination {
    /// Creates a destination with an explicit bound and prior visible bytes.
    ///
    /// # Errors
    /// Returns `ResourceLimit` when prior output exceeds the bound.
    pub fn new(visible: Vec<u8>, limit: usize) -> Result<Self, ContentError> {
        if visible.len() > limit {
            return Err(ContentError::ResourceLimit);
        }
        Ok(Self { visible, limit })
    }
    /// Borrows only application-visible bytes, never private staging.
    pub fn visible(&self) -> &[u8] {
        &self.visible
    }
}
impl TransactionalContentDestination for MemoryContentDestination {
    fn supports_atomic_promotion(&self) -> bool {
        true
    }
    fn byte_limit(&self) -> usize {
        self.limit
    }
    fn promote(&mut self, content: VerifiedContent) -> Result<(), ContentError> {
        if content.bytes.len() > self.limit {
            return Err(ContentError::ResourceLimit);
        }
        self.visible = content.into_bytes();
        Ok(())
    }
}

/// A complete-object success with explicitly unsupported durability evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentReceipt {
    target: ContentTarget,
}
impl ContentReceipt {
    /// Constructs an exact-byte receipt for an already sealed object.
    ///
    /// Adapters may emit it only after their named operation succeeds. It grants
    /// no storage, generation, absence, retention, or durability evidence.
    pub const fn from_verified(content: &VerifiedContent) -> Self {
        Self {
            target: content.target,
        }
    }
    /// Returns the target bound to the complete successful operation.
    pub const fn target(&self) -> ContentTarget {
        self.target
    }
    /// These initial adapters establish no synchronization or crash durability.
    pub const fn establishes_durability(&self) -> bool {
        false
    }
    /// These initial adapters establish no authenticated complete-view absence.
    pub const fn establishes_complete_view(&self) -> bool {
        false
    }
}

/// Invisible, bounded expected-ingestion work; dropping it publishes nothing.
#[derive(Debug)]
pub struct StagedContent(VerifiedContent);
impl StagedContent {
    /// Reads and seals a finite source under an explicit allocation byte bound.
    ///
    /// # Errors
    /// Returns source I/O, resource failure, or exact identity/length mismatch.
    pub fn read_expected(
        target: ContentTarget,
        source: &mut dyn Read,
        byte_limit: usize,
    ) -> Result<Self, ContentError> {
        let mut buffer = PrivateContentBuffer::new(byte_limit);
        io::copy(source, &mut buffer).map_err(|error| buffer.map_error(error))?;
        Ok(Self(VerifiedContent::seal(target, buffer.bytes)?))
    }
    /// Returns its exact already-verified target.
    pub const fn target(&self) -> ContentTarget {
        self.0.target
    }
    /// Transfers complete staged bytes into an adapter's private publication.
    pub fn into_verified(self) -> VerifiedContent {
        self.0
    }
}

/// Borrowed physical view; its concrete capability binds the source backend.
///
/// A view cannot outlive its store borrow. The existing disk view authenticates
/// bytes at read time; it does not certify a pinned filesystem generation.
pub trait PhysicalContentView {
    /// Reconstructs a complete object into a transactional destination.
    ///
    /// # Errors
    /// On any error, no new bytes are visible and no success receipt is emitted.
    fn reconstruct(
        &self,
        target: ContentTarget,
        destination: &mut dyn TransactionalContentDestination,
    ) -> Result<ContentReceipt, ContentError>;
}

/// Expected staged ingestion, explicit publication, and an immutable read view.
pub trait PhysicalContentBackend {
    /// Borrowed capability for complete-object reading.
    type View<'a>: PhysicalContentView
    where
        Self: 'a;
    /// Creates a view tied to this backend borrow.
    fn content_view(&self) -> Self::View<'_>;
    /// Publishes already verified staged content under this backend's posture.
    ///
    /// # Errors
    /// Returns backend I/O or resource failure without a publication receipt.
    fn publish_content(&mut self, content: StagedContent) -> Result<ContentReceipt, ContentError>;
}

/// Bounded private reconstruction and exact verification shared by adapters.
///
/// The callback receives only a private writer. A complete successful callback
/// is still rehashed before promotion. Its writer may contain an untrusted
/// prefix on failure, but that prefix is dropped without reaching the caller.
///
/// # Errors
/// Returns failure without changing visible output, including failed promotion.
pub fn reconstruct_quarantined(
    target: ContentTarget,
    destination: &mut dyn TransactionalContentDestination,
    reconstruct: impl FnOnce(&mut dyn Write) -> Result<(), ContentError>,
) -> Result<ContentReceipt, ContentError> {
    if !destination.supports_atomic_promotion() {
        return Err(ContentError::CapabilityUnavailable);
    }
    let limit = destination.byte_limit();
    if usize::try_from(target.length)
        .ok()
        .is_none_or(|n| n > limit)
    {
        return Err(ContentError::ResourceLimit);
    }
    let mut staging = PrivateContentBuffer::new(limit);
    if let Err(error) = reconstruct(&mut staging) {
        return Err(if staging.resource_failure {
            ContentError::ResourceLimit
        } else {
            error
        });
    }
    if staging.resource_failure {
        return Err(ContentError::ResourceLimit);
    }
    let verified = VerifiedContent::seal(target, staging.bytes)?;
    destination.promote(verified)?;
    Ok(ContentReceipt { target })
}

struct PrivateContentBuffer {
    bytes: Vec<u8>,
    limit: usize,
    resource_failure: bool,
}
impl PrivateContentBuffer {
    const fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            resource_failure: false,
        }
    }
    fn map_error(&self, error: io::Error) -> ContentError {
        if self.resource_failure {
            ContentError::ResourceLimit
        } else {
            ContentError::Io(error)
        }
    }
}
impl Write for PrivateContentBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self.bytes.len().checked_add(bytes.len());
        if length.is_none_or(|n| n > self.limit)
            || self.bytes.try_reserve_exact(bytes.len()).is_err()
        {
            self.resource_failure = true;
            return Err(io::Error::other("private content allocation limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Immutable in-process CAS read capability.
pub struct MemoryContentView<'a>(&'a MemoryTier);
impl PhysicalContentView for MemoryContentView<'_> {
    fn reconstruct(
        &self,
        target: ContentTarget,
        destination: &mut dyn TransactionalContentDestination,
    ) -> Result<ContentReceipt, ContentError> {
        reconstruct_quarantined(target, destination, |output| {
            let bytes = self
                .0
                .get(&target.hash)
                .ok_or(ContentError::CapabilityUnavailable)?;
            output.write_all(&bytes)?;
            Ok(())
        })
    }
}
impl PhysicalContentBackend for MemoryTier {
    type View<'a> = MemoryContentView<'a>;
    fn content_view(&self) -> Self::View<'_> {
        MemoryContentView(self)
    }
    fn publish_content(&mut self, content: StagedContent) -> Result<ContentReceipt, ContentError> {
        let target = content.target();
        self.put_verified(target.hash, content.0.bytes())
            .map_err(|_| ContentError::Mismatch)?;
        Ok(ContentReceipt { target })
    }
}

/// Borrowed disk CAS capability without a pinned-generation assertion.
pub struct DiskContentView<'a>(&'a DiskTier);
impl PhysicalContentView for DiskContentView<'_> {
    fn reconstruct(
        &self,
        target: ContentTarget,
        destination: &mut dyn TransactionalContentDestination,
    ) -> Result<ContentReceipt, ContentError> {
        reconstruct_quarantined(target, destination, |output| {
            let mut file = match std::fs::File::open(self.0.blob_path(&target.hash)) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return Err(ContentError::CapabilityUnavailable)
                }
                Err(error) => return Err(error.into()),
            };
            io::copy(&mut file, output)?;
            Ok(())
        })
    }
}
impl PhysicalContentBackend for DiskTier {
    type View<'a> = DiskContentView<'a>;
    fn content_view(&self) -> Self::View<'_> {
        DiskContentView(self)
    }
    fn publish_content(&mut self, content: StagedContent) -> Result<ContentReceipt, ContentError> {
        let target = content.target();
        self.put_verified(target.hash, content.0.bytes())?;
        Ok(ContentReceipt { target })
    }
}
