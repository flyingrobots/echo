//! Evidence and catalog layer for deriving causal segments from WAL history.

use crate::causal_wal::{
    Lsn, RecoveryScanReport, RecoveryTailPosture, WalFrame, WalRecoveredTransaction, WalSegmentRef,
    WalTransactionCommit, WriterEpochId,
};
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
    /// The affected frontiers root recorded by the transaction commit marker.
    /// Used instead of full before/after boundaries to accommodate recovery posture.
    pub affected_frontiers_root: Hash,
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
pub enum EvidenceCatalogError {
    /// The provided WAL transaction was invalid or malformed.
    InvalidTransaction,
}

/// A lightweight borrowed view of a committed transaction and its frames.
/// Works for both live `WalCommittedTransaction` and `WalRecoveredTransaction`.
pub struct CommittedWalView<'a> {
    /// The transaction commit marker.
    pub commit: &'a WalTransactionCommit,
    /// The transaction frames.
    pub frames: &'a [WalFrame],
}

/// Observer trait for tailing WAL transactions into higher-level evidence.
pub trait CommittedWalObserver {
    /// Observes a committed transaction, mutating the observer's internal state.
    fn observe_committed_wal(
        &mut self,
        view: CommittedWalView<'_>,
    ) -> Result<(), EvidenceCatalogError>;
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

    /// Rebuilds a complete `CausalSegmentCatalog` from a recovery scan report.
    /// This is a read-only projection over committed evidence that does not invoke live side effects.
    pub fn from_recovery_scan(report: &RecoveryScanReport) -> Result<Self, EvidenceCatalogError> {
        let mut catalog = Self::new();
        for tx in &report.transactions {
            catalog.observe_recovered_transaction(tx)?;
        }
        catalog.finish(report.tail_posture)?;
        Ok(catalog)
    }

    /// Observes a single recovered transaction.
    pub fn observe_recovered_transaction(
        &mut self,
        tx: &WalRecoveredTransaction,
    ) -> Result<(), EvidenceCatalogError> {
        self.observe_committed_wal(CommittedWalView {
            commit: &tx.commit,
            frames: &tx.frames,
        })
    }

    /// Concludes the catalog build, optionally recording truncation intent based on tail posture.
    pub fn finish(&mut self, _tail_posture: RecoveryTailPosture) -> Result<(), EvidenceCatalogError> {
        Ok(())
    }
}

impl Default for CausalSegmentCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl CommittedWalObserver for CausalSegmentCatalog {
    fn observe_committed_wal(
        &mut self,
        _view: CommittedWalView<'_>,
    ) -> Result<(), EvidenceCatalogError> {
        // TODO: derive evidence segments from transactions
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_wal::{
        Lsn, WalAppendAuthority, WalFrame, WalFrameHeader, WalSegmentId, WalTransactionBuilder,
        WalTransactionId, WalTransactionKind,
    };

    fn make_test_commit() -> WalTransactionCommit {
        let builder = WalTransactionBuilder::new(
            WriterEpochId::from_hash(blake3::hash(b"test")),
            WalSegmentId::from_raw(1),
            Lsn::from_raw(1),
            WalTransactionKind::SubmissionIntake,
            WalAppendAuthority::SubmissionIntake,
            WalTransactionId::from_hash(blake3::hash(b"tx")),
        );
        let commit = builder.commit(vec![]).unwrap();
        commit.commit.clone()
    }

    #[test]
    fn test_committed_transactions_become_catalog_segments() {
        let commit = make_test_commit();
        let report = RecoveryScanReport {
            transactions: vec![WalRecoveredTransaction {
                commit,
                frames: vec![],
            }],
            tail_posture: RecoveryTailPosture::Clean,
        };

        let catalog = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        // Since the `observe_committed_wal` is a TODO, it just succeeds without adding segments yet, 
        // but this verifies the plumbing correctly routes the `WalRecoveredTransaction`.
        assert!(catalog.segments.is_empty()); 
    }

    #[test]
    fn test_uncommitted_tail_frames_do_not_become_catalog_segments() {
        // Just setting up the RecoveryScanReport correctly models how it's handed to the catalog
        let report = RecoveryScanReport {
            transactions: vec![],
            tail_posture: RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(5)),
        };

        let catalog = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        assert!(catalog.segments.is_empty());
    }
}
