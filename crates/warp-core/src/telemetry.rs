// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

// Telemetry helpers for JSONL logging when the `telemetry` feature is enabled.
// Manually formats JSON to avoid non-deterministic serde_json dependency.

use crate::ident::Hash;
use crate::tx::TxId;

#[inline]
fn short_id(h: &Hash) -> String {
    #[cfg(feature = "telemetry")]
    {
        let mut short = [0u8; 8];
        short.copy_from_slice(&h[0..8]);
        return hex::encode(short);
    }
    #[allow(unreachable_code)]
    String::new()
}

#[cfg(feature = "telemetry")]
fn ts_micros() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
}

#[cfg(feature = "telemetry")]
fn emit(kind: &str, tx: TxId, rule: &Hash) {
    use std::io::Write as _;
    // Manually format JSON to avoid serde_json dependency
    let mut out = std::io::stdout().lock();
    let _ = write!(
        out,
        r#"{{"timestamp_micros":{},"tx_id":{},"event":"{}","rule_id_short":"{}"}}"#,
        ts_micros(),
        tx.value(),
        kind,
        short_id(rule)
    );
    let _ = out.write_all(b"\n");
}

/// Emits a conflict telemetry event when a rewrite fails independence checks.
///
/// Logs the transaction id and rule id (shortened) as a JSON line to stdout
/// when the `telemetry` feature is enabled. Best-effort: I/O errors are
/// ignored and timestamps fall back to 0 on clock errors.
#[cfg(feature = "telemetry")]
pub fn conflict(tx: TxId, rule: &Hash) {
    emit("conflict", tx, rule);
}

/// Emits a reserved telemetry event when a rewrite passes independence checks.
///
/// Logs the transaction id and rule id (shortened) as a JSON line to stdout
/// when the `telemetry` feature is enabled. Best-effort: I/O errors are
/// ignored and timestamps fall back to 0 on clock errors.
#[cfg(feature = "telemetry")]
pub fn reserved(tx: TxId, rule: &Hash) {
    emit("reserved", tx, rule);
}

/// Emits a summary telemetry event with transaction statistics.
///
/// Logs the transaction id, reserved count, and conflict count as a JSON line
/// to stdout when the `telemetry` feature is enabled. Called at transaction
/// finalization. Best-effort: I/O errors are ignored and timestamps may fall
/// back to 0 on clock errors.
#[cfg(feature = "telemetry")]
pub fn summary(tx: TxId, reserved_count: u64, conflict_count: u64) {
    use std::io::Write as _;
    // Manually format JSON to avoid serde_json dependency
    let mut out = std::io::stdout().lock();
    let _ = write!(
        out,
        r#"{{"timestamp_micros":{},"tx_id":{},"event":"summary","reserved":{},"conflicts":{}}}"#,
        ts_micros(),
        tx.value(),
        reserved_count,
        conflict_count
    );
    let _ = out.write_all(b"\n");
}
