// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `echo-cli wal` — read-only WAL inspection helpers.

use anyhow::Result;
use serde::Serialize;
use warp_core::causal_wal::{doctor_in_memory_store, InMemoryWalStore, RecoveryTailPosture};

use crate::cli::OutputFormat;
use crate::output::emit;

/// Read-only WAL doctor JSON/text report.
#[derive(Debug, Serialize)]
pub(crate) struct WalDoctorOutput {
    pub(crate) posture: String,
    pub(crate) tail_posture: String,
    pub(crate) committed_transactions_replayed: u64,
    pub(crate) obstruction_count: u64,
}

/// Runs `echo-cli wal doctor`.
pub(crate) fn doctor(format: &OutputFormat) -> Result<()> {
    let store = InMemoryWalStore::new();
    let report = doctor_in_memory_store(&store)?;
    let output = WalDoctorOutput {
        posture: format!("{:?}", report.posture),
        tail_posture: tail_posture_label(report.tail_posture).to_owned(),
        committed_transactions_replayed: report
            .recovery_certificate
            .committed_transactions_replayed,
        obstruction_count: report.recovery_certificate.obstruction_count,
    };
    let text = format!(
        "echo-cli wal doctor\nPosture: {}\nTail: {}\nCommitted transactions replayed: {}\nObstructions: {}\n",
        output.posture,
        output.tail_posture,
        output.committed_transactions_replayed,
        output.obstruction_count
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
