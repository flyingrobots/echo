// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli wsc` — read-only WSC causal-history bundle helpers.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use warp_core::causal_wal::{
    canonical_segment_path, observe_causal_anchor_admissions, project_filesystem_wal_recovery,
    recover_filesystem_store, recover_retention_index, Lsn, ObservedCausalAnchorAdmission,
    ReadingRefRecord, RecoveryAccessMode, RecoveryCertificateRef, RecoveryTailPosture,
    RetainedMaterialRecord, SubmissionAcceptanceRecord, TickReceiptRecord, WalCommitAnchor,
    WalProjectionGraphObservationPosture, WalReceiptCorrelationRecord, WalRecordKind,
    WalRecoveryProjectionPosture, WalRoot, WalSegmentId, WalSegmentSealPosture,
    WalSegmentStorageLocator, WalTransactionId, WalWriterEpoch, WriterEpochId,
};
use warp_core::wsc::{
    validate_wsc_cas_addressed_wal_export, validate_wsc_ref_only_wal_export,
    validate_wsc_self_contained_wal_export, wsc_ref_only_wal_export, wsc_self_contained_wal_export,
    WscCasAddressedWalExport, WscCasBlobStorePort, WscCausalHistoryExportProfileKind,
    WscRefOnlyWalExport, WscSelfContainedWalExport, WscSelfContainedWalSegmentMaterial,
    WscStoreEnvelope, WscWalCausalHistoryRecords,
};
use warp_core::Hash;

use crate::cli::OutputFormat;
use crate::output::{emit, hex_hash};

const BUNDLE_MANIFEST: &str = "bundle.json";
const PROJECTION_ENVELOPE: &str = "projection.ecwsc";
const ACCEPTED_SUBMISSIONS_ENVELOPE: &str = "accepted-submissions.ecwsc";
const RECEIPT_CORRELATIONS_ENVELOPE: &str = "receipt-correlations.ecwsc";
const CAUSAL_ANCHORS_ENVELOPE: &str = "causal-anchors.ecwsc";
const RETAINED_EVIDENCE_ENVELOPE: &str = "retained-evidence.ecwsc";
const RETAINED_PAYLOADS_ENVELOPE: &str = "retained-payloads.ecwsc";
const SEGMENT_MATERIAL_ENVELOPE: &str = "segment-material.ecwsc";

/// Exports a ref-only WAL causal-history WSC bundle.
pub(crate) fn export_ref_only(
    wal_root: &Path,
    writer_epochs: &Path,
    out: &Path,
    format: &OutputFormat,
) -> Result<()> {
    let projected = project_wal_root(wal_root, writer_epochs)?;
    let export = wsc_ref_only_wal_export(&projected.root, projected.records())?;

    let manifest = write_ref_only_bundle(out, &projected.root, &export)?;
    emit_export_report("export-ref-only", out, &manifest, format)
}

/// Exports a self-contained WAL causal-history WSC bundle.
pub(crate) fn export_self_contained(
    wal_root: &Path,
    writer_epochs: &Path,
    out: &Path,
    format: &OutputFormat,
) -> Result<()> {
    let projected = project_wal_root(wal_root, writer_epochs)?;
    let segment_materials = segment_materials(wal_root, &projected.root)?;
    let export = wsc_self_contained_wal_export(
        &projected.root,
        &segment_materials,
        &[],
        projected.records(),
    )?;

    let manifest = write_self_contained_bundle(out, &projected.root, &export)?;
    emit_export_report("export-self-contained", out, &manifest, format)
}

/// Inspects a WSC causal-history bundle without importing history.
pub(crate) fn inspect(bundle: &Path, format: &OutputFormat) -> Result<()> {
    let manifest = read_manifest(bundle)?;
    let mut envelopes = Vec::new();
    for (role, path) in manifest.envelopes.paths() {
        envelopes.push(envelope_summary(bundle, role, path)?);
    }
    let report = BundleInspectOutput {
        bundle: bundle.display().to_string(),
        profile: manifest.profile.clone(),
        root_identity_digest: hex_hash(&manifest.root.to_wal_root()?.identity_digest()),
        envelope_count: envelopes.len(),
        envelopes,
    };
    let text = format!(
        "echo-cli wsc causal-history inspect\nBundle: {}\nProfile: {}\nRoot identity: {}\nEnvelopes: {}\n",
        report.bundle, report.profile, report.root_identity_digest, report.envelope_count
    );
    let json = serde_json::to_value(&report)?;
    emit(format, &text, &json)
}

/// Verifies a WSC causal-history bundle without importing history.
pub(crate) fn verify(bundle: &Path, format: &OutputFormat) -> Result<()> {
    let manifest = read_manifest(bundle)?;
    let root = manifest.root.to_wal_root()?;
    let report = match manifest.profile.as_str() {
        "ref-only" => verify_ref_only(bundle, &manifest, &root)?,
        "self-contained" => verify_self_contained(bundle, &manifest, &root)?,
        "cas-addressed" => verify_cas_addressed(bundle, &manifest, &root)?,
        profile => bail!("unsupported WSC causal-history profile: {profile}"),
    };
    let text = format!(
        "echo-cli wsc causal-history verify\nBundle: {}\nProfile: {}\nResult: {}\nRoot identity: {}\nObstructions: {}\n",
        report.bundle,
        report.profile,
        report.result,
        report.root_identity_digest,
        report.obstructions.len()
    );
    let json = serde_json::to_value(&report)?;
    emit(format, &text, &json)
}

struct ProjectedWal {
    root: WalRoot,
    accepted_submissions: Vec<SubmissionAcceptanceRecord>,
    receipts: Vec<TickReceiptRecord>,
    correlations: Vec<WalReceiptCorrelationRecord>,
    causal_anchors: Vec<ObservedCausalAnchorAdmission>,
    retained_materials: Vec<RetainedMaterialRecord>,
    reading_refs: Vec<ReadingRefRecord>,
}

struct RecoveredCausalHistoryRecords {
    accepted_submissions: Vec<SubmissionAcceptanceRecord>,
    receipts: Vec<TickReceiptRecord>,
    correlations: Vec<WalReceiptCorrelationRecord>,
    causal_anchors: Vec<ObservedCausalAnchorAdmission>,
}

impl ProjectedWal {
    fn records(&self) -> WscWalCausalHistoryRecords<'_> {
        WscWalCausalHistoryRecords {
            retained_materials: &self.retained_materials,
            reading_refs: &self.reading_refs,
            accepted_submissions: &self.accepted_submissions,
            receipts: &self.receipts,
            correlations: &self.correlations,
            causal_anchors: &self.causal_anchors,
        }
    }
}

fn project_wal_root(wal_root: &Path, writer_epochs: &Path) -> Result<ProjectedWal> {
    let writer_epochs = read_writer_epochs(writer_epochs)?;
    let report =
        recover_filesystem_store(wal_root, RecoveryAccessMode::ReadOnly).with_context(|| {
            format!(
                "failed to recover filesystem WAL root {}",
                wal_root.display()
            )
        })?;
    let projection = project_filesystem_wal_recovery(wal_root, &report, &writer_epochs, None);
    if projection.posture != WalRecoveryProjectionPosture::Present {
        bail!(
            "WAL recovery projection obstructed: {:?}",
            projection.obstructions
        );
    }
    let root = projection
        .root
        .ok_or_else(|| anyhow::anyhow!("WAL recovery projection did not produce a root"))?;
    let causal_history = causal_history_records_from_report(&report)?;
    let retention = recover_retention_index(&report)?;
    Ok(ProjectedWal {
        root,
        accepted_submissions: causal_history.accepted_submissions,
        receipts: causal_history.receipts,
        correlations: causal_history.correlations,
        causal_anchors: causal_history.causal_anchors,
        retained_materials: retention.material_by_digest.into_values().collect(),
        reading_refs: retention.reading_by_id.into_values().collect(),
    })
}

fn causal_history_records_from_report(
    report: &warp_core::causal_wal::RecoveryScanReport,
) -> Result<RecoveredCausalHistoryRecords> {
    let mut accepted_submissions = Vec::new();
    let mut receipts = Vec::new();
    let mut correlations = Vec::new();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            match frame.header.record_kind {
                WalRecordKind::SubmissionAcceptedRecorded => {
                    accepted_submissions.push(
                        SubmissionAcceptanceRecord::from_payload_bytes(
                            &frame.payload.canonical_bytes,
                        )
                        .context("failed to decode WAL submission acceptance record")?,
                    );
                }
                WalRecordKind::TickReceiptRecorded => {
                    receipts.push(
                        TickReceiptRecord::from_payload_bytes(&frame.payload.canonical_bytes)
                            .context("failed to decode WAL tick receipt record")?,
                    );
                }
                WalRecordKind::ReceiptCorrelationRecorded => {
                    correlations.push(
                        WalReceiptCorrelationRecord::from_payload_bytes(
                            &frame.payload.canonical_bytes,
                        )
                        .context("failed to decode WAL receipt correlation record")?,
                    );
                }
                _ => {}
            }
        }
    }
    let causal_anchors = observe_causal_anchor_admissions(report)
        .context("failed to observe WAL causal-anchor admission records")?;
    Ok(RecoveredCausalHistoryRecords {
        accepted_submissions,
        receipts,
        correlations,
        causal_anchors,
    })
}

fn segment_materials(
    wal_root: &Path,
    root: &WalRoot,
) -> Result<Vec<WscSelfContainedWalSegmentMaterial>> {
    root.segments
        .iter()
        .map(|segment| {
            let path = canonical_segment_path(wal_root, segment.segment_id);
            let segment_bytes = fs::read(&path)
                .with_context(|| format!("failed to read WAL segment {}", path.display()))?;
            Ok(WscSelfContainedWalSegmentMaterial {
                segment_id: segment.segment_id,
                segment_bytes,
            })
        })
        .collect()
}

fn write_ref_only_bundle(
    out: &Path,
    root: &WalRoot,
    export: &WscRefOnlyWalExport,
) -> Result<BundleManifest> {
    fs::create_dir_all(out)
        .with_context(|| format!("failed to create bundle directory {}", out.display()))?;
    write_envelope(out, PROJECTION_ENVELOPE, &export.projection_envelope)?;
    write_envelope(
        out,
        ACCEPTED_SUBMISSIONS_ENVELOPE,
        &export.accepted_submission_envelope,
    )?;
    write_envelope(
        out,
        RECEIPT_CORRELATIONS_ENVELOPE,
        &export.receipt_correlation_envelope,
    )?;
    write_envelope(out, CAUSAL_ANCHORS_ENVELOPE, &export.causal_anchor_envelope)?;
    write_envelope(out, RETAINED_EVIDENCE_ENVELOPE, &export.retention_envelope)?;
    let manifest = BundleManifest {
        schema_version: 2,
        profile: "ref-only".to_owned(),
        root: WalRootJson::from_wal_root(root),
        envelopes: BundleEnvelopePaths {
            projection: PROJECTION_ENVELOPE.to_owned(),
            accepted_submissions: ACCEPTED_SUBMISSIONS_ENVELOPE.to_owned(),
            receipt_correlations: RECEIPT_CORRELATIONS_ENVELOPE.to_owned(),
            causal_anchors: CAUSAL_ANCHORS_ENVELOPE.to_owned(),
            retained_evidence: RETAINED_EVIDENCE_ENVELOPE.to_owned(),
            segment_material: None,
            retained_payloads: None,
            cas_references: None,
        },
    };
    write_manifest(out, &manifest)?;
    Ok(manifest)
}

fn write_self_contained_bundle(
    out: &Path,
    root: &WalRoot,
    export: &WscSelfContainedWalExport,
) -> Result<BundleManifest> {
    fs::create_dir_all(out)
        .with_context(|| format!("failed to create bundle directory {}", out.display()))?;
    write_envelope(out, PROJECTION_ENVELOPE, &export.projection_envelope)?;
    write_envelope(
        out,
        SEGMENT_MATERIAL_ENVELOPE,
        &export.segment_material_envelope,
    )?;
    write_envelope(
        out,
        RETAINED_PAYLOADS_ENVELOPE,
        &export.retained_material_envelope,
    )?;
    write_envelope(
        out,
        ACCEPTED_SUBMISSIONS_ENVELOPE,
        &export.accepted_submission_envelope,
    )?;
    write_envelope(
        out,
        RECEIPT_CORRELATIONS_ENVELOPE,
        &export.receipt_correlation_envelope,
    )?;
    write_envelope(out, CAUSAL_ANCHORS_ENVELOPE, &export.causal_anchor_envelope)?;
    write_envelope(out, RETAINED_EVIDENCE_ENVELOPE, &export.retention_envelope)?;
    let manifest = BundleManifest {
        schema_version: 2,
        profile: "self-contained".to_owned(),
        root: WalRootJson::from_wal_root(root),
        envelopes: BundleEnvelopePaths {
            projection: PROJECTION_ENVELOPE.to_owned(),
            accepted_submissions: ACCEPTED_SUBMISSIONS_ENVELOPE.to_owned(),
            receipt_correlations: RECEIPT_CORRELATIONS_ENVELOPE.to_owned(),
            causal_anchors: CAUSAL_ANCHORS_ENVELOPE.to_owned(),
            retained_evidence: RETAINED_EVIDENCE_ENVELOPE.to_owned(),
            segment_material: Some(SEGMENT_MATERIAL_ENVELOPE.to_owned()),
            retained_payloads: Some(RETAINED_PAYLOADS_ENVELOPE.to_owned()),
            cas_references: None,
        },
    };
    write_manifest(out, &manifest)?;
    Ok(manifest)
}

fn verify_ref_only(
    bundle: &Path,
    manifest: &BundleManifest,
    root: &WalRoot,
) -> Result<BundleVerifyOutput> {
    let expected_dependencies =
        wsc_ref_only_wal_export(root, WscWalCausalHistoryRecords::empty())?.segment_dependencies;
    let export = WscRefOnlyWalExport {
        profile: WscCausalHistoryExportProfileKind::RefOnly,
        projection_envelope: read_envelope(bundle, &manifest.envelopes.projection)?,
        accepted_submission_envelope: read_envelope(
            bundle,
            &manifest.envelopes.accepted_submissions,
        )?,
        receipt_correlation_envelope: read_envelope(
            bundle,
            &manifest.envelopes.receipt_correlations,
        )?,
        causal_anchor_envelope: read_envelope(bundle, &manifest.envelopes.causal_anchors)?,
        retention_envelope: read_envelope(bundle, &manifest.envelopes.retained_evidence)?,
        segment_dependencies: expected_dependencies,
    };
    let imported = validate_wsc_ref_only_wal_export(&export, root)?;
    let obstructions = imported
        .segment_dependencies
        .iter()
        .map(|dependency| MaterialObstruction {
            kind: "ExternalSegmentBytesUnavailable".to_owned(),
            segment_id: Some(dependency.segment_id.as_u64()),
            content_hash: None,
            semantic_coordinate_digest: None,
        })
        .collect::<Vec<_>>();
    Ok(verify_output(
        bundle,
        "ref-only",
        root,
        VerificationSummary {
            result: "obstructed",
            accepted_submission_count: imported.accepted_submissions.len(),
            receipt_count: imported.receipts.len(),
            correlation_count: imported.correlations.len(),
            causal_anchor_count: imported.causal_anchors.len(),
            projection_node_count: imported.projection.node_count,
            obstructions,
        },
    ))
}

fn verify_self_contained(
    bundle: &Path,
    manifest: &BundleManifest,
    root: &WalRoot,
) -> Result<BundleVerifyOutput> {
    let segment_material = manifest
        .envelopes
        .segment_material
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("self-contained bundle is missing segment material"))?;
    let retained_payloads = manifest
        .envelopes
        .retained_payloads
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("self-contained bundle is missing retained payloads"))?;
    let export = WscSelfContainedWalExport {
        profile: WscCausalHistoryExportProfileKind::SelfContained,
        projection_envelope: read_envelope(bundle, &manifest.envelopes.projection)?,
        segment_material_envelope: read_envelope(bundle, segment_material)?,
        retained_material_envelope: read_envelope(bundle, retained_payloads)?,
        accepted_submission_envelope: read_envelope(
            bundle,
            &manifest.envelopes.accepted_submissions,
        )?,
        receipt_correlation_envelope: read_envelope(
            bundle,
            &manifest.envelopes.receipt_correlations,
        )?,
        causal_anchor_envelope: read_envelope(bundle, &manifest.envelopes.causal_anchors)?,
        retention_envelope: read_envelope(bundle, &manifest.envelopes.retained_evidence)?,
    };
    let imported = validate_wsc_self_contained_wal_export(&export, root)?;
    Ok(verify_output(
        bundle,
        "self-contained",
        root,
        VerificationSummary {
            result: "pass",
            accepted_submission_count: imported.accepted_submissions.len(),
            receipt_count: imported.receipts.len(),
            correlation_count: imported.correlations.len(),
            causal_anchor_count: imported.causal_anchors.len(),
            projection_node_count: imported.projection.node_count,
            obstructions: Vec::new(),
        },
    ))
}

fn verify_cas_addressed(
    bundle: &Path,
    manifest: &BundleManifest,
    root: &WalRoot,
) -> Result<BundleVerifyOutput> {
    let cas_references = manifest
        .envelopes
        .cas_references
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("CAS-addressed bundle is missing CAS references"))?;
    let export = WscCasAddressedWalExport {
        profile: WscCausalHistoryExportProfileKind::CasAddressed,
        projection_envelope: read_envelope(bundle, &manifest.envelopes.projection)?,
        cas_reference_envelope: read_envelope(bundle, cas_references)?,
        accepted_submission_envelope: read_envelope(
            bundle,
            &manifest.envelopes.accepted_submissions,
        )?,
        receipt_correlation_envelope: read_envelope(
            bundle,
            &manifest.envelopes.receipt_correlations,
        )?,
        causal_anchor_envelope: read_envelope(bundle, &manifest.envelopes.causal_anchors)?,
        retention_envelope: read_envelope(bundle, &manifest.envelopes.retained_evidence)?,
    };
    match validate_wsc_cas_addressed_wal_export(&export, root, &UnavailableCasStore) {
        Ok(imported) => Ok(verify_output(
            bundle,
            "cas-addressed",
            root,
            VerificationSummary {
                result: "pass",
                accepted_submission_count: imported.accepted_submissions.len(),
                receipt_count: imported.receipts.len(),
                correlation_count: imported.correlations.len(),
                causal_anchor_count: imported.causal_anchors.len(),
                projection_node_count: imported.projection.node_count,
                obstructions: Vec::new(),
            },
        )),
        Err(error) => Ok(verify_output(
            bundle,
            "cas-addressed",
            root,
            VerificationSummary {
                result: "obstructed",
                accepted_submission_count: 0,
                receipt_count: 0,
                correlation_count: 0,
                causal_anchor_count: 0,
                projection_node_count: 0,
                obstructions: vec![MaterialObstruction {
                    kind: format!("{error:?}"),
                    segment_id: None,
                    content_hash: None,
                    semantic_coordinate_digest: None,
                }],
            },
        )),
    }
}

struct VerificationSummary {
    result: &'static str,
    accepted_submission_count: usize,
    receipt_count: usize,
    correlation_count: usize,
    causal_anchor_count: usize,
    projection_node_count: u64,
    obstructions: Vec<MaterialObstruction>,
}

fn verify_output(
    bundle: &Path,
    profile: &str,
    root: &WalRoot,
    summary: VerificationSummary,
) -> BundleVerifyOutput {
    BundleVerifyOutput {
        bundle: bundle.display().to_string(),
        profile: profile.to_owned(),
        result: summary.result.to_owned(),
        root_identity_digest: hex_hash(&root.identity_digest()),
        accepted_submission_count: summary.accepted_submission_count,
        receipt_count: summary.receipt_count,
        correlation_count: summary.correlation_count,
        causal_anchor_count: summary.causal_anchor_count,
        projection_posture: format!(
            "{:?}",
            WalProjectionGraphObservationPosture::ObservationOnly
        ),
        projection_node_count: summary.projection_node_count,
        obstructions: summary.obstructions,
    }
}

fn emit_export_report(
    command: &str,
    out: &Path,
    manifest: &BundleManifest,
    format: &OutputFormat,
) -> Result<()> {
    let root = manifest.root.to_wal_root()?;
    let report = BundleExportOutput {
        command: command.to_owned(),
        bundle: out.display().to_string(),
        profile: manifest.profile.clone(),
        root_identity_digest: hex_hash(&root.identity_digest()),
        manifest: BUNDLE_MANIFEST.to_owned(),
        envelopes: manifest.envelopes.clone(),
    };
    let text = format!(
        "echo-cli wsc causal-history {command}\nBundle: {}\nProfile: {}\nRoot identity: {}\n",
        report.bundle, report.profile, report.root_identity_digest
    );
    let json = serde_json::to_value(&report)?;
    emit(format, &text, &json)
}

fn write_envelope(out: &Path, name: &str, envelope: &WscStoreEnvelope) -> Result<()> {
    let path = out.join(name);
    fs::write(&path, envelope.encode())
        .with_context(|| format!("failed to write WSC envelope {}", path.display()))
}

fn read_envelope(bundle: &Path, name: &str) -> Result<WscStoreEnvelope> {
    let path = bundle.join(name);
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read WSC envelope {}", path.display()))?;
    WscStoreEnvelope::decode(&bytes).map_err(|error| {
        anyhow::anyhow!(
            "failed to decode WSC envelope {}: {error:?}",
            path.display()
        )
    })
}

fn write_manifest(out: &Path, manifest: &BundleManifest) -> Result<()> {
    let path = out.join(BUNDLE_MANIFEST);
    let json = serde_json::to_vec_pretty(manifest)?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write WSC bundle manifest {}", path.display()))
}

fn read_manifest(bundle: &Path) -> Result<BundleManifest> {
    let path = bundle.join(BUNDLE_MANIFEST);
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read WSC bundle manifest {}", path.display()))?;
    let schema: BundleSchemaVersion = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to decode WSC bundle manifest {}", path.display()))?;
    if schema.schema_version != 2 {
        bail!(
            "unsupported WSC bundle schema version {}",
            schema.schema_version
        );
    }
    let manifest: BundleManifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to decode WSC bundle manifest {}", path.display()))?;
    Ok(manifest)
}

fn read_writer_epochs(path: &Path) -> Result<Vec<WalWriterEpoch>> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read writer epoch evidence {}", path.display()))?;
    let epochs: Vec<WriterEpochJson> = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to decode writer epoch evidence {}", path.display()))?;
    epochs
        .into_iter()
        .map(|epoch| epoch.to_wal_writer_epoch())
        .collect()
}

fn envelope_summary(bundle: &Path, role: &str, path: &str) -> Result<EnvelopeSummary> {
    let envelope = read_envelope(bundle, path)?;
    Ok(EnvelopeSummary {
        role: role.to_owned(),
        path: path.to_owned(),
        envelope_id: hex_hash(&envelope.id().as_hash()),
        basis_digest: hex_hash(envelope.basis_digest()),
        schema_hash: hex_hash(envelope.schema_hash()),
        wsc_digest: hex_hash(envelope.wsc_digest()),
        tick: envelope.tick(),
        encoded_len: envelope.encode().len(),
    })
}

fn hash_from_hex(input: &str) -> Result<Hash> {
    let bytes = hex::decode(input)?;
    if bytes.len() != 32 {
        bail!(
            "expected a 64-character hex hash, got {} bytes",
            bytes.len()
        );
    }
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

fn optional_hash_from_hex(input: Option<&String>) -> Result<Option<Hash>> {
    input.map(String::as_str).map(hash_from_hex).transpose()
}

fn optional_writer_epoch_id(input: Option<&String>) -> Result<Option<WriterEpochId>> {
    optional_hash_from_hex(input).map(|value| value.map(WriterEpochId::from_hash))
}

fn optional_lsn(input: Option<u64>) -> Option<Lsn> {
    input.map(Lsn::from_raw)
}

#[derive(Deserialize)]
struct BundleSchemaVersion {
    schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleManifest {
    schema_version: u32,
    profile: String,
    root: WalRootJson,
    envelopes: BundleEnvelopePaths,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleEnvelopePaths {
    projection: String,
    accepted_submissions: String,
    receipt_correlations: String,
    causal_anchors: String,
    retained_evidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    segment_material: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retained_payloads: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cas_references: Option<String>,
}

impl BundleEnvelopePaths {
    fn paths(&self) -> Vec<(&'static str, &str)> {
        let mut paths = vec![
            ("projection", self.projection.as_str()),
            ("accepted_submissions", self.accepted_submissions.as_str()),
            ("receipt_correlations", self.receipt_correlations.as_str()),
            ("causal_anchors", self.causal_anchors.as_str()),
            ("retained_evidence", self.retained_evidence.as_str()),
        ];
        if let Some(path) = self.segment_material.as_deref() {
            paths.push(("segment_material", path));
        }
        if let Some(path) = self.retained_payloads.as_deref() {
            paths.push(("retained_payloads", path));
        }
        if let Some(path) = self.cas_references.as_deref() {
            paths.push(("cas_references", path));
        }
        paths
    }
}

#[derive(Debug, Serialize)]
struct BundleExportOutput {
    command: String,
    bundle: String,
    profile: String,
    root_identity_digest: String,
    manifest: String,
    envelopes: BundleEnvelopePaths,
}

#[derive(Debug, Serialize)]
struct BundleInspectOutput {
    bundle: String,
    profile: String,
    root_identity_digest: String,
    envelope_count: usize,
    envelopes: Vec<EnvelopeSummary>,
}

#[derive(Debug, Serialize)]
struct BundleVerifyOutput {
    bundle: String,
    profile: String,
    result: String,
    root_identity_digest: String,
    accepted_submission_count: usize,
    receipt_count: usize,
    correlation_count: usize,
    causal_anchor_count: usize,
    projection_posture: String,
    projection_node_count: u64,
    obstructions: Vec<MaterialObstruction>,
}

#[derive(Debug, Serialize)]
struct MaterialObstruction {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    segment_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_coordinate_digest: Option<String>,
}

#[derive(Debug, Serialize)]
struct EnvelopeSummary {
    role: String,
    path: String,
    envelope_id: String,
    basis_digest: String,
    schema_hash: String,
    wsc_digest: String,
    tick: u64,
    encoded_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalRootJson {
    root_digest: String,
    writer_epochs: Vec<WriterEpochJson>,
    segments: Vec<WalSegmentRefJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_certificate: Option<RecoveryCertificateRefJson>,
}

impl WalRootJson {
    fn from_wal_root(root: &WalRoot) -> Self {
        Self {
            root_digest: hex_hash(&root.root_digest),
            writer_epochs: root
                .writer_epochs
                .iter()
                .map(WriterEpochJson::from_wal_writer_epoch)
                .collect(),
            segments: root
                .segments
                .iter()
                .map(WalSegmentRefJson::from_wal_segment_ref)
                .collect(),
            recovery_certificate: root
                .recovery_certificate
                .as_ref()
                .map(RecoveryCertificateRefJson::from_recovery_certificate_ref),
        }
    }

    fn to_wal_root(&self) -> Result<WalRoot> {
        Ok(WalRoot {
            root_digest: hash_from_hex(&self.root_digest)?,
            writer_epochs: self
                .writer_epochs
                .iter()
                .map(WriterEpochJson::to_wal_writer_epoch)
                .collect::<Result<Vec<_>>>()?,
            segments: self
                .segments
                .iter()
                .map(WalSegmentRefJson::to_wal_segment_ref)
                .collect::<Result<Vec<_>>>()?,
            recovery_certificate: self
                .recovery_certificate
                .as_ref()
                .map(RecoveryCertificateRefJson::to_recovery_certificate_ref)
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WriterEpochJson {
    epoch_id: String,
    storage_fencing_token: String,
    process_identity: String,
    host_identity: String,
    started_at_lsn: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_epoch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_epoch_final_commit_digest: Option<String>,
    lease_or_lock_evidence: String,
}

impl WriterEpochJson {
    fn from_wal_writer_epoch(epoch: &WalWriterEpoch) -> Self {
        Self {
            epoch_id: hex_hash(&epoch.epoch_id.as_hash()),
            storage_fencing_token: hex_hash(&epoch.storage_fencing_token),
            process_identity: hex_hash(&epoch.process_identity),
            host_identity: hex_hash(&epoch.host_identity),
            started_at_lsn: epoch.started_at_lsn.as_u64(),
            previous_epoch_id: epoch
                .previous_epoch_id
                .map(|epoch_id| hex_hash(&epoch_id.as_hash())),
            previous_epoch_final_commit_digest: epoch
                .previous_epoch_final_commit_digest
                .map(|digest| hex_hash(&digest)),
            lease_or_lock_evidence: hex_hash(&epoch.lease_or_lock_evidence),
        }
    }

    fn to_wal_writer_epoch(&self) -> Result<WalWriterEpoch> {
        Ok(WalWriterEpoch {
            epoch_id: WriterEpochId::from_hash(hash_from_hex(&self.epoch_id)?),
            storage_fencing_token: hash_from_hex(&self.storage_fencing_token)?,
            process_identity: hash_from_hex(&self.process_identity)?,
            host_identity: hash_from_hex(&self.host_identity)?,
            started_at_lsn: Lsn::from_raw(self.started_at_lsn),
            previous_epoch_id: optional_writer_epoch_id(self.previous_epoch_id.as_ref())?,
            previous_epoch_final_commit_digest: optional_hash_from_hex(
                self.previous_epoch_final_commit_digest.as_ref(),
            )?,
            lease_or_lock_evidence: hash_from_hex(&self.lease_or_lock_evidence)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalSegmentRefJson {
    writer_epoch: String,
    segment_id: u64,
    first_lsn: u64,
    last_lsn: u64,
    previous_commit_digest: String,
    final_commit_digest: String,
    segment_digest: String,
    commit_anchors: Vec<WalCommitAnchorJson>,
    seal_posture: WalSegmentSealPostureJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_locator: Option<WalSegmentStorageLocatorJson>,
}

impl WalSegmentRefJson {
    fn from_wal_segment_ref(segment: &warp_core::causal_wal::WalSegmentRef) -> Self {
        Self {
            writer_epoch: hex_hash(&segment.writer_epoch.as_hash()),
            segment_id: segment.segment_id.as_u64(),
            first_lsn: segment.first_lsn.as_u64(),
            last_lsn: segment.last_lsn.as_u64(),
            previous_commit_digest: hex_hash(&segment.previous_commit_digest),
            final_commit_digest: hex_hash(&segment.final_commit_digest),
            segment_digest: hex_hash(&segment.segment_digest),
            commit_anchors: segment
                .commit_anchors
                .iter()
                .map(WalCommitAnchorJson::from_wal_commit_anchor)
                .collect(),
            seal_posture: WalSegmentSealPostureJson::from_wal_segment_seal_posture(
                &segment.seal_posture,
            ),
            storage_locator: segment
                .storage_locator
                .as_ref()
                .map(WalSegmentStorageLocatorJson::from_wal_segment_storage_locator),
        }
    }

    fn to_wal_segment_ref(&self) -> Result<warp_core::causal_wal::WalSegmentRef> {
        Ok(warp_core::causal_wal::WalSegmentRef {
            writer_epoch: WriterEpochId::from_hash(hash_from_hex(&self.writer_epoch)?),
            segment_id: WalSegmentId::from_raw(self.segment_id),
            first_lsn: Lsn::from_raw(self.first_lsn),
            last_lsn: Lsn::from_raw(self.last_lsn),
            previous_commit_digest: hash_from_hex(&self.previous_commit_digest)?,
            final_commit_digest: hash_from_hex(&self.final_commit_digest)?,
            segment_digest: hash_from_hex(&self.segment_digest)?,
            commit_anchors: self
                .commit_anchors
                .iter()
                .map(WalCommitAnchorJson::to_wal_commit_anchor)
                .collect::<Result<Vec<_>>>()?,
            seal_posture: self.seal_posture.to_wal_segment_seal_posture(),
            storage_locator: self
                .storage_locator
                .as_ref()
                .map(WalSegmentStorageLocatorJson::to_wal_segment_storage_locator),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalCommitAnchorJson {
    transaction_id: String,
    commit_digest: String,
    first_lsn: u64,
    last_lsn: u64,
    record_count: u64,
}

impl WalCommitAnchorJson {
    fn from_wal_commit_anchor(anchor: &WalCommitAnchor) -> Self {
        Self {
            transaction_id: hex_hash(&anchor.transaction_id.as_hash()),
            commit_digest: hex_hash(&anchor.commit_digest),
            first_lsn: anchor.first_lsn.as_u64(),
            last_lsn: anchor.last_lsn.as_u64(),
            record_count: anchor.record_count,
        }
    }

    fn to_wal_commit_anchor(&self) -> Result<WalCommitAnchor> {
        Ok(WalCommitAnchor {
            transaction_id: WalTransactionId::from_hash(hash_from_hex(&self.transaction_id)?),
            commit_digest: hash_from_hex(&self.commit_digest)?,
            first_lsn: Lsn::from_raw(self.first_lsn),
            last_lsn: Lsn::from_raw(self.last_lsn),
            record_count: self.record_count,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum WalSegmentSealPostureJson {
    Open,
    Sealed { sealed_lsn: Option<u64> },
}

impl WalSegmentSealPostureJson {
    fn from_wal_segment_seal_posture(posture: &WalSegmentSealPosture) -> Self {
        match posture {
            WalSegmentSealPosture::Open => Self::Open,
            WalSegmentSealPosture::Sealed { sealed_lsn } => Self::Sealed {
                sealed_lsn: sealed_lsn.map(Lsn::as_u64),
            },
        }
    }

    fn to_wal_segment_seal_posture(&self) -> WalSegmentSealPosture {
        match self {
            Self::Open => WalSegmentSealPosture::Open,
            Self::Sealed { sealed_lsn } => WalSegmentSealPosture::Sealed {
                sealed_lsn: optional_lsn(*sealed_lsn),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum WalSegmentStorageLocatorJson {
    RelativePath { path: PathBuf },
    AbsolutePath { path: PathBuf },
}

impl WalSegmentStorageLocatorJson {
    fn from_wal_segment_storage_locator(locator: &WalSegmentStorageLocator) -> Self {
        match locator {
            WalSegmentStorageLocator::RelativePath(path) => {
                Self::RelativePath { path: path.clone() }
            }
            WalSegmentStorageLocator::AbsolutePath(path) => {
                Self::AbsolutePath { path: path.clone() }
            }
        }
    }

    fn to_wal_segment_storage_locator(&self) -> WalSegmentStorageLocator {
        match self {
            Self::RelativePath { path } => WalSegmentStorageLocator::RelativePath(path.clone()),
            Self::AbsolutePath { path } => WalSegmentStorageLocator::AbsolutePath(path.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecoveryCertificateRefJson {
    certificate_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checkpoint_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_lsn: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_lsn: Option<u64>,
    tail_posture: RecoveryTailPostureJson,
    recovered_frontier_root: String,
    recovered_indexes_root: String,
}

impl RecoveryCertificateRefJson {
    fn from_recovery_certificate_ref(certificate: &RecoveryCertificateRef) -> Self {
        Self {
            certificate_digest: hex_hash(&certificate.certificate_digest),
            checkpoint_used: certificate.checkpoint_used.map(|digest| hex_hash(&digest)),
            first_lsn: certificate.first_lsn.map(Lsn::as_u64),
            last_lsn: certificate.last_lsn.map(Lsn::as_u64),
            tail_posture: RecoveryTailPostureJson::from_recovery_tail_posture(
                certificate.tail_posture,
            ),
            recovered_frontier_root: hex_hash(&certificate.recovered_frontier_root),
            recovered_indexes_root: hex_hash(&certificate.recovered_indexes_root),
        }
    }

    fn to_recovery_certificate_ref(&self) -> Result<RecoveryCertificateRef> {
        Ok(RecoveryCertificateRef {
            certificate_digest: hash_from_hex(&self.certificate_digest)?,
            checkpoint_used: optional_hash_from_hex(self.checkpoint_used.as_ref())?,
            first_lsn: optional_lsn(self.first_lsn),
            last_lsn: optional_lsn(self.last_lsn),
            tail_posture: self.tail_posture.to_recovery_tail_posture(),
            recovered_frontier_root: hash_from_hex(&self.recovered_frontier_root)?,
            recovered_indexes_root: hash_from_hex(&self.recovered_indexes_root)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum RecoveryTailPostureJson {
    Clean,
    TruncatedAll,
    TruncatedAfter { lsn: u64 },
    WouldTruncateAll,
    WouldTruncateAfter { lsn: u64 },
}

impl RecoveryTailPostureJson {
    fn from_recovery_tail_posture(posture: RecoveryTailPosture) -> Self {
        match posture {
            RecoveryTailPosture::Clean => Self::Clean,
            RecoveryTailPosture::TruncatedAll => Self::TruncatedAll,
            RecoveryTailPosture::TruncatedAfter(lsn) => Self::TruncatedAfter { lsn: lsn.as_u64() },
            RecoveryTailPosture::WouldTruncateAll => Self::WouldTruncateAll,
            RecoveryTailPosture::WouldTruncateAfter(lsn) => {
                Self::WouldTruncateAfter { lsn: lsn.as_u64() }
            }
        }
    }

    fn to_recovery_tail_posture(&self) -> RecoveryTailPosture {
        match self {
            Self::Clean => RecoveryTailPosture::Clean,
            Self::TruncatedAll => RecoveryTailPosture::TruncatedAll,
            Self::TruncatedAfter { lsn } => {
                RecoveryTailPosture::TruncatedAfter(Lsn::from_raw(*lsn))
            }
            Self::WouldTruncateAll => RecoveryTailPosture::WouldTruncateAll,
            Self::WouldTruncateAfter { lsn } => {
                RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(*lsn))
            }
        }
    }
}

struct UnavailableCasStore;

impl WscCasBlobStorePort for UnavailableCasStore {
    fn cas_blob_bytes(&self, _content_hash: &Hash) -> Option<Vec<u8>> {
        None
    }
}
