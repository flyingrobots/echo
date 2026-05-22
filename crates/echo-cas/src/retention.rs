// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Semantic retention coordinates above the content-only CAS layer.

use std::collections::BTreeMap;
use std::sync::Arc;

use thiserror::Error;

use crate::{blob_hash, BlobHash, BlobStore};

/// Semantic role of a retained Echo blob.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedBlobRole {
    /// Generated contract artifact bytes.
    ContractArtifact,
    /// Scheduler receipt or receipt-adjacent material.
    ContractReceipt,
    /// Witness material.
    Witness,
    /// Reading payload bytes.
    ReadingPayload,
    /// Encoded reading envelope bytes.
    ReadingEnvelope,
    /// Generated observer artifact bytes.
    ObserverArtifact,
}

/// Semantic coordinate for a retained contract blob.
///
/// CAS identity remains content-only. This coordinate names the question the
/// retained bytes answer: contract namespace, schema, artifact, role, and a
/// caller-supplied semantic digest for the specific receipt, witness, reading,
/// or artifact coordinate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticBlobCoordinate {
    /// Contract or package namespace.
    pub namespace: String,
    /// Hex-encoded schema hash for the contract family.
    pub schema_hash_hex: String,
    /// Hex-encoded package/artifact hash.
    pub artifact_hash_hex: String,
    /// Retained blob role.
    pub role: RetainedBlobRole,
    /// Domain-separated semantic coordinate digest owned by the caller.
    pub semantic_digest: [u8; 32],
}

/// Descriptor for retained bytes under a semantic coordinate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedBlobDescriptor {
    /// Semantic coordinate that names the retained bytes.
    pub coordinate: SemanticBlobCoordinate,
    /// Content-only CAS hash for the retained bytes.
    pub content_hash: BlobHash,
    /// Retained byte length.
    pub byte_len: u64,
}

/// Retained bytes plus their descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedBlob {
    /// Semantic descriptor.
    pub descriptor: RetainedBlobDescriptor,
    /// Retained content bytes.
    pub bytes: Arc<[u8]>,
}

/// Bounded byte range loaded through a semantic coordinate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedBlobRange {
    /// Semantic descriptor for the source blob.
    pub descriptor: RetainedBlobDescriptor,
    /// Starting byte offset in the source blob.
    pub offset: u64,
    /// Bounded retained bytes.
    pub bytes: Arc<[u8]>,
}

/// Typed semantic-retention lookup failures.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RetentionError {
    /// No descriptor exists for the semantic coordinate.
    #[error("missing semantic retention coordinate: {coordinate:?}")]
    MissingSemanticCoordinate {
        /// Requested semantic coordinate.
        coordinate: SemanticBlobCoordinate,
    },
    /// The semantic descriptor exists, but its content bytes are not retained locally.
    #[error("missing retained blob content: {content_hash}")]
    MissingBlob {
        /// Missing content hash.
        content_hash: BlobHash,
    },
    /// The requested bounded range exceeds the caller's byte budget.
    #[error("retained blob range exceeds budget: requested {requested_bytes}, max {max_bytes}")]
    RangeExceedsBudget {
        /// Requested range length.
        requested_bytes: u64,
        /// Caller-provided byte budget.
        max_bytes: u64,
    },
    /// The requested range is outside the retained blob.
    #[error(
        "retained blob range is out of bounds: offset {offset}, len {len}, blob len {byte_len}"
    )]
    RangeOutOfBounds {
        /// Requested start offset.
        offset: u64,
        /// Requested range length.
        len: u64,
        /// Retained blob length.
        byte_len: u64,
    },
    /// A semantic coordinate already names different retained bytes.
    #[error(
        "semantic retention coordinate conflict: {coordinate:?} already names {existing_content_hash}, not {new_content_hash}"
    )]
    SemanticCoordinateConflict {
        /// Conflicting semantic coordinate.
        coordinate: Box<SemanticBlobCoordinate>,
        /// Existing content hash for the coordinate.
        existing_content_hash: BlobHash,
        /// Newly supplied content hash.
        new_content_hash: BlobHash,
    },
}

/// In-memory semantic index over a [`BlobStore`].
///
/// This index does not change CAS hashing. It records which content-only blob
/// answers a specific semantic coordinate and pins retained bytes as local
/// retention roots.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RetainedBlobIndex {
    descriptors: BTreeMap<SemanticBlobCoordinate, RetainedBlobDescriptor>,
}

impl RetainedBlobIndex {
    /// Retains bytes under a semantic coordinate and pins the content hash.
    pub fn retain<S: BlobStore>(
        &mut self,
        store: &mut S,
        coordinate: SemanticBlobCoordinate,
        bytes: &[u8],
    ) -> Result<RetainedBlobDescriptor, RetentionError> {
        let content_hash = blob_hash(bytes);
        if let Some(existing) = self.descriptors.get(&coordinate) {
            if existing.content_hash != content_hash || existing.byte_len != bytes.len() as u64 {
                return Err(RetentionError::SemanticCoordinateConflict {
                    coordinate: Box::new(coordinate),
                    existing_content_hash: existing.content_hash,
                    new_content_hash: content_hash,
                });
            }
            store.put(bytes);
            store.pin(&content_hash);
            return Ok(existing.clone());
        }

        let content_hash = store.put(bytes);
        store.pin(&content_hash);
        let descriptor = RetainedBlobDescriptor {
            coordinate: coordinate.clone(),
            content_hash,
            byte_len: bytes.len() as u64,
        };
        self.descriptors.insert(coordinate, descriptor.clone());
        Ok(descriptor)
    }

    /// Returns the descriptor for a semantic coordinate.
    #[must_use]
    pub fn descriptor(
        &self,
        coordinate: &SemanticBlobCoordinate,
    ) -> Option<&RetainedBlobDescriptor> {
        self.descriptors.get(coordinate)
    }

    /// Loads retained bytes by content hash only.
    ///
    /// This is byte lookup, not semantic authority. Call [`Self::load`] when the
    /// caller needs proof that the bytes answer a specific semantic coordinate.
    pub fn load_by_hash<S: BlobStore>(
        &self,
        store: &S,
        content_hash: BlobHash,
    ) -> Result<Arc<[u8]>, RetentionError> {
        store
            .get(&content_hash)
            .ok_or(RetentionError::MissingBlob { content_hash })
    }

    /// Loads retained bytes only when the semantic coordinate is indexed and
    /// the content bytes are still present locally.
    pub fn load<S: BlobStore>(
        &self,
        store: &S,
        coordinate: &SemanticBlobCoordinate,
    ) -> Result<RetainedBlob, RetentionError> {
        let descriptor = self.descriptors.get(coordinate).cloned().ok_or_else(|| {
            RetentionError::MissingSemanticCoordinate {
                coordinate: coordinate.clone(),
            }
        })?;
        let bytes = self.load_by_hash(store, descriptor.content_hash)?;
        Ok(RetainedBlob { descriptor, bytes })
    }

    /// Loads a bounded byte range through an exact semantic coordinate.
    ///
    /// This is retained-payload lookup, not a streaming subscription surface.
    /// The semantic coordinate must match first, then the requested range must
    /// fit inside the caller-provided byte budget.
    pub fn load_range<S: BlobStore>(
        &self,
        store: &S,
        coordinate: &SemanticBlobCoordinate,
        offset: u64,
        len: u64,
        max_bytes: u64,
    ) -> Result<RetainedBlobRange, RetentionError> {
        let retained = self.load(store, coordinate)?;
        if len > max_bytes {
            return Err(RetentionError::RangeExceedsBudget {
                requested_bytes: len,
                max_bytes,
            });
        }
        let end = offset
            .checked_add(len)
            .ok_or(RetentionError::RangeOutOfBounds {
                offset,
                len,
                byte_len: retained.descriptor.byte_len,
            })?;
        if end > retained.descriptor.byte_len {
            return Err(RetentionError::RangeOutOfBounds {
                offset,
                len,
                byte_len: retained.descriptor.byte_len,
            });
        }
        let start = usize::try_from(offset).map_err(|_| RetentionError::RangeOutOfBounds {
            offset,
            len,
            byte_len: retained.descriptor.byte_len,
        })?;
        let end = usize::try_from(end).map_err(|_| RetentionError::RangeOutOfBounds {
            offset,
            len,
            byte_len: retained.descriptor.byte_len,
        })?;

        Ok(RetainedBlobRange {
            descriptor: retained.descriptor,
            offset,
            bytes: Arc::from(&retained.bytes[start..end]),
        })
    }
}
