// Telemetry helpers for JSONL logging when the `telemetry` feature is enabled.

#[cfg(feature = "telemetry")]
use serde::Serialize;

use crate::ident::Hash;
use crate::tx::TxId;

#[cfg(feature = "telemetry")]
#[derive(Serialize)]
struct Event<'a> {
    timestamp_micros: u128,
    tx_id: u64,
    event: &'a str,
    rule_id_short: String,
}

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
    let ev = Event {
        timestamp_micros: ts_micros(),
        tx_id: tx.value(),
        event: kind,
        rule_id_short: short_id(rule),
    };
    // Best-effort stdout with a single locked write sequence to avoid interleaving.
    let mut out = std::io::stdout().lock();
    let _ = serde_json::to_writer(&mut out, &ev);
    use std::io::Write as _;
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
    use serde::Serialize;
    #[derive(Serialize)]
    struct Summary {
        timestamp_micros: u128,
        tx_id: u64,
        event: &'static str,
        reserved: u64,
        conflicts: u64,
    }
    let s = Summary {
        timestamp_micros: ts_micros(),
        tx_id: tx.value(),
        event: "summary",
        reserved: reserved_count,
        conflicts: conflict_count,
    };
    let mut out = std::io::stdout().lock();
    let _ = serde_json::to_writer(&mut out, &s);
    use std::io::Write as _;
    let _ = out.write_all(b"\n");
}
