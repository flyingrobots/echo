//! Evidence and catalog layer for deriving causal segments from WAL history.

use crate::causal_wal::{Lsn, WalCommittedTransaction, WalFrame, WalSegmentRef, WriterEpochId};
use crate::wsc::WscStoreEnvelopeId;
use blake3::Hash;

/// The compaction tier of an evidence segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceTier {
    /// Hot, uncompressed material (exact WAL/WSC representations).
    Hot,
    /// Warm, compressed material with derived indexes.
    Warm,
    /// Cold, proof-carrying wormhole.
    Cold,
}

/// The available posture of the material contained in an evidence segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningPosture {
    /// Exact raw row/patch material available.
    Exact,
    /// Exact material available via sparse index lookup.
    Indexed,
    /// Compressed WSC material, requiring lazy open/replay.
    CompressedButOpenable,
    /// Proof only (ZK Wormhole), material unavailable.
    ProofOnly,
    /// Purged prior to a checkpoint, inherently un-openable.
    PrunedWithCheckpoint,
    /// Data missing unexpectedly (obstruction).
    ObstructedMissingMaterial,
    /// Data is corrupt and unreadable.
    Corrupt,
    /// Segment is known to exist but cannot be reached.
    Unavailable,
}

/// A contiguous semantic tick range covered by an evidence segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickRange {
    /// The starting tick of the causal range.
    pub start: u64,
    /// The ending tick of the causal range.
    pub end: u64,
}

/// A derived catalog entry mapping WAL ranges to causal state or proofs.
#[derive(Clone, Debug)]
pub struct CausalEvidenceSegment {
    /// The writer epoch of the transactions covering this segment.
    pub writer_epoch: WriterEpochId,
    /// The first WAL Log Sequence Number in this segment's causal range.
    pub first_lsn: Lsn,
    /// The last WAL Log Sequence Number in this segment's causal range.
    pub last_lsn: Lsn,
    /// Digest of the first transaction commit in this segment.
    pub first_commit_digest: Hash,
    /// Digest of the last transaction commit in this segment.
    pub last_commit_digest: Hash,
    /// The semantic tick range covered, if applicable.
    pub tick_range: Option<TickRange>,
    /// The causal frontier root before this segment began.
    pub frontier_before: Hash,
    /// The causal frontier root after this segment concluded.
    pub frontier_after: Hash,
    /// References to the underlying WAL material backing this segment.
    pub wal_segment_refs: Vec<WalSegmentRef>,
    /// References to the deterministic WSC envelopes materialized for this segment.
    pub wsc_envelope_refs: Vec<WscStoreEnvelopeId>,
    /// The root digest of the sparse selector index (Roaring bitmap) for this segment.
    pub selector_index_root: Option<Hash>,
    /// The root digest of the retained evidence material.
    pub retained_material_root: Option<Hash>,
    /// The root of the ZK wormhole proof, if this is a Cold segment.
    pub wormhole_proof_root: Option<Hash>,
    /// The compaction tier of this segment.
    pub tier: EvidenceTier,
    /// The degree to which data within this segment can be queried.
    pub opening_posture: OpeningPosture,
}

/// Errors occurring during the extraction or indexing of causal evidence.
#[derive(Debug)]
pub enum EvidenceError {
    /// The provided WAL transaction was invalid or malformed.
    InvalidTransaction,
}

/// Observer trait for tailing or recovering WAL transactions into higher-level evidence.
pub trait CommittedWalObserver {
    /// Observes a committed transaction, mutating the observer's internal state.
    fn observe_committed_transaction(
        &mut self,
        tx: &WalCommittedTransaction,
        frames: &[WalFrame],
    ) -> Result<(), EvidenceError>;
}

/// An indexed catalog mapping ranges of causal history to their underlying evidence.
pub struct CausalSegmentCatalog {
    /// The ordered list of causal evidence segments.
    pub segments: Vec<CausalEvidenceSegment>,
}

impl CausalSegmentCatalog {
    /// Creates a new, empty causal segment catalog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }
}

impl Default for CausalSegmentCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl CommittedWalObserver for CausalSegmentCatalog {
    fn observe_committed_transaction(
        &mut self,
        _tx: &WalCommittedTransaction,
        _frames: &[WalFrame],
    ) -> Result<(), EvidenceError> {
        // Build initial catalog from recovered WAL.
        // Implementation will tail transactions and derive Evidence segments.
        Ok(())
    }
}
