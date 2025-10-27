#![allow(missing_docs)]

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
        .unwrap()
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
    // Intentionally ignore errors; stdout is best‑effort in dev builds.
    let _ = serde_json::to_writer(std::io::stdout(), &ev);
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\n");
}

#[cfg(feature = "telemetry")]
pub fn conflict(tx: TxId, rule: &Hash) {
    emit("conflict", tx, rule);
}

#[cfg(feature = "telemetry")]
pub fn reserved(tx: TxId, rule: &Hash) {
    emit("reserved", tx, rule);
}

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
    let _ = serde_json::to_writer(std::io::stdout(), &s);
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\n");
}
