//! Evidence and catalog layer for deriving causal segments from WAL history.

use std::collections::BTreeMap;
use crate::causal_wal::{
    Lsn, RecoveryScanReport, RecoveryTailPosture, WalFrame, WalRecoveredTransaction, WalSegmentRef,
    WalTransactionCommit, WalTransactionId, WalTransactionKind, WriterEpochId,
};
use crate::wsc::WscStoreEnvelopeId;
use blake3::Hash;

/// The unique identifier of a causal evidence segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceSegmentId(pub u64);

/// The kind of segment, denoting how it was derived and what it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceSegmentKind {
    /// A base segment derived from a single committed WAL transaction.
    CommittedTransaction,
    /// A derived segment representing a sealed WAL storage segment.
    WalStorageSegment,
    /// A derived segment representing a checkpoint range.
    CheckpointRange,
    /// A derived segment representing a WSC bundle.
    WscBundle,
    /// A derived segment representing a ZK wormhole proof.
    ZkWormhole,
}

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
    /// Exact committed WAL representation available.
    ExactCommittedWal,
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

/// A key for querying derived segments by LSN range.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceRangeKey {
    /// The first WAL Log Sequence Number in this segment's causal range.
    pub first_lsn: Lsn,
    /// The last WAL Log Sequence Number in this segment's causal range.
    pub last_lsn: Lsn,
}

/// A derived catalog entry mapping WAL ranges to causal state or proofs.
#[derive(Clone, Debug)]
pub struct CausalEvidenceSegment {
    /// The local unique identifier for this segment.
    pub id: EvidenceSegmentId,
    /// The kind of this segment.
    pub kind: EvidenceSegmentKind,
    /// The writer epoch of the transactions covering this segment.
    pub writer_epoch: WriterEpochId,
    /// The first WAL Log Sequence Number in this segment's causal range.
    pub first_lsn: Lsn,
    /// The last WAL Log Sequence Number in this segment's causal range.
    pub last_lsn: Lsn,
    /// The transaction ID if this segment maps exactly to one transaction.
    pub transaction_id: Option<WalTransactionId>,
    /// The transaction kind if this segment maps exactly to one transaction.
    pub transaction_kind: Option<WalTransactionKind>,
    /// The number of records in this segment.
    pub record_count: u64,
    /// The number of frames in this segment.
    pub frame_count: u64,
    /// The records root digest.
    pub records_root: [u8; 32],
    /// Digest of the previous committed transaction.
    pub previous_committed_transaction_digest: [u8; 32],
    /// Digest of the commit marker if this is a single transaction segment.
    pub commit_digest: [u8; 32],
    /// The semantic tick range covered, if applicable.
    pub tick_range: Option<TickRange>,
    /// The affected frontiers root recorded by the transaction commit marker.
    pub affected_frontiers_root: [u8; 32],
    /// References to the underlying WAL material backing this segment.
    pub wal_segment_refs: Vec<WalSegmentRef>,
    /// References to the deterministic WSC envelopes materialized for this segment.
    pub wsc_envelope_refs: Vec<WscStoreEnvelopeId>,
    /// The root digest of the sparse selector index (Roaring bitmap) for this segment.
    pub selector_index_root: Option<[u8; 32]>,
    /// The root digest of the retained evidence material.
    pub retained_material_root: Option<[u8; 32]>,
    /// The root of the ZK wormhole proof, if this is a Cold segment.
    pub wormhole_proof_root: Option<[u8; 32]>,
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
    /// The frame count does not match the record count for the transaction.
    FrameCountMismatch,
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
    /// All segments indexed by local segment ID.
    pub segments_by_id: BTreeMap<EvidenceSegmentId, CausalEvidenceSegment>,
    /// Index mapping commit digest to base segment ID.
    pub base_by_commit_digest: BTreeMap<[u8; 32], EvidenceSegmentId>,
    /// Index mapping transaction ID to base segment ID.
    pub base_by_transaction_id: BTreeMap<WalTransactionId, EvidenceSegmentId>,
    /// Index mapping start LSN to base segment ID.
    pub base_by_lsn_start: BTreeMap<Lsn, EvidenceSegmentId>,
    /// Index mapping ranges to derived segment IDs covering that range.
    pub coverings_by_range: BTreeMap<EvidenceRangeKey, Vec<EvidenceSegmentId>>,
    /// Next available local segment ID.
    next_id: u64,
}

impl CausalSegmentCatalog {
    /// Creates a new, empty causal segment catalog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            segments_by_id: BTreeMap::new(),
            base_by_commit_digest: BTreeMap::new(),
            base_by_transaction_id: BTreeMap::new(),
            base_by_lsn_start: BTreeMap::new(),
            coverings_by_range: BTreeMap::new(),
            next_id: 0,
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

    /// Inserts a base segment into the layered indexes.
    pub fn insert_base_segment(&mut self, segment: CausalEvidenceSegment) {
        let id = segment.id;
        if segment.kind == EvidenceSegmentKind::CommittedTransaction {
            self.base_by_commit_digest.insert(segment.commit_digest, id);
            if let Some(tx_id) = segment.transaction_id {
                self.base_by_transaction_id.insert(tx_id, id);
            }
            self.base_by_lsn_start.insert(segment.first_lsn, id);
        }
        self.segments_by_id.insert(id, segment);
    }

    /// Concludes the catalog build, optionally recording truncation intent based on tail posture.
    pub fn finish(&mut self, _tail_posture: RecoveryTailPosture) -> Result<(), EvidenceCatalogError> {
        Ok(())
    }
    
    fn next_id(&mut self) -> EvidenceSegmentId {
        let id = self.next_id;
        self.next_id += 1;
        EvidenceSegmentId(id)
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
        view: CommittedWalView<'_>,
    ) -> Result<(), EvidenceCatalogError> {
        let commit = view.commit;
        let frame_count = view.frames.len() as u64;
        
        if frame_count != commit.record_count {
            return Err(EvidenceCatalogError::FrameCountMismatch);
        }
        
        let id = self.next_id();
        let segment = CausalEvidenceSegment {
            id,
            kind: EvidenceSegmentKind::CommittedTransaction,
            writer_epoch: commit.writer_epoch,
            first_lsn: commit.first_lsn,
            last_lsn: commit.last_lsn,
            transaction_id: Some(commit.transaction_id),
            transaction_kind: Some(commit.transaction_kind),
            record_count: commit.record_count,
            frame_count,
            records_root: commit.records_root,
            affected_frontiers_root: commit.affected_frontiers_root,
            previous_committed_transaction_digest: commit.previous_committed_transaction_digest,
            commit_digest: commit.commit_digest,
            tick_range: None,
            wal_segment_refs: Vec::new(),
            wsc_envelope_refs: Vec::new(),
            selector_index_root: None,
            retained_material_root: None,
            wormhole_proof_root: None,
            tier: EvidenceTier::Hot,
            opening_posture: OpeningPosture::ExactCommittedWal,
        };
        self.insert_base_segment(segment);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_wal::Lsn;

    fn make_test_commit(tx_kind: WalTransactionKind, tx_id_hash: &[u8], start_lsn: Lsn) -> WalRecoveredTransaction {
        let commit = WalTransactionCommit {
            writer_epoch: WriterEpochId::from_hash(*blake3::hash(b"test").as_bytes()),
            transaction_id: WalTransactionId::from_hash(*blake3::hash(tx_id_hash).as_bytes()),
            transaction_kind: tx_kind,
            first_lsn: start_lsn,
            last_lsn: start_lsn,
            record_count: 0,
            records_root: *blake3::hash(b"records").as_bytes(),
            affected_frontiers_root: *blake3::hash(b"frontiers").as_bytes(),
            previous_committed_transaction_digest: *blake3::hash(b"prev").as_bytes(),
            durability_mode: crate::causal_wal::WalDurabilityMode::StrictFilesystem,
            schema_version: 0,
            commit_digest: *blake3::hash(tx_id_hash).as_bytes(),
        };
        WalRecoveredTransaction {
            commit,
            frames: vec![],
        }
    }

    #[test]
    fn test_committed_transactions_become_catalog_segments() {
        let tx = make_test_commit(WalTransactionKind::SubmissionIntake, b"tx1", Lsn::from_raw(1));
        let report = RecoveryScanReport {
            transactions: vec![tx],
            tail_posture: RecoveryTailPosture::Clean,
        };

        let catalog = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        assert_eq!(catalog.segments_by_id.len(), 1);
        let segment = catalog.segments_by_id.values().next().unwrap();
        assert_eq!(segment.kind, EvidenceSegmentKind::CommittedTransaction);
    }
    
    #[test]
    fn test_multiple_transactions_produce_multiple_segments() {
        let tx1 = make_test_commit(WalTransactionKind::SubmissionIntake, b"tx1", Lsn::from_raw(1));
        let tx2 = make_test_commit(WalTransactionKind::SchedulerTick, b"tx2", Lsn::from_raw(2));
        
        let report = RecoveryScanReport {
            transactions: vec![tx1.clone(), tx2.clone()],
            tail_posture: RecoveryTailPosture::Clean,
        };
        
        let catalog = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        assert_eq!(catalog.segments_by_id.len(), 2);
        
        let seg1 = catalog.base_by_commit_digest.get(&tx1.commit.commit_digest).and_then(|id| catalog.segments_by_id.get(id)).unwrap();
        assert_eq!(seg1.transaction_kind, Some(WalTransactionKind::SubmissionIntake));
        
        let seg2 = catalog.base_by_commit_digest.get(&tx2.commit.commit_digest).and_then(|id| catalog.segments_by_id.get(id)).unwrap();
        assert_eq!(seg2.transaction_kind, Some(WalTransactionKind::SchedulerTick));
    }
    
    #[test]
    fn test_base_segment_preserves_transaction_properties() {
        let tx = make_test_commit(WalTransactionKind::SubmissionIntake, b"tx1", Lsn::from_raw(1));
        let report = RecoveryScanReport {
            transactions: vec![tx.clone()],
            tail_posture: RecoveryTailPosture::Clean,
        };
        let catalog = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        let segment = catalog.segments_by_id.values().next().unwrap();
        
        // base segment preserves transaction_kind
        assert_eq!(segment.transaction_kind, Some(tx.commit.transaction_kind));
        // base segment lsn range equals commit.first_lsn..commit.last_lsn
        assert_eq!(segment.first_lsn, tx.commit.first_lsn);
        assert_eq!(segment.last_lsn, tx.commit.last_lsn);
        // base segment records_root equals commit.records_root
        assert_eq!(segment.records_root, tx.commit.records_root);
        // base segment affected_frontiers_root equals commit.affected_frontiers_root
        assert_eq!(segment.affected_frontiers_root, tx.commit.affected_frontiers_root);
    }

    #[test]
    fn test_uncommitted_tail_frames_do_not_become_catalog_segments() {
        let report = RecoveryScanReport {
            transactions: vec![],
            tail_posture: RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(5)),
        };

        let catalog = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        assert!(catalog.segments_by_id.is_empty());
    }
    
    #[test]
    fn test_rebuilding_catalog_yields_identical_segments() {
        let tx1 = make_test_commit(WalTransactionKind::SubmissionIntake, b"tx1", Lsn::from_raw(1));
        let tx2 = make_test_commit(WalTransactionKind::SchedulerTick, b"tx2", Lsn::from_raw(2));
        let report = RecoveryScanReport {
            transactions: vec![tx1, tx2],
            tail_posture: RecoveryTailPosture::Clean,
        };
        
        let catalog1 = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        let catalog2 = CausalSegmentCatalog::from_recovery_scan(&report).unwrap();
        
        assert_eq!(catalog1.segments_by_id.len(), catalog2.segments_by_id.len());
        for (id, seg1) in &catalog1.segments_by_id {
            let seg2 = catalog2.segments_by_id.get(id).unwrap();
            assert_eq!(seg1.commit_digest, seg2.commit_digest);
            assert_eq!(seg1.first_lsn, seg2.first_lsn);
        }
    }
}
