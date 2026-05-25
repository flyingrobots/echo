// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli wal` — read-only WAL inspection helpers.

use std::path::Path;

use anyhow::{bail, Result};
use serde::Serialize;
use warp_core::causal_wal::{
    doctor_filesystem_store, recover_filesystem_store, recover_receipt_index,
    recover_submission_index, RecoveredSubmissionPosture, RecoveryAccessMode, RecoveryTailPosture,
    SubmissionRetryPosture,
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
    pub(crate) root: String,
    pub(crate) submission_id: String,
    pub(crate) canonical_envelope_digest: String,
    pub(crate) retry_posture: &'static str,
    pub(crate) recovered_posture: Option<&'static str>,
    pub(crate) receipt_digest: Option<String>,
    pub(crate) ticket_digest: Option<String>,
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
    let entry = submissions.get(&submission_id);
    let output = WalSubmissionPostureOutput {
        root: root.display().to_string(),
        submission_id: hex_hash(&submission_id),
        canonical_envelope_digest: hex_hash(&canonical_envelope_digest),
        retry_posture: retry_posture_label(
            submissions.retry_posture(submission_id, canonical_envelope_digest),
        ),
        recovered_posture: entry.map(|entry| recovered_posture_label(entry.posture)),
        receipt_digest: entry
            .and_then(|entry| entry.receipt_digest.map(|digest| hex_hash(&digest))),
        ticket_digest: receipts
            .ticket_by_submission
            .get(&submission_id)
            .map(hex_hash),
    };
    let text = format!(
        "echo-cli wal submission-posture\nRoot: {}\nSubmission: {}\nCanonical envelope: {}\nRetry posture: {}\nRecovered posture: {}\nReceipt: {}\nTicket: {}\n",
        output.root,
        output.submission_id,
        output.canonical_envelope_digest,
        output.retry_posture,
        output.recovered_posture.unwrap_or("None"),
        output.receipt_digest.as_deref().unwrap_or("None"),
        output.ticket_digest.as_deref().unwrap_or("None")
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

fn recovered_posture_label(posture: RecoveredSubmissionPosture) -> &'static str {
    match posture {
        RecoveredSubmissionPosture::AcceptedPending => "AcceptedPending",
        RecoveredSubmissionPosture::DecidedApplied => "DecidedApplied",
        RecoveredSubmissionPosture::DecidedRejected => "DecidedRejected",
        RecoveredSubmissionPosture::Obstructed => "Obstructed",
        RecoveredSubmissionPosture::RecoveryFaulted => "RecoveryFaulted",
    }
}

fn retry_posture_label(posture: SubmissionRetryPosture) -> &'static str {
    match posture {
        SubmissionRetryPosture::NotAccepted => "NotAccepted",
        SubmissionRetryPosture::AlreadyAcceptedPending => "AlreadyAcceptedPending",
        SubmissionRetryPosture::AlreadyDecidedApplied => "AlreadyDecidedApplied",
        SubmissionRetryPosture::AlreadyDecidedRejected => "AlreadyDecidedRejected",
        SubmissionRetryPosture::AlreadyObstructed => "AlreadyObstructed",
        SubmissionRetryPosture::ConflictSameIdDifferentEnvelope => {
            "ConflictSameIdDifferentEnvelope"
        }
        SubmissionRetryPosture::NewSubmissionWithoutPolicyDedupe => {
            "NewSubmissionWithoutPolicyDedupe"
        }
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
