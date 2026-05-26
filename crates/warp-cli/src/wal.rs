// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli wal` — read-only WAL inspection helpers.

use std::path::Path;

use anyhow::{bail, Result};
use serde::Serialize;
use warp_core::causal_wal::{
    doctor_filesystem_store, recover_filesystem_store, recover_receipt_index,
    recover_submission_index, RecoveryAccessMode, RecoveryTailPosture,
};

use crate::cli::OutputFormat;
use crate::output::{emit, hex_hash};

/// Read-only WAL doctor JSON/text report.
#[derive(Debug, Serialize)]
pub(crate) struct WalDoctorOutput {
    pub(crate) posture: String,
    pub(crate) tail_posture: String,
    pub(crate) committed_transactions_replayed: u64,
    pub(crate) obstruction_count: u64,
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
    };
    let text = format!(
        "echo-cli wal doctor\nRoot: {}\nPosture: {}\nTail: {}\nCommitted transactions replayed: {}\nObstructions: {}\n",
        root.display(),
        output.posture,
        output.tail_posture,
        output.committed_transactions_replayed,
        output.obstruction_count
    );
    let json = serde_json::to_value(&output)?;
    emit(format, &text, &json)
}

/// Runs `echo-cli wal submission-posture`.
pub(crate) fn submission_posture(
    root: &Path,
    submission_id: &str,
    canonical_envelope_digest: &str,
    format: &OutputFormat,
) -> Result<()> {
    let submission_id = parse_hash_hex(submission_id)?;
    let canonical_envelope_digest = parse_hash_hex(canonical_envelope_digest)?;
    let recovery = recover_filesystem_store(root, RecoveryAccessMode::ReadOnly)?;
    let submissions = recover_submission_index(&recovery)?;
    let receipts = recover_receipt_index(&recovery)?;
    let matching_entry = submissions
        .get(&submission_id)
        .filter(|entry| entry.acceptance.canonical_envelope_digest == canonical_envelope_digest);
    let status = submissions.status(submission_id, canonical_envelope_digest);
    let output = WalSubmissionPostureOutput {
        schema_version: "echo.wal.submission_posture.v1",
        producer: "echo-cli",
        root: root.display().to_string(),
        submission: WalSubmissionIdentityOutput {
            submission_id: hex_hash(&submission_id),
            canonical_envelope_digest: hex_hash(&canonical_envelope_digest),
        },
        intake: WalSubmissionIntakeOutput {
            disposition: status.intake_disposition.as_str(),
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
    };
    let text = format!(
        "echo-cli wal submission-posture\nRoot: {}\nSubmission: {}\nCanonical envelope: {}\nIntake disposition: {}\nAccepted evidence: {}\nLifecycle posture: {}\nDecision result: {}\nEvidence health: {}\nReceipt: {}\nTicket: {}\n",
        output.root,
        output.submission.submission_id,
        output.submission.canonical_envelope_digest,
        output.intake.disposition,
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
