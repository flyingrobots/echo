// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Telemetry sink trait for observability without coupling to I/O.
//!
//! The core engine emits telemetry events through this trait, allowing
//! adapters to decide how to handle them (stdout, file, network, etc.).
//! This maintains hexagonal architecture by keeping I/O concerns outside
//! the deterministic core.

use crate::ident::Hash;
use crate::tx::TxId;

/// Telemetry sink for observing scheduler events.
///
/// Implementations can log to stdout, files, metrics systems, or discard events.
/// The core engine calls these methods during scheduling but does not depend
/// on any specific I/O implementation.
///
/// All methods have default no-op implementations, so callers can implement
/// only the events they care about.
pub trait TelemetrySink: Send + Sync {
    /// Called when a rewrite fails independence checks (conflict detected).
    ///
    /// # Arguments
    /// * `tx` - The transaction ID
    /// * `rule_id` - The rule that conflicted
    fn on_conflict(&self, _tx: TxId, _rule_id: &Hash) {}

    /// Called when a rewrite passes independence checks (successfully reserved).
    ///
    /// # Arguments
    /// * `tx` - The transaction ID
    /// * `rule_id` - The rule that was reserved
    fn on_reserved(&self, _tx: TxId, _rule_id: &Hash) {}

    /// Called when a transaction is finalized with summary statistics.
    ///
    /// # Arguments
    /// * `tx` - The transaction ID
    /// * `reserved_count` - Number of rewrites successfully reserved
    /// * `conflict_count` - Number of rewrites that conflicted
    fn on_summary(&self, _tx: TxId, _reserved_count: u64, _conflict_count: u64) {}
}

/// A no-op telemetry sink that discards all events.
///
/// This is the default when no telemetry is configured.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullTelemetrySink;

impl TelemetrySink for NullTelemetrySink {}
