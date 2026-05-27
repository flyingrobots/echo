// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli wal` — read-only WAL inspection helpers.

use std::path::Path;

use anyhow::{bail, Result};
use serde::Serialize;
use warp_core::causal_wal::{
    doctor_filesystem_store, project_absent_causal_commit_evidence, project_causal_commit_evidence,
    recover_filesystem_store, recover_receipt_index, recover_retention_index,
    recover_submission_index, recovered_receipt_reading_chain, CausalCommitEvidence,
    CausalCommitEvidencePosture, RecoveredCausalChainPosture, RecoveredReceiptReadingChain,
    RecoveredReceiptReadingChainRequest, RecoveryAccessMode, RecoverySubmissionPostureCounts,
    RecoveryTailPosture, WalDurabilityMode,
};

use crate::cli::OutputFormat;
use crate::output::{emit, hex_hash};

const CAUSAL_COMMIT_EVIDENCE_SCHEMA_VERSION: &str = "echo.causal_commit_evidence.v1";
const CAUSAL_COMMIT_EVIDENCE_CONTRACT: &str = "echo.causal_commit_evidence";
const RECOVERY_GATE_SCHEMA_VERSION: &str = "echo.recovery.external_app_gate.v1";
const RECOVERY_GATE_CONTRACT: &str = "echo.recovery.external_app_gate";
const ECHO_CLI_PRODUCER: &str = "echo-cli";

/// Read-only WAL doctor JSON/text report.
#[derive(Debug, Serialize)]
pub(crate) struct WalDoctorOutput {
    pub(crate) posture: String,
    pub(crate) tail_posture: String,
    pub(crate) committed_transactions_replayed: u64,
    pub(crate) obstruction_count: u64,
    pub(crate) submission_posture_counts: WalSubmissionPostureCountsOutput,
}

/// Recovered submission count fields for WAL doctor reports.
#[derive(Debug, Serialize)]
pub(crate) struct WalSubmissionPostureCountsOutput {
    pub(crate) total: u64,
    pub(crate) accepted_pending: u64,
    pub(crate) decided_applied: u64,
    pub(crate) decided_rejected: u64,
    pub(crate) obstructed: u64,
    pub(crate) recovery_faulted: u64,
}

/// Read-only recovered posture for one submission id/envelope pair.
#[derive(Debug, Serialize)]
pub(crate) struct WalSubmissionPostureOutput {
    pub(crate) schema_version: &'static str,
    pub(crate) producer: &'static str,
    pub(crate) root: String,
    pub(crate) submission: WalSubmissionIdentityOutput,
    pub(crate) intake: WalSubmissionIntakeOutput,
    pub(crate) lifecycle: WalSubmissionLifecycleOutput,
    pub(crate) decision: WalSubmissionDecisionOutput,
    pub(crate) evidence_health: WalSubmissionEvidenceHealthOutput,
}

/// Recovered submission identity fields.
#[derive(Debug, Serialize)]
pub(crate) struct WalSubmissionIdentityOutput {
    pub(crate) submission_id: String,
    pub(crate) canonical_envelope_digest: String,
}

/// Recovered submission intake-disposition fields.
#[derive(Debug, Serialize)]
pub(crate) struct WalSubmissionIntakeOutput {
    pub(crate) disposition: &'static str,
    pub(crate) idempotency_law: &'static str,
    pub(crate) accepted_evidence: &'static str,
}

/// Recovered submission lifecycle fields.
#[derive(Debug, Serialize)]
pub(crate) struct WalSubmissionLifecycleOutput {
    pub(crate) posture: &'static str,
}

/// Recovered submission decision fields.
#[derive(Debug, Serialize)]
pub(crate) struct WalSubmissionDecisionOutput {
    pub(crate) result: &'static str,
    pub(crate) receipt_digest: Option<String>,
    pub(crate) ticket_digest: Option<String>,
}

/// Recovered submission evidence-health fields.
#[derive(Debug, Serialize)]
pub(crate) struct WalSubmissionEvidenceHealthOutput {
    pub(crate) status: &'static str,
}

/// Versioned causal commit evidence envelope.
#[derive(Debug, Serialize)]
pub(crate) struct CausalCommitEvidenceEnvelopeOutput {
    pub(crate) schema_version: &'static str,
    pub(crate) producer: &'static str,
    pub(crate) producer_version: &'static str,
    pub(crate) compatibility: CausalCommitEvidenceCompatibilityOutput,
    pub(crate) evidence: Vec<CausalCommitEvidenceOutput>,
}

/// Compatibility metadata for external consumers.
#[derive(Debug, Serialize)]
pub(crate) struct CausalCommitEvidenceCompatibilityOutput {
    pub(crate) contract: &'static str,
    pub(crate) minimum_consumer_schema_version: &'static str,
}

/// Stable JSON shape for one causal commit evidence anchor.
#[derive(Debug, Serialize)]
pub(crate) struct CausalCommitEvidenceOutput {
    pub(crate) evidence_id: String,
    pub(crate) posture: &'static str,
    pub(crate) source: &'static str,
    pub(crate) durability_mode: &'static str,
    pub(crate) writer_epoch: String,
    pub(crate) lsn: u64,
    pub(crate) transaction_id: String,
    pub(crate) commit_digest: String,
    pub(crate) checkpoint_digest: Option<String>,
    pub(crate) recovery_certificate_digest: Option<String>,
    pub(crate) obstruction_digest: Option<String>,
    pub(crate) reason: Option<&'static str>,
}

/// Optional reading-chain arguments for the generic recovery gate.
pub(crate) struct RecoveryGateReadingArgs<'a> {
    pub(crate) basis_digest: Option<&'a str>,
    pub(crate) reading_basis_digest: Option<&'a str>,
    pub(crate) semantic_coordinate_digest: Option<&'a str>,
    pub(crate) reading_id: Option<&'a str>,
}

/// Versioned external-app recovery gate envelope.
#[derive(Debug, Serialize)]
pub(crate) struct RecoveryGateOutput {
    pub(crate) schema_version: &'static str,
    pub(crate) producer: &'static str,
    pub(crate) producer_version: &'static str,
    pub(crate) compatibility: RecoveryGateCompatibilityOutput,
    pub(crate) tail_posture: String,
    pub(crate) certificate: RecoveryGateCertificateOutput,
    pub(crate) submission: WalSubmissionPostureOutput,
    pub(crate) causal_chain: RecoveryGateCausalChainOutput,
    pub(crate) commit_evidence: CausalCommitEvidenceEnvelopeOutput,
}

/// Compatibility metadata for external-app recovery gate consumers.
#[derive(Debug, Serialize)]
pub(crate) struct RecoveryGateCompatibilityOutput {
    pub(crate) contract: &'static str,
    pub(crate) minimum_consumer_schema_version: &'static str,
}

/// Recovery certificate summary included in the external-app gate.
#[derive(Debug, Serialize)]
pub(crate) struct RecoveryGateCertificateOutput {
    pub(crate) committed_transactions_replayed: u64,
    pub(crate) obstruction_count: u64,
    pub(crate) submission_posture_counts: WalSubmissionPostureCountsOutput,
}

/// Optional recovered receipt-to-reading chain included in the gate output.
#[derive(Debug, Serialize)]
pub(crate) struct RecoveryGateCausalChainOutput {
    pub(crate) status: &'static str,
    pub(crate) posture: Option<&'static str>,
    pub(crate) evidence_health: Option<&'static str>,
    pub(crate) ticket_digest: Option<String>,
    pub(crate) receipt_digest: Option<String>,
    pub(crate) basis_digest: Option<String>,
    pub(crate) reading_basis_digest: Option<String>,
    pub(crate) semantic_coordinate_digest: Option<String>,
    pub(crate) reading_id: Option<String>,
    pub(crate) reading_source: Option<&'static str>,
    pub(crate) reading_authority: Option<&'static str>,
}

/// Runs `echo-cli wal doctor`.
pub(crate) fn doctor(root: &Path, format: &OutputFormat) -> Result<()> {
    let report = doctor_filesystem_store(root)?;
    let output = WalDoctorOutput {
        posture: format!("{:?}", report.posture),
        tail_posture: tail_posture_label(report.tail_posture).to_owned(),
        committed_transactions_replayed: report
            .recovery_certificate
            .committed_transactions_replayed,
        obstruction_count: report.recovery_certificate.obstruction_count,
        submission_posture_counts: submission_posture_counts_output(
            report.recovery_certificate.submission_posture_counts,
        ),
    };
    let text = format!(
        "echo-cli wal doctor\nRoot: {}\nPosture: {}\nTail: {}\nCommitted transactions replayed: {}\nObstructions: {}\nRecovered submissions: total={} accepted_pending={} decided_applied={} decided_rejected={} obstructed={} recovery_faulted={}\n",
        root.display(),
        output.posture,
        output.tail_posture,
        output.committed_transactions_replayed,
        output.obstruction_count,
        output.submission_posture_counts.total,
        output.submission_posture_counts.accepted_pending,
        output.submission_posture_counts.decided_applied,
        output.submission_posture_counts.decided_rejected,
        output.submission_posture_counts.obstructed,
        output.submission_posture_counts.recovery_faulted
    );
    let json = serde_json::to_value(&output)?;
    emit(format, &text, &json)
}

fn submission_posture_counts_output(
    counts: RecoverySubmissionPostureCounts,
) -> WalSubmissionPostureCountsOutput {
    WalSubmissionPostureCountsOutput {
        total: counts.total,
        accepted_pending: counts.accepted_pending,
        decided_applied: counts.decided_applied,
        decided_rejected: counts.decided_rejected,
        obstructed: counts.obstructed,
        recovery_faulted: counts.recovery_faulted,
    }
}

/// Runs `echo-cli recovery commit-evidence`.
pub(crate) fn recovery_commit_evidence(
    root: &Path,
    evidence_id: Option<&str>,
    format: &OutputFormat,
) -> Result<()> {
    let recovery = recover_filesystem_store(root, RecoveryAccessMode::ReadOnly)?;
    let evidence_filter = evidence_id.map(parse_hash_hex).transpose()?;
    let mut evidence = project_causal_commit_evidence(&recovery);
    if let Some(evidence_id) = evidence_filter {
        evidence.retain(|item| item.evidence_id == evidence_id);
        if evidence.is_empty() {
            evidence.push(project_absent_causal_commit_evidence(
                evidence_id,
                WalDurabilityMode::ReadOnlyRecovery,
            ));
        }
    }
    let output = causal_commit_evidence_envelope(evidence);
    let text = format!(
        "echo-cli recovery commit-evidence\nRoot: {}\nSchema: {}\nEvidence count: {}\n",
        root.display(),
        output.schema_version,
        output.evidence.len()
    );
    let json = serde_json::to_value(&output)?;
    emit(format, &text, &json)
}

fn causal_commit_evidence_envelope(
    evidence: Vec<CausalCommitEvidence>,
) -> CausalCommitEvidenceEnvelopeOutput {
    CausalCommitEvidenceEnvelopeOutput {
        schema_version: CAUSAL_COMMIT_EVIDENCE_SCHEMA_VERSION,
        producer: ECHO_CLI_PRODUCER,
        producer_version: env!("CARGO_PKG_VERSION"),
        compatibility: CausalCommitEvidenceCompatibilityOutput {
            contract: CAUSAL_COMMIT_EVIDENCE_CONTRACT,
            minimum_consumer_schema_version: CAUSAL_COMMIT_EVIDENCE_SCHEMA_VERSION,
        },
        evidence: evidence
            .into_iter()
            .map(causal_commit_evidence_output)
            .collect(),
    }
}

fn causal_commit_evidence_output(evidence: CausalCommitEvidence) -> CausalCommitEvidenceOutput {
    CausalCommitEvidenceOutput {
        evidence_id: hex_hash(&evidence.evidence_id),
        posture: evidence.posture.as_str(),
        source: evidence.source.as_str(),
        durability_mode: evidence.durability_mode.as_str(),
        writer_epoch: hex_hash(&evidence.writer_epoch.as_hash()),
        lsn: evidence.lsn.as_u64(),
        transaction_id: hex_hash(&evidence.transaction_id.as_hash()),
        commit_digest: hex_hash(&evidence.commit_digest),
        checkpoint_digest: evidence.checkpoint_digest.map(|digest| hex_hash(&digest)),
        recovery_certificate_digest: evidence
            .recovery_certificate_digest
            .map(|digest| hex_hash(&digest)),
        obstruction_digest: evidence.obstruction_digest.map(|digest| hex_hash(&digest)),
        reason: causal_commit_evidence_reason(evidence),
    }
}

fn causal_commit_evidence_reason(evidence: CausalCommitEvidence) -> Option<&'static str> {
    match evidence.posture {
        CausalCommitEvidencePosture::Present => None,
        CausalCommitEvidencePosture::Absent => Some("no_recovered_commit_anchor"),
        CausalCommitEvidencePosture::Obstructed => Some("commit_anchor_obstructed"),
    }
}

/// Runs `echo-cli recovery gate`.
pub(crate) fn recovery_gate(
    root: &Path,
    submission_id: &str,
    canonical_envelope_digest: &str,
    reading_args: RecoveryGateReadingArgs<'_>,
    format: &OutputFormat,
) -> Result<()> {
    let submission_id = parse_hash_hex(submission_id)?;
    let canonical_envelope_digest = parse_hash_hex(canonical_envelope_digest)?;
    let recovery = recover_filesystem_store(root, RecoveryAccessMode::ReadOnly)?;
    let submissions = recover_submission_index(&recovery)?;
    let receipts = recover_receipt_index(&recovery)?;
    let retention = recover_retention_index(&recovery)?;
    let submission = submission_posture_output(
        root,
        submission_id,
        canonical_envelope_digest,
        &submissions,
        &receipts,
        "echo.recovery.submission_posture.v1",
    );
    let causal_chain = recovery_gate_chain_output(
        reading_args,
        &submissions,
        &receipts,
        &retention,
        submission_id,
    )?;
    let output = RecoveryGateOutput {
        schema_version: RECOVERY_GATE_SCHEMA_VERSION,
        producer: ECHO_CLI_PRODUCER,
        producer_version: env!("CARGO_PKG_VERSION"),
        compatibility: RecoveryGateCompatibilityOutput {
            contract: RECOVERY_GATE_CONTRACT,
            minimum_consumer_schema_version: RECOVERY_GATE_SCHEMA_VERSION,
        },
        tail_posture: tail_posture_label(recovery.tail_posture).to_owned(),
        certificate: RecoveryGateCertificateOutput {
            committed_transactions_replayed: recovery.transactions.len() as u64,
            obstruction_count: 0,
            submission_posture_counts: submission_posture_counts_output(
                submissions.posture_counts(),
            ),
        },
        submission,
        causal_chain,
        commit_evidence: causal_commit_evidence_envelope(project_causal_commit_evidence(&recovery)),
    };
    let text = format!(
        "echo-cli recovery gate\nRoot: {}\nSchema: {}\nTail: {}\nSubmission: {}\nLifecycle posture: {}\nDecision result: {}\nEvidence count: {}\n",
        root.display(),
        output.schema_version,
        output.tail_posture,
        output.submission.submission.submission_id,
        output.submission.lifecycle.posture,
        output.submission.decision.result,
        output.commit_evidence.evidence.len()
    );
    let json = serde_json::to_value(&output)?;
    emit(format, &text, &json)
}

fn recovery_gate_chain_output(
    reading_args: RecoveryGateReadingArgs<'_>,
    submissions: &warp_core::causal_wal::RecoveredSubmissionIndex,
    receipts: &warp_core::causal_wal::RecoveredReceiptIndex,
    retention: &warp_core::causal_wal::RecoveredRetentionIndex,
    submission_id: warp_core::Hash,
) -> Result<RecoveryGateCausalChainOutput> {
    match (
        reading_args.basis_digest,
        reading_args.reading_basis_digest,
        reading_args.semantic_coordinate_digest,
        reading_args.reading_id,
    ) {
        (None, None, None, None) => Ok(RecoveryGateCausalChainOutput {
            status: "not_requested",
            posture: None,
            evidence_health: None,
            ticket_digest: receipts
                .ticket_by_submission
                .get(&submission_id)
                .map(hex_hash),
            receipt_digest: receipts
                .receipt_by_submission
                .get(&submission_id)
                .map(hex_hash),
            basis_digest: None,
            reading_basis_digest: None,
            semantic_coordinate_digest: None,
            reading_id: None,
            reading_source: None,
            reading_authority: None,
        }),
        (
            Some(basis_digest),
            Some(reading_basis_digest),
            Some(semantic_coordinate_digest),
            Some(reading_id),
        ) => {
            let chain = recovered_receipt_reading_chain(
                submissions,
                receipts,
                retention,
                RecoveredReceiptReadingChainRequest {
                    submission_id,
                    basis_digest: parse_hash_hex(basis_digest)?,
                    reading_basis_digest: parse_hash_hex(reading_basis_digest)?,
                    semantic_coordinate_digest: parse_hash_hex(semantic_coordinate_digest)?,
                    reading_id: parse_hash_hex(reading_id)?,
                },
            );
            Ok(recovery_gate_requested_chain_output(&chain))
        }
        _ => bail!("reading chain arguments must be supplied together"),
    }
}

fn recovery_gate_requested_chain_output(
    chain: &RecoveredReceiptReadingChain,
) -> RecoveryGateCausalChainOutput {
    RecoveryGateCausalChainOutput {
        status: recovery_gate_chain_status(chain.chain_posture),
        posture: Some(chain.chain_posture.as_str()),
        evidence_health: Some(chain.evidence_health.as_str()),
        ticket_digest: chain.ticket_digest.map(|digest| hex_hash(&digest)),
        receipt_digest: chain.receipt_digest.map(|digest| hex_hash(&digest)),
        basis_digest: Some(hex_hash(&chain.basis_digest)),
        reading_basis_digest: Some(hex_hash(&chain.reading_basis_digest)),
        semantic_coordinate_digest: Some(hex_hash(&chain.semantic_coordinate_digest)),
        reading_id: Some(hex_hash(&chain.reading_id)),
        reading_source: Some(chain.reading_evidence.reading_source.as_str()),
        reading_authority: Some(chain.reading_evidence.reading_authority.as_str()),
    }
}

fn recovery_gate_chain_status(posture: RecoveredCausalChainPosture) -> &'static str {
    match posture {
        RecoveredCausalChainPosture::Complete => "evaluated",
        RecoveredCausalChainPosture::NotFound
        | RecoveredCausalChainPosture::IncompleteEvidence
        | RecoveredCausalChainPosture::BasisMismatch => "incomplete",
    }
}

/// Runs `echo-cli wal submission-posture`.
pub(crate) fn submission_posture(
    root: &Path,
    submission_id: &str,
    canonical_envelope_digest: &str,
    format: &OutputFormat,
) -> Result<()> {
    submission_posture_read(
        root,
        submission_id,
        canonical_envelope_digest,
        format,
        "echo.wal.submission_posture.v1",
        "echo-cli wal submission-posture",
    )
}

/// Runs `echo-cli recovery submission-posture`.
pub(crate) fn recovery_submission_posture(
    root: &Path,
    submission_id: &str,
    canonical_envelope_digest: &str,
    format: &OutputFormat,
) -> Result<()> {
    submission_posture_read(
        root,
        submission_id,
        canonical_envelope_digest,
        format,
        "echo.recovery.submission_posture.v1",
        "echo-cli recovery submission-posture",
    )
}

fn submission_posture_read(
    root: &Path,
    submission_id: &str,
    canonical_envelope_digest: &str,
    format: &OutputFormat,
    schema_version: &'static str,
    text_header: &'static str,
) -> Result<()> {
    let submission_id = parse_hash_hex(submission_id)?;
    let canonical_envelope_digest = parse_hash_hex(canonical_envelope_digest)?;
    let recovery = recover_filesystem_store(root, RecoveryAccessMode::ReadOnly)?;
    let submissions = recover_submission_index(&recovery)?;
    let receipts = recover_receipt_index(&recovery)?;
    let output = submission_posture_output(
        root,
        submission_id,
        canonical_envelope_digest,
        &submissions,
        &receipts,
        schema_version,
    );
    let text = format!(
        "{}\nRoot: {}\nSubmission: {}\nCanonical envelope: {}\nIntake disposition: {}\nIdempotency law: {}\nAccepted evidence: {}\nLifecycle posture: {}\nDecision result: {}\nEvidence health: {}\nReceipt: {}\nTicket: {}\n",
        text_header,
        output.root,
        output.submission.submission_id,
        output.submission.canonical_envelope_digest,
        output.intake.disposition,
        output.intake.idempotency_law,
        output.intake.accepted_evidence,
        output.lifecycle.posture,
        output.decision.result,
        output.evidence_health.status,
        output.decision.receipt_digest.as_deref().unwrap_or("None"),
        output.decision.ticket_digest.as_deref().unwrap_or("None")
    );
    let json = serde_json::to_value(&output)?;
    emit(format, &text, &json)
}

fn submission_posture_output(
    root: &Path,
    submission_id: warp_core::Hash,
    canonical_envelope_digest: warp_core::Hash,
    submissions: &warp_core::causal_wal::RecoveredSubmissionIndex,
    receipts: &warp_core::causal_wal::RecoveredReceiptIndex,
    schema_version: &'static str,
) -> WalSubmissionPostureOutput {
    let matching_entry = submissions
        .get(&submission_id)
        .filter(|entry| entry.acceptance.canonical_envelope_digest == canonical_envelope_digest);
    let status = submissions.status(submission_id, canonical_envelope_digest);
    WalSubmissionPostureOutput {
        schema_version,
        producer: ECHO_CLI_PRODUCER,
        root: root.display().to_string(),
        submission: WalSubmissionIdentityOutput {
            submission_id: hex_hash(&submission_id),
            canonical_envelope_digest: hex_hash(&canonical_envelope_digest),
        },
        intake: WalSubmissionIntakeOutput {
            disposition: status.intake_disposition.as_str(),
            idempotency_law: status.idempotency_law.as_str(),
            accepted_evidence: accepted_evidence_label(matching_entry.is_some()),
        },
        lifecycle: WalSubmissionLifecycleOutput {
            posture: status.lifecycle_posture.as_str(),
        },
        decision: WalSubmissionDecisionOutput {
            result: status.decision_result.as_str(),
            receipt_digest: matching_entry
                .and_then(|entry| entry.receipt_digest.map(|digest| hex_hash(&digest))),
            ticket_digest: matching_entry
                .and_then(|_| receipts.ticket_by_submission.get(&submission_id))
                .map(hex_hash),
        },
        evidence_health: WalSubmissionEvidenceHealthOutput {
            status: status.evidence_health.as_str(),
        },
    }
}

fn tail_posture_label(posture: RecoveryTailPosture) -> &'static str {
    match posture {
        RecoveryTailPosture::Clean => "Clean",
        RecoveryTailPosture::TruncatedAll => "TruncatedAll",
        RecoveryTailPosture::TruncatedAfter(_) => "TruncatedAfter",
        RecoveryTailPosture::WouldTruncateAll => "WouldTruncateAll",
        RecoveryTailPosture::WouldTruncateAfter(_) => "WouldTruncateAfter",
    }
}

fn accepted_evidence_label(present: bool) -> &'static str {
    if present {
        "present"
    } else {
        "absent"
    }
}

fn parse_hash_hex(input: &str) -> Result<warp_core::Hash> {
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
