// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Optional non-durable physical backend, with private Keep coordinates.
use crate::IdentityBinding;
use echo_cas::{
    BlobHash,
    physical_content::{
        ContentError, ContentReceipt, ContentTarget, PhysicalContentBackend, PhysicalContentView,
        StagedContent, TransactionalContentDestination, reconstruct_quarantined,
    },
};
use keep::{
    IngestionError, LayoutEntryLimit, LayoutId, LayoutValidationError, PublishError,
    ReferenceStore, ReferenceStoreCapacity,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::Cursor;

struct Binding {
    identity: IdentityBinding,
    layout: LayoutId,
}

/// Capacity-bounded experimental adapter over Keep's in-memory ReferenceStore.
///
/// Process death loses both store and bindings. No receipt establishes crash
/// durability, retention, generation, complete-view absence or causal authority.
/// The backend is unavailable unless the `reference-adapter` feature is enabled.
pub struct KeepReferenceAdapter {
    store: ReferenceStore,
    bindings: BTreeMap<BlobHash, Binding>,
    object_limit: usize,
    object_count: usize,
    layout_limit: LayoutEntryLimit,
}
impl KeepReferenceAdapter {
    /// Creates an empty backend with explicit physical payload, per-object,
    /// object-count and per-layout entry limits. Metadata is bounded by the
    /// object-count and layout-entry limits, not the physical payload capacity.
    ///
    /// # Errors
    /// Returns ResourceLimit if the layout limit exceeds the supported protocol.
    pub fn new(
        physical_bytes: usize,
        object_bytes: usize,
        objects: usize,
        layout_entries: u32,
    ) -> Result<Self, ContentError> {
        let layout_limit =
            LayoutEntryLimit::new(layout_entries).map_err(|_| ContentError::ResourceLimit)?;
        Ok(Self {
            store: ReferenceStore::new(ReferenceStoreCapacity::new(physical_bytes)),
            bindings: BTreeMap::new(),
            object_limit: object_bytes,
            object_count: objects,
            layout_limit,
        })
    }
}

/// An immutable borrowed Keep capability without a durable-generation claim.
pub struct KeepReferenceView<'a>(&'a KeepReferenceAdapter);
impl PhysicalContentView for KeepReferenceView<'_> {
    fn reconstruct(
        &self,
        target: ContentTarget,
        destination: &mut dyn TransactionalContentDestination,
    ) -> Result<ContentReceipt, ContentError> {
        reconstruct_quarantined(target, destination, |output| {
            let binding = self
                .0
                .bindings
                .get(&target.hash)
                .ok_or(ContentError::CapabilityUnavailable)?;
            if binding.identity.echo_identity() != target.hash
                || binding.identity.length() != target.length
            {
                return Err(ContentError::Mismatch);
            }
            let receipt = self
                .0
                .store
                .reconstruct(binding.identity.keep, output)
                .map_err(|error| backend_failure("reconstruction", error))?;
            if receipt.target() != binding.identity.keep
                || receipt.layout_id() != binding.layout
                || receipt.bytes_written().get() != target.length
            {
                return Err(ContentError::Mismatch);
            }
            // Shared quarantine independently verifies Echo identity and length
            // before a complete handle can reach destination promotion.
            Ok(())
        })
    }
}
impl PhysicalContentBackend for KeepReferenceAdapter {
    type View<'a> = KeepReferenceView<'a>;
    fn content_view(&self) -> Self::View<'_> {
        KeepReferenceView(self)
    }
    fn publish_content(&mut self, staged: StagedContent) -> Result<ContentReceipt, ContentError> {
        let target = staged.target();
        if usize::try_from(target.length)
            .ok()
            .is_none_or(|length| length > self.object_limit)
        {
            return Err(ContentError::ResourceLimit);
        }
        if !self.bindings.contains_key(&target.hash) && self.bindings.len() >= self.object_count {
            return Err(ContentError::ResourceLimit);
        }
        let content = staged.into_verified();
        let identity =
            IdentityBinding::from_source(&mut Cursor::new(content.bytes()), target.length)
                .map_err(|error| backend_failure("identity verification", error))?;
        if identity.echo_identity() != target.hash || identity.length() != target.length {
            return Err(ContentError::Mismatch);
        }
        if self
            .bindings
            .get(&target.hash)
            .is_some_and(|existing| existing.identity != identity)
        {
            return Err(ContentError::Mismatch);
        }
        let staged = self
            .store
            .stage_expected(
                &mut Cursor::new(content.bytes()),
                identity.keep,
                self.layout_limit,
            )
            .map_err(ingestion_error)?;
        let layout = staged.layout_id();
        if staged.target() != identity.keep {
            return Err(ContentError::Mismatch);
        }
        let receipt = staged.commit(&mut self.store).map_err(publication_error)?;
        if receipt.target() != identity.keep || receipt.layout_id() != layout {
            return Err(ContentError::Mismatch);
        }
        self.bindings
            .insert(target.hash, Binding { identity, layout });
        Ok(ContentReceipt::from_verified(&content))
    }
}
fn ingestion_error(error: IngestionError) -> ContentError {
    match error {
        IngestionError::CapacityExceeded { .. }
        | IngestionError::Allocation { .. }
        | IngestionError::Layout(
            LayoutValidationError::EntryLimitExceeded { .. }
            | LayoutValidationError::Allocation { .. },
        ) => ContentError::ResourceLimit,
        error => backend_failure("staging", error),
    }
}

fn publication_error(error: PublishError) -> ContentError {
    match error {
        PublishError::CapacityExceeded { .. } => ContentError::ResourceLimit,
        error => backend_failure("publication", error),
    }
}

// Keep coordinates and concrete types cannot escape through error formatting,
// downcast or the public source chain. The original cause remains privately
// owned; ordinary Echo staging and destination I/O errors retain their API.
struct BackendFailure {
    operation: &'static str,
    _cause: Box<dyn Error + Send + Sync>,
}
impl fmt::Debug for BackendFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeepBackendFailure")
            .field("operation", &self.operation)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for BackendFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Keep reference {} failed", self.operation)
    }
}
impl Error for BackendFailure {}
fn backend_failure(
    operation: &'static str,
    cause: impl Error + Send + Sync + 'static,
) -> ContentError {
    ContentError::Backend(Box::new(BackendFailure {
        operation,
        _cause: Box::new(cause),
    }))
}

#[cfg(test)]
mod tests;
