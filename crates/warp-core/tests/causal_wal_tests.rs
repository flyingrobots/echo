// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Causal WAL foundation tests.

#![allow(
    clippy::match_wild_err_arm,
    clippy::needless_continue,
    clippy::panic,
    clippy::unnecessary_debug_formatting
)]

use echo_cas::{
    BlobHash, BlobStore, MemoryTier, RetainedBlobIndex, RetainedBlobRole, SemanticBlobCoordinate,
};
use warp_core::causal_wal::{
    apply_committed_transaction, audit_wal_release_readiness,
    build_checkpoint_publication_transaction, build_materialization_outbox_transaction,
    build_recovery_certificate, build_retained_reading_transaction,
    build_submission_acceptance_transaction, build_tick_transaction,
    build_topology_intent_transaction, canonical_segment_relative_path, doctor_in_memory_store,
    evaluate_checkpoint_publication, lint_wal_schema_terms, materialize_wal_projection_graph,
    missing_material_scope, observe_wal_projection_graph_wsc, project_causal_commit_evidence,
    project_filesystem_wal_recovery, project_wal_recovery, read_checkpoint_record,
    rebuild_durability_indexes_after_recovery, recover_checkpoint_publications,
    recover_filesystem_store, recover_in_memory_store, recover_materialization_outbox,
    recover_materialization_outbox_with_retained_material, recover_receipt_index,
    recover_retention_index, recover_submission_index, recover_topology_index,
    recovered_submission_receipt_index_root, recovered_topology_index_root,
    retained_material_obstructions, shadow_replay_matches, validate_checkpoint_record,
    validate_strict_object_store_capabilities, wal_projection_graph_schema_hash,
    write_checkpoint_record_atomic, AffectedFrontier, AffectedFrontierKind,
    BraidShellRetentionRecord, CheckpointPublicationRecord, CheckpointRecord,
    CheckpointValidationPosture, EvidenceMaterialPosture, ExistingMaterializedArtifact,
    FilesystemWalStore, InMemoryWalStore, Lsn, MaterializationIntentRecord,
    MaterializationObservationRecord, MaterializationRecoveryPosture, MaterializationReplayPosture,
    MissingMaterialScope, ObjectStoreCapabilityError, ObjectStoreReadAfterWritePosture,
    ObjectStoreWalCapabilities, PayloadCodecId, PayloadSchemaId, ReadingRefRecord,
    RecoveredReceiptIndex, RecoveredRetentionIndex, RecoveredState, RecoveredSubmissionIndex,
    RecoveredSubmissionPosture, RecoveredTopologyIndex, RecoveryAccessMode, RecoveryCertificateRef,
    RecoveryScanReport, RecoveryTailPosture, RetainedMaterialKind, RetainedMaterialRecord,
    StrandDropRecord, StrandForkRecord, SubmissionAcceptanceRecord, SubmissionRetryPosture,
    SuffixImportRecord, TickReceiptRecord, TopologyBraidEventRecord, TopologyImportOutcomeKind,
    TopologyIntentRecord, TransactionLocalIndex, WalAppendAuthority, WalBuildError,
    WalCommitAnchor, WalCommittedTransaction, WalDoctorPosture, WalDurabilityMode, WalManifest,
    WalProjectionGraphObservationPosture, WalReceiptCorrelationRecord, WalRecordKind,
    WalRecoveryBootstrapSource, WalRecoveryCheckpointPosture, WalRecoveryError,
    WalRecoveryIndexError, WalRecoveryPlan, WalRecoveryProjectionObstruction,
    WalRecoveryProjectionPosture, WalRecoveryRetainedMaterialAvailability,
    WalRecoveryRetainedMaterialPosture, WalRecoverySegmentEvidence, WalReleaseReadinessGates,
    WalRoot, WalSchemaLintError, WalSegmentId, WalSegmentRef, WalSegmentSealPosture,
    WalSegmentStorageLocator, WalStoreError, WalStorePort, WalTickDecision, WalTransactionBuilder,
    WalTransactionId, WalTransactionKind, WalWriterEpoch, WriterEpoch, WriterEpochId,
    WriterEpochRequest, WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_EDGE_TYPE,
    WAL_PROJECTION_GRAPH_ROOT_COMMIT_ANCHOR_EDGE_TYPE,
    WAL_PROJECTION_GRAPH_SEGMENT_COMMIT_ANCHOR_EDGE_TYPE, WAL_PROJECTION_GRAPH_SEGMENT_EDGE_TYPE,
    WAL_PROJECTION_GRAPH_WRITER_EPOCH_EDGE_TYPE,
};
use warp_core::wsc::{
    accepted_submission_records_from_wsc_envelope, build_one_warp_input,
    receipt_correlation_records_from_wsc_envelope, topology_records_from_wsc_envelope,
    validate_wsc, validate_wsc_cas_addressed_wal_export, validate_wsc_causal_history_store,
    validate_wsc_ref_only_wal_export, validate_wsc_self_contained_wal_export, write_wsc_one_warp,
    wsc_cas_addressed_wal_export, wsc_ref_only_wal_export, wsc_self_contained_wal_export,
    InMemoryWscStore, WscCasAddressedRetainedMaterialReference, WscCasAddressedWalImportError,
    WscCasAddressedWalSegmentMaterial, WscCasBlobStorePort, WscCausalHistoryExportProfileKind,
    WscFile, WscRefOnlyWalExportError, WscRefOnlyWalLocatorPosture,
    WscRefOnlyWalMaterialDependency, WscSelfContainedRetainedMaterial,
    WscSelfContainedWalExportError, WscSelfContainedWalImportError,
    WscSelfContainedWalSegmentMaterial, WscStoreEnvelope, WscStoreObstructionKind, WscStorePort,
    WscStoreRecordKind, WscWalCausalHistoryRecords,
};
use warp_core::{
    make_strand_id, make_type_id, AuthorityDomainId, AuthorityDomainRef, BraidEvent, BraidStatus,
    Hash, HeadId, OriginId, WorldlineId, WorldlineTick, WriterHeadKey,
};

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

struct EchoCasAvailability<'a>(&'a MemoryTier);

impl WscCasBlobStorePort for EchoCasAvailability<'_> {
    fn cas_blob_bytes(&self, content_hash: &Hash) -> Option<Vec<u8>> {
        self.0
            .get(&BlobHash::from_bytes(*content_hash))
            .map(|bytes| bytes.as_ref().to_vec())
    }
}

fn semantic_coordinate(label: &str) -> SemanticBlobCoordinate {
    SemanticBlobCoordinate {
        namespace: "echo:test-wsc-cas".to_owned(),
        schema_hash_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_owned(),
        artifact_hash_hex: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_owned(),
        role: RetainedBlobRole::ReadingPayload,
        semantic_digest: digest(label),
    }
}

fn must_ok<T, E>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("expected Ok(..), got Err(..)"),
    }
}

fn must_some<T>(option: Option<T>) -> T {
    match option {
        Some(value) => value,
        None => panic!("expected Some(..), got None"),
    }
}

fn must_err<T, E>(result: Result<T, E>, context: &str) -> E {
    match result {
        Ok(_) => panic!("expected Err(..): {context}"),
        Err(error) => error,
    }
}

fn deterministic_test_dir(prefix: &str, label: &str) -> PathBuf {
    let root = PathBuf::from("target").join("warp-core-test-tmp");
    must_ok(fs::create_dir_all(&root));
    for _ in 0..1024 {
        let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = root.join(format!("{prefix}-{label}-{unique}"));
        match fs::create_dir(&dir) {
            Ok(()) => return dir,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                must_ok(fs::remove_dir_all(&dir));
                match fs::create_dir(&dir) {
                    Ok(()) => return dir,
                    Err(retry_error) if retry_error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(retry_error) => {
                        panic!(
                            "failed to recreate deterministic test directory {dir:?}: {retry_error}"
                        )
                    }
                }
            }
            Err(error) => panic!("failed to create deterministic test directory {dir:?}: {error}"),
        }
    }
    panic!("exhausted deterministic test directory attempts for {prefix}-{label}");
}

fn temp_checkpoint_path(label: &str) -> PathBuf {
    deterministic_test_dir("echo-causal-wal", label).join("checkpoint.ecwal")
}

fn temp_wal_dir(label: &str) -> PathBuf {
    deterministic_test_dir("echo-causal-wal-store", label)
}

fn epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(digest("epoch:1"))
}

fn transaction_id(label: &str) -> WalTransactionId {
    WalTransactionId::from_hash(digest(label))
}

fn writer_epoch_request() -> WriterEpochRequest {
    WriterEpochRequest {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("fencing"),
        process_identity: digest("process"),
        host_identity: digest("host"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("lease"),
    }
}

#[test]
fn wal_projection_fact_identity_excludes_absolute_storage_locators() {
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&WriterEpoch {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("projection:fencing"),
        process_identity: digest("projection:process"),
        host_identity: digest("projection:host"),
        started_at_lsn: Lsn::from_raw(7),
        previous_epoch_id: Some(WriterEpochId::from_hash(digest(
            "projection:previous-epoch",
        ))),
        previous_epoch_final_commit_digest: Some(digest("projection:previous-final-commit")),
        lease_or_lock_evidence: digest("projection:lease"),
    });
    let commit_anchor = WalCommitAnchor {
        transaction_id: transaction_id("projection:tx"),
        commit_digest: digest("projection:commit"),
        first_lsn: Lsn::from_raw(7),
        last_lsn: Lsn::from_raw(9),
        record_count: 3,
    };
    let relative_locator = WalSegmentStorageLocator::RelativePath(PathBuf::from("segments/0001"));
    let absolute_locator =
        WalSegmentStorageLocator::AbsolutePath(PathBuf::from("/var/tmp/echo/wal/segments/0001"));
    let segment = WalSegmentRef {
        writer_epoch: writer_epoch.epoch_id,
        segment_id: WalSegmentId::from_raw(1),
        first_lsn: Lsn::from_raw(7),
        last_lsn: Lsn::from_raw(9),
        previous_commit_digest: digest("projection:previous-commit"),
        final_commit_digest: digest("projection:commit"),
        segment_digest: digest("projection:segment"),
        commit_anchors: vec![commit_anchor.clone()],
        seal_posture: WalSegmentSealPosture::Sealed {
            sealed_lsn: Some(Lsn::from_raw(9)),
        },
        storage_locator: Some(relative_locator),
    };
    let relocated_segment = WalSegmentRef {
        storage_locator: Some(absolute_locator),
        ..segment.clone()
    };

    assert_eq!(
        segment.identity_digest(),
        relocated_segment.identity_digest()
    );
    assert_ne!(segment.storage_locator, relocated_segment.storage_locator);

    let changed_segment_digest = WalSegmentRef {
        segment_digest: digest("projection:other-segment"),
        ..segment.clone()
    };
    assert_ne!(
        segment.identity_digest(),
        changed_segment_digest.identity_digest()
    );

    let changed_anchor = WalSegmentRef {
        commit_anchors: vec![WalCommitAnchor {
            commit_digest: digest("projection:other-commit"),
            ..commit_anchor
        }],
        ..segment.clone()
    };
    assert_ne!(segment.identity_digest(), changed_anchor.identity_digest());

    let changed_seal_posture = WalSegmentRef {
        seal_posture: WalSegmentSealPosture::Open,
        ..segment.clone()
    };
    assert_ne!(
        segment.identity_digest(),
        changed_seal_posture.identity_digest()
    );

    let recovery = RecoveryCertificateRef {
        certificate_digest: digest("projection:certificate"),
        checkpoint_used: Some(digest("projection:checkpoint")),
        first_lsn: Some(Lsn::from_raw(7)),
        last_lsn: Some(Lsn::from_raw(9)),
        tail_posture: RecoveryTailPosture::Clean,
        recovered_frontier_root: digest("projection:frontier"),
        recovered_indexes_root: digest("projection:indexes"),
    };
    let second_anchor = WalCommitAnchor {
        transaction_id: transaction_id("projection:tx:second"),
        commit_digest: digest("projection:second-commit"),
        first_lsn: Lsn::from_raw(10),
        last_lsn: Lsn::from_raw(12),
        record_count: 3,
    };
    let second_segment = WalSegmentRef {
        writer_epoch: writer_epoch.epoch_id,
        segment_id: WalSegmentId::from_raw(2),
        first_lsn: Lsn::from_raw(10),
        last_lsn: Lsn::from_raw(12),
        previous_commit_digest: digest("projection:commit"),
        final_commit_digest: digest("projection:second-commit"),
        segment_digest: digest("projection:second-segment"),
        commit_anchors: vec![second_anchor],
        seal_posture: WalSegmentSealPosture::Sealed {
            sealed_lsn: Some(Lsn::from_raw(12)),
        },
        storage_locator: Some(WalSegmentStorageLocator::RelativePath(PathBuf::from(
            "segments/0002",
        ))),
    };
    let root = WalRoot {
        root_digest: digest("projection:root"),
        writer_epochs: vec![writer_epoch.clone()],
        segments: vec![relocated_segment, second_segment.clone()],
        recovery_certificate: Some(recovery.clone()),
    };
    let reordered_root = WalRoot {
        root_digest: digest("projection:root"),
        writer_epochs: vec![writer_epoch],
        segments: vec![second_segment, segment.clone()],
        recovery_certificate: Some(recovery),
    };

    assert_eq!(
        root.segments[0].identity_digest(),
        segment.identity_digest()
    );
    assert_eq!(root.identity_digest(), reordered_root.identity_digest());
}

#[test]
fn wal_projection_graph_materializes_deterministically() {
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&WriterEpoch {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("projection-graph:fencing"),
        process_identity: digest("projection-graph:process"),
        host_identity: digest("projection-graph:host"),
        started_at_lsn: Lsn::from_raw(1),
        previous_epoch_id: Some(WriterEpochId::from_hash(digest(
            "projection-graph:previous-epoch",
        ))),
        previous_epoch_final_commit_digest: Some(digest("projection-graph:previous-final")),
        lease_or_lock_evidence: digest("projection-graph:lease"),
    });
    let first_anchor = WalCommitAnchor {
        transaction_id: transaction_id("projection-graph:tx:first"),
        commit_digest: digest("projection-graph:commit:first"),
        first_lsn: Lsn::from_raw(1),
        last_lsn: Lsn::from_raw(3),
        record_count: 3,
    };
    let second_anchor = WalCommitAnchor {
        transaction_id: transaction_id("projection-graph:tx:second"),
        commit_digest: digest("projection-graph:commit:second"),
        first_lsn: Lsn::from_raw(4),
        last_lsn: Lsn::from_raw(6),
        record_count: 3,
    };
    let first_segment = WalSegmentRef {
        writer_epoch: writer_epoch.epoch_id,
        segment_id: WalSegmentId::from_raw(1),
        first_lsn: Lsn::from_raw(1),
        last_lsn: Lsn::from_raw(3),
        previous_commit_digest: blake3::hash(b"").into(),
        final_commit_digest: first_anchor.commit_digest,
        segment_digest: digest("projection-graph:segment:first"),
        commit_anchors: vec![first_anchor.clone()],
        seal_posture: WalSegmentSealPosture::Sealed {
            sealed_lsn: Some(Lsn::from_raw(3)),
        },
        storage_locator: Some(WalSegmentStorageLocator::RelativePath(PathBuf::from(
            "segments/first.ecwal",
        ))),
    };
    let second_segment = WalSegmentRef {
        writer_epoch: writer_epoch.epoch_id,
        segment_id: WalSegmentId::from_raw(2),
        first_lsn: Lsn::from_raw(4),
        last_lsn: Lsn::from_raw(6),
        previous_commit_digest: first_anchor.commit_digest,
        final_commit_digest: second_anchor.commit_digest,
        segment_digest: digest("projection-graph:segment:second"),
        commit_anchors: vec![second_anchor],
        seal_posture: WalSegmentSealPosture::Sealed {
            sealed_lsn: Some(Lsn::from_raw(6)),
        },
        storage_locator: Some(WalSegmentStorageLocator::AbsolutePath(PathBuf::from(
            "/tmp/echo/segments/second.ecwal",
        ))),
    };
    let recovery = RecoveryCertificateRef {
        certificate_digest: digest("projection-graph:certificate"),
        checkpoint_used: Some(digest("projection-graph:checkpoint")),
        first_lsn: Some(Lsn::from_raw(1)),
        last_lsn: Some(Lsn::from_raw(6)),
        tail_posture: RecoveryTailPosture::Clean,
        recovered_frontier_root: digest("projection-graph:frontier"),
        recovered_indexes_root: digest("projection-graph:indexes"),
    };
    let root = WalRoot {
        root_digest: digest("projection-graph:root"),
        writer_epochs: vec![writer_epoch.clone()],
        segments: vec![second_segment.clone(), first_segment.clone()],
        recovery_certificate: Some(recovery.clone()),
    };
    let reordered_root = WalRoot {
        root_digest: digest("projection-graph:root"),
        writer_epochs: vec![writer_epoch],
        segments: vec![first_segment, second_segment],
        recovery_certificate: Some(recovery),
    };

    let graph = materialize_wal_projection_graph(&root);
    let edges = graph
        .store
        .iter_edges()
        .flat_map(|(_, edges)| edges.iter())
        .collect::<Vec<_>>();
    assert_eq!(graph.store.iter_nodes().count(), 7);
    assert_eq!(edges.len(), 8);
    assert_eq!(
        edges
            .iter()
            .filter(|edge| edge.ty == make_type_id(WAL_PROJECTION_GRAPH_WRITER_EPOCH_EDGE_TYPE))
            .count(),
        1
    );
    assert_eq!(
        edges
            .iter()
            .filter(|edge| edge.ty == make_type_id(WAL_PROJECTION_GRAPH_SEGMENT_EDGE_TYPE))
            .count(),
        2
    );
    assert_eq!(
        edges
            .iter()
            .filter(|edge| {
                edge.ty == make_type_id(WAL_PROJECTION_GRAPH_ROOT_COMMIT_ANCHOR_EDGE_TYPE)
            })
            .count(),
        2
    );
    assert_eq!(
        edges
            .iter()
            .filter(|edge| {
                edge.ty == make_type_id(WAL_PROJECTION_GRAPH_SEGMENT_COMMIT_ANCHOR_EDGE_TYPE)
            })
            .count(),
        2
    );
    assert_eq!(
        edges
            .iter()
            .filter(|edge| {
                edge.ty == make_type_id(WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_EDGE_TYPE)
            })
            .count(),
        1
    );

    let input = build_one_warp_input(&graph.store, graph.root_node_id);
    let wsc_bytes = must_ok(write_wsc_one_warp(
        &input,
        wal_projection_graph_schema_hash(),
        0,
    ));
    let file = must_ok(WscFile::from_bytes(wsc_bytes.clone()));
    must_ok(validate_wsc(&file));
    assert_eq!(file.schema_hash(), &wal_projection_graph_schema_hash());
    let view = must_ok(file.warp_view(0));
    assert_eq!(view.nodes().len(), 7);
    assert_eq!(view.edges().len(), 8);
    let node_attachment_count = (0..view.nodes().len())
        .map(|index| view.node_attachments(index).len())
        .sum::<usize>();
    assert_eq!(node_attachment_count, 7);

    let reordered_graph = materialize_wal_projection_graph(&reordered_root);
    let reordered_input =
        build_one_warp_input(&reordered_graph.store, reordered_graph.root_node_id);
    let reordered_wsc_bytes = must_ok(write_wsc_one_warp(
        &reordered_input,
        wal_projection_graph_schema_hash(),
        0,
    ));
    assert_eq!(wsc_bytes, reordered_wsc_bytes);
}

#[test]
fn wal_projection_cannot_bootstrap_recovery() {
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&WriterEpoch {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("projection-negative:fencing"),
        process_identity: digest("projection-negative:process"),
        host_identity: digest("projection-negative:host"),
        started_at_lsn: Lsn::from_raw(1),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("projection-negative:lease"),
    });
    let anchor = WalCommitAnchor {
        transaction_id: transaction_id("projection-negative:tx"),
        commit_digest: digest("projection-negative:commit"),
        first_lsn: Lsn::from_raw(1),
        last_lsn: Lsn::from_raw(2),
        record_count: 2,
    };
    let segment = WalSegmentRef {
        writer_epoch: writer_epoch.epoch_id,
        segment_id: WalSegmentId::from_raw(7),
        first_lsn: Lsn::from_raw(1),
        last_lsn: Lsn::from_raw(2),
        previous_commit_digest: blake3::hash(b"").into(),
        final_commit_digest: anchor.commit_digest,
        segment_digest: digest("projection-negative:segment"),
        commit_anchors: vec![anchor],
        seal_posture: WalSegmentSealPosture::Sealed {
            sealed_lsn: Some(Lsn::from_raw(2)),
        },
        storage_locator: Some(WalSegmentStorageLocator::AbsolutePath(PathBuf::from(
            "/tmp/echo/projection-negative/segment.ecwal",
        ))),
    };
    let root = WalRoot {
        root_digest: digest("projection-negative:root"),
        writer_epochs: vec![writer_epoch],
        segments: vec![segment],
        recovery_certificate: Some(RecoveryCertificateRef {
            certificate_digest: digest("projection-negative:certificate"),
            checkpoint_used: None,
            first_lsn: Some(Lsn::from_raw(1)),
            last_lsn: Some(Lsn::from_raw(2)),
            tail_posture: RecoveryTailPosture::Clean,
            recovered_frontier_root: digest("projection-negative:frontier"),
            recovered_indexes_root: digest("projection-negative:indexes"),
        }),
    };

    let graph = materialize_wal_projection_graph(&root);
    let input = build_one_warp_input(&graph.store, graph.root_node_id);
    let wsc_bytes = must_ok(write_wsc_one_warp(
        &input,
        wal_projection_graph_schema_hash(),
        0,
    ));
    let observation = must_ok(observe_wal_projection_graph_wsc(&wsc_bytes));
    assert_eq!(
        observation.posture,
        WalProjectionGraphObservationPosture::ObservationOnly
    );
    assert_eq!(observation.schema_hash, wal_projection_graph_schema_hash());
    assert_eq!(observation.root_node_id, graph.root_node_id);
    assert_eq!(observation.node_count, 5);
    assert_eq!(observation.edge_count, 5);
    assert_eq!(observation.node_attachment_count, 5);
    assert_eq!(observation.edge_attachment_count, 0);
    assert!(observation.blob_len > 0);

    let envelope = must_ok(WscStoreEnvelope::validated(
        WscStoreRecordKind::CausalHistory,
        root.identity_digest(),
        wsc_bytes,
    ));
    assert_eq!(
        must_err(
            accepted_submission_records_from_wsc_envelope(&envelope),
            "projection graph is not accepted-submission causal history",
        )
        .kind,
        WscStoreObstructionKind::InvalidWsc
    );
    assert_eq!(
        must_err(
            receipt_correlation_records_from_wsc_envelope(&envelope),
            "projection graph is not receipt-correlation causal history",
        )
        .kind,
        WscStoreObstructionKind::InvalidWsc
    );
    assert_eq!(
        must_err(
            topology_records_from_wsc_envelope(&envelope),
            "projection graph is not topology causal history",
        )
        .kind,
        WscStoreObstructionKind::InvalidWsc
    );

    let projection_only_recovery = project_wal_recovery(
        &RecoveryScanReport {
            transactions: Vec::new(),
            tail_posture: RecoveryTailPosture::Clean,
        },
        None,
        &root.writer_epochs,
        &[],
        None,
    );
    assert_eq!(
        projection_only_recovery.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert_eq!(projection_only_recovery.root, None);
    assert!(projection_only_recovery
        .obstructions
        .contains(&WalRecoveryProjectionObstruction::MissingManifest));
}

#[test]
fn wal_projection_from_recovery() {
    let dir = temp_wal_dir("projection-recovery");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "projection-recovery",
        Lsn::from_raw(0),
    )));
    must_ok(store.append_transaction(durable_tick_transaction(
        "projection-recovery",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));
    let seal = must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let last_commit_digest = must_some(
        store
            .read_commits()
            .last()
            .map(|commit| commit.commit_digest),
    );
    let manifest = WalManifest {
        manifest_digest: digest("projection-recovery:manifest"),
        last_committed_lsn: Some(Lsn::from_raw(4)),
        last_commit_digest: Some(last_commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest.clone()));
    let segment_before = must_ok(fs::read(store.segment_path()));
    let manifest_before = must_ok(fs::read(dir.join("manifest.ecwal")));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let certificate = build_recovery_certificate(
        &report,
        None,
        0,
        digest("projection-recovery:frontier"),
        digest("projection-recovery:indexes"),
    );
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let projection = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    let repeated = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );

    assert_eq!(projection, repeated);
    assert_eq!(projection.posture, WalRecoveryProjectionPosture::Present);
    assert!(projection.obstructions.is_empty());
    let root = must_some(projection.root);
    assert_eq!(root.root_digest, manifest.manifest_digest);
    assert_eq!(root.writer_epochs, vec![writer_epoch.clone()]);
    assert_eq!(root.segments.len(), 1);
    assert_eq!(root.segments[0].writer_epoch, epoch_id());
    assert_eq!(root.segments[0].segment_id, WalSegmentId::from_raw(1));
    assert_eq!(root.segments[0].first_lsn, Lsn::from_raw(0));
    assert_eq!(root.segments[0].last_lsn, Lsn::from_raw(4));
    assert_eq!(root.segments[0].segment_digest, seal.segment_digest);
    assert_eq!(root.segments[0].commit_anchors.len(), 2);
    assert_eq!(
        root.segments[0].seal_posture,
        WalSegmentSealPosture::Sealed {
            sealed_lsn: Some(Lsn::from_raw(4))
        }
    );
    assert_eq!(
        root.segments[0].storage_locator,
        Some(WalSegmentStorageLocator::RelativePath(
            canonical_segment_relative_path(WalSegmentId::from_raw(1))
        ))
    );
    assert!(root.recovery_certificate.is_some());
    assert_eq!(must_ok(fs::read(store.segment_path())), segment_before);
    assert_eq!(
        must_ok(fs::read(dir.join("manifest.ecwal"))),
        manifest_before
    );

    let segment_evidence = WalRecoverySegmentEvidence {
        segment_id: seal.segment_id,
        segment_digest: seal.segment_digest,
        seal_posture: WalSegmentSealPosture::Sealed {
            sealed_lsn: seal.sealed_lsn,
        },
        storage_locator: Some(WalSegmentStorageLocator::RelativePath(
            canonical_segment_relative_path(WalSegmentId::from_raw(1)),
        )),
    };
    let missing_manifest = project_wal_recovery(
        &report,
        None,
        std::slice::from_ref(&writer_epoch),
        std::slice::from_ref(&segment_evidence),
        Some(&certificate),
    );
    assert_eq!(
        missing_manifest.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert_eq!(missing_manifest.root, None);
    assert!(missing_manifest
        .obstructions
        .contains(&WalRecoveryProjectionObstruction::MissingManifest));

    let absent = project_wal_recovery(
        &RecoveryScanReport {
            transactions: Vec::new(),
            tail_posture: RecoveryTailPosture::Clean,
        },
        None,
        &[],
        &[],
        None,
    );
    assert_eq!(absent.posture, WalRecoveryProjectionPosture::Absent);
    assert_eq!(absent.root, None);
    assert!(absent.obstructions.is_empty());

    let empty_report_with_segment_evidence = project_wal_recovery(
        &RecoveryScanReport {
            transactions: Vec::new(),
            tail_posture: RecoveryTailPosture::Clean,
        },
        None,
        &[],
        std::slice::from_ref(&segment_evidence),
        None,
    );
    assert_eq!(
        empty_report_with_segment_evidence.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert_eq!(empty_report_with_segment_evidence.root, None);
    assert!(empty_report_with_segment_evidence
        .obstructions
        .contains(&WalRecoveryProjectionObstruction::MissingManifest));

    let empty_uncommitted_tail = project_wal_recovery(
        &RecoveryScanReport {
            transactions: Vec::new(),
            tail_posture: RecoveryTailPosture::WouldTruncateAll,
        },
        None,
        &[],
        &[],
        None,
    );
    assert_eq!(
        empty_uncommitted_tail.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert!(empty_uncommitted_tail.obstructions.contains(
        &WalRecoveryProjectionObstruction::TailPostureObstructed {
            posture: RecoveryTailPosture::WouldTruncateAll
        }
    ));
    assert!(empty_uncommitted_tail
        .obstructions
        .contains(&WalRecoveryProjectionObstruction::MissingManifest));

    let duplicate_segments = [segment_evidence.clone(), segment_evidence.clone()];
    let duplicate_segment_evidence = project_wal_recovery(
        &report,
        Some(&manifest),
        std::slice::from_ref(&writer_epoch),
        &duplicate_segments,
        Some(&certificate),
    );
    assert_eq!(
        duplicate_segment_evidence.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert!(duplicate_segment_evidence.obstructions.contains(
        &WalRecoveryProjectionObstruction::DuplicateSegmentEvidence {
            segment_id: WalSegmentId::from_raw(1)
        }
    ));

    let short_seal_evidence = WalRecoverySegmentEvidence {
        seal_posture: WalSegmentSealPosture::Sealed {
            sealed_lsn: Some(Lsn::from_raw(3)),
        },
        ..segment_evidence.clone()
    };
    let short_seal = project_wal_recovery(
        &report,
        Some(&manifest),
        std::slice::from_ref(&writer_epoch),
        std::slice::from_ref(&short_seal_evidence),
        Some(&certificate),
    );
    assert_eq!(short_seal.posture, WalRecoveryProjectionPosture::Obstructed);
    assert!(short_seal.obstructions.contains(
        &WalRecoveryProjectionObstruction::SegmentSealDoesNotCoverRecoveredCommit {
            segment_id: WalSegmentId::from_raw(1),
            sealed_lsn: Some(Lsn::from_raw(3)),
            recovered_last_lsn: Lsn::from_raw(4)
        }
    ));

    let bad_digest_evidence = WalRecoverySegmentEvidence {
        segment_digest: digest("projection-recovery:wrong-segment"),
        ..segment_evidence.clone()
    };
    let bad_segment_digest = project_wal_recovery(
        &report,
        Some(&manifest),
        std::slice::from_ref(&writer_epoch),
        std::slice::from_ref(&bad_digest_evidence),
        Some(&certificate),
    );
    assert_eq!(
        bad_segment_digest.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert!(bad_segment_digest.obstructions.contains(
        &WalRecoveryProjectionObstruction::SegmentDigestMismatch {
            segment_id: WalSegmentId::from_raw(1),
            expected: seal.segment_digest,
            actual: digest("projection-recovery:wrong-segment")
        }
    ));

    let mismatched_certificate = warp_core::causal_wal::RecoveryCertificate {
        committed_transactions_replayed: 99,
        ..certificate
    };
    let certificate_mismatch = project_wal_recovery(
        &report,
        Some(&manifest),
        std::slice::from_ref(&writer_epoch),
        std::slice::from_ref(&segment_evidence),
        Some(&mismatched_certificate),
    );
    assert_eq!(
        certificate_mismatch.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert!(certificate_mismatch.obstructions.contains(
        &WalRecoveryProjectionObstruction::RecoveryCertificateScanMismatch {
            expected_first_lsn: Some(Lsn::from_raw(0)),
            actual_first_lsn: Some(Lsn::from_raw(0)),
            expected_last_lsn: Some(Lsn::from_raw(4)),
            actual_last_lsn: Some(Lsn::from_raw(4)),
            expected_committed_transactions_replayed: 2,
            actual_committed_transactions_replayed: 99,
            expected_tail_posture: RecoveryTailPosture::Clean,
            actual_tail_posture: RecoveryTailPosture::Clean
        }
    ));

    must_ok(store.publish_manifest(
        epoch_id(),
        WalManifest {
            last_committed_lsn: Some(Lsn::from_raw(3)),
            ..manifest
        },
    ));
    let filesystem_manifest_mismatch = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(
        filesystem_manifest_mismatch.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert!(filesystem_manifest_mismatch.obstructions.contains(
        &WalRecoveryProjectionObstruction::ManifestLastCommittedLsnMismatch {
            expected: Some(Lsn::from_raw(4)),
            actual: Some(Lsn::from_raw(3))
        }
    ));

    must_ok(store.publish_manifest(
        epoch_id(),
        WalManifest {
            sealed_segment_count: 2,
            ..manifest
        },
    ));
    let filesystem_segment_count_mismatch = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(
        filesystem_segment_count_mismatch.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert!(filesystem_segment_count_mismatch.obstructions.contains(
        &WalRecoveryProjectionObstruction::ManifestSegmentCountMismatch {
            expected: 2,
            actual: 1,
        }
    ));

    let unavailable_locator = project_wal_recovery(
        &report,
        Some(&manifest),
        &[writer_epoch],
        &[WalRecoverySegmentEvidence {
            storage_locator: None,
            ..segment_evidence
        }],
        Some(&certificate),
    );
    assert_eq!(
        unavailable_locator.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert_eq!(unavailable_locator.root, None);
    assert!(unavailable_locator.obstructions.contains(
        &WalRecoveryProjectionObstruction::SegmentLocatorUnavailable {
            segment_id: WalSegmentId::from_raw(1)
        }
    ));
}

#[test]
fn recovery_plan_bootstraps_from_wal_root() {
    let label = "recovery-plan";
    let dir = temp_wal_dir(label);
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = retained_material(
        label,
        RetainedMaterialKind::ReadingPayload,
        EvidenceMaterialPosture::Present,
    );
    let reading = reading_ref(label, EvidenceMaterialPosture::Present);

    must_ok(store.append_transaction(durable_submission_transaction(label, Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        label,
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));
    let builder = builder(
        transaction_id("tx:recovery-plan:retained-reading"),
        Lsn::from_raw(5),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            std::slice::from_ref(&material),
            reading,
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "recovery-plan:reading:before",
                "recovery-plan:reading:after",
            )],
        ))),
    );
    must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let last_commit_digest = must_some(
        store
            .read_commits()
            .last()
            .map(|commit| commit.commit_digest),
    );
    let manifest = WalManifest {
        manifest_digest: digest("recovery-plan:manifest"),
        last_committed_lsn: Some(Lsn::from_raw(6)),
        last_commit_digest: Some(last_commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest.clone()));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let checkpoint_digest = digest("recovery-plan:checkpoint");
    let certificate = build_recovery_certificate(
        &report,
        Some(checkpoint_digest),
        0,
        digest("recovery-plan:frontier"),
        digest("recovery-plan:indexes"),
    );
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let projection = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(projection.posture, WalRecoveryProjectionPosture::Present);
    let root = must_some(projection.root.clone());
    let retention = must_ok(recover_retention_index(&report));
    let available_material = BTreeSet::from([material.material_digest]);
    let retained_material_posture =
        WalRecoveryRetainedMaterialPosture::from_retention_index(&retention, &available_material);

    let plan = WalRecoveryPlan::from_wal_root(
        &root,
        &report,
        WalRecoveryCheckpointPosture::Selected {
            checkpoint_digest,
            validation: CheckpointValidationPosture::PublishedAndUsable,
        },
        &certificate,
        retained_material_posture,
        &projection,
    );

    assert_eq!(
        plan.bootstrap_source,
        WalRecoveryBootstrapSource::WalRoot {
            root_digest: root.root_digest,
            root_identity_digest: root.identity_digest()
        }
    );
    assert_eq!(
        plan.checkpoint_posture,
        WalRecoveryCheckpointPosture::Selected {
            checkpoint_digest,
            validation: CheckpointValidationPosture::PublishedAndUsable
        }
    );
    assert_eq!(plan.replay_suffix.first_lsn, Some(Lsn::from_raw(0)));
    assert_eq!(plan.replay_suffix.last_lsn, Some(Lsn::from_raw(6)));
    assert_eq!(plan.replay_suffix.committed_transactions, 3);
    assert_eq!(
        plan.replay_suffix.final_commit_digest,
        Some(last_commit_digest)
    );
    assert_eq!(plan.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(
        plan.index_roots.recovered_frontier_root,
        certificate.recovered_frontier_root
    );
    assert_eq!(
        plan.index_roots.recovered_indexes_root,
        certificate.recovered_indexes_root
    );
    assert_eq!(
        plan.retained_material_posture.availability,
        WalRecoveryRetainedMaterialAvailability::Available
    );
    assert_eq!(plan.retained_material_posture.material_count, 1);
    assert_eq!(plan.retained_material_posture.reading_count, 1);
    assert_eq!(plan.retained_material_posture.obstruction_count, 0);
    assert_eq!(
        plan.projected_evidence_posture,
        WalRecoveryProjectionPosture::Present
    );
    assert_eq!(plan.projected_evidence_obstruction_count, 0);

    let manifest_plan = WalRecoveryPlan::from_manifest(
        &manifest,
        &report,
        WalRecoveryCheckpointPosture::NotSelected,
        &certificate,
        retained_material_posture,
        &projection,
    );
    assert_eq!(
        manifest_plan.bootstrap_source,
        WalRecoveryBootstrapSource::StorageManifest {
            manifest_digest: manifest.manifest_digest
        }
    );
    assert_eq!(
        manifest_plan.checkpoint_posture,
        WalRecoveryCheckpointPosture::NotSelected
    );
    assert_eq!(manifest_plan.replay_suffix, plan.replay_suffix);
}

#[test]
fn wal_recovery_rebuilds_all_durability_indexes() {
    let label = "durability-index-rebuild";
    let dir = temp_wal_dir(label);
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let acceptance = submission_acceptance(label);
    let receipt = receipt_record(label, WalTickDecision::Applied);
    let correlation = correlation_record(label);
    let material = retained_material(
        label,
        RetainedMaterialKind::ReadingPayload,
        EvidenceMaterialPosture::Present,
    );
    let reading = reading_ref(label, EvidenceMaterialPosture::Present);
    let materialization_intent = MaterializationIntentRecord {
        effect_id: digest("effect:durability-index-rebuild"),
        expected_artifact_digest: digest("artifact:durability-index-rebuild"),
        materialization_intent_digest: digest("materialization:durability-index-rebuild"),
        idempotency_token: digest("idempotency:durability-index-rebuild"),
        target_metadata_digest: digest("metadata:durability-index-rebuild"),
    };
    let materialization_observation = MaterializationObservationRecord {
        effect_id: materialization_intent.effect_id,
        observed_artifact_digest: materialization_intent.expected_artifact_digest,
        observed_metadata_digest: materialization_intent.target_metadata_digest,
    };

    must_ok(store.append_transaction(durable_submission_transaction(label, Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        label,
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));
    let reading_builder = builder(
        transaction_id("tx:durability-index-rebuild:reading"),
        Lsn::from_raw(5),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            reading_builder,
            std::slice::from_ref(&material),
            reading,
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "durability-index:reading:before",
                "durability-index:reading:after",
            )],
        ))),
    );
    let materialization_builder = builder(
        transaction_id("tx:durability-index-rebuild:materialization"),
        Lsn::from_raw(7),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::MaterializationOutbox,
    );
    must_ok(
        store.append_transaction(must_ok(build_materialization_outbox_transaction(
            materialization_builder,
            materialization_intent,
            Some(materialization_observation),
            vec![frontier(
                AffectedFrontierKind::ReceiptIndex,
                "durability-index:outbox:before",
                "durability-index:outbox:after",
            )],
        ))),
    );
    must_ok(store.append_transaction(topology_transaction(Lsn::from_raw(9))));
    must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let last_commit_digest = must_some(
        store
            .read_commits()
            .last()
            .map(|commit| commit.commit_digest),
    );
    let manifest = WalManifest {
        manifest_digest: digest("durability-index:manifest"),
        last_committed_lsn: Some(Lsn::from_raw(13)),
        last_commit_digest: Some(last_commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest));

    let clean_report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let clean_projection = project_filesystem_wal_recovery(
        &dir,
        &clean_report,
        std::slice::from_ref(&writer_epoch),
        None,
    );
    let clean_indexes = must_ok(rebuild_durability_indexes_after_recovery(
        &clean_report,
        &BTreeMap::new(),
        &clean_projection,
    ));
    let expected_submissions = must_ok(
        RecoveredSubmissionIndex::from_acceptance_and_receipt_records([acceptance], [receipt]),
    );
    let expected_receipts =
        RecoveredReceiptIndex::from_receipt_correlation_records([receipt], [correlation]);
    let expected_retention = must_ok(RecoveredRetentionIndex::from_retention_records(
        [material],
        [reading],
    ));
    let expected_topology = must_ok(RecoveredTopologyIndex::from_topology_records(
        topology_records(),
    ));
    let clean_root = must_some(clean_projection.root.as_ref());

    assert_eq!(clean_report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(clean_indexes.submissions, expected_submissions);
    assert_eq!(clean_indexes.receipts, expected_receipts);
    assert_eq!(clean_indexes.retention, expected_retention);
    assert_eq!(clean_indexes.topology, expected_topology);
    assert_eq!(
        clean_indexes
            .materialization_outbox
            .get(&materialization_intent.effect_id)
            .map(|entry| entry.posture),
        Some(MaterializationReplayPosture::AlreadyObserved)
    );
    assert_eq!(
        clean_indexes.projection.posture,
        WalRecoveryProjectionPosture::Present
    );
    assert_eq!(
        clean_indexes.projection.root_identity_digest,
        Some(clean_root.identity_digest())
    );
    assert_eq!(clean_indexes.projection.writer_epoch_count, 1);
    assert_eq!(clean_indexes.projection.segment_count, 1);
    assert_eq!(clean_indexes.projection.obstruction_count, 0);
    assert_eq!(
        clean_indexes.submission_receipt_index_root,
        recovered_submission_receipt_index_root(
            &clean_indexes.submissions,
            &clean_indexes.receipts
        )
    );
    assert_eq!(
        clean_indexes.topology_index_root,
        recovered_topology_index_root(&clean_indexes.topology)
    );

    let tail_tx = durable_submission_transaction("uncommitted-tail", Lsn::from_raw(14));
    must_ok(store.append_uncommitted_frame(epoch_id(), tail_tx.frames[0].clone()));
    let tail_report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let tail_projection = project_filesystem_wal_recovery(
        &dir,
        &tail_report,
        std::slice::from_ref(&writer_epoch),
        None,
    );
    let tail_indexes = must_ok(rebuild_durability_indexes_after_recovery(
        &tail_report,
        &BTreeMap::new(),
        &tail_projection,
    ));

    assert_eq!(
        tail_report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(13))
    );
    assert!(tail_indexes
        .submissions
        .get(&submission_acceptance("uncommitted-tail").submission_id)
        .is_none());
    assert_eq!(tail_indexes.submissions, clean_indexes.submissions);
    assert_eq!(tail_indexes.receipts, clean_indexes.receipts);
    assert_eq!(tail_indexes.retention, clean_indexes.retention);
    assert_eq!(
        tail_indexes.materialization_outbox,
        clean_indexes.materialization_outbox
    );
    assert_eq!(tail_indexes.topology, clean_indexes.topology);
    assert_eq!(
        tail_indexes.projection.posture,
        WalRecoveryProjectionPosture::Obstructed
    );
    assert_eq!(tail_indexes.projection.root_identity_digest, None);
    assert!(tail_indexes.projection.obstruction_count > 0);
    assert!(tail_projection.obstructions.contains(
        &WalRecoveryProjectionObstruction::TailPostureObstructed {
            posture: RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(13))
        }
    ));
}

#[test]
fn wsc_ref_only_export_preserves_wal_identity() {
    let label = "wsc-ref-only";
    let dir = temp_wal_dir(label);
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let acceptance = submission_acceptance(label);
    let receipt = receipt_record(label, WalTickDecision::Applied);
    let correlation = correlation_record(label);

    must_ok(store.append_transaction(durable_submission_transaction(label, Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        label,
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));
    let seal = must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let last_commit_digest = must_some(
        store
            .read_commits()
            .last()
            .map(|commit| commit.commit_digest),
    );
    let manifest = WalManifest {
        manifest_digest: digest("wsc-ref-only:manifest"),
        last_committed_lsn: Some(Lsn::from_raw(4)),
        last_commit_digest: Some(last_commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let certificate = build_recovery_certificate(
        &report,
        None,
        0,
        digest("wsc-ref-only:frontier"),
        digest("wsc-ref-only:indexes"),
    );
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let projection = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(projection.posture, WalRecoveryProjectionPosture::Present);
    let root = must_some(projection.root);
    let segment = &root.segments[0];
    assert_eq!(segment.segment_digest, seal.segment_digest);

    let export = must_ok(wsc_ref_only_wal_export(
        &root,
        wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
    ));
    let imported = must_ok(validate_wsc_ref_only_wal_export(&export, &root));

    assert_eq!(imported.profile, WscCausalHistoryExportProfileKind::RefOnly);
    assert_eq!(
        imported.projection.schema_hash,
        wal_projection_graph_schema_hash()
    );
    assert_eq!(imported.root_identity_digest, root.identity_digest());
    assert_eq!(imported.accepted_submissions, vec![acceptance]);
    assert_eq!(imported.receipts, vec![receipt]);
    assert_eq!(imported.correlations, vec![correlation]);

    assert_eq!(imported.segment_dependencies.len(), 1);
    let dependency = &imported.segment_dependencies[0];
    assert_eq!(
        dependency.material_dependency,
        WscRefOnlyWalMaterialDependency::ExternalSegmentBytes
    );
    assert_eq!(
        dependency.locator_posture,
        WscRefOnlyWalLocatorPosture::RelativePath
    );
    assert_eq!(dependency.segment_id, segment.segment_id);
    assert_eq!(
        dependency.segment_identity_digest,
        segment.identity_digest()
    );
    assert_eq!(dependency.segment_digest, segment.segment_digest);
    assert_eq!(dependency.first_lsn, segment.first_lsn);
    assert_eq!(dependency.last_lsn, segment.last_lsn);
    let mut expected_anchor_digests = segment
        .commit_anchors
        .iter()
        .map(WalCommitAnchor::identity_digest)
        .collect::<Vec<_>>();
    expected_anchor_digests.sort_unstable();
    assert_eq!(dependency.commit_anchor_digests, expected_anchor_digests);

    let mut wsc_store = InMemoryWscStore::default();
    must_ok(wsc_store.write_envelope(export.projection_envelope.clone()));
    must_ok(wsc_store.write_envelope(export.accepted_submission_envelope.clone()));
    must_ok(wsc_store.write_envelope(export.receipt_correlation_envelope.clone()));
    must_ok(validate_wsc_causal_history_store(&wsc_store));

    let absolute_a = root_with_absolute_locator(&root, "/tmp/echo-a/segments/00000001.ecwal");
    let absolute_b = root_with_absolute_locator(&root, "/var/tmp/echo-b/segments/00000001.ecwal");
    assert_eq!(root.identity_digest(), absolute_a.identity_digest());
    assert_eq!(absolute_a.identity_digest(), absolute_b.identity_digest());

    let absolute_export_a = must_ok(wsc_ref_only_wal_export(
        &absolute_a,
        wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
    ));
    let absolute_export_b = must_ok(wsc_ref_only_wal_export(
        &absolute_b,
        wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
    ));
    assert_eq!(
        absolute_export_a.projection_envelope.basis_digest(),
        export.projection_envelope.basis_digest()
    );
    assert_eq!(
        absolute_export_a.projection_envelope.wsc_bytes(),
        export.projection_envelope.wsc_bytes()
    );
    assert_eq!(
        absolute_export_a.projection_envelope.wsc_bytes(),
        absolute_export_b.projection_envelope.wsc_bytes()
    );

    let absolute_import_a = must_ok(validate_wsc_ref_only_wal_export(
        &absolute_export_a,
        &absolute_a,
    ));
    let absolute_import_b = must_ok(validate_wsc_ref_only_wal_export(
        &absolute_export_b,
        &absolute_b,
    ));
    assert_eq!(
        absolute_import_a.segment_dependencies[0].locator_posture,
        WscRefOnlyWalLocatorPosture::AbsolutePathNormalized
    );
    assert_eq!(
        absolute_import_a.segment_dependencies,
        absolute_import_b.segment_dependencies
    );

    let mut missing_locator = root.clone();
    missing_locator.segments[0].storage_locator = None;
    assert_eq!(
        must_err(
            wsc_ref_only_wal_export(
                &missing_locator,
                wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
            ),
            "ref-only WSC exports require segment locators",
        ),
        WscRefOnlyWalExportError::MissingSegmentLocator {
            segment_id: segment.segment_id,
        }
    );
}

#[test]
fn wsc_self_contained_export_replays_segment_bytes() {
    let label = "wsc-self-contained";
    let dir = temp_wal_dir(label);
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let acceptance = submission_acceptance(label);
    let receipt = receipt_record(label, WalTickDecision::Applied);
    let correlation = correlation_record(label);

    must_ok(store.append_transaction(durable_submission_transaction(label, Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        label,
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));
    let seal = must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let segment_path = store.segment_path();
    let segment_bytes = must_ok(fs::read(&segment_path));
    let last_commit_digest = must_some(
        store
            .read_commits()
            .last()
            .map(|commit| commit.commit_digest),
    );
    let manifest = WalManifest {
        manifest_digest: digest("wsc-self-contained:manifest"),
        last_committed_lsn: Some(Lsn::from_raw(4)),
        last_commit_digest: Some(last_commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let certificate = build_recovery_certificate(
        &report,
        None,
        0,
        digest("wsc-self-contained:frontier"),
        digest("wsc-self-contained:indexes"),
    );
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let projection = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(projection.posture, WalRecoveryProjectionPosture::Present);
    let root = must_some(projection.root);
    let segment = root.segments[0].clone();
    assert_eq!(segment.segment_digest, seal.segment_digest);

    assert_eq!(
        must_err(
            wsc_self_contained_wal_export(
                &root,
                &[],
                &[],
                wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
            ),
            "self-contained WSC exports require embedded segment material",
        ),
        WscSelfContainedWalExportError::MissingSegmentMaterial {
            segment_id: segment.segment_id,
        }
    );

    let export = must_ok(wsc_self_contained_wal_export(
        &root,
        &[WscSelfContainedWalSegmentMaterial {
            segment_id: segment.segment_id,
            segment_bytes: segment_bytes.clone(),
        }],
        &[],
        wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
    ));
    drop(store);
    must_ok(fs::remove_dir_all(&dir));

    let imported = must_ok(validate_wsc_self_contained_wal_export(&export, &root));

    assert_eq!(
        imported.profile,
        WscCausalHistoryExportProfileKind::SelfContained
    );
    assert_eq!(
        imported.projection.schema_hash,
        wal_projection_graph_schema_hash()
    );
    assert_eq!(imported.root_identity_digest, root.identity_digest());
    assert_eq!(imported.accepted_submissions, vec![acceptance]);
    assert_eq!(imported.receipts, vec![receipt]);
    assert_eq!(imported.correlations, vec![correlation]);

    assert_eq!(imported.segment_recoveries.len(), 1);
    let segment_recovery = &imported.segment_recoveries[0];
    assert_eq!(segment_recovery.segment_id, segment.segment_id);
    assert_eq!(segment_recovery.segment_digest, segment.segment_digest);
    assert_eq!(
        segment_recovery.report.tail_posture,
        RecoveryTailPosture::Clean
    );
    assert_eq!(
        segment_recovery.report.first_committed_lsn(),
        Some(segment.first_lsn)
    );
    assert_eq!(
        segment_recovery.report.last_committed_lsn(),
        Some(segment.last_lsn)
    );
    assert_eq!(
        segment_recovery.report.last_commit_digest(),
        Some(segment.final_commit_digest)
    );

    assert_eq!(
        imported.submission_index.retry_posture(
            acceptance.submission_id,
            acceptance.canonical_envelope_digest
        ),
        SubmissionRetryPosture::AlreadyDecidedApplied
    );
    assert_eq!(
        imported
            .receipt_index
            .receipt_by_submission
            .get(&acceptance.submission_id),
        Some(&receipt.receipt_digest)
    );
    assert_eq!(
        imported
            .receipt_index
            .receipt_by_ticket
            .get(&receipt.ticket_digest),
        Some(&receipt.receipt_digest)
    );

    let mut wsc_store = InMemoryWscStore::default();
    must_ok(wsc_store.write_envelope(export.projection_envelope.clone()));
    must_ok(wsc_store.write_envelope(export.segment_material_envelope.clone()));
    must_ok(wsc_store.write_envelope(export.accepted_submission_envelope.clone()));
    must_ok(wsc_store.write_envelope(export.receipt_correlation_envelope));
    must_ok(validate_wsc_causal_history_store(&wsc_store));

    let mut tampered_segment_bytes = segment_bytes;
    let Some(last_byte) = tampered_segment_bytes.last_mut() else {
        panic!("self-contained WAL fixture unexpectedly produced empty segment bytes");
    };
    *last_byte ^= 0x55;
    let tampered_export = must_ok(wsc_self_contained_wal_export(
        &root,
        &[WscSelfContainedWalSegmentMaterial {
            segment_id: segment.segment_id,
            segment_bytes: tampered_segment_bytes,
        }],
        &[],
        wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
    ));
    let error = must_err(
        validate_wsc_self_contained_wal_export(&tampered_export, &root),
        "tampered embedded segment bytes",
    );
    assert!(matches!(
        error,
        WscSelfContainedWalImportError::SegmentRecovery {
            segment_id,
            error: WalRecoveryError::Store(WalStoreError::SegmentRecordDigestMismatch),
        } if segment_id == segment.segment_id
    ));
}

#[test]
fn wsc_cas_addressed_export_requires_present_blobs() {
    let label = "wsc-cas-addressed";
    let dir = temp_wal_dir(label);
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let acceptance = submission_acceptance(label);
    let receipt = receipt_record(label, WalTickDecision::Applied);
    let correlation = correlation_record(label);

    must_ok(store.append_transaction(durable_submission_transaction(label, Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        label,
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));
    must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let segment_path = store.segment_path();
    let segment_bytes = must_ok(fs::read(&segment_path));
    let last_commit_digest = must_some(
        store
            .read_commits()
            .last()
            .map(|commit| commit.commit_digest),
    );
    let manifest = WalManifest {
        manifest_digest: digest("wsc-cas-addressed:manifest"),
        last_committed_lsn: Some(Lsn::from_raw(4)),
        last_commit_digest: Some(last_commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let certificate = build_recovery_certificate(
        &report,
        None,
        0,
        digest("wsc-cas-addressed:frontier"),
        digest("wsc-cas-addressed:indexes"),
    );
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let projection = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(projection.posture, WalRecoveryProjectionPosture::Present);
    let root = must_some(projection.root);
    let segment = root.segments[0].clone();

    let mut cas_store = MemoryTier::new();
    let segment_content_hash = *cas_store.put(&segment_bytes).as_bytes();
    let retained_bytes = b"shared retained WSC material";
    let mut retained_index = RetainedBlobIndex::default();
    let retained_a = must_ok(retained_index.retain(
        &mut cas_store,
        semantic_coordinate("wsc-cas:reading-a"),
        retained_bytes,
    ));
    let retained_b = must_ok(retained_index.retain(
        &mut cas_store,
        semantic_coordinate("wsc-cas:reading-b"),
        retained_bytes,
    ));
    assert_eq!(retained_a.content_hash, retained_b.content_hash);
    assert_ne!(retained_a.coordinate, retained_b.coordinate);

    let export = must_ok(wsc_cas_addressed_wal_export(
        &root,
        &[WscCasAddressedWalSegmentMaterial {
            segment_id: segment.segment_id,
            content_hash: segment_content_hash,
            semantic_coordinate_digest: digest("wsc-cas:segment"),
            byte_len: u64::try_from(segment_bytes.len()).unwrap_or(u64::MAX),
        }],
        &[
            WscCasAddressedRetainedMaterialReference {
                material_kind: RetainedMaterialKind::ReadingPayload,
                content_hash: *retained_a.content_hash.as_bytes(),
                semantic_coordinate_digest: retained_a.coordinate.semantic_digest,
                byte_len: retained_a.byte_len,
            },
            WscCasAddressedRetainedMaterialReference {
                material_kind: RetainedMaterialKind::ReadingPayload,
                content_hash: *retained_b.content_hash.as_bytes(),
                semantic_coordinate_digest: retained_b.coordinate.semantic_digest,
                byte_len: retained_b.byte_len,
            },
        ],
        wsc_records(&[], &[], &[acceptance], &[receipt], &[correlation]),
    ));

    let imported = must_ok(validate_wsc_cas_addressed_wal_export(
        &export,
        &root,
        &EchoCasAvailability(&cas_store),
    ));
    assert_eq!(
        imported.profile,
        WscCausalHistoryExportProfileKind::CasAddressed
    );
    assert_eq!(
        imported.projection.schema_hash,
        wal_projection_graph_schema_hash()
    );
    assert_eq!(imported.root_identity_digest, root.identity_digest());
    assert_eq!(imported.accepted_submissions, vec![acceptance]);
    assert_eq!(imported.receipts, vec![receipt]);
    assert_eq!(imported.correlations, vec![correlation]);

    assert_eq!(imported.cas_references.segments.len(), 1);
    let segment_reference = &imported.cas_references.segments[0];
    assert_eq!(segment_reference.segment_id, segment.segment_id);
    assert_eq!(
        segment_reference.segment_identity_digest,
        segment.identity_digest()
    );
    assert_eq!(segment_reference.segment_digest, segment.segment_digest);
    assert_eq!(segment_reference.content_hash, segment_content_hash);
    assert_eq!(
        imported.cas_references.retained_materials.len(),
        2,
        "equal content hashes under different semantic coordinates stay distinct"
    );
    assert_eq!(
        imported.cas_references.retained_materials[0].content_hash,
        imported.cas_references.retained_materials[1].content_hash
    );
    assert_ne!(
        imported.cas_references.retained_materials[0].semantic_coordinate_digest,
        imported.cas_references.retained_materials[1].semantic_coordinate_digest
    );

    let empty_cas = MemoryTier::new();
    let missing = must_err(
        validate_wsc_cas_addressed_wal_export(&export, &root, &EchoCasAvailability(&empty_cas)),
        "CAS-addressed WSC import requires present blobs",
    );
    assert!(matches!(
        missing,
        WscCasAddressedWalImportError::MissingCasBlob {
            content_hash,
            semantic_coordinate_digest,
        } if content_hash == segment_content_hash
            && semantic_coordinate_digest == digest("wsc-cas:segment")
    ));
}

#[test]
fn wsc_retained_evidence_export_modes() {
    let label = "wsc-retained-evidence";
    let dir = temp_wal_dir(label);
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let acceptance = submission_acceptance(label);
    let receipt = receipt_record(label, WalTickDecision::Applied);
    let correlation = correlation_record(label);
    let retained_payload = b"retained WSC payload bytes";
    let retained_digest = blake3::hash(retained_payload).into();
    let retained_coordinate = digest("coordinate:wsc-retained-evidence");
    let retained_material = RetainedMaterialRecord {
        material_digest: retained_digest,
        semantic_coordinate_digest: retained_coordinate,
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Present,
    };
    let reading_ref = ReadingRefRecord {
        reading_id: digest("reading:wsc-retained-evidence"),
        semantic_coordinate_digest: retained_coordinate,
        payload_digest: retained_digest,
        envelope_digest: digest("envelope:wsc-retained-evidence"),
        posture: EvidenceMaterialPosture::Present,
    };

    must_ok(store.append_transaction(durable_submission_transaction(label, Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        label,
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));
    must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let segment_path = store.segment_path();
    let segment_bytes = must_ok(fs::read(&segment_path));
    let last_commit_digest = must_some(
        store
            .read_commits()
            .last()
            .map(|commit| commit.commit_digest),
    );
    let manifest = WalManifest {
        manifest_digest: digest("wsc-retained-evidence:manifest"),
        last_committed_lsn: Some(Lsn::from_raw(4)),
        last_commit_digest: Some(last_commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let certificate = build_recovery_certificate(
        &report,
        None,
        0,
        digest("wsc-retained-evidence:frontier"),
        digest("wsc-retained-evidence:indexes"),
    );
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let projection = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(projection.posture, WalRecoveryProjectionPosture::Present);
    let root = must_some(projection.root);
    let segment = root.segments[0].clone();

    let ref_only = must_ok(wsc_ref_only_wal_export(
        &root,
        wsc_records(
            &[retained_material],
            &[reading_ref],
            &[acceptance],
            &[receipt],
            &[correlation],
        ),
    ));
    let ref_only_import = must_ok(validate_wsc_ref_only_wal_export(&ref_only, &root));
    assert_eq!(ref_only_import.retention.materials, vec![retained_material]);
    assert_eq!(ref_only_import.retention.readings, vec![reading_ref]);
    assert_eq!(
        ref_only_import.retention.materials[0].semantic_coordinate_digest,
        retained_coordinate
    );
    assert_eq!(
        ref_only_import.retention.materials[0].material_digest,
        retained_digest
    );

    let self_contained = must_ok(wsc_self_contained_wal_export(
        &root,
        &[WscSelfContainedWalSegmentMaterial {
            segment_id: segment.segment_id,
            segment_bytes: segment_bytes.clone(),
        }],
        &[WscSelfContainedRetainedMaterial {
            material: retained_material,
            material_bytes: retained_payload.to_vec(),
        }],
        wsc_records(
            &[retained_material],
            &[reading_ref],
            &[acceptance],
            &[receipt],
            &[correlation],
        ),
    ));
    let self_contained_import = must_ok(validate_wsc_self_contained_wal_export(
        &self_contained,
        &root,
    ));
    assert_eq!(
        self_contained_import.retained_payloads[0].material,
        retained_material
    );
    assert_eq!(
        self_contained_import.retained_payloads[0].material_bytes,
        retained_payload
    );
    assert_eq!(self_contained_import.retention.readings, vec![reading_ref]);

    let mut cas_store = MemoryTier::new();
    let segment_content_hash = *cas_store.put(&segment_bytes).as_bytes();
    let retained_content_hash = *cas_store.put(retained_payload).as_bytes();
    let cas_addressed = must_ok(wsc_cas_addressed_wal_export(
        &root,
        &[WscCasAddressedWalSegmentMaterial {
            segment_id: segment.segment_id,
            content_hash: segment_content_hash,
            semantic_coordinate_digest: digest("wsc-retained-evidence:segment"),
            byte_len: u64::try_from(segment_bytes.len()).unwrap_or(u64::MAX),
        }],
        &[WscCasAddressedRetainedMaterialReference {
            material_kind: RetainedMaterialKind::ReadingPayload,
            content_hash: retained_content_hash,
            semantic_coordinate_digest: retained_coordinate,
            byte_len: u64::try_from(retained_payload.len()).unwrap_or(u64::MAX),
        }],
        wsc_records(
            &[retained_material],
            &[reading_ref],
            &[acceptance],
            &[receipt],
            &[correlation],
        ),
    ));
    let cas_import = must_ok(validate_wsc_cas_addressed_wal_export(
        &cas_addressed,
        &root,
        &EchoCasAvailability(&cas_store),
    ));
    assert_eq!(cas_import.retention.materials, vec![retained_material]);
    assert_eq!(cas_import.retention.readings, vec![reading_ref]);
    assert_eq!(cas_import.cas_references.retained_materials.len(), 1);
    assert_eq!(
        cas_import.cas_references.retained_materials[0].content_hash,
        retained_content_hash
    );

    let mut segment_only_cas = MemoryTier::new();
    assert_eq!(
        *segment_only_cas.put(&segment_bytes).as_bytes(),
        segment_content_hash
    );
    let missing = must_err(
        validate_wsc_cas_addressed_wal_export(
            &cas_addressed,
            &root,
            &EchoCasAvailability(&segment_only_cas),
        ),
        "CAS-addressed retained evidence import requires present blobs",
    );
    assert!(matches!(
        missing,
        WscCasAddressedWalImportError::MissingCasBlob { content_hash, .. }
            if content_hash == retained_content_hash
    ));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DindDurabilityOutcome {
    submission_id: Hash,
    canonical_envelope_digest: Hash,
    ticket_digest: Hash,
    receipt_digest: Hash,
    decision: WalTickDecision,
    reading_id: Hash,
    reading_coordinate_digest: Hash,
    reading_payload_digest: Hash,
    reading_envelope_digest: Hash,
    retained_material_digest: Hash,
    retained_payload_len: u64,
}

fn dind_durability_outcome(
    acceptance: SubmissionAcceptanceRecord,
    receipt: TickReceiptRecord,
    reading: ReadingRefRecord,
    retained_material: RetainedMaterialRecord,
    retained_payload_len: usize,
) -> DindDurabilityOutcome {
    DindDurabilityOutcome {
        submission_id: acceptance.submission_id,
        canonical_envelope_digest: acceptance.canonical_envelope_digest,
        ticket_digest: receipt.ticket_digest,
        receipt_digest: receipt.receipt_digest,
        decision: receipt.decision,
        reading_id: reading.reading_id,
        reading_coordinate_digest: reading.semantic_coordinate_digest,
        reading_payload_digest: reading.payload_digest,
        reading_envelope_digest: reading.envelope_digest,
        retained_material_digest: retained_material.material_digest,
        retained_payload_len: u64::try_from(retained_payload_len).unwrap_or(u64::MAX),
    }
}

#[test]
fn dind_durability_convergence_gate() {
    let label = "dind-durability-convergence";
    let dir = temp_wal_dir(label);
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    let writer_epoch = must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let acceptance = submission_acceptance(label);
    let receipt = receipt_record(label, WalTickDecision::Applied);
    let correlation = correlation_record(label);
    let retained_payload = b"dind convergence retained reading bytes";
    let retained_digest = blake3::hash(retained_payload).into();
    let retained_coordinate = digest("coordinate:dind-durability-convergence");
    let retained_material = RetainedMaterialRecord {
        material_digest: retained_digest,
        semantic_coordinate_digest: retained_coordinate,
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Present,
    };
    let reading = ReadingRefRecord {
        reading_id: digest("reading:dind-durability-convergence"),
        semantic_coordinate_digest: retained_coordinate,
        payload_digest: retained_digest,
        envelope_digest: digest("envelope:dind-durability-convergence"),
        posture: EvidenceMaterialPosture::Present,
    };
    let expected_outcome = dind_durability_outcome(
        acceptance,
        receipt,
        reading,
        retained_material,
        retained_payload.len(),
    );

    let submission_tx = durable_submission_transaction(label, Lsn::from_raw(0));
    let tick_tx = durable_tick_transaction(label, Lsn::from_raw(2), WalTickDecision::Applied);
    let reading_tx = must_ok(build_retained_reading_transaction(
        builder(
            transaction_id("tx:dind-durability-convergence:reading"),
            Lsn::from_raw(5),
            WalAppendAuthority::TrustedScheduler,
            WalTransactionKind::SchedulerTick,
        ),
        std::slice::from_ref(&retained_material),
        reading,
        vec![frontier(
            AffectedFrontierKind::ReadingIndex,
            "dind-convergence:reading:before",
            "dind-convergence:reading:after",
        )],
    ));
    let mut live_state = RecoveredState::default();
    for transaction in [&submission_tx, &tick_tx, &reading_tx] {
        live_state = must_ok(apply_committed_transaction(live_state, transaction));
    }

    must_ok(store.append_transaction(submission_tx));
    must_ok(store.append_transaction(tick_tx));
    must_ok(store.append_transaction(reading_tx));
    must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    let segment_path = store.segment_path();
    let segment_bytes = must_ok(fs::read(&segment_path));
    let last_commit = must_some(store.read_commits().last().cloned());
    let manifest = WalManifest {
        manifest_digest: digest("dind-durability-convergence:manifest"),
        last_committed_lsn: Some(last_commit.last_lsn),
        last_commit_digest: Some(last_commit.commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(store.publish_manifest(epoch_id(), manifest));

    assert_eq!(live_state.applied_transactions.len(), 3);
    assert_eq!(
        live_state
            .frontiers
            .get(&AffectedFrontierKind::RuntimeState),
        Some(&digest("state:dind-durability-convergence:after"))
    );
    assert_eq!(
        live_state
            .frontiers
            .get(&AffectedFrontierKind::ReadingIndex),
        Some(&digest("dind-convergence:reading:after"))
    );

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let recovered_submissions = must_ok(recover_submission_index(&report));
    let recovered_receipts = must_ok(recover_receipt_index(&report));
    let recovered_retention = must_ok(recover_retention_index(&report));
    let recovered_outcome = dind_durability_outcome(
        must_some(
            recovered_submissions
                .get(&acceptance.submission_id)
                .map(|entry| entry.acceptance),
        ),
        TickReceiptRecord {
            submission_id: acceptance.submission_id,
            ticket_digest: must_some(
                recovered_receipts
                    .ticket_by_submission
                    .get(&acceptance.submission_id)
                    .copied(),
            ),
            receipt_digest: must_some(
                recovered_receipts
                    .receipt_by_submission
                    .get(&acceptance.submission_id)
                    .copied(),
            ),
            decision: must_some(
                recovered_receipts
                    .decisions_by_receipt
                    .get(&receipt.receipt_digest)
                    .copied(),
            ),
        },
        must_some(
            recovered_retention
                .reading_by_id
                .get(&reading.reading_id)
                .copied(),
        ),
        must_some(
            recovered_retention
                .material_by_digest
                .get(&retained_material.material_digest)
                .copied(),
        ),
        retained_payload.len(),
    );
    assert_eq!(recovered_outcome, expected_outcome);
    assert!(retained_material_obstructions(
        &recovered_retention,
        &BTreeSet::from([retained_material.material_digest]),
    )
    .is_empty());

    let certificate = build_recovery_certificate(
        &report,
        None,
        0,
        digest("dind-durability-convergence:frontier"),
        digest("dind-durability-convergence:indexes"),
    );
    let writer_epoch = WalWriterEpoch::from_writer_epoch(&writer_epoch);
    let projection = project_filesystem_wal_recovery(
        &dir,
        &report,
        std::slice::from_ref(&writer_epoch),
        Some(&certificate),
    );
    assert_eq!(projection.posture, WalRecoveryProjectionPosture::Present);
    let root = must_some(projection.root);
    let segment = root.segments[0].clone();
    let root_identity_digest = root.identity_digest();

    let self_contained = must_ok(wsc_self_contained_wal_export(
        &root,
        &[WscSelfContainedWalSegmentMaterial {
            segment_id: segment.segment_id,
            segment_bytes: segment_bytes.clone(),
        }],
        &[WscSelfContainedRetainedMaterial {
            material: retained_material,
            material_bytes: retained_payload.to_vec(),
        }],
        wsc_records(
            &[retained_material],
            &[reading],
            &[acceptance],
            &[receipt],
            &[correlation],
        ),
    ));
    let imported_self_contained = must_ok(validate_wsc_self_contained_wal_export(
        &self_contained,
        &root,
    ));
    let self_contained_outcome = dind_durability_outcome(
        imported_self_contained.accepted_submissions[0],
        imported_self_contained.receipts[0],
        imported_self_contained.retention.readings[0],
        imported_self_contained.retention.materials[0],
        imported_self_contained.retained_payloads[0]
            .material_bytes
            .len(),
    );
    assert_eq!(self_contained_outcome, expected_outcome);
    assert_eq!(
        imported_self_contained.root_identity_digest,
        root_identity_digest
    );
    assert_eq!(
        imported_self_contained.retained_payloads[0].material_bytes,
        retained_payload
    );

    let mut cas_store = MemoryTier::new();
    let segment_content_hash = *cas_store.put(&segment_bytes).as_bytes();
    let retained_content_hash = *cas_store.put(retained_payload).as_bytes();
    let cas_addressed = must_ok(wsc_cas_addressed_wal_export(
        &root,
        &[WscCasAddressedWalSegmentMaterial {
            segment_id: segment.segment_id,
            content_hash: segment_content_hash,
            semantic_coordinate_digest: digest("dind-durability-convergence:segment"),
            byte_len: u64::try_from(segment_bytes.len()).unwrap_or(u64::MAX),
        }],
        &[WscCasAddressedRetainedMaterialReference {
            material_kind: RetainedMaterialKind::ReadingPayload,
            content_hash: retained_content_hash,
            semantic_coordinate_digest: retained_coordinate,
            byte_len: u64::try_from(retained_payload.len()).unwrap_or(u64::MAX),
        }],
        wsc_records(
            &[retained_material],
            &[reading],
            &[acceptance],
            &[receipt],
            &[correlation],
        ),
    ));
    let imported_cas = must_ok(validate_wsc_cas_addressed_wal_export(
        &cas_addressed,
        &root,
        &EchoCasAvailability(&cas_store),
    ));
    let cas_outcome = dind_durability_outcome(
        imported_cas.accepted_submissions[0],
        imported_cas.receipts[0],
        imported_cas.retention.readings[0],
        imported_cas.retention.materials[0],
        usize::try_from(imported_cas.cas_references.retained_materials[0].byte_len)
            .unwrap_or(usize::MAX),
    );
    assert_eq!(cas_outcome, expected_outcome);
    assert_eq!(imported_cas.root_identity_digest, root_identity_digest);
    assert_eq!(
        imported_cas.cas_references.retained_materials[0].content_hash,
        retained_content_hash
    );

    let mut segment_only_cas = MemoryTier::new();
    assert_eq!(
        *segment_only_cas.put(&segment_bytes).as_bytes(),
        segment_content_hash
    );
    let missing_retained = must_err(
        validate_wsc_cas_addressed_wal_export(
            &cas_addressed,
            &root,
            &EchoCasAvailability(&segment_only_cas),
        ),
        "missing retained material must obstruct rather than diverge",
    );
    assert!(matches!(
        missing_retained,
        WscCasAddressedWalImportError::MissingCasBlob {
            content_hash,
            semantic_coordinate_digest,
        } if content_hash == retained_content_hash
            && semantic_coordinate_digest == retained_coordinate
    ));

    let mut corrupted_retained_bytes = retained_payload.to_vec();
    corrupted_retained_bytes[0] ^= 0x7f;
    let corrupt_retained = must_err(
        wsc_self_contained_wal_export(
            &root,
            &[WscSelfContainedWalSegmentMaterial {
                segment_id: segment.segment_id,
                segment_bytes: segment_bytes.clone(),
            }],
            &[WscSelfContainedRetainedMaterial {
                material: retained_material,
                material_bytes: corrupted_retained_bytes,
            }],
            wsc_records(
                &[retained_material],
                &[reading],
                &[acceptance],
                &[receipt],
                &[correlation],
            ),
        ),
        "corrupt embedded retained material must obstruct rather than diverge",
    );
    assert!(matches!(
        corrupt_retained,
        WscSelfContainedWalExportError::RetainedMaterialDigestMismatch {
            expected,
            actual
        } if expected == retained_digest && actual != expected
    ));

    let mut corrupted_segment_bytes = segment_bytes;
    let Some(last_byte) = corrupted_segment_bytes.last_mut() else {
        panic!("DIND convergence WAL fixture unexpectedly produced empty segment bytes");
    };
    *last_byte ^= 0x7f;
    let corrupt_self_contained = must_ok(wsc_self_contained_wal_export(
        &root,
        &[WscSelfContainedWalSegmentMaterial {
            segment_id: segment.segment_id,
            segment_bytes: corrupted_segment_bytes,
        }],
        &[WscSelfContainedRetainedMaterial {
            material: retained_material,
            material_bytes: retained_payload.to_vec(),
        }],
        wsc_records(
            &[retained_material],
            &[reading],
            &[acceptance],
            &[receipt],
            &[correlation],
        ),
    ));
    let corrupt_segment = must_err(
        validate_wsc_self_contained_wal_export(&corrupt_self_contained, &root),
        "corrupt embedded WAL material must obstruct rather than diverge",
    );
    assert!(matches!(
        corrupt_segment,
        WscSelfContainedWalImportError::SegmentRecovery {
            segment_id,
            error: WalRecoveryError::Store(WalStoreError::SegmentRecordDigestMismatch),
        } if segment_id == segment.segment_id
    ));
}

fn root_with_absolute_locator(root: &WalRoot, path: &str) -> WalRoot {
    let mut root = root.clone();
    root.segments[0].storage_locator =
        Some(WalSegmentStorageLocator::AbsolutePath(PathBuf::from(path)));
    root
}

fn builder(
    transaction_id: WalTransactionId,
    first_lsn: Lsn,
    authority: WalAppendAuthority,
    kind: WalTransactionKind,
) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        epoch_id(),
        WalSegmentId::from_raw(1),
        transaction_id,
        kind,
        authority,
        first_lsn,
        digest("previous-frame"),
        digest("previous-commit"),
        WalDurabilityMode::Buffered,
        PayloadCodecId::from_hash(digest("codec")),
        PayloadSchemaId::from_hash(digest("schema")),
        1,
        1,
        digest("domain"),
    )
}

fn frontier(kind: AffectedFrontierKind, before: &str, after: &str) -> AffectedFrontier {
    AffectedFrontier {
        kind,
        before_digest: digest(before),
        after_digest: digest(after),
    }
}

fn submission_acceptance(label: &str) -> SubmissionAcceptanceRecord {
    SubmissionAcceptanceRecord {
        submission_id: digest(&format!("submission:{label}")),
        canonical_envelope_digest: digest(&format!("envelope:{label}")),
        idempotency_key_digest: None,
        acceptance_evidence_digest: digest(&format!("accepted-evidence:{label}")),
    }
}

fn receipt_record(label: &str, decision: WalTickDecision) -> TickReceiptRecord {
    TickReceiptRecord {
        submission_id: digest(&format!("submission:{label}")),
        ticket_digest: digest(&format!("ticket:{label}")),
        receipt_digest: digest(&format!("receipt:{label}")),
        decision,
    }
}

fn correlation_record(label: &str) -> WalReceiptCorrelationRecord {
    WalReceiptCorrelationRecord {
        submission_id: digest(&format!("submission:{label}")),
        ticket_digest: digest(&format!("ticket:{label}")),
        receipt_digest: digest(&format!("receipt:{label}")),
    }
}

fn wsc_records<'a>(
    retained_materials: &'a [RetainedMaterialRecord],
    reading_refs: &'a [ReadingRefRecord],
    accepted_submissions: &'a [SubmissionAcceptanceRecord],
    receipts: &'a [TickReceiptRecord],
    correlations: &'a [WalReceiptCorrelationRecord],
) -> WscWalCausalHistoryRecords<'a> {
    WscWalCausalHistoryRecords {
        retained_materials,
        reading_refs,
        accepted_submissions,
        receipts,
        correlations,
    }
}

fn retained_material(
    label: &str,
    kind: RetainedMaterialKind,
    posture: EvidenceMaterialPosture,
) -> RetainedMaterialRecord {
    RetainedMaterialRecord {
        material_digest: digest(&format!("material:{label}")),
        semantic_coordinate_digest: digest(&format!("coordinate:{label}")),
        kind,
        posture,
    }
}

fn reading_ref(label: &str, posture: EvidenceMaterialPosture) -> ReadingRefRecord {
    ReadingRefRecord {
        reading_id: digest(&format!("reading:{label}")),
        semantic_coordinate_digest: digest(&format!("coordinate:{label}")),
        payload_digest: digest(&format!("material:{label}:payload")),
        envelope_digest: digest(&format!("material:{label}:envelope")),
        posture,
    }
}

fn worldline(seed: u8) -> WorldlineId {
    WorldlineId::from_bytes([seed; 32])
}

fn head(seed: u8, worldline_id: WorldlineId) -> WriterHeadKey {
    WriterHeadKey {
        worldline_id,
        head_id: HeadId::from_bytes([seed; 32]),
    }
}

fn authority(seed: u8) -> AuthorityDomainRef {
    AuthorityDomainRef::new(
        OriginId::from_bytes([seed; 32]),
        AuthorityDomainId::from_bytes([seed.wrapping_add(1); 32]),
    )
}

fn topology_records() -> Vec<TopologyIntentRecord> {
    let source_worldline = worldline(1);
    let child_worldline = worldline(2);
    let strand_id = make_strand_id("wal-topology-strand");
    let braid_id = digest("topology:braid");
    vec![
        TopologyIntentRecord::StrandFork(StrandForkRecord {
            topology_intent_id: digest("topology:intent:fork"),
            strand_id,
            source_worldline_id: source_worldline,
            fork_tick: WorldlineTick::from_raw(7),
            source_commit_hash: digest("topology:source-commit"),
            source_boundary_hash: digest("topology:source-boundary"),
            child_worldline_id: child_worldline,
            writer_heads: vec![head(3, child_worldline)],
            retention_posture_digest: digest("topology:retention:fork"),
            issuer_evidence_digest: digest("topology:issuer:fork"),
            idempotency_key_digest: Some(digest("topology:idempotency:fork")),
        }),
        TopologyIntentRecord::StrandDrop(StrandDropRecord {
            topology_intent_id: digest("topology:intent:drop"),
            strand_id,
            child_worldline_id: child_worldline,
            final_tick: WorldlineTick::from_raw(11),
            drop_receipt_digest: digest("topology:drop-receipt"),
            issuer_evidence_digest: digest("topology:issuer:drop"),
            idempotency_key_digest: Some(digest("topology:idempotency:drop")),
        }),
        TopologyIntentRecord::BraidEvent(TopologyBraidEventRecord {
            topology_intent_id: digest("topology:intent:braid-event"),
            braid_id,
            event_index: 0,
            event: BraidEvent::BraidCreated {
                braid_id,
                creator_domain: authority(9),
            },
            status_after: BraidStatus::Active,
            event_digest: digest("topology:braid-event"),
            issuer_evidence_digest: digest("topology:issuer:braid"),
            idempotency_key_digest: Some(digest("topology:idempotency:braid-event")),
        }),
        TopologyIntentRecord::BraidShell(BraidShellRetentionRecord {
            topology_intent_id: digest("topology:intent:braid-shell"),
            braid_id,
            shell_digest: digest("topology:braid-shell"),
            material_digest: digest("topology:braid-shell-material"),
            basis_digest: digest("topology:braid-shell-basis"),
            outcome_kind: TopologyImportOutcomeKind::Plural,
            retention_posture_digest: digest("topology:retention:braid-shell"),
            witness_digest: digest("topology:witness:braid-shell"),
            idempotency_key_digest: Some(digest("topology:idempotency:braid-shell")),
        }),
        TopologyIntentRecord::SuffixImport(SuffixImportRecord {
            import_id: digest("topology:import"),
            remote_suffix_family_digest: digest("topology:remote-suffix-family"),
            authorship_evidence_digest: digest("topology:authorship"),
            basis_anchor_digest: digest("topology:basis-anchor"),
            bundle_digest: digest("topology:bundle"),
            source_shell_digest: digest("topology:source-shell"),
            target_basis_digest: digest("topology:target-basis"),
            outcome_kind: TopologyImportOutcomeKind::Derived,
            import_shell_digest: digest("topology:import-shell"),
            retention_posture_digest: digest("topology:retention:import"),
            idempotency_key_digest: digest("topology:idempotency:import"),
        }),
    ]
}

fn submission_transaction(first_lsn: Lsn) -> WalCommittedTransaction {
    let mut builder = builder(
        transaction_id("tx:submission"),
        first_lsn,
        WalAppendAuthority::SubmissionIntake,
        WalTransactionKind::SubmissionIntake,
    );
    must_ok(builder.push_record(
        WalRecordKind::SubmissionAcceptedRecorded,
        b"accepted".to_vec(),
    ));
    must_ok(builder.commit(vec![frontier(
        AffectedFrontierKind::SubmissionQueue,
        "queue:before",
        "queue:after",
    )]))
}

fn topology_transaction(first_lsn: Lsn) -> WalCommittedTransaction {
    let builder = builder(
        transaction_id("tx:topology"),
        first_lsn,
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::TopologyIntent,
    );
    must_ok(build_topology_intent_transaction(
        builder,
        &topology_records(),
        vec![frontier(
            AffectedFrontierKind::TopologyIndex,
            "topology:before",
            "topology:after",
        )],
    ))
}

fn durable_submission_transaction(label: &str, first_lsn: Lsn) -> WalCommittedTransaction {
    let builder = builder(
        transaction_id(&format!("tx:submission:{label}")),
        first_lsn,
        WalAppendAuthority::SubmissionIntake,
        WalTransactionKind::SubmissionIntake,
    );
    must_ok(build_submission_acceptance_transaction(
        builder,
        submission_acceptance(label),
        vec![frontier(
            AffectedFrontierKind::SubmissionQueue,
            &format!("queue:{label}:before"),
            &format!("queue:{label}:after"),
        )],
    ))
}

fn tick_transaction(first_lsn: Lsn) -> WalCommittedTransaction {
    let mut builder = builder(
        transaction_id("tx:tick"),
        first_lsn,
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(builder.push_record(WalRecordKind::TickReceiptRecorded, b"receipt".to_vec()));
    must_ok(builder.push_record(WalRecordKind::RuntimeStateDeltaRecorded, b"delta".to_vec()));
    must_ok(builder.commit(vec![
        frontier(
            AffectedFrontierKind::RuntimeState,
            "state:before",
            "state:after",
        ),
        frontier(
            AffectedFrontierKind::ReceiptIndex,
            "receipt:before",
            "receipt:after",
        ),
    ]))
}

fn durable_tick_transaction(
    label: &str,
    first_lsn: Lsn,
    decision: WalTickDecision,
) -> WalCommittedTransaction {
    let builder = builder(
        transaction_id(&format!("tx:tick:{label}")),
        first_lsn,
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(build_tick_transaction(
        builder,
        receipt_record(label, decision),
        correlation_record(label),
        digest(&format!("state-delta:{label}")),
        vec![
            frontier(
                AffectedFrontierKind::RuntimeState,
                &format!("state:{label}:before"),
                &format!("state:{label}:after"),
            ),
            frontier(
                AffectedFrontierKind::ReceiptIndex,
                &format!("receipt:{label}:before"),
                &format!("receipt:{label}:after"),
            ),
        ],
    ))
}

#[test]
fn record_kind_name_does_not_imply_commit_before_transaction_commit() {
    let kinds = [
        WalRecordKind::SubmissionAcceptedRecorded,
        WalRecordKind::SubmissionAcceptanceEvidenceRecorded,
        WalRecordKind::RuntimeLawWitnessRecorded,
        WalRecordKind::RuntimeAdmissionTicketIssued,
        WalRecordKind::TicketedRuntimeIngressRecorded,
        WalRecordKind::TickReceiptRecorded,
        WalRecordKind::RuntimeStateDeltaRecorded,
        WalRecordKind::ReceiptCorrelationRecorded,
        WalRecordKind::ReadingEnvelopeRetained,
        WalRecordKind::RetainedMaterialRefRecorded,
        WalRecordKind::SchedulerFaultQuarantined,
        WalRecordKind::TrustedRuntimeControlRecorded,
        WalRecordKind::CheckpointPublicationRecorded,
        WalRecordKind::MaterializationIntentRecorded,
        WalRecordKind::MaterializationEffectObserved,
        WalRecordKind::RecoveryPostureRecorded,
    ];

    assert!(kinds
        .iter()
        .all(|kind| kind.obeys_recorded_not_committed_grammar()));
}

#[test]
fn application_cannot_append_tick_or_runtime_control_records() {
    let mut builder = builder(
        transaction_id("tx:bad-authority"),
        Lsn::from_raw(0),
        WalAppendAuthority::SubmissionIntake,
        WalTransactionKind::SubmissionIntake,
    );

    let error = builder.push_record(WalRecordKind::TickReceiptRecorded, b"receipt".to_vec());

    assert!(matches!(
        error,
        Err(WalBuildError::WrongAppendAuthority {
            record_kind: WalRecordKind::TickReceiptRecorded,
            required: WalAppendAuthority::TrustedScheduler,
            actual: WalAppendAuthority::SubmissionIntake
        })
    ));
}

#[test]
fn transaction_builder_creates_contiguous_lsn_and_local_indexes() {
    let tx = tick_transaction(Lsn::from_raw(7));

    assert_eq!(tx.frames[0].header.lsn, Lsn::from_raw(7));
    assert_eq!(tx.frames[1].header.lsn, Lsn::from_raw(8));
    assert_eq!(
        tx.frames[0].header.transaction_local_index,
        TransactionLocalIndex::from_raw(0)
    );
    assert_eq!(
        tx.frames[1].header.transaction_local_index,
        TransactionLocalIndex::from_raw(1)
    );
    assert_eq!(tx.commit.first_lsn, Lsn::from_raw(7));
    assert_eq!(tx.commit.last_lsn, Lsn::from_raw(8));
    assert!(tx.validate().is_ok());
}

#[test]
fn commit_validation_rejects_record_count_mismatch() {
    let mut tx = tick_transaction(Lsn::from_raw(0));
    tx.commit.record_count = 99;

    assert!(tx.validate().is_err());
}

#[test]
fn commit_validation_rejects_corrupt_payload_even_when_commit_shape_exists() {
    let mut tx = submission_transaction(Lsn::from_raw(0));
    tx.frames[0].payload.canonical_bytes = b"tampered".to_vec();

    assert!(tx.validate().is_err());
}

#[test]
fn semantic_validation_rejects_record_authority_mismatch() {
    let mut builder = builder(
        transaction_id("tx:mismatched-kind"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SubmissionIntake,
    );
    must_ok(builder.push_record(WalRecordKind::TickReceiptRecorded, b"receipt".to_vec()));
    let tx = builder.commit(vec![frontier(
        AffectedFrontierKind::ReceiptIndex,
        "receipt:before",
        "receipt:after",
    )]);

    assert!(matches!(
        tx,
        Err(WalBuildError::Validation(
            warp_core::causal_wal::WalValidationError::RecordAuthorityMismatch
        ))
    ));
}

#[test]
fn in_memory_store_appends_and_recovers_committed_transactions() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));

    assert_eq!(report.transactions.len(), 1);
    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(store.read_frames().len(), 1);
}

#[test]
fn wal_store_port_seals_segment_and_publishes_manifest() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));

    let seal = must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    assert_eq!(seal.segment_id, WalSegmentId::from_raw(1));
    assert_eq!(seal.sealed_lsn, Some(Lsn::from_raw(0)));

    must_ok(store.publish_manifest(
        epoch_id(),
        WalManifest {
            manifest_digest: digest("manifest"),
            last_committed_lsn: Some(Lsn::from_raw(0)),
            last_commit_digest: Some(store.read_commits()[0].commit_digest),
            sealed_segment_count: 1,
        },
    ));
    assert_eq!(store.manifests().len(), 1);
}

#[test]
fn overlapping_writer_epochs_block_recovery() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));

    let second = store.acquire_writer_epoch(WriterEpochRequest {
        epoch_id: WriterEpochId::from_hash(digest("epoch:2")),
        ..writer_epoch_request()
    });

    assert!(matches!(
        second,
        Err(WalStoreError::WriterEpochAlreadyActive)
    ));
}

#[test]
fn read_only_recovery_reports_uncommitted_tail_without_truncating() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));
    let uncommitted = tick_transaction(Lsn::from_raw(1)).frames.remove(0);
    must_ok(store.append_uncommitted_frame(epoch_id(), uncommitted));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(0))
    );
    assert_eq!(store.read_frames().len(), 2);
}

#[test]
fn writable_recovery_truncates_uncommitted_tail() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));
    let uncommitted = tick_transaction(Lsn::from_raw(1)).frames.remove(0);
    must_ok(store.append_uncommitted_frame(epoch_id(), uncommitted));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::Writable,
    ));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::TruncatedAfter(Lsn::from_raw(0))
    );
    assert_eq!(store.read_frames().len(), 1);
}

#[test]
fn pure_replay_reducer_is_deterministic_over_committed_transactions() {
    let tx = tick_transaction(Lsn::from_raw(0));

    let first = must_ok(apply_committed_transaction(RecoveredState::default(), &tx));
    let second = must_ok(apply_committed_transaction(RecoveredState::default(), &tx));

    assert_eq!(first, second);
    assert_eq!(first.applied_transactions, vec![tx.commit.transaction_id]);
    assert_eq!(
        first
            .frontiers
            .get(&AffectedFrontierKind::RuntimeState)
            .copied(),
        Some(digest("state:after"))
    );
}

#[test]
fn schema_linter_rejects_app_nouns_and_authority_leaks() {
    let app_noun = lint_wal_schema_terms(["TextBufferCommit"], &["TextBuffer"]);
    assert!(matches!(
        app_noun,
        Err(WalSchemaLintError::ForbiddenAppNoun { .. })
    ));

    let authority_leak = lint_wal_schema_terms(["ClientRuntimeControl"], &[]);
    assert!(matches!(
        authority_leak,
        Err(WalSchemaLintError::ForbiddenAuthorityLeak { .. })
    ));

    let commit_name = lint_wal_schema_terms(["TickReceiptCommitted"], &[]);
    assert!(matches!(
        commit_name,
        Err(WalSchemaLintError::RecordNameImpliesCommit { .. })
    ));

    assert!(lint_wal_schema_terms(
        [
            "SubmissionAcceptedRecorded",
            "TickReceiptRecorded",
            "WalTransactionCommit"
        ],
        &["TextBuffer"]
    )
    .is_ok());
}

#[test]
fn accepted_submission_is_not_returned_before_wal_commit() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let tx = durable_submission_transaction("accept", Lsn::from_raw(0));
    let uncommitted = tx.frames[0].clone();
    must_ok(store.append_uncommitted_frame(epoch_id(), uncommitted));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));

    assert!(index.is_empty());
    assert_eq!(
        index.retry_posture(
            submission_acceptance("accept").submission_id,
            submission_acceptance("accept").canonical_envelope_digest
        ),
        SubmissionRetryPosture::NotAccepted
    );
}

#[test]
fn crash_after_submission_commit_recovers_pending_submission() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("pending", Lsn::from_raw(0))));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));
    let entry = must_some(index.get(&submission_acceptance("pending").submission_id));

    assert_eq!(entry.posture, RecoveredSubmissionPosture::AcceptedPending);
    assert_eq!(index.len(), 1);
}

#[test]
fn crash_after_submission_commit_before_ack_retry_returns_duplicate_posture() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("retry", Lsn::from_raw(0))));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));
    let record = submission_acceptance("retry");

    assert_eq!(
        index.retry_posture(record.submission_id, record.canonical_envelope_digest),
        SubmissionRetryPosture::AlreadyAcceptedPending
    );
}

#[test]
fn same_payload_new_submission_id_is_not_duplicate_without_policy() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "same-payload",
        Lsn::from_raw(0),
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));
    let existing = submission_acceptance("same-payload");

    assert_eq!(
        index.retry_posture(
            digest("submission:new-id"),
            existing.canonical_envelope_digest
        ),
        SubmissionRetryPosture::NewSubmissionWithoutPolicyDedupe
    );
    assert_eq!(
        index.retry_posture(existing.submission_id, digest("envelope:different")),
        SubmissionRetryPosture::ConflictSameIdDifferentEnvelope
    );
}

#[test]
fn recovery_certificate_reports_clean_submission_posture() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "certificate",
        Lsn::from_raw(0),
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let certificate =
        build_recovery_certificate(&report, None, 0, digest("frontiers"), digest("indexes"));

    assert_eq!(certificate.committed_transactions_replayed, 1);
    assert_eq!(certificate.first_lsn, Some(Lsn::from_raw(0)));
    assert_eq!(certificate.last_lsn, Some(Lsn::from_raw(1)));
    assert_eq!(certificate.tail_posture, RecoveryTailPosture::Clean);
}

#[test]
fn crash_before_tick_commit_commits_no_receipt() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "tick-tail",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction("tick-tail", Lsn::from_raw(2), WalTickDecision::Applied);
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let receipt_index = must_ok(recover_receipt_index(&report));

    assert!(receipt_index.receipt_by_submission.is_empty());
    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
}

#[test]
fn crash_after_tick_commit_recovers_receipt_and_state_delta() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("applied", Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        "applied",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let submissions = must_ok(recover_submission_index(&report));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        must_some(submissions.get(&submission_acceptance("applied").submission_id)).posture,
        RecoveredSubmissionPosture::DecidedApplied
    );
    assert_eq!(
        receipts
            .receipt_by_submission
            .get(&digest("submission:applied"))
            .copied(),
        Some(digest("receipt:applied"))
    );
}

#[test]
fn committed_receipt_correlation_rebuilds_after_restart() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_tick_transaction(
        "correlated",
        Lsn::from_raw(0),
        WalTickDecision::Applied,
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        receipts
            .receipt_by_ticket
            .get(&digest("ticket:correlated"))
            .copied(),
        Some(digest("receipt:correlated"))
    );
    assert_eq!(
        receipts
            .ticket_by_submission
            .get(&digest("submission:correlated"))
            .copied(),
        Some(digest("ticket:correlated"))
    );
}

#[test]
fn lawful_rejection_recovers_without_fault_posture() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("rejected", Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        "rejected",
        Lsn::from_raw(2),
        WalTickDecision::RejectedFootprintConflict,
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let submissions = must_ok(recover_submission_index(&report));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        must_some(submissions.get(&digest("submission:rejected"))).posture,
        RecoveredSubmissionPosture::DecidedRejected
    );
    assert!(must_some(
        receipts
            .decisions_by_receipt
            .get(&digest("receipt:rejected"))
            .copied()
    )
    .is_lawful_rejection());
}

#[test]
fn retained_queryview_reading_lookup_recovers_by_semantic_coordinate() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = [
        retained_material(
            "query",
            RetainedMaterialKind::ReadingPayload,
            EvidenceMaterialPosture::Present,
        ),
        RetainedMaterialRecord {
            material_digest: digest("material:query:envelope"),
            semantic_coordinate_digest: digest("coordinate:query"),
            kind: RetainedMaterialKind::ReadingEnvelope,
            posture: EvidenceMaterialPosture::Present,
        },
    ];
    let builder = builder(
        transaction_id("tx:reading"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &material,
            reading_ref("query", EvidenceMaterialPosture::Present),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "reading:before",
                "reading:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));

    assert!(must_some(
        retention
            .readings_by_semantic_coordinate
            .get(&digest("coordinate:query"))
    )
    .contains(&digest("reading:query")));
}

#[test]
fn same_payload_with_different_query_coordinate_remains_distinct() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let payload = digest("shared-payload");
    let first = RetainedMaterialRecord {
        material_digest: payload,
        semantic_coordinate_digest: digest("coordinate:first"),
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Present,
    };
    let second = RetainedMaterialRecord {
        material_digest: payload,
        semantic_coordinate_digest: digest("coordinate:second"),
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Present,
    };
    let builder = builder(
        transaction_id("tx:semantic-identity"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &[first, second],
            reading_ref("semantic-identity", EvidenceMaterialPosture::Present),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "reading:before",
                "reading:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));

    assert!(retention
        .material_by_semantic_coordinate
        .contains_key(&digest("coordinate:first")));
    assert!(retention
        .material_by_semantic_coordinate
        .contains_key(&digest("coordinate:second")));
}

#[test]
fn topology_intent_transaction_recovers_strands_braids_shells_and_imports() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let tx = topology_transaction(Lsn::from_raw(0));
    must_ok(store.append_transaction(tx));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let topology = must_ok(recover_topology_index(&report));
    let records = topology_records();
    let strand_id = make_strand_id("wal-topology-strand");
    let child_worldline = worldline(2);
    let braid_id = digest("topology:braid");
    let shell_digest = digest("topology:braid-shell");
    let import_id = digest("topology:import");
    let topology_root = recovered_topology_index_root(&topology);
    let certificate = build_recovery_certificate(&report, None, 0, [0; 32], topology_root);

    assert_eq!(topology.len(), 5);
    assert_eq!(
        topology.strand_forks.get(&strand_id),
        match &records[0] {
            TopologyIntentRecord::StrandFork(record) => Some(record),
            _ => None,
        }
    );
    assert_eq!(
        topology.child_worldlines.get(&child_worldline),
        Some(&strand_id)
    );
    assert!(topology.strand_drops.contains_key(&strand_id));
    assert_eq!(must_some(topology.braid_events.get(&braid_id)).len(), 1);
    assert!(topology.braid_shells.contains_key(&shell_digest));
    assert!(topology.suffix_imports.contains_key(&import_id));
    assert_eq!(
        topology
            .suffix_imports_by_idempotency_key
            .get(&digest("topology:idempotency:import")),
        Some(&import_id)
    );
    assert_eq!(certificate.recovered_indexes_root, topology_root);
}

#[test]
fn topology_uncommitted_tail_does_not_materialize_half_fork() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let tx = topology_transaction(Lsn::from_raw(0));
    let epoch = tx.commit.writer_epoch;
    for frame in tx.frames {
        must_ok(store.append_uncommitted_frame(epoch, frame));
    }

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let topology = must_ok(recover_topology_index(&report));

    assert_eq!(report.tail_posture, RecoveryTailPosture::WouldTruncateAll);
    assert!(topology.is_empty());
}

#[test]
fn topology_duplicate_idempotent_records_replay_once_and_divergent_records_obstruct() {
    let records = topology_records();
    let fork = match &records[0] {
        TopologyIntentRecord::StrandFork(record) => record.clone(),
        _ => panic!("expected strand fork fixture"),
    };
    let idempotent = must_ok(RecoveredTopologyIndex::from_topology_records([
        TopologyIntentRecord::StrandFork(fork.clone()),
        TopologyIntentRecord::StrandFork(fork.clone()),
    ]));
    assert_eq!(idempotent.strand_forks.len(), 1);

    let mut divergent = fork;
    divergent.child_worldline_id = worldline(99);
    let obstruction = must_err(
        RecoveredTopologyIndex::from_topology_records([
            TopologyIntentRecord::StrandFork(match &records[0] {
                TopologyIntentRecord::StrandFork(record) => record.clone(),
                _ => panic!("expected strand fork fixture"),
            }),
            TopologyIntentRecord::StrandFork(divergent),
        ]),
        "divergent duplicate strand fork obstructs",
    );

    assert!(matches!(
        obstruction,
        WalRecoveryIndexError::ConflictingStrandFork { .. }
    ));
}

#[test]
fn topology_strand_fork_writer_heads_are_canonical_for_payload_and_recovery() {
    let mut fork = match &topology_records()[0] {
        TopologyIntentRecord::StrandFork(record) => record.clone(),
        _ => panic!("expected strand fork fixture"),
    };
    fork.writer_heads = vec![head(9, worldline(9)), head(1, worldline(1))];
    let mut reversed = fork.clone();
    reversed.writer_heads.reverse();

    assert_eq!(fork.to_payload_bytes(), reversed.to_payload_bytes());
    let decoded = must_ok(StrandForkRecord::from_payload_bytes(
        &fork.to_payload_bytes(),
    ));
    assert_eq!(decoded.writer_heads, reversed.writer_heads);

    let index = must_ok(RecoveredTopologyIndex::from_topology_records([
        TopologyIntentRecord::StrandFork(fork),
        TopologyIntentRecord::StrandFork(reversed),
    ]));
    let recovered = must_some(index.strand_forks.values().next());
    assert_eq!(recovered.writer_heads, decoded.writer_heads);
}

#[test]
fn topology_strand_fork_decode_bounds_writer_head_count_before_allocating() {
    let fork = match &topology_records()[0] {
        TopologyIntentRecord::StrandFork(record) => record.clone(),
        _ => panic!("expected strand fork fixture"),
    };
    let mut payload = fork.to_payload_bytes();
    let writer_count_offset = 200;
    payload[writer_count_offset..writer_count_offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());

    assert!(StrandForkRecord::from_payload_bytes(&payload).is_err());
}

#[test]
fn topology_strand_fork_and_drop_must_name_same_child_worldline() {
    let records = topology_records();
    let fork = match &records[0] {
        TopologyIntentRecord::StrandFork(record) => record.clone(),
        _ => panic!("expected strand fork fixture"),
    };
    let mut drop = match &records[1] {
        TopologyIntentRecord::StrandDrop(record) => record.clone(),
        _ => panic!("expected strand drop fixture"),
    };
    drop.child_worldline_id = worldline(99);

    let obstruction = must_err(
        RecoveredTopologyIndex::from_topology_records([
            TopologyIntentRecord::StrandFork(fork),
            TopologyIntentRecord::StrandDrop(drop),
        ]),
        "fork/drop child mismatch obstructs",
    );

    assert!(matches!(
        obstruction,
        WalRecoveryIndexError::ConflictingStrandDrop { .. }
    ));
}

#[test]
fn topology_braid_event_records_must_be_self_consistent() {
    let braid_event = match &topology_records()[2] {
        TopologyIntentRecord::BraidEvent(record) => record.clone(),
        _ => panic!("expected braid event fixture"),
    };
    let mut mismatched_braid = braid_event.clone();
    mismatched_braid.event = BraidEvent::BraidCreated {
        braid_id: digest("topology:other-braid"),
        creator_domain: authority(9),
    };
    let mismatch = must_err(
        RecoveredTopologyIndex::from_topology_records([TopologyIntentRecord::BraidEvent(
            mismatched_braid,
        )]),
        "mismatched embedded braid id obstructs",
    );
    assert!(matches!(
        mismatch,
        WalRecoveryIndexError::ConflictingBraidEvent { .. }
    ));

    let mut impossible_status = braid_event;
    impossible_status.status_after = BraidStatus::Collapsed;
    let status = must_err(
        RecoveredTopologyIndex::from_topology_records([TopologyIntentRecord::BraidEvent(
            impossible_status,
        )]),
        "impossible braid status obstructs",
    );
    assert!(matches!(
        status,
        WalRecoveryIndexError::ConflictingBraidEvent { .. }
    ));
}

#[test]
fn security_and_redaction_postures_decode_without_becoming_missing() {
    for posture in [
        EvidenceMaterialPosture::Present,
        EvidenceMaterialPosture::RedactedByPolicy,
        EvidenceMaterialPosture::EncryptedKeyUnavailable,
        EvidenceMaterialPosture::Missing,
        EvidenceMaterialPosture::Corrupt,
        EvidenceMaterialPosture::Obstructed,
    ] {
        let record = retained_material("posture", RetainedMaterialKind::ReadingPayload, posture);
        let decoded = must_ok(RetainedMaterialRecord::from_payload_bytes(
            &record.to_payload_bytes(),
        ));
        assert_eq!(decoded.posture, posture);
    }
}

#[test]
fn missing_retained_material_scope_matrix_is_precise() {
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::SubmissionPayload),
        MissingMaterialScope::Submission
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::TickReceipt),
        MissingMaterialScope::ReceiptOrTicket
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::RuntimeStateDelta),
        MissingMaterialScope::RuntimeGlobal
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::ReadingEnvelope),
        MissingMaterialScope::Reading
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::Diagnostic),
        MissingMaterialScope::DiagnosticLoss
    );
}

#[test]
fn missing_retained_material_returns_typed_obstruction() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let missing = retained_material(
        "missing",
        RetainedMaterialKind::RuntimeStateDelta,
        EvidenceMaterialPosture::Present,
    );
    let builder = builder(
        transaction_id("tx:missing-material"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &[missing],
            reading_ref("missing", EvidenceMaterialPosture::Present),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "reading:before",
                "reading:after",
            )],
        ))),
    );
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());

    assert_eq!(obstructions.len(), 1);
    assert_eq!(obstructions[0].scope, MissingMaterialScope::RuntimeGlobal);
    assert_eq!(obstructions[0].posture, EvidenceMaterialPosture::Missing);
}

#[test]
fn retained_material_wal_commit_order_crash_boundaries_are_explicit() {
    let orphan_material = digest("material:orphan-before-wal");
    let mut before_wal = InMemoryWalStore::new();
    must_ok(before_wal.acquire_writer_epoch(writer_epoch_request()));
    let before_report = must_ok(recover_in_memory_store(
        &mut before_wal,
        RecoveryAccessMode::ReadOnly,
    ));
    let before_retention = must_ok(recover_retention_index(&before_report));
    let available_orphan = BTreeSet::from([orphan_material]);

    assert!(before_retention.material_by_digest.is_empty());
    assert!(before_retention.reading_by_id.is_empty());
    assert!(retained_material_obstructions(&before_retention, &available_orphan).is_empty());

    let mut after_wal = InMemoryWalStore::new();
    must_ok(after_wal.acquire_writer_epoch(writer_epoch_request()));
    let present = retained_material(
        "order-present",
        RetainedMaterialKind::ReadingPayload,
        EvidenceMaterialPosture::Present,
    );
    let redacted = retained_material(
        "order-redacted",
        RetainedMaterialKind::TickReceipt,
        EvidenceMaterialPosture::RedactedByPolicy,
    );
    let builder = builder(
        transaction_id("tx:retained-material-order"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        after_wal.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &[present, redacted],
            reading_ref("retained-material-order", EvidenceMaterialPosture::Present),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "retention-order:before",
                "retention-order:after",
            )],
        ))),
    );
    let after_report = must_ok(recover_in_memory_store(
        &mut after_wal,
        RecoveryAccessMode::ReadOnly,
    ));
    let after_retention = must_ok(recover_retention_index(&after_report));
    let present_available = BTreeSet::from([present.material_digest]);
    let available_obstructions =
        retained_material_obstructions(&after_retention, &present_available);
    let missing_obstructions = retained_material_obstructions(&after_retention, &BTreeSet::new());

    assert_eq!(
        after_retention
            .material_by_digest
            .get(&present.material_digest),
        Some(&present)
    );
    assert!(must_some(
        after_retention
            .readings_by_semantic_coordinate
            .get(&digest("coordinate:retained-material-order"))
    )
    .contains(&digest("reading:retained-material-order")));
    assert_eq!(available_obstructions.len(), 1);
    assert_eq!(
        available_obstructions[0].kind,
        RetainedMaterialKind::TickReceipt
    );
    assert_eq!(
        available_obstructions[0].scope,
        MissingMaterialScope::ReceiptOrTicket
    );
    assert_eq!(
        available_obstructions[0].posture,
        EvidenceMaterialPosture::RedactedByPolicy
    );
    assert_eq!(missing_obstructions.len(), 2);
    assert!(missing_obstructions.iter().any(|obstruction| {
        obstruction.material_digest == present.material_digest
            && obstruction.scope == MissingMaterialScope::Reading
            && obstruction.posture == EvidenceMaterialPosture::Missing
    }));
    assert!(missing_obstructions.iter().any(|obstruction| {
        obstruction.material_digest == redacted.material_digest
            && obstruction.scope == MissingMaterialScope::ReceiptOrTicket
            && obstruction.posture == EvidenceMaterialPosture::RedactedByPolicy
    }));
}

#[test]
fn retained_material_wal_commit_order_loss_scope_matrix_is_precise() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = [
        retained_material(
            "order-submission",
            RetainedMaterialKind::SubmissionPayload,
            EvidenceMaterialPosture::Present,
        ),
        retained_material(
            "order-receipt",
            RetainedMaterialKind::TickReceipt,
            EvidenceMaterialPosture::Present,
        ),
        retained_material(
            "order-runtime-state",
            RetainedMaterialKind::RuntimeStateDelta,
            EvidenceMaterialPosture::Present,
        ),
        retained_material(
            "order-runtime-control",
            RetainedMaterialKind::RuntimeControl,
            EvidenceMaterialPosture::Present,
        ),
        retained_material(
            "order-reading-payload",
            RetainedMaterialKind::ReadingPayload,
            EvidenceMaterialPosture::Present,
        ),
        retained_material(
            "order-reading-envelope",
            RetainedMaterialKind::ReadingEnvelope,
            EvidenceMaterialPosture::Present,
        ),
        retained_material(
            "order-diagnostic",
            RetainedMaterialKind::Diagnostic,
            EvidenceMaterialPosture::Present,
        ),
    ];
    let builder = builder(
        transaction_id("tx:retained-material-scope-matrix"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &material,
            reading_ref(
                "retained-material-scope-matrix",
                EvidenceMaterialPosture::Present,
            ),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "retention-scope:before",
                "retention-scope:after",
            )],
        ))),
    );
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());
    let scope_by_kind = obstructions
        .iter()
        .map(|obstruction| (obstruction.kind, obstruction.scope))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(obstructions.len(), material.len());
    assert_eq!(
        scope_by_kind.get(&RetainedMaterialKind::SubmissionPayload),
        Some(&MissingMaterialScope::Submission)
    );
    assert_eq!(
        scope_by_kind.get(&RetainedMaterialKind::TickReceipt),
        Some(&MissingMaterialScope::ReceiptOrTicket)
    );
    assert_eq!(
        scope_by_kind.get(&RetainedMaterialKind::RuntimeStateDelta),
        Some(&MissingMaterialScope::RuntimeGlobal)
    );
    assert_eq!(
        scope_by_kind.get(&RetainedMaterialKind::RuntimeControl),
        Some(&MissingMaterialScope::RuntimeGlobal)
    );
    assert_eq!(
        scope_by_kind.get(&RetainedMaterialKind::ReadingPayload),
        Some(&MissingMaterialScope::Reading)
    );
    assert_eq!(
        scope_by_kind.get(&RetainedMaterialKind::ReadingEnvelope),
        Some(&MissingMaterialScope::Reading)
    );
    assert_eq!(
        scope_by_kind.get(&RetainedMaterialKind::Diagnostic),
        Some(&MissingMaterialScope::DiagnosticLoss)
    );
}

#[test]
fn valid_checkpoint_without_checkpoint_published_record_can_be_used_after_validation() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "checkpoint",
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let checkpoint = CheckpointRecord {
        checkpoint_id: digest("checkpoint"),
        last_included_lsn: must_some(report.last_committed_lsn()),
        last_included_commit_digest: must_some(report.last_commit_digest()),
        state_root: digest("state-root"),
        index_root: digest("index-root"),
        retained_material_root: digest("material-root"),
        schema_version: 1,
        created_from_wal_digest: digest("wal-chain"),
    };

    assert_eq!(
        validate_checkpoint_record(&checkpoint, &report, &[]),
        CheckpointValidationPosture::UsableWithoutPublicationRecord
    );
}

#[test]
fn checkpoint_writer_roundtrips_atomic_checkpoint_file() {
    let checkpoint = CheckpointRecord {
        checkpoint_id: digest("checkpoint-file"),
        last_included_lsn: Lsn::from_raw(42),
        last_included_commit_digest: digest("commit:file"),
        state_root: digest("state:file"),
        index_root: digest("index:file"),
        retained_material_root: digest("material:file"),
        schema_version: 1,
        created_from_wal_digest: digest("wal:file"),
    };
    let path = temp_checkpoint_path("roundtrip");

    must_ok(write_checkpoint_record_atomic(&path, &checkpoint));
    let restored = must_ok(read_checkpoint_record(&path));

    assert_eq!(restored, checkpoint);
    let temp_path = path.with_file_name(".checkpoint.ecwal.tmp");
    assert!(!temp_path.exists());
}

#[test]
fn checkpoint_published_without_checkpoint_blocks_or_obstructs_according_to_scope() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "checkpoint-missing",
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let publication = CheckpointPublicationRecord {
        checkpoint_id: digest("checkpoint-missing"),
        checkpoint_digest: digest("checkpoint-digest"),
    };

    assert_eq!(
        evaluate_checkpoint_publication(&publication, None, &report),
        CheckpointValidationPosture::PublishedCheckpointMaterialMissing
    );
}

#[test]
fn checkpoint_publication_roundtrip_recovers_from_wal() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let publication = CheckpointPublicationRecord {
        checkpoint_id: digest("checkpoint-publication"),
        checkpoint_digest: digest("checkpoint-publication-digest"),
    };
    let builder = builder(
        transaction_id("tx:checkpoint-publication"),
        Lsn::from_raw(0),
        WalAppendAuthority::Recovery,
        WalTransactionKind::Checkpoint,
    );
    must_ok(
        store.append_transaction(must_ok(build_checkpoint_publication_transaction(
            builder,
            publication,
            vec![frontier(
                AffectedFrontierKind::CheckpointIndex,
                "checkpoint:before",
                "checkpoint:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let publications = must_ok(recover_checkpoint_publications(&report));

    assert_eq!(publications, vec![publication]);
}

#[test]
fn read_only_wal_doctor_reports_uncommitted_tail_without_truncating() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("doctor", Lsn::from_raw(0))));
    let tx = durable_tick_transaction("doctor", Lsn::from_raw(2), WalTickDecision::Applied);
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));

    let before = store.read_frames().len();
    let report = must_ok(doctor_in_memory_store(&store));

    assert_eq!(
        report.posture,
        WalDoctorPosture::RecoverableWithUncommittedTail
    );
    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(store.read_frames().len(), before);
}

#[test]
fn filesystem_wal_adapter_recovers_committed_transaction_from_disk() {
    let dir = temp_wal_dir("filesystem-committed");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem",
        Lsn::from_raw(0),
    )));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let index = must_ok(recover_submission_index(&report));

    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(
        index.retry_posture(
            submission_acceptance("filesystem").submission_id,
            submission_acceptance("filesystem").canonical_envelope_digest
        ),
        SubmissionRetryPosture::AlreadyAcceptedPending
    );
}

#[test]
fn filesystem_read_only_recovery_reports_uncommitted_tail_without_truncating() {
    let dir = temp_wal_dir("filesystem-tail-read-only");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem-tail",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction(
        "filesystem-tail",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    );
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));
    let before = must_ok(fs::metadata(store.segment_path())).len();

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let after = must_ok(fs::metadata(store.segment_path())).len();

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(after, before);
}

#[test]
fn filesystem_writable_recovery_truncates_uncommitted_tail() {
    let dir = temp_wal_dir("filesystem-tail-writable");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem-writable",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction(
        "filesystem-writable",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    );
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));

    let writable = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::Writable));
    let read_only_after = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));

    assert_eq!(
        writable.tail_posture,
        RecoveryTailPosture::TruncatedAfter(Lsn::from_raw(1))
    );
    assert_eq!(read_only_after.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(read_only_after.transactions.len(), 1);
}

#[test]
fn filesystem_torn_final_record_is_reported_as_tail_not_history() {
    let dir = temp_wal_dir("filesystem-torn-tail");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem-torn",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction(
        "filesystem-torn",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    );
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));
    let segment_path = store.segment_path();
    let original_len = must_ok(fs::metadata(&segment_path)).len();
    let file = must_ok(OpenOptions::new().write(true).open(&segment_path));
    must_ok(file.set_len(original_len.saturating_sub(13)));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert!(receipts.receipt_by_submission.is_empty());
}

#[test]
fn filesystem_manifest_publish_writes_manifest_material() {
    let dir = temp_wal_dir("filesystem-manifest");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let manifest = WalManifest {
        manifest_digest: digest("manifest:filesystem"),
        last_committed_lsn: Some(Lsn::from_raw(9)),
        last_commit_digest: Some(digest("commit:filesystem")),
        sealed_segment_count: 1,
    };

    must_ok(store.publish_manifest(epoch_id(), manifest));

    assert!(dir.join("manifest.ecwal").exists());
}

#[test]
fn strict_object_store_requires_conditional_manifest_commit() {
    let invalid = ObjectStoreWalCapabilities {
        content_addressed_object_write: true,
        verify_object_version: true,
        conditional_manifest_commit: false,
        read_after_write: ObjectStoreReadAfterWritePosture::Verified,
    };
    assert!(matches!(
        validate_strict_object_store_capabilities(invalid),
        Err(ObjectStoreCapabilityError::MissingConditionalManifestCommit)
    ));

    let valid = ObjectStoreWalCapabilities {
        conditional_manifest_commit: true,
        ..invalid
    };
    assert!(validate_strict_object_store_capabilities(valid).is_ok());
}

#[test]
fn materialization_replay_detects_existing_artifact_before_retry() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let intent = MaterializationIntentRecord {
        effect_id: digest("effect:existing"),
        expected_artifact_digest: digest("artifact:expected"),
        materialization_intent_digest: digest("materialization:intent"),
        idempotency_token: digest("idempotency:effect"),
        target_metadata_digest: digest("artifact:metadata"),
    };
    let builder = builder(
        transaction_id("tx:outbox"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::MaterializationOutbox,
    );
    must_ok(
        store.append_transaction(must_ok(build_materialization_outbox_transaction(
            builder,
            intent,
            None,
            vec![frontier(
                AffectedFrontierKind::ReceiptIndex,
                "outbox:before",
                "outbox:after",
            )],
        ))),
    );
    let mut existing = BTreeMap::new();
    existing.insert(
        intent.effect_id,
        ExistingMaterializedArtifact {
            effect_id: intent.effect_id,
            artifact_digest: intent.expected_artifact_digest,
            metadata_digest: intent.target_metadata_digest,
        },
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let outbox = must_ok(recover_materialization_outbox(&report, &existing));

    assert_eq!(
        must_some(outbox.get(&intent.effect_id)).posture,
        MaterializationReplayPosture::ExistingArtifactMatches
    );
}

#[test]
fn materialization_observation_marks_effect_already_observed() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let intent = MaterializationIntentRecord {
        effect_id: digest("effect:observed"),
        expected_artifact_digest: digest("artifact:observed"),
        materialization_intent_digest: digest("materialization:observed"),
        idempotency_token: digest("idempotency:observed"),
        target_metadata_digest: digest("artifact:observed:metadata"),
    };
    let observation = MaterializationObservationRecord {
        effect_id: intent.effect_id,
        observed_artifact_digest: intent.expected_artifact_digest,
        observed_metadata_digest: intent.target_metadata_digest,
    };
    let builder = builder(
        transaction_id("tx:outbox-observed"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::MaterializationOutbox,
    );
    must_ok(
        store.append_transaction(must_ok(build_materialization_outbox_transaction(
            builder,
            intent,
            Some(observation),
            vec![frontier(
                AffectedFrontierKind::ReceiptIndex,
                "outbox:before",
                "outbox:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let outbox = must_ok(recover_materialization_outbox(&report, &BTreeMap::new()));

    assert_eq!(
        must_some(outbox.get(&intent.effect_id)).posture,
        MaterializationReplayPosture::AlreadyObserved
    );
}

#[test]
fn materialization_outbox_recovery_returns_typed_posture() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let intent = |label: &str| MaterializationIntentRecord {
        effect_id: digest(&format!("effect:{label}")),
        expected_artifact_digest: digest(&format!("artifact:{label}")),
        materialization_intent_digest: digest(&format!("materialization:{label}")),
        idempotency_token: digest(&format!("idempotency:{label}")),
        target_metadata_digest: digest(&format!("metadata:{label}")),
    };
    let missing = intent("missing");
    let mismatch = intent("mismatch");
    let observed_mismatch = intent("observed-mismatch");
    let retained_unavailable = MaterializationIntentRecord {
        expected_artifact_digest: digest("material:retained-unavailable"),
        ..intent("retained-unavailable")
    };

    for (first_lsn, label, intent, observation) in [
        (0, "missing", missing, None),
        (1, "mismatch", mismatch, None),
        (
            2,
            "observed-mismatch",
            observed_mismatch,
            Some(MaterializationObservationRecord {
                effect_id: observed_mismatch.effect_id,
                observed_artifact_digest: digest("artifact:observed-mismatch:wrong"),
                observed_metadata_digest: observed_mismatch.target_metadata_digest,
            }),
        ),
        (4, "retained-unavailable", retained_unavailable, None),
    ] {
        must_ok(
            store.append_transaction(must_ok(build_materialization_outbox_transaction(
                builder(
                    transaction_id(&format!("tx:outbox:{label}")),
                    Lsn::from_raw(first_lsn),
                    WalAppendAuthority::TrustedScheduler,
                    WalTransactionKind::MaterializationOutbox,
                ),
                intent,
                observation,
                vec![frontier(
                    AffectedFrontierKind::ReceiptIndex,
                    &format!("{label}:outbox:before"),
                    &format!("{label}:outbox:after"),
                )],
            ))),
        );
    }
    let retained_material = RetainedMaterialRecord {
        material_digest: retained_unavailable.expected_artifact_digest,
        semantic_coordinate_digest: digest("coordinate:retained-unavailable"),
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Missing,
    };
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder(
                transaction_id("tx:outbox:retained-unavailable"),
                Lsn::from_raw(5),
                WalAppendAuthority::TrustedScheduler,
                WalTransactionKind::SchedulerTick,
            ),
            std::slice::from_ref(&retained_material),
            reading_ref("retained-unavailable", EvidenceMaterialPosture::Missing),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "retained-unavailable:before",
                "retained-unavailable:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let mut existing = BTreeMap::new();
    existing.insert(
        mismatch.effect_id,
        ExistingMaterializedArtifact {
            effect_id: mismatch.effect_id,
            artifact_digest: digest("artifact:mismatch:wrong"),
            metadata_digest: digest("metadata:mismatch:wrong"),
        },
    );
    let outbox = must_ok(recover_materialization_outbox_with_retained_material(
        &report,
        &existing,
        &retention,
        &BTreeSet::new(),
    ));

    let missing_entry = must_some(outbox.get(&missing.effect_id));
    assert_eq!(missing_entry.posture, MaterializationReplayPosture::Pending);
    assert_eq!(
        missing_entry.recovery_posture,
        MaterializationRecoveryPosture::MissingArtifact {
            effect_id: missing.effect_id,
            expected_artifact_digest: missing.expected_artifact_digest,
            expected_metadata_digest: missing.target_metadata_digest,
        }
    );

    let mismatch_entry = must_some(outbox.get(&mismatch.effect_id));
    assert_eq!(
        mismatch_entry.posture,
        MaterializationReplayPosture::Obstructed
    );
    assert_eq!(
        mismatch_entry.recovery_posture,
        MaterializationRecoveryPosture::ArtifactAndMetadataDigestMismatch {
            expected_artifact_digest: mismatch.expected_artifact_digest,
            actual_artifact_digest: digest("artifact:mismatch:wrong"),
            expected_metadata_digest: mismatch.target_metadata_digest,
            actual_metadata_digest: digest("metadata:mismatch:wrong"),
        }
    );

    let observed_mismatch_entry = must_some(outbox.get(&observed_mismatch.effect_id));
    assert_eq!(
        observed_mismatch_entry.posture,
        MaterializationReplayPosture::Obstructed
    );
    assert_eq!(
        observed_mismatch_entry.recovery_posture,
        MaterializationRecoveryPosture::ObservationDigestMismatch {
            expected_artifact_digest: observed_mismatch.expected_artifact_digest,
            observed_artifact_digest: digest("artifact:observed-mismatch:wrong"),
            expected_metadata_digest: observed_mismatch.target_metadata_digest,
            observed_metadata_digest: observed_mismatch.target_metadata_digest,
        }
    );

    let retained_unavailable_entry = must_some(outbox.get(&retained_unavailable.effect_id));
    assert_eq!(
        retained_unavailable_entry.posture,
        MaterializationReplayPosture::Obstructed
    );
    assert_eq!(
        retained_unavailable_entry.recovery_posture,
        MaterializationRecoveryPosture::RetainedMaterialUnavailable {
            material_digest: retained_material.material_digest,
            kind: retained_material.kind,
            scope: MissingMaterialScope::Reading,
            posture: EvidenceMaterialPosture::Missing,
        }
    );
}

#[test]
fn causal_commit_evidence_projects_wal_commit_anchor() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let tx = durable_submission_transaction("commit-evidence", Lsn::from_raw(0));
    let commit_digest = tx.commit.commit_digest;
    must_ok(store.append_transaction(tx));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let evidence = project_causal_commit_evidence(&report);

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].commit_digest, commit_digest);
    assert_eq!(evidence[0].lsn, Lsn::from_raw(1));
}

#[test]
fn wal_shadow_replay_detects_recovered_state_match() {
    let tx = tick_transaction(Lsn::from_raw(0));
    let live_state = must_ok(apply_committed_transaction(RecoveredState::default(), &tx));

    assert!(must_ok(shadow_replay_matches(&live_state, &[tx])));
}

#[test]
fn wal_release_readiness_audit_reports_blocked_and_ready_gates() {
    let blocked = audit_wal_release_readiness(WalReleaseReadinessGates {
        filesystem_adapter: true,
        object_store_capability_gate: true,
        ..WalReleaseReadinessGates::default()
    });
    assert!(!blocked.ready);
    assert!(blocked.passed_gates.contains(&"filesystem_adapter"));
    assert!(blocked.blocked_gates.contains(&"shadow_replay"));
    assert!(blocked.blocked_gates.contains(&"topology_recovery"));

    let ready = audit_wal_release_readiness(WalReleaseReadinessGates {
        filesystem_adapter: true,
        object_store_capability_gate: true,
        segment_repair: true,
        segment_layout_policy: true,
        segment_manifest_validation: true,
        crash_matrix: true,
        crashpoint_manifest: true,
        shadow_replay: true,
        outbox: true,
        commit_evidence: true,
        wal_doctor: true,
        semantic_validator: true,
        topology_recovery: true,
        filesystem_sync_evidence: true,
        object_store_manifest_negatives: true,
        security_redaction: true,
        app_noun_guard: true,
        external_consumer_gate: true,
    });
    assert!(ready.ready);
    assert!(ready.blocked_gates.is_empty());
}
